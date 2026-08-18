"use client";

import { useActionState } from "react";
import { ArrowRight } from "lucide-react";
import { continuarLogin, type LoginState } from "@/app/actions/auth";
import { TriangleMark } from "@/components/brand/triangle-mark";

const INICIAL: LoginState = { step: "email", email: "" };

const campo =
  "clip-notch focus-ring h-12 w-full border border-border-default bg-background-primary px-4 text-sm text-text-primary placeholder:text-text-muted";

const botao =
  "clip-notch sheen group focus-ring flex h-12 w-full items-center justify-center gap-2 bg-accent-strong px-4 text-sm font-semibold tracking-tight text-text-primary transition duration-200 hover:bg-accent-strong-hover active:scale-[0.99] disabled:cursor-not-allowed disabled:opacity-50";

export function LoginForm({ configured }: { configured: boolean }) {
  const [state, enviar, pendente] = useActionState(continuarLogin, INICIAL);
  const noCodigo = state.step === "code";

  return (
    <form action={enviar} className="mt-6 flex flex-col gap-4">
      {state.error ? (
        <p className="clip-notch border border-accent-border bg-accent-soft px-4 py-3 text-sm text-text-secondary" role="alert">
          {state.error}
        </p>
      ) : null}

      {noCodigo && state.sent ? (
        <p className="text-sm leading-6 text-text-muted" aria-live="polite">
          Enviei um código de 6 dígitos para <b className="text-text-secondary">{state.email}</b>.
          Ele vale por uma hora.
        </p>
      ) : null}

      {noCodigo ? (
        <>
          <input name="email" type="hidden" value={state.email} />
          <label className="flex flex-col gap-2">
            <span className="text-[11px] font-medium uppercase tracking-[0.22em] text-text-muted">Código</span>
            <input
              autoComplete="one-time-code"
              autoFocus
              className={`${campo} text-center text-lg tracking-[0.5em] tabular-nums`}
              inputMode="numeric"
              maxLength={6}
              name="codigo"
              pattern="[0-9]{6}"
              placeholder="000000"
              required
            />
          </label>
        </>
      ) : (
        <label className="flex flex-col gap-2">
          <span className="text-[11px] font-medium uppercase tracking-[0.22em] text-text-muted">E-mail</span>
          <input
            autoComplete="email"
            autoFocus
            className={campo}
            defaultValue={state.email}
            name="email"
            placeholder="voce@exemplo.com"
            required
            type="email"
          />
        </label>
      )}

      <button className={botao} disabled={!configured || pendente} type="submit">
        <TriangleMark className="rotate-90 opacity-90" size={11} variant="solid" />
        {pendente ? "Um instante" : noCodigo ? "Entrar" : "Enviar código"}
        <ArrowRight className="transition-transform duration-300 group-hover:translate-x-0.5" size={16} />
      </button>

      {noCodigo ? (
        <button
          className="focus-ring self-start text-xs uppercase tracking-[0.18em] text-text-muted underline-offset-4 hover:text-text-secondary hover:underline"
          disabled={pendente}
          name="intent"
          type="submit"
          value="trocar-email"
        >
          Usar outro e-mail
        </button>
      ) : null}
    </form>
  );
}
