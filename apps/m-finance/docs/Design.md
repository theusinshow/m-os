# Design.md — M Finance

**Projeto:** `m-finance`  
**Produto:** M Finance  
**Base visual:** Design system da Coded by M  
**Versão do documento:** 1.0  
**Status:** Direção visual para implementação inicial e refinamento posterior.

---

## 1. Objetivo do documento

Este documento define a direção visual e de experiência do **M Finance**, adaptando a identidade da **Coded by M** para um software financeiro pessoal.

O M Finance deve parecer:

- pessoal;
- premium;
- técnico;
- limpo;
- preciso;
- confiável;
- organizado;
- fácil de usar;
- visualmente alinhado ao universo Coded by M.

O app não deve parecer:

- dashboard genérico de SaaS;
- template financeiro pronto;
- app bancário comum;
- planilha estilizada;
- painel corporativo pesado;
- interface colorida demais;
- sistema com excesso de gráficos;
- clone visual do Mobills.

---

## 2. Base visual da Coded by M

O brand hub da Coded by M define uma marca com:

- design minimalista;
- interfaces limpas;
- elementos com função clara;
- estrutura técnica;
- código limpo;
- performance;
- presença profissional;
- identidade visual sólida;
- confiança e diferenciação.

Para o M Finance, esses conceitos devem ser traduzidos em um sistema financeiro de alta clareza.

A adaptação principal é:

```txt
Coded by M = web design premium, estrutura e presença profissional.
M Finance = cockpit financeiro pessoal, estrutura e clareza mensal.
```

---

## 3. Princípio visual central

> **O M Finance deve transformar finanças mensais em uma estrutura visual clara.**

A interface deve fazer o usuário entender rapidamente:

- quanto entrou;
- quanto precisa sair;
- o que já foi pago;
- o que ainda está pendente;
- o que vence em breve;
- quanto sobra;
- se o mês está saudável ou apertado.

A experiência visual não deve competir com os dados. Deve organizar os dados.

---

## 4. Personalidade visual

### Palavras-chave

```txt
Dark
Premium
Técnico
Minimalista
Preciso
Calmo
Estruturado
Financeiro
Pessoal
Direto
```

### Sensação desejada

Ao abrir o app, Matheus deve sentir:

```txt
Estou no controle do meu mês.
```

Não deve sentir:

```txt
Estou preenchendo mais uma ferramenta financeira chata.
```

---

## 5. Paleta de cores base

A paleta principal vem da Coded by M.

### Cores oficiais

```txt
Preto principal: #000F08
Vermelho destaque: #FB3640
Branco quente: #F5F2ED
Verde profundo: #0E1810
Cinza 100: #E8E4DE
Cinza 400: #8A8780
```

---

## 6. Uso das cores no M Finance

### Preto principal — `#000F08`

Uso:

- fundo principal do app;
- sidebar;
- área externa do dashboard;
- base visual do cockpit.

Regra:

```txt
Usar como fundo dominante.
```

Não usar preto puro genérico se o design system já define `#000F08` como preto principal.

---

### Verde profundo — `#0E1810`

Uso:

- cards principais;
- superfícies secundárias;
- blocos do dashboard;
- containers;
- painéis de calendário;
- blocos de formulário.

Regra:

```txt
Usar como superfície elevada sobre o fundo escuro.
```

---

### Branco quente — `#F5F2ED`

Uso:

- textos principais;
- números importantes;
- títulos;
- labels de alto contraste;
- ícones principais.

Regra:

```txt
Usar para informação principal.
```

---

### Cinza 100 — `#E8E4DE`

Uso:

- textos de apoio em fundos escuros;
- linhas claras sutis;
- divisores leves;
- detalhes de formulário.

---

### Cinza 400 — `#8A8780`

Uso:

- legendas;
- descrições;
- labels secundários;
- datas;
- placeholders;
- metadados;
- textos de apoio.

---

### Vermelho destaque — `#FB3640`

Uso:

- CTAs principais;
- ponto de atenção;
- alertas críticos;
- vencidos;
- detalhes ativos;
- foco visual em ações importantes.

Regra importante:

```txt
Não usar vermelho em excesso.
```

Como vermelho também é uma cor de perigo em interfaces financeiras, ele deve ser reservado para:

