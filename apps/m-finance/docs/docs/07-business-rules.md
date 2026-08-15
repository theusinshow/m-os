# 07 — Business Rules

# M Finance — Regras de Negócio

## Objetivo do Documento

Este documento define as regras internas do M Finance.

As regras de negócio devem orientar cálculos, estados, geração de mês, recorrência, alertas, faturas e simulações.

---

# Regra 1 — Mês Financeiro

O mês financeiro começa no dia 1 e termina no último dia do mesmo mês.

Exemplo:

```txt
Junho 2026 = 01/06/2026 até 30/06/2026
Julho 2026 = 01/07/2026 até 31/07/2026
```

Não usar mês financeiro baseado no dia de recebimento.

---

# Regra 2 — Criação de Novo Mês

O sistema não deve gerar automaticamente um novo mês sem revisão.

Quando detectar que o mês atual ainda não existe, deve perguntar:

```txt
Novo mês disponível. Deseja gerar com base nas contas recorrentes?
```

Opções:

- Revisar antes.
- Gerar mês.
- Cancelar.

## Recomendação

O fluxo padrão deve ser “Revisar antes”.

---

# Regra 3 — Geração Mensal com Revisão

Ao revisar novo mês, o sistema deve listar:

- contas recorrentes ativas;
- valor sugerido;
- vencimento sugerido;
- categoria;
- opção de editar;
- opção de remover daquele mês;
- receita principal sugerida, se aplicável;
- faturas previstas, se aplicável.

Apenas após confirmação o mês é criado.

---

# Regra 4 — Contas Recorrentes

Uma conta recorrente representa um compromisso que tende a se repetir mensalmente.

Exemplos:

- internet;
- faculdade;
- assinatura;
- academia;
- celular;
- consórcio;
- seguro;
- aluguel.

Ao marcar uma ocorrência como paga, apenas aquela ocorrência mensal é alterada.

A recorrência permanece ativa para meses futuros.

---

# Regra 5 — Conta Única

Uma conta única existe apenas em um mês.

Exemplos:

- manutenção;
- compra pontual;
- taxa anual;
- imposto;
- reparo;
- evento específico.

Ela não deve aparecer em meses futuros.

---

# Regra 6 — Valor Variável

Para contas recorrentes com valor variável, o sistema deve sugerir o valor do mês anterior.

Exemplo:

```txt
Energia Junho: R$ 180
Energia Julho sugerido: R$ 180
```

O usuário pode editar antes de confirmar.

---

# Regra 7 — Status de Conta

Uma conta pode ter os seguintes status:

```txt
pending
paid
overdue
```

## Pendente

Conta ainda não paga e vencimento não passou.

## Pago

Conta marcada como paga.

## Vencido

Conta não paga e data de vencimento menor que a data atual.

---

# Regra 8 — Atualização de Status Vencido

O sistema deve detectar contas vencidas automaticamente.

Se:

```txt
due_date < today
and status != paid
```

Então:

```txt
status = overdue
```

Essa atualização pode ocorrer:

- ao carregar dashboard;
- em server action;
- em job futuro;
- em cálculo derivado, sem gravar imediatamente.

Para MVP 1, pode ser cálculo derivado na consulta.

---

# Regra 9 — Marcar Conta como Paga

Ao marcar conta como paga:

- status passa para `paid`;
- `paid_at` recebe timestamp atual;
- dashboard recalcula totais;
- alertas relacionados podem ser resolvidos ou ocultados.

Não exigir:

- comprovante;
- método de pagamento;
- data manual;
- observação.

---

# Regra 10 — Desmarcar Pago

O sistema deve permitir reverter uma conta paga para pendente, pelo menos por edição.

Opcional no MVP 1:

- botão desfazer logo após marcar como pago.

---

# Regra 11 — Cartões

Cartões são usados apenas para controlar faturas simples.

Não registrar compra por compra.

Cada cartão tem:

- nome;
- tipo pessoal ou PJ;
- dia de vencimento;
- status ativo.

---

# Regra 12 — Cartão PJ

Cartões PJ devem ter identificação visual clara.

Motivo:

- evitar confusão entre vida pessoal e profissional;
- manter clareza no dashboard;
- permitir evolução futura para filtros pessoal/PJ.

No MVP 1, ainda pode entrar no resumo geral, mas deve estar marcado como PJ.

---

# Regra 13 — Faturas

Uma fatura representa o valor total mensal de um cartão.

Campos principais:

- cartão;
- mês;
- valor;
- vencimento;
- status.

