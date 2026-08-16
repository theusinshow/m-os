# Pesquisa de referencias UX para M Finance

Data: 2026-08-11

Escopo considerado: cockpit mensal de contas, vencimentos, faturas simples, lembretes e decisao de compra. Fora de escopo: transacoes detalhadas, extrato, banco, investimentos, OpenFinance e automacao pesada.

## Sintese executiva

O melhor posicionamento para o M Finance e ser uma "mesa de controle do mes": o usuario entra para saber o que vence, o que ja foi pago, quanto ainda esta comprometido e se uma compra cabe agora. As referencias atuais mostram que apps grandes como Rocket Money, Copilot, Monarch e YNAB tendem a crescer para uma suite financeira ampla; para o M Finance, vale copiar a clareza operacional de recorrencias, alertas e planejamento, mas evitar copiar a ambicao de "ver tudo da vida financeira".

Ideias mais implementaveis em Next/Tailwind:

1. Dashboard mensal com uma faixa de status: "Hoje", "Proximos 7 dias", "Restante do mes" e "Ja resolvido".
2. Linha do tempo/calendario de vencimentos com contas agrupadas por semana, nao por categoria financeira abstrata.
3. Cartao "comprometido este mes" separado de "a decidir", para compra planejada nao virar lancamento.
4. Faturas simples por cartao: total estimado, vencimento, fechamento, itens manuais e alerta de fechamento.
5. Alertas acionaveis: "pagar", "adiar lembrete", "marcar como pago", "ver impacto no mes".
6. Decisao de compra como calculadora contextual: valor, cartao/forma, vencimento afetado, impacto em fatura atual/proxima e resultado legivel.

## Fontes usadas

- Rocket Money, "Manage Subscriptions": https://www.rocketmoney.com/feature/manage-subscriptions
- Copilot Money, Dispatch/updates: https://www.copilot.money/dispatch
- Copilot Help, Web app: https://help.copilot.money/en/articles/11780342-copilot-money-for-web
- YNAB Help, plan/edit plan/cost to be me: https://support.ynab.com/en_us/plan-and-adjust-with-edit-plan-and-cost-to-be-me-ByR7vpqPyx
- NN/g, empty states in complex applications: https://www.nngroup.com/articles/empty-state-interface-design/
- Carbon Design System, empty states: https://carbondesignsystem.com/patterns/empty-states-pattern/
- Apple HIG, notifications: https://developer.apple.com/design/human-interface-guidelines/notifications
- Material Design 3, badges: https://m3.material.io/components/badges
- Setproduct, dashboard UI design 2026: https://www.setproduct.com/blog/dashboard-ui-design
- Bach et al., Dashboard Design Patterns: https://arxiv.org/abs/2205.00757

## 1. Padroes UI/UX interessantes

### Centro de recorrencias

Referencia: Rocket Money posiciona assinaturas e contas recorrentes como um "control center", com lista unica e foco em detectar aquilo que o usuario esqueceu ou nao precisa mais.

Aplicacao para M Finance:

- Tela "Recorrentes" com aluguel, internet, energia, streamings, academia e assinaturas.
- Cada item com valor, vencimento, metodo, status do mes e proximo lembrete.
- Diferenciar "conta fixa", "assinatura cancelavel" e "fatura/cartao".
- Evitar copiar cancelamento concierge ou negociacao de conta; isso exige operacao e vira outro produto.

Componentes Next/Tailwind:

- `RecurringItemCard` compacto, com icone, nome, valor, proximo vencimento, badge de status e menu de acoes.
- Filtros por `todas`, `vence em 7 dias`, `sem lembrete`, `assinaturas`.
- Barra lateral ou drawer de detalhe, sem navegar para um fluxo pesado.

### Briefing proativo

Referencia: Copilot vem explorando assistente/briefings proativos que "surfacing what matters" antes de o usuario procurar. O ponto bom nao e chat; e priorizacao.

Aplicacao para M Finance:

- Um bloco no topo: "Precisa de atencao".
- Maximo 3 mensagens por dia/mes, por exemplo:
  - "Sua fatura Nubank fecha em 2 dias; compras acima de R$ X devem cair na proxima."
  - "Voce tem R$ 740 vencendo ate sexta."
  - "A compra de R$ 380 cabe melhor no cartao X porque vence depois do dia 10."
