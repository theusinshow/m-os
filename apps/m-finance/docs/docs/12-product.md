# Product.md — M Finance

**Projeto:** `m-finance`  
**Produto:** M Finance  
**Tipo:** Software web pessoal de planejamento financeiro mensal  
**Usuário principal:** Matheus Mendes  
**Versão do documento:** 1.0  
**Status:** Documento base para implementação com Claude Code, Codex, Cursor ou outro agente de desenvolvimento.

---

## 1. Resumo executivo

O **M Finance** é um software web pessoal criado para substituir o uso atual de aplicativos financeiros genéricos, como Mobills, no controle de contas mensais.

O produto não existe para registrar cada compra, cada gasto pequeno ou cada transação financeira do dia a dia. O objetivo é concentrar o controle financeiro no que realmente impacta a organização mensal de Matheus:

- receitas previstas;
- receitas extras;
- contas mensais;
- vencimentos;
- faturas simples de cartão;
- contas pagas;
- contas pendentes;
- alertas de vencimento;
- calendário financeiro;
- histórico mensal simples;
- simulação de compras futuras;
- acompanhamento de metas.

A pergunta central do produto é:

> **O que eu tenho que pagar, o que já paguei, o que ainda vence e se posso comprar mais alguma coisa sem bagunçar os próximos meses?**

O M Finance deve funcionar como um **cockpit financeiro pessoal**, com alto nível de clareza, pouco atrito de uso e uma experiência visual própria.

---

## 2. Frase-guia do produto

> **O M Finance mostra o que ainda pode ser feito com segurança, não apenas o que já foi gasto.**

Essa frase deve orientar decisões de produto, design, arquitetura, copy e desenvolvimento.

Sempre que houver dúvida entre adicionar mais controle detalhado ou manter a experiência simples, priorizar o que ajuda Matheus a tomar decisões mensais com menos esforço.

---

## 3. Categoria do produto

O M Finance não deve ser tratado como um aplicativo financeiro tradicional completo.

### Categoria correta

```txt
Cockpit pessoal de contas, vencimentos e poder de compra.
```

### Categoria expandida

```txt
Sistema web pessoal de planejamento financeiro mensal, focado em vencimentos, obrigações e tomada de decisão sobre compras futuras.
```

### Categorias que o produto não deve assumir

O M Finance **não** deve se tornar:

- app de controle de transações;
- app de extrato bancário;
- app de conciliação financeira;
- ERP pessoal;
- app de investimentos;
- app de contabilidade;
- clone do Mobills;
- sistema empresarial;
- sistema multiusuário comercial;
- app para registrar toda compra no cartão.

---

## 4. Problema que o produto resolve

Matheus usa um app financeiro principalmente para lembrar, no início de cada mês, tudo que precisa pagar.

O problema do uso atual é:

- o app genérico cobra pelo uso;
- possui funcionalidades além do necessário;
- não tem a identidade visual desejada;
- não segue o modelo mental específico de Matheus;
- exige ou incentiva um nível de registro financeiro maior do que ele realmente quer manter;
- não é uma experiência própria, construída sob medida.

O M Finance resolve isso criando um ambiente único, privado e focado na rotina real:

1. Entrar no app.
2. Ver o mês.
3. Ver o que precisa pagar.
4. Marcar o que já foi pago.
5. Entender o que falta.
6. Avaliar se pode comprar algo novo.

---

## 5. Usuário e escopo pessoal

### Usuário principal

```txt
Matheus Mendes
```

### Tipo de uso

```txt
Uso pessoal, individual e exclusivo.
```

### Implicações de produto

Como o produto é de uso individual:

- não precisa cadastro público aberto;
- não precisa fluxo de onboarding complexo;
- não precisa planos pagos;
- não precisa área administrativa multiusuário;
- não precisa recursos de colaboração;
- não precisa permissões por equipe;
- não precisa convite de usuários;
- não precisa suporte a múltiplas organizações.

