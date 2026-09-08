import { Alert, Badge, Button } from "@tokimo/ui";
import { Bell } from "lucide-react";
import type { DeliveryDto, DeliveryStatus } from "../api/types";
import { formatDateTime, safeHttpUrl } from "../lib/format";

interface DeliveryListProps {
  deliveries: DeliveryDto[];
  locale: string;
  testing: boolean;
  t: (key: string) => string;
  onTest: () => void;
}

const badgeStatus: Record<DeliveryStatus, "default" | "processing" | "success" | "warning"> = {
  pending: "default",
  submitting: "processing",
  accepted: "success",
  cancelled: "warning",
};

export function DeliveryList({ deliveries, locale, testing, t, onTest }: DeliveryListProps) {
  return (
    <section>
      <div className="mb-3 flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold text-fg-primary">{t("deliveries")}</h2>
        <Button size="small" icon={<Bell />} loading={testing} onClick={onTest}>{t("sendTest")}</Button>
      </div>
      <Alert type="info" showIcon message={t("deliveryNotice")} className="mb-3" />
      <div className="overflow-hidden rounded-lg border border-border-base bg-surface-raised text-fg-on-raised">
        {deliveries.length === 0 ? (
          <p className="p-5 text-center text-xs text-fg-muted">{t("noDeliveries")}</p>
        ) : deliveries.map((delivery) => {
          const entryUrl = safeHttpUrl(delivery.entryUrl);
          return (
            <div key={delivery.id} className="border-b border-border-subtle px-4 py-3 last:border-b-0">
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div className="min-w-0 flex-1">
                  {entryUrl ? (
                    <a
                      href={entryUrl}
                      target="_blank"
                      rel="noreferrer"
                      className="block cursor-pointer truncate rounded-sm text-sm font-medium text-fg-primary hover:text-accent-text focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-border-accent"
                    >
                      {delivery.entryTitle}
                    </a>
                  ) : (
                    <div className="truncate text-sm font-medium text-fg-primary">{delivery.entryTitle}</div>
                  )}
                  <div className="mt-1 text-[10px] text-fg-muted">
                    {delivery.sourceName} · {formatDateTime(delivery.createdAt, locale)} · {t("attempts").replace("{count}", String(delivery.attempts))}
                  </div>
                </div>
                <Badge status={badgeStatus[delivery.status]} text={t(`delivery${delivery.status[0].toUpperCase()}${delivery.status.slice(1)}`)} />
              </div>
              {delivery.matchedRules.length > 0 ? <p className="mt-1 text-[11px] text-fg-secondary">{delivery.matchedRules.join(" · ")}</p> : null}
              {delivery.lastError ? <p className="mt-1 text-[11px] text-state-danger-text">{delivery.lastError}</p> : null}
            </div>
          );
        })}
      </div>
    </section>
  );
}
