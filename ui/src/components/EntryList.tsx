import { Button, cn, Tag } from "@tokimo/ui";
import { ChevronRight, Inbox } from "lucide-react";
import type { EntryDto } from "../api/types";
import { formatDateTime } from "../lib/format";

interface EntryListProps {
  entries: EntryDto[];
  selectedId?: string;
  locale: string;
  loading: boolean;
  nextCursor: string | null;
  t: (key: string) => string;
  onSelect: (entry: EntryDto) => void;
  onLoadMore: () => void;
}

export function EntryList({ entries, selectedId, locale, loading, nextCursor, t, onSelect, onLoadMore }: EntryListProps) {
  if (!loading && entries.length === 0) {
    return (
      <div className="grid h-full place-items-center px-8 text-center">
        <div>
          <Inbox className="mx-auto mb-3 h-8 w-8 text-fg-muted" />
          <p className="text-sm text-fg-secondary">{t("emptyEntries")}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="divide-y divide-border-subtle">
      {entries.map((entry) => (
        <button
          key={entry.id}
          type="button"
          onClick={() => onSelect(entry)}
          className={cn(
            "group flex w-full cursor-pointer items-start gap-2 px-4 py-3 text-left transition-colors focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-accent",
            entry.id === selectedId
              ? "bg-accent-subtle"
              : "hover:bg-fill-tertiary",
          )}
        >
          <div className="min-w-0 flex-1">
            <div className="mb-1 flex items-center gap-2 text-[10px] text-fg-muted">
              <span className="truncate font-medium text-accent-text">{entry.sourceName}</span>
              <span>·</span>
              <span className="shrink-0">
                {formatDateTime(entry.publishedAt ?? entry.firstSeenAt, locale)}
              </span>
            </div>
            <h3 className="line-clamp-2 text-sm font-medium leading-5 text-fg-primary">
              {entry.title}
            </h3>
            {entry.summary ? (
              <p className="mt-1 line-clamp-2 text-xs leading-4 text-fg-secondary">
                {entry.summary}
              </p>
            ) : null}
            {entry.categories.length > 0 ? (
              <div className="mt-2 flex flex-wrap gap-1">
                {entry.categories.slice(0, 3).map((category) => (
                  <Tag key={category} size="small" bordered={false}>
                    {category}
                  </Tag>
                ))}
              </div>
            ) : null}
          </div>
          <ChevronRight className="mt-5 h-4 w-4 shrink-0 text-fg-muted transition-transform group-hover:translate-x-0.5" />
        </button>
      ))}
      {loading ? <div className="px-4 py-6 text-center text-xs text-fg-muted">{t("loading")}</div> : null}
      {!loading && nextCursor ? (
        <div className="p-3 text-center">
          <Button size="small" onClick={onLoadMore}>{t("loadMore")}</Button>
        </div>
      ) : null}
    </div>
  );
}
