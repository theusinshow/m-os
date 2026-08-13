# M/OS — Design Foundations

## 1. Status

**Status:** aprovado para a fundação da UI desktop v0.1

**Data:** 2026-08-13

Este documento transforma `UX-PRINCIPLES.md` em decisões operacionais para o desktop v0.1.

Ele define estrutura, comportamento, tokens e critérios de qualidade. Não define telas finais. O gate arquitetural foi concluído pelo spike documentado em `TECHNICAL-SPIKE-DESKTOP-SHELL.md`.

## 2. Cena de uso

Um único usuário alterna durante horas entre código, design, documentos e ferramentas técnicas em um monitor Windows, tanto sob luz natural quanto à noite. M/OS precisa aparecer por poucos segundos para capturar ou permanecer aberto por longos períodos para organizar, sem competir com o trabalho principal.

Essa cena exige:

- tema claro e escuro seguindo o Windows;
- contraste e densidade estáveis, não estética baseada apenas em dark mode;
- janela principal silenciosa;
- Quick Capture instantânea e visualmente inequívoca;
- cor com função, nunca como atmosfera;
- navegação previsível que não cresce a cada feature.

## 3. Direção

### Caráter

```text
preciso
silencioso
pessoal
técnico
durável
rápido
```

O M/OS deve parecer uma ferramenta profissional de uso diário. Não deve parecer:

- website dentro de uma janela;
- dashboard SaaS;
- launcher gamer;
- terminal cenográfico;
- demo de inteligência artificial;
- cópia visual de Windows Settings, Notion, Linear ou Raycast.

### Estratégia cromática

**Restrained.** Superfícies neutras e uma cor primária restrita a menos de 10% da interface.

O seed violeta `294.3°` ancora identidade apenas em foco, seleção e ação primária. Ele não colore o fundo, não cria gradientes e não domina a interface. Um ciano claro funciona como segundo acento raro para informação contextual. Estados semânticos possuem cores próprias.

### Tema

O padrão é seguir o tema do Windows. O usuário poderá escolher claro, escuro ou sistema quando Settings entrar no escopo.

Tema claro não é bege. Tema escuro não é azul-marinho. Ambos partem de neutros puros e recebem cromaticidade mínima apenas nas camadas secundárias.

## 4. Princípios aplicados

1. **Capture domina a entrada.** Home nunca compete visualmente com o campo de captura.
2. **Informação antes de container.** Listas e divisores estruturam conteúdo; cards são reservados a itens realmente independentes.
3. **Uma superfície, uma intenção.** Inbox processa. Search encontra. Project contextualiza. Kanban muda estado.
4. **Familiar primeiro.** Controles, seleção, menus, foco e atalhos seguem padrões desktop reconhecíveis.
5. **Expert fast.** Tudo que é frequente funciona com teclado, sem esconder a operação de usuários de mouse.
6. **Complexidade progressiva.** Metadata e relações aparecem no detalhe, não em todas as linhas.
7. **Estado é visível.** Toda ação mostra resultado, falha ou progresso sem bloquear desnecessariamente.
8. **Confiança supera espetáculo.** Commit, persistência e recuperação importam mais que transições elaboradas.

## 5. Arquitetura de informação

### v0.1a

```text
Home
├── Universal Capture
├── Recent Captures
└── acesso a Inbox

Inbox
├── Capture list
└── Capture detail / processing actions

Search
└── Capture results
```

### v0.1b

```text
Home
Inbox
Projects
Tasks
Search
```

### v0.1c

Kanban entra como uma view de Tasks. Não ganha um item permanente separado na navegação.

### Navegação permanente

A navegação primária ocupa uma coluna compacta e estável:

- Home;
- Inbox, com contador apenas quando útil;
- Projects, a partir da v0.1b;
- Tasks, a partir da v0.1b.

Search é uma ação global no topo e via `Ctrl+K`. Quick Capture é global via atalho configurável e não ocupa item de navegação.

Novas features não entram automaticamente na sidebar.

## 6. Estrutura da janela principal

### Dimensões

- viewport de referência: `1280 × 800`;
- mínimo suportado: `840 × 600`;
- sidebar expandida: `208px`;
- rail compacto: `56px` abaixo de `960px` de largura;
- top bar: `48px`;
- inspector opcional: `320px`, somente quando houver espaço acima de `1120px`;
- conteúdo principal: `minmax(0, 1fr)`.

O layout nunca usa fonte fluida por viewport.

