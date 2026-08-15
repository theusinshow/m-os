import { deriveSectionName, disambiguate, type SectionHints } from "../lib/capture/section-name";

let pass = 0, fail = 0;
function eq(label: string, got: unknown, want: unknown) {
  if (got === want) { console.log(`  ✓ ${label}`); pass++; }
  else { console.log(`  ✗ ${label} — got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`); fail++; }
}

const h = (p: Partial<SectionHints>): SectionHints => ({ tag: "section", ...p });

// ── Palavra-chave em classe/id/aria ──
eq("class hero-section → Hero", deriveSectionName(h({ className: "hero-section dark" })), "Hero");
eq("id about → Sobre", deriveSectionName(h({ id: "about" })), "Sobre");
eq("class our-services → Serviços", deriveSectionName(h({ className: "our-services" })), "Serviços");
eq("class services (plural) → Serviços", deriveSectionName(h({ className: "services" })), "Serviços");
eq("class pricing-table → Planos", deriveSectionName(h({ className: "pricing-table" })), "Planos");
eq("class testimonials → Depoimentos", deriveSectionName(h({ className: "testimonials" })), "Depoimentos");
eq("aria-label Contato → Contato", deriveSectionName(h({ ariaLabel: "Seção de contato" })), "Contato");
eq("data-section sobre-nos → Sobre", deriveSectionName(h({ dataSection: "sobre-nos", id: "sobre-nos" })), "Sobre");

// ── Heading quando não há palavra-chave ──
eq("sem keyword, usa heading", deriveSectionName(h({ className: "block-xyz", heading: "Nossa equipe de elite" })), "Nossa equipe de elite");

// ── Tag semântica ──
eq("footer sem pistas → Rodapé", deriveSectionName(h({ tag: "footer" })), "Rodapé");
eq("header sem pistas → Cabeçalho", deriveSectionName(h({ tag: "header" })), "Cabeçalho");

// ── Id significativo title-case ──
eq("id significativo → title case", deriveSectionName(h({ id: "nossa-historia" })), "Nossa Historia");

// ── Ids genéricos são rejeitados (cai pra undefined) ──
eq("section-3 → undefined", deriveSectionName(h({ id: "section-3" })), undefined);
eq("hash → undefined", deriveSectionName(h({ id: "a1b2c3d4" })), undefined);
eq("plain section sem nada → undefined", deriveSectionName(h({})), undefined);

// ── Prioridade: keyword vence heading ──
eq("keyword vence heading", deriveSectionName(h({ className: "pricing", heading: "Escolha seu plano" })), "Planos");

// ── Desambiguação ──
const names = disambiguate(["Serviços", "Hero", "Serviços", "Serviços", undefined]);
eq("1ª ocorrência sem sufixo", names[0], "Serviços");
eq("2ª ocorrência → Serviços 2", names[2], "Serviços 2");
eq("3ª ocorrência → Serviços 3", names[3], "Serviços 3");
eq("undefined permanece undefined", names[4], undefined);
eq("nome único intacto", names[1], "Hero");

console.log(`\n${pass}/${pass + fail} passaram.`);
process.exit(fail === 0 ? 0 : 1);