Mesmo assim, o sistema deve ser bem arquitetado, seguro e escalável o suficiente para evoluir futuramente.

---

## 6. Princípios de produto

### 6.1. Clareza antes de volume

O M Finance deve mostrar a informação certa, não toda a informação possível.

Errado:

```txt
Listar todas as transações do mês como centro do produto.
```

Correto:

```txt
Mostrar o que vence, o que está pago, o que está pendente e quanto ainda sobra.
```

---

### 6.2. Baixo atrito de uso

A ação mais usada será:

```txt
Marcar como pago.
```

Essa ação deve ser extremamente rápida, visível e acessível.

Não exigir campos extras no MVP 1, como:

- data real de pagamento;
- comprovante;
- conta bancária usada;
- forma de pagamento;
- observação obrigatória.

Esses campos podem existir futuramente, mas não devem atrapalhar o fluxo principal.

---

### 6.3. Planejamento acima de contabilidade

O M Finance deve olhar para o presente e para os próximos meses.

O objetivo principal não é explicar detalhadamente o passado, mas ajudar Matheus a tomar melhores decisões sobre o mês atual e as compras futuras.

---

### 6.4. Manual, mas inteligente

Os dados serão lançados manualmente.

Mesmo assim, o app deve reduzir trabalho repetitivo por meio de:

- contas recorrentes;
- geração mensal com revisão;
- sugestão de valores com base no mês anterior;
- status automáticos;
- alertas internos;
- histórico simples.

---

### 6.5. Privado por padrão

O app contém dados financeiros pessoais.

Portanto:

- deve exigir login;
- deve usar autenticação com Google;
- deve proteger todas as rotas internas;
- deve manter dados vinculados ao usuário autenticado;
- não deve expor dados em páginas públicas;
- não deve permitir acesso sem sessão válida.

---

## 7. Escopo funcional geral

### Funcionalidades centrais

O M Finance deve permitir:

- login com Google;
- visualização do dashboard mensal;
- cadastro de receitas;
- cadastro de receitas extras/freelancers;
- cadastro de contas fixas e variáveis;
- definição de contas recorrentes;
- geração mensal com revisão;
- marcação de contas como pagas;
- listagem de contas pendentes;
- listagem de contas vencidas;
- controle simples de cartões;
- cadastro simples de faturas;
- calendário financeiro;
- alertas internos;
- histórico mensal simples;
- configurações básicas;
- simulador de compras futuras no MVP 2;
- metas financeiras separadas no MVP 2.

---

## 8. Fora de escopo

### Fora do MVP 1

O MVP 1 **não** deve incluir:

- registro de cada compra no cartão;
- importação de extrato bancário;
- integração com banco;
- leitura automática de SMS ou e-mail;
- OCR de boletos;
- conciliação financeira;
- controle de investimentos;
- relatórios avançados;
- exportação CSV/PDF;
- notificações push;
- PWA completo;
- multiusuário;
- planos pagos;
- app mobile nativo;
- dashboard público;
- IA financeira;
- previsão automática complexa.

### Fora do produto como regra geral

O produto não deve incentivar o lançamento de microgastos.

Exemplo de itens que não precisam entrar individualmente:

- café;
- lanche;
- compra pequena de mercado;
- compra avulsa no cartão;
- gasto pontual irrelevante;
- transação bancária pequena.

Esses itens só devem ser cadastrados se forem tratados como uma conta, fatura, evento financeiro ou compra planejada relevante.

---

## 9. Modelo mental financeiro

O cálculo principal do M Finance é:

```txt
Receita total prevista do mês
- contas do mês
- faturas do mês
= sobra prevista
```

As metas, na definição atual, ficam separadas do cálculo obrigatório.

Modelo detalhado:

```txt
Receita principal do mês
+ receitas extras
+ freelancers
= receita total prevista

Receita total prevista
- contas pagas
- contas pendentes
- faturas pagas
- faturas pendentes
= sobra prevista
```

