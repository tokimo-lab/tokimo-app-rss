import { Alert, Button, Input, Modal } from "@tokimo/ui";
import { CheckCircle, Rss, TestTube2 } from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../api/client";
import type { SourceDto, TestSourceResp } from "../api/types";
import { safeHttpUrl } from "../lib/format";

interface SourceEditorProps {
  open: boolean;
  source: SourceDto | null;
  t: (key: string) => string;
  onClose: () => void;
  onSaved: (source: SourceDto) => void;
}

export function SourceEditor({ open, source, t, onClose, onSaved }: SourceEditorProps) {
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [interval, setInterval] = useState("300");
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [testResult, setTestResult] = useState<TestSourceResp | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setName(source?.name ?? "");
    setUrl(source?.url ?? "");
    setInterval(String(source?.pollIntervalSeconds ?? 300));
    setTestResult(null);
    setError(null);
  }, [open, source]);

  const validate = (): number | null => {
    if (!name.trim() || !url.trim()) {
      setError(t("requiredFields"));
      return null;
    }
    if (!safeHttpUrl(url.trim())) {
      setError(t("urlInvalid"));
      return null;
    }
    const seconds = Number(interval);
    if (!Number.isInteger(seconds) || seconds < 60) {
      setError(t("intervalInvalid"));
      return null;
    }
    return seconds;
  };

  const test = async () => {
    if (!safeHttpUrl(url.trim())) {
      setError(t("urlInvalid"));
      return;
    }
    setTesting(true);
    setError(null);
    try {
      setTestResult(await api.sources.test({ url: url.trim() }));
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setTesting(false);
    }
  };

  const save = async () => {
    const seconds = validate();
    if (seconds === null) return;
    setSaving(true);
    setError(null);
    try {
      const saved = source
        ? await api.sources.patch(source.id, { name: name.trim(), pollIntervalSeconds: seconds })
        : await api.sources.create({ name: name.trim(), url: url.trim(), pollIntervalSeconds: seconds });
      onSaved(saved);
      onClose();
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setSaving(false);
    }
  };

  const applyNodeSeek = () => {
    setName("NodeSeek");
    setUrl("https://rss.nodeseek.com/");
    setInterval("60");
    setTestResult(null);
    setError(null);
  };

  return (
    <Modal
      open={open}
      title={source ? t("editSource") : t("addSource")}
      onCancel={onClose}
      footer={null}
      width={560}
      destroyOnClose
    >
      <div className="space-y-4 text-fg-primary">
        {!source ? (
          <Button variant="dashed" icon={<Rss />} block onClick={applyNodeSeek}>
            {t("nodeSeekPreset")}
          </Button>
        ) : null}
        <label className="block">
          <span className="mb-1 block text-xs font-medium text-fg-secondary">{t("sourceName")}</span>
          <Input value={name} onChange={(event) => setName(event.target.value)} className="w-full" autoFocus />
        </label>
        <label className="block">
          <span className="mb-1 block text-xs font-medium text-fg-secondary">{t("feedUrl")}</span>
          <Input value={url} onChange={(event) => { setUrl(event.target.value); setTestResult(null); }} className="w-full" disabled={Boolean(source)} />
        </label>
        <label className="block">
          <span className="mb-1 block text-xs font-medium text-fg-secondary">{t("interval")}</span>
          <Input type="number" min={60} step={1} value={interval} onChange={(event) => setInterval(event.target.value)} className="w-full" />
          <span className="mt-1 block text-[11px] leading-4 text-fg-muted">{t("intervalHint")}</span>
        </label>
        {error ? <Alert type="error" showIcon message={`${t("errorPrefix")}${error}`} /> : null}
        {testResult ? (
          <Alert
            type="success"
            showIcon
            icon={<CheckCircle />}
            message={t("connectionOk").replace("{title}", testResult.title).replace("{count}", String(testResult.entryCount))}
            description={[
              testResult.ttlMinutes === null ? null : t("feedTtl").replace("{count}", String(testResult.ttlMinutes)),
              testResult.limitedWindow ? t("finiteWindow") : null,
            ].filter((value): value is string => Boolean(value)).join(" · ") || undefined}
          />
        ) : null}
        <div className="flex justify-between gap-2 border-t border-border-subtle pt-4">
          <Button icon={<TestTube2 />} loading={testing} onClick={test}>
            {testing ? t("testing") : t("testConnection")}
          </Button>
          <div className="flex gap-2">
            <Button onClick={onClose}>{t("cancel")}</Button>
            <Button variant="primary" loading={saving} onClick={save}>{t("save")}</Button>
          </div>
        </div>
      </div>
    </Modal>
  );
}
