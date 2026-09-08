import { Button, cn } from "@tokimo/ui";
import { Archive, List, Plus, Rss, Settings2, SlidersHorizontal } from "lucide-react";
import type { ReactNode } from "react";
import type { SourceDto } from "../api/types";

export type AppView =
  | { kind: "entries"; sourceId?: string }
  | { kind: "sources" }
  | { kind: "rules" };

interface SidebarProps {
  sources: SourceDto[];
  view: AppView;
  t: (key: string) => string;
  onChange: (view: AppView) => void;
  onAddSource: () => void;
}

function isSelected(view: AppView, sourceId?: string): boolean {
  return view.kind === "entries" && view.sourceId === sourceId;
}

export function Sidebar({ sources, view, t, onChange, onAddSource }: SidebarProps) {
  return (
    <aside className="flex w-60 shrink-0 flex-col border-r border-border-subtle bg-surface-sidebar text-fg-primary">
      <header className="flex items-center gap-2.5 border-b border-border-subtle px-3 py-3.5">
        <span className="grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-accent text-fg-on-accent shadow-sm">
          <Rss className="h-4 w-4" />
        </span>
        <div className="min-w-0">
          <div className="truncate text-sm font-semibold">{t("appName")}</div>
          <div className="truncate text-[10px] text-fg-muted">{t("appSubtitle")}</div>
        </div>
      </header>

      <nav className="min-h-0 flex-1 overflow-y-auto px-2 py-2">
        <NavButton
          active={isSelected(view)}
          icon={<List />}
          label={t("allEntries")}
          onClick={() => onChange({ kind: "entries" })}
        />

        <div className="mt-4 mb-1 flex items-center justify-between px-2">
          <span className="text-[10px] font-semibold uppercase tracking-wider text-fg-muted">
            {t("subscriptions")}
          </span>
          <Button
            variant="text"
            size="xs"
            shape="circle"
            icon={<Plus />}
            aria-label={t("addSource")}
            title={t("addSource")}
            onClick={onAddSource}
          />
        </div>

        {sources.map((source) => (
          <NavButton
            key={source.id}
            active={isSelected(view, source.id)}
            icon={source.archivedAt ? <Archive /> : <Rss />}
            label={source.name}
            suffix={source.entryCount.toLocaleString()}
            muted={Boolean(source.archivedAt) || !source.enabled}
            warning={Boolean(source.lastError) || source.gapSuspected}
            onClick={() => onChange({ kind: "entries", sourceId: source.id })}
          />
        ))}

        <div className="mt-3 border-t border-border-subtle pt-2">
          <NavButton
            active={view.kind === "sources"}
            icon={<Settings2 />}
            label={t("manageSources")}
            onClick={() => onChange({ kind: "sources" })}
          />
          <NavButton
            active={view.kind === "rules"}
            icon={<SlidersHorizontal />}
            label={t("rules")}
            onClick={() => onChange({ kind: "rules" })}
          />
        </div>
      </nav>
    </aside>
  );
}

interface NavButtonProps {
  active: boolean;
  icon: ReactNode;
  label: string;
  suffix?: string;
  muted?: boolean;
  warning?: boolean;
  onClick: () => void;
}

function NavButton({ active, icon, label, suffix, muted, warning, onClick }: NavButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "mb-0.5 flex w-full cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition-colors focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent",
        active
          ? "bg-accent-subtle text-accent-text"
          : "text-fg-secondary hover:bg-fill-tertiary hover:text-fg-primary",
        muted && "opacity-60",
      )}
    >
      <span className="shrink-0 [&>svg]:h-3.5 [&>svg]:w-3.5">{icon}</span>
      <span className="min-w-0 flex-1 truncate">{label}</span>
      {warning ? <span className="h-1.5 w-1.5 rounded-full bg-state-warning-base" /> : null}
      {suffix ? <span className="text-[10px] tabular-nums text-fg-muted">{suffix}</span> : null}
    </button>
  );
}
