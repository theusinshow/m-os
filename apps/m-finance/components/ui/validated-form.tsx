"use client";

import { createContext, useActionState, useContext, useEffect, useRef } from "react";
import { initialFormState, type FormState } from "@/lib/form-state";
import { useToast } from "@/components/ui/toast";
import { cn } from "@/lib/utils";

type FormAction = (prevState: FormState, formData: FormData) => Promise<FormState>;

const FieldErrorContext = createContext<Record<string, string>>({});

/**
 * Sinal de "o form acabou de se limpar".
 *
 * `form.reset()` e reset nativo do DOM: devolve os inputs ao `defaultValue` e
 * nao tem como tocar em estado do React. Campo controlado dentro de um form com
 * `resetOnSuccess` sobrevivia ao reset e seguia enviando o valor anterior — com
 * o agravante de seguir ACESO na tela, entao o formulario parecia limpo e
 * mentia. Foi assim que quatro contas seguidas nasceram na categoria da
 * primeira, em 19/06/2026, cinco minutos depois do formulario existir.
 *
 * O sinal e o proprio objeto de estado da action, que nasce novo a cada retorno
 * — comparar identidade basta, e nao custa um `useState` que so existiria para
 * contar. Campo controlado que viva aqui dentro assina e se zera junto;
 * `CategoryChips` e o exemplo.
 */
const FormResetContext = createContext<FormState | null>(null);

export function useFormResetSignal() {
  return useContext(FormResetContext);
}

function useFieldError(name: string) {
  return useContext(FieldErrorContext)[name];
}

export function ValidatedForm({
  action,
  children,
  className,
  successMessage,
  resetOnSuccess = false,
  onSuccess,
}: {
  action: FormAction;
  children: React.ReactNode;
  className?: string;
  successMessage: string;
  resetOnSuccess?: boolean;
  /** Chamado após sucesso (ex.: fechar o drawer que contém o form). */
  onSuccess?: () => void;
}) {
  const [state, formAction] = useActionState(action, initialFormState);
  const { addToast } = useToast();
  const formRef = useRef<HTMLFormElement>(null);

  // `onSuccess` chega como arrow inline em todos os seis usos, ou seja: muda de
  // identidade a cada render. Como dependencia do efeito abaixo, qualquer render
  // extra enquanto `state` seguisse em "success" repetiria o toast e o reset.
  // A ref mantem a versao atual sem que a identidade entre nas dependencias.
  const onSuccessRef = useRef(onSuccess);
  useEffect(() => {
    onSuccessRef.current = onSuccess;
  });

  useEffect(() => {
    if (state.status === "success") {
      addToast(state.message ?? successMessage, "success");
      if (resetOnSuccess) {
        formRef.current?.reset();
      }
      onSuccessRef.current?.();
    } else if (state.status === "error" && state.message && !state.fieldErrors) {
      addToast(state.message, "error");
    }
  }, [state, addToast, successMessage, resetOnSuccess]);

  return (
    <FieldErrorContext.Provider value={state.fieldErrors ?? {}}>
      <FormResetContext.Provider
        value={resetOnSuccess && state.status === "success" ? state : null}
      >
        <form action={formAction} className={cn(className)} ref={formRef}>
          {children}
        </form>
      </FormResetContext.Provider>
    </FieldErrorContext.Provider>
  );
}

export function FieldError({ name }: { name: string }) {
  const error = useFieldError(name);
  if (!error) return null;
  return (
    <p className="mt-1.5 text-xs font-medium text-accent" role="alert">
      {error}
    </p>
  );
}

const ERROR_BORDER = "border-accent ring-1 ring-accent/30";

export function ValidatedInput({
  name,
  className,
  ...props
}: React.InputHTMLAttributes<HTMLInputElement> & { name: string }) {
  const error = useFieldError(name);
  return (
    <>
      <input
        aria-invalid={error ? true : undefined}
        className={cn(className, error && ERROR_BORDER)}
        name={name}
        {...props}
      />
      <FieldError name={name} />
    </>
  );
}

export function ValidatedSelect({
  name,
  className,
  children,
  ...props
}: React.SelectHTMLAttributes<HTMLSelectElement> & { name: string }) {
  const error = useFieldError(name);
  return (
    <>
      <select
        aria-invalid={error ? true : undefined}
        className={cn(className, error && ERROR_BORDER)}
        name={name}
        {...props}
      >
        {children}
      </select>
      <FieldError name={name} />
    </>
  );
}
