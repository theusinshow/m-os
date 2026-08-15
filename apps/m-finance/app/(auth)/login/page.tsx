import { ArrowRight, ShieldCheck } from "lucide-react";
import { signInWithGoogle } from "@/app/actions/auth";
import { TriangleField } from "@/components/brand/triangle-field";
import { TriangleMark } from "@/components/brand/triangle-mark";
import { isSupabaseConfigured } from "@/lib/env";

type LoginPageProps = {
  searchParams: Promise<{
    error?: string;
  }>;
};

const errorMessages: Record<string, string> = {
  supabase_not_configured: "Configure as variáveis do Supabase para ativar o login.",
  oauth: "Não foi possível iniciar o login com Google.",
  callback: "Não foi possível concluir a autenticação.",
  missing_code: "O retorno do Google veio sem código de autenticação.",
  unauthorized_email: "Este app é privado e aceita apenas o e-mail autorizado.",
};

export default async function LoginPage({ searchParams }: LoginPageProps) {
  const { error } = await searchParams;
  const configured = isSupabaseConfigured();

  return (
    <main className="relative min-h-screen overflow-hidden bg-background-primary text-text-primary">
      <div className="pointer-events-none absolute inset-0 -z-10">
        <TriangleField />
      </div>
      <div
        aria-hidden="true"
        className="absolute inset-0 -z-10 bg-[radial-gradient(120%_90%_at_15%_-10%,transparent_30%,rgba(2,10,6,0.72)_72%,rgba(2,10,6,0.96)_100%)]"
      />

      <div className="mx-auto flex min-h-screen w-full max-w-6xl flex-col px-6 py-8">
        <header className="flex items-center justify-between">
          <div className="flex items-center gap-2.5 text-text-muted">
            <TriangleMark className="spin-slow text-accent" size={18} variant="nested" />
            <span className="text-xs font-medium uppercase tracking-[0.28em]">Coded by M</span>
          </div>
          <span className="hidden text-xs font-medium uppercase tracking-[0.28em] text-text-muted sm:block">
            Cockpit financeiro
          </span>
        </header>

        <section className="grid flex-1 items-center gap-12 py-10 lg:grid-cols-[1.15fr_0.85fr]">
          <div className="max-w-2xl">
            <div className="mb-7 inline-flex items-center gap-2 border border-border-subtle bg-background-secondary/60 px-3 py-2 text-[11px] font-medium uppercase tracking-[0.22em] text-text-muted backdrop-blur-sm clip-notch">
              <span className="tri-mark h-2 w-2 bg-accent" />
              Acesso privado
            </div>
            <h1 className="font-display text-6xl font-semibold leading-[0.92] tracking-tight text-text-primary md:text-7xl">
              M Finance
            </h1>
            <p className="mt-7 max-w-xl text-lg leading-7 text-text-secondary">
              Controle mensal de contas, vencimentos, faturas simples e sobra prevista. Sem
              transações miúdas, sem ruído de app financeiro genérico.
            </p>
            <div className="mt-9 flex flex-wrap gap-x-8 gap-y-3 text-sm text-text-muted">
              {["Vencimentos", "Faturas simples", "Sobra prevista"].map((item) => (
                <span className="flex items-center gap-2" key={item}>
                  <TriangleMark className="text-accent" size={11} variant="solid" />
                  {item}
                </span>
              ))}
            </div>
          </div>

          <div className="clip-notch-lg relative border border-border-default bg-background-secondary/80 p-7 backdrop-blur-md">
            <div
              aria-hidden="true"
              className="absolute right-0 top-0 h-[1.1rem] w-[1.1rem] bg-accent/70"
              style={{ clipPath: "polygon(100% 0, 0 0, 100% 100%)" }}
            />
            <div className="mb-6 flex h-12 w-12 items-center justify-center border border-accent-border bg-accent-soft text-accent clip-notch">
              <ShieldCheck size={24} />
            </div>
            <h2 className="font-display text-2xl font-semibold text-text-primary">Acesso restrito</h2>
            <p className="mt-2 text-sm leading-6 text-text-muted">
              Entre com a conta Google autorizada para abrir o cockpit financeiro.
            </p>

            {error ? (
              <div className="mt-5 border border-accent-border bg-accent-soft px-4 py-3 text-sm text-text-secondary clip-notch">
                {errorMessages[error] ?? "Erro de autenticação."}
              </div>
            ) : null}

            <form action={signInWithGoogle} className="mt-6">
              <button
                className="clip-notch sheen group focus-ring flex h-12 w-full items-center justify-center gap-2 bg-accent-strong px-4 text-sm font-semibold tracking-tight text-text-primary transition duration-200 hover:bg-accent-strong-hover active:scale-[0.99] disabled:cursor-not-allowed disabled:opacity-50"
                disabled={!configured}
                type="submit"
              >
                <TriangleMark className="rotate-90 opacity-90" size={11} variant="solid" />
                Entrar com Google
                <ArrowRight
                  className="transition-transform duration-300 group-hover:translate-x-0.5"
                  size={16}
                />
              </button>
            </form>

            {!configured ? (
              <p className="mt-4 text-xs leading-5 text-text-muted">
                Supabase ainda não configurado. Preencha o .env com URL e chave pública.
              </p>
            ) : null}
          </div>
        </section>
      </div>
    </main>
  );
}