- alertas reais;
- conta vencida;
- fatura vencida;
- ação primária pontual;
- destaque de navegação ativo.

Não transformar o app em uma interface vermelha.

---

## 7. Tokens de cor recomendados

### Backgrounds

```txt
--color-bg-primary: #000F08;
--color-bg-secondary: #0E1810;
--color-bg-card: #0E1810;
--color-bg-card-hover: #132116;
--color-bg-elevated: #101C12;
```

### Textos

```txt
--color-text-primary: #F5F2ED;
--color-text-secondary: #E8E4DE;
--color-text-muted: #8A8780;
--color-text-inverse: #000F08;
```

### Bordas

```txt
--color-border-subtle: rgba(245, 242, 237, 0.08);
--color-border-default: rgba(245, 242, 237, 0.14);
--color-border-strong: rgba(245, 242, 237, 0.24);
--color-border-accent: #FB3640;
```

### Acentos

```txt
--color-accent: #FB3640;
--color-accent-hover: #ff4b54;
--color-accent-soft: rgba(251, 54, 64, 0.12);
--color-accent-border: rgba(251, 54, 64, 0.35);
```

---

## 8. Cores semânticas financeiras

A paleta da marca não possui cores semânticas completas para finanças. Portanto, o M Finance pode introduzir cores funcionais discretas.

### Recomendações

```txt
Positivo: verde funcional discreto
Justo: amarelo/âmbar discreto
Apertado: laranja discreto
Negativo: vermelho da marca
```

### Regra

As cores semânticas devem aparecer principalmente em:

- badges;
- pequenos indicadores;
- gráficos;
- bordas sutis;
- textos de status.

Não devem dominar a interface.

### Tokens sugeridos

```txt
--status-positive: #54D18A;
--status-positive-soft: rgba(84, 209, 138, 0.12);

--status-fair: #D8B45A;
--status-fair-soft: rgba(216, 180, 90, 0.12);

--status-tight: #D98245;
--status-tight-soft: rgba(217, 130, 69, 0.12);

--status-negative: #FB3640;
--status-negative-soft: rgba(251, 54, 64, 0.12);
```

Essas cores são complementares ao design system. Não substituem a paleta principal.

---

## 9. Tipografia

A base do brand hub define:

```txt
Display: Panchang
Body: Satoshi
```

### Uso no M Finance

#### Panchang

Uso:

- logo textual;
- títulos curtos;
- números hero quando fizer sentido;
- headers de página;
- labels de impacto;
- momentos de identidade.

Evitar usar Panchang em textos longos, tabelas ou muitos cards, porque pode reduzir legibilidade em interface financeira.

#### Satoshi

Uso:

- corpo do app;
- navegação;
- formulários;
- labels;
- tabelas;
- dados financeiros;
- tooltips;
- botões;
- descrições.

### Regra tipográfica

```txt
Panchang dá identidade.
Satoshi garante usabilidade.
```

---

## 10. Escala tipográfica sugerida

### Display

```txt
Display XL: 48px / 56px / Panchang / 600
Display LG: 40px / 48px / Panchang / 600
Display MD: 32px / 40px / Panchang / 600
```

### Headings

```txt
H1: 32px / 40px / Panchang ou Satoshi / 600
H2: 24px / 32px / Satoshi / 600
H3: 20px / 28px / Satoshi / 600
H4: 18px / 26px / Satoshi / 600
```

### Body

```txt
Body LG: 18px / 28px / Satoshi / 400
Body MD: 16px / 24px / Satoshi / 400
Body SM: 14px / 20px / Satoshi / 400
Body XS: 12px / 16px / Satoshi / 400
```

### Interface

```txt
Label: 12px / 16px / Satoshi / 500 / uppercase opcional
Button: 14px / 20px / Satoshi / 600
Caption: 12px / 16px / Satoshi / 400
Metric: 32px-56px / Satoshi ou Panchang / 600
```

---

## 11. Números financeiros

Números são o conteúdo mais importante do app.

### Regras

- Números principais devem ter alto contraste.
- Usar alinhamento consistente.
- Evitar excesso de casas decimais em cards principais.
- Em listas, manter valores alinhados à direita.
- Em cards grandes, valores podem ficar alinhados à esquerda.
- Usar `R$` de forma consistente.

