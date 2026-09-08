import { Alert, Button, Input, Select, TextArea } from "@tokimo/ui";
import { useMemo, useState } from "react";
import { api } from "../api/client";
import type {
  CreateSavedViewReq,
  MatchScope,
  SavedViewDto,
} from "../api/types";
import { joinTerms, splitTerms } from "../lib/format";

interface SavedViewEditorProps {
  sourceId: string;
  sourceName: string;
  view: SavedViewDto | null;
  t: (key: string) => string;
  onClose: () => void;
  onSaved: () => void;
}

export function SavedViewEditor({
  sourceId,
  sourceName,
  view,
  t,
  onClose,
  onSaved,
}: SavedViewEditorProps) {
  const [name, setName] = useState(view?.name ?? "");
  const [categories, setCategories] = useState(joinTerms(view?.categories ?? []));
  const [includeAny, setIncludeAny] = useState(joinTerms(view?.includeAny ?? []));
  const [excludeAny, setExcludeAny] = useState(joinTerms(view?.excludeAny ?? []));
  const [matchScope, setMatchScope] = useState<MatchScope>(
    view?.matchScope ?? "title",
  );
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const input = useMemo<CreateSavedViewReq>(
    () => ({
      sourceId,
      name: name.trim(),
      categories: splitTerms(categories),
      includeAny: splitTerms(includeAny),
      excludeAny: splitTerms(excludeAny),
      matchScope,
    }),
    [categories, excludeAny, includeAny, matchScope, name, sourceId],
  );

  const save = async () => {
    if (
      !input.name ||
      (input.categories.length === 0 && input.includeAny.length === 0)
    ) {
      setError(t("savedViewRequired"));
      return;
    }
    setSaving(true);
    setError(null);
    try {
      if (view) {
        await api.views.patch(view.id, {
          name: input.name,
          categories: input.categories,
          includeAny: input.includeAny,
          excludeAny: input.excludeAny,
          matchScope: input.matchScope,
        });
      } else {
        await api.views.create(input);
      }
      onSaved();
      onClose();
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="h-full min-h-0 overflow-y-auto bg-surface-base p-5 text-fg-primary">
      <div className="space-y-4">
        <div className="rounded-md border border-border-subtle bg-surface-raised px-3 py-2 text-xs text-fg-secondary">
          {t("source")}: <strong className="text-fg-primary">{sourceName}</strong>
        </div>
        <label className="block">
          <span className="mb-1 block text-xs font-medium text-fg-secondary">
            {t("savedViewName")}
          </span>
          <Input
            value={name}
            onChange={(event) => setName(event.target.value)}
            className="w-full"
            autoFocus
          />
        </label>
        <label className="block">
          <span className="mb-1 block text-xs font-medium text-fg-secondary">
            {t("categories")}
          </span>
          <Input
            value={categories}
            onChange={(event) => setCategories(event.target.value)}
            className="w-full"
          />
        </label>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <label>
            <span className="mb-1 block text-xs font-medium text-fg-secondary">
              {t("includeAny")}
            </span>
            <TextArea
              rows={4}
              value={includeAny}
              onChange={(event) => setIncludeAny(event.target.value)}
            />
          </label>
          <label>
            <span className="mb-1 block text-xs font-medium text-fg-secondary">
              {t("excludeAny")}
            </span>
            <TextArea
              rows={4}
              value={excludeAny}
              onChange={(event) => setExcludeAny(event.target.value)}
            />
          </label>
        </div>
        <label className="block">
          <span className="mb-1 block text-xs font-medium text-fg-secondary">
            {t("matchScope")}
          </span>
          <Select
            value={matchScope}
            options={[
              { label: t("titleOnly"), value: "title" },
              { label: t("titleAndSummary"), value: "title_summary" },
            ]}
            onChange={(value: string | number) =>
              setMatchScope(String(value) as MatchScope)
            }
            className="w-full"
          />
        </label>
        <Alert
          type="info"
          showIcon
          message={t("savedViewHint")}
          description={t("literalHint")}
        />
        {error ? (
          <Alert
            type="error"
            showIcon
            message={`${t("errorPrefix")}${error}`}
          />
        ) : null}
        <div className="flex justify-end gap-2 border-t border-border-subtle pt-4">
          <Button disabled={saving} onClick={onClose}>
            {t("cancel")}
          </Button>
          <Button
            variant="primary"
            loading={saving}
            onClick={() => void save()}
          >
            {t("save")}
          </Button>
        </div>
      </div>
    </div>
  );
}
