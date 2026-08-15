# Hermes Premium Chat — auditoria e plano de evolução

**Status:** auditoria concluída · **P0 implementado** · P1–P5 aguardando aprovação
**Data:** 2026-08-15
**Escopo:** evolução da superfície Hermes para AI Workspace do M/OS

> **Nota de leitura.** As seções 1 a 10 descrevem o estado encontrado na auditoria, e são
> mantidas como registro do que havia. O que mudou desde então está em §11 (roadmap, com o
> P0 marcado) e no resumo abaixo. Onde a implementação contradisse a auditoria, o texto diz
> isso em vez de ser reescrito para parecer que sempre soube.

## Estado da implementação

P0 foi implementado e verificado. As decisões que ele exigiu estão registradas como
ADR-025 a ADR-030 em `DECISIONS.md`, e o desenho seguido é
`M-OS Hermes - Design Direction v1` (ADR-030).

| Camada | O que mudou |
|---|---|
| `mos-hermes` | `clarify.request` e `sudo.request` tratados; `status.update`, `session.history` e `session.title` deixam de ser descartados |
| `mos-core` | módulo `conversation` — Conversation, Message, MessagePart, ConversationService |
| `mos-storage-sqlite` | migration 0010; busca por trigger, só sobre partes de texto |
| `src-tauri` | `jarvis.rs` — orquestra ponte, conversa e contexto; grava por mensagem, nunca por delta |
| renderer | superfície única na direção Marginália; Markdown próprio; ações por mensagem; chips com registro do que foi enviado |

**O único item de P0 deliberadamente adiado é o Capability Service** — ver §11.

---

**As fases seguintes não entram em código antes de aprovação explícita.**

Documentos-mãe: `VISION.md`, `PRODUCT.md`, `UX-PRINCIPLES.md`, `DESIGN-FOUNDATIONS.md`,
`ARCHITECTURE.md`, `DECISIONS.md` (ADR-024), `HERMES-GATEWAY-CONTRACT.md`,
`superpowers/specs/2026-08-13-mos-hermes-bridge-design.md`.

Este documento registra **o que foi lido no código**, não o que foi suposto. Onde algo não
pôde ser verificado, está marcado como aberto. Onde a verificação contradisse a suposição
anterior, o texto diz isso.

---

## 1. Current State

### 1.1 O que existe, e é bom

A ponte com o Hermes é a parte mais bem construída do M/OS hoje. Ela não deve ser
reescrita — deve ser preservada e cercada.

| Camada | Arquivo | Linhas | Avaliação |
|---|---|---:|---|
| Protocolo | `crates/mos-hermes/src/protocol.rs` | 291 | Sólido. Vocabulário verificado contra o gateway, `Unknown` preserva o nome do tipo divergente, testes com frames reais. |
| Transporte | `crates/mos-hermes/src/transport.rs` | 158 | Sólido. Trata lote newline-delimited, frame binário, e traduz códigos de fechamento `4401`/`4403` em causas distintas. |
| Auth | `crates/mos-hermes/src/auth.rs` | 274 | Sólido. Cookie jar persistente, ticket efêmero cunhado por socket, `429` sem retry. |
| Ponte | `crates/mos-hermes/src/bridge.rs` | 501 | Sólido. Máquina de estados correta, `gateway.ready` como único promotor para Online, fallback de `session.resume` rejeitado. |
| Fronteira Tauri | `apps/desktop/src-tauri/src/hermes.rs` | 312 | Boa. Task dedicada, ordens por canal, anúncio de estado só quando muda. |
| Fronteira renderer | `apps/desktop/src/hermes.ts` | 101 | Boa. Falha tipada com `kind` e `retriable`; o renderer nunca vê credencial. |
| Supervisor de reconexão | `App.tsx:1547-1606` | 60 | Boa. Backoff exponencial com teto de 5 min, trava definitiva para falha não-retriável. |

Cinco decisões estruturais que devem sobreviver a qualquer evolução:

1. **`mos-hermes` não declara `mos-storage-sqlite`.** "Hermes nunca escreve no SQLite" é
   impossibilidade de compilação, não regra a lembrar (ADR-024).
2. **Credencial só existe no Rust.** O renderer aprende que existe credencial, nunca qual é.
3. **Streaming não é rebufferizado.** O servidor desliga Nagle de propósito; a ponte respeita.
4. **`approval.request` é respondido, com `deny` por omissão.** Fechar sem escolher nega.
5. **Ausência declarada em vez de controle que finge.** Os modos `ACT` e `ORGANIZE` estão
   visivelmente desabilitados com o motivo no `title`. Isso é caro de manter e é o
   comportamento certo — deve ser mantido como regra, não como exceção.

### 1.2 O que existe, e é frágil

A camada de conversa acima da ponte é fina, e não sustenta o que se quer construir.

**A conversa é um triplo de strings.**

```ts
type Turn = { question: string; answer: string; reasoning: string };
```

`HermesPage.tsx:20` e `App.tsx:1086`. Este tipo não consegue representar tool call,
citação, anexo, imagem, artifact, status ou erro posicionado. Toda funcionalidade pedida
para o AI Workspace esbarra nele primeiro.

**Não há renderização de Markdown.** A resposta é `<p>{turn.answer}</p>`
(`HermesPage.tsx:221`). Bloco de código, tabela, lista e heading chegam como texto cru com
os asteriscos e as crases visíveis. `package.json` não tem nenhuma dependência de markdown,
highlight ou virtualização. Este é o maior buraco isolado entre o estado atual e "premium".

**Existem duas superfícies de chat, e elas colidem.** `HermesPage` (tela cheia,
`App.tsx:1667`) e `CommandSurface` em modo Hermes (`App.tsx:1242`) são implementações
independentes da mesma conversa. As duas assinam `hermes.onEvent` globalmente
(`HermesPage.tsx:82` e `App.tsx:1128`), e `CommandSurface` monta **por cima** da página
(`App.tsx:1698`). Com a página Hermes aberta e `Ctrl+K` acionado, os dois componentes
recebem os mesmos deltas e os anexam a listas de turnos separadas. A pergunta feita no
Command aparece parcialmente na página, e vice-versa. Isto é um defeito, não uma
duplicação estética.

**O `session_id` nunca é persistido em disco.** Ele vive em `HermesState`
(`hermes.rs:48`), um `Mutex` de memória de processo. O comentário no campo diz
*"guardado localmente para `session.resume` na próxima abertura"* — mas nada o escreve.
Na prática: `session.resume` funciona para reconexão dentro da mesma execução, e **nunca**
entre reinícios do app. Toda abertura do M/OS cria uma sessão nova na VPS. O mecanismo de
resume em `bridge.rs:94-111`, com teste próprio, está correto e efetivamente morto no
caminho para o qual foi escrito.

