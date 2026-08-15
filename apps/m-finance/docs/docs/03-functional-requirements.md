# 03 — Functional Requirements

# M Finance — Requisitos Funcionais

## Objetivo do Documento

Este documento especifica as funcionalidades do M Finance.

Ele descreve o que cada módulo deve fazer, quais campos deve possuir, quais ações o usuário poderá executar e quais comportamentos são esperados.

---

# Visão Geral dos Módulos

O M Finance será composto pelos seguintes módulos:

1. Autenticação.
2. Dashboard.
3. Receitas.
4. Contas.
5. Categorias.
6. Cartões.
7. Faturas.
8. Calendário.
9. Alertas.
10. Histórico.
11. Configurações.
12. Simulador.
13. Metas.

---

# 1. Autenticação

## Objetivo

Permitir acesso seguro ao app usando Google.

## Requisitos

### RF-AUTH-01 — Login com Google

O usuário deve conseguir entrar no sistema usando conta Google.

### RF-AUTH-02 — Rotas protegidas

Todas as rotas internas do app devem exigir autenticação.

### RF-AUTH-03 — Usuário único

O sistema deve ser privado.

Mesmo que tecnicamente o auth permita múltiplos logins, a aplicação deve restringir o acesso ao e-mail autorizado.

### RF-AUTH-04 — Logout

O usuário deve conseguir sair da aplicação.

### RF-AUTH-05 — Redirecionamento

Após login bem-sucedido, o usuário deve ser redirecionado para `/dashboard`.

---

# 2. Dashboard

## Objetivo

Ser a tela principal do sistema.

O dashboard deve apresentar um cockpit financeiro do mês atual.

## Requisitos

### RF-DASH-01 — Exibir mês atual

Mostrar mês e ano em destaque.

Exemplo:

```txt
Junho 2026
```

### RF-DASH-02 — Trocar mês

Permitir navegar para mês anterior e próximo mês.

### RF-DASH-03 — Resumo financeiro

Exibir:

- receita prevista;
- total de contas;
- total de faturas;
- total pago;
- total pendente;
- total vencido;
- sobra estimada.

### RF-DASH-04 — Status do mês

Exibir status calculado:

- positivo;
- justo;
- apertado;
- negativo.

### RF-DASH-05 — Próximos vencimentos

Exibir lista das próximas contas e faturas.

### RF-DASH-06 — Contas vencidas

Exibir alerta quando houver conta ou fatura vencida.

### RF-DASH-07 — Ações rápidas

Disponibilizar botões para:

- adicionar conta;
- adicionar receita;
- adicionar fatura;
- marcar item como pago;
- ir para calendário.

### RF-DASH-08 — Cards principais

O dashboard deve usar cards para organizar informações importantes.

Cards mínimos:

- Receita prevista.
- Contas do mês.
- Faturas.
- Sobra estimada.
- Próximos vencimentos.
- Alertas.

---

# 3. Receitas

## Objetivo

Permitir cadastrar entradas previstas no mês.

## Tipos de Receita

- Principal.
- Extra.
- Freelance.

## Requisitos

### RF-INC-01 — Criar receita

Campos:

- nome;
- valor;
- tipo;
- data prevista;
- status recebido;
- mês de referência.

### RF-INC-02 — Editar receita

Permitir alterar dados da receita.

### RF-INC-03 — Excluir receita

Permitir excluir receita cadastrada.

### RF-INC-04 — Marcar como recebida

Permitir marcar receita como recebida.

### RF-INC-05 — Receita principal variável

A receita principal deve ser manual por mês.

O sistema pode sugerir valor anterior, mas não deve travar o valor.

### RF-INC-06 — Receitas extras

Freelancers e entradas eventuais devem poder ser adicionados sem alterar a receita principal.

---

# 4. Contas

## Objetivo

Controlar contas mensais a pagar.

## Tipos

- Recorrente.
- Única.

## Status

- Pendente.
- Pago.
- Vencido.

## Requisitos

