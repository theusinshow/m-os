/**
 * O service worker: a única parte do M/OS que roda com o app fechado.
 *
 * # O que ele NÃO faz, e por quê
 *
 * Ele não guarda nada em cache. A tentação é óbvia — um service worker é o
 * lugar canônico para offline —, mas a PWA daqui vai *dentro do binário*
 * (`rust-embed`), e um cache de página serviria a versão antiga depois de um
 * deploy, sem nada na tela indicando isso. O pior sintoma possível: você
 * conserta um defeito, reinstala, e o celular continua mostrando o defeito.
 *
 * Offline de verdade — capturar sem sinal e enfileirar — é uma decisão
 * separada, com Background Sync e uma fila própria. Está fora daqui de
 * propósito.
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
