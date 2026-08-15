# 05 — User Flows

# M Finance — Fluxos de Usuário

## Objetivo do Documento

Este documento descreve os principais fluxos de uso do M Finance.

Cada fluxo deve orientar implementação, UX, componentes e validações.

---

# Fluxo 1 — Primeiro Acesso

## Objetivo

Permitir que Matheus entre no app e configure a base inicial.

## Passos

1. Usuário acessa `/`.
2. Sistema verifica autenticação.
3. Se não autenticado, redireciona para `/login`.
4. Usuário clica em “Entrar com Google”.
5. Supabase autentica.
6. Sistema verifica se o e-mail é autorizado.
7. Se autorizado, redireciona para `/app/dashboard`.
8. Se for primeiro acesso, exibe estado inicial.

## Estado Inicial

O app deve sugerir:

- criar primeiro mês;
- adicionar receita;
- adicionar primeiras contas;
- cadastrar cartões.

## Observação

O app é privado. Não deve permitir onboarding público genérico.

---

# Fluxo 2 — Criar Primeiro Mês

## Objetivo

Criar a base do mês financeiro atual.

## Passos

1. Usuário abre dashboard.
2. Sistema detecta ausência de mês atual.
3. Mostra card “Criar mês atual”.
4. Usuário clica em criar.
5. Sistema abre modal/drawer.
6. Usuário confirma mês e ano.
7. Sistema cria registro de mês.
8. Usuário adiciona receita principal.
9. Usuário adiciona contas.
10. Dashboard recalcula resumo.

## Resultado

Mês atual criado e pronto para uso.

---

# Fluxo 3 — Adicionar Receita

## Objetivo

Cadastrar receita prevista ou recebida.

## Passos

1. Usuário clica em “Adicionar receita”.
2. Sistema abre formulário.
3. Usuário informa:
   - nome;
   - valor;
   - tipo;
   - data prevista;
   - recebido ou previsto.
4. Usuário salva.
5. Sistema valida dados.
6. Receita é criada.
7. Dashboard atualiza resumo.

## Tipos

- Principal.
- Extra.
- Freelance.

## Validações

- Valor obrigatório e maior que zero.
- Nome obrigatório.
- Mês obrigatório.

---

# Fluxo 4 — Adicionar Conta

## Objetivo

Cadastrar uma conta mensal.

## Passos

1. Usuário clica em “Adicionar conta”.
2. Sistema abre formulário.
3. Usuário informa:
   - nome;
   - valor;
   - categoria;
   - vencimento;
   - recorrente ou não;
   - observação opcional.
4. Usuário salva.
5. Sistema valida.
6. Conta aparece no dashboard, calendário e lista de contas.

## Variação — Conta Recorrente

Se recorrente:

- sistema salva regra de recorrência;
- conta aparece no mês atual;
- meses futuros só serão criados na geração mensal revisada.

## Variação — Conta Única

Se não recorrente:

- conta existe apenas naquele mês.

---

# Fluxo 5 — Marcar Conta como Paga

## Objetivo

Executar a ação mais frequente do app.

## Passos

1. Usuário vê conta pendente.
2. Usuário clica em “Marcar como pago”.
3. Sistema atualiza status para pago.
4. Dashboard recalcula:
   - total pago;
   - total pendente;
   - vencidos;
   - sobra estimada.
5. Conta aparece como paga.

## Regras

- Não pedir comprovante.
- Não pedir método de pagamento.
- Não pedir data, pelo menos no MVP 1.
- A ação deve ser simples.

## Undo

Recomendável ter opção de desfazer por alguns segundos.

---

# Fluxo 6 — Ver Contas do Mês

## Objetivo

Listar e gerenciar contas.

## Passos

1. Usuário acessa `/app/bills`.
2. Sistema lista contas do mês selecionado.
3. Usuário pode filtrar por status.
4. Usuário pode filtrar por categoria.
5. Usuário pode marcar como pago.
6. Usuário pode editar ou excluir.

## Ordenação Padrão

Por vencimento crescente.

---

# Fluxo 7 — Ver Calendário Financeiro

## Objetivo

Visualizar vencimentos no mês.

## Passos

1. Usuário acessa `/app/calendar`.
2. Sistema exibe calendário mensal.
3. Contas aparecem no dia de vencimento.
4. Faturas aparecem no dia de vencimento.
5. Receitas aparecem no dia previsto.
6. Usuário clica em um item.
7. Sistema abre detalhe rápido.
8. Usuário pode marcar como pago se aplicável.

---

# Fluxo 8 — Cadastrar Cartão

## Objetivo

Criar cartão para controlar faturas simples.

## Passos