---

# Regra 14 — Status de Fatura

Uma fatura pode ter:

```txt
pending
paid
overdue
```

Mesma lógica de contas.

---

# Regra 15 — Marcar Fatura como Paga

Ao marcar fatura como paga:

- status passa para `paid`;
- `paid_at` recebe timestamp atual;
- dashboard recalcula totais.

---

# Regra 16 — Receita

Receitas podem ser:

```txt
main
extra
freelance
```

A receita principal é variável por mês.

Não assumir valor fixo permanente.

O sistema pode sugerir valor anterior, mas deve permitir alteração.

---

# Regra 17 — Receita Total

A receita total prevista do mês é:

```txt
receita principal + receitas extras + freelancers
```

Receitas não recebidas ainda podem entrar como previstas, desde que cadastradas.

---

# Regra 18 — Sobra Estimada

A sobra estimada do mês é:

```txt
receita total prevista - total de contas - total de faturas
```

Metas não entram nesse cálculo no MVP 1/MVP 2, pois são acompanhamento separado.

---

# Regra 19 — Total de Contas

O total de contas inclui:

- contas pagas;
- contas pendentes;
- contas vencidas.

A ideia é entender o comprometimento total do mês.

---

# Regra 20 — Total Pago

O total pago inclui:

- contas pagas;
- faturas pagas.

---

# Regra 21 — Total Pendente

O total pendente inclui:

- contas pendentes;
- faturas pendentes.

---

# Regra 22 — Total Vencido

O total vencido inclui:

- contas vencidas;
- faturas vencidas.

---

# Regra 23 — Saúde do Mês

Como não há margem mínima definida, o sistema deve classificar o mês com base na sobra estimada e nos vencidos.

Estados:

```txt
positive
fair
tight
negative
```

## Positive

Sobra positiva confortável e sem vencidos.

## Fair

Sobra positiva, mas baixa.

## Tight

Sobra próxima de zero ou há vários compromissos próximos.

## Negative

Sobra negativa ou existência de contas/faturas vencidas relevantes.

## Observação

Os limites exatos podem ser ajustados depois.

---

# Regra 24 — Alertas

Alertas internos devem ser gerados para:

- vencimento em 3 dias;
- vencimento hoje;
- vencido.

Aplicável a:

- contas;
- faturas.

---

# Regra 25 — Alerta 3 Dias Antes

Se:

```txt
due_date - today = 3 dias
and status != paid
```

Então gerar alerta preventivo.

---

# Regra 26 — Alerta No Dia

Se:

```txt
due_date = today
and status != paid
```

Então gerar alerta urgente.

---

# Regra 27 — Alerta Vencido

Se:

```txt
due_date < today
and status != paid
```

Então gerar alerta vencido.

---

# Regra 28 — Histórico Simples

O histórico mensal deve preservar resumo do mês.

Dados:

- receita total;
- total de contas;
- total de faturas;
- total pago;
- total pendente;
- total vencido;
- sobra estimada;
- saúde do mês.

---

# Regra 29 — Metas

Metas são acompanhamento separado.

Elas não entram automaticamente no cálculo de contas ou sobra.

Motivo:

- meta não é boleto;
- evitar distorção do mês;
- permitir acompanhamento mais flexível.

---

# Regra 30 — Simulação de Compra

MVP 2.

O simulador deve calcular impacto de:

- compra à vista;
- compra parcelada.

Para compra parcelada:

```txt
monthly_impact = total_amount / installments
```

O impacto deve ser aplicado a partir do mês de início por N meses.

---

# Regra 31 — Resultado da Simulação

O resultado deve conter:

- resposta direta;
- risco;
- impacto por mês;
- recomendação textual.

Exemplo:

```txt
Pode comprar, mas julho ficará apertado.
Risco: controlado.
```

---

# Regra 32 — Não Criar Transações

No MVP 1, não criar entidade nem tela de transações.

O produto não deve exigir lançamento de toda compra.

---

# Regra 33 — Edição e Exclusão

Todo item criado manualmente deve poder ser editado e excluído.

Para itens recorrentes, sempre diferenciar:

- editar apenas este mês;
- editar recorrência futura.

---

# Regra 34 — Segurança

Todo dado pertence a um usuário.

O usuário só pode acessar seus próprios dados.

O app é privado e deve restringir acesso ao e-mail autorizado.

---

# Regra Final

Sempre que uma regra gerar complexidade excessiva, escolher a alternativa que mantém o app útil, simples e fiel ao uso mensal real.
