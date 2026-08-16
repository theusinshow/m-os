# Spec — ações do Hermes sobre os Apps do M/OS

**Status:** proposta, aguardando aprovação
**Data:** 2026-08-15
**Caso motivador:** *"adicione uma conta de água recorrente, 200 reais por mês, vencimento dia 10"* → o Hermes cria a regra no M-Finance.

---

## 1. A decisão que define tudo: o modelo propõe, o M/OS executa

A pergunta natural é "como dou ao Hermes acesso ao M-Finance?". A resposta é: **não dou.**

O Hermes roda numa VPS. Dar a ele um caminho de escrita nas suas finanças significa que uma
credencial de escrita vive fora da sua máquina, e que uma frase mal interpretada vira um
lançamento. O desenho abaixo evita isso invertendo quem age:

```
o Hermes escreve uma PROPOSTA          (texto, sem poder)
o M/OS valida, mostra e executa        (código local, com poder)
```

O modelo nunca executa nada. Ele devolve uma frase estruturada, e quem age é o M/OS —
pelos mesmos serviços que a interface usa, com a mesma classificação de risco.

Isso também resolve um problema técnico real: o protocolo do gateway **não tem registro de
ferramenta do lado do cliente** (verificado em `tui_gateway/server.py`, ver
`HERMES-PREMIUM-CHAT.md` §6.4). Não há como o agente chamar o M/OS de volta no meio do
turno sem MCP ou fork. A proposta não precisa disso: ela cabe no texto da resposta.

**Custo honesto:** o agente não itera. Ele propõe uma vez; não faz "executa → vê resultado
→ corrige". Para criar uma conta isso é irrelevante. Para trabalho agêntico longo, não é —
e aí a ADR-028 já registra o caminho de upgrade (MCP local, com ADR própria).

---

## 2. As peças já existem, e é isso que torna barato

Três coisas construídas antes desta conversa encaixam sem adaptação:

| Peça | Onde | O que já resolve |
|---|---|---|
| Registro de Functions | `crates/mos-core/src/functions.rs` | 21 ações com `risk` e `confirmation` declarados |
| Capacidades de App | `RegisteredApp.canOpen/canRead/canWrite/canAutomate` | quais Apps aceitam escrita |
| Injeção de contexto | `jarvis.rs::assemble_context` (ADR-028) | o canal por onde o catálogo de ações desce |

O comentário que já está no código do registro de Apps diz exatamente o que esta spec
formaliza: *"Capacidade não declarada é capacidade que o Hermes não tenta usar."*

---

## 3. A cadeia, no caso da conta de água

```
1  "adicione uma conta de água recorrente, R$200/mês, vence dia 10"

2  M/OS injeta no prompt: contexto + CATÁLOGO DE AÇÕES
   (só de Apps com canWrite, só ações que o usuário habilitou)

3  Hermes responde em texto, com um bloco estruturado:

   Posso criar isso no M-Finance. Água costuma variar de mês
   para mês, então deixei o valor como variável.

   ```mos-action
   { "action": "m-finance.recurrence.create",
     "args": { "name": "Água", "dueDay": 10,
               "defaultAmountCents": 20000,
               "isVariableAmount": true } }
   ```

4  M/OS parseia e VALIDA contra o esquema declarado pelo App.
   Argumento fora do esquema = proposta recusada, não corrigida.

5  Preview, porque mexe em dinheiro:

   ┌ M-FINANCE · CRIAR CONTA RECORRENTE ────────────┐
   │ Água · R$ 200,00 · vence dia 10                │
   │ valor variável                                  │
   │                          [Cancelar]  [Criar]   │
   └────────────────────────────────────────────────┘

6  Confirmado → Tool Gateway → adapter → M-Finance

7  Recibo explícito, com Undo enquanto ele existir.
```

O passo 3 mostra por que isto vale a pena: o agente inferiu `isVariableAmount` porque conta
de água varia. O campo existe no schema do M-Finance (`recurrence_rules`), e o preview põe
a inferência na tela para você corrigir — que é `UX-PRINCIPLES` §19, a IA explicando o que
entendeu em vez de agir por conta.

---

## 4. O que falta construir, por camada

### 4.1 M/OS — Tool Gateway (`mos-core`)

Executa ações declaradas, sempre pelo mesmo caminho da UI. Nunca SQL próprio, nunca um
atalho. Lê `functions.rs` para saber quanta cerimônia cada ação pede:

> **Corrigido na implementação da fase 1.** A tabela abaixo dizia que risco baixo executava
> direto, e isso estava errado por confundir dois eixos.
>
> O risco classifica a **consequência da ação**. O preview responde a outra coisa: a
> **incerteza da interpretação**. Quem clica "Criar Task" na interface escolheu aquilo;
> quem falou uma frase pode ter sido mal entendido — e `UX-PRINCIPLES` §19 pede que o
> sistema mostre o que compreendeu antes de agir.
>
> Então **toda proposta mostra preview**, inclusive as de risco baixo. O risco decide o
> **peso** da confirmação, não a existência dela.

| risco | comportamento |
|---|---|
| low | preview com um botão; Undo depois |
| medium | preview com destaque; Undo depois |
| high | preview, confirmação inequívoca, borda em `--danger`; sem prometer Undo |

**Toda ação sobre dinheiro é `high`.** Não é negociável nesta spec.

