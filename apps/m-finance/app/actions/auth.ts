"use server";

import { redirect } from "next/navigation";
import { createServerSupabaseClient } from "@/lib/supabase/server";
import { env } from "@/lib/env";

/**
 * Login por codigo de 6 digitos, e nao OAuth.
 *
 * O Google recusa renderizar a tela de sign-in dentro de iframe ou webview
 * embutida — politica dele contra clickjacking, nao algo que a nossa CSP possa
 * liberar. Como o M-Finance e exibido dentro do M/OS (ADR-039), com OAuth o
 * app ficava inacessivel de la: a janela mostrava um 403 do proprio Google.
 *
 * O codigo por e-mail roda inteiro em pagina nossa, entao funciona igual no
 * navegador e dentro do embed. A trava de quem entra continua sendo a mesma de
 * antes, `AUTHORIZED_EMAIL`, checada aqui e de novo no `proxy.ts`.
 */

export type LoginState = {
  step: "email" | "code";
  email: string;
  error?: string;
  sent?: boolean;
};

function emailAutorizado(email: string) {
  return Boolean(env.authorizedEmail) && email === env.authorizedEmail.trim().toLowerCase();
}

/**
 * Despacha o passo certo. O formulario e um so — e o mesmo `useActionState`
 * dos dois lados — entao quem sabe em que etapa estamos e o estado anterior,
 * nao o cliente.
 */
export async function continuarLogin(anterior: LoginState, formData: FormData): Promise<LoginState> {
  if (formData.get("intent") === "trocar-email") {
    return { step: "email", email: "" };
  }

  return anterior.step === "code"
    ? confirmarCodigo(anterior, formData)
    : pedirCodigo(anterior, formData);
}

async function pedirCodigo(_anterior: LoginState, formData: FormData): Promise<LoginState> {
  const email = String(formData.get("email") ?? "").trim().toLowerCase();

  if (!email) {
    return { step: "email", email, error: "Informe o e-mail." };
  }

  if (!emailAutorizado(email)) {
    return { step: "email", email, error: "Este app aceita apenas o e-mail autorizado." };
  }

  const supabase = await createServerSupabaseClient();
  if (!supabase) {
    return { step: "email", email, error: "Supabase não configurado." };
  }

  const { error } = await supabase.auth.signInWithOtp({
    email,
    // Sem criacao de usuario: o unico dono ja existe, e deixar o padrao ligado
    // faria de um e-mail digitado errado uma conta nova em silencio.
    options: { shouldCreateUser: false },
  });

  if (error) {
    // A mensagem do provedor vai junto de proposito. Um app de um dono so nao
    // ganha nada escondendo "Error sending magic link email" atras de um texto
    // generico: quem le a tela e quem configura o SMTP, e sem a causa a falha
    // vira adivinhacao.
    return {
      step: "email",
      email,
      error: `Não consegui enviar o código: ${error.message}`,
    };
  }

  return { step: "code", email, sent: true };
}

async function confirmarCodigo(anterior: LoginState, formData: FormData): Promise<LoginState> {
  const email = String(formData.get("email") ?? anterior.email).trim().toLowerCase();
  const codigo = String(formData.get("codigo") ?? "").replace(/\D/g, "");

  if (codigo.length !== 6) {
    return { step: "code", email, error: "O código tem 6 dígitos." };
  }

  if (!emailAutorizado(email)) {
    return { step: "email", email: "", error: "Este app aceita apenas o e-mail autorizado." };
  }

  const supabase = await createServerSupabaseClient();
  if (!supabase) {
    return { step: "code", email, error: "Supabase não configurado." };
  }

  const { error } = await supabase.auth.verifyOtp({ email, token: codigo, type: "email" });

  if (error) {
    return { step: "code", email, error: "Código inválido ou expirado. Peça outro." };
  }

  // Fora de try/catch de proposito: `redirect` sinaliza por excecao, e
  // engoli-la deixaria o usuario autenticado parado na tela de login.
  redirect("/app/dashboard");
}

export async function signOut() {
  const supabase = await createServerSupabaseClient();
  await supabase?.auth.signOut();
  redirect("/login");
}