**Eventos de ferramenta são descartados.** `HermesPage.tsx:83-91` trata `delta`,
`reasoning`, `complete`, `approval` e `failed`. `Outcome::Tool` e `Outcome::UnknownFrame`
caem fora do `if` e somem. A ponte os produz corretamente; a UI não os usa.

**Nenhuma conversa é guardada.** O drawer declara isso com honestidade
(`HermesPage.tsx:291`): *"Esta conversa não é salva pelo M/OS. O histórico vive na sua
VPS"*. Consequência real: fechar a página perde a thread da tela, não existe lista de
conversas, não existe busca, e o histórico da VPS nunca é reidratado — apesar de
`session.history` e `session.list` existirem no gateway e estarem documentados no contrato
(`HERMES-GATEWAY-CONTRACT.md` §4).

### 1.3 Defeito vivo confirmado: `clarify.request` e `sudo.request` travam a conversa

Este é o achado mais sério da auditoria, e foi verificado no código do Hermes, não inferido.

A Spec B identificou corretamente que `approval.request` é um **evento de entrada** e que
ignorá-lo trava a conversa em silêncio. Existem mais dois eventos da mesma classe, e o
M/OS não trata nenhum dos dois:

| Evento | Origem | Timeout | Tratado pelo M/OS |
|---|---|---:|---|
| `approval.request` | `server.py` | 300 s | Sim |
| `clarify.request` | `server.py:3018` | 300 s | **Não** |
| `sudo.request` | `server.py:3035` | 120 s | **Não** |

Os três passam pelo mesmo `_block()` (`server.py:1398-1413`), que emite o evento e
**bloqueia a thread do agente** com `ev.wait(timeout=...)` até o cliente responder.

Comportamento atual do M/OS quando uma skill chama clarify: o frame chega como
`Outcome::UnknownFrame`, `HermesPage` o descarta em silêncio, e a resposta congela por
até cinco minutos antes de o agente continuar com uma resposta vazia. Não há indicação
de causa na tela. O usuário vê "PENSANDO" e nada mais.

A saída de emergência existe e está certa: `session.interrupt` libera as pendências da
sessão (`_clear_pending`), então o botão Cancelar destrava. Mas ninguém sabe que precisa
apertá-lo.

**Isto é P0 e independe de qualquer feature nova.**

### 1.4 Contexto por `@` é decorativo

`HermesPage.tsx:128-153` faz busca real no acervo local e mostra sugestões reais — a
metade da mecânica está correta. Mas ao escolher um item, o que acontece é:

```ts
setDraft((current) => current.replace(/@([\wÀ-ú]*)$/, `@${name} `));
```

Substitui-se o texto pelo **nome** do Project. Nenhum dado estruturado acompanha o prompt.
O Hermes recebe a string `@M/OS o que falta aqui?` e não tem como saber o que é `@M/OS`,
quais Tasks existem ou qual é o estado. A lista `cited` no drawer registra o que foi
mencionado, mas é um registro local — ela não influencia o que sai pelo socket.

O usuário pediu explicitamente para evitar mocks que pareçam funcionalidades reais. Este é
o caso mais próximo disso no código atual, e é involuntário: a intenção era clara e a
metade que falta é a que atravessa a ponte.

---

## 2. Architecture

### 2.1 Como está

```text
HermesPage.tsx            CommandSurface (App.tsx)
   │ (assina hermes-event)    │ (assina hermes-event)
   └───────────┬──────────────┘        ← duas superfícies, um só barramento
               ↓ invoke / listen
       src-tauri/src/hermes.rs         ← comandos + estado de sessão em memória
               ↓
       mos-hermes::Bridge              ← sessão, correlação, máquina de estados
               ↓
       transport (WebSocket)
               ↓
       127.0.0.1:9119 → túnel SSH → Hermes VPS
```

Não existe camada de conversa. `hermes.rs` é uma fronteira de comandos, não um serviço de
aplicação: ele encaminha ordens e reemite eventos, sem modelo de mensagem, sem
persistência e sem contexto.

### 2.2 Como deve ficar

A arquitetura pedida é correta e cabe no que já existe, com uma condição inegociável:
**`mos-hermes` continua sem conhecer SQLite.** A persistência da conversa é do `mos-core`,
e o orquestrador do desktop é o único lugar onde os dois se encontram.

```text
Hermes UI (superfície única, uma assinatura)
        │ comandos tipados / eventos
        ▼
Jarvis Application Layer            src-tauri/src/jarvis.rs
        │                           orquestra; não persiste e não fala rede
        ├──────────────┬───────────────────┬──────────────────┐
        ▼              ▼                   ▼                  ▼
Conversation     Context           Capability          Hermes Bridge
Service          Service           Service             (mos-hermes)
mos-core         mos-core          mos-core                  │
        │              │                   │                  ▼
        ▼              ▼                   │           Hermes VPS
mos-storage-     repositories              │
sqlite           (read-only)               └── tools.list / toolsets.list
                                               commands.catalog / /api/status
```

Regras da fronteira, herdadas de `ARCHITECTURE.md` §9 e ADR-024:

- **React nunca escreve SQLite.** Já é verdade; continua.
- **Hermes nunca escreve SQLite.** Garantido pelo Cargo.toml; continua.
- **`mos-hermes` não ganha dependência de `mos-core`.** A tradução entre `Outcome` e
  `MessagePart` acontece no orquestrador, não dentro da ponte. Isso mantém a ponte
  testável sem domínio e o domínio testável sem rede.
- **Toda ação do Hermes no M/OS atravessa os mesmos serviços que a UI usa.** Nunca um
  caminho paralelo. Ver §6.4.

### 2.3 O que muda em cada arquivo

| Arquivo | Mudança | Fase |
|---|---|---|
| `crates/mos-hermes/*` | Novos `Outcome` para os eventos hoje perdidos (`Clarify`, `Sudo`, `ToolDelta`, `Title`, `Usage`). Nada mais. | P0 |
| `crates/mos-core/src/conversation.rs` | Novo. Entidades e serviço de conversa. | P0 |
| `crates/mos-storage-sqlite` | Migration das tabelas de conversa. | P0 |
| `src-tauri/src/hermes.rs` | Vira `jarvis.rs`: orquestra ponte + conversa + contexto. | P0 |
| `apps/desktop/src/HermesPage.tsx` | Uma superfície, modelo de partes, renderer de Markdown. | P0 |
| `apps/desktop/src/App.tsx` | `CommandSurface` deixa de ter modo Hermes próprio. | P0 |

---

## 3. Missing capabilities

Ordenado por custo de não ter. Cada linha diz se o bloqueio é do M/OS ou do gateway.