### Composição

```text
┌──────────┬─────────────────────────────────────┐
│ sidebar  │ top bar: context + search + actions │
│          ├─────────────────────────────────────┤
│          │                                     │
│          │ primary work surface                │
│          │                                     │
└──────────┴─────────────────────────────────────┘
```

Quando um detalhe precisa de espaço:

- desktop largo: inspector lateral;
- desktop estreito: detalhe substitui temporariamente a lista e mantém Back previsível;
- modal é reservado a decisão que realmente interrompe contexto.

## 7. Superfícies

### Home

Intenção: capturar e retomar contexto recente.

Ordem visual:

1. Universal Capture;
2. Captures recentes;
3. trabalho em andamento, somente quando Tasks existirem;
4. Projects recentes, somente quando Projects existirem.

Não entram métricas, gráficos, feed, calendário ou cards de feature.

### Inbox

Intenção: decidir o destino de Captures.

Em largura normal, usa list-detail. A lista mostra somente:

- trecho do conteúdo;
- origem quando não for óbvia;
- tempo relativo discreto;
- estado de seleção.

Ações de Archive e Trash permanecem secundárias. Processar como Task é contextual e não exige formulário antes de preservar a Capture.

### Search

Intenção: encontrar sem saber o tipo.

Resultados utilizam uma lista única com type label discreto. Capture e derivado aparecem agrupados conforme `CORE-FOUNDATION.md`.

Search abre como command surface por `Ctrl+K` e pode expandir para uma superfície completa quando resultados ou filtros exigirem espaço.

### Project

Intenção: compreender e agir dentro de um contexto.

O overview combina título, descrição curta e Tasks relacionadas sem virar dashboard. Ações raras ficam em menu de overflow.

### Tasks e Kanban

Tasks possuem views segmentadas:

- List;
- Board, a partir da v0.1c.

O controle segmentado troca projeção da mesma informação. Board usa colunas Backlog, Doing e Done, com largura estável e scroll horizontal apenas abaixo do mínimo necessário.

## 8. Quick Capture

Quick Capture é uma janela separada, sempre sobre o contexto atual, sem backdrop ou dimming.

### Geometria

- largura padrão: `640px`;
- largura mínima: `480px` quando o monitor exigir;
- altura inicial: `88px`;
- altura máxima com multiline/erro: `220px`;
- posição: centralizada horizontalmente, aproximadamente no primeiro terço vertical do monitor ativo;
- radius: `8px`;
- shadow curta e definida, sem border simultânea decorativa.

### Fluxo

```text
atalho
  -> campo focado
  -> digitar
  -> Enter
  -> COMMIT
  -> feedback Saved
  -> fechar
  -> restaurar foco anterior
```

### Teclado

- `Enter`: salvar;
- `Shift+Enter`: nova linha;
- `Esc`: fechar sem salvar;
- `Ctrl+Enter`: reservado, não implementado sem função real;
- atalhos continuam visíveis apenas em tooltip ou menu de comando quando necessário.

### Feedback

- sucesso: confirmação visual e anúncio acessível; fechamento entre `120–180ms` depois do commit;
- erro: janela permanece aberta, texto é preservado e a causa é informada;
- loading prolongado não usa spinner central; o botão/estado de envio mostra progresso no próprio lugar;
- reduced motion: confirmação sem transformação e fechamento imediato depois do anúncio acessível.

## 9. Tokens

Todos os tokens cromáticos usam OKLCH.

### Light

```css
:root {
  --mos-bg: oklch(1 0 0);
  --mos-surface-1: oklch(0.975 0.002 294.3);
  --mos-surface-2: oklch(0.945 0.004 294.3);
  --mos-ink: oklch(0.19 0.015 294.3);
  --mos-muted: oklch(0.44 0.018 294.3);
  --mos-border: oklch(0.87 0.006 294.3);
  --mos-primary: oklch(0.50 0.12 294.3);
  --mos-primary-hover: oklch(0.45 0.13 294.3);
  --mos-on-primary: oklch(1 0 0);
  --mos-accent: oklch(0.87 0.08 205);
  --mos-on-accent: oklch(0.22 0.035 220);
  --mos-success: oklch(0.55 0.14 145);
  --mos-warning: oklch(0.68 0.14 78);
  --mos-danger: oklch(0.55 0.18 28);
  --mos-info: oklch(0.56 0.12 230);
}
```

### Dark

