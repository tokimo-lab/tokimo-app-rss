import { type ShellWindowHandle, useWindowActions } from "@tokimo/sdk";
import { Alert, Badge, Button } from "@tokimo/ui";
import { Archive, Edit3, Pause, Play, Plus, RefreshCw, Rss } from "lucide-react";
import { useState } from "react";
import { api } from "../api/client";
import type { SourceDto } from "../api/types";
import { formatDateTime, safeHttpUrl } from "../lib/format";

interface SourcesPanelProps {
  sources: SourceDto[];
  locale: string;
  t: (key: string) => string;
  onOpenEditor: (source?: SourceDto | null) => void;
  onSourcesChanged: () => Promise<void>;
}

export function SourcesPanel({ sources, locale, t, onOpenEditor, onSourcesChanged }: SourcesPanelProps) {
  const [busyId, setBusyId] = useState<string | null>(null);
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const { openModalWindow } = useWindowActions();

  const run = async (sourceId: string, action: () => Promise<unknown>, success: string) => {
    setBusyId(sourceId);
    setMessage(null);
    try {
      await action();
      setMessage({ type: "success", text: success });
      await onSourcesChanged();
    } catch (reason: unknown) {
      setMessage({ type: "error", text: `${t("errorPrefix")}${reason instanceof Error ? reason.message : String(reason)}` });
    } finally {
      setBusyId(null);
    }
  };

  const refreshSource = async (source: SourceDto) => {
    setBusyId(source.id);
    setMessage(null);
    try {
      const response = await api.sources.refresh(source.id);
      setMessage({ type: "success", text: response.status === "started" ? t("refreshStarted") : t("refreshRunning") });
      await onSourcesChanged();
    } catch (reason: unknown) {
      setMessage({ type: "error", text: `${t("errorPrefix")}${reason instanceof Error ? reason.message : String(reason)}` });
    } finally {
      setBusyId(null);
    }
  };

  const confirmArchive = (source: SourceDto) => {
    openModalWindow({
      component: async () => {
        const { ConfirmWindow } = await import("./ConfirmWindow");
        return {
          default: ({ win }: { win: ShellWindowHandle }) => (
            <ConfirmWindow
              message={t("archiveConfirm")}
              confirmLabel={t("archive")}
              cancelLabel={t("cancel")}
              errorPrefix={t("errorPrefix")}
              danger
              onClose={win.close}
              onConfirm={async () => {
                setBusyId(source.id);
                setMessage(null);
                try {
                  await api.sources.patch(source.id, { archived: true });
                  setMessage({ type: "success", text: t("archived") });
                  await onSourcesChanged();
                } catch (reason: unknown) {
                  setMessage({ type: "error", text: `${t("errorPrefix")}${reason instanceof Error ? reason.message : String(reason)}` });
                  throw reason;
                } finally {
                  setBusyId(null);
                }
              }}
            />
          ),
        };
      },
      title: t("archive"),
      width: 420,
      height: 240,
    });
  };

  return (
    <div className="h-full overflow-y-auto bg-surface-base px-5 py-5 text-fg-primary">
      <header className="mb-4 flex items-start justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold">{t("manageSources")}</h1>
          <p className="mt-1 max-w-2xl text-xs leading-5 text-fg-secondary">{t("coverageBody")}</p>
        </div>
        <Button variant="primary" icon={<Plus />} onClick={() => onOpenEditor()}>
          {t("addSource")}
        </Button>
      </header>
      {message ? <Alert type={message.type} showIcon message={message.text} className="mb-4" /> : null}
      <div className="space-y-3">
        {sources.map((source) => {
          const archived = Boolean(source.archivedAt);
          const status = archived ? "archived" : source.lastError ? "failed" : source.enabled ? "active" : "paused";
          const statusKind = status === "active" ? "success" : status === "failed" ? "error" : status === "paused" ? "warning" : "default";
          const link = safeHttpUrl(source.url);
          return (
            <section key={source.id} className="rounded-lg border border-border-base bg-surface-raised p-4 text-fg-on-raised shadow-sm">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <Rss className="h-4 w-4 shrink-0 text-accent-text" />
                    <h2 className="truncate text-sm font-semibold">{source.name}</h2>
                    <Badge status={statusKind} text={t(status)} />
                  </div>
                  {link ? <a href={link} target="_blank" rel="noreferrer" className="mt-1 block truncate text-xs text-accent-text hover:underline">{source.url}</a> : <p className="mt-1 truncate text-xs text-fg-muted">{source.url}</p>}
                </div>
                <div className="flex flex-wrap gap-1.5">
                  <Button size="small" icon={<Edit3 />} onClick={() => onOpenEditor(source)}>{t("editSource")}</Button>
                  {!archived ? (
                    <Button
                      size="small"
                      icon={source.enabled ? <Pause /> : <Play />}
                      loading={busyId === source.id}
                      onClick={() => run(source.id, () => api.sources.patch(source.id, { enabled: !source.enabled }), source.enabled ? t("paused") : t("active"))}
                    >
                      {source.enabled ? t("pause") : t("resume")}
                    </Button>
                  ) : null}
                  {!archived ? (
                    <Button
                      size="small"
                      icon={<RefreshCw />}
                      loading={busyId === source.id}
                      onClick={() => void refreshSource(source)}
                    >
                      {t("manualRefresh")}
                    </Button>
                  ) : null}
                  {!archived ? (
                    <Button
                      size="small"
                      danger
                      icon={<Archive />}
                      loading={busyId === source.id}
                      onClick={() => confirmArchive(source)}
                    >
                      {t("archive")}
                    </Button>
                  ) : null}
                </div>
              </div>
              <dl className="mt-4 grid gap-3 text-xs sm:grid-cols-2 xl:grid-cols-4">
                <StatusItem label={t("entriesCollected").replace("{count}", String(source.entryCount))} value={`${t("interval")}: ${source.pollIntervalSeconds}s`} />
                <StatusItem label={t("lastSuccess")} value={source.lastSuccessAt ? formatDateTime(source.lastSuccessAt, locale) : t("neverSucceeded")} />
                <StatusItem label={t("nextPoll")} value={formatDateTime(source.nextPollAt, locale)} />
                <StatusItem label={t("collectionStarted")} value={formatDateTime(source.initializedAt, locale)} />
              </dl>
              {source.lastError ? <Alert type="error" showIcon message={source.lastError} className="mt-3" /> : null}
              {source.gapSuspected ? <Alert type="warning" showIcon message={t("gapSuspected")} className="mt-3" /> : null}
            </section>
          );
        })}
      </div>
    </div>
  );
}

function StatusItem({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-[10px] text-fg-muted">{label}</dt>
      <dd className="mt-0.5 truncate font-medium text-fg-secondary" title={value}>{value}</dd>
    </div>
  );
}
