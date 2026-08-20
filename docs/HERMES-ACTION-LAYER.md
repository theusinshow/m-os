# M/OS Action Layer — o Hermes como agente operacional

**Status:** implementado
**Data:** 2026-08-20
**Decisão:** ADR-051. Antecedentes: ADR-024, ADR-027, ADR-028, `SPEC-ACOES-ENTRE-APPS.md`.

---

## 1. O que mudou, em uma frase

O Hermes deixou de receber uma frase solta e passou a receber **a frase mais o
estado do M/OS que ela cita** — porque o M/OS pesquisa a própria base antes de
enviar.

Antes:

```
usuário → frase → VPS → texto de volta
```

Agora:

```
usuário
   ↓
frase
   ↓
M/OS: quem sou eu · que horas são · onde você está
      · o que eu já sei fazer · o que eu ACABEI DE ENCONTRAR
   ↓
VPS
   ↓
proposta com id
   ↓
M/OS: resolve → preview → confirma → executa → recibo → undo
```

---

## 2. O caso que definiu o desenho

> *"Criar lembrete para hoje de noite às 20:30 para enviar tipos de bases
> faltantes para o Victor, task já cadastrada no kanban."*

Quatro coisas faltavam, e nenhuma delas era do modelo:

| Faltava | Onde entrou |
|---|---|
| "estou dentro do M/OS" | `agent::system_context` |
| que dia é hoje, em que fuso | `agent::now_block`, alimentado por `surface.rs` |
| que a Task existe | `agent::candidates_block`, alimentado por `jarvis::candidates_for` |
| uma ação que agende | `ActionKind::ReminderCreate` |

A quarta é a que explica o sintoma. Com nove ações e nenhuma delas capaz de
agendar, a única coisa que o modelo podia propor sobre aquela frase era
`mos.task.create` — **a duplicata exata que o pedido proibia**. Ele não estava
sendo obtuso; estava usando o único martelo que tinha.

---

## 3. As peças, e onde cada uma mora

| Peça | Arquivo | O que faz |
|---|---|---|
| Identidade, tempo, lugar, candidatos | `crates/mos-core/src/agent.rs` | monta os blocos do preâmbulo; puro, sem I/O |
| Resolução de referência | `agent::resolve` | id → prefixo → título exato → começo → pedaço |
| Termos de busca | `agent::search_terms` | tira conectivo, verbo de comando e vocabulário do M/OS |
| Catálogo e validação | `crates/mos-core/src/action.rs` | 13 ações, preview, undo, auditoria |
| Busca na base | `apps/desktop/src-tauri/src/jarvis.rs` | varredura por termo, ranking, execução |
| Contexto da tela | `apps/desktop/src-tauri/src/surface.rs` | tela aberta, Project, Task, Workspace, fuso |
| Ponte e salto de busca | `apps/desktop/src-tauri/src/hermes.rs` | preâmbulo, registro do que sai, `mos-query` |

Nada em `mos-core` lê banco ou abre rede. É a mesma disciplina do `action.rs`, e
pelo mesmo motivo: o core não conhece SQLite nem o Tauri.

---

## 4. O preâmbulo, na ordem em que é lido

```
[Quem você é]         identidade operacional; "kanban" é o Kanban do M/OS
[Agora]               quinta-feira, 20 de agosto de 2026, 14:32 (UTC-03:00)
[Onde o usuário está] Tela aberta: Kanban · Project aberto: Minarum (id 1f4c9a2b)
[Ações disponíveis]   o catálogo, uma linha por ação
[Buscar mais no M/OS] só enquanto ainda há salto disponível
[Entidades encontradas] - task 7c3e2b19 · Enviar tipos de bases… · doing · Minarum
[Contexto anexado]    o que o usuário anexou com @
<a pergunta>
```

A ordem não é estética. Identidade primeiro porque reenquadra tudo o que vem
depois; tempo e lugar em seguida porque são o que resolve pronome e data; o
catálogo depois, porque só faz sentido quando já se sabe sobre o que agir; e os
candidatos por último, colados na pergunta, porque são o dado mais específico e
o mais fácil de perder no meio.

