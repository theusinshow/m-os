import type { Page } from "playwright";

const HIDE_CSS = `
  /* Cookie / GDPR banners */
  [id*="cookie"i], [class*="cookie-banner"i], [class*="CookieBanner"],
  [class*="cookie-consent"i], [class*="CookieConsent"],
  [id*="consent"i], [class*="consent-banner"i],
  #onetrust-consent-sdk, .cc-window, .cc-banner, .cookiealert,
  [class*="gdpr"i], [id*="gdpr"i],
  #CybotCookiebotDialog, .cookie-law-info-bar,

  /* Chat widgets */
  #intercom-container, .intercom-lightweight-app, [class*="intercom-"]:not(body),
  #crisp-chatbox, .crisp-client, [id^="crisp"],
  #hubspot-messages-iframe-container, #hubspot-messages-iframe-container + *,
  #drift-widget-container, [class*="drift-widget"],
  [class*="zendesk"i], [id*="zendesk"i],
  iframe[src*="tawk.to"], [id*="tawkchat"],
  [id="launcher"], [class*="chat-launcher"],
  [id*="live-chat"], [class*="live-chat"i],

  /* Newsletter / promo popups */
  [class*="newsletter-popup"], [class*="NewsletterPopup"],
  [class*="popup-overlay"], [class*="modal-overlay"],
  [class*="promotional-modal"],

  /* Scroll-lock e blur residual */
  body.modal-open { overflow: auto !important; }
  body { filter: none !important; }
`;

const CLOSE_BUTTON_TEXTS = [
  "accept", "aceitar", "aceitar tudo", "accept all", "accept cookies",
  "agree", "concordo", "i agree", "i accept",
  "ok", "got it", "entendi", "entendido",
  "close", "fechar", "dismiss", "×", "✕", "✖",
  "allow", "allow all", "permitir",
  "continue", "continuar",
  "rejeitar tudo", "reject all", // também fecha o banner
];

/**
 * Dispensa overlays comuns antes de capturar: cookie banners, popups, chat widgets.
 *
 * Estratégia em duas etapas:
 * 1. Clica em botões conhecidos de aceitar/fechar (no máximo 3 cliques para não
 *    afetar a navegação da página).
 * 2. Injeta CSS que oculta seletores de overlay com `!important`, como fallback
 *    para o que o clique não conseguiu fechar.
 *
 * Todas as falhas são silenciosas — a captura nunca é bloqueada por isso.
 */
export async function dismissOverlays(page: Page): Promise<void> {
  // 1. Clique em botões de fechar/aceitar
  try {
    const clicked = await page.evaluate((texts: string[]) => {
      let count = 0;
      const els = Array.from(
        document.querySelectorAll(
          "button, [role='button'], a[class*='btn'], a[class*='button'], input[type='submit']"
        )
      );
      for (const el of els) {
        const text = (el.textContent ?? "").trim().toLowerCase();
        if (texts.some((t) => text === t || text === t + " →")) {
          (el as HTMLElement).click();
          count++;
          if (count >= 3) break;
        }
      }
      return count;
    }, CLOSE_BUTTON_TEXTS);

    if (clicked > 0) await page.waitForTimeout(350);
  } catch {
    // não bloqueia
  }

  // 2. CSS de ocultação para o que restou
  try {
    await page.addStyleTag({ content: HIDE_CSS });
  } catch {
    // não bloqueia
  }
}
