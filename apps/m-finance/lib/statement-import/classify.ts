/**
 * Sugere categoria e "é assinatura" para cada compra importada.
 *
 * Duas fontes, nesta ordem: o que a pessoa já escolheu antes para a mesma
 * origem (aprendido das compras gravadas) e, na falta, uma tabela de palavras.
 * A tabela erra pouco nas origens comuns e cala nas raras — "Mateusribeiro" fica
 * sem categoria até alguém escolher uma vez, e a partir daí é lembrado.
 */

export type Suggestion = {
  categorySlug: string | null;
  isSubscription: boolean;
  /** De onde veio a sugestão, para a tela poder dizer "como da última vez". */
  source: "learned" | "rule" | "none";
};

export type LearnedChoice = { categorySlug: string | null; isSubscription: boolean };

/** As categorias que a importação cria quando ainda não existem. */
export const IMPORT_CATEGORIES = [
  { slug: "software-apis", name: "Software & APIs" },
  { slug: "alimentacao", name: "Alimentação" },
  { slug: "mercado", name: "Mercado" },
  { slug: "compras", name: "Compras" },
  { slug: "taxas", name: "Taxas" },
] as const;

function fold(text: string) {
  return text
    .trim()
    .toLocaleLowerCase("pt-BR")
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "");
}

/**
 * A origem da compra, sem o que muda de um mês para o outro. "Openai *Chatgpt
 * Subscr" e "Openai" são origens diferentes de propósito: uma é a assinatura,
 * a outra é a API.
 */
export function merchantKey(description: string) {
  const tokens = fold(description)
    .replace(/\b(nupay|parcela|parcelamento de compra|pg|ec|dl|pag|ebn|ifd)\b/g, " ")
    .replace(/[^a-z\s]/g, " ")
    .split(/\s+/)
    .filter((token) => token.length > 1);
  return tokens.slice(0, 2).join(" ");
}

type Rule = { slug: string; subscription: boolean; pattern: RegExp };

// A ordem importa: a primeira que casa vence. "Amazon Ad Free" é assinatura
// antes de ser Amazon, e "Openai *Chatgpt" é assinatura antes de ser API.
const RULES: Rule[] = [
  { slug: "taxas", subscription: false, pattern: /\biof\b|anuidade|juros|tarifa|encargo|multa/ },
  {
    slug: "assinaturas",
    subscription: true,
    pattern:
      /chatgpt|spotify|youtube|hbo|netflix|disney|apple\.com|google g1|google one|amazon ad free|prime video|deezer|croma|league pass|meli\+|paramount|globoplay|crunchyroll/,
  },
  {
    slug: "software-apis",
    subscription: true,
    pattern:
      /anthropic|claude|cursor|openai|railway|vercel|github|supabase|heroku|digitalocean|cloudflare|twilio|figma|notion|aws|google cloud/,
  },
  { slug: "software-apis", subscription: false, pattern: /registro\.?br|godaddy|hostinger/ },
  { slug: "moto", subscription: false, pattern: /cf moto|honda moto|yamaha/ },
  { slug: "saude", subscription: true, pattern: /wellhub|gympass|totalpass|smart ?fit/ },
  { slug: "saude", subscription: false, pattern: /farmacia|drogaria|droga|raia|pague menos|hospital|clinica|laborat/ },
  {
    slug: "alimentacao",
    subscription: false,
    pattern: /ifood|restaura|pizza|burger|lanche|kfc|mcdonald|padaria|yooga|camaroes|delivery|waguinho|sushi|acai|cafe/,
  },
  { slug: "mercado", subscription: false, pattern: /supermerc|hiper|atacad|carrefour|assai|bistek|angeloni|giassi/ },
  { slug: "transporte", subscription: false, pattern: /posto|uber|99 ?app|combust|estaciona|shell|ipiranga|petrobras|sem parar/ },
  { slug: "lazer", subscription: false, pattern: /airbnb|steam|teddy bear|cinema|ingresso|playstation|xbox|booking|hotel/ },
  {
    slug: "compras",
    subscription: false,
    pattern:
      /mercado ?livre|mercadopago|amazon|kabum|pichau|aliexpress|shopee|samsung|stanley|magalu|secretoutlet|kawaperei|lojasq2|shein|americanas|mercado\*/,
  },
];

export function suggestFor(
  description: string,
  learned: Map<string, LearnedChoice>,
): Suggestion {
  const remembered = learned.get(merchantKey(description));
  if (remembered) return { ...remembered, source: "learned" };

  const text = fold(description);
  const rule = RULES.find((candidate) => candidate.pattern.test(text));
  if (rule) return { categorySlug: rule.slug, isSubscription: rule.subscription, source: "rule" };

  return { categorySlug: null, isSubscription: false, source: "none" };
}

/**
 * O aprendido, a partir das compras já gravadas (mais recentes primeiro): a
 * primeira escolha de cada origem é a que vale.
 */
export function learnFrom(
  rows: { description: string; categorySlug: string | null; isSubscription: boolean }[],
) {
  const learned = new Map<string, LearnedChoice>();
  for (const row of rows) {
    if (!row.categorySlug && !row.isSubscription) continue;
    const key = merchantKey(row.description);
    if (!key || learned.has(key)) continue;
    learned.set(key, { categorySlug: row.categorySlug, isSubscription: row.isSubscription });
  }
  return learned;
}
