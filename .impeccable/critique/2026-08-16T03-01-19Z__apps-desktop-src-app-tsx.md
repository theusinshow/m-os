---
target: M/OS desktop, foco na Home
total_score: 29
p0_count: 1
p1_count: 2
timestamp: 2026-08-16T03-01-19Z
slug: apps-desktop-src-app-tsx
---
# Crítica — M/OS desktop, com foco na Home

## Design Health Score

| # | Heurística | Nota | Questão central |
|---|-----------|-------|-----------|
| 1 | Visibilidade do estado do sistema | 3 | forte (busy na topbar, streaming, conexão, recibos, aria-live); os buracos que existiam eram bugs, já corrigidos |
| 2 | Correspondência com o mundo real | 2 | três estados vazios em inglês no meio de uma interface em português — corrigido nesta passagem |
| 3 | Controle e liberdade | 4 | Undo com Ctrl+Z, Esc em toda superfície, cancelar em todo formulário, arquivar/restaurar |
| 4 | Consistência e padrões | 3 | design system unificado e um único acento; a mistura de idioma era a exceção |
| 5 | Prevenção de erro | 4 | confirmação para destrutivo, ambiguidade recusa, esquema valida, exclusão recusa o que está ativo |
| 6 | Reconhecer em vez de lembrar | 3 | ícones rotulados, rail de 8 destinos, command palette; a Home com 11 widgets pesa contra |
| 7 | Flexibilidade e eficiência | 4 | Ctrl+K, Ctrl+Z, Ctrl+N, setas nas listas, atalho global, ⌘1-9 nos apps |
| 8 | Estética e design minimalista | 2 | **o ponto fraco.** 11 widgets contra o próprio PRODUCT.md §4 |
| 9 | Recuperação de erro | 3 | mensagens de domínio legíveis; o recibo do updater respondia fora de vista, corrigido |
| 10 | Ajuda e documentação | 1 | não há ajuda em app, nem onboarding, nem tooltip sistemático |
| **Total** | | **29/40** | **Bom — base sólida, com áreas fracas nomeadas** |

## Veredito de anti-padrões

**Avaliação do modelo:** não parece feito por IA. O sistema tem decisões próprias e defensáveis — sódio como único acento, régua e rótulo mono em vez de card, ausência deliberada de contagem onde ela mentiria. Nenhum dos banimentos absolutos aparece: sem gradiente em texto, sem glassmorphism, sem borda lateral colorida, sem eyebrow em toda seção, sem grade de cards idênticos.

**Varredura determinística:** o detector achou **um** item em todo o renderer — `transition: width` em `.row-progress i` (App.css). Corrigido para `scaleX`. Nova varredura retorna vazio, exit 0.

**Overlays visuais:** não executados. Sem automação de browser nesta sessão; a janela do Tauri não é inspecionável daqui.

## Impressão geral

O sistema é maduro e coeso. O problema da Home não é gosto nem excesso de widgets: é **matemática de grade**.

São 12 colunas com larguras misturadas de 6 e 3, mais dois widgets de duas linhas. O auto-placement esparso não reorganiza: quando o próximo item não cabe no que sobrou da linha, ele desce e deixa o vão aberto. Na ordem anterior isso abria **três buracos de 3 colunas** nas linhas 2 e 3. É isso que se lê como bagunça.

## O que está funcionando

**A régua de decisão sobre contagem.** O badge da Inbox foi removido porque mentia duas vezes, e o comentário no código registra o porquê. Sistemas raramente removem um número; quase sempre o mantêm errado.

**O vocabulário de estados.** Toda ação destrutiva confirma, toda ação reversível oferece Undo, e a exclusão definitiva recusa o que ainda está ativo. É raro ver essa disciplina completa.

**Atalhos sem custo para o iniciante.** Ctrl+K, Ctrl+Z, setas nas listas, ⌘1-9 — todos invisíveis até serem procurados.

## Problemas prioritários

### [P0] A grade da Home abria buracos no meio — CORRIGIDO
Três vãos de 3 colunas nas linhas 2 e 3. Reordenado para cada faixa fechar em 12; só sobra vão no fim, que é o único lugar onde vão não parece defeito. Preferido a `grid-auto-flow: dense`, que taparia movendo itens para trás e faria a ordem visual divergir da de foco.

### [P1] A Home tem 11 widgets, e o PRODUCT.md avisa contra isso
O documento diz: *"A Home não deve se transformar em um dashboard sobrecarregado."* Onze widgets do mesmo peso visual violam a regra de carga cognitiva (≤4 itens por grupo) e o próprio brief. Existe mecanismo de ocultar, então **isto é decisão de produto, não de implementação** — não removi nada.
**Comando sugerido:** `$impeccable distill` sobre a Home.

### [P1] Três estados vazios em inglês — CORRIGIDO
"Nothing in progress right now" ao lado de "Workspaces ativos aparecerão aqui", na mesma grade. Traduzidos e reescritos para ensinar a interface.

### [P2] Ajuda e onboarding não existem
Nota 1 de 10. Usuário único e autor do sistema torna isso menos urgente, mas o custo aparece depois de um mês sem abrir uma tela.
**Comando sugerido:** `$impeccable onboard`.

### [P2] Animação de largura na barra de progresso — CORRIGIDO
`transition: width` numa barra que vive em listas de dezenas de linhas.

## Bandeiras por persona

**Alex (power user):** bem servido. Atalhos para tudo, Esc em toda superfície, navegação por setas.

**Sam (dependente de acessibilidade):** foco visível em todo o app, `aria-live` nos anúncios, `prefers-reduced-motion` coberto em três folhas. A grade não usa `dense`, então ordem visual e ordem de foco coincidem. Ponto fraco: alguns botões dependem de `title` em vez de rótulo.

**Riley (testador de limites):** empty states agora ensinam; o corte silencioso em 5 nos Projects já tinha sido tratado com link condicional.

## Observações menores

- `HomePage` recebe 23 props. Não é problema de usuário, mas é o sinal de que a Home acumulou responsabilidade.
- O placeholder "What's on your mind" fica em inglês por decisão de produto: está escrito assim no PRODUCT.md §24.

## Perguntas

- A Home responde "o que está acontecendo e o que preciso fazer?" com onze widgets, ou responderia melhor com quatro?
- Os sete widgets que dependem de dado inexistente vão chegar. A grade suporta dezoito?
- Qual widget você olha primeiro ao abrir? Ele está na posição que essa resposta merece?