### Exemplo de hierarquia

```txt
R$ 4.500,00       Receita prevista
R$ 3.200,00       Total comprometido
R$ 1.300,00       Sobra prevista
```

### Formatação

```txt
R$ 0,00
R$ 120,00
R$ 1.450,00
R$ 12.500,00
```

---

## 12. Layout geral

### Prioridade

O app é desktop-first, mas deve funcionar bem em mobile.

### Estrutura desktop recomendada

```txt
Sidebar fixa à esquerda
Header superior discreto
Área principal com dashboard em grid
Painel lateral opcional para alertas ou próximos vencimentos
```

### Estrutura mobile recomendada

```txt
Header compacto
Navegação inferior ou menu lateral
Cards empilhados
Ações rápidas sempre acessíveis
```

---

## 13. Grid do dashboard

### Desktop

Grid recomendado:

```txt
12 colunas
Gap: 16px ou 24px
Padding externo: 24px a 32px
Max-width: não limitar demais em telas grandes
```

### Mobile

```txt
1 coluna
Gap: 12px a 16px
Padding externo: 16px
```

### Tablet

```txt
2 a 6 colunas dependendo do bloco
```

---

## 14. Dashboard como cockpit

O usuário escolheu o dashboard como estilo **cockpit financeiro**.

Isso significa que o dashboard deve reunir múltiplas informações críticas, mas com hierarquia clara.

### Blocos principais

```txt
Status do mês
Sobra prevista
Receita prevista
Total a pagar
Total pago
Total pendente
Próximos vencimentos
Faturas
Calendário resumido
Alertas
Ações rápidas
```

### Regra

Cockpit não é poluição.

A tela deve parecer avançada, mas não confusa.

---

## 15. Hierarquia do dashboard

### Nível 1 — Decisão imediata

```txt
Status do mês
Sobra prevista
Contas vencendo
Contas vencidas
```

### Nível 2 — Resumo financeiro

```txt
Receita total
Total comprometido
Total pago
Total pendente
Faturas
```

### Nível 3 — Navegação e detalhe

```txt
Calendário compacto
Lista de próximas contas
Metas
Histórico simples
```

---

## 16. Componentes principais

### 16.1. AppShell

Estrutura global autenticada.

Inclui:

- sidebar;
- header;
- conteúdo;
- área de alertas;
- responsividade.

---

### 16.2. Sidebar

Itens:

```txt
Dashboard
Calendário
Contas
Cartões
Metas
Simulador
Configurações
```

### Estado ativo

- texto em branco quente;
- detalhe vermelho sutil;
- fundo levemente elevado;
- ícone com contraste maior.

---

### 16.3. Header

Deve conter:

- mês atual;
- seletor de mês;
- botão de gerar mês quando aplicável;
- ação rápida principal;
- avatar/conta Google.

Não deve ser grande demais.

---

### 16.4. MetricCard

Card para número principal.

Campos:

```txt
Título
Valor
Descrição
Variação opcional
Status opcional
Ícone opcional
```

Exemplo:

```txt
Sobra prevista
R$ 820,00
Depois de contas e faturas do mês
Status: Justo
```

---

### 16.5. BillCard

Card de conta.

Campos:

```txt
Nome da conta
Categoria
Valor
Vencimento
Status
Ação: Marcar como pago
```

Estados:

```txt
Pendente
Pago
Vence hoje
Vencida
```

---

### 16.6. InvoiceCard

Card de fatura.

Campos:

```txt
Cartão
Tipo: pessoal/PJ
Valor da fatura
Vencimento
Status
Ação: Marcar como paga
```

---

### 16.7. AlertCard

Card de alerta.

Tipos:

```txt
Preventivo
Hoje
Vencido
Sistema
```

Exemplo:

```txt
Fatura Nubank vence hoje.
```

---

### 16.8. CalendarDay

Dia do calendário com eventos financeiros.

Deve indicar:

- quantidade de eventos;
- status crítico;
- contas pagas;
- contas pendentes;
- faturas.

---

### 16.9. QuickActionButton

Botão de ação rápida.

Ações:

