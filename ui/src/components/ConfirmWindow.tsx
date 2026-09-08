import { Alert, Button } from "@tokimo/ui";
import { useState } from "react";

interface ConfirmWindowProps {
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  errorPrefix: string;
  danger?: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
}

export function ConfirmWindow({
  message,
  confirmLabel,
  cancelLabel,
  errorPrefix,
  danger = false,
  onClose,
  onConfirm,
}: ConfirmWindowProps) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const confirm = async () => {
    setLoading(true);
    setError(null);
    try {
      await onConfirm();
      onClose();
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col bg-surface-base p-5 text-fg-primary">
      <p className="min-h-0 flex-1 overflow-y-auto text-sm leading-6 text-fg-secondary">{message}</p>
      {error ? <Alert type="error" showIcon message={`${errorPrefix}${error}`} className="mt-3" /> : null}
      <div className="mt-4 flex justify-end gap-2 border-t border-border-subtle pt-4">
        <Button disabled={loading} onClick={onClose}>{cancelLabel}</Button>
        <Button variant="primary" danger={danger} loading={loading} onClick={() => void confirm()}>
          {confirmLabel}
        </Button>
      </div>
    </div>
  );
}
