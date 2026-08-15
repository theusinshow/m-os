/**
 * Derivação de nomes legíveis para seções (v1.5) — lógica pura, sem DOM,
 * para ser testável fora do navegador. O `detect-sections` extrai as pistas
 * do DOM e aplica estas funções no Node.
 */

export interface SectionHints {
  tag: string;
  id?: string;
  className?: string;
  ariaLabel?: string;
  dataSection?: string;
  dataBlock?: string;
  heading?: string;
}

const SEMANTIC_NAMES: Record<string, string> = {
  header: "Cabeçalho",
  footer: "Rodapé",
  nav: "Navegação",
  article: "Artigo",
};

// Palavra-chave (em classe/id/aria/data) → nome canônico. Primeira casa vence,
// então os mais específicos (hero, sobre...) vêm antes de header/footer.
const KEYWORDS: Array<[RegExp, string]> = [
  [/(?:^|[\s_-])(hero|banner|masthead|jumbotron|intro)/, "Hero"],
  [/(?:^|[\s_-])(about|sobre|quem[-_ ]?somos|who[-_ ]?we[-_ ]?are)/, "Sobre"],
  [/(?:^|[\s_-])(service|serviç|solution|soluç)/, "Serviços"],
  [/(?:^|[\s_-])(feature|recurso|funcionalidade)/, "Recursos"],
  [/(?:^|[\s_-])(benefit|benef[ií]cio|vantagen|vantagem)/, "Benefícios"],
  [/(?:^|[\s_-])(pricing|price|plano|preç)/, "Planos"],
  [/(?:^|[\s_-])(testimonial|depoimento|review|avaliaç)/, "Depoimentos"],
  [/(?:^|[\s_-])(portfolio|portf[óo]lio|project|projeto|work|case|trabalho)/, "Portfólio"],
  [/(?:^|[\s_-])(gallery|galeria)/, "Galeria"],
  [/(?:^|[\s_-])(team|equipe)/, "Equipe"],
  [/(?:^|[\s_-])(client|cliente|partner|parceiro|brand)/, "Clientes"],
  [/(?:^|[\s_-])(contact|contato|fale[-_ ]?conosco)/, "Contato"],
  [/(?:^|[\s_-])(faq|perguntas|d[úu]vidas)/, "FAQ"],
  [/(?:^|[\s_-])(steps?|how[-_ ]?it[-_ ]?works|como[-_ ]?funciona|processo|process)/, "Como funciona"],
  [/(?:^|[\s_-])(stats?|numbers?|n[úu]meros?|metric)/, "Números"],
  [/(?:^|[\s_-])(blog|news|not[íi]cia|artigo|posts?)/, "Blog"],
  [/(?:^|[\s_-])(cta|call[-_ ]?to[-_ ]?action|comece|começ?ar|get[-_ ]?started|newsletter)/, "Chamada"],
  [/(?:^|[\s_-])(footer|rodap[ée])/, "Rodapé"],
  [/(?:^|[\s_-])(header|topo|navbar)/, "Cabeçalho"],
];

function titleCase(s: string): string {
  return s
    .replace(/[-_]/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

/** Rejeita ids genéricos (section-3, hashes, wrappers) que dariam nome ruim. */
function isMeaningfulId(id: string): boolean {
  const s = id.toLowerCase();
  if (/^[0-9a-f]{6,}$/.test(s)) return false; // hash
  if (/^(section|block|sect|sec|div|el|item|comp|wrapper|container|content|row|col|main|page)[-_]?\d*$/.test(s)) return false;
  return /[a-z]{3,}/.test(s); // precisa de uma palavra real
}

/**
 * Nome legível de uma seção a partir das pistas. Ordem:
 * palavra-chave > heading visível > tag semântica > id significativo.
 * Sem nada legível → undefined (o chamador usa o número da seção).
 */
export function deriveSectionName(h: SectionHints): string | undefined {
  const haystack = [h.id, h.className, h.ariaLabel, h.dataSection, h.dataBlock]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();

  for (const [re, name] of KEYWORDS) {
    if (re.test(haystack)) return name;
  }
  if (h.heading) return h.heading.slice(0, 48);
  if (SEMANTIC_NAMES[h.tag]) return SEMANTIC_NAMES[h.tag];
  if (h.id && isMeaningfulId(h.id)) return titleCase(h.id).slice(0, 40);
  return undefined;
}

/** Desambigua nomes repetidos: "Serviços", "Serviços 2", "Serviços 3". */
export function disambiguate(names: Array<string | undefined>): Array<string | undefined> {
  const seen: Record<string, number> = {};
  return names.map((n) => {
    if (!n) return n;
    seen[n] = (seen[n] ?? 0) + 1;
    return seen[n] > 1 ? `${n} ${seen[n]}` : n;
  });
}
