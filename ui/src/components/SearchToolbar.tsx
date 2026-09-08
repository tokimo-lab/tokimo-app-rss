import { Button, Input, Select } from "@tokimo/ui";
import { RotateCcw, Search } from "lucide-react";
import type { ReactNode } from "react";
import type { CreateRuleReq, EntriesQuery, MatchScope, SourceDto } from "../api/types";

export type SearchDraft = Omit<EntriesQuery, "cursor">;

interface SearchToolbarProps {
  draft: SearchDraft;
  sources: SourceDto[];
  busy: boolean;
  t: (key: string) => string;
  onChange: (next: SearchDraft) => void;
  onSubmit: () => void;
  onReset: () => void;
  onSaveAsRule: (input: CreateRuleReq) => void;
}

export function SearchToolbar({ draft, sources, busy, t, onChange, onSubmit, onReset, onSaveAsRule }: SearchToolbarProps) {
  const terms = (draft.q ?? "").trim().split(/\s+/).filter(Boolean);
  const canSaveAsRule = Boolean(draft.sourceId) && terms.length === 1 && !draft.author && !draft.publishedFrom && !draft.publishedTo;
  const sourceOptions = [
    { label: t("allSources"), value: "" },
    ...sources.map((source) => ({ label: source.name, value: source.id })),
  ];

  return (
    <form
      className="border-b border-border-subtle bg-surface-raised px-4 py-3 text-fg-on-raised"
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit();
      }}
    >
      <div className="flex flex-wrap items-end gap-2">
        <Field label={t("keyword")} className="min-w-52 flex-[2]">
          <Input
            value={draft.q ?? ""}
            placeholder={t("keywordHint")}
            prefix={<Search />}
            onChange={(event) => onChange({ ...draft, q: event.target.value })}
            className="w-full"
          />
        </Field>
        <Field label={t("source")} className="min-w-36 flex-1">
          <Select
            value={draft.sourceId ?? ""}
            options={sourceOptions}
            onChange={(value: string | number) =>
              onChange({ ...draft, sourceId: String(value) || undefined })
            }
            className="w-full"
          />
        </Field>
        <Field label={t("matchScope")} className="min-w-36 flex-1">
          <Select
            value={draft.matchScope ?? "title"}
            options={[
              { label: t("titleOnly"), value: "title" },
              { label: t("titleAndSummary"), value: "title_summary" },
            ]}
            onChange={(value: string | number) =>
              onChange({ ...draft, matchScope: String(value) as MatchScope })
            }
            className="w-full"
          />
        </Field>
        <Button htmlType="submit" variant="primary" loading={busy} icon={<Search />}>
          {t("search")}
        </Button>
        <Button htmlType="button" variant="text" icon={<RotateCcw />} onClick={onReset}>
          {t("reset")}
        </Button>
        {canSaveAsRule && draft.sourceId ? (
          <Button
            htmlType="button"
            variant="dashed"
            onClick={() => onSaveAsRule({
              sourceId: draft.sourceId as string,
              name: terms[0],
              enabled: true,
              categories: draft.category?.trim() ? [draft.category.trim()] : [],
              includeAny: terms,
              excludeAny: [],
              matchScope: draft.matchScope ?? "title",
            })}
          >
            {t("saveAsRule")}
          </Button>
        ) : null}
      </div>
      <div className="mt-2 grid grid-cols-2 gap-2 lg:grid-cols-4">
        <Input
          value={draft.category ?? ""}
          placeholder={t("category")}
          aria-label={t("category")}
          onChange={(event) => onChange({ ...draft, category: event.target.value })}
        />
        <Input
          value={draft.author ?? ""}
          placeholder={t("author")}
          aria-label={t("author")}
          onChange={(event) => onChange({ ...draft, author: event.target.value })}
        />
        <Input
          type="date"
          value={draft.publishedFrom ?? ""}
          aria-label={t("publishedFrom")}
          title={t("publishedFrom")}
          onChange={(event) => onChange({ ...draft, publishedFrom: event.target.value })}
        />
        <Input
          type="date"
          value={draft.publishedTo ?? ""}
          aria-label={t("publishedTo")}
          title={t("publishedTo")}
          onChange={(event) => onChange({ ...draft, publishedTo: event.target.value })}
        />
      </div>
    </form>
  );
}

function Field({ label, className, children }: { label: string; className?: string; children: ReactNode }) {
  return (
    <label className={className}>
      <span className="mb-1 block text-[10px] font-medium text-fg-muted">{label}</span>
      {children}
    </label>
  );
}
