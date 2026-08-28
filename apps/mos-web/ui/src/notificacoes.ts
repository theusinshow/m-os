/**
 * O lado do navegador na conversa de push.
 *
 * # Por que isto é um arquivo separado, e não trinta linhas dentro do App
 *
 * Porque quase tudo aqui é *diagnóstico*, e não ação. Ativar notificação no
 * iPhone falha de cinco jeitos diferentes, e os cinco produzem exatamente o
 * mesmo nada na tela. [`situacao`] transforma cada um deles numa frase que diz o
 * que fazer — que é a única coisa útil quando a notificação não chega.
 */

import { api, type AssinaturaPush } from "./api";

export type Situacao =
  /** Dá para ativar agora. */
  | { estado: "pronto" }
  /** Já está ativo neste aparelho. */
  | { estado: "ativo" }
  /** Falta um passo do usuário, e a frase diz qual. */
  | { estado: "falta"; motivo: string }
  /** Não vai funcionar aqui, e a frase diz por quê. */
  | { estado: "impossivel"; motivo: string };

/**
 * O app está instalado na tela de início?
 *
 * No iOS isto **não é cosmético**: o Safari só entrega Web Push para uma PWA
 * instalada. No navegador comum, `Notification.requestPermission()` nem existe,
 * e o botão de ativar falharia sem dizer por quê.
 */
export function instalado(): boolean {
  return (
    window.matchMedia("(display-mode: standalone)").matches ||
    // O caminho do iOS. Ele nunca implementou `display-mode: standalone` no
    // Safari, e sem esta segunda checagem um iPhone instalado parece não
    // instalado.
    (navigator as { standalone?: boolean }).standalone === true
  );
}

/** Um iPhone ou iPad, onde a exigência de instalar existe. */
function ehApple(): boolean {
  return (
    /iPad|iPhone|iPod/.test(navigator.userAgent) ||
    // iPadOS moderno se apresenta como Mac; o toque é o que o denuncia.
    (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1)
  );
}

/**
 * O que dá para fazer, agora, neste aparelho.
 *
 * `chavePush` vazio significa que o SERVIDOR não manda notificação — e essa
 * distinção importa: é a diferença entre "toque no botão" e "não adianta tocar
 * em botão nenhum".
 */
export async function situacao(chavePush: string | null): Promise<Situacao> {
  if (!chavePush) {
    return {
      estado: "impossivel",
      motivo:
        "Este servidor não tem chave de notificação configurada. Não é o seu aparelho.",
    };
  }
  if (!("serviceWorker" in navigator) || !("PushManager" in window)) {
    return {
      estado: "impossivel",
      motivo: ehApple()
        ? "Este iPhone precisa de iOS 16.4 ou mais novo para receber notificações."
        : "Este navegador não sabe receber notificações.",
    };
  }
  if (!instalado()) {
    return {
      estado: "falta",
      motivo: ehApple()
        ? "Toque em Compartilhar e depois em “Adicionar à Tela de Início”. O iPhone só entrega notificação para o app instalado — no Safari, nada chega."
        : "Instale o app pelo menu do navegador para receber notificações.",
    };
  }
  if (Notification.permission === "denied") {
    return {
      estado: "falta",
      motivo:
        "Você já recusou notificações para este app. Só dá para reverter em Ajustes → Notificações → M/OS.",
    };
  }
  // Uma assinatura que já existe é a resposta mais confiável — mais que a
  // permissão, que continua "granted" depois de o usuário desinstalar e
  // reinstalar o app com uma assinatura nova.
  const registro = await navigator.serviceWorker.getRegistration();
  const assinatura = await registro?.pushManager.getSubscription();
  return assinatura ? { estado: "ativo" } : { estado: "pronto" };
}

/**
 * Ativa. Precisa ser chamado DE DENTRO de um toque.
 *
 * O iOS recusa `requestPermission()` que não venha de um gesto do usuário — e
 * recusa devolvendo "default", sem erro. Por isso não há um "ativar automático
 * ao abrir": ele falharia calado, para sempre.
 */
export async function ativar(chavePublica: string): Promise<void> {
  const registro = await navigator.serviceWorker.register("/sw.js");
  // `ready` e não o retorno do `register`: o worker recém-registrado ainda não
  // está ativo, e assinar antes disso falha.
  await navigator.serviceWorker.ready;

  const permissao = await Notification.requestPermission();
  if (permissao !== "granted") {
    throw new Error(
      permissao === "denied"
        ? "Notificações recusadas. Dá para reverter em Ajustes → Notificações → M/OS."
        : "O aparelho não respondeu ao pedido de permissão. Tente tocar de novo.",
    );
  }

  const assinatura = await registro.pushManager.subscribe({
    // Obrigatório, e não é formalidade: é a promessa de que todo push vira uma
    // notificação visível. Ver `sw.js`.
    userVisibleOnly: true,
    applicationServerKey: deBase64Url(chavePublica),
  });

  await api.assinarPush(assinatura.toJSON() as AssinaturaPush);
}

/**
 * base64url para os bytes que o `subscribe` exige.
 *
 * Ele não aceita a string — só um `Uint8Array` ou `ArrayBuffer`. E é base64
 * **url-safe sem padding**, que o `atob` não entende: sem esta tradução o
 * navegador recusa a chave com "InvalidCharacterError".
 */
function deBase64Url(valor: string): Uint8Array<ArrayBuffer> {
  const preenchido = valor
    .padEnd(valor.length + ((4 - (valor.length % 4)) % 4), "=")
    .replace(/-/g, "+")
    .replace(/_/g, "/");
  const cru = atob(preenchido);
  // Alocado sobre um `ArrayBuffer` explicito: `new Uint8Array(n)` tem tipo
  // `Uint8Array<ArrayBufferLike>`, que abrange `SharedArrayBuffer` — e a
  // assinatura do `subscribe` nao aceita memoria compartilhada.
  const bytes = new Uint8Array(new ArrayBuffer(cru.length));
  for (let i = 0; i < cru.length; i += 1) bytes[i] = cru.charCodeAt(i);
  return bytes;
}
