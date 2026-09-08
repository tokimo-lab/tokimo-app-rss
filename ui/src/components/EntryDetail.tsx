import { Button, Tag } from "@tokimo/ui";
import { ArrowLeft, ExternalLink, FileText } from "lucide-react";
import type { EntryDto } from "../api/types";
import { formatDateTime, safeHttpUrl } from "../lib/format";

interface EntryDetailProps {
  entry: EntryDto | null;
  locale: string;
  narrow: boolean;
  loading?: boolean;
  t: (key: string) => string;
  onBack: () => void;
}

export function EntryDetail({ entry, locale, narrow, loading, t, onBack }: EntryDetailProps) {
  if (loading) {
    return <div className="grid h-full place-items-center text-sm text-fg-muted">{t("loading")}</div>;
  }
  if (!entry) {
    return (
      <div className="grid h-full place-items-center px-8 text-center">
        <div>
          <FileText className="mx-auto mb-3 h-8 w-8 text-fg-muted" />
          <p className="text-sm text-fg-secondary">{t("selectEntry")}</p>
        </div>
      </div>
    );
  }

  const link = safeHttpUrl(entry.url);
  return (
    <article className="h-full overflow-y-auto px-6 py-5 text-fg-primary">
      {narrow ? (
        <Button variant="text" size="small" icon={<ArrowLeft />} className="mb-3" onClick={onBack}>
          {t("backToList")}
        </Button>
      ) : null}
      <div className="mb-2 text-xs font-medium text-accent-text">{entry.sourceName}</div>
      <h1 className="text-xl font-semibold leading-7">{entry.title}</h1>
      <dl className="mt-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-xs">
        <dt className="text-fg-muted">{t("publishedAt")}</dt>
        <dd>{formatDateTime(entry.publishedAt, locale)}</dd>
        <dt className="text-fg-muted">{t("author")}</dt>
        <dd>{entry.author ?? t("unknown")}</dd>
        <dt className="text-fg-muted">{t("collectedAt")}</dt>
        <dd>{formatDateTime(entry.firstSeenAt, locale)}</dd>
        <dt className="text-fg-muted">{t("lastSeenAt")}</dt>
        <dd>{formatDateTime(entry.lastSeenAt, locale)}</dd>
      </dl>
      {entry.categories.length > 0 ? (
        <div className="mt-4 flex flex-wrap gap-1.5">
          {entry.categories.map((category) => <Tag key={category}>{category}</Tag>)}
        </div>
      ) : null}
      {entry.summary ? (
        <section className="mt-5 whitespace-pre-wrap border-t border-border-subtle pt-5 text-sm leading-6 text-fg-secondary">
          {entry.summary}
        </section>
      ) : null}
      {link ? (
        <a
          href={link}
          target="_blank"
          rel="noreferrer"
          className="mt-6 inline-flex cursor-pointer items-center gap-1.5 rounded-md bg-accent px-3 py-2 text-sm font-medium text-fg-on-accent transition-colors hover:bg-accent-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
        >
          <ExternalLink className="h-4 w-4" />
          {t("originalLink")}
        </a>
      ) : null}
    </article>
  );
}
