import { Tag } from "@tokimo/ui";
import type { SavedViewDto } from "../api/types";

interface SavedViewSummaryProps {
  view: SavedViewDto;
  t: (key: string) => string;
}

export function SavedViewSummary({ view, t }: SavedViewSummaryProps) {
  return (
    <div className="border-b border-border-subtle bg-accent-subtle px-4 py-2.5 text-xs text-fg-secondary">
      <div className="flex flex-wrap items-center gap-2">
        <strong className="text-accent-text">
          {t("savedViewConditions")}: {view.name}
        </strong>
        <span className="text-fg-muted">
          {view.matchScope === "title"
            ? t("titleOnly")
            : t("titleAndSummary")}
        </span>
      </div>
      <div className="mt-2 flex flex-wrap gap-1">
        {view.categories.map((category) => (
          <Tag key={`category:${category}`} size="small">
            {t("category")}: {category}
          </Tag>
        ))}
        {view.includeAny.map((keyword) => (
          <Tag key={`include:${keyword}`} color="processing" size="small">
            + {keyword}
          </Tag>
        ))}
        {view.excludeAny.map((keyword) => (
          <Tag key={`exclude:${keyword}`} color="default" size="small">
            − {keyword}
          </Tag>
        ))}
      </div>
    </div>
  );
}
