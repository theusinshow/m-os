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

- [ ] Estados dos componentes contra `M-OS Components v0.5`: Fields, Controls,
      Rows, Navigation, Overlays, Feedback. Buttons já foi.
- [ ] Os cinco estados que o `AGENTS.md` exige em cada componente: repouso,
      hover, focus, ativo, bloqueado.
- [ ] Varredura de CSS morto e classe órfã (o `.app-name` e o badge do rail já
      apareceram assim; provavelmente há mais).
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
