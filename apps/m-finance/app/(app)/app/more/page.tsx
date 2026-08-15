import Link from "next/link";
import { Calculator, ChevronRight, History, RefreshCw, Settings, Target } from "lucide-react";
import { PageHeading } from "@/components/page-heading";

const links = [
  { href: "/app/subscriptions", label: "Assinaturas", description: "Testes grátis e cobranças recorrentes", icon: RefreshCw },
  { href: "/app/simulator", label: "Simulador", description: "Cabe uma compra sem bagunçar o mês?", icon: Calculator },
  { href: "/app/goals", label: "Metas", description: "Objetivos e quanto já guardou", icon: Target },
  { href: "/app/history", label: "Histórico", description: "Meses anteriores e snapshots", icon: History },
  { href: "/app/settings", label: "Configurações", description: "Alertas, notificações, categorias e dados", icon: Settings },
] as const;

export default function MorePage() {
  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Menu" title="Mais" />

      <div className="space-y-2">
        {links.map((link) => {
          const Icon = link.icon;
          return (
            <Link
              className="focus-ring group flex items-center gap-4 rounded-xl border border-border-subtle bg-background-card/95 px-4 py-3.5 transition duration-200 hover:border-border-default hover:bg-background-hover"
              href={link.href}
              key={link.href}
            >
              <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-border-subtle bg-background-elevated text-text-secondary">
                <Icon size={18} aria-hidden="true" />
              </span>
              <span className="min-w-0 flex-1">
                <span className="block text-sm font-semibold text-text-primary">{link.label}</span>
                <span className="block truncate text-xs text-text-muted">{link.description}</span>
              </span>
              <ChevronRight className="shrink-0 text-text-muted" size={18} aria-hidden="true" />
            </Link>
          );
        })}
      </div>
    </div>
  );
}
