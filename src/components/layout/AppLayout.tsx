import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { Topbar } from "./Topbar";
import { MonitorPrompts } from "@/features/monitoring/MonitorPrompts";

/** Casca principal: navegacao lateral + barra superior + area de conteudo. */
export function AppLayout() {
  return (
    <div className="app-shell flex h-screen overflow-hidden bg-bg text-text">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <Topbar />
        <main className="app-main flex-1 overflow-y-auto px-6 py-6">
          <div className="mx-auto max-w-5xl">
            <Outlet />
          </div>
        </main>
      </div>
      {/* Lembretes de abertura/fechamento de programas monitorados (secao 10). */}
      <MonitorPrompts />
    </div>
  );
}