**Este bloco desce em toda mensagem.** É o maior custo fixo de token do chat, e
é por isso que a assinatura de cada ação cabe em uma linha.

---

## 5. A busca automática

`search_terms` quebra a frase e joga fora três famílias:

- conectivos e artigos;
- **o vocabulário do próprio M/OS** — "task", "kanban", "lembrete", "projeto".
  Procurar por "task" numa base em que tudo é task devolve tudo, e procurar por
  "lembrete" numa frase que *pede* um lembrete devolve os lembretes antigos em
  vez da coisa lembrada;
- verbos de comando e palavras de tempo, que dizem o que fazer e quando, nunca
  qual.

Da frase do Victor sobram `victor`, `bases`, `faltantes`, `enviar`, `tipos`.

Cada termo vira **uma varredura separada**. O FTS do M/OS junta termos com
`AND` (`to_fts_query`), o que é certo para a caixa de busca e errado aqui:
exigir que uma Task contenha os cinco ao mesmo tempo não acharia
`"Enviar tipos de bases faltantes p/ Victor"` — falta um "para", sobra um "p/".
Buscas separadas dão semântica de OU, e **quantos termos bateram** vira o
ranking.

O tokenizador preserva hífen e barra dentro da palavra, ao contrário do
`voice_when`: `063-26` é um código de projeto, e parti-lo procuraria por dois
números que aparecem em qualquer lugar.

Uma frase que só tem ruído — "cria uma task" — não produz busca nenhuma. Sem
essa guarda, ela viraria uma varredura por nada e traria os doze primeiros itens
da base como se fossem candidatos.

---

## 6. Resolução em degraus

`agent::resolve` compara em cinco degraus, e **o primeiro que acerta decide**:

1. id inteiro
2. prefixo de id, com no mínimo seis dígitos — é o que o bloco de candidatos mostra
3. título exato, sem acento e sem caixa
4. começo do título
5. pedaço do título

Só quando o degrau que acertou devolve mais de um é que existe dúvida de
verdade. Aí o M/OS **pergunta e nomeia os candidatos** — "bate com 2 Tasks:
'Enviar tipos de bases…', 'Enviar proposta…'. Diga qual." —, em vez de escolher
o primeiro.

Isso é a §8 do pedido em forma de código: perguntar é exceção, não fluxo padrão.
Mas continua existindo, porque agir sobre a Task errada é pior que perguntar.

---

## 7. O salto de busca

Quando a busca automática não basta, o modelo escreve:

````
```mos-query
{ "search": "victor bases", "kinds": ["task"] }
```
````

O M/OS lê, executa na base local e devolve o resultado como um novo
`prompt.submit` na mesma sessão. **Um salto por pergunta**, e o contrato some do
preâmbulo quando o orçamento acaba — oferecer uma ferramenta que não vai ser
executada ensinaria o modelo a pedi-la e receber silêncio.

A busca aparece na thread como um passo (`ToolRun`), porque a pausa entre a
pergunta e a resposta, sem isso, parece travamento.

Quando a resposta traz proposta **e** busca, a proposta ganha: se o modelo já
sabe o que propor, procurar mais seria gastar um turno para confirmar o que ele
acabou de afirmar.

---

## 8. O catálogo hoje

```
mos.capture.create      { content }
mos.capture.to_task     { capture, title?, project? }
mos.task.create         { title, description?, project? }
mos.task.set_state      { task, state: inbox|backlog|planned|doing|review|done }
mos.task.set_project    { task, project }
mos.project.create      { name, description? }
mos.resource.create     { kind, title, url?, note? }
mos.reminder.create     { title, at?, when?, body?, taskRef?, projectRef?, captureRef? }
mos.reminder.resolve    { reminder, state: done|cancelled }
mos.time.start          { project, activity?, description? }
mos.time.stop           { }
mos.time.record         { project, minutes, day?, activity?, description? }
m-finance.create_bill   { amountCents, description, dueDay?, isRecurring }
```