- Nao abrir chat financeiro nem IA generica. A recomendacao pode ser regra simples e explicavel.

Componentes:

- `AttentionStrip` com severidade discreta: info, aviso, urgente.
- `InsightCard` com uma frase, um numero principal e uma acao.
- Estado "dispensado" persistido para nao repetir o mesmo aviso.

### Secao "esperado este mes"

Referencia: Copilot Web mostra gasto total/medio, gasto/budget/restante por categoria, e uma secao "Expected this month" com recorrencias futuras.

Aplicacao para M Finance:

- Trocar "budget por categoria" por "compromissos do mes".
- Mostrar:
  - total previsto do mes;
  - total ja pago;
  - total ainda aberto;
  - maior vencimento pendente;
  - dias ate o proximo vencimento.
- Para compra, mostrar impacto no aberto: "se comprar agora, fatura aberta vai para R$ X".

Componentes:

- `MonthlyCommitmentSummary` com 3 KPIs pequenos e um progress bar.
- `OpenCommitmentsList` ordenada por data, nao por valor.
- `MonthSwitcher` simples, com atual como default.

### Planejamento mensal opinativo

Referencia: YNAB tem forte linguagem de planejamento e usa conceitos como transacoes agendadas, metas e "Cost to Be Me" para explicar o custo de financiar o mes.

Aplicacao para M Finance:

- Criar um indicador equivalente, mas mais simples: "Custo fixo do mes".
- Nao implementar orcamento zero-based. Isso mudaria o produto.
- Usar o custo fixo como ancora para decisao de compra:
  - "Com as contas cadastradas, este mes ja tem R$ X comprometidos."
  - "Depois desta compra, faltariam R$ Y ate o limite definido por voce."

Componentes:

- `FixedMonthCostCard`.
- `PurchaseDecisionPanel` com resultado `Cabe`, `Aperta`, `Melhor jogar para proxima fatura`.
- `UserLimitInput` opcional: limite mensal pessoal, nao saldo bancario.

### Dashboard operacional, nao analytics

Referencia: guias de dashboards enfatizam tempo ate resposta, hierarquia e proximo passo. O erro comum e transformar a tela em despejo de dados.

Aplicacao para M Finance:

- O dashboard deve responder em ate 5 segundos:
  - "O que vence agora?"
  - "O que falta pagar?"
  - "Minha proxima fatura esta sob controle?"
  - "Posso fazer esta compra?"
- Graficos devem ser poucos. Melhor uma timeline/agenda e KPIs pequenos do que pizza, area chart e ranking de categorias.

Componentes:

- Layout de 2 colunas no desktop: agenda do mes + painel de decisao/alertas.
- Mobile: uma coluna com "proxima acao" no topo e tabs/chips para `Hoje`, `Semana`, `Mes`.
- Usar Recharts apenas onde ha comparacao util, por exemplo "comprometido por semana do mes".

## 2. Ideias que cabem no M Finance sem virar app generico

### Cockpit mensal

Tela inicial:

- Header: mes atual, total previsto, pendente e proximo vencimento.
- Bloco principal: "Proxima acao" com a conta/fatura mais urgente.
- Agenda: grupos `Hoje`, `Amanha`, `Esta semana`, `Depois`.
- Painel lateral: faturas abertas e decisao de compra.

Valor: reforca o posicionamento de cockpit, reduz navegação e nao exige conta bancaria.

### Janela de vencimento

Ao cadastrar uma conta, permitir:

- vencimento fixo;
- lembrete N dias antes;
- tolerancia/urgencia;
- recorrencia mensal;
- valor fixo ou variavel.

Para valor variavel, o app pode mostrar "valor a confirmar" em vez de inventar previsao.

### Fatura simples

Por cartao:

- melhor dia de compra;
- dia de fechamento;
- dia de vencimento;
- fatura atual estimada;
- fatura proxima estimada;
- compras manuais simples, sem extrato.

Boa regra de produto: item de fatura e apenas um compromisso manual para decisao e lembranca, nao uma transacao contabil completa.

