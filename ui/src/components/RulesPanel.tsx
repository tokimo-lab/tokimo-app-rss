import { Alert, Badge, Button, Switch, Tag } from "@tokimo/ui";
import { Edit3, Plus, Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { api } from "../api/client";
import type { CreateRuleReq, DeliveryDto, RuleDto, SourceDto } from "../api/types";
import { DeliveryList } from "./DeliveryList";
import { RuleEditor } from "./RuleEditor";

interface RulesPanelProps {
  sources: SourceDto[];
  locale: string;
  t: (key: string) => string;
  initialDraft: CreateRuleReq | null;
  onInitialDraftConsumed: () => void;
}

export function RulesPanel({ sources, locale, t, initialDraft, onInitialDraftConsumed }: RulesPanelProps) {
  const [rules, setRules] = useState<RuleDto[]>([]);
  const [deliveries, setDeliveries] = useState<DeliveryDto[]>([]);
  const [editing, setEditing] = useState<RuleDto | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editorDraft, setEditorDraft] = useState<CreateRuleReq | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);

  const load = useCallback(async () => {
    try {
      const [ruleResponse, deliveryResponse] = await Promise.all([api.rules.list(), api.deliveries.list()]);
      setRules(ruleResponse.rules);
      setDeliveries(deliveryResponse.deliveries);
    } catch (reason: unknown) {
      setMessage({ type: "error", text: `${t("errorPrefix")}${reason instanceof Error ? reason.message : String(reason)}` });
    }
  }, [t]);

  useEffect(() => { void load(); }, [load]);

  useEffect(() => {
    if (!initialDraft) return;
    setEditing(null);
    setEditorDraft(initialDraft);
    setEditorOpen(true);
    onInitialDraftConsumed();
  }, [initialDraft, onInitialDraftConsumed]);

  const mutate = async (id: string, action: () => Promise<unknown>) => {
    setBusyId(id);
    setMessage(null);
    try {
      await action();
      await load();
    } catch (reason: unknown) {
      setMessage({ type: "error", text: `${t("errorPrefix")}${reason instanceof Error ? reason.message : String(reason)}` });
    } finally {
      setBusyId(null);
    }
  };

  const sendTest = async () => {
    setTesting(true);
    setMessage(null);
    try {
      await api.notifications.test();
      setMessage({ type: "success", text: t("testSubmitted") });
      await load();
    } catch (reason: unknown) {
      setMessage({ type: "error", text: `${t("errorPrefix")}${reason instanceof Error ? reason.message : String(reason)}` });
    } finally {
      setTesting(false);
    }
  };

  return (
    <div className="h-full overflow-y-auto bg-surface-base px-5 py-5 text-fg-primary">
      <header className="mb-4 flex items-start justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold">{t("rules")}</h1>
          <p className="mt-1 text-xs text-fg-secondary">{t("ruleLogic")}</p>
        </div>
        <Button variant="primary" icon={<Plus />} disabled={sources.every((source) => Boolean(source.archivedAt))} onClick={() => { setEditing(null); setEditorDraft(null); setEditorOpen(true); }}>
          {t("addRule")}
        </Button>
      </header>
      {message ? <Alert type={message.type} showIcon message={message.text} className="mb-4" /> : null}
      <Alert type="info" showIcon message={t("previewOnly")} className="mb-4" />
      <div className="space-y-3">
        {rules.length === 0 ? (
          <div className="rounded-lg border border-border-base bg-surface-raised p-8 text-center text-sm text-fg-muted">{t("noRules")}</div>
        ) : rules.map((rule) => {
          const source = sources.find((candidate) => candidate.id === rule.sourceId);
          return (
            <section key={rule.id} className="rounded-lg border border-border-base bg-surface-raised p-4 text-fg-on-raised shadow-sm">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <h2 className="truncate text-sm font-semibold text-fg-primary">{rule.name}</h2>
                    <Badge status={rule.enabled ? "success" : "default"} text={rule.enabled ? t("active") : t("paused")} />
                  </div>
                  <p className="mt-1 text-xs text-fg-muted">{source?.name ?? t("unknown")} · {rule.matchScope === "title" ? t("titleOnly") : t("titleAndSummary")}</p>
                </div>
                <div className="flex items-center gap-2">
                  <Switch checked={rule.enabled} loading={busyId === rule.id} size="small" onChange={(enabled) => void mutate(rule.id, () => api.rules.patch(rule.id, { enabled }))} />
                  <Button size="small" icon={<Edit3 />} onClick={() => { setEditing(rule); setEditorDraft(null); setEditorOpen(true); }}>{t("editRule")}</Button>
                  <Button size="small" danger icon={<Trash2 />} loading={busyId === rule.id} onClick={() => {
                    if (window.confirm(t("deleteRuleConfirm"))) void mutate(rule.id, () => api.rules.delete(rule.id));
                  }}>{t("delete")}</Button>
                </div>
              </div>
              <div className="mt-3 space-y-2 text-xs">
                {rule.categories.length > 0 ? <RuleTerms label={t("category")} values={rule.categories} /> : null}
                <RuleTerms label={t("includeAny")} values={rule.includeAny} accent />
                {rule.excludeAny.length > 0 ? <RuleTerms label={t("excludeAny")} values={rule.excludeAny} /> : null}
              </div>
            </section>
          );
        })}
      </div>
      <DeliveryList deliveries={deliveries} locale={locale} testing={testing} t={t} onTest={() => void sendTest()} />
      <RuleEditor
        open={editorOpen}
        rule={editing}
        initialDraft={editorDraft}
        sources={sources}
        t={t}
        onClose={() => { setEditorOpen(false); setEditorDraft(null); }}
        onSaved={() => { setMessage({ type: "success", text: t("ruleSaved") }); void load(); }}
      />
    </div>
  );
}

function RuleTerms({ label, values, accent = false }: { label: string; values: string[]; accent?: boolean }) {
  return (
    <div className="flex items-start gap-2">
      <span className="w-24 shrink-0 pt-0.5 text-fg-muted">{label}</span>
      <div className="flex flex-wrap gap-1">{values.map((value) => <Tag key={value} color={accent ? "processing" : "default"} size="small">{value}</Tag>)}</div>
    </div>
  );
}
