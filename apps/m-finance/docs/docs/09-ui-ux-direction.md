# 09 — UI/UX Direction

# M Finance — Direção de UI e UX

## Objetivo do Documento

Este documento define a direção de interface e experiência do M Finance.

Ele não substitui o design system final que será enviado futuramente, mas define princípios, hierarquia, componentes e comportamento visual.

---

# Direção Visual Geral

O M Finance deve seguir uma mistura de:

```txt
financeiro limpo + Coded by M
```

Ou seja:

- clareza de app financeiro;
- aparência premium;
- dashboard técnico;
- dark mode como base provável;
- cards bem organizados;
- tipografia forte;
- hierarquia numérica clara;
- visual próprio, sem parecer template genérico.

---

# Princípio Principal

A interface deve ser um cockpit financeiro.

Um cockpit não mostra tudo com o mesmo peso.

Ele mostra primeiro o que importa para decisão.

---

# Sensação Desejada

O usuário deve sentir:

- controle;
- precisão;
- organização;
- clareza;
- calma;
- previsibilidade;
- domínio sobre o mês.

Não deve sentir:

- excesso;
- ansiedade;
- poluição;
- burocracia;
- confusão;
- aparência de planilha comum.

---

# Modo Visual

## Base

Dark mode como direção provável.

## Estilo

- fundo escuro;
- cards com contraste controlado;
- bordas sutis;
- brilho discreto;
- tipografia limpa;
- cores funcionais para status;
- poucos efeitos;
- microinterações objetivas.

## Observação

O design system final será enviado futuramente.

A arquitetura de UI deve permitir troca de tokens.

---

# Hierarquia do Dashboard

## Primeiro olhar

O usuário deve perceber imediatamente:

1. mês atual;
2. status do mês;
3. sobra estimada;
4. próximos vencimentos;
5. vencidos, se existirem.

## Segundo olhar

Depois deve entender:

- receita prevista;
- total de contas;
- total de faturas;
- total pago;
- total pendente.

## Terceiro olhar

Por fim:

- categorias;
- histórico;
- mini gráficos;
- atalhos secundários.

---

# Layout do Dashboard

## Desktop

Estrutura recomendada:

```txt
Sidebar fixa
Header do mês
Grid de cards principais
Coluna lateral com próximos vencimentos/alertas
Área inferior com calendário resumido e distribuição
```

## Mobile

Estrutura recomendada:

```txt
Topbar compacta
Cards empilhados
Ações rápidas visíveis
Lista de próximos vencimentos antes dos gráficos
Bottom nav
```

---

# Componentes Principais

## 1. Month Header

Mostra:

- mês atual;
- seletor de mês;
- botão de gerar novo mês se necessário;
- status do mês.

## 2. Status Card

Card grande com:

- sobra estimada;
- status;
- texto curto de interpretação.

Exemplo:

```txt
Sobra estimada: R$ 620
Status: Justo
Seu mês está positivo, mas sem muita margem.
```

## 3. Summary Cards

Cards menores:

- receita;
- contas;
- faturas;
- pago;
- pendente;
- vencido.

## 4. Upcoming Due List

Lista de próximos vencimentos.

Campos:

- nome;
- valor;
- data;
- status;
- ação pagar.

## 5. Alert Card

Mostra alertas relevantes.

Deve ter prioridade visual alta quando houver vencidos.

## 6. Bill Row

Linha de conta.

Campos:

- nome;
- categoria;
- vencimento;
- valor;
- status;
- botão pagar.

## 7. Invoice Card

Card de fatura.

Campos:

- cartão;
- tipo pessoal/PJ;
- valor;
- vencimento;
- status;
- ação pagar.

## 8. Calendar Event

Item visual dentro do calendário.

Deve indicar tipo:

- conta;
- fatura;
- receita.

---

# Estados Visuais

## Status de Conta/Fatura

### Pendente

Estado neutro.

### Pago

Estado resolvido.

### Vence em 3 dias

Estado preventivo.

