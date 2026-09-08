import { type ShellWindowHandle, useWindowActions } from "@tokimo/sdk";
import { Alert, Badge, Button, Switch, Tag } from "@tokimo/ui";
import { Edit3, Plus, Trash2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../api/client";
import type { CreateRuleReq, DeliveryDto, RuleDto, SourceDto } from "../api/types";
import { DeliveryList } from "./DeliveryList";

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
  const [busyId, setBusyId] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const consumedDraftRef = useRef<CreateRuleReq | null>(null);
  const { openModalWindow } = useWindowActions();

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

  const openEditor = useCallback((rule: RuleDto | null, draft: CreateRuleReq | null = null) => {
    openModalWindow({
      component: async () => {
        const { RuleEditor } = await import("./RuleEditor");
        return {
          default: ({ win }: { win: ShellWindowHandle }) => (
            <RuleEditor
              rule={rule}
              initialDraft={draft}
              sources={sources}
              t={t}
              onClose={win.close}
              onSaved={() => {
                setMessage({ type: "success", text: t("ruleSaved") });
                void load();
              }}
            />
          ),
        };
      },
      title: rule ? t("editRule") : t("addRule"),
      width: 720,
      height: 720,
    });
  }, [load, openModalWindow, sources, t]);

  useEffect(() => {
    if (!initialDraft || consumedDraftRef.current === initialDraft) return;
    consumedDraftRef.current = initialDraft;
    openEditor(null, initialDraft);
    onInitialDraftConsumed();
  }, [initialDraft, onInitialDraftConsumed, openEditor]);

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

  const confirmDelete = (rule: RuleDto) => {
    openModalWindow({
      component: async () => {
        const { ConfirmWindow } = await import("./ConfirmWindow");
        return {
          default: ({ win }: { win: ShellWindowHandle }) => (
            <ConfirmWindow
              message={t("deleteRuleConfirm")}
              confirmLabel={t("delete")}
              cancelLabel={t("cancel")}
              errorPrefix={t("errorPrefix")}
              danger
              onClose={win.close}
              onConfirm={async () => {
                setBusyId(rule.id);
                setMessage(null);
                try {
                  await api.rules.delete(rule.id);
                  await load();
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
      title: t("delete"),
      width: 420,
      height: 240,
    });
  };

  return (
    <div className="h-full overflow-y-auto bg-surface-base px-5 py-5 text-fg-primary">
      <header className="mb-4 flex items-start justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold">{t("rules")}</h1>
          <p className="mt-1 text-xs text-fg-secondary">{t("ruleLogic")}</p>
        </div>
        <Button variant="primary" icon={<Plus />} disabled={sources.every((source) => Boolean(source.archivedAt))} onClick={() => openEditor(null)}>
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
                  <Button size="small" icon={<Edit3 />} onClick={() => openEditor(rule)}>{t("editRule")}</Button>
                  <Button size="small" danger icon={<Trash2 />} loading={busyId === rule.id} onClick={() => confirmDelete(rule)}>{t("delete")}</Button>
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
