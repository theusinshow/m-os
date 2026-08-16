import Link from "next/link";
import { Calculator, ChevronRight, History, PiggyBank, RefreshCw, Settings, Target } from "lucide-react";
import { PageHeading } from "@/components/page-heading";

const planningLinks = [
  { href: "/app/subscriptions", label: "Assinaturas", description: "Testes grátis e cobranças recorrentes", icon: RefreshCw },
  { href: "/app/simulator", label: "Simulador", description: "Cabe uma compra sem bagunçar o mês?", icon: Calculator },
  { href: "/app/budgets", label: "Orçamento", description: "Tetos de gasto por categoria e cartão", icon: PiggyBank },
  { href: "/app/goals", label: "Metas", description: "Objetivos e quanto já guardou", icon: Target },
];

const archiveLinks = [
  { href: "/app/history", label: "Histórico", description: "Meses anteriores e snapshots", icon: History },
  { href: "/app/settings", label: "Configurações", description: "Alertas, notificações, categorias e dados", icon: Settings },
] as const;

export default function MorePage() {
  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Menu" title="Mais" />

      <MenuSection eyebrow="Planejamento" links={planningLinks} />
      <MenuSection eyebrow="Dados e ajustes" links={archiveLinks} />
    </div>
  );
}

function MenuSection({
  eyebrow,
  links,
}: {
  eyebrow: string;
  links: readonly {
    href: string;
    label: string;
    description: string;
    icon: typeof Calculator;
  }[];
}) {
  return (
    <section className="space-y-2">
      <h2 className="px-1 text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
        {eyebrow}
      </h2>
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
                <span className="mt-0.5 block text-xs leading-5 text-text-muted">
                  {link.description}
                </span>
              </span>
              <ChevronRight className="shrink-0 text-text-muted" size={18} aria-hidden="true" />
            </Link>
          );
        })}
      </div>
    </section>
  );
}
