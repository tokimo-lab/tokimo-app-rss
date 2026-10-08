import { useStandaloneDocumentScroll } from "@tokimo/sdk";
import { type AppRuntimeCtx, type ShellWindowHandle, makeTranslator, useWindowActions } from "@tokimo/sdk";
import { Alert, Button, Select } from "@tokimo/ui";
import { ListPlus, Plus } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api/client";
import type { CreateRuleReq, SavedViewDto, SourceDto } from "./api/types";
import { EntryBrowser } from "./components/EntryBrowser";
import { RulesPanel } from "./components/RulesPanel";
import { type AppView, Sidebar } from "./components/Sidebar";
import { SourcesPanel } from "./components/SourcesPanel";
import { enUS, zhCN } from "./i18n";

interface RssAppProps {
  ctx: AppRuntimeCtx;
}

export function RssApp({ ctx }: RssAppProps) {
  const documentScroll = useStandaloneDocumentScroll();
  const rootRef = useRef<HTMLDivElement>(null);
  const [locale, setLocale] = useState(ctx.locale);
  const t = useMemo(() => makeTranslator({ "zh-CN": zhCN, "en-US": enUS }, locale), [locale]);
  const [narrow, setNarrow] = useState(false);
  const [view, setView] = useState<AppView>({ kind: "entries" });
  const [sources, setSources] = useState<SourceDto[]>([]);
  const [savedViews, setSavedViews] = useState<SavedViewDto[]>([]);
  const [sourceError, setSourceError] = useState<string | null>(null);
  const [ruleDraft, setRuleDraft] = useState<CreateRuleReq | null>(null);
  const { openModalWindow } = useWindowActions();

  useEffect(() => ctx.shell.subscribeLocale(setLocale), [ctx.shell]);

  const loadSources = useCallback(async () => {
    try {
      const [sourceResponse, viewResponse] = await Promise.all([
        api.sources.list(),
        api.views.list(),
      ]);
      setSources(sourceResponse.sources);
      setSavedViews(viewResponse.views);
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

  const openSourceEditor = useCallback((source: SourceDto | null = null) => {
    setView({ kind: "sources" });
    openModalWindow({
      component: async () => {
        const { SourceEditor } = await import("./components/SourceEditor");
        return {
          default: ({ win }: { win: ShellWindowHandle }) => (
            <SourceEditor
              source={source}
              t={t}
              onClose={win.close}
              onSaved={() => { void loadSources(); }}
            />
          ),
        };
      },
      title: source ? t("editSource") : t("addSource"),
      width: 560,
      height: 620,
    });
  }, [loadSources, openModalWindow, t]);

  const openSavedViewEditor = useCallback(
    (source: SourceDto, savedView: SavedViewDto | null = null) => {
      openModalWindow({
        component: async () => {
          const { SavedViewEditor } = await import(
            "./components/SavedViewEditor"
          );
          return {
            default: ({ win }: { win: ShellWindowHandle }) => (
              <SavedViewEditor
                sourceId={source.id}
                sourceName={source.name}
                view={savedView}
                t={t}
                onClose={win.close}
                onSaved={() => {
                  void loadSources();
                }}
              />
            ),
          };
        },
        title: savedView ? t("editSavedView") : t("addSavedView"),
        width: 620,
        height: 620,
      });
    },
    [loadSources, openModalWindow, t],
  );

  const editSavedView = useCallback(
    (savedView: SavedViewDto) => {
      const source = sources.find(
        (candidate) => candidate.id === savedView.sourceId,
      );
      if (source) openSavedViewEditor(source, savedView);
    },
    [openSavedViewEditor, sources],
  );

  const deleteSavedView = useCallback(
    (savedView: SavedViewDto) => {
      openModalWindow({
        component: async () => {
          const { ConfirmWindow } = await import("./components/ConfirmWindow");
          return {
            default: ({ win }: { win: ShellWindowHandle }) => (
              <ConfirmWindow
                message={t("deleteSavedViewConfirm")}
                confirmLabel={t("delete")}
                cancelLabel={t("cancel")}
                errorPrefix={t("errorPrefix")}
                danger
                onClose={win.close}
                onConfirm={async () => {
                  await api.views.delete(savedView.id);
                  setView((current) =>
                    current.kind === "saved-view" &&
                    current.viewId === savedView.id
                      ? { kind: "entries", sourceId: savedView.sourceId }
                      : current,
                  );
                  await loadSources();
                }}
              />
            ),
          };
        },
        title: t("delete"),
        width: 420,
        height: 240,
      });
    },
    [loadSources, openModalWindow, t],
  );

  const activeSavedView =
    view.kind === "saved-view"
      ? savedViews.find((savedView) => savedView.id === view.viewId)
      : undefined;

  const content = view.kind === "entries" || view.kind === "saved-view" ? (
    <EntryBrowser
      key={
        view.kind === "saved-view"
          ? `view:${view.viewId}`
          : `source:${view.sourceId ?? "all"}`
      }
      sources={sources}
      sourceId={view.sourceId}
      viewId={view.kind === "saved-view" ? view.viewId : undefined}
      savedView={activeSavedView}
      locale={locale}
      narrow={narrow}
      t={t}
      onSaveAsRule={(input) => { setRuleDraft(input); setView({ kind: "rules" }); }}
    />
  ) : view.kind === "sources" ? (
    <SourcesPanel
      sources={sources}
      locale={locale}
      t={t}
      onOpenEditor={openSourceEditor}
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
    <div ref={rootRef} className={`flex w-full min-w-0 bg-surface-base text-fg-primary ${documentScroll ? "min-h-dvh" : "h-full"}`}>
      {!narrow ? (
        <Sidebar
          sources={sources}
          savedViews={savedViews}
          view={view}
          t={t}
          onChange={setView}
          onAddSource={() => openSourceEditor()}
          onAddSavedView={(source) => openSavedViewEditor(source)}
          onEditSavedView={editSavedView}
          onDeleteSavedView={deleteSavedView}
        />
      ) : null}
      <main className="app-safe-area flex min-w-0 flex-1 flex-col bg-surface-base">
        {narrow ? (
          <CompactNav
            sources={sources}
            savedViews={savedViews}
            view={view}
            t={t}
            onChange={setView}
            onAddSource={() => openSourceEditor()}
            onAddSavedView={(source) => openSavedViewEditor(source)}
          />
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
  savedViews: SavedViewDto[];
  view: AppView;
  t: (key: string) => string;
  onChange: (view: AppView) => void;
  onAddSource: () => void;
  onAddSavedView: (source: SourceDto) => void;
}

function CompactNav({
  sources,
  savedViews,
  view,
  t,
  onChange,
  onAddSource,
  onAddSavedView,
}: CompactNavProps) {
  const value = view.kind === "entries"
    ? `entries:${view.sourceId ?? "all"}`
    : view.kind === "saved-view"
      ? `view:${view.viewId}`
      : view.kind;
  const options = [
    { label: t("allEntries"), value: "entries:all" },
    ...sources.flatMap((source) => [
      { label: source.name, value: `entries:${source.id}` },
      ...savedViews
        .filter((savedView) => savedView.sourceId === source.id)
        .map((savedView) => ({
          label: `↳ ${source.name} / ${savedView.name}`,
          value: `view:${savedView.id}`,
        })),
    ]),
    { label: t("manageSources"), value: "sources" },
    { label: t("rules"), value: "rules" },
  ];
  const activeSourceId =
    view.kind === "entries" || view.kind === "saved-view"
      ? view.sourceId
      : undefined;
  const activeSource = activeSourceId
    ? sources.find((source) => source.id === activeSourceId)
    : undefined;

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
          else if (selected.startsWith("view:")) {
            const savedView = savedViews.find(
              (candidate) => candidate.id === selected.slice("view:".length),
            );
            if (savedView) {
              onChange({
                kind: "saved-view",
                sourceId: savedView.sourceId,
                viewId: savedView.id,
              });
            }
          }
          else {
            const sourceId = selected.slice("entries:".length);
            onChange({ kind: "entries", sourceId: sourceId === "all" ? undefined : sourceId });
          }
        }}
      />
      {activeSource ? (
        <Button
          shape="circle"
          size="small"
          icon={<ListPlus />}
          aria-label={`${t("addSavedView")}: ${activeSource.name}`}
          title={t("addSavedView")}
          onClick={() => onAddSavedView(activeSource)}
        />
      ) : null}
      <Button variant="primary" shape="circle" size="small" icon={<Plus />} aria-label={t("addSource")} title={t("addSource")} onClick={onAddSource} />
    </div>
  );
}