A sobra prevista não é uma margem segura fixa. Como Matheus não definiu uma margem mínima obrigatória, o sistema deve apenas classificar o mês por status.

---

## 10. Status do mês

O dashboard deve classificar o mês em estados simples.

### Estados recomendados

```txt
Positivo
Justo
Apertado
Negativo
```

### Interpretação

#### Positivo

O mês tem sobra prevista confortável.

#### Justo

O mês fecha positivo, mas sem muita folga.

#### Apertado

O mês ainda pode fechar positivo, mas qualquer compra extra pode comprometer a organização.

#### Negativo

As obrigações previstas ultrapassam a receita prevista.

---

## 11. Receita

### Modelo escolhido

```txt
Receita fixa variável + freelancers.
```

Matheus possui uma receita principal, mas o valor pode mudar mensalmente. Além disso, há receitas extras por freelancers.

### Tipos de receita

```txt
Principal
Freelancer
Extra
```

### Campos mínimos

```txt
Nome
Valor
Tipo
Mês de referência
Data prevista opcional
Recebido: sim/não
```

### Regra

A receita principal pode ser sugerida com base no mês anterior, mas deve ser editável em cada novo mês.

---

## 12. Contas

### Tipos de conta

```txt
Recorrente
Única
Variável
```

### Campos mínimos

```txt
Nome
Valor
Categoria
Vencimento
Recorrente: sim/não
Status
Mês de referência
```

### Status

```txt
Pendente
Pago
Vencido
```

### Ação principal

```txt
Marcar como pago
```

### Regra de usabilidade

A ação de marcar como pago deve estar disponível:

- no dashboard;
- na lista de contas;
- no calendário;
- nos alertas;
- nos detalhes da conta.

---

## 13. Contas recorrentes

### Regra escolhida

Quando uma conta recorrente for marcada como paga no mês atual, apenas aquele mês muda de status.

Exemplo:

```txt
Internet — Junho — Pago
Internet — Julho — Pendente
Internet — Agosto — Pendente
```

A recorrência permanece ativa.

---

## 14. Geração mensal

### Regra escolhida

No dia 1 de cada mês, o app deve perguntar antes de gerar o novo mês.

Fluxo ideal:

```txt
Novo mês disponível: Julho 2026
Deseja gerar o mês com base nas contas recorrentes?

[Revisar antes]
[Gerar mês]
```

### Revisão antes de gerar

A revisão deve mostrar:

- contas recorrentes que serão criadas;
- faturas previstas;
- receitas sugeridas;
- contas variáveis com valor sugerido;
- contas sem valor definido;
- possíveis pendências do mês anterior.

### Regra de segurança

O app não deve criar o mês novo silenciosamente sem confirmação de Matheus.

---

## 15. Valores variáveis

### Regra escolhida

Usar o valor do mês anterior como sugestão.

Exemplo:

```txt
Energia — Junho: R$ 180
Energia — Julho: sugerido R$ 180
```

O valor deve ser editável antes de confirmar o novo mês.

---

## 16. Cartões

### Modelo escolhido

Controle simples de fatura.

O sistema deve controlar o valor da fatura, não cada compra do cartão.

### Cartões iniciais

```txt
Mercado Pago
Itaú
Nubank Pessoal
Nubank PJ
```

Observação: embora o usuário tenha citado “3 cartões”, listou quatro entidades. O sistema deve permitir cadastrar quatro cartões e diferenciar cartões pessoais e PJ.

### Campos de cartão

```txt
Nome
Tipo: pessoal ou PJ
Dia de vencimento
Ativo: sim/não
```

### Campos de fatura

```txt
Cartão
Mês de referência
Valor
Data de vencimento
Status
```

### Status de fatura

```txt
Pendente
Pago
Vencido
```

---

## 17. Calendário financeiro

O calendário deve ser uma das telas principais.

### Deve exibir

- contas por dia;
- faturas por dia;
- receitas previstas;
- status de pagamento;
- alertas visuais;
- filtros simples.

