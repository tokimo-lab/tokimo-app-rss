import { Alert, Button } from "@tokimo/ui";
import { RefreshCw } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../api/client";
import type {
  CreateRuleReq,
  EntriesQuery,
  EntryDto,
  SavedViewDto,
  SourceDto,
} from "../api/types";
import { formatDateTime } from "../lib/format";
import { EntryDetail } from "./EntryDetail";
import { EntryList } from "./EntryList";
import { type SearchDraft, SearchToolbar } from "./SearchToolbar";
import { SavedViewSummary } from "./SavedViewSummary";

interface EntryBrowserProps {
  sources: SourceDto[];
  sourceId?: string;
  viewId?: string;
  savedView?: SavedViewDto;
  locale: string;
  narrow: boolean;
  t: (key: string) => string;
  onSaveAsRule: (input: CreateRuleReq) => void;
}

function initialQuery(sourceId?: string, viewId?: string): SearchDraft {
  return { sourceId, viewId, matchScope: "title" };
}

export function EntryBrowser({
  sources,
  sourceId,
  viewId,
  savedView,
  locale,
  narrow,
  t,
  onSaveAsRule,
}: EntryBrowserProps) {
  const [draft, setDraft] = useState<SearchDraft>(() =>
    initialQuery(sourceId, viewId),
  );
  const [query, setQuery] = useState<SearchDraft>(() =>
    initialQuery(sourceId, viewId),
  );
  const [entries, setEntries] = useState<EntryDto[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [selected, setSelected] = useState<EntryDto | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);
  const detailRequest = useRef(0);

  useEffect(() => {
    const next = initialQuery(sourceId, viewId);
    setDraft(next);
    setQuery(next);
    setSelected(null);
  }, [sourceId, viewId]);

  useEffect(() => {
    let current = true;
    detailRequest.current += 1;
    setSelected(null);
    setDetailLoading(false);
    setLoading(true);
    setError(null);
    setEntries([]);
    setNextCursor(null);
    api.entries.list(query)
      .then((response) => {
        if (!current) return;
        setEntries(response.entries);
        setNextCursor(response.nextCursor);
      })
      .catch((reason: unknown) => {
        if (current) setError(reason instanceof Error ? reason.message : String(reason));
      })
      .finally(() => {
        if (current) setLoading(false);
      });
    return () => { current = false; };
  }, [query, revision]);

  useEffect(() => {
    if (!narrow && !selected && entries[0]) setSelected(entries[0]);
  }, [entries, narrow, selected]);

  const selectEntry = useCallback((entry: EntryDto) => {
    const requestId = detailRequest.current + 1;
    detailRequest.current = requestId;
    setSelected(entry);
    setDetailLoading(true);
    api.entries.get(entry.id)
      .then((detail) => {
        if (detailRequest.current === requestId) setSelected(detail);
      })
      .catch(() => undefined)
      .finally(() => {
        if (detailRequest.current === requestId) setDetailLoading(false);
      });
  }, []);

  const loadMore = useCallback(() => {
    if (!nextCursor || loading) return;
    setLoading(true);
    const paged: EntriesQuery = { ...query, cursor: nextCursor };
    api.entries.list(paged)
      .then((response) => {
        setEntries((current) => [...current, ...response.entries]);
        setNextCursor(response.nextCursor);
      })
      .catch((reason: unknown) => setError(reason instanceof Error ? reason.message : String(reason)))
      .finally(() => setLoading(false));
  }, [loading, nextCursor, query]);

  const selectedSource = sourceId ? sources.find((source) => source.id === sourceId) : undefined;
  const coverageDate = selectedSource?.initializedAt ??
    sources.map((source) => source.initializedAt).filter((value): value is string => Boolean(value)).sort()[0] ?? null;

  const listPane = (
    <div className="min-h-0 flex-1 overflow-y-auto bg-surface-base">
      <EntryList
        entries={entries}
        selectedId={selected?.id}
        locale={locale}
        loading={loading}
        nextCursor={nextCursor}
        t={t}
        onSelect={selectEntry}
        onLoadMore={loadMore}
      />
    </div>
  );

  return (
    <div className="flex h-full min-h-0 flex-col bg-surface-base text-fg-primary">
      <SearchToolbar
        draft={draft}
        sources={sources}
        busy={loading}
        sourceLocked={Boolean(viewId)}
        t={t}
        onChange={setDraft}
        onSubmit={() => { setSelected(null); setQuery({ ...draft }); }}
        onReset={() => {
          const next = initialQuery(sourceId, viewId);
          setDraft(next);
          setQuery(next);
          setSelected(null);
        }}
        onSaveAsRule={onSaveAsRule}
      />
      {savedView ? <SavedViewSummary view={savedView} t={t} /> : null}
      <div className="border-b border-border-subtle bg-state-info-subtle px-4 py-2 text-[11px] text-state-info-text">
        <strong>{t("coverageTitle")}：</strong> {t("coverageBody")}
        {coverageDate ? ` ${t("collectionStarted")} ${formatDateTime(coverageDate, locale)}。` : ""}
      </div>
      {error ? (
        <Alert
          type="error"
          showIcon
          message={`${t("errorPrefix")}${error}`}
          action={<Button size="small" icon={<RefreshCw />} onClick={() => setRevision((value) => value + 1)}>{t("retry")}</Button>}
          className="m-3"
        />
      ) : null}
      <div className="flex min-h-0 flex-1">
        {narrow ? (
          selected ? (
            <div className="min-w-0 flex-1"><EntryDetail entry={selected} locale={locale} narrow loading={detailLoading} t={t} onBack={() => setSelected(null)} /></div>
          ) : listPane
        ) : (
          <>
            <div className="flex w-[42%] min-w-72 flex-col border-r border-border-subtle">{listPane}</div>
            <div className="min-w-0 flex-1"><EntryDetail entry={selected} locale={locale} narrow={false} loading={detailLoading} t={t} onBack={() => setSelected(null)} /></div>
          </>
        )}
      </div>
    </div>
  );
}
