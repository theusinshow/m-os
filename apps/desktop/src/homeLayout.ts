/**
 * O arranjo da Home: o que existe, onde o desenho pos, e o que a pessoa mudou.
 *
 * Vive fora do `App.tsx` para poder ser testado. Nao ha teste de DOM neste
 * repo por decisao registrada no `vitest.config.ts`, e a consequencia pratica e
 * esta: o que da para verificar tem de ser funcao pura, e por isso a regra do
 * arranjo mora aqui em vez de dentro dos callbacks do componente.
 *
 * A METADE DE REGISTRO desta regra e `arrange_widgets`, em
 * `crates/mos-core/src/work.rs`. As duas existem porque resolver o arranjo e
 * trabalho de cada render, e um round-trip por render para ordenar quinze itens
 * custaria muito mais que a duplicacao. Mudar uma sem a outra e o defeito a
 * evitar — as duas tem os mesmos casos nos testes, de proposito.
 */
import type { WidgetPlacement, WidgetPlacementInput } from "./types";

export type HomeWidgetRole = "focus" | "attention" | "overview" | "collection" | "utility";
export type HomeWidgetSpan = 3 | 4 | 5 | 6 | 8 | 9 | 12;

/* As larguras que a INTERFACE oferece. O banco aceita 1..12 (migration 0017):
   la e forma, aqui e vocabulario do desenho, e por isso a lista vive deste
   lado. Os degraus saltados — sem 7, sem 10, sem 11 — sao os que dividem doze
   de um jeito que a linha fecha com um vizinho. */
export const HOME_SPANS: HomeWidgetSpan[] = [3, 4, 5, 6, 8, 9, 12];

/* As faixas, na ordem em que a Home as apresenta. A ordem delas e do desenho e
   nao se arruma: o que a pessoa arruma e o que mora DENTRO de cada faixa. */
export const HOME_SECTIONS: { id: string; title: string }[] = [
  { id: "now", title: "Agora" },
  { id: "resume", title: "Retomar" },
  { id: "overview", title: "Visão" },
  { id: "collection", title: "Acervo" },
  { id: "utilities", title: "Utilidades" },
];

/* Fonte de verdade unica dos widgets da Home: o que existe, como se chama, em
   que faixa o desenho o pos, que papel ele cumpre e que largura ele tem.

   Os ids VAO PARA O BANCO: renomear um deles apaga em silencio a escolha de
   quem tinha ocultado, movido ou redimensionado o widget, porque a linha
   guardada deixa de casar com qualquer widget do catalogo. O rotulo pode mudar
   a vontade; o id, nunca.

   A ORDEM tambem importa, e de dois jeitos. Dentro da faixa ela e a ordem que a
   Home mostra para quem nunca arrumou nada — e por isso ela segue o desenho, e
   nao o alfabeto. E entre widgets sem posicao guardada ela e o desempate: e o
   que faz um widget criado depois ir para o FIM de uma Home ja arrumada, em vez
   de se enfiar no meio da escolha de alguem. */
export const HOME_WIDGETS: { id: string; label: string; section: string; role: HomeWidgetRole; span: HomeWidgetSpan }[] = [
  { id: "now", label: "EM ANDAMENTO", section: "now", role: "focus", span: 6 },
  { id: "timer", label: "CRONÔMETRO", section: "now", role: "focus", span: 3 },
  { id: "today_hours", label: "HOJE", section: "now", role: "focus", span: 3 },
  { id: "inbox_pulse", label: "INBOX", section: "resume", role: "attention", span: 3 },
  { id: "recent", label: "RECENTES", section: "resume", role: "attention", span: 5 },
  { id: "projects", label: "PROJECTS", section: "resume", role: "attention", span: 4 },
  { id: "month_density", label: "MÊS", section: "overview", role: "overview", span: 6 },
  // Ids novos e nao renomeados: `week_rings` continua sendo a semana de TASKS, e
  // reaproveitar o id daria a quem ocultou um o outro escondido sem ter pedido.
  { id: "week_rings", label: "SEMANA", section: "overview", role: "overview", span: 6 },
  { id: "week_by_project", label: "SEMANA POR PROJECT", section: "overview", role: "overview", span: 6 },
  { id: "task_progress", label: "CONCLUÍDO", section: "overview", role: "overview", span: 3 },
  { id: "budget_ring", label: "META", section: "overview", role: "overview", span: 3 },
  { id: "recent_resources", label: "RECURSOS", section: "collection", role: "collection", span: 8 },
  { id: "apps", label: "APPS", section: "collection", role: "collection", span: 4 },
  { id: "quick_actions", label: "AÇÕES", section: "utilities", role: "utility", span: 6 },
  { id: "system_health", label: "SISTEMA", section: "utilities", role: "utility", span: 6 },
];

/* Um widget ja resolvido: onde ele esta e que largura tem, DEPOIS de aplicar o
   que foi guardado por cima do desenho.

   `savedSpan` anda junto por um motivo que o teste
   `reordering_must_carry_the_stored_width_along` guarda no Rust: a escrita e
   autoritativa, entao reordenar precisa REPASSAR a largura ja guardada. Mandar
   o `span` resolvido congelaria o desenho na primeira arrastada; mandar `null`
   apagaria a escolha da pessoa. O valor certo e este, e por isso ele viaja. */
export type ArrangedWidget = { id: string; label: string; role: HomeWidgetRole; section: string; span: HomeWidgetSpan; savedSpan: number | null };

