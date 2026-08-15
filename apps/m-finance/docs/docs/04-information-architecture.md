# 04 — Information Architecture

# M Finance — Arquitetura de Informação

## Objetivo do Documento

Este documento define a organização das telas, rotas, navegação e hierarquia de informação do M Finance.

Ele não define layout final, mas estabelece a estrutura que o produto deve seguir.

---

# Princípio Principal

O app deve ser organizado ao redor do mês financeiro.

A pergunta central é:

> Como está meu mês agora?

Todas as telas devem se conectar a essa lógica.

---

# Estrutura de Rotas

```txt
/
/login
/app/dashboard
/app/calendar
/app/bills
/app/cards
/app/goals
/app/simulator
/app/history
/app/settings
```

---

# Rotas Públicas

## `/`

Pode redirecionar para:

- `/login` se usuário não estiver autenticado;
- `/app/dashboard` se usuário estiver autenticado.

## `/login`

Tela de login com Google.

Conteúdo mínimo:

- nome do produto;
- descrição curta;
- botão de entrar com Google;
- aviso de app privado.

---

# Rotas Privadas

Todas as rotas abaixo exigem autenticação.

---

# `/app/dashboard`

## Função

Tela principal do app.

## Pergunta que responde

> Como está meu mês financeiro agora?

## Conteúdo

1. Header do mês.
2. Resumo financeiro.
3. Status do mês.
4. Próximos vencimentos.
5. Contas vencidas.
6. Faturas.
7. Sobra estimada.
8. Ações rápidas.
9. Alertas.
10. Pequeno resumo de calendário.

## Hierarquia

### Nível 1

- Mês atual.
- Sobra estimada.
- Total pendente.
- Contas vencidas.

### Nível 2

- Receitas.
- Faturas.
- Próximos vencimentos.
- Status do mês.

### Nível 3

- Atalhos.
- Gráficos simples.
- Histórico resumido.

---

# `/app/calendar`

## Função

Visualizar vencimentos em formato de calendário.

## Pergunta que responde

> O que vence e quando?

## Conteúdo

1. Calendário mensal.
2. Contas por dia.
3. Faturas por dia.
4. Receitas previstas.
5. Indicadores de status.
6. Detalhe rápido de item.

## Interações

- Clicar em dia.
- Clicar em conta.
- Clicar em fatura.
- Marcar item como pago.
- Ir para edição do item.

---

# `/app/bills`

## Função

Gerenciar contas do mês.

## Pergunta que responde

> Quais contas tenho que pagar?

## Conteúdo

1. Lista de contas.
2. Filtros.
3. Busca.
4. Botão de adicionar conta.
5. Status de pagamento.
6. Ação de marcar como paga.

## Filtros

- Todas.
- Pendentes.
- Pagas.
- Vencidas.
- Recorrentes.
- Únicas.
- Categoria.

## Ordenação padrão

Por data de vencimento crescente.

---

# `/app/cards`

## Função

Gerenciar cartões e faturas simples.

## Pergunta que responde

> Quanto minhas faturas vão pesar neste mês?

## Conteúdo

1. Lista de cartões.
2. Valor da fatura do mês por cartão.
3. Status da fatura.
4. Vencimento.
5. Identificação pessoal/PJ.
6. Ação de marcar fatura como paga.

## Cartões iniciais

- Mercado Pago.
- Itaú.
- Nubank Pessoal.
- Nubank PJ.

---

# `/app/goals`

## Função

Acompanhar metas financeiras.

## Status

MVP 2.

## Pergunta que responde

> Estou avançando nas minhas metas?

## Conteúdo

1. Lista de metas.
2. Progresso visual.
3. Valor alvo.
4. Valor atual.
5. Quanto falta.
6. Prazo.
7. Prioridade.

## Observação

Metas não devem entrar automaticamente no cálculo de obrigações do mês.

---

# `/app/simulator`

## Função

Simular impacto de compras futuras.

## Status

MVP 2.

## Pergunta que responde

> Posso comprar isso agora?

## Conteúdo

1. Formulário de simulação.
2. Resultado direto.
3. Risco.
4. Impacto mensal.
5. Recomendação textual.
6. Histórico de simulações.

## Resultado esperado

A tela deve ser orientada a decisão, não apenas cálculo.

---

# `/app/history`

## Função

Visualizar histórico simples de meses anteriores.

## Pergunta que responde

> Como foram meus meses anteriores?

## Conteúdo

1. Lista de meses.
2. Receita total.
3. Total de contas.
4. Total de faturas.
5. Total pago.
6. Total vencido.
7. Sobra estimada.
8. Status final.

## MVP 1

Histórico simples.

## MVP 3

Histórico com gráficos e comparações.

---

# `/app/settings`

## Função

Gerenciar configurações.

## Conteúdo

1. Perfil.
2. Categorias.
3. Cartões.
4. Preferências de alerta.
5. Tema visual.
6. Dados do app.

---

# Navegação Principal

## Desktop

Sidebar ou topbar fixa.

Itens:

- Dashboard.
- Calendário.
- Contas.
- Cartões.
- Metas.
- Simulador.
- Histórico.
- Configurações.

## Mobile

Bottom navigation ou menu compacto.

Itens principais:

- Dashboard.
- Calendário.
- Contas.
- Cartões.
- Mais.

Dentro de “Mais”:

- Metas.
- Simulador.
- Histórico.
- Configurações.

---

# Estrutura de Layout

## Layout Global

- Sidebar/topbar.
- Conteúdo principal.
- Header da página.
- Área de ações.
- Cards.
- Modais ou drawers.

## Modais/Drawers

Usar para:

- adicionar conta;
- editar conta;
- adicionar receita;
- adicionar fatura;
- marcar detalhes;
- gerar novo mês.

---

# Componentes de Informação

## Cards de Resumo

Usados no dashboard.

Exemplos:

- Receita prevista.
- Total a pagar.
- Pago.
- Pendente.
- Vencido.
- Sobra.

## Listas Operacionais

Usadas em contas e faturas.

Devem priorizar:

- nome;
- valor;
- vencimento;
- status;
- ação principal.

## Calendário

Deve ser funcional e legível.

Não deve esconder informação importante atrás de interações difíceis.

## Badges de Status

Estados:

- Pago.
- Pendente.
- Vencido.
- Vence hoje.
- Vence em 3 dias.

---

# Hierarquia de Informação do Dashboard

## Acima da dobra

- Mês atual.
- Sobra estimada.
- Status do mês.
- Total pendente.
- Próximos vencimentos.

## Segunda camada

- Contas pagas.
- Faturas.
- Receitas.
- Alertas.

## Terceira camada

- Mini calendário.
- Histórico resumido.
- Distribuição por categoria.

---

# Padrões de Ação

## Ação primária global

Adicionar conta.

## Ação mais frequente

Marcar como pago.

## Ações secundárias

- Adicionar receita.
- Adicionar fatura.
- Editar item.
- Simular compra.

---

# Regra Final

A arquitetura deve permitir que o usuário entenda o mês atual sem precisar navegar por muitas telas.

O Dashboard deve ser suficiente para uma leitura rápida.

As telas internas servem para detalhe, edição e organização.