### 4.2 Cada App — catálogo de ações

Um arquivo declarativo por App, com o nome da ação, o esquema dos argumentos e o risco.
O M/OS não aprende o que é uma conta: ele aprende que existe
`m-finance.recurrence.create` e qual a forma dos argumentos. **A regra de domínio continua
dentro do App.**

### 4.3 M-Finance — a peça que **passou a existir**

> **Corrigido em 2026-08-16.** O texto abaixo dizia que esta era a peça que não existia.
> Deixou de ser verdade, e por um caminho que ninguém planejou: o M-Finance ganhou um
> agente de WhatsApp — 14 commits que só chegaram ao monorepo quando os repositórios
> originais foram arquivados — e com ele um executor de ações completo.
>
> O que existe hoje em `lib/whatsapp/`, 2.605 linhas:
>
> | Arquivo | O que resolve |
> |---|---|
> | `action-executor.ts` | executa ações com payload validado por Zod |
> | `pending-intents.ts` | proposta pendente, confirmação, expiração |
> | `audit.ts` | registro do que foi executado |
> | `auth.ts` | autenticação por segredo, máquina a máquina |
>
> E o esquema de `create_bill` é `{ amountCents, description, dueDay, isRecurring }` —
> exatamente a conta de água que motivou esta spec.
>
> Vale notar o que aconteceu: **os dois lados chegaram sozinhos à mesma arquitetura.**
> Propor, deixar pendente, confirmar, executar, registrar. Não foi coordenação; foi o
> mesmo problema empurrando para a mesma forma.
>
> **Consequência para a fase 3:** ela deixa de ser "construir uma Action API" e vira
> "expor o executor que já existe num endpoint que o M/OS chame". O trabalho restante é
> a rota HTTP autenticada, o adapter do lado do M/OS e a credencial — não o domínio.

O texto original, mantido porque a regra que ele enuncia continua valendo:

Precisa ganhar uma **Action API**: endpoint estreito, versionado, autenticado por máquina,
que chama as mesmas funções que a UI dele chama. O precedente de autenticação máquina a
máquina já está lá, em `app/api/cron/reminders`.

Nada de o M/OS falar com o Postgres do M-Finance direto. Seria mais rápido e quebraria a
única regra que sustenta o resto: **toda escrita atravessa as regras de aplicação do dono
do dado.**

### 4.4 Credencial

O token que autoriza o M/OS a escrever no M-Finance mora no **Windows Credential Manager**,
como a credencial do Hermes (`ARCHITECTURE.md` §15.4, e o precedente em `mos-hermes/auth.rs`).
Nunca no renderer, nunca em `.env`, nunca no prompt.

---

## 5. O risco que importa nomear

O perigo desta feature não é a fiação. É que **uma frase em linguagem natural vira um
lançamento financeiro.**

As proteções, em ordem de importância:

1. o modelo só propõe — ele nunca tem credencial nem executa;
2. a proposta é validada contra um esquema declarado, e recusada se não bater;
3. dinheiro é risco alto: preview sempre, confirmação sempre, nunca silencioso;
4. o App é o dono da regra — o M/OS não sabe validar uma conta, e não tenta;
5. tudo que a ação fez fica registrado na conversa, com o resultado explícito.

E uma que vale dizer em voz alta: **o catálogo de ações desce no prompt, então ele sai da
máquina.** Nomes de ação e formas de argumento vão para a VPS. Não são dados pessoais, mas
são um mapa do que o sistema sabe fazer — e isso entra no registro da ADR-027 como o resto.

---

## 6. Ordem sugerida

| Fase | O que | Por que primeiro |
|---|---|---|
| 1 ✔ | Tool Gateway + catálogo, com ações **locais** do M/OS (criar Capture, criar Task) | prova a cadeia inteira sem tocar em dinheiro nem em rede |
| 2 ✔ | Preview, recibo e Undo | a cerimônia precisa estar pronta antes de a primeira ação de risco existir |
| 3 | Action API no M-Finance + adapter | o buraco real, e o que exige mais cuidado |
| 4 | CronoCAD | ele vai estar absorvido (ADR-032 fase 3), então é ação local |
| 5 | Coded Atlas | gerar asset é ação longa; pede o modelo de execução em background |

A fase 1 é deliberadamente sem graça: criar uma Task pelo Hermes não impressiona ninguém.
Mas é ela que prova preview, confirmação, recibo e Undo com risco baixo — e é exatamente o
que você quer ter testado antes de a primeira frase virar uma conta a pagar.

> **Fases 1 e 2 entregues.** A cerimônia está completa e exercitada com risco baixo:
> proposta, preview, confirmação, execução pelos serviços da aplicação, recibo e desfazer.
>
> Uma decisão apareceu só na implementação e virou ADR-035: **desfazer arquiva, nunca
> apaga.** A exclusão definitiva recusa o que ainda está ativo, de propósito, e todo Undo
> que o M/OS já oferecia é restauração de estado. Apagar aqui exigiria contornar uma guarda
> escrita justamente para impedir apagamento acidental — e ofereceria o único caminho sem
> volta do app no momento em que o usuário acabou de dizer que errou.
>
> A fase 3 herda isso pronto. É o que ela precisava: quando a primeira ação mexer em
> dinheiro, a cerimônia não estará sendo inventada junto com o risco.
