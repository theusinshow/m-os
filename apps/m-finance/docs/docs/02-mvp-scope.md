# 02 — MVP Scope

# M Finance — Escopo do MVP

## Objetivo do Documento

Este documento define o que entra e o que não entra em cada fase do M Finance.

Ele serve para proteger o projeto contra aumento excessivo de escopo.

---

# Estratégia de Evolução

O M Finance será dividido em três grandes fases:

1. **MVP 1 — Controle Mensal Real**
2. **MVP 2 — Decisão de Compra**
3. **MVP 3 — Experiência Premium**

A primeira versão deve ser funcional, útil e suficiente para substituir o uso atual do Mobills.

---

# MVP 1 — Controle Mensal Real

## Objetivo

Permitir que Matheus controle o mês atual com clareza.

O MVP 1 deve resolver:

- contas mensais;
- vencimentos;
- faturas simples;
- receitas previstas;
- marcação de pagamento;
- dashboard;
- calendário;
- alertas internos;
- geração mensal com revisão;
- histórico simples.

## Resultado Esperado

Matheus deve conseguir abandonar o Mobills para o uso principal:

> lembrar tudo que precisa pagar no mês e marcar o que já foi pago.

---

## Funcionalidades Incluídas no MVP 1

### 1. Autenticação

- Login com Google.
- App privado.
- Usuário único permitido.
- Redirecionamento para dashboard após login.
- Proteção das rotas internas.

### 2. Dashboard Mensal

- Resumo do mês atual.
- Receita total prevista.
- Total de contas.
- Total pago.
- Total pendente.
- Total vencido.
- Total de faturas.
- Sobra estimada.
- Status do mês.
- Próximos vencimentos.
- Alertas importantes.
- Ações rápidas.

### 3. Receitas

- Cadastro de receita principal do mês.
- Cadastro de receitas extras.
- Cadastro de freelancers.
- Valor manual por mês.
- Campo de data prevista.
- Status recebido ou previsto.

### 4. Contas

- Cadastro de conta.
- Nome.
- Valor.
- Categoria.
- Data de vencimento.
- Recorrente ou única.
- Status: pendente, pago ou vencido.
- Marcar como pago.
- Editar conta.
- Excluir conta.

### 5. Contas Recorrentes

- Definir conta como recorrente.
- Repetição mensal.
- Geração mediante revisão no início do mês.
- Valor sugerido com base no mês anterior.
- Edição de ocorrência mensal.

### 6. Calendário Financeiro

- Visualização mensal.
- Contas nos respectivos dias.
- Faturas nos respectivos dias.
- Receita prevista nos respectivos dias.
- Indicador de status por item.
- Destaque de vencimentos próximos.

### 7. Cartões Simples

- Cadastro de cartão.
- Nome.
- Tipo: pessoal ou PJ.
- Dia de vencimento.
- Status ativo ou inativo.

### 8. Faturas Simples

- Cadastro de valor da fatura mensal.
- Vincular fatura a cartão.
- Data de vencimento.
- Status: pendente, pago ou vencido.
- Marcar fatura como paga.

### 9. Alertas Internos

- Conta vence em 3 dias.
- Conta vence hoje.
- Conta vencida.
- Fatura vence em 3 dias.
- Fatura vence hoje.
- Fatura vencida.

### 10. Geração Mensal com Revisão

- Detectar novo mês.
- Sugerir criação do mês.
- Listar contas recorrentes.
- Sugerir valores com base no mês anterior.
- Permitir revisar antes de gerar.
- Criar mês confirmado.

### 11. Histórico Simples

- Guardar resumo final de cada mês.
- Receita total.
- Total de contas.
- Total de faturas.
- Total pago.
- Total pendente.
- Total vencido.
- Sobra estimada.
- Status final.

### 12. Configurações Básicas

- Perfil do usuário.
- Categorias.
- Cartões.
- Preferências básicas.
- Tema visual se viável.

---

# Funcionalidades Fora do MVP 1

Não implementar no MVP 1:

