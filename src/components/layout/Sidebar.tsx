import { NavLink } from "react-router-dom";
import { NAV_ITEMS } from "@/app/routes";
import { APP } from "@/config/app";
import { cn } from "@/lib/cn";

/** Navegacao lateral fixa. Largura vem do token --nav-width. */
export function Sidebar() {
  return (
    <aside className="no-print flex w-nav shrink-0 flex-col border-r border-border bg-bg-subtle">
      <div className="flex h-14 items-center gap-2.5 border-b border-border px-4">
        <div className="font-display flex h-7 w-7 items-center justify-center bg-accent text-sm font-bold text-accent-contrast">
          C
        </div>
        <span className="font-display text-base font-bold tracking-tight text-text">
          {APP.name}
        </span>
      </div>

      <nav className="flex-1 space-y-1 p-2">
        {NAV_ITEMS.map(({ path, label, icon: Icon }) => (
          <NavLink
            key={path}
            to={path}
            end={path === "/"}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-3 rounded px-3 py-2 text-sm font-medium transition-colors duration-fast",
                isActive
                  ? "bg-accent-muted text-text"
                  : "text-text-muted hover:bg-surface-hover hover:text-text",
              )
            }
          >
            <Icon size={17} strokeWidth={1.75} aria-hidden />
            {label}
          </NavLink>
        ))}
      </nav>

      <div className="border-t border-border px-4 py-3 text-2xs text-text-subtle">
        {APP.name} v{APP.version} · local-first
      </div>
    </aside>
  );
}
