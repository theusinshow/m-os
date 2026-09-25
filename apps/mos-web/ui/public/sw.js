/**
 * O service worker: a única parte do M/OS que roda com o app fechado.
 *
 * # Cache: só quando a rede falha (ADR-077)
 *
 * Com rede, TUDO vem do servidor, sempre — a PWA vai *dentro do binário*
 * (`rust-embed`), e um cache servido primeiro mostraria a versão antiga depois
 * de um deploy, sem nada na tela indicando isso: você conserta um defeito,
 * reinstala, e o celular continua mostrando o defeito. Isso continua proibido.
 *
 * Sem rede, a casca (a página e os assets com hash) sai da última cópia que
 * funcionou. É o que permite abrir o app em modo avião e CAPTURAR — a captura
 * vai para a fila do aparelho (`filaDeCaptura.ts`) e segue quando a rede volta.
 * `/api/*` nunca é guardado: dado velho fingindo ser atual é pior que um aviso
 * de "sem conexão".
 *
 * # A regra do iOS que não perdoa
 *
 * Todo push RECEBIDO tem que virar uma notificação VISÍVEL. O `userVisibleOnly`
 * da assinatura é uma promessa, e um service worker que recebe um push e não
 * mostra nada faz o sistema revogar a assinatura inteira — em silêncio. Por
 * isso o `catch` abaixo ainda mostra um cartão genérico: melhor um aviso vago
 * que uma assinatura morta.
 */

// Assume o controle sem esperar a próxima abertura. Sem isto, um service worker
// novo fica "esperando" enquanto o antigo continua no comando — e a correção
// que você acabou de publicar só passa a valer depois que o app for fechado.
self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (evento) =>
  evento.waitUntil(self.clients.claim()),
);

const CASCA = "mos-casca-v1";
const PAGINA = "/__casca__";

self.addEventListener("fetch", (evento) => {
  const pedido = evento.request;
  const url = new URL(pedido.url);
  if (pedido.method !== "GET" || url.origin !== self.location.origin) return;
  if (url.pathname.startsWith("/api/") || url.pathname === "/sw.js") return;
  const navegacao = pedido.mode === "navigate";
  evento.respondWith(
    fetch(pedido)
      .then((resposta) => {
        // Só o que deu certo e não foi redirecionado (a porta sem sessão
        // redireciona) vira cópia de reserva.
        if (resposta.ok && !resposta.redirected) {
          const copia = resposta.clone();
          evento.waitUntil(guardarCasca(navegacao ? PAGINA : pedido, copia, navegacao));
        }
        return resposta;
      })
      .catch(async () => {
        const cache = await caches.open(CASCA);
        const guardada = await cache.match(navegacao ? PAGINA : pedido);
        return guardada ?? Response.error();
      }),
  );
});

/** Guarda a cópia. Na página, apaga os assets que ela não usa mais — senão
 *  cada deploy deixaria o JS anterior ocupando espaço para sempre. */
async function guardarCasca(chave, resposta, pagina) {
  const cache = await caches.open(CASCA);
  if (!pagina) {
    await cache.put(chave, resposta);
    return;
  }
  const html = await resposta.clone().text();
  await cache.put(chave, resposta);
  const usados = new Set([...html.matchAll(/\/assets\/[^"']+/g)].map((m) => m[0]));
  for (const pedido of await cache.keys()) {
    const caminho = new URL(pedido.url).pathname;
    if (caminho.startsWith("/assets/") && !usados.has(caminho)) await cache.delete(pedido);
  }
}

self.addEventListener("push", (evento) => {
  let aviso;
  try {
    aviso = evento.data.json();
  } catch {
    aviso = { titulo: "M/OS", corpo: "Você tem algo novo.", tag: "m-os" };
  }
  evento.waitUntil(
    Promise.all([
      self.registration.showNotification(aviso.titulo ?? "M/OS", {
        body: aviso.corpo ?? "",
        // Mesma `tag` substitui o cartão anterior em vez de empilhar.
        tag: aviso.tag ?? "m-os",
        icon: "/icone-192.png",
        badge: "/icone-192.png",
        data: { url: aviso.url ?? "/" },
      }),
      marcarIcone(aviso.badge),
    ]),
  );
});

/**
 * O número no canto do ícone, na tela de início.
 *
 * # Por que aqui, e não só no app
 *
 * Este é o único lugar que roda com o app FECHADO — e um badge que só se
 * atualiza quando você abre o app avisa exatamente quando você já não precisa
 * ser avisado.
 *
 * `undefined` significa "não sei", e aí o número fica como estava: o aviso de
 * sync sai de um lugar que às vezes não tem a lista de lembretes na mão, e
 * zerar o ícone ali seria inventar um número.
 *
 * Falha em silêncio de propósito: no iOS a API só existe com a permissão de
 * notificação concedida, e uma exceção aqui derrubaria o `waitUntil` inteiro —
 * a notificação não apareceria, e uma promessa de push não cumprida faz o
 * sistema revogar a assinatura.
 */
async function marcarIcone(quantos) {
  if (typeof quantos !== "number") return;
  try {
    if (quantos > 0) await self.navigator.setAppBadge?.(quantos);
    else await self.navigator.clearAppBadge?.();
  } catch {
    // Sem badge, a notificação continua chegando. É o que importa.
  }
}

self.addEventListener("notificationclick", (evento) => {
  evento.notification.close();
  const destino = evento.notification.data?.url ?? "/";
  evento.waitUntil(
    // Reaproveita a janela que já existe em vez de abrir outra: no iPhone, uma
    // segunda janela do mesmo PWA é uma segunda cópia do app na tela.
    self.clients
      .matchAll({ type: "window", includeUncontrolled: true })
      .then((janelas) => {
        for (const janela of janelas) {
          if ("focus" in janela) return janela.focus();
        }
        return self.clients.openWindow(destino);
      }),
  );
});