- registro de cada compra;
- extrato bancário;
- integração com bancos;
- Open Finance;
- importação CSV;
- exportação CSV/PDF;
- comprovantes;
- anexos;
- notificações push;
- PWA completo;
- controle detalhado de investimentos;
- relatórios avançados;
- gráficos complexos;
- múltiplos usuários;
- compartilhamento familiar;
- app mobile nativo.

---

# MVP 2 — Decisão de Compra

## Objetivo

Adicionar inteligência de decisão.

O MVP 2 deve responder:

> Posso comprar isso agora?

---

## Funcionalidades Incluídas no MVP 2

### 1. Simulador de Compra

- Nome da compra.
- Valor total.
- Forma de pagamento.
- À vista ou parcelado.
- Número de parcelas.
- Mês de início.
- Categoria.
- Impacto mensal.
- Resultado textual.
- Classificação de risco.

### 2. Projeção de Meses Futuros

- Simular impacto nos próximos meses.
- Considerar contas recorrentes.
- Considerar faturas previstas.
- Considerar receitas previstas quando disponíveis.
- Mostrar meses afetados.

### 3. Classificação de Risco

- Seguro.
- Controlado.
- Apertado.
- Crítico.

### 4. Recomendação Textual

Exemplos:

- compra possível;
- compra possível, mas reduz margem;
- compra arriscada;
- compra não recomendada;
- adiar um mês reduz o risco.

### 5. Metas Financeiras Separadas

- Nome da meta.
- Valor alvo.
- Valor atual.
- Prazo opcional.
- Prioridade.
- Progresso visual.
- Contribuição manual.

### 6. Histórico de Simulações

- Guardar simulações feitas.
- Permitir revisar simulações anteriores.
- Permitir excluir simulações.

---

# MVP 3 — Experiência Premium

## Objetivo

Refinar a experiência e transformar o produto em um app pessoal completo e polido.

---

## Funcionalidades Incluídas no MVP 3

### 1. PWA

- Instalação no celular.
- Ícone próprio.
- Manifest.
- Service worker se necessário.
- Experiência mobile mais refinada.

### 2. Notificações

- Push ou alternativa viável.
- Conta vence em 3 dias.
- Conta vence hoje.
- Conta vencida.
- Fatura vencendo.

### 3. Exportação

- Exportação CSV.
- Exportação de histórico mensal.
- Backup manual.

### 4. Histórico Visual

- Comparação entre meses.
- Evolução de contas.
- Evolução de faturas.
- Evolução de sobra estimada.

### 5. Gráficos Refinados

- Distribuição por categoria.
- Evolução mensal.
- Faturas por cartão.
- Contas pagas versus pendentes.

### 6. Design System Completo

- Tokens.
- Cards.
- Estados.
- Tabelas.
- Calendário.
- Formulários.
- Modais.
- Microinterações.

---

# Critério de Priorização

Sempre priorizar nesta ordem:

1. controle mensal;
2. marcação de pagamento;
3. vencimentos;
4. dashboard;
5. faturas simples;
6. geração mensal;
7. histórico;
8. simulação;
9. metas;
10. refinamento visual avançado.

---

# Riscos de Escopo

## Risco 1 — Virar app de transações

Não permitir que o MVP vire um controle de cada gasto.

## Risco 2 — Cartão ficar complexo demais

Não cadastrar compra por compra no cartão.

## Risco 3 — Relatórios antes do básico

Relatórios avançados só fazem sentido depois que o app estiver sendo usado.

## Risco 4 — Design atrasar função

A identidade visual é importante, mas o primeiro valor está no uso real.

## Risco 5 — Simulador entrar antes da base

O simulador depende de meses, contas, receitas e faturas funcionando bem.

---

# Regra Final de Escopo

O MVP 1 só deve ser considerado concluído quando o fluxo mensal estiver sólido:

1. gerar mês;
2. revisar contas;
3. ver dashboard;
4. ver calendário;
5. marcar contas como pagas;
6. controlar faturas;
7. guardar histórico.
