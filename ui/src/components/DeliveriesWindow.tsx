import { Alert } from "@tokimo/ui";
import { useCallback, useEffect, useState } from "react";
import { api } from "../api/client";
import type { DeliveryDto } from "../api/types";
import { DeliveryList } from "./DeliveryList";

interface DeliveriesWindowProps {
  locale: string;
  t: (key: string) => string;
}

export function DeliveriesWindow({ locale, t }: DeliveriesWindowProps) {
  const [deliveries, setDeliveries] = useState<DeliveryDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [testing, setTesting] = useState(false);
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const response = await api.deliveries.list();
      setDeliveries(response.deliveries);
    } catch (reason: unknown) {
      setMessage({
        type: "error",
        text: `${t("errorPrefix")}${reason instanceof Error ? reason.message : String(reason)}`,
      });
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  const sendTest = async () => {
    setTesting(true);
    setMessage(null);
    try {
      await api.notifications.test();
      setMessage({ type: "success", text: t("testSubmitted") });
      await load();
    } catch (reason: unknown) {
      setMessage({
        type: "error",
        text: `${t("errorPrefix")}${reason instanceof Error ? reason.message : String(reason)}`,
      });
    } finally {
      setTesting(false);
    }
  };

  return (
    <div className="h-full min-h-0 overflow-y-auto bg-surface-base p-5 text-fg-primary">
      {message ? <Alert type={message.type} showIcon message={message.text} className="mb-4" /> : null}
      {loading && deliveries.length === 0 ? (
        <p className="py-8 text-center text-sm text-fg-muted">{t("loading")}</p>
      ) : (
        <DeliveryList
          deliveries={deliveries}
          locale={locale}
          testing={testing}
          t={t}
          onTest={() => void sendTest()}
        />
      )}
    </div>
  );
}
