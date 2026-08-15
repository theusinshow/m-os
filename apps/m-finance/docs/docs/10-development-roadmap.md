# 10 — Development Roadmap

# M Finance — Roadmap de Desenvolvimento

## Objetivo do Documento

Este documento define a ordem recomendada de desenvolvimento do M Finance.

Ele foi estruturado para orientar implementação em Claude Code, Codex ou Cursor, reduzindo retrabalho e evitando escopo fora de hora.

---

# Estratégia Geral

O desenvolvimento deve priorizar primeiro o fluxo real de uso:

1. entrar no app;
2. criar mês;
3. cadastrar receitas;
4. cadastrar contas;
5. marcar contas como pagas;
6. ver dashboard;
7. ver calendário;
8. controlar faturas;
9. gerar próximo mês;
10. guardar histórico.

---

# Fase 0 — Setup do Projeto

## Objetivo

Criar base técnica limpa.

## Tarefas

- Criar projeto Next.js.
- Configurar TypeScript.
- Configurar Tailwind CSS.
- Configurar ESLint/Prettier se desejado.
- Criar estrutura de pastas.
- Criar layout base.
- Criar tema inicial.
- Configurar variáveis de ambiente.

## Entregável

Projeto rodando localmente.

---

# Fase 1 — Supabase e Autenticação

## Objetivo

Permitir login privado com Google.

## Tarefas

- Criar projeto no Supabase.
- Configurar Google Auth.
- Criar client Supabase.
- Criar rota/login.
- Criar middleware de proteção.
- Criar função `requireUser`.
- Criar allowlist por e-mail autorizado.
- Criar layout privado `/app`.

## Entregável

Usuário consegue logar e acessar dashboard protegido.

---

# Fase 2 — Banco e Schema

## Objetivo

Criar estrutura de dados inicial.

## Tarefas

- Configurar Drizzle.
- Criar schema inicial.
- Criar tabelas:
  - users;
  - months;
  - incomes;
  - bill_categories;
  - recurrence_rules;
  - bills;
  - credit_cards;
  - credit_card_invoices;
  - alerts;
  - monthly_snapshots;
  - settings.
- Criar migrations.
- Configurar RLS no Supabase.
- Criar seed de categorias.
- Criar seed de cartões.

## Entregável

Banco funcional e tipado.

---

# Fase 3 — App Shell

## Objetivo

Criar estrutura visual interna.

## Tarefas

- Criar sidebar desktop.
- Criar topbar.
- Criar mobile nav.
- Criar header de página.
- Criar componentes UI base:
  - Button;
  - Card;
  - Input;
  - Select;
  - Badge;
  - Modal;
  - Drawer;
  - EmptyState;
  - Skeleton.

## Entregável

Layout navegável com rotas internas.

---

# Fase 4 — Mês Atual

## Objetivo

Criar e consultar mês financeiro.

## Tarefas

- Criar server actions de mês.
- Criar `getCurrentMonth`.
- Criar `createMonth`.
- Criar estado vazio quando não houver mês.
- Criar botão “Criar mês atual”.
- Exibir mês no dashboard.

## Entregável

Usuário consegue criar e visualizar o mês atual.

---

# Fase 5 — Receitas

## Objetivo

Cadastrar receitas previstas.

## Tarefas

- Criar schema Zod de receita.
- Criar server actions:
  - createIncome;
  - updateIncome;
  - deleteIncome;
  - markIncomeAsReceived.
- Criar formulário de receita.
- Integrar no dashboard.
- Calcular receita total.

## Entregável

Receitas aparecem no dashboard e entram no cálculo.

---

# Fase 6 — Contas

## Objetivo

Criar principal módulo do app.

## Tarefas

- Criar schema Zod de conta.
- Criar server actions:
  - createBill;
  - updateBill;
  - deleteBill;
  - markBillAsPaid;
  - markBillAsPending.
- Criar lista de contas.
- Criar formulário de conta.
- Criar filtros.
- Criar badges de status.
- Integrar ação “Marcar como pago”.
- Integrar no dashboard.

## Entregável

Usuário consegue cadastrar, listar e pagar contas.

---

# Fase 7 — Categorias

## Objetivo

Organizar contas por categoria.

## Tarefas

- Criar seed de categorias.
- Exibir categorias no formulário.
- Criar tela simples de gerenciamento em settings.
- Permitir criar/editar/arquivar categorias.

## Entregável

Contas categorizadas.

---

# Fase 8 — Dashboard Aggregator

## Objetivo

Criar cálculo central do dashboard.

## Tarefas

- Criar `getDashboardSummary`.
- Calcular:
  - receita total;
  - total contas;
  - total faturas;
  - total pago;
  - total pendente;
  - total vencido;
  - sobra estimada;
  - status do mês;
  - próximos vencimentos.
