import { type AppRuntimeCtx, makeTranslator } from "@tokimo/sdk";
import { Alert, Button, Select } from "@tokimo/ui";
import { Plus } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api/client";
import type { CreateRuleReq, SourceDto } from "./api/types";
import { EntryBrowser } from "./components/EntryBrowser";
import { RulesPanel } from "./components/RulesPanel";
import { type AppView, Sidebar } from "./components/Sidebar";
import { SourcesPanel } from "./components/SourcesPanel";
import { enUS, zhCN } from "./i18n";

interface RssAppProps {
  ctx: AppRuntimeCtx;
}

export function RssApp({ ctx }: RssAppProps) {
  const rootRef = useRef<HTMLDivElement>(null);
  const [locale, setLocale] = useState(ctx.locale);
  const t = useMemo(() => makeTranslator({ "zh-CN": zhCN, "en-US": enUS }, locale), [locale]);
  const [narrow, setNarrow] = useState(false);
  const [view, setView] = useState<AppView>({ kind: "entries" });
  const [sources, setSources] = useState<SourceDto[]>([]);
  const [sourceEditorOpen, setSourceEditorOpen] = useState(false);
  const [sourceError, setSourceError] = useState<string | null>(null);
  const [ruleDraft, setRuleDraft] = useState<CreateRuleReq | null>(null);

  useEffect(() => ctx.shell.subscribeLocale(setLocale), [ctx.shell]);

  const loadSources = useCallback(async () => {
    try {
      const response = await api.sources.list();
      setSources(response.sources);
      setSourceError(null);
    } catch (reason: unknown) {
      setSourceError(reason instanceof Error ? reason.message : String(reason));
    }
  }, []);

  useEffect(() => { void loadSources(); }, [loadSources]);

  useEffect(() => {
    const element = rootRef.current;
    if (!element) return;
    const update = () => setNarrow(element.clientWidth < 760);
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const openSourceEditor = () => {
    setView({ kind: "sources" });
    setSourceEditorOpen(true);
  };

  const content = view.kind === "entries" ? (
    <EntryBrowser
      sources={sources}
      sourceId={view.sourceId}
      locale={locale}
      narrow={narrow}
      t={t}
      onSaveAsRule={(input) => { setRuleDraft(input); setView({ kind: "rules" }); }}
    />
  ) : view.kind === "sources" ? (
    <SourcesPanel
      sources={sources}
      locale={locale}
      editorOpen={sourceEditorOpen}
      t={t}
      onEditorOpenChange={setSourceEditorOpen}
      onSourcesChanged={loadSources}
    />
  ) : (
    <RulesPanel
      sources={sources}
      locale={locale}
      t={t}
      initialDraft={ruleDraft}
      onInitialDraftConsumed={() => setRuleDraft(null)}
    />
  );

  return (
    <div ref={rootRef} className="flex h-full w-full min-w-0 bg-surface-base text-fg-primary">
      {!narrow ? (
        <Sidebar sources={sources} view={view} t={t} onChange={setView} onAddSource={openSourceEditor} />
      ) : null}
      <main className="flex min-w-0 flex-1 flex-col">
        {narrow ? (
          <CompactNav sources={sources} view={view} t={t} onChange={setView} onAddSource={openSourceEditor} />
        ) : null}
        {sourceError ? (
          <Alert type="error" banner showIcon message={`${t("errorPrefix")}${sourceError}`} action={<Button size="small" onClick={() => void loadSources()}>{t("retry")}</Button>} />
        ) : null}
        <div className="min-h-0 flex-1">{content}</div>
      </main>
    </div>
  );
}

interface CompactNavProps {
  sources: SourceDto[];
  view: AppView;
  t: (key: string) => string;
  onChange: (view: AppView) => void;
  onAddSource: () => void;
}

function CompactNav({ sources, view, t, onChange, onAddSource }: CompactNavProps) {
  const value = view.kind === "entries" ? `entries:${view.sourceId ?? "all"}` : view.kind;
  const options = [
    { label: t("allEntries"), value: "entries:all" },
    ...sources.map((source) => ({ label: source.name, value: `entries:${source.id}` })),
    { label: t("manageSources"), value: "sources" },
    { label: t("rules"), value: "rules" },
  ];

  return (
    <div className="flex items-center gap-2 border-b border-border-subtle bg-surface-sidebar px-2 py-2">
      <Select
        value={value}
        options={options}
        className="min-w-0 flex-1"
        onChange={(next: string | number) => {
          const selected = String(next);
          if (selected === "sources") onChange({ kind: "sources" });
          else if (selected === "rules") onChange({ kind: "rules" });
          else {
            const sourceId = selected.slice("entries:".length);
            onChange({ kind: "entries", sourceId: sourceId === "all" ? undefined : sourceId });
          }
        }}
      />
      <Button variant="primary" shape="circle" size="small" icon={<Plus />} aria-label={t("addSource")} title={t("addSource")} onClick={onAddSource} />
    </div>
  );
}