### Visões recomendadas

```txt
Mês
Semana
Lista
```

Para o MVP 1, a visão mensal pode ser suficiente, desde que exista uma lista lateral ou inferior de eventos do dia selecionado.

---

## 18. Alertas

### Regras escolhidas

Alertas devem aparecer:

```txt
3 dias antes
No dia do vencimento
Depois de vencido
```

### Tipos de alerta

```txt
Conta vence em 3 dias
Conta vence hoje
Conta vencida
Fatura vence em 3 dias
Fatura vence hoje
Fatura vencida
Novo mês disponível para geração
Receita principal ainda não cadastrada
```

### Local dos alertas

No MVP 1, alertas são internos do app.

Podem aparecer em:

- dashboard;
- topo da tela;
- painel de alertas;
- calendário;
- cards de contas.

Notificações push ficam para MVP 3.

---

## 19. Histórico mensal simples

### Modelo escolhido

Guardar histórico mensal simples.

### Dados do histórico

```txt
Mês
Ano
Receita total
Total de contas
Total de faturas
Total pago
Total pendente
Total vencido
Sobra final estimada
Status final do mês
Quantidade de contas pagas
Quantidade de contas vencidas
```

### Objetivo

Permitir que Matheus veja se os meses estão melhorando ou piorando, sem transformar o app em uma ferramenta analítica complexa no MVP 1.

---

## 20. Metas

### Modelo escolhido

Metas como acompanhamento separado.

As metas não entram automaticamente no cálculo de obrigações do mês.

### Campos mínimos

```txt
Nome
Valor alvo
Valor atual
Prazo opcional
Prioridade
Status
```

### Exemplos

```txt
Reserva de emergência
Moto
Viagem
Setup
Quitar dívida
```

### MVP

Metas entram no MVP 2.

---

## 21. Simulador de compra

### Valor estratégico

O simulador é uma das funcionalidades de maior valor do produto.

Ele responde:

> **Posso comprar isso agora?**

### Entradas

```txt
Nome da compra
Valor total
Forma de pagamento: à vista ou parcelado
Número de parcelas
Mês de início
Categoria opcional
Prioridade opcional
```

### Saídas

```txt
Pode comprar?
Risco geral
Impacto por mês
Recomendação textual
Meses mais afetados
```

### Tipos de resposta

A resposta deve combinar:

- resposta direta;
- classificação de risco;
- recomendação textual;
- impacto mensal.

Exemplo:

```txt
Compra possível, mas julho ficará apertado.
Risco: Controlado.
Impacto: R$ 250/mês por 4 meses.
Recomendação: evite assumir outra parcela no mesmo período.
```

### MVP

Simulador entra no MVP 2.

---

## 22. Dashboard

O dashboard é a tela inicial e o coração do produto.

### Deve responder

```txt
Como está meu mês financeiro agora?
```

### Blocos obrigatórios

```txt
Resumo do mês
Receita prevista
Total a pagar
Total pago
Total pendente
Total vencido
Sobra prevista
Status do mês
Próximos vencimentos
Faturas
Alertas
Ações rápidas
```

### Ações rápidas

```txt
Adicionar conta
Adicionar receita
Adicionar fatura
Marcar conta como paga
Gerar novo mês
Simular compra
```

A ação “Simular compra” pode aparecer bloqueada ou como teaser no MVP 1 se ainda não estiver implementada.

---

## 23. Arquitetura de navegação

Rotas principais:

```txt
/dashboard
/calendar
/bills
/cards
/goals
/simulator
/settings
```

### Prioridade de rotas no MVP 1

```txt
/dashboard
/calendar
/bills
/cards
/settings
```

### Rotas do MVP 2

```txt
/goals
/simulator
```

---

## 24. Stack definida

### Stack base

```txt
Next.js
TypeScript
Tailwind CSS
Supabase
PostgreSQL
Supabase Auth com Google
Drizzle ORM
Zod
React Hook Form
Recharts
```

