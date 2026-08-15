# 11 — AI Implementation Prompt

# M Finance — Prompt Mestre para Claude Code / Codex

## Objetivo do Documento

Este documento contém o prompt mestre para iniciar a implementação do M Finance em Claude Code, Codex, Cursor ou outra IA de desenvolvimento.

Use este prompt no início do projeto e sempre que abrir uma nova sessão.

---

# Prompt Mestre

```txt
Você está trabalhando no projeto M Finance.

Antes de responder ou implementar, leia todos os arquivos da pasta /docs.

O M Finance é um software web pessoal e privado de planejamento financeiro mensal para Matheus Mendes.

Nome do repositório: m-finance.

O produto NÃO é um app financeiro genérico e NÃO deve virar um clone completo do Mobills.

O objetivo principal do MVP 1 é substituir o uso atual do Mobills: lembrar todas as contas do mês, mostrar vencimentos, controlar faturas simples, permitir marcar contas como pagas e mostrar um dashboard claro do mês atual.

Regra principal do produto:
O M Finance mostra o que ainda pode ser feito com segurança, não apenas o que já foi gasto.

Stack definida:
- Next.js
- TypeScript
- Tailwind CSS
- Supabase
- PostgreSQL
- Supabase Auth com Google
- Drizzle ORM
- Zod
- React Hook Form
- Recharts

Rotas principais:
- /login
- /app/dashboard
- /app/calendar
- /app/bills
- /app/cards
- /app/goals
- /app/simulator
- /app/history
- /app/settings

Prioridade do MVP 1:
1. Login com Google
2. Dashboard mensal
3. Receitas do mês
4. Contas recorrentes e únicas
5. Marcar conta como paga
6. Calendário financeiro
7. Cartões simples
8. Faturas simples
9. Alertas internos
10. Geração mensal com revisão
11. Histórico simples
12. Configurações básicas

Não implemente no MVP 1:
- registro de todas as compras
- integração bancária
- transações detalhadas
- importação de extrato
- exportação CSV/PDF
- notificações push
- PWA completo
- controle avançado de investimentos
- múltiplos usuários
- app público

Regras importantes:
- O mês financeiro vai do dia 1 ao último dia do mês.
- O novo mês deve ser gerado com revisão, não automaticamente sem confirmação.
- Contas recorrentes usam o valor do mês anterior como sugestão.
- Marcar uma conta como paga afeta apenas o mês atual.
- Cartões usam controle simples de fatura; não registrar compra por compra.
- Metas são acompanhamento separado e não entram automaticamente no cálculo de obrigações.
- Alertas internos: 3 dias antes, no dia e depois de vencido.
- Login deve ser com Google e acesso restrito ao e-mail autorizado.

UI/UX:
- visual financeiro limpo + Coded by M
- dashboard estilo cockpit
- desktop como prioridade inicial, mas mobile deve funcionar bem
- dark mode como base provável
- clareza acima de estética
- ação “marcar como pago” deve ser extremamente rápida

Ao implementar:
- trabalhe em blocos pequenos
- explique decisões técnicas antes de mudanças grandes
- não invente funcionalidades fora dos docs
- não crie tabela de transactions no MVP 1
- não crie complexidade desnecessária
- mantenha regras financeiras em lib/calculations
- use Zod para validação
- use server actions para mutações principais
- mantenha componentes visuais separados da regra de negócio

Primeira tarefa recomendada:
Criar setup base do projeto com Next.js, TypeScript, Tailwind, estrutura de pastas, Supabase Auth com Google, layout privado e proteção das rotas /app.
```

---

# Prompt Curto de Continuação

Use quando o projeto já existir:

```txt
Leia todos os arquivos da pasta /docs e continue a implementação do M Finance respeitando o escopo do MVP 1.
Não implemente funcionalidades fora do roadmap.
Antes de alterar arquitetura, explique o motivo.
Priorize o fluxo real: dashboard, contas, vencimentos, faturas simples e marcar como pago.
```

---

# Prompt para Revisão de Escopo

```txt
Revise a tarefa abaixo contra a documentação do M Finance.
Diga se pertence ao MVP 1, MVP 2, MVP 3 ou se está fora de escopo.
Não implemente ainda.
Explique riscos de escopo se houver.

Tarefa:
[cole a tarefa aqui]
```

---

# Prompt para Implementar um Bloco

```txt
Leia a documentação do M Finance em /docs.
Implemente apenas o bloco abaixo, sem adicionar funcionalidades extras.
Ao final, liste arquivos criados/alterados, decisões tomadas e próximos passos.

Bloco:
[descrever bloco]
```

---

# Prompt para Debug

```txt
Analise o erro abaixo dentro do contexto do M Finance.
Leia /docs se necessário.
Corrija o problema com a menor alteração segura possível.
Não refatore partes não relacionadas.

Erro:
[cole o erro]
```

---

# Prompt para UI

```txt
Leia /docs/09-ui-ux-direction.md antes de mexer na interface.
Aplique uma UI de dashboard financeiro limpo com influência Coded by M.
Priorize clareza, hierarquia numérica e ação rápida de marcar como pago.
Não adicione efeitos visuais que prejudiquem uso.
```

---

# Prompt para Banco de Dados

```txt
Leia /docs/06-data-model.md e /docs/07-business-rules.md.
Implemente o schema do banco com Drizzle e PostgreSQL.
Não crie tabela de transações no MVP 1.
Todos os dados financeiros devem ter user_id.
Prepare o schema para RLS no Supabase.
```

---

# Regra Final para IA

Se houver conflito entre uma sugestão da IA e a documentação, a documentação vence.

Se houver dúvida entre completar o produto financeiro inteiro ou manter o MVP focado, manter o MVP focado.