```css
[data-theme="dark"] {
  --mos-bg: oklch(0.10 0 0);
  --mos-surface-1: oklch(0.145 0.008 294.3);
  --mos-surface-2: oklch(0.19 0.012 294.3);
  --mos-ink: oklch(0.94 0.006 294.3);
  --mos-muted: oklch(0.70 0.018 294.3);
  --mos-border: oklch(0.29 0.012 294.3);
  --mos-primary: oklch(0.72 0.12 294.3);
  --mos-primary-hover: oklch(0.77 0.13 294.3);
  --mos-on-primary: oklch(0.10 0 0);
  --mos-accent: oklch(0.78 0.10 205);
  --mos-on-accent: oklch(0.10 0.015 220);
  --mos-success: oklch(0.72 0.13 145);
  --mos-warning: oklch(0.78 0.13 78);
  --mos-danger: oklch(0.70 0.17 28);
  --mos-info: oklch(0.72 0.11 230);
}
```

Antes de UI de produto, contrastes precisam ser medidos programaticamente para texto, placeholder, focus, estados e high contrast. Valores podem ser ajustados sem alterar a estratégia.

### Typography

```css
:root {
  --mos-font-ui: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
  --mos-font-mono: "Cascadia Mono", "Consolas", monospace;

  --mos-text-xs: 0.75rem;
  --mos-text-sm: 0.8125rem;
  --mos-text-md: 0.875rem;
  --mos-text-lg: 1rem;
  --mos-title-sm: 1.25rem;
  --mos-title-md: 1.75rem;
}
```

Regras:

- labels e metadata: `12–13px`;
- corpo e controles: `14px`;
- títulos compactos: `16–20px`;
- título de superfície: máximo `28px`;
- letter spacing: `0`;
- monospace somente para IDs, paths ou informação técnica;
- prosa limitada a `70ch`.

### Spacing e geometry

```css
:root {
  --mos-space-1: 4px;
  --mos-space-2: 8px;
  --mos-space-3: 12px;
  --mos-space-4: 16px;
  --mos-space-6: 24px;
  --mos-space-8: 32px;
  --mos-space-12: 48px;

  --mos-radius-sm: 4px;
  --mos-radius-md: 6px;
  --mos-radius-lg: 8px;

  --mos-control-sm: 28px;
  --mos-control-md: 36px;
  --mos-control-lg: 44px;
}
```

Cards não excedem `8px`. Icon buttons possuem dimensões fixas e tooltip quando a ação não for universal.

### Z-index

```text
base 0
sticky 10
dropdown 20
backdrop 30
dialog 40
toast 50
tooltip 60
quick-capture 70
```

## 10. Component vocabulary

### Commands

- primary text button para ação dominante rara;
- icon button para ações universais como Search, Close, Archive e More;
- destructive button somente em contexto destrutivo;
- menu para conjuntos de opções;
- segmented control para views;
- checkbox/toggle para estado binário;
- inline edit para título e texto simples.

Ícones devem vir de Lucide, com stroke e tamanho consistentes. Não serão desenhados SVGs equivalentes manualmente.

### Lists

List row é a unidade principal de Captures, Tasks, Projects e Search.

Estados obrigatórios:

- default;
- hover;
- keyboard focus;
- selected;
- pressed;
- disabled quando aplicável;
- loading local;
- error contextual.

Seleção usa background e focus ring, nunca apenas cor de texto.

### Inputs

- label visível quando o significado não for evidente;
- placeholder nunca substitui label em formulários;
- Universal Capture pode usar a pergunta assinatura por ser uma única intenção dominante;
- error aparece junto do campo e preserva o valor;
- focus ring mínimo de `2px` com contraste verificável.

### Empty states

Empty state usa uma frase curta e uma ação direta. Não inclui ilustração genérica, tutorial ou texto promocional.

Exemplos de direção:

```text
Inbox vazia

Nenhuma Task neste projeto
[Criar Task]
```

## 11. Linguagem e copy

Locale inicial: `pt-BR`.

Nomes conceituais podem permanecer em inglês no código e na documentação, mas a UI não mistura idiomas dentro do mesmo fluxo.

Exceção deliberada: `What's on your mind?` permanece como pergunta assinatura de Universal Capture enquanto uma revisão de conteúdo não decidir sua localização. Botões e feedback continuam em português:

- `Salvar`;
- `Salvo na Inbox`;
- `Não foi possível salvar`;
- `Tentar novamente`;
- `Arquivar`;
- `Mover para a Lixeira`.