1. Usuário acessa `/app/cards` ou configurações.
2. Clica em “Adicionar cartão”.
3. Informa:
   - nome;
   - tipo pessoal/PJ;
   - dia de vencimento.
4. Salva.
5. Cartão fica disponível para faturas.

## Cartões previstos

- Mercado Pago.
- Itaú.
- Nubank Pessoal.
- Nubank PJ.

---

# Fluxo 9 — Adicionar Fatura

## Objetivo

Registrar valor total de fatura do mês.

## Passos

1. Usuário acessa `/app/cards`.
2. Seleciona cartão.
3. Clica em “Adicionar fatura”.
4. Informa:
   - mês;
   - valor;
   - vencimento.
5. Salva.
6. Fatura aparece no dashboard e calendário.

## Regra

Não cadastrar compras individuais.

---

# Fluxo 10 — Marcar Fatura como Paga

## Objetivo

Atualizar status de fatura.

## Passos

1. Usuário vê fatura pendente.
2. Clica em “Marcar como paga”.
3. Sistema atualiza status.
4. Dashboard recalcula resumo.

---

# Fluxo 11 — Virada de Mês com Revisão

## Objetivo

Criar novo mês com base em contas recorrentes.

## Passos

1. Usuário abre app no dia 1 ou em mês sem registro.
2. Sistema detecta novo mês.
3. Sistema exibe aviso:

```txt
Novo mês disponível. Deseja gerar com base nas recorrências?
```

4. Usuário escolhe “Revisar antes”.
5. Sistema lista:
   - contas recorrentes;
   - valores sugeridos;
   - faturas previstas se aplicável;
   - receita sugerida se existir.
6. Usuário edita valores se necessário.
7. Usuário confirma.
8. Sistema cria o novo mês.
9. Dashboard passa a mostrar o novo mês.

## Regra

O app não deve gerar mês automaticamente sem revisão.

---

# Fluxo 12 — Valor Variável Sugerido

## Objetivo

Facilitar contas com valor variável.

## Exemplo

Energia em junho: R$ 180.

Na geração de julho, o sistema sugere R$ 180.

## Passos

1. Sistema busca última ocorrência da conta recorrente.
2. Copia valor como sugestão.
3. Usuário pode manter ou editar.
4. Novo mês é criado com valor confirmado.

---

# Fluxo 13 — Alertas Internos

## Objetivo

Avisar vencimentos relevantes.

## Passos

1. Sistema calcula diferença entre hoje e vencimento.
2. Se faltar 3 dias, cria alerta preventivo.
3. Se vencer hoje, cria alerta do dia.
4. Se passou e não foi pago, cria alerta vencido.
5. Alertas aparecem no dashboard.

## MVP 1

Alertas apenas internos.

---

# Fluxo 14 — Consultar Histórico

## Objetivo

Ver resumo de meses anteriores.

## Passos

1. Usuário acessa `/app/history`.
2. Sistema lista meses anteriores.
3. Usuário seleciona mês.
4. Sistema mostra resumo.

## Dados

- receita total;
- contas;
- faturas;
- pago;
- pendente;
- vencido;
- sobra;
- status.

---

# Fluxo 15 — Simular Compra

## Status

MVP 2.

## Objetivo

Avaliar se uma compra cabe no orçamento.

## Passos

1. Usuário acessa `/app/simulator`.
2. Informa:
   - nome;
   - valor total;
   - à vista ou parcelado;
   - parcelas;
   - mês de início;
   - categoria.
3. Sistema calcula impacto.
4. Sistema projeta meses afetados.
5. Sistema classifica risco.
6. Sistema mostra recomendação.

## Resultado

Exemplo:

```txt
Compra possível, mas julho ficará apertado.
Risco: Controlado.
```

---

# Fluxo 16 — Criar Meta

## Status

MVP 2.

## Objetivo

Acompanhar objetivo financeiro.

## Passos

1. Usuário acessa `/app/goals`.
2. Clica em “Nova meta”.
3. Informa:
   - nome;
   - valor alvo;
   - valor atual;
   - prazo;
   - prioridade.
4. Salva.
5. Meta aparece com progresso.

## Regra

Meta não entra automaticamente no cálculo das obrigações.

---

# Fluxo 17 — Editar Configurações

## Objetivo

Ajustar base do sistema.

## Passos

1. Usuário acessa `/app/settings`.
2. Edita categorias, cartões ou preferências.
3. Sistema salva alterações.
4. Interface reflete mudanças.

---

# Regra Final dos Fluxos

Todo fluxo deve exigir o mínimo possível de campos.

A experiência deve ser rápida, principalmente para:

- marcar como pago;
- adicionar conta;
- revisar novo mês;
- adicionar fatura.