Todo campo que aponta para uma entidade existente aceita **id ou título**, e o
contrato ensina a preferir o id.

`at` e `when` são alternativos e existem por motivos diferentes: `at` é para
quando o modelo consegue fazer a conta; `when` é para quando ele não consegue, e
aí quem resolve a frase é o M/OS, com o mesmo leitor de datas faladas que a voz
usa. Duas gramáticas de "sexta que vem" no mesmo app dariam duas sextas.

**O instante é resolvido na leitura da proposta, e não na execução**, por causa
do preview: o cartão precisa dizer *"quinta-feira, 20 de agosto, 20:30"* para o
usuário conferir antes de autorizar. Ao lado dele aparece a frase original —
ver as duas juntas é o que permite pegar "sexta" entendida como a sexta errada.

---

## 9. Relações

`ReminderTarget` é um enum de um braço só, por decisão registrada: a ADR-012
recusou tabela genérica de arestas e o preço aceito foi este. A proposta, porém,
chega quase sempre com dois — "lembrete da task X, do projeto Y".

A saída é a **especificidade**: entre Task e Project, o vínculo útil é a Task,
porque dela se chega ao Project e do Project não se chega à Task. A ordem é
`task > capture > resource > meeting > project`, e o preview **nomeia o alvo
escolhido** — escolher em silêncio seria adivinhar.

---

## 10. Auditabilidade

O §11 do pedido lista sete coisas. Seis já existiam na conversa: a proposta
guarda a ação crua, a mensagem guarda o instante e o id da conversa, e
`source = hermes` é o próprio fato de a parte ser uma proposta.

Faltavam **a entidade resolvida e o estado anterior**, e os dois entraram em
`ActionAudit`, dentro da própria parte. Uma tabela nova custaria migration e uma
segunda fonte de verdade sobre o que o Hermes fez; as partes já são persistidas
como JSON (ADR-025), então o campo entrou sem tocar no esquema — e é `Option`
com `serde(default)`, porque toda proposta gravada antes de hoje continua
legível.

`ReminderSource::Hermes` existia no domínio desde o P0 do Attention System e
nunca tinha sido escrito por ninguém. Agora é.

---

## 11. O que continua valendo, e não mudou

- **O modelo nunca executa nada.** Ele propõe; o M/OS valida, mostra o preview,
  espera a confirmação e executa pelos mesmos serviços que a interface usa.
- **Toda proposta mostra preview**, inclusive as de risco baixo. O risco decide
  o peso da confirmação, não a existência dela (`SPEC-ACOES-ENTRE-APPS` §4.1).
- **Desfazer arquiva, nunca apaga** (ADR-035). O lembrete criado se desfaz
  cancelando; a Capture convertida volta para a Inbox, porque o que se desfez
  foi a decisão sobre ela.
- **Nada sai sem registro** (ADR-027). A busca automática vira uma parte
  `context_ref` de origem automática, com os nomes do que foi em `fields`.

---

## 12. O que ficou de fora, e por quê

| Fora | Motivo |
|---|---|
| People / Knowledge Graph | não existe entidade Pessoa no domínio. "Victor" é texto dentro do título de uma Task, e inventar a entidade aqui seria criar domínio a partir de uma frase de prompt |
| Waiting for | mesmo motivo: não há o conceito no `CORE.md` |
| `mos.resource.link_project` | o serviço de Resource liga a Workspace, não a Project |
| Mais de um salto de busca | cada salto roda o turno inteiro de novo; dois seriam o agente tateando enquanto o usuário espera |
| MCP local | continua adiado pela ADR-028, e continua exigindo ADR própria |

As três primeiras são ausências do **domínio**, e não da camada de ação. Quando
`Person` existir, ela entra aqui com um `EntityKind` e uma linha em cada `match`
— que é exatamente o custo que a ADR-012 aceitou pagar.
