# Polimento autônomo — backlog vivo

Estado do trabalho autônomo. Sobrevive entre ciclos: cada volta lê este arquivo,
faz o próximo item e marca o resultado. Sem ele o contexto se perde no wakeup.

**Branch:** `feat/polimento-autonomo` — nunca commitar em `master`.
**Relatório:** só ao final, por decisão do proprietário.

## Limites acordados

Autonomia total, incluindo features novas — decisão explícita do proprietário,
tomada depois de eu apontar que contraria o `AGENTS.md` do repo. Registrado aqui
para não virar mal-entendido depois.

Mitigação que eu aplico por conta própria: **feature nova sai do `ROADMAP.md`,
não da minha cabeça.** O produto tem sequenciamento pensado e um `IDEAS.md` que
o roadmap deliberadamente não puxou. Inventar escopo contra isso é retrabalho.
Cada decisão vira entrada em `DECISIONS.md`.

## Visão — resolvido pela metade

A CLI **funciona**. O subcomando é `orca computer`, não `orca computer-use` —
foi esse chute errado que me fez concluir que ela não existia. O guia
versionado sai de `orca skills get computer-use` e deve ser lido a cada sessão,
porque os flags mudam entre releases.

Loop principal:

```text
orca computer list-apps --json
orca computer list-windows --app mos-desktop --json
orca computer get-app-state --app mos-desktop --window-id <id> --json
```

O que **ainda** bloqueia: o `mos-desktop` tem uma única janela, de 16×16,
titulada `com.codedbym.mos-siw` — a auxiliar de single-instance. A janela
principal não está aberta; o processo vive só como tray. Não há o que olhar até
alguém abrir a interface.

Quando ela abrir, `get-app-state` devolve a árvore de acessibilidade em
`result.snapshot.treeText`, e os índices de elemento são efêmeros: eles vencem
a cada re-render, scroll ou troca de foco. Reler o estado antes de cada ação.

Enquanto isso, o trabalho segue no que tem verdade objetiva sem precisar de
olhos: comparação com o HTML dos protótipos, estados de componente, CSS morto,
disciplina de token, CI.

## Fila

### Sem precisar de olhos

- [x] Buttons — 13px, destrutivo só em outline.
- [x] Fields — cinco estados; vazio e preenchido têm bordas diferentes.
- [x] Rows — estrutura confirmada; faltava a project row com barra de progresso,
      agora implementada a partir das tasks concluídas.
- [x] Controls — checkbox desenhado no lugar do nativo.
- [x] Feedback — nada quebrado; duas nuances registradas abaixo.
- [x] Navigation e Overlays — **não usar a folha aqui.** Ela mostra um rail
      antigo (símbolo de 18px, linha divisória, ícones de 19px, gap 20) que a
      v0.7 substituiu. Etags: folha `1786597925`, v0.7 `1786674649`.

**Regra de precedência que vale para as próximas voltas:** a folha de
componentes é mais antiga que a v0.7 em tudo. Ela só manda onde a v0.7 é
omissa — os cinco estados de botão, campo e controle, que nenhuma tela mostra.
Onde as duas falam da mesma coisa, a v0.7 ganha. Minerar a folha para layout de
tela é reintroduzir desenho velho.
- [x] Varredura de seletor duplicado. Achou uma colisão real entre componentes
      (`.hermes-turn` valia para o Command e para a tela ao mesmo tempo) e duas
      duplicatas minhas. Os sete restantes são grupo compartilhado + bloco
      específico, que é composição intencional.
- [ ] Varredura de classe órfã (CSS sem uso no TSX).
- [ ] `/code-review` no diff acumulado da série.
- [ ] Inbox: comparar o pane esquerdo linha a linha (só o direito foi conferido).
- [ ] Projects: lista da esquerda ainda não comparada com o protótipo.

### Precisa de olhos

- [ ] Conferir as seis telas contra `M-OS Redesign v0.7 - Telas.dc.html`.
- [ ] Light mode em paridade nas seis telas.
- [ ] `prefers-reduced-motion`.
- [ ] Navegação por teclado e foco visível em 840×600.
- [ ] Tela do Hermes contra `M-OS Hermes - Chat Completo v0.1`.

### Features do roadmap

Só depois do polimento — corrigir o que existe vale mais que somar o que falta.

- [ ] Fase 3 · Context: relações entre informações, Project Context, recents.
- [ ] Fase 4 · Time: reminders e Today. **Destrava o `Agendar` que o desenho do
      Hermes mostra e que hoje não tem lastro no domínio.**
- [ ] Fase 5 · GitHub: a integração de fato, além do campo `repository`.

## Registro

| Volta | O que foi feito | Resultado |
|---|---|---|
| 1 | Branch, backlog, tentativa de visão | CLI ausente; app sem janela; seguindo sem olhos |
| 2 | Fields, Rows com progresso, varredura de duplicados | colisão `.hermes-turn` corrigida; app segue sem janela |
| 3 | Classes órfãs (nenhuma), Feedback, botão pequeno de criação | 170 classes todas em uso; app segue sem janela |

## Nuance registrada

A folha de componentes v0.5 diz **8 s** de janela de undo; a v0.7 e o README
dizem ~5,2 s. Não é contradição: a folha fala de **ação em lote**, a v0.7 mostra
o recibo de **ação única**. Não há ação em lote no app hoje, então 5,2 s vale.
Quando lote existir, 8 s.

A folha também define um **recibo passivo** de 1,6 s — sem botão, sem fechar,
só um ponto de 4px e um rótulo mono. É outro componente, não o de undo.
Não implementei: não há chamador hoje, e componente sem chamador é peso morto.
| 4 | Checkbox desenhado, cinco estados | switch de tema conferido, sem regressão; app segue sem janela |
| 5 | Precedência folha vs v0.7; `/code-review` na ponte e na migration | folha superada para layout; revisão em background |