- Criar cards visuais.
- Criar lista de alertas.

## Entregável

Dashboard funcional e útil.

---

# Fase 9 — Cartões e Faturas

## Objetivo

Controlar faturas simples.

## Tarefas

- Criar CRUD de cartões.
- Criar CRUD de faturas.
- Criar formulário de fatura.
- Criar ação marcar fatura como paga.
- Exibir faturas no dashboard.
- Exibir faturas em cards.
- Indicar cartão PJ visualmente.

## Entregável

Usuário controla faturas mensais dos cartões.

---

# Fase 10 — Calendário Financeiro

## Objetivo

Visualizar vencimentos.

## Tarefas

- Criar calendário mensal.
- Mapear contas por dia.
- Mapear faturas por dia.
- Mapear receitas por dia.
- Criar evento visual.
- Criar detalhe rápido.
- Permitir marcar como pago via detalhe.

## Entregável

Calendário mostra vencimentos do mês.

---

# Fase 11 — Alertas Internos

## Objetivo

Avisar vencimentos relevantes.

## Tarefas

- Criar cálculo de alertas.
- Detectar vencimento em 3 dias.
- Detectar vencimento hoje.
- Detectar vencidos.
- Mostrar alertas no dashboard.
- Criar estado de alerta lido se necessário.

## Entregável

Dashboard mostra alertas úteis.

---

# Fase 12 — Geração Mensal com Revisão

## Objetivo

Criar novo mês baseado em recorrências.

## Tarefas

- Criar recurrence_rules.
- Vincular contas recorrentes.
- Criar função de revisão.
- Buscar valores do mês anterior.
- Montar tela/modal de revisão.
- Permitir editar valores antes de gerar.
- Criar mês confirmado.

## Entregável

Usuário gera novo mês com revisão.

---

# Fase 13 — Histórico Simples

## Objetivo

Guardar e visualizar meses anteriores.

## Tarefas

- Criar monthly_snapshots.
- Gerar snapshot sob demanda ou ao fechar mês.
- Criar rota `/app/history`.
- Exibir lista de meses.
- Exibir resumo mensal.

## Entregável

Histórico simples funcional.

---

# Fase 14 — Polimento do MVP 1

## Objetivo

Tornar o MVP 1 usável no dia a dia.

## Tarefas

- Ajustar responsividade.
- Melhorar empty states.
- Melhorar loading states.
- Melhorar errors.
- Validar fluxo no mobile.
- Corrigir bugs.
- Revisar UX da ação “Marcar como pago”.
- Revisar dashboard.

## Entregável

MVP 1 pronto para uso real.

---

# Fase 15 — MVP 2: Simulador

## Objetivo

Criar decisão de compra.

## Tarefas

- Criar rota `/app/simulator`.
- Criar formulário.
- Criar cálculo à vista.
- Criar cálculo parcelado.
- Projetar meses futuros.
- Classificar risco.
- Gerar recomendação.
- Salvar simulação.

## Entregável

Usuário consegue simular compra futura.

---

# Fase 16 — MVP 2: Metas

## Objetivo

Acompanhar objetivos financeiros.

## Tarefas

- Criar rota `/app/goals`.
- Criar CRUD de metas.
- Criar progresso visual.
- Criar contribuições manuais.
- Criar cards de metas.

## Entregável

Metas acompanhadas separadamente.

---

# Fase 17 — MVP 3

## Objetivo

Refinamento premium.

## Tarefas

- PWA.
- Notificações.
- Exportação CSV.
- Histórico visual.
- Gráficos refinados.
- Aplicação completa do design system.
- Microinterações.
- Backup.

---

# Ordem de Implementação Recomendada para IA

Ao usar Claude Code ou Codex, trabalhar em blocos:

## Bloco 1

Setup + Auth + Layout.

## Bloco 2

Banco + schema + seeds.

## Bloco 3

Mês + receitas + contas.

## Bloco 4

Dashboard aggregator.

## Bloco 5

Cartões + faturas.

## Bloco 6

Calendário + alertas.

## Bloco 7

Geração mensal + histórico.

## Bloco 8

Polimento.

---

# Critério de Conclusão do MVP 1

O MVP 1 está concluído quando:

- login funciona;
- mês pode ser criado;
- receitas podem ser cadastradas;
- contas podem ser cadastradas;
- contas podem ser marcadas como pagas;
- dashboard calcula corretamente;
- calendário mostra vencimentos;
- faturas funcionam;
- alertas aparecem;
- novo mês pode ser gerado com revisão;
- histórico simples existe;
- app é utilizável no desktop e razoável no mobile.

---

# Regra Final do Roadmap

Não implementar o MVP 2 antes do MVP 1 estar utilizável.

O simulador depende de uma base financeira confiável.