/**
 * Resolve o catalogo contra o que foi guardado: faixa, largura e ordem.
 *
 * Tres regras, e cada uma existe para um caso que da errado sozinho:
 *
 * 1. campo guardado vazio significa o que o DESENHO escolheu, e nao zero;
 * 2. dentro da faixa, quem tem posicao guardada vem primeiro, na ordem dela;
 *    quem nao tem cai para o fim, na ordem do catalogo. Widget novo se enfiando
 *    no meio de um arranjo que a pessoa montou seria o sistema desfazendo a
 *    escolha dela;
 * 3. posicao repetida ou salteada nao some com ninguem — o desempate e a ordem
 *    do catalogo. Banco meio gravado nao pode custar um widget.
 */
export function arrangeHome(placements: WidgetPlacement[], workspaceId: string): ArrangedWidget[] {
  const saved = new Map(placements.filter((entry) => entry.workspaceId === workspaceId).map((entry) => [entry.widgetId, entry] as const));
  const bands = HOME_SECTIONS.map((section) => section.id);

  const resolved = HOME_WIDGETS.map((widget, index) => {
    const entry = saved.get(widget.id);
    /* Faixa guardada que o desenho nao conhece mais volta para a de origem. A
       Home nao tem titulo para desenhar uma faixa que nao existe, e o widget
       sumir junto com ela seria muito pior que ele voltar para casa. */
    const section = entry?.section && bands.includes(entry.section) ? entry.section : widget.section;
    return { id: widget.id, label: widget.label, role: widget.role, section, span: (entry?.span ?? widget.span) as HomeWidgetSpan, savedSpan: entry?.span ?? null, position: entry?.position ?? null, index };
  });

  const bandOf = (section: string) => { const at = bands.indexOf(section); return at < 0 ? bands.length : at; };
  resolved.sort((left, right) => {
    const band = bandOf(left.section) - bandOf(right.section);
    if (band !== 0) return band;
    if (left.position !== null && right.position !== null) return left.position - right.position || left.index - right.index;
    if (left.position !== null) return -1;
    if (right.position !== null) return 1;
    return left.index - right.index;
  });
  return resolved.map(({ position: _position, index: _index, ...slot }) => slot);
}

/**
 * Move um widget para antes de `before`, ou para o fim da faixa quando ele e
 * nulo.
 *
 * A mira e um VIZINHO, e nao um indice, de proposito: a lista muda de tamanho
 * quando o widget sai dela, e todo indice calculado antes da remocao erra por
 * um em metade dos casos — no arrasto da esquerda para a direita, e nas setas
 * "para frente". Um vizinho continua sendo o mesmo vizinho depois da remocao.
 */
export function moveInArrangement(arrangement: ArrangedWidget[], widgetId: string, section: string, before: string | null): ArrangedWidget[] {
  const atual = arrangement.find((slot) => slot.id === widgetId);
  if (!atual) return arrangement;

  const next = arrangement.filter((slot) => slot.id !== widgetId);
  const movido = { ...atual, section };
  const destino = next.findIndex((slot) => slot.id === before && slot.section === section);
  if (before !== null && destino >= 0) {
    next.splice(destino, 0, movido);
    return next;
  }
  /* Sem mira, entra depois do ultimo da faixa de destino — e nao no fim da
     lista. A lista e plana: empurrar para o fim dela poria o widget depois de
     todas as OUTRAS faixas, e a proxima escrita gravaria essa ordem errada. */
  let ultimo = -1;
  next.forEach((slot, at) => { if (slot.section === section) ultimo = at; });
  next.splice(ultimo + 1, 0, movido);
  return next;
}

/** Quais faixas uma mudanca de `widgetId` para `section` obriga a regravar. */
export function touchedSections(arrangement: ArrangedWidget[], widgetId: string, section: string): string[] {
  const origem = arrangement.find((slot) => slot.id === widgetId)?.section;
  /* A faixa de ORIGEM tambem mudou: quem ficou la subiu uma posicao. Gravar so
     o destino deixaria a origem com um buraco na numeracao. */
  return origem === undefined || origem === section ? [section] : [origem, section];
}

/**
 * O que vai para o banco: as faixas pedidas, cada uma renumerada de zero.
 *
 * `savedSpan` e nao `span` — ver o comentario de `ArrangedWidget`. Este e o
 * unico lugar do front que monta a escrita, e e de proposito: a escrita e
 * autoritativa campo por campo, e espalhar isso por varios lugares seria a
 * forma mais facil de apagar em silencio a largura de alguem.
 */
export function placementsFor(arrangement: ArrangedWidget[], sections: string[]): WidgetPlacementInput[] {
  return sections.flatMap((section) =>
    arrangement.filter((slot) => slot.section === section).map((slot, position) => ({ widgetId: slot.id, position, section, span: slot.savedSpan })),
  );
}

/**
 * O degrau seguinte de largura, ou `null` quando ja esta no fim da escala.
 *
 * Procura o proximo VALOR e nao o proximo indice, o que faz diferenca para uma
 * largura que nao esta na escala. O banco aceita 1..12 (a migration 0017 guarda
 * forma, nao vocabulario), entao um dia pode chegar aqui um 7 — de um banco
 * editado a mao, ou de uma versao com outra escala. Procurando por indice, esse
 * 7 travaria os dois botoes e a pessoa ficaria presa numa largura que a
 * interface nem oferece. Procurando por valor, ele sobe para 8 e desce para 6.
 */
export function stepSpan(span: number, direction: -1 | 1): HomeWidgetSpan | null {
  return direction === 1
    ? HOME_SPANS.find((candidato) => candidato > span) ?? null
    : [...HOME_SPANS].reverse().find((candidato) => candidato < span) ?? null;
}