Erros sempre informam:

1. o que falhou;
2. se o texto foi preservado;
3. próxima ação possível.

## 12. Keyboard model

Atalhos propostos, sujeitos a conflito do sistema:

- Quick Capture global: configurável, candidato `Ctrl+Shift+Space`;
- Search/Command: `Ctrl+K`;
- fechar overlay/janela transitória: `Esc`;
- confirmar ação focada: `Enter`;
- navegar listas: setas;
- mover foco estrutural: `Tab` e `Shift+Tab`;
- menu contextual: `Shift+F10`;
- voltar: `Alt+Left` quando existir histórico navegável.

Nenhum fluxo crítico depende de drag and drop. Kanban sempre oferece alternativa por menu/teclado.

## 13. Motion

- microtransições: `120ms`;
- mudança de estado ou painel: `160–180ms`;
- limite para operação repetitiva: `200ms`;
- easing: ease-out-quart ou equivalente;
- não animar layout quando transform/opacity explicam a mudança;
- sem page-load choreography;
- sem bounce, elastic, partículas ou glow.

`prefers-reduced-motion` remove deslocamento, encurta duração e preserva feedback por estado.

## 14. Accessibility baseline

- WCAG 2.2 AA como baseline visual;
- texto normal mínimo `4.5:1`;
- texto grande mínimo `3:1`;
- foco, bordas de controles e estados não textuais mínimo `3:1`;
- Narrator e UI Automation no spike;
- semântica correta para list, button, input, tabs e dialogs;
- ordem de foco acompanha ordem visual;
- foco nunca é removido sem destino previsível;
- target de mouse mínimo `28px`; ações frequentes preferem `36px`;
- nenhum estado depende apenas de cor;
- Windows High Contrast precisa manter operação e seleção compreensíveis;
- scaling a `100%`, `125%` e `150%` sem overlap ou truncamento funcional.

## 15. Estados e falhas

### Startup

A shell estável aparece primeiro. Migration ou verificação de integridade usa progresso textual discreto; não simula conteúdo que ainda não existe.

### Local loading

Operações locais rápidas usam optimistic feedback somente quando reversíveis. Capture nunca mostra sucesso antes do commit.

### Empty

Estados vazios preservam a intenção da superfície e oferecem no máximo uma próxima ação dominante.

### Error

Erro de persistência mantém o conteúdo editável. Erro de integridade bloqueia writes e oferece recuperação. Toast não carrega erro que exige decisão.

### Archive e Trash

Archive é reversível e usa feedback com Undo. Trash é visualmente distinto; exclusão definitiva não fica em ação primária.

## 16. Quality gates

Antes da UI de produto ser considerada pronta, cada superfície deve passar por:

1. screenshot em `1280×800`, `1024×768` e `840×600`;
2. scaling Windows `100%`, `125%` e `150%`;
3. tema claro, escuro e High Contrast;
4. navegação completa por teclado;
5. Narrator e árvore UI Automation;
6. contraste programático;
7. long content e strings pt-BR;
8. estados default, hover, focus, pressed, loading, empty, error e disabled;
9. reduced motion;
10. uso repetido sem layout shift ou animação bloqueante.

## 17. Fora desta fundação

- identidade final de marca;
- logo e iconografia própria;
- onboarding;
- mobile/iOS layout;
- voz;
- Hermes;
- Resources visuais;
- gráficos;
- personalização estética;
- design de integrações;
- sons.

Esses itens não devem contaminar o primeiro sistema de componentes.

## 18. Gate de implementação visual

Design foundations estão definidas e foram aplicadas na UI da v0.1a. Os gates concluídos são:

- [x] spike aceitar uma shell desktop;
- [x] ADRs de shell, renderer, Core e persistência serem aceitas;
- [x] contrastes dos tokens medidos no renderer final;
- [x] wireframes comportamentais dos fluxos v0.1a produzidos;
- [x] estados de Capture, Inbox e Search revisados contra `UX-PRINCIPLES.md`.

Contrastes medidos em 2026-08-13:

| Par | Light | Dark |
|---|---:|---:|
| texto principal / canvas | `15.76:1` | `14.39:1` |
| texto secundário / canvas | `7.77:1` | `7.58:1` |
| texto do botão / primary | `6.60:1` | `6.90:1` |
| focus / canvas | `3.79:1` | `7.91:1` |

Todos os pares atendem ao baseline definido nesta fundação.