```txt
Adicionar conta
Adicionar receita
Adicionar fatura
Marcar como pago
Simular compra
```

---

### 16.10. StatusBadge

Badge para status.

Estados:

```txt
Positivo
Justo
Apertado
Negativo
Pendente
Pago
Vencido
```

---

## 17. Botões

### Primário

Uso:

- ação principal da tela;
- gerar mês;
- salvar;
- confirmar;
- simular compra.

Visual:

```txt
Fundo vermelho #FB3640
Texto #F5F2ED ou #000F08 dependendo do contraste final
Borda sutil
Altura mínima 40px
Raio discreto
```

### Secundário

Uso:

- cancelar;
- editar;
- revisar;
- ver detalhes.

Visual:

```txt
Fundo transparente ou verde profundo
Borda sutil clara
Texto branco quente
```

### Ghost

Uso:

- ações pequenas;
- ícones;
- menus;
- detalhes.

Visual:

```txt
Sem fundo
Hover com superfície sutil
```

### Botão “Marcar como pago”

Deve ser extremamente claro e acessível.

Recomendação:

```txt
Usar botão secundário com estado positivo após ação.
```

Evitar esconder essa ação em menus.

---

## 18. Cards

### Estilo base

```txt
Fundo: #0E1810
Borda: rgba(245, 242, 237, 0.08)
Raio: 16px a 24px
Padding: 16px a 24px
Sombra: muito sutil ou nenhuma
```

### Regra

Cards devem parecer superfícies técnicas, não caixas genéricas.

### Hover

```txt
Leve elevação
Borda um pouco mais visível
Fundo levemente mais claro
Transição suave
```

---

## 19. Formulários

### Campos principais

```txt
Input de texto
Input de valor monetário
Date picker
Select
Switch de recorrência
Radio de tipo
Textarea opcional
```

### Visual

```txt
Fundo escuro elevado
Borda sutil
Label clara
Placeholder cinza
Focus com vermelho discreto
Erro com vermelho
```

### Regras

- Formulários devem ser curtos.
- Não exigir dados que não serão usados no MVP.
- Valores financeiros devem ter máscara monetária.
- Datas devem aceitar seleção rápida.
- Recorrência deve ser clara.

---

## 20. Modais e drawers

Para o M Finance, drawers laterais podem funcionar melhor que modais grandes no desktop.

### Usos recomendados

```txt
Adicionar conta
Editar conta
Adicionar receita
Adicionar fatura
Revisar novo mês
Detalhes de um dia do calendário
```

### Desktop

Usar drawer lateral direito.

### Mobile

Usar bottom sheet.

---

## 21. Calendário visual

O calendário deve ser funcional, não decorativo.

### Dia com evento

Indicar com:

- pequeno marcador;
- linha colorida;
- contador;
- tooltip ou painel lateral.

### Dia vencido

Usar destaque vermelho controlado.

### Dia atual

Usar borda clara ou acento sutil.

### Dia selecionado

Usar fundo elevado e borda mais forte.

---

## 22. Gráficos

Gráficos devem ser usados com moderação.

### Gráficos úteis

```txt
Distribuição de contas por categoria
Resumo pago x pendente
Evolução de sobra mensal
Faturas por cartão
```

### Gráficos que devem ser evitados no MVP 1

```txt
Análises complexas
Comparações excessivas
Projeções visuais avançadas
Gráficos com muitas cores
```

### Estilo

- linhas finas;
- cores discretas;
- fundo transparente;
- labels legíveis;
- tooltips em dark mode;
- nada de visual chamativo sem função.

---

## 23. Estados visuais financeiros

### Conta pendente

```txt
Badge neutro
Borda discreta
Ação de pagar visível
```

### Conta paga

```txt
Menor contraste
Badge positivo
Ação principal removida ou substituída por “Pago”
```

### Conta vence hoje

```txt
Alerta destacado
Borda de atenção
Prioridade no dashboard
```

### Conta vencida

```txt
Vermelho da marca
Badge “Vencida”
Alta prioridade
```

---

## 24. Motion e microinterações

O M Finance pode herdar o cuidado de movimento da Coded by M, mas com contenção.

### Princípio

```txt
Movimento deve confirmar, revelar ou orientar.
```

### Bons usos

