/**
 * A porta, do lado do navegador: registrar este aparelho e entrar com ele.
 *
 * # O trabalho chato que este arquivo existe para conter
 *
 * A API do WebAuthn não fala JSON. Ela recebe e devolve `ArrayBuffer` em cinco
 * campos aninhados, enquanto o servidor manda e espera base64url — porque JSON
 * não tem bytes. Traduzir isso é mecânico, é fácil de errar num campo só, e o
 * erro de um campo só aparece como `NotAllowedError`, que é a mesma coisa que o
 * iPhone diz quando você simplesmente cancela o Face ID.
 *
 * Então a tradução mora aqui, inteira, e a tela não sabe que ela existe.
 *
 * O arquivo se chama `cerimonia` e não `porta` porque `Porta.tsx` já existe, e
 * num sistema de arquivos que não distingue maiúscula os dois seriam o mesmo
 * módulo — o TypeScript recusa, e com razão.
 */

/** O que a tela precisa saber antes de qualquer login. */
export type EstadoDaPorta = {
  /** Há porta configurada neste servidor? */
  porta: boolean;
  /** Já existe algum aparelho registrado? */
  registrado: boolean;
  /** A cerimônia WebAuthn foi compilada neste binário? */
  passkey: boolean;
};

export async function estadoDaPorta(): Promise<EstadoDaPorta> {
  const resposta = await fetch("/api/porta/estado");
  if (!resposta.ok) throw new Error("A porta não respondeu.");
  return (await resposta.json()) as EstadoDaPorta;
}

/**
 * As opções que o servidor manda, antes da tradução.
 *
 * Tipar isto como `PublicKeyCredentialCreationOptions` seria mentir: naquele
 * tipo os campos de bytes já são `BufferSource`, e aqui eles ainda são as
 * strings base64url que vieram no JSON. A mentira não fica escondida — ela
 * reaparece como um `as` impossível no meio da tradução.
 */
type OpcoesCruas = Record<string, unknown> & {
  challenge: string;
  user?: { id: string } & Record<string, unknown>;
  excludeCredentials?: Array<{ id: string } & Record<string, unknown>>;
  allowCredentials?: Array<{ id: string } & Record<string, unknown>>;
};

/** Troca o `id` base64url de cada credencial pelos bytes. */
function comBytes(
  lista: Array<{ id: string } & Record<string, unknown>> | undefined,
): PublicKeyCredentialDescriptor[] {
  return (lista ?? []).map(
    (cada) =>
      ({ ...cada, id: paraBytes(cada.id) }) as unknown as PublicKeyCredentialDescriptor,
  );
}

/** Registra este aparelho. Precisa do convite. */
export async function registrar(convite: string, apelido: string): Promise<void> {
  const { desafio, opcoes } = await pedir("/api/porta/registro/inicio", {
    convite,
    apelido,
  });
  const cruas = opcoes.publicKey as OpcoesCruas;

  const credencial = (await navigator.credentials.create({
    publicKey: {
      ...cruas,
      challenge: paraBytes(cruas.challenge),
      user: {
        ...cruas.user,
        id: paraBytes(cruas.user?.id ?? ""),
      },
      excludeCredentials: comBytes(cruas.excludeCredentials),
    } as unknown as PublicKeyCredentialCreationOptions,
  })) as PublicKeyCredential | null;

  if (!credencial) throw new Error("O aparelho não devolveu credencial.");
  const resposta = credencial.response as AuthenticatorAttestationResponse;

  await pedir("/api/porta/registro/fim", {
    desafio,
    apelido,
    credencial: {
      id: credencial.id,
      rawId: paraBase64Url(credencial.rawId),
      type: credencial.type,
      response: {
        attestationObject: paraBase64Url(resposta.attestationObject),
        clientDataJSON: paraBase64Url(resposta.clientDataJSON),
      },
      extensions: credencial.getClientExtensionResults(),
    },
  });
}

/** Entra. É aqui que o Face ID aparece. */
export async function entrar(): Promise<void> {
  const { desafio, opcoes } = await pedir("/api/porta/login/inicio", {});
  const cruas = opcoes.publicKey as OpcoesCruas;

  const credencial = (await navigator.credentials.get({
    publicKey: {
      ...cruas,
      challenge: paraBytes(cruas.challenge),
      allowCredentials: comBytes(cruas.allowCredentials),
    } as unknown as PublicKeyCredentialRequestOptions,
  })) as PublicKeyCredential | null;

  if (!credencial) throw new Error("O aparelho não devolveu credencial.");
  const resposta = credencial.response as AuthenticatorAssertionResponse;

  await pedir("/api/porta/login/fim", {
    desafio,
    credencial: {
      id: credencial.id,
      rawId: paraBase64Url(credencial.rawId),
      type: credencial.type,
      response: {
        authenticatorData: paraBase64Url(resposta.authenticatorData),
        clientDataJSON: paraBase64Url(resposta.clientDataJSON),
        signature: paraBase64Url(resposta.signature),
        userHandle: resposta.userHandle ? paraBase64Url(resposta.userHandle) : null,
      },
      extensions: credencial.getClientExtensionResults(),
    },
  });
}

export async function sair(): Promise<void> {
  await fetch("/api/porta/sair", { method: "POST" });
}

/**
 * Passkey funciona aqui?
 *
 * O iPhone exige a PWA instalada na Tela de Início para o Face ID de uma
 * passkey de plataforma. No Safari comum ele pede a câmera para ler um QR code
 * de outro aparelho — o que, para quem só quer abrir o próprio app, parece
 * defeito.
 */
export function passkeyDisponivel(): boolean {
  return (
    typeof window.PublicKeyCredential !== "undefined" &&
    typeof navigator.credentials?.create === "function"
  );
}

// --------------------------------------------------------------------- apoio

type Desafio = { desafio: string; opcoes: Record<string, unknown> };

async function pedir(caminho: string, corpo: unknown): Promise<Desafio> {
  const resposta = await fetch(caminho, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(corpo),
  });
  const json = await resposta.json().catch(() => null);
  if (!resposta.ok) {
    throw new Error(json?.erro ?? `A porta respondeu ${resposta.status}.`);
  }
  return json as Desafio;
}

/**
 * base64url para bytes.
 *
 * É url-safe e sem padding, que o `atob` não entende: sem esta tradução o
 * navegador recusa com `InvalidCharacterError`, que não menciona base64 em
 * lugar nenhum.
 */
function paraBytes(valor: string): Uint8Array<ArrayBuffer> {
  const preenchido = valor
    .padEnd(valor.length + ((4 - (valor.length % 4)) % 4), "=")
    .replace(/-/g, "+")
    .replace(/_/g, "/");
  const cru = atob(preenchido);
  const bytes = new Uint8Array(new ArrayBuffer(cru.length));
  for (let i = 0; i < cru.length; i += 1) bytes[i] = cru.charCodeAt(i);
  return bytes;
}

/** Bytes para base64url, que é o que o servidor sabe ler. */
function paraBase64Url(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binario = "";
  for (const byte of bytes) binario += String.fromCharCode(byte);
  return btoa(binario).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}