| # | Ausente | Bloqueio | Impacto |
|---|---|---|---|
| 1 | Tratamento de `clarify.request` / `sudo.request` | M/OS | Conversa congela até 5 min, sem causa na tela |
| 2 | Superfície única de chat | M/OS | Deltas duplicados entre página e Command |
| 3 | Modelo de mensagem com partes | M/OS | Bloqueia tool, citação, anexo, artifact — tudo |
| 4 | Persistência de conversa | M/OS | Sem histórico, lista, busca, rename, branch |
| 5 | Renderização de Markdown e código | M/OS | Resposta técnica ilegível |
| 6 | `session_id` em disco | M/OS | Sessão nova a cada abertura; resume morto |
| 7 | Stop / retry / regenerate / edit | M/OS | Só existe Cancelar |
| 8 | Descoberta de capabilities | M/OS | UI adivinha o que o gateway faz |
| 9 | Contexto estruturado do M/OS no prompt | M/OS | O `@` não leva dado nenhum |
| 10 | Render de tool run | M/OS | Eventos já chegam e são jogados fora |
| 11 | Reidratação via `session.history` | M/OS | Histórico existe na VPS e nunca é lido |
| 12 | Título automático (`session.title`) | M/OS | Conversas sem nome |
| 13 | Anexos (`file.attach`, `image.attach`, `pdf.attach`) | M/OS | Métodos existem e não são chamados |
| 14 | Steering (`session.steer`) | M/OS | Redirecionar turno em andamento |
| 15 | Uso de tokens (`session.usage`) | M/OS | Sem noção de custo |
| 16 | Citations estruturadas | **Gateway** | Ver §6.3 — não confirmado |
| 17 | Artifact workspace | M/OS | Conceito não existe em lugar nenhum |
| 18 | Memory separada de histórico | M/OS + Gateway | Ver §6.3 |
| 19 | Tools do M/OS (`mos_search` etc.) | **Gate aberto** | Ver §6.4 |

Nota importante sobre a coluna Bloqueio: **quase tudo é bloqueio do M/OS, não do Hermes.**
O gateway expõe muito mais do que o M/OS consome. Isso é uma boa notícia para o plano: a
maior parte do trabalho é local e não depende de mudar a VPS.

---

## 4. Feature inventory

Legenda: **✓** existe e funciona · **~** existe parcial ou decorativo · **✗** não existe ·
**⊘** ausência declarada na UI (honesto).

### 4.1 Messaging

| Feature | Hoje | Fase | Nota |
|---|:--:|:--:|---|
| Streaming token a token | ✓ | — | Sem rebuffer, correto |
| Stop generation | ✓ | — | `session.interrupt` |
| Retry em erro | ✗ | P0 | Hoje o erro vira texto na resposta |
| Regenerate response | ✗ | P0 | |
| Editar mensagem do usuário | ✗ | P0 | Exige modelo de mensagem |
| Reenviar mensagem editada | ✗ | P0 | |
| Branch a partir de mensagem | ✗ | P2 | Exige árvore de conversa |
| Continue generation | ✗ | P1 | Verificar suporte no gateway |
| Copy resposta | ✗ | P0 | |
| Selecionar texto | ✓ | — | É `<p>`, seleciona |
| Timestamps discretos | ✗ | P0 | |
| Feedback ±  | ✗ | P3 | Sem destino para o sinal hoje |
| Fixar mensagem | ✗ | P2 | |
| Salvar resposta como Resource | ✗ | P3 | Atravessa `MemoryService` |

### 4.2 Rendering

| Feature | Hoje | Fase |
|---|:--:|:--:|
| Markdown, headings, listas, blockquote, links | ✗ | P0 |
| Tabelas | ✗ | P0 |
| Código com highlight | ✗ | P0 |
| Copy Code | ✗ | P0 |
| Inline code | ✗ | P0 |
| Matemática | ✗ | P2 |
| Imagens | ✗ | P1 |
| Arquivos | ✗ | P1 |
| Citations | ✗ | P2 |
| Tool results | ✗ | P0 (básico) |
| Status / progress | ~ | P0 |
| Virtualização | ✗ | P1 (ver §9) |

### 4.3 Composer

| Feature | Hoje | Fase | Nota |
|---|:--:|:--:|---|
| Multiline | ✗ | P0 | É `<input>`, não `<textarea>` — `Shift+Enter` é anunciado mas não funciona |
| Enter envia | ✓ | — | |
| Esc cancela | ~ | P0 | Fecha Command; sem papel na página |
| Drag and drop | ✗ | P1 | |
| Paste image / screenshot | ✗ | P1 | Gateway tem `image.attach_bytes`, `clipboard.paste` |
| Arquivos / PDF | ✗ | P1 | Gateway tem `file.attach`, `pdf.attach` |
| Áudio | ✗ | P5 | |
| `@Project` / `@Resource` | ~ | P0 | Busca real, contexto decorativo |
| `@Task` / `@Capture` | ✗ | P1 | |
| Slash commands | ✗ | P2 | Gateway tem `commands.catalog`, `slash.exec` |
| Skills explícitas | ✗ | P2 | Gateway tem `skills.manage` |
| Voice / dictation | ✗ | P5 | Gateway tem `voice.record`, `voice.transcript` |
| Message queue | ✗ | P1 | Hoje o campo bloqueia enquanto roda |

**Defeito de rodapé:** `HermesPage.tsx:254` anuncia `⇧↵ NOVA LINHA`, mas o campo é um
`<input>` e `Shift+Enter` não insere quebra. O rodapé promete o que a UI não faz.

### 4.4 Conversations

| Feature | Hoje | Fase |
|---|:--:|:--:|
| New chat | ✗ | P0 |
| Rename | ✗ | P0 |
| Título automático | ✗ | P0 |
| Search em conversas | ✗ | P0 |
| Pin | ✗ | P2 |
| Archive / Delete | ✗ | P0 |
| Recent | ✗ | P0 |
| Relação com Project | ✗ | P2 |
| Duplicate / Branch | ✗ | P2 |
| Temporary chat | ✗ | P2 |

### 4.5 Estado de conexão

| Estado | Hoje | Nota |
|---|:--:|---|
| Online / Connecting / Offline | ✓ | Modelado no Rust |
| `sessionReady` separado de Online | ✓ | Distinção correta e rara |
| Authentication failed | ✓ | `kind: unauthorized` |
| Rate limited | ✓ | Sem retry, correto |
| Server unavailable | ✓ | `kind: gateway` |
| Streaming interrompido | ✗ | **P0.** Socket cai no meio: o texto fica na tela, mas nada diz que acabou por queda, e não há Retry |
| M/OS funciona sem Hermes | ✓ | Requisito atendido |

Esta é a área mais madura da implementação atual. A única lacuna real é a interrupção
durante o streaming.

---

## 5. Data model

### 5.1 Avaliação do modelo proposto