### RF-BILL-01 — Criar conta

Campos:

- nome;
- valor;
- categoria;
- data de vencimento;
- recorrente;
- mês de referência;
- observação opcional.

### RF-BILL-02 — Marcar como paga

A ação mais importante do app.

Deve estar visível, rápida e simples.

Ao clicar, o status muda para pago.

### RF-BILL-03 — Editar conta

Permitir editar nome, valor, vencimento, categoria e recorrência.

### RF-BILL-04 — Excluir conta

Permitir excluir uma conta.

Para contas recorrentes, deve haver cuidado para escolher entre:

- excluir apenas este mês;
- excluir recorrência futura.

### RF-BILL-05 — Conta recorrente

Permitir marcar uma conta como recorrente mensal.

### RF-BILL-06 — Conta única

Permitir cadastrar conta que só existe em um mês.

### RF-BILL-07 — Valor sugerido

Para conta recorrente com valor variável, o sistema deve sugerir o valor do mês anterior.

### RF-BILL-08 — Filtro por status

Permitir filtrar:

- todas;
- pendentes;
- pagas;
- vencidas.

### RF-BILL-09 — Filtro por categoria

Permitir filtrar contas por categoria.

### RF-BILL-10 — Ordenação

Ordenar por vencimento como padrão.

---

# 5. Categorias

## Objetivo

Organizar contas por tipo.

## Categorias Iniciais

- Moradia.
- Transporte.
- Moto.
- Faculdade.
- Assinaturas.
- Saúde.
- Cartão.
- Dívidas.
- Lazer.
- Investimentos.
- Outros.

## Requisitos

### RF-CAT-01 — Listar categorias

Mostrar categorias cadastradas.

### RF-CAT-02 — Criar categoria

Permitir criar nova categoria.

### RF-CAT-03 — Editar categoria

Permitir editar nome, cor e ícone.

### RF-CAT-04 — Arquivar categoria

Preferir arquivar em vez de deletar quando houver contas associadas.

---

# 6. Cartões

## Objetivo

Cadastrar cartões para controle simples de faturas.

## Cartões Iniciais

- Mercado Pago.
- Itaú.
- Nubank Pessoal.
- Nubank PJ.

## Requisitos

### RF-CARD-01 — Criar cartão

Campos:

- nome;
- tipo: pessoal ou PJ;
- dia de vencimento;
- ativo.

### RF-CARD-02 — Editar cartão

Permitir editar informações do cartão.

### RF-CARD-03 — Inativar cartão

Permitir inativar sem apagar histórico.

### RF-CARD-04 — Identificar PJ

Cartões PJ devem ter indicação visual clara para evitar mistura com pessoal.

---

# 7. Faturas

## Objetivo

Controlar valor total mensal de cada cartão.

O app não deve registrar compra por compra.

## Requisitos

### RF-INV-01 — Criar fatura

Campos:

- cartão;
- mês de referência;
- valor;
- data de vencimento;
- status.

### RF-INV-02 — Editar fatura

Permitir alterar valor e vencimento.

### RF-INV-03 — Marcar como paga

Permitir marcar fatura como paga.

### RF-INV-04 — Excluir fatura

Permitir excluir fatura criada por engano.

### RF-INV-05 — Exibir no dashboard

Faturas devem aparecer no resumo do mês.

### RF-INV-06 — Exibir no calendário

Faturas devem aparecer na data de vencimento.

---

# 8. Calendário

## Objetivo

Mostrar visualmente os vencimentos do mês.

## Requisitos

### RF-CAL-01 — Visualização mensal

Exibir mês em formato de calendário.

### RF-CAL-02 — Exibir contas por data

Cada conta deve aparecer no dia correspondente ao vencimento.

### RF-CAL-03 — Exibir faturas por data

Cada fatura deve aparecer no dia de vencimento.

### RF-CAL-04 — Exibir receitas previstas

Receitas podem aparecer no calendário como entradas.

### RF-CAL-05 — Cores por status

Os itens devem indicar status:

- pendente;
- pago;
- vencido.

### RF-CAL-06 — Interação com item

Clicar em uma conta ou fatura deve abrir detalhe rápido.

### RF-CAL-07 — Marcar como pago pelo calendário

Sempre que possível, permitir marcar como pago direto no detalhe do item.

---

# 9. Alertas

## Objetivo

Avisar sobre vencimentos relevantes dentro do app.

## Requisitos

### RF-ALERT-01 — Conta vence em 3 dias

Gerar alerta preventivo.

### RF-ALERT-02 — Conta vence hoje

Gerar alerta urgente.

### RF-ALERT-03 — Conta vencida

Gerar alerta de atraso.

### RF-ALERT-04 — Fatura vence em 3 dias

Gerar alerta preventivo.

### RF-ALERT-05 — Fatura vence hoje

Gerar alerta urgente.

### RF-ALERT-06 — Fatura vencida

Gerar alerta de atraso.

### RF-ALERT-07 — Alertas internos

No MVP 1, alertas aparecem dentro da interface.

Não implementar push no MVP 1.

---

# 10. Histórico

## Objetivo

Guardar resumo simples de meses anteriores.

## Requisitos

### RF-HIST-01 — Criar histórico mensal

Ao fechar ou consultar mês anterior, o sistema deve preservar dados.

### RF-HIST-02 — Exibir resumo histórico

Mostrar:

- receita total;
- total de contas;
- total de faturas;
- total pago;
- total pendente;
- total vencido;
- sobra estimada;
- status final.

### RF-HIST-03 — Comparação simples

No MVP 1, comparação pode ser básica ou não existir.

---

# 11. Configurações

## Objetivo

Permitir ajustar dados de base do sistema.

## Requisitos

### RF-SET-01 — Perfil

Mostrar nome e e-mail do usuário autenticado.

### RF-SET-02 — Categorias

Gerenciar categorias.

### RF-SET-03 — Cartões

Gerenciar cartões.

### RF-SET-04 — Preferências de alerta

Permitir configurar alerta padrão.

No MVP 1, o padrão será:

- 3 dias antes;
- no dia;
- depois de vencido.

### RF-SET-05 — Tema visual

Preparar estrutura para tema visual.

A aplicação deve seguir o visual definido no documento UI/UX.

---

# 12. Simulador

## Status

MVP 2.

## Objetivo

Permitir simular impacto de uma compra futura.

## Requisitos

### RF-SIM-01 — Criar simulação

Campos:

- nome da compra;
- valor total;
- pagamento à vista ou parcelado;
- número de parcelas;
- mês de início;
- categoria.

### RF-SIM-02 — Calcular impacto mensal

Para compra parcelada, dividir valor pelo número de parcelas.

### RF-SIM-03 — Projetar meses afetados

Mostrar todos os meses impactados pela compra.

### RF-SIM-04 — Classificar risco

Classificar em:

- seguro;
- controlado;
- apertado;
- crítico.

### RF-SIM-05 — Recomendação textual

Gerar recomendação direta.

### RF-SIM-06 — Histórico de simulações

Guardar simulações feitas.

---

# 13. Metas

## Status

MVP 2.

## Objetivo

Acompanhar objetivos financeiros separados do cálculo obrigatório do mês.

## Requisitos

### RF-GOAL-01 — Criar meta

Campos:

- nome;
- valor alvo;
- valor atual;
- prazo opcional;
- prioridade.

### RF-GOAL-02 — Atualizar valor atual

Permitir atualizar progresso manualmente.

### RF-GOAL-03 — Exibir progresso

Mostrar percentual concluído.

### RF-GOAL-04 — Separar do cálculo mensal

Metas não entram automaticamente como obrigação mensal.

---

# Requisito Global

Toda funcionalidade deve respeitar a natureza do produto:

> planejamento financeiro mensal com baixa fricção.

Se uma funcionalidade exigir muitos dados para pouco benefício, deve ser simplificada ou adiada.