### Simulador "posso comprar?"

Entrada minima:

- valor da compra;
- cartao/forma;
- parcelamento simples;
- data da compra.

Saida:

- fatura afetada;
- vencimento real;
- novo total estimado;
- alerta se passa do limite pessoal definido;
- sugestao simples: comprar agora, esperar fechamento, parcelar, ou rever.

Implementacao:

- pode ser um drawer persistente no dashboard.
- nao precisa IA no v1; regras deterministicas sao mais confiaveis.

### Revisao semanal de contas

Uma tela ou modal curto:

- "3 contas vencem esta semana."
- "2 contas sem valor confirmado."
- "1 fatura fecha antes do fim de semana."
- "Marcar revisado" com timestamp.

Isso cria habito sem virar gamificacao financeira.

### Lembretes multi-canal controlados

Como o projeto tem `twilio` e `web-push`, cabem:

- push do navegador;
- WhatsApp/SMS opcional;
- email se ja existir infraestrutura.

Regra UX: lembretes devem ser configuraveis por tipo e horario. Nao enviar lembrete para todo evento por default.

## 3. Microinteracoes, empty states e alertas uteis

### Microinteracoes

- Marcar como pago: item desliza para "resolvido", contador pendente diminui, snackbar com `desfazer`.
- Adiar lembrete: menu rapido `amanha`, `em 3 dias`, `no vencimento`.
- Confirmar valor variavel: campo inline no card; ao salvar, recalcula total do mes sem trocar de pagina.
- Compra simulada: ao digitar valor, atualizar em tempo real a fatura afetada e o status `Cabe/Aperta`.
- Fechamento de fatura: badge muda de `aberta` para `fecha em 2 dias`, depois `fechada`.
- Progresso do mes: barra segmentada por pago/pendente/atrasado, evitando grafico grande.
- Drag mental, nao drag fisico: permitir "mover para proximo mes" por botao claro, nao exigir DnD em mobile.

### Empty states

Base em NN/g e Carbon: empty state deve explicar o estado, ensinar a funcao e dar caminho direto.

Exemplos:

- Dashboard sem contas:
  - Titulo: "Monte seu cockpit de agosto"
  - Texto: "Adicione uma conta fixa ou uma fatura para ver vencimentos e lembretes aqui."
  - Acoes: `Adicionar conta`, `Adicionar cartao`

- Sem vencimentos na semana:
  - Titulo: "Nada vence esta semana"
  - Texto: "Seu proximo compromisso e em 18/08."
  - Acao: `Ver mes completo`

- Fatura sem compras:
  - Titulo: "Fatura limpa por enquanto"
  - Texto: "Use o simulador antes de cadastrar uma compra."
  - Acoes: `Simular compra`, `Adicionar compra`

- Sem alertas:
  - Titulo: "Nenhum alerta ativo"
  - Texto: "Voce sera avisado quando algo estiver perto do vencimento, atrasado ou sem valor confirmado."
  - Acao secundaria: `Configurar lembretes`

- Filtro sem resultado:
  - Titulo: "Nenhum vencimento neste filtro"
  - Texto: "Tente ver o mes completo ou limpar os filtros."
  - Acoes: `Limpar filtros`, `Ver mes`

### Alertas

Niveis recomendados:

- Info: "Fatura fecha em 5 dias."
- Aviso: "Conta vence amanha."
- Critico: "Conta atrasada ha 2 dias."
- Confirmacao: "Conta marcada como paga."

Regras:

- Critico deve ser raro e visualmente claro.
- Info nao deve competir com vencimento real.
- Toda mensagem deve ter acao proxima.
- Badges devem indicar status/count; evitar transformar badge em decoracao.
- Notificacao externa so para eventos importantes e consentidos.

Exemplos de textos:

- "Energia vence amanha. Marcar como paga ou lembrar no fim do dia?"
- "Sua fatura fecha em 2 dias. Compras novas podem cair na proxima fatura."
- "Compra de R$ 420: cai na fatura de 10/09 e deixa R$ 1.840 estimados."
- "Valor da agua ainda nao foi confirmado. Use o valor do mes passado?"
- "Voce adiou este lembrete 2 vezes. Quer mudar a data padrao?"

