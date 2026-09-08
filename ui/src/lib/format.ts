export function formatDateTime(value: string | null, locale: string): string {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

export function safeHttpUrl(value: string): string | null {
  try {
    const url = new URL(value);
    const isHttp = url.protocol === "http:" || url.protocol === "https:";
    return isHttp && !url.username && !url.password ? url.href : null;
  } catch {
    return null;
  }
}

export function splitTerms(value: string): string[] {
  return Array.from(
    new Set(
      value
        .split(/[,，\n]/)
        .map((term) => term.trim())
        .filter(Boolean),
    ),
  );
}

export function joinTerms(values: string[]): string {
  return values.join(", ");
}