- card entrando suavemente;
- botão confirmando pagamento;
- badge mudando de status;
- drawer abrindo;
- alerta aparecendo;
- calendário mudando de mês;
- gráfico carregando discretamente.

### Evitar

- animação cinematográfica pesada;
- loading longo;
- transições lentas demais;
- efeitos 3D desnecessários;
- excesso de glow;
- elementos se mexendo sem função.

---

## 25. Loading

Como o app será usado com frequência, o loading deve ser rápido.

### Recomendação

- usar skeletons;
- evitar tela de loading longa;
- usar pequenos indicadores triangulares se quiser reforçar identidade;
- priorizar tempo de resposta.

### Regra

```txt
No portfólio, o loading pode ser narrativo.
No M Finance, o loading deve ser funcional.
```

---

## 26. Elementos triangulares

A identidade da Coded by M usa triângulos e estruturas técnicas.

No M Finance, essa linguagem pode aparecer de forma sutil.

### Usos permitidos

- marcador de item ativo;
- ícone abstrato no header;
- divisores geométricos discretos;
- padrão quase invisível no fundo;
- loading pequeno;
- estados de hover;
- detalhes em cards importantes.

### Evitar

- fundos muito complexos;
- malha competindo com números;
- triângulos demais no dashboard;
- visual experimental acima da legibilidade.

---

## 27. Ícones

Ícones devem ser simples e lineares.

### Estilo

```txt
Stroke fino/médio
Sem preenchimentos pesados
Geometria simples
Consistência de tamanho
```

### Biblioteca sugerida

```txt
Lucide React
```

### Tamanhos

```txt
16px — labels e ações pequenas
20px — navegação
24px — cards principais
```

---

## 28. Espaçamento

### Escala recomendada

```txt
4px
8px
12px
16px
24px
32px
48px
64px
```

### Regras

- usar bastante respiro;
- evitar colar números e labels;
- cards financeiros precisam de leitura rápida;
- listas devem ter densidade moderada;
- dashboard pode ser denso, mas não comprimido.

---

## 29. Bordas e raios

### Radius

```txt
Small: 8px
Medium: 12px
Large: 16px
XL: 24px
```

### Regra

Usar radius médio/grande nos cards para aparência premium, mas sem parecer app infantil.

---

## 30. Densidade visual

O M Finance deve ter densidade média-alta no desktop e média no mobile.

### Desktop

Pode mostrar muitos dados ao mesmo tempo, desde que agrupados.

### Mobile

Deve priorizar:

1. status do mês;
2. próximas contas;
3. ação de marcar como pago;
4. alertas;
5. faturas.

---

## 31. Empty states

Estados vazios devem orientar ação.

### Exemplos

```txt
Nenhuma conta cadastrada para este mês.
Adicione suas contas principais para montar o cockpit financeiro.

Nenhuma fatura cadastrada.
Cadastre o valor da fatura do mês sem precisar lançar compra por compra.

Nenhum alerta ativo.
Não há contas vencendo nos próximos dias.
```

Evitar mensagens exageradamente descontraídas.

---

## 32. Copy de interface

### Tom

```txt
Direto
Calmo
Técnico
Útil
Sem exagero
```

### Exemplos aprovados

```txt
3 contas vencem nos próximos dias.
Fatura Nubank vence hoje.
Julho está justo.
Você ainda tem R$ 820 previstos para o mês.
Nenhuma conta vencida.
```

### Exemplos rejeitados

```txt
Uhuu, você está dominando seu dinheiro!
Sua jornada financeira está incrível!
Vamos turbinar suas finanças!
```

---

## 33. Layout das principais telas

## 33.1. Dashboard

### Desktop

Estrutura recomendada:

```txt
Header: mês atual + ações rápidas
Linha 1: Status do mês + Sobra prevista + Receita + Total comprometido
Linha 2: Próximos vencimentos + Faturas + Alertas
Linha 3: Calendário compacto + Contas pendentes + Histórico simples
```

### Mobile

Ordem recomendada:

```txt
Status do mês
Sobra prevista
Alertas
Próximos vencimentos
Ação rápida: adicionar conta
Faturas
Resumo financeiro
Calendário compacto
```

---

## 33.2. Contas