## 4. Riscos de copiar padroes ruins

### Virar dashboard de vaidade

Risco: muitos KPIs, graficos e categorias fazem o app parecer completo, mas pioram a pergunta central do usuario.

Evitar:

- pizza de gastos;
- ranking de categorias;
- patrimonio liquido;
- investimentos;
- score financeiro inventado.

Melhor:

- vencimento, valor, status, impacto e acao.

### Misturar pago, previsto e simulado

Risco: mostrar compra simulada como se fosse compromisso real; misturar conta paga com vencimento futuro; incluir lembrete futuro no "gasto ate agora".

Mitigacao:

- usar estados separados: `pago`, `pendente`, `atrasado`, `previsto`, `simulado`.
- rotulos explicitos em totais: "pago", "aberto", "estimado".
- simulacao nunca altera totals reais sem confirmacao.

### Copiar OpenFinance sem ter OpenFinance

Risco: UI prometer atualizacao automatica, deteccao de assinaturas ou extrato sem integracao bancaria.

Mitigacao:

- assumir manual-first.
- usar "cadastre", "confirme", "estime".
- nao usar linguagem de "detectamos" a menos que a informacao venha de fato de automacao confiavel.

### Alert fatigue

Risco: todo vencimento vira notificacao, o usuario desliga tudo.

Mitigacao:

- preferencias por tipo de alerta;
- quiet hours;
- resumo diario/semanal;
- deduplicacao;
- escalonamento so quando vira atraso.

### IA como interface principal

Risco: chat financeiro pode parecer poderoso, mas para contas mensais o usuario precisa de certeza, nao conversa.

Mitigacao:

- se usar IA, limitar a "explicacao" ou "resumo", com aprovacao antes de editar.
- manter acoes principais deterministicamente visiveis.

### Humor em estado financeiro sensivel

Risco: empty states e erros com piada podem soar inadequados quando ha atraso ou falta de dinheiro.

Mitigacao:

- linguagem direta, calma e util.
- diferenciar estado neutro ("nada vence") de estado sensivel ("conta atrasada").

### Mobile com excesso de densidade

Risco: tentar colocar o dashboard desktop inteiro no celular.

Mitigacao:

- mobile first: uma proxima acao, depois semana, depois mes.
- esconder graficos complexos.
- usar bottom nav e drawers curtos.

## Backlog sugerido por impacto

### V1 rapido

1. `AttentionStrip` no dashboard.
2. Lista de vencimentos por grupos temporais.
3. Badges padronizados de status.
4. Empty states nas telas de contas, cartoes e dashboard.
5. Snackbar com desfazer para "marcar como pago".

### V1.5

1. Simulador "posso comprar?".
2. Fatura simples com fechamento/vencimento.
3. Lembretes configuraveis por item.
4. Revisao semanal.

### V2

1. Resumo proativo do mes.
2. Sugestoes deterministicamente explicadas.
3. Notificacao externa via push/WhatsApp, com consentimento e preferencias.

## Direcao visual pratica

- Paleta neutra e funcional, com vermelho/ambar apenas para atraso/urgencia.
- Cards compactos, raio baixo, densidade media.
- Numeros grandes apenas nos 2 ou 3 indicadores principais.
- Icones Lucide: `CalendarDays`, `Bell`, `CreditCard`, `Receipt`, `AlertTriangle`, `CheckCircle2`, `Clock`, `Calculator`.
- Tailwind: usar variantes de status consistentes, por exemplo `data-status="overdue|due-soon|open|paid|simulated"`.
- Tabelas no desktop; listas/cards no mobile.

## Criterio de qualidade para qualquer implementacao

Uma tela nova do M Finance deve passar nestas perguntas:

1. A tela responde qual e a proxima acao financeira do usuario?
2. O usuario entende a diferenca entre pago, pendente, atrasado, estimado e simulado?
3. Existe uma acao direta para cada alerta?
4. A tela funciona sem OpenFinance e sem extrato?
5. O mobile mostra uma decisao por vez?
6. A interface evita prometer automacao que ainda nao existe?