### Vence hoje

Estado de atenção.

### Vencido

Estado crítico.

---

# Saúde do Mês

## Positive

Mês positivo.

Uso:

- sobra estimada positiva;
- sem vencidos.

## Fair

Mês justo.

Uso:

- sobra positiva, mas baixa;
- atenção moderada.

## Tight

Mês apertado.

Uso:

- sobra muito baixa;
- vários compromissos próximos.

## Negative

Mês negativo.

Uso:

- sobra negativa;
- vencidos relevantes.

---

# Formulários

## Princípios

- poucos campos;
- labels claros;
- máscara monetária;
- datas fáceis;
- salvar rápido;
- feedback imediato.

## Formulário de Conta

Campos:

- nome;
- valor;
- categoria;
- vencimento;
- recorrente;
- observação opcional.

## Formulário de Receita

Campos:

- nome;
- valor;
- tipo;
- data prevista;
- recebido.

## Formulário de Fatura

Campos:

- cartão;
- mês;
- valor;
- vencimento.

---

# Modais e Drawers

## Desktop

Modais centrais ou drawers laterais.

## Mobile

Bottom sheets.

## Uso

- criar conta;
- editar conta;
- criar receita;
- criar fatura;
- revisar novo mês;
- detalhe de calendário.

---

# Calendário

## Objetivo

Visualizar vencimentos.

## Regras

- não esconder informação demais;
- dias com vencimentos precisam ser claros;
- vencidos devem ter destaque;
- clicar no item deve abrir detalhe;
- marcar como pago deve ser possível rapidamente.

---

# Gráficos

## MVP 1

Usar poucos gráficos.

Possíveis:

- distribuição por categoria;
- contas pagas versus pendentes;
- faturas por cartão.

## Regra

Gráfico só deve existir se ajudar decisão.

Não usar gráfico apenas para enfeitar.

---

# Microinterações

Devem ser discretas e funcionais.

Exemplos:

- feedback ao marcar como pago;
- transição suave de status;
- hover em cards;
- foco em inputs;
- expansão de detalhe;
- alertas entrando de forma controlada.

Evitar:

- animações longas;
- efeitos que atrasam o uso;
- movimento excessivo;
- estética gamer;
- neon genérico.

---

# Empty States

## Sem contas

```txt
Nenhuma conta cadastrada para este mês.
Adicione sua primeira conta para começar o controle.
```

## Sem receitas

```txt
Nenhuma receita prevista cadastrada.
Adicione uma receita para calcular sua sobra estimada.
```

## Sem faturas

```txt
Nenhuma fatura cadastrada para este mês.
```

## Sem histórico

```txt
Seu histórico aparecerá depois que você usar o M Finance por mais de um mês.
```

---

# Linguagem da Interface

## Tom

- direto;
- objetivo;
- sem excesso emocional;
- claro;
- técnico quando necessário.

## Exemplos bons

```txt
Marcar como pago
Vence hoje
Vence em 3 dias
Mês justo
Sobra estimada
Gerar novo mês
Revisar recorrências
```

## Evitar

```txt
Parabéns, você está arrasando!
Seu dinheiro está voando!
Controle suas finanças como um mestre!
```

---

# Desktop vs Mobile

## Prioridade

Desktop primeiro, mas mobile precisa ser funcional.

## Uso mobile mais importante

- abrir dashboard;
- ver vencimentos;
- marcar conta como paga;
- ver calendário;
- adicionar conta simples.

## Uso desktop mais importante

- revisar mês;
- gerenciar contas;
- analisar dashboard;
- cadastrar faturas;
- usar simulador.

---

# Acessibilidade

Requisitos:

- contraste suficiente;
- foco visível;
- botões com labels claros;
- navegação por teclado razoável;
- textos legíveis;
- não depender apenas de cor para status.

---

# Regra Visual Final

O M Finance deve parecer um produto financeiro pessoal premium, construído sob medida, mas nunca deve sacrificar velocidade e clareza por estética.