### Motivo da stack

- Next.js fornece base moderna para app web.
- TypeScript reduz erros em regras financeiras.
- Tailwind acelera implementação do design system.
- Supabase resolve autenticação e banco sem backend complexo.
- PostgreSQL oferece estrutura relacional sólida.
- Drizzle mantém controle direto sobre schema e queries.
- Zod valida entradas e regras de formulário.
- React Hook Form simplifica formulários.
- Recharts cobre gráficos simples do dashboard.

---

## 25. Autenticação

### Modelo escolhido

```txt
Login com Google.
```

### Regras

- Rotas internas exigem autenticação.
- Usuário não autenticado deve ser redirecionado para login.
- Dados devem ser associados ao ID do usuário.
- O app não deve permitir acesso público ao dashboard.

---

## 26. Responsividade

### Escolha feita

```txt
Desktop é prioridade, mas mobile deve funcionar bem.
```

### Interpretação

O app deve ser mais confortável no desktop, especialmente para dashboard, calendário e gestão de contas.

Porém, ações rápidas no mobile precisam ser boas, principalmente:

- ver próximas contas;
- marcar como pago;
- ver alertas;
- consultar calendário;
- adicionar conta simples.

---

## 27. Tom de interface

A interface deve falar de forma objetiva e útil.

### Exemplos bons

```txt
3 contas vencem nos próximos dias.
Você ainda tem R$ 820 previstos para o mês.
Julho ficará apertado se esta compra for confirmada.
Fatura Nubank vence hoje.
```

### Exemplos ruins

```txt
Uau, sua jornada financeira está incrível!
Vamos turbinar sua vida financeira!
Parabéns por dominar seu dinheiro!
```

O tom deve ser direto, técnico e calmo.

---

## 28. Critérios de sucesso do MVP 1

O MVP 1 será considerado bem-sucedido se permitir que Matheus:

1. Entre com Google.
2. Veja o mês atual.
3. Cadastre receitas do mês.
4. Cadastre contas recorrentes.
5. Gere novo mês revisando contas.
6. Marque contas como pagas rapidamente.
7. Veja contas vencendo em calendário.
8. Controle faturas simples.
9. Receba alertas internos.
10. Substitua o uso atual do Mobills para lembrar o que deve pagar.

---

## 29. Riscos de produto

### Risco 1 — virar app financeiro genérico

Evitar adicionar funcionalidades só porque outros apps possuem.

### Risco 2 — excesso de campos

Campos demais reduzem o uso real.

### Risco 3 — dashboard poluído

Cockpit financeiro não significa tela confusa.

### Risco 4 — visual acima da função

O design pode ser premium, mas a informação financeira precisa continuar clara.

### Risco 5 — simulação complexa demais

O simulador deve começar simples e compreensível.

---

## 30. Ordem recomendada de implementação

1. Setup do projeto.
2. Supabase Auth com Google.
3. Layout protegido.
4. Modelagem inicial do banco.
5. CRUD de categorias.
6. CRUD de contas.
7. Dashboard com dados reais.
8. Marcar como pago.
9. Contas recorrentes.
10. Geração mensal com revisão.
11. Cartões e faturas simples.
12. Calendário financeiro.
13. Alertas internos.
14. Histórico simples.
15. Polimento visual.
16. MVP 2: metas.
17. MVP 2: simulador.

---

## 31. Regra final para agentes de IA

Qualquer agente implementando este projeto deve seguir esta regra:

> **Não transformar o M Finance em um app de transações. O produto é um cockpit mensal de obrigações, vencimentos e decisão de compra.**

Antes de implementar qualquer funcionalidade nova, verificar se ela reforça uma destas ações:

- entender o mês;
- lembrar vencimentos;
- marcar pagamentos;
- prever sobra;
- controlar faturas simples;
- simular compras;
- acompanhar metas.

Se não reforçar nenhuma dessas ações, a funcionalidade provavelmente está fora do escopo.