O modelo sugerido (Conversation, Message, MessagePart, Attachment, Citation, Artifact,
ToolRun, AgentRun, ContextReference) está conceitualmente certo e **cedo demais**. Nove
entidades antes de existir uma única conversa persistida contradiz ADR-012 ("relações
explícitas até casos reais exigirem"), `ARCHITECTURE.md` §20 e a cultura do repositório,
que só criou `Resource` quando havia um caso concreto (ADR-021).

A parte que deve ser adotada imediatamente é a que custa caro depois: **`MessagePart`.**
É o discriminador que permite promover qualquer uma das outras entidades sem reescrever a
thread.

### 5.2 Modelo recomendado para P0

Três tabelas. Local, em `mos-core` + `mos-storage-sqlite`.

> **Implementado sem `pinned` e sem `temporary`.** Os dois pertencem a P2 (fixar conversa,
> chat temporário) e nenhum tem uso em P0. Uma coluna booleana que não faz nada é a mesma
> antecipação que ADR-012 proíbe, e acrescentá-la depois é uma migration de cinco linhas.

```text
Conversation
  id                TEXT PK
  title             TEXT              -- vazio até session.title responder
  hermes_session_id TEXT NULL         -- o vínculo com a VPS, agora em disco
  lifecycle_state   TEXT              -- active | archived | trashed
  created_at        TEXT
  updated_at        TEXT

Message
  id              TEXT PK
  conversation_id TEXT FK
  seq             INTEGER             -- ordem estável na conversa
  role            TEXT                -- user | assistant | system
  status          TEXT                -- pending | streaming | complete | interrupted | failed
  parent_id       TEXT NULL           -- reservado para branch (P2); NULL em P0
  created_at      TEXT

MessagePart
  id          TEXT PK
  message_id  TEXT FK
  seq         INTEGER
  kind        TEXT                    -- ver abaixo
  payload     TEXT                    -- JSON, validado por kind
```

`kind` em P0: `text` · `reasoning` · `tool_run` · `status` · `error`
`kind` depois: `citation` (P2) · `attachment` (P1) · `image` (P1) · `artifact_ref` (P2) ·
`context_ref` (P0, ver abaixo)

**Por que payload JSON e não coluna por tipo:** as formas de `tool_run` e `citation` ainda
não foram observadas de verdade — o M/OS nunca renderizou uma. Congelar colunas agora é
adivinhar. JSON validado por `kind` no domínio dá a mesma segurança com metade do custo de
migração, e a promoção para tabela própria é uma migration mecânica quando a forma
estabilizar.

**Quem vira tabela própria depois, e por quê:**

- `Attachment` — precisa de lifecycle próprio (o arquivo sobrevive à mensagem, e vai para
  a Library). **Vira tabela em P1.**
- `Artifact` — precisa de versão, edição e relação com Project. **Vira tabela em P2.**
- `Citation` — provavelmente nunca precisa: só é consultada junto da mensagem. Fica parte.
- `ToolRun` — vira tabela só se surgir consulta transversal ("todas as execuções de
  `mos_search` desta semana"). Não há caso hoje.
- `AgentRun` — só existe quando houver execução em background (P5).

### 5.3 `ContextReference` entra em P0, como parte

Esta é a exceção deliberada. O contexto anexado a uma mensagem precisa ser **persistido
junto da mensagem**, porque a pergunta "o que exatamente eu mandei para a VPS?" precisa ser
respondível depois — é requisito de segurança (§8), não conveniência.

```json
{ "kind": "context_ref",
  "payload": { "origin": "explicit" | "automatic",
               "entity": "project" | "task" | "capture" | "resource" | "screen" | "workspace",
               "id": "...", "label": "M/OS",
               "sent": { "fields": ["name","description","open_tasks"], "bytes": 412 } } }
```

`sent` é o que efetivamente atravessou o socket. Sem esse campo, o chip vira promessa não
auditável.

### 5.4 Search

Apenas partes `text` de mensagens entram no FTS5, na mesma transação da escrita, seguindo
a regra já estabelecida em `ARCHITECTURE.md` §11.2. Reasoning e tool payload ficam fora:
poluem resultado e crescem sem limite.

---

## 6. Hermes capability map

### 6.1 Princípio: descoberto, não declarado

A UI não deve conter uma lista fixa do que o Hermes sabe fazer. O gateway expõe os métodos
necessários para perguntar, e isso foi verificado no checkout local:

| Método | Verificado em | Responde |
|---|---|---|
| `GET /api/status` | contrato §3.0 | versão, auth, plataformas do gateway |
| `tools.list` | `server.py:10092` | ferramentas disponíveis |
| `toolsets.list` | `server.py` | conjuntos de ferramentas |
| `commands.catalog` | `server.py:8086` | slash commands |
| `model.options` | `server.py` | modelos disponíveis |

Três estados por capability, e a UI só habilita o primeiro:

- **`verified`** — método existe e respondeu na sessão atual;
- **`absent`** — perguntado, gateway não tem;
- **`unknown`** — não perguntado ainda.

`unknown` nunca habilita controle. Isto resolve diretamente o requisito de não quebrar o
chat quando uma feature não existe, e elimina a categoria de bug "funciona na minha VPS".

### 6.2 Capabilities confirmadas no gateway, não usadas pelo M/OS

Verificadas por leitura de `tui_gateway/server.py` no checkout `0.16.0`:

| Domínio | Métodos / eventos | M/OS usa |
|---|---|:--:|
| Sessão | `session.create` `resume` `close` `interrupt` `status` | ✓ |
| Sessão | `session.history` `list` `title` `save` `steer` `undo` `usage` | ✗ |
| Prompt | `prompt.submit` | ✓ |
| Entrada bloqueante | `approval.request` / `respond` | ✓ |
| Entrada bloqueante | `clarify.request` / `respond` | ✗ **defeito** |
| Entrada bloqueante | `sudo.request` / `respond` | ✗ **defeito** |
| Anexos | `file.attach` `image.attach` `image.attach_bytes` `image.detach` `pdf.attach` `input.pdf` `clipboard.paste` `input.detect_drop` | ✗ |
| Ferramentas | `tool.start` `generating` `complete` `started` | parcial (perdido na UI) |
| Ferramentas | `tools.list` `show` `configure` `toolsets.list` | ✗ |
| Skills | `skills.manage` `skills.reload` | ✗ |
| Comandos | `commands.catalog` `complete.slash` `slash.exec` | ✗ |
| Subagentes | `subagent.start` `text` `thinking` `tool` `complete` `interrupt` | ✗ |
| Voz | `voice.record` `toggle` `transcript` `tts` `status` | ✗ |
| Agendamento | `cron.manage` | ✗ |
| Navegador | `browser.manage` `browser.progress` | ✗ |
| Agente | `agent.reasoning_effort` `system_prompt` `service_tier` | ✗ (deliberado, ADR-024) |
| Raciocínio | `reasoning.delta` `thinking.delta` | ✓ |

O diretório `skills/` do checkout contém `research`, `github`, `email`, `data-science`,
`software-development`, `productivity`, `note-taking` e outros — o que significa que
**Deep Research não precisa ser construído do zero**, e sim exposto e observado. A forma
exata dos eventos dessa skill não foi verificada e é gate de P2.

### 6.3 Aberto — precisa de verificação antes de virar plano

1. **Citations estruturadas.** Não foi encontrado evento de citação no gateway. Se a skill
   de research devolve fontes dentro do texto Markdown, a arquitetura de citation do M/OS
   precisa extrair de texto, não receber estrutura. **Isto muda o desenho.** Gate de P2.
2. **Memory.** Nada indica um store de memória do agente exposto pelo gateway. Memory do
   M/OS provavelmente é local e do M/OS, não do Hermes. Gate de P5.
3. **Geração de imagem.** Não verificado.
4. **`session.steer` durante turno.** Método existe; comportamento não observado.

### 6.4 Gate crítico: como o Hermes lê o M/OS

Este é o gate arquitetural mais importante do documento, e ele **não pode ser decidido sem
verificação**. Não há, no protocolo WebSocket, registro de tool do lado do cliente. Existem
três caminhos possíveis e eles não são equivalentes:

| Caminho | Como funciona | Custo | Risco |
|---|---|---|---|
| **A. Injeção de contexto** | O M/OS monta um bloco estruturado e prefixa ao prompt | Baixo. Só código local | O agente não pode pedir mais dados; contexto é fixo no envio |
| **B. MCP server local** | O M/OS expõe um MCP server; o Hermes o consome como tool | Médio | Exige que a VPS alcance a máquina (túnel reverso) e que o M/OS rode um servidor — muda o threat model |
| **C. Extensão do gateway** | Tool do lado do cliente sobre o WS existente | Alto | Fork ou upstream do Hermes |

O checkout tem `mcp_serve.py` e um diretório `optional-mcps/`, então **B é plausível** —
mas a direção do túnel hoje é M/OS → VPS, e B exige o inverso.

**Recomendação:** P3 começa por **A**, que entrega "Jarvis conhece meu M/OS" sem mudar
topologia de rede nem threat model, e responde a maioria das perguntas reais ("o que falta
aqui?"). **B** só entra depois de uma ADR própria, porque expor um servidor local à VPS é
uma mudança de superfície de ataque que `ARCHITECTURE.md` §15.2 não cobre.

Não incluir B no plano antes dessa ADR.

---

## 7. UX surfaces

### 7.1 Uma superfície, não duas

A duplicação atual precisa acabar, e a divisão certa segue `UX-PRINCIPLES.md` §17 e §18
("Hermes complementa, não sequestra" / "camada, não destino"):

| Superfície | Intenção dominante | O que faz |
|---|---|---|
| **Página Hermes** | Conversar e trabalhar com o Hermes | Thread completa, artifact, conversas, contexto |
| **Command (`Ctrl+K`)** | Encontrar e executar | **Perde o modo Hermes.** Ganha "perguntar ao Hermes" como ação que abre a página já com a pergunta enviada |

Isso remove o defeito de assinatura dupla, elimina a segunda implementação de thread, e
respeita §13 ("uma tela, uma intenção dominante"). O Command continua sendo o caminho
rápido — ele só deixa de tentar ser um chat pequeno dentro de um overlay.

### 7.2 Layout da página

> **Superado por `M-OS Hermes - Design Direction v1` (ADR-030), que chegou depois desta
> auditoria.** O esboço abaixo fica como registro; o que foi construído é a direção
> **Marginália**, e ela é melhor pelo motivo que a auditoria não tinha: a separação não é
> de colunas, é de papéis.

```text
RAIL 52px   CONVERSAS 260px   THREAD minmax(0,1fr)      INSPECTOR 380px
navegação   histórico         ┌──────────────────┐      fontes · contexto
            busca             │ gutter │ prosa   │      memória · artifact
            nova conversa     │ 108px  │ 62ch    │      (P2 — não existe ainda)
                              └──────────────────┘
```

Tudo que o sistema **faz** — buscar, ler, citar, executar — mora no gutter de 108px. Tudo
que ele **diz** mora na coluna de leitura de 62ch. É por isso que atividade de ferramenta
não empurra a prosa: ela nunca esteve nela.

O reconhecimento é tipográfico, não gráfico: a pergunta é 21px, a resposta é 15px. A
diferença de escala separa os interlocutores sem bolha, avatar, borda ou cor.

Ordem de sacrifício em telas estreitas, sempre a mesma: conversas, depois inspector, depois
margem. **A coluna de leitura nunca é sacrificada** — em ultrawide sobra canvas dos dois
lados, porque uma linha de 200 caracteres não é um recurso.

### 7.3 Chips de contexto

Regra de produto, não de estética: **nada sai da máquina sem um chip visível.**

```text
[Project: M/OS ✕]  [Tela: Hermes ✕]  [Resource: architecture.md ✕]
 explícito          automático         explícito
```

Explícito e automático se distinguem por peso tipográfico e não por cor
(`DESIGN-FOUNDATIONS.md` §14: nenhum estado depende só de cor). Clicar no chip abre o que
exatamente foi enviado — o campo `sent` de §5.3.

### 7.4 Tool run

Estados obrigatórios, que a UI precisa distinguir sem depender de cor:

`queued` · `running` · `success` · `error` · `cancelled` · `waiting_permission`

Uma linha por execução, em mono, atrás do filete de 2px que o design já usa como marcador
de autoria do sistema:

```text
│ Buscando na web · 3 fontes                        1.2s  ▸
│ Lendo M/OS · Project M-Finance                    0.3s  ▸
```

`▸` expande para o payload técnico. Fechado por padrão. Nunca despejar log.

### 7.5 Modos — recomendação de não implementar

O usuário pediu para avaliar `Fast / Think / Research / Act` e proibiu modos falsos.
A avaliação honesta:

| Modo | Lastro real | Veredito |
|---|---|---|
| Fast | Nenhum controle de latência exposto que o M/OS possa usar sem violar ADR-024 | Não implementar |
| Think | `agent.reasoning_effort` existe — mas ADR-024 diz que **reasoning é do Hermes** | Exige emenda de ADR |
| Research | Skill `research` existe, forma dos eventos não verificada | P2, depois do gate |
| Act | Ferramentas já são escolhidas pelo agente | Redundante |

**Recomendação:** remover o seletor `ASK / ACT / ORGANIZE` da UI em P0 e **não colocar
nada no lugar**. Os três modos atuais têm dois desabilitados; substituí-los por quatro
modos com um só funcionando piora a situação. O Hermes escolhe skill sozinho, e é isso que
`UX-PRINCIPLES.md` §75 pede — IA só entra onde reduz trabalho.

Se o usuário quiser controle de esforço, isso é uma **emenda formal a ADR-024** (o M/OS
passaria a opinar sobre reasoning), não uma decisão de UI. Fica registrado como decisão em
aberto, não como plano.

> **Revisado em 2026-08-15, com o handoff do design.**
> `M-OS Hermes - Design Direction v1` §07 especifica os quatro modos e define o que eles
> são: *"a única promessa que o sistema faz sobre o que vai acontecer com seus dados"*.
>
> Isso muda a conclusão pela metade. O slot existe e vale a pena — o que não pode existir
> são promessas que o sistema não cumpre. Hoje há uma promessa verdadeira, e ela é
> garantida pela arquitetura em vez de por intenção: `mos-hermes` não compila com acesso
> ao banco. O composer exibe **`NÃO ESCREVE`** no slot e na tipografia do design.
>
> O ciclo de quatro entra quando houver mais de uma promessa a fazer, o que acontece em P4
> com `ACT`. Ver ADR-029, emenda.

### 7.6 Microinterações

Permitido, porque representa estado real:
- caret durante geração;
- transição de estado do tool run;
- scroll suave com trava inteligente e "novas mensagens abaixo";
- ações no hover da mensagem.

Proibido, e isto é requisito herdado de `UX-PRINCIPLES.md` §16 e da Spec B §8:
bolha de chat, avatar, orb, sparkle, gradiente "AI", glow permanente, partícula, spinner
grande. React Bits só entra se o efeito **for** o estado — nunca como enfeite. Na prática,
para P0 isso significa: nenhum.

---

## 8. Security model

### 8.1 O que já está certo

- Credencial no Windows Credential Manager, lida só no Rust (`auth.rs:19-78`).
- Renderer recebe `hasCredentials: boolean` e nada mais.
- Ticket de WS cunhado por conexão, 30 s, uso único, nunca guardado.
- `429` não dispara retry.
- `approval.respond` com `deny` por omissão.
- `mos-hermes` sem acesso ao banco, por construção.

### 8.2 Mudança de perfil de risco que o AI Workspace introduz

Esta seção precisa de decisão do proprietário do produto antes de P3.

`ARCHITECTURE.md` §15.2 modela o M/OS como dados locais no perfil do Windows. O baseline
não cobre **envio deliberado de conteúdo pessoal para uma VPS**. Hoje isso já acontece em
pequena escala: o que o usuário digita vai para o Hermes. A partir do momento em que
contexto do M/OS (Projects, Tasks, Captures, Resources) é anexado automaticamente, o volume
e a sensibilidade mudam de ordem de grandeza.

Controles que o desenho precisa carregar desde P0:

1. **Nenhum contexto automático sem chip visível.** Requisito, não preferência.
2. **`sent` persistido por mensagem** (§5.3), para a pergunta "o que vazou?" ter resposta.
3. **Contexto automático desligável**, e desligado por padrão até P3 existir.
4. **Anexo nunca é enviado implicitamente.** Arrastar um arquivo mostra o que será enviado
   antes de enviar.
5. **Temporary chat não gera memória nem contexto persistido** (P2).

### 8.3 Classificação de permissão para ações (P4)

Não precisa ser inventada. Ela **já existe** em `crates/mos-core/src/functions.rs`: 21
funções, cada uma com `risk` (low/medium/high) e `confirmation` (none/explicit). A regra de
confirmação do Hermes deve ler esse registro, não uma segunda lista:

| risk | confirmation | Comportamento do Hermes |
|---|---|---|
| low | none | Executa e informa, com Undo |
| medium | none | Executa e informa, com Undo e destaque |
| medium | explicit | Preview antes |
| high | explicit | Preview + confirmação inequívoca, sem Undo prometido |

Isso satisfaz `UX-PRINCIPLES.md` §20 e §21 (autonomia proporcional ao risco, Undo antes de
confirmation overload) sem criar uma segunda fonte de verdade. É o melhor ativo escondido
do repositório para esta evolução.

### 8.4 CSP e dependências novas

Renderizar Markdown introduz a primeira superfície de injeção do M/OS: conteúdo vindo da
VPS interpretado como marcação. Requisitos:

- sanitização obrigatória, sem `dangerouslySetInnerHTML` sem sanitizer;
- link externo abre pelo backend nativo, como `open_resource` já faz — nunca navegação
  dentro do WebView;
- imagem remota não é carregada automaticamente (a CSP restritiva de
  `ARCHITECTURE.md` §15.3 já bloqueia; a UI precisa explicar em vez de mostrar quebrado).

ADR-019 restringe dependências novas de UI. Markdown e highlight **são** dependências
novas e exigem justificativa registrada, como o plano da ponte fez com `tokio-tungstenite`.
A justificativa aqui é a mesma classe: escrever parser de Markdown e sanitizer à mão num
caminho que interpreta conteúdo remoto é trocar biblioteca auditada por código próprio no
exato lugar onde isso é mais perigoso.

---

## 9. Performance risks

### 9.1 Medido por leitura, ainda não instrumentado

| # | Risco | Onde | Gravidade |
|---|---|---|:--:|
| 1 | Um `setState` por token, com `map` sobre todos os turnos | `HermesPage.tsx:31-34` | Alta |
| 2 | Thread inteira re-renderiza a cada delta | `HermesPage.tsx:215-225` | Alta |
| 3 | Markdown reparseado a cada token quando entrar | futuro | **Crítica** |
| 4 | Highlight de código reexecutado por token | futuro | **Crítica** |
| 5 | Sem virtualização | — | Média (só em thread longa) |
| 6 | `aria-live` na thread com streaming | `App.tsx:1242` | Alta (§10) |
| 7 | Sem memoização de mensagem | — | Média |
| 8 | Persistir cada delta em SQLite | futuro | Alta se ingênuo |

O item 1 é O(n) por token: cada delta cria um novo array e um novo objeto para todos os
turnos. Numa resposta de 2 000 tokens sobre uma thread de 50 turnos, são 100 000 objetos.
Hoje passa despercebido porque a resposta é texto simples; com Markdown e highlight no
caminho, não passa.

### 9.2 Mitigações que devem entrar junto da renderização

- **Acumular delta fora do estado React** (ref) e sincronizar por `requestAnimationFrame`.
  O contrato de não rebufferizar é sobre a **rede** (`ws.py:155-170` desliga Nagle) e sobre
  a **ponte** — ele não obriga um commit de React por token. Um quadro por frame preserva
  a cadência percebida e é a leitura correta do requisito.
- **Só a última mensagem é volátil.** Mensagens completas viram componentes memoizados que
  nunca re-renderizam.
- **Markdown incremental:** durante o streaming, renderizar texto puro; ao `complete`,
  parsear uma vez. Alternativa: parsear só o último bloco. A primeira é mais simples e
  visualmente honesta (o texto chega, depois assenta).
- **Highlight sob `requestIdleCallback`**, nunca no caminho do delta.
- **Persistência por mensagem, não por delta.** Escrever no `complete`, no `interrupted` e
  num checkpoint periódico. Um `INSERT` por token com `synchronous=FULL` (ADR-017) seria
  um fsync por token.
- **Virtualização só quando medida.** Entra em P1 com um número, não por precaução.

### 9.3 Budgets propostos

Seguindo o formato de `ARCHITECTURE.md` §12 — budgets de engenharia, não garantias:

- primeiro token na tela após `prompt.submit`: perceptualmente imediato;
- streaming sustentado sem frame drop em thread de 100 mensagens;
- abrir conversa de 200 mensagens: primeiros resultados em até 100 ms;
- nenhuma escrita em SQLite no caminho crítico do delta.

---

## 10. Accessibility

`DESIGN-FOUNDATIONS.md` §14 define WCAG 2.2 AA como baseline e §16 lista dez quality gates.
A superfície Hermes atual não passa em vários.

| # | Problema | Onde | Fase |
|---|---|---|:--:|
| 1 | `aria-live="polite"` na thread inteira durante streaming | `App.tsx:1242` | P0 |
| 1b | A página Hermes não tem live region nenhuma: nada é anunciado | `HermesPage.tsx` | P0 |
| 2 | Tool state comunicado só por texto/cor, sem semântica | — | P0 |
| 3 | `Tab` sequestrado para trocar modo | `HermesPage.tsx:171` | P0 |
| 4 | Campo é `<input>`; `Shift+Enter` anunciado e inexistente | `HermesPage.tsx:260` | P0 |
| 5 | Sem região semântica de conversa (`log` / `feed`) | — | P0 |
| 6 | Menções sem combobox pattern (sem `aria-activedescendant`) | `HermesPage.tsx:237` | P0 |
| 7 | `reduced-motion` não considerado no caret e no scroll | — | P0 |
| 8 | Mensagem sem timestamp legível por leitor de tela | — | P0 |

Os itens 1 e 1b são as duas metades erradas do mesmo problema, e é instrutivo que as duas
superfícies tenham errado em direções opostas. No Command, `aria-live="polite"` numa região
que recebe um token por vez faz o Narrator anunciar a resposta caractere por caractere, ou
descartar tudo. Na página, não há live region alguma: o Hermes responde e o leitor de tela
não diz nada.

O padrão certo é o mesmo para as duas: anunciar **estados** (`Respondendo`,
`Resposta concluída`, `Ferramenta em execução`) numa região `status` pequena e separada, e
deixar a resposta como conteúdo navegável — nunca como live region.

O item 3 é conflito direto com `DESIGN-FOUNDATIONS.md` §12, que reserva `Tab` para mover
foco estrutural. Sequestrar `Tab` num campo de texto quebra a saída do teclado da
superfície. Some junto com o seletor de modos (§7.5).

---

## 11. Roadmap

Cada fase tem um gate. Nenhuma começa antes de a anterior passar. Sem estimativa de prazo,
por decisão de `ROADMAP.md` §1.

### P0 — Premium Chat Foundation · IMPLEMENTADO

Objetivo: **a experiência de conversar fica impecável.** Nada de agente onipotente.

- [x] 1. Corrigir `clarify.request` e `sudo.request` (defeito vivo, §1.3)
- [x] 2. Superfície única: Command perde o modo Hermes (§7.1)
- [x] 3. Modelo Conversation / Message / MessagePart + migration (§5.2)
- [x] 4. `session_id` persistido; `session.resume` volta a funcionar entre aberturas
- [x] 5. Reidratação por `session.history` — pedida só quando não há conversa local
- [x] 6. Renderização de Markdown, tabela, código com realce e Copy Code
- [x] 7. Composer multiline real (`<textarea>`), Enter/Shift+Enter honestos
- [x] 8. Stop · Retry · Regenerate · Edit e reenviar · Copy · timestamps
- [x] 9. Conversas: nova, renomear, título automático (`session.title`), excluir, buscar
- [x] 10. Render de tool run com os seis estados, na margem (§7.4, ADR-030)
- [x] 11. Streaming interrompido: preserva texto, nomeia a causa, oferece Retry
- [ ] 12. **Capability Service — adiado para P1.** Ver a nota abaixo.
- [x] 13. Context chips + `context_ref` persistido, **com o `@` levando contexto de
      verdade** — resolve §1.4
- [x] 14. Arquitetura de anexo definida (ADR-025: entra como `kind` de parte, vira tabela
      própria em P1 quando ganhar lifecycle)
- [x] 15. Correções de performance do §9.2
- [x] 16. Correções de acessibilidade do §10
- [x] 17. Seletor de modos: substituído pela promessa que o sistema cumpre (ADR-029, emenda)

**Por que o Capability Service saiu do P0.** A justificativa dele em §6.1 é impedir a UI de
adivinhar o que o gateway faz. Acontece que nenhuma feature de P0 adivinha: todas usam
métodos que o `HERMES-GATEWAY-CONTRACT.md` verificou ao vivo, e nenhuma se habilita ou
desabilita por capacidade. Construir agora um serviço de descoberta sem consumidor seria
exatamente a infraestrutura especulativa que ADR-012 proíbe.

O primeiro consumidor real aparece em P1, com anexos: `file.attach` e `image.attach_bytes`
existem no checkout mas nunca foram exercitados desta instalação, e habilitar o botão de
anexo sem perguntar seria a adivinhação que §6.1 quer evitar. O serviço entra junto dele.

**Gate P0 — verificado:** `cargo fmt --check`, `cargo clippy -D warnings`,
`cargo test --workspace` (122 testes), `tsc --noEmit` e `npm run build`, todos limpos.
Zero literal de cor novo. `mos-hermes` continua sem `mos-core` e sem `mos-storage-sqlite`.

**Gate P0 — pendente de execução em tela:** os dez quality gates de
`DESIGN-FOUNDATIONS.md` §16 (screenshots em três larguras, scaling 100/125/150%, tema
claro e High Contrast, Narrator, contraste programático) e os cenários de falha do §12
que exigem o túnel aberto. Nenhum deles pode ser declarado a partir do código.

### P1 — Context + Multimodal

18. Anexo real: `file.attach`, `image.attach_bytes`, `pdf.attach`
19. Drag and drop, colar imagem e screenshot
20. `Attachment` vira tabela; relação com Library definida (§12.4)
21. `@Task` e `@Capture`
22. Message queue (enviar enquanto roda)
23. Virtualização, se a medição de P0 exigir
24. Continue generation, se o gateway suportar

**Gate P1:** um arquivo anexado, respondido e reencontrado na Library sem duplicação.

### P2 — Research + Artifacts

25. Verificar a forma dos eventos da skill `research` (**gate obrigatório**)
26. Modo Research como tipo de execução distinto, com progresso e steering (`session.steer`)
27. Arquitetura de citations, dependente do resultado de 25 (§6.3)
28. Sources panel e Source Inspector
29. Artifact: modelo, superfície paralela, copiar, exportar — **sem editor**
30. Slash commands via `commands.catalog`
31. Branch de conversa (`parent_id` sai de reserva)
32. Pin, duplicate, temporary chat, relação com Project

**Gate P2:** uma pesquisa real, com fontes abríveis e rastreáveis até a origem.

### P3 — M/OS Read Tools

33. Context Service: montagem estruturada e orçada de contexto
34. `mos_search`, `mos_get_context`, `mos_get_project`, `mos_get_task`, `mos_get_capture`,
    `mos_get_resource` — **somente leitura**, pelo caminho A (§6.4)
35. Contexto automático de tela, desligado por padrão, com chip sempre visível
36. ADR de envio de dados pessoais à VPS (§8.2)

**Gate P3:** "o que falta fazer aqui?" dentro de um Project responde com dados reais, e o
usuário consegue ver exatamente o que foi enviado.

### P4 — M/OS Actions

37. Tool Gateway em `mos-core`, lendo `functions.rs` para risco e confirmação (§8.3)
38. `mos_create_capture`, `mos_create_task`, `mos_update_task`, `mos_move_task`,
     `mos_create_project`, `mos_create_resource`
39. Preview, confirmação proporcional, resultado explícito, Undo onde reversível
40. Painel "criado nesta sessão" — a moldura já existe em `HermesPage.tsx:293`

**Gate P4:** nenhuma ação atravessa caminho paralelo aos serviços que a UI usa; toda ação
de risco `high` exige confirmação; toda ação reversível oferece Undo.

### P5 — Voice + Automations + Advanced Agent

41. Dictation (`voice.record`, `voice.transcript`)
42. Voice conversation com transcript persistido
43. Memory (Personal / Project / Conversation), visualizável e editável
44. Agent Run em background (`subagent.*`)
45. Automations via `cron.manage`, com ADR de segurança própria
46. Connectors como capabilities

**Gate P5:** cada item exige ADR própria antes de entrar.

### Diferença em relação à sugestão inicial de P0

A sugestão do usuário incluía "composer premium" e "attachment architecture" no P0. O corte
acima mantém a **arquitetura** de anexo em P0 e move a **implementação** para P1, e reduz
"composer premium" a multiline honesto + contexto real. Motivo: P0 já carrega o modelo de
dados, a correção de dois defeitos vivos, Markdown e performance. Acrescentar upload de
arquivo ali transformaria a fundação em entrega grande demais para ser verificada de uma
vez — que é exatamente o risco principal registrado em `ROADMAP.md` §22.

Adições ao P0 que a sugestão não previa e a auditoria tornou obrigatórias:
`clarify`/`sudo` (§1.3), superfície única (§1.2) e persistência do `session_id` (§1.2).

---

## 12. Definition of Done

P0 só é considerado concluído quando **todos** os itens abaixo forem verificados. Nenhum
é declarável sem evidência.

### 12.1 Funcional

- [ ] Uma pergunta enviada produz exatamente uma thread, com o Command aberto ou fechado
- [ ] Fechar e reabrir o M/OS retoma a conversa, com histórico da VPS reidratado
- [ ] Resposta com heading, lista, tabela, código e link renderiza corretamente
- [ ] Copy Code copia só o código; Copy copia a resposta em Markdown
- [ ] Stop interrompe e preserva o texto recebido
- [ ] Retry após falha reenvia sem duplicar a mensagem do usuário
- [ ] Regenerate substitui a resposta e mantém a pergunta
- [ ] Editar e reenviar preserva a mensagem original no histórico
- [ ] `@Project` envia contexto estruturado, e o chip mostra exatamente o que foi enviado
- [ ] Conversas listadas, renomeáveis, buscáveis, arquiváveis
- [ ] Tool run aparece com os seis estados, colapsado por padrão

### 12.2 Falha — cenários observados, não presumidos

Espelha o formato da Spec B §10, que exigiu observar em vez de supor.

- [ ] Túnel fechado: página inteira usável, causa nomeada, M/OS íntegro
- [ ] Túnel cai no meio do streaming: texto preservado, interrupção nomeada, Retry oferecido
- [ ] `clarify.request` chega: pergunta renderizada, resposta enviada, turno continua
- [ ] `sudo.request` chega: tratado, e negar não trava a sessão
- [ ] `approval.request` continua funcionando, com deny por omissão
- [ ] `4009 session busy`: UI oferece cancelar, não repete
- [ ] `session.resume` com id morto: recuperação por título, sem erro na cara do usuário
- [ ] Frame desconhecido: nomeado, sem derrubar a conexão
- [ ] `429`: nenhuma tentativa automática

### 12.3 Qualidade — os dez gates de `DESIGN-FOUNDATIONS.md` §16

- [ ] Screenshots em `1280×800`, `1024×768`, `840×600`
- [ ] Scaling Windows 100%, 125%, 150%
- [ ] Tema claro, escuro e High Contrast
- [ ] Navegação completa por teclado, sem `Tab` sequestrado
- [ ] Narrator: estados anunciados, resposta navegável, sem flood de token
- [ ] Contraste medido programaticamente
- [ ] Conteúdo longo e strings pt-BR
- [ ] Estados default, hover, focus, pressed, loading, empty, error, disabled
- [ ] `prefers-reduced-motion`
- [ ] Uso repetido sem layout shift

### 12.4 Arquitetura — invariantes que não podem ser quebradas

- [ ] `crates/mos-hermes/Cargo.toml` continua sem `mos-storage-sqlite`
- [ ] `mos-hermes` continua sem `mos-core`
- [ ] Nenhuma chamada de rede em componente React
- [ ] Nenhuma credencial, cookie ou ticket no renderer
- [ ] Nenhum SQL no renderer
- [ ] Toda escrita atravessa os serviços de `mos-core` já usados pela UI
- [ ] Zero literal de cor fora de `mos-tokens.css`
- [ ] Dependências novas justificadas e registradas, como fez o plano da ponte
- [ ] `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`, `npm run build`

### 12.5 Conceitos definidos antes de código (§ pedido explicitamente)

- [ ] Relação entre `Attachment`, `Resource` e Library escrita e aprovada, sem duplicação
- [ ] Diferença entre contexto explícito e automático definida na UI e no modelo
- [ ] Fronteira entre Memory e histórico de conversa definida
- [ ] Caminho de leitura do M/OS (§6.4) escolhido com ADR

---

## 13. Decisões em aberto para o proprietário do produto

Nenhuma delas bloqueia P0. Todas bloqueiam alguma fase posterior.

1. **Controle de esforço de raciocínio.** Expor exige emendar ADR-024, que dá ao Hermes a
   posse de model, provider e reasoning. Recomendação: não expor.
2. **Caminho de leitura do M/OS** (§6.4). Recomendação: A em P3; B só com ADR.
3. **Envio de dados pessoais à VPS** (§8.2). Precisa de ADR antes de P3.
4. **Markdown e highlight como dependências novas** (§8.4). Precisa de decisão registrada
   sob ADR-019.
5. **Conversa em SQLite local.** Confirma que o M/OS passa a guardar o que hoje só vive na
   VPS — muda o que backup e export contêm, e `ARCHITECTURE.md` §16 avisa que backup pode
   conter dado pessoal em texto claro.
6. **Destino do sinal de feedback ±** (§4.1). Sem destino, é botão decorativo — fora do
   plano até existir.
