import { Alert, Button, Input, Select, Switch, TextArea } from "@tokimo/ui";
import { Eye, Sparkles } from "lucide-react";
import { useMemo, useState } from "react";
import { api } from "../api/client";
import type { CreateRuleReq, EntryDto, MatchScope, RuleDto, SourceDto } from "../api/types";
import { joinTerms, splitTerms } from "../lib/format";

interface RuleEditorProps {
  rule: RuleDto | null;
  initialDraft?: CreateRuleReq | null;
  sources: SourceDto[];
  t: (key: string) => string;
  onClose: () => void;
  onSaved: () => void;
}

export function RuleEditor({ rule, initialDraft, sources, t, onClose, onSaved }: RuleEditorProps) {
  const [sourceId, setSourceId] = useState(rule?.sourceId ?? initialDraft?.sourceId ?? sources.find((source) => !source.archivedAt)?.id ?? "");
  const [name, setName] = useState(rule?.name ?? initialDraft?.name ?? "");
  const [enabled, setEnabled] = useState(rule?.enabled ?? initialDraft?.enabled ?? true);
  const [categories, setCategories] = useState(joinTerms(rule?.categories ?? initialDraft?.categories ?? []));
  const [includeAny, setIncludeAny] = useState(joinTerms(rule?.includeAny ?? initialDraft?.includeAny ?? []));
  const [excludeAny, setExcludeAny] = useState(joinTerms(rule?.excludeAny ?? initialDraft?.excludeAny ?? []));
  const [matchScope, setMatchScope] = useState<MatchScope>(rule?.matchScope ?? initialDraft?.matchScope ?? "title");
  const [preview, setPreview] = useState<EntryDto[] | null>(null);
  const [previewCursor, setPreviewCursor] = useState<string | null>(null);
  const [busy, setBusy] = useState<"save" | "preview" | null>(null);
  const [error, setError] = useState<string | null>(null);

  const input = useMemo<CreateRuleReq>(() => ({
    sourceId,
    name: name.trim(),
    enabled,
    categories: splitTerms(categories),
    includeAny: splitTerms(includeAny),
    excludeAny: splitTerms(excludeAny),
    matchScope,
  }), [categories, enabled, excludeAny, includeAny, matchScope, name, sourceId]);

  const previewRules = async (cursor?: string) => {
    if (!sourceId || input.includeAny.length === 0) {
      setError(t("requiredFields"));
      return;
    }
    setBusy("preview");
    setError(null);
    try {
      const response = await api.rules.preview({
        sourceId,
        categories: input.categories,
        includeAny: input.includeAny,
        excludeAny: input.excludeAny,
        matchScope,
        cursor,
      });
      setPreview((current) => cursor && current ? [...current, ...response.entries] : response.entries);
      setPreviewCursor(response.nextCursor);
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(null);
    }
  };

  const save = async () => {
    if (!sourceId || !name.trim() || input.includeAny.length === 0) {
      setError(t("requiredFields"));
      return;
    }
    setBusy("save");
    setError(null);
    try {
      if (rule) {
        await api.rules.patch(rule.id, {
          name: input.name,
          enabled: input.enabled,
          categories: input.categories,
          includeAny: input.includeAny,
          excludeAny: input.excludeAny,
          matchScope: input.matchScope,
        });
      } else {
        await api.rules.create(input);
      }
      onSaved();
      onClose();
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(null);
    }
  };

  const useTradePreset = () => {
    setName("Trade VPS");
    setCategories("trade");
    setIncludeAny("瓦工, 搬瓦工, Bandwagon, BWH, DMIT");
    setExcludeAny("");
    setMatchScope("title");
    setPreview(null);
  };

  return (
    <div className="h-full min-h-0 overflow-y-auto bg-surface-base p-5 text-fg-primary">
      <div className="space-y-4">
        {!rule ? <Button block variant="dashed" icon={<Sparkles />} onClick={useTradePreset}>{t("useTradePreset")}</Button> : null}
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-[1fr_1.4fr_auto]">
          <label>
            <span className="mb-1 block text-xs font-medium text-fg-secondary">{t("source")}</span>
            <Select
              value={sourceId}
              options={sources.filter((source) => !source.archivedAt).map((source) => ({ label: source.name, value: source.id }))}
              disabled={Boolean(rule)}
              onChange={(value: string | number) => { setSourceId(String(value)); setPreview(null); }}
              className="w-full"
            />
          </label>
          <label>
            <span className="mb-1 block text-xs font-medium text-fg-secondary">{t("ruleName")}</span>
            <Input value={name} onChange={(event) => setName(event.target.value)} className="w-full" />
          </label>
          <label className="flex items-end gap-2 pb-1 text-xs text-fg-secondary">
            <Switch checked={enabled} onChange={setEnabled} size="small" /> {t("enabled")}
          </label>
        </div>
        <label className="block">
          <span className="mb-1 block text-xs font-medium text-fg-secondary">{t("categories")}</span>
          <Input value={categories} onChange={(event) => { setCategories(event.target.value); setPreview(null); }} className="w-full" />
        </label>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <label>
            <span className="mb-1 block text-xs font-medium text-fg-secondary">{t("includeAny")}</span>
            <TextArea rows={3} value={includeAny} onChange={(event) => { setIncludeAny(event.target.value); setPreview(null); }} />
          </label>
          <label>
            <span className="mb-1 block text-xs font-medium text-fg-secondary">{t("excludeAny")}</span>
            <TextArea rows={3} value={excludeAny} onChange={(event) => { setExcludeAny(event.target.value); setPreview(null); }} />
          </label>
        </div>
        <label className="block">
          <span className="mb-1 block text-xs font-medium text-fg-secondary">{t("matchScope")}</span>
          <Select
            value={matchScope}
            options={[{ label: t("titleOnly"), value: "title" }, { label: t("titleAndSummary"), value: "title_summary" }]}
            onChange={(value: string | number) => { setMatchScope(String(value) as MatchScope); setPreview(null); }}
            className="w-full"
          />
        </label>
        <Alert type="info" showIcon message={t("ruleLogic")} description={`${t("literalHint")} ${t("previewOnly")}`} />
        {error ? <Alert type="error" showIcon message={`${t("errorPrefix")}${error}`} /> : null}
        {preview ? (
          <div className="max-h-52 overflow-y-auto rounded-md border border-border-base bg-surface-sunken">
            {preview.length === 0 ? <p className="p-4 text-center text-xs text-fg-muted">{t("noPreview")}</p> : preview.map((entry) => (
              <div key={entry.id} className="border-b border-border-subtle px-3 py-2 last:border-b-0">
                <div className="text-xs font-medium text-fg-primary">{entry.title}</div>
                <div className="mt-0.5 text-[10px] text-fg-muted">{entry.sourceName}</div>
              </div>
            ))}
            {previewCursor ? <div className="p-2 text-center"><Button size="small" loading={busy === "preview"} onClick={() => previewRules(previewCursor)}>{t("loadMore")}</Button></div> : null}
          </div>
        ) : null}
        <div className="flex flex-wrap justify-between gap-2 border-t border-border-subtle pt-4">
          <Button icon={<Eye />} loading={busy === "preview"} onClick={() => previewRules()}>{t("previewHistory")}</Button>
          <div className="flex gap-2">
            <Button onClick={onClose}>{t("cancel")}</Button>
            <Button variant="primary" loading={busy === "save"} onClick={save}>{t("save")}</Button>
          </div>
        </div>
      </div>
    </div>
  );
}