### Desktop

```txt
Header com filtros e botão adicionar
Tabela/lista de contas
Painel lateral de detalhes
Filtros por status, categoria e vencimento
```

### Mobile

```txt
Lista em cards
Filtros compactos
Botão fixo de adicionar
Ação de marcar como pago visível em cada card
```

---

## 33.3. Calendário

### Desktop

```txt
Calendário mensal grande
Painel lateral com eventos do dia selecionado
Filtros por tipo
```

### Mobile

```txt
Calendário compacto
Lista abaixo por data
```

---

## 33.4. Cartões

### Desktop

```txt
Cards por cartão
Fatura atual
Status
Vencimento
Ação de marcar como paga
Histórico simples abaixo
```

### Mobile

```txt
Cards empilhados
Valor da fatura em destaque
Status claro
```

---

## 33.5. Simulador

### Desktop

```txt
Formulário à esquerda
Resultado à direita
Impacto por mês abaixo
```

### Mobile

```txt
Formulário primeiro
Resultado em card destacado
Impacto mensal em lista
```

---

## 34. Acessibilidade

Mesmo sendo app pessoal, a interface deve seguir boas práticas.

### Regras

- contraste adequado;
- foco visível;
- botões com área clicável confortável;
- labels claros;
- textos não dependerem apenas de cor;
- status com texto + cor;
- navegação por teclado básica;
- inputs com mensagens de erro.

---

## 35. Implementação Tailwind — tokens sugeridos

### Exemplo de tema

```ts
const colors = {
  background: {
    primary: '#000F08',
    secondary: '#0E1810',
    elevated: '#101C12',
  },
  text: {
    primary: '#F5F2ED',
    secondary: '#E8E4DE',
    muted: '#8A8780',
  },
  accent: {
    DEFAULT: '#FB3640',
    hover: '#ff4b54',
    soft: 'rgba(251, 54, 64, 0.12)',
  },
  status: {
    positive: '#54D18A',
    fair: '#D8B45A',
    tight: '#D98245',
    negative: '#FB3640',
  },
}
```

---

## 36. Component naming

Sugestão de nomes de componentes:

```txt
AppShell
SidebarNav
TopBar
MonthSelector
MetricCard
StatusBadge
BillCard
InvoiceCard
AlertPanel
FinancialCalendar
CalendarEventDot
QuickActionButton
CreateBillDrawer
CreateIncomeDrawer
CreateInvoiceDrawer
MonthGenerationReview
SimulationResultCard
GoalProgressCard
```

---

## 37. Design de dados

A interface deve tratar os dados financeiros como blocos estruturados.

### Exemplo de card correto

```txt
Internet
Vence dia 10
R$ 120,00
Status: Pendente
[Marcar como pago]
```

### Exemplo ruim

```txt
Internet - R$120 - venc 10 - pendente - recorrente - categoria moradia - mês junho - id 123
```

A interface deve esconder complexidade técnica e mostrar decisão.

---

## 38. Regras para agentes de IA

Ao implementar o design:

1. Não usar tema financeiro genérico.
2. Não usar azul bancário padrão.
3. Não criar dashboard poluído.
4. Não transformar vermelho em cor dominante.
5. Não esconder a ação “Marcar como pago”.
6. Não usar animações sem função.
7. Não criar gráficos antes dos dados essenciais.
8. Não criar layout mobile quebrado.
9. Não substituir clareza por estética.
10. Não inventar novo design system sem necessidade.

---

## 39. Prioridade visual por MVP

### MVP 1

Priorizar:

```txt
layout funcional
cards claros
dashboard limpo
contas fáceis de pagar
calendário legível
login simples
responsividade básica
```

### MVP 2

Adicionar:

```txt
simulador visual
impacto por mês
metas com progresso
recomendações textuais
```

### MVP 3

Refinar:

```txt
microinterações
notificações
PWA
histórico visual
exportação
acabamento premium
```

---

## 40. Regra final de design

> **O M Finance deve parecer um produto pessoal premium, mas sua principal qualidade precisa ser clareza financeira imediata.**

Quando houver conflito:

```txt
Clareza vence estética.
Rapidez vence animação.
Hierarquia vence quantidade.
Controle vence decoração.
```
