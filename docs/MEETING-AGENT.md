# M/OS — Meeting Agent

**Status:** **Fases 1 a 5 e a interface concluídas.** Gates A, B, D e E passam; C passa com
áudio sintético. A cadeia é operável ponta a ponta: gravar → recuperar → transcrever →
analisar → virar Task e Reminder, com evidência clicável e desfazer. Reuniões entrou no rail
pela **ADR-044**. **Falta o Gate F (os dez itens do `DESIGN-FOUNDATIONS.md` §16) e o Gate G
— uma reunião de verdade.**

**Data:** 2026-08-18

**Subordinado a:** `VISION.md`, `PRODUCT.md`, `CORE.md`, `CORE-FOUNDATION.md`, `UX-PRINCIPLES.md`, `ARCHITECTURE.md`, `DECISIONS.md`, `DESIGN-FOUNDATIONS.md`

**Decisões do proprietário do produto tomadas em 2026-08-18, antes deste documento:** D-A, D-B, D-C e D-D da §24.

---

## 0. Auditoria: o terreno real

Esta seção existe pela mesma razão que a §0 do `ATTENTION-SYSTEM.md`: registrar o que já
existe antes de desenhar sobre coisa que não existe.

### 0.1 O Meeting Agent não é uma feature lateral — ele desbloqueia outro documento aprovado

O `ATTENTION-SYSTEM.md` §0.2 lista, em tabela, as capacidades que ficaram bloqueadas por
falta de dado. Uma delas é literal:

> | Smart Snooze "após a reunião" | **bloqueado** — não há reunião no sistema |

E o `CORE.md` §25 já definia Reminder como *"a intenção de ser lembrado sobre algo em
determinado momento **ou condição**"*. A condição "quando a reunião acabar" estava prevista
na linguagem do domínio e sem nenhum dado que a sustentasse.

**Consequência:** este trabalho abre uma fronteira que outro documento aprovado já esperava.
Ele não introduz um substantivo estranho ao produto.

### 0.2 A cadeia de "propor → confirmar → executar" já existe e já está provada

`SPEC-ACOES-ENTRE-APPS.md` fases 1 e 2 estão entregues. O que existe hoje, em produção:

| Peça | Onde | O que resolve para o Meeting Agent |
|---|---|---|
| `parse_action` / `preview_of` / `UndoStep` | `mos-core/src/action.rs` (1140 linhas) | é exatamente o "Task Draft → usuário confirma → Core cria Task" do pedido |
| Registro de Functions com `risk` e `confirmation` | `mos-core/src/functions.rs` | 27 ações declaradas; `meeting.*` entra ali, não num catálogo paralelo |
| Execução pelos serviços da aplicação | `jarvis.rs::action_resolve` | nunca SQL próprio, nunca atalho |
| Desfazer que arquiva e nunca apaga | ADR-035 | o inverso de "criar Task a partir de reunião" já está decidido |

**Este documento não desenha um segundo pipeline de confirmação.** Ele usa o que está lá.

### 0.3 `Task` não tem prazo, e não existe `Event`

Verificado no código, não inferido. `struct Task` em `mos-core/src/work.rs` tem `id`,
`title`, `description`, `project_id`, `source_capture_id`, `state`, `lifecycle_state`,
`created_at`, `updated_at`, `completed_at`. **Não tem `due_at`.**

`mos-core/src/calendar.rs` é retrospectivo por construção — o comentário de cabeçalho diz
*"O que aconteceu, em forma de item de calendario (fase 1)"* — e os cinco `CalendarKind`
são todos fatos passados.

**Consequência:** o deadline de um Action Item não tem onde pousar como campo de Task.
A decisão D-C da §24 resolve isso sem migration em Task. Ver §14.

### 0.4 Infraestrutura que já existe e que não será reconstruída

| Peça | Estado | Onde |
|---|---|---|
| Migrations numeradas | até `0016_widget_order.sql`; a nossa é a `0017` | `mos-storage-sqlite` |
| `SearchItem` com variantes por tipo + FTS5 reconstruível | existe | `work.rs`, `work_repository.rs` |
| `LifecycleState` (`active`/`archived`/`trashed`) | existe, reusado por Capture, Task, Project, Resource, Conversation, Reminder | `capture.rs` |
| `Clock` como porta | existe (`SystemClock`, `FixedClock`) | `clock.rs` |
| `ReminderTarget` enum fechado + `AttentionService` | existe, com agendador rodando | `attention.rs`, `attention_repository.rs` |
| `ActivityType::Meeting` no rastreio de tempo | **já existe a variante** | `tracking.rs` |
| Ponte Hermes (sessão, streaming, interrupt) | existe e exercitada | `mos-hermes`, `hermes.rs` |
| Injeção de contexto com chip e registro | existe | `jarvis.rs::assemble_context`, ADR-027/028 |
| `ffmpeg` no PATH da máquina | existe | verificado 2026-08-18 |

### 0.5 A máquina alvo, medida e não suposta

| Item | Valor verificado em 2026-08-18 |
|---|---|
| Windows | 11 Pro, build 10.0.26200 |
| CPU | AMD Ryzen 9 9950X3D, 16 núcleos / 32 threads |
| RAM | 47,2 GB |
| GPU | NVIDIA GeForce RTX 5070 Ti |
| CUDA Toolkit | **ausente** |
| Toolchain Rust | `stable-x86_64-pc-windows-msvc` 1.97.0 — **não é GNU** |
| MSVC Build Tools | 17.14.37411.7, com `cl.exe` |
| `cargo check -p mos-core` | passa nesta árvore |

O item que muda decisão é o penúltimo. O `SETUP-MAQUINA.md` descreve a máquina principal
com toolchain **GNU** e registra, na §4, que `cargo test -p mos-desktop` **não roda lá**.
Este worktree usa MSVC. As duas coisas juntas dão a mesma conclusão, por caminhos
diferentes: **a lógica precisa morar em crates que testam.** Ver §21.

### 0.6 Restrições que este desenho não pode violar

- **`ARCHITECTURE.md` §9** — Domain não depende de Tauri, React, SQLite ou cloud. WASAPI é
  adapter, e o domínio de Meeting não pode conhecê-lo.
- **`CORE-FOUNDATION.md` §2, princípio 7** — Inbox, Kanban, Library, Home e Search são
  projeções. A lista de Meetings também é: ela não é uma tabela de "itens de reunião".
- **`CORE-FOUNDATION.md` §2, princípio 6** — nada é duplicado para aparecer em outra
  visualização. O Meeting Agent não guarda cópia de Task nem de Reminder.
- **`ADR-012`** — sem abstração genérica de grafo. `ReminderTarget` ganha um braço, com
  migration e uma linha no `match`, que é exatamente a consequência que a ADR aceitou.
- **`ADR-024`** — Hermes é superfície, não segundo agente. Nenhum framework de agente novo.
- **`ADR-027`** — nada sai para o Hermes sem chip visível e registro do que foi enviado.
- **`ADR-034`** — orçamento de movimento: um loop por tela, movimento que carrega dado.
- **`ADR-039`** — o rail está em onze; o décimo segundo exige retirar um **ou** ADR nova.
  Ver D-B na §24 e a ADR-044.

---

## 1. Product Thesis

Uma reunião é o lugar onde o M/OS mais perde informação hoje.

Durante uma hora de conversa nascem decisões, compromissos, prazos e pendências — exatamente
os quatro substantivos que o M/OS existe para guardar. E hoje eles não entram no sistema por
nenhum caminho, porque o único caminho de entrada é digitar, e ninguém digita enquanto fala.

A `VISION.md` §14 chama isso pelo nome:

> Sempre que uma funcionalidade exigir que o usuário pare o que está fazendo para alimentar
> o sistema, devemos perguntar se o próprio M/OS poderia realizar esse trabalho.

O Meeting Agent é a resposta a essa pergunta para o caso da reunião. A tese, em uma frase:

> **Entrar numa reunião não deveria custar atenção de secretário.**

E a promessa de confiabilidade, que é o que separa esta feature de uma demo:

> **Nada do que foi gravado se perde em silêncio.** Se o M/OS cair, se o headset for
> arrancado, se a transcrição falhar ou se o Hermes estiver fora do ar, o sistema diz o que
> aconteceu e preserva tudo que já tinha.

Nenhuma parte deste documento pode contradizer essa frase. Onde houver conflito entre
"impressionante" e "não perde", ganha "não perde".

### 1.1 A pergunta do §66 do `UX-PRINCIPLES`

> Ela reduz algo que preciso lembrar? — **Sim.** Quem prometeu o quê, e até quando.
>
> Ela facilita encontrar algo? — **Sim.** "O que ficou decidido no NexoDoc?" passa a ter resposta.
>
> Ela conecta informações hoje separadas? — **Sim.** Reunião ↔ Project ↔ Task ↔ Reminder.
>
> Ela reduz etapas para executar algo? — **Sim.** Compromisso vira Task sem redigitação.
>
> Ela melhora compreensão ou confiança? — **Sim, com a condição da §12.3:** toda afirmação
> carrega evidência clicável na transcrição, ou não é afirmada.

---

## 2. Goals

1. Iniciar e parar a captura de uma reunião manualmente, em dois cliques.
2. Capturar o microfone e o áudio do Windows **em canais separados e preservados**.
3. Gravar de forma incremental e durável, de modo que uma queda custe no máximo um chunk.
4. Detectar, na abertura seguinte, uma gravação interrompida e oferecer processá-la.
5. Transcrever localmente, com timestamps e origem por segmento.
6. Analisar a transcrição com o Hermes e receber saída **estruturada e validável**.
7. Apresentar resumo, decisões, minhas ações, ações de outros, prazos, follow-ups, questões
   em aberto e riscos — cada um com evidência apontando para a transcrição.
8. Converter o que for relevante em Task e Reminder **mediante confirmação**, pela cadeia de
   ação que já existe.
9. Relacionar a reunião a um Project.
10. Encontrar a reunião pela Search global e conversar sobre ela com o Jarvis.
11. Funcionar sem internet durante a gravação e a transcrição.
12. Nunca gravar sem indicação visível.

---

## 3. Non-Goals

Fora da V1, e não por esquecimento:

| Fora | Por quê |
|---|---|
| Bot entrando no Google Meet | pedido explícito do proprietário; e exigiria credencial de terceiros |
| API do Google Meet / transcrição nativa do Meet | acopla a feature a uma fonte, contra o princípio fundamental |
| Início automático da gravação | `ADR-037` — *"observação não vira hora sozinha"*; o mesmo vale para gravação |
| Realtime: transcrição, ações ou assistência ao vivo | conflita com "pós-reunião confiável", e a regra de decisão do brief manda escolher confiabilidade |
| Diarização perfeita de todos os speakers | a distinção MIC vs SYSTEM já entrega o valor; ver §10.4 |
| Captura de vídeo, tela ou gravação em nuvem | `ADR-037` desenha a fronteira de observação, e vídeo a atravessa |
| Captura por processo (`AUDIOCLIENT_ACTIVATION_PARAMS`) | possível na V2 — a API existe no crate escolhido; ver §23 |
| Integração específica com Zoom/Teams | o desenho é agnóstico à fonte por construção |
| Android / iOS | `ADR-001`, `ADR-003` |
| Reuniões presenciais só com microfone | funciona por consequência, mas não é alvo de QA da V1 |

---

## 4. Architecture

### 4.1 Camadas

```text
React UI                       recebe estado e eventos; nunca PCM, nunca WASAPI
    │ comandos tipados / eventos Tauri
    ▼
Tauri commands  (apps/desktop/src-tauri/src/meeting.rs)
    │
    ▼
MeetingService  (crates/mos-core/src/meeting.rs)
    │                          dominio puro: maquina de estados, contrato de analise
    ├──► MeetingRepository ──► crates/mos-storage-sqlite
    ├──► RecordingPort ──────► crates/mos-audio          (WASAPI, Windows-only)
    ├──► TranscriptionPort ──► provider local (whisper) | provider cloud (futuro)
    └──► AnalysisPort ───────► jarvis.rs ──► mos-hermes ──► tunel SSH ──► Hermes VPS
```

### 4.2 O crate novo, e por que ele não pode ver o banco

`crates/mos-audio` **existe**, Windows-only, e **não depende de `mos-storage-sqlite` nem de
`mos-core`** — a ausência é verificada pelo compilador, não por convenção.

Esse isolamento é o mesmo truque estrutural da ADR-024, que tirou `mos-storage-sqlite` do
`mos-hermes` para que *"Hermes nunca escreve no SQLite"* deixasse de ser regra a lembrar e
virasse impossibilidade de compilação. Aqui ele compra duas garantias:

1. **A thread de captura não pode esperar o lock do SQLite.** O `SqliteStorage` guarda a
   conexão num `Mutex`; um `INSERT` no caminho do áudio significaria uma thread de tempo real
   bloqueada por um backup em curso. Sem a dependência, esse código não compila.
2. **A captura não pode inventar semântica de domínio.** Ela produz bytes e fatos
   (`frames escritos`, `descontinuidade em t`, `dispositivo sumiu`). O que isso significa
   para uma Meeting é decidido em `mos-core`.

`mos-audio` tem erro próprio (`AudioError`), como `mos-hermes` tem o dele. A tradução para
`CoreError` acontece no crate do desktop, que é o único lugar onde adapter e domínio se
encontram.

### 4.3 O que a UI recebe

Um evento por segundo, e só isto:

```ts
type RecordingState = {
  meetingId: string
  status: "recording" | "stopping"
  durationMs: number
  mic:    { state: "capturing" | "unavailable" | "lost"; level: number }
  system: { state: "capturing" | "unavailable" | "lost"; level: number }
  bytesOnDisk: number
}
```

`level` é um RMS já reduzido a `0..1` no Rust. **Não existe caminho de PCM para o renderer.**
`UX-PRINCIPLES` §51 e a instrução explícita do brief ("No waveform showcase") são a razão.

---

## 5. Audio Capture Strategy

### 5.1 A escolha da API, com a alternativa registrada

| Alternativa | Avaliação | Decisão |
|---|---|---|
| **`wasapi` 0.24** (HEnquist) | wrapper fino de WASAPI, Windows-only, loopback de dispositivo, modo por evento e por polling, notificação de mudança de dispositivo, `BufferInfo` com flags/posição/QPC | **Escolhida** |
| `cpal` | abstração multiplataforma, callback-based; loopback existe, mas escondido atrás de "chame `build_input_stream` num device de saída"; descontinuidade e troca de dispositivo mal expostas | Rejeitada: paga abstração multiplataforma que a `ADR-001` não pede e esconde exatamente os sinais de falha que a promessa da §1 exige |
| `windows` crate cru | controle máximo | Rejeitada agora: é o que o `wasapi` já faz, com mais código nosso para revisar. Continua sendo o caminho de saída se o crate limitar algo |

O `wasapi` foi escolhido por um motivo que não é conveniência: ele **expõe os sinais de
falha**. Uma API que escondesse `AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY` nos obrigaria a
prometer continuidade que não podemos verificar.

### 5.2 Microfone

Dispositivo padrão de `Direction::Capture`, shared mode, `StreamMode::EventsShared`.

### 5.3 Áudio do sistema — loopback de dispositivo

Confirmado na documentação da Microsoft e no código do crate:

> Em modo loopback, o cliente obtém um `IMMDevice` do **endpoint de renderização** e
> inicializa um stream de captura com `AUDCLNT_STREAMFLAGS_LOOPBACK`. Loopback só existe em
> shared mode; exclusive mode não opera em loopback.

No `wasapi`, isso é expresso pela combinação de direções, e a combinação inválida é recusada
com erro tipado em vez de comportamento indefinido:

```rust
let mut streamflags = match (&self.direction, direction, sharemode) {
    (Direction::Render, Direction::Capture, ShareMode::Shared)    => AUDCLNT_STREAMFLAGS_LOOPBACK,
    (Direction::Render, Direction::Capture, ShareMode::Exclusive) => return Err(WasapiError::LoopbackWithExclusiveMode),
    (Direction::Capture, Direction::Render, _)                    => return Err(WasapiError::RenderToCaptureDevice),
    _ => 0,
};
```

Ou seja: pegamos o device **de saída** e pedimos `Direction::Capture` nele.

### 5.4 O buraco do loopback, e o keep-alive de silêncio

Este é o risco técnico número um da feature, e ele não é um bug nosso:

> **O WASAPI só empurra dados para o endpoint de renderização quando existe algum stream
> ativo. Quando nada está tocando, não há nada para capturar.**

Numa reunião isso acontece o tempo todo: ninguém do outro lado fala por 40 segundos e o canal
SYSTEM simplesmente **para de produzir pacotes**. A consequência não é só um arquivo menor —
é que a linha do tempo do canal SYSTEM deixa de corresponder à do MIC, e a evidência `14:04`
passa a apontar para o lugar errado.

**Decisão: enquanto a gravação existir, o M/OS mantém um stream de renderização de silêncio
no mesmo endpoint.** Ele escreve zeros, não produz som audível, e mantém o motor de áudio
rodando — o que garante que o cliente de loopback receba pacotes contínuos, marcados com
`AUDCLNT_BUFFERFLAGS_SILENT` quando forem silêncio de verdade.

O custo é um stream de render compartilhado, ocioso. O ganho é que os dois canais compartilham
uma única linha do tempo verificável. Sem isso, "canais separados" seria uma promessa que a
evidência não sustenta.

> **Confirmado em 2026-08-18 pelo spike da Fase 1.** Contra um endpoint de saída ocioso
> (NVIDIA HDMI), 25 segundos de silêncio deram **2.498 pacotes com o keep-alive e zero
> pacotes sem ele**. O buraco é real e total: sem um stream de renderização ativo, o canal
> SYSTEM não produz um único frame. Ver `TECHNICAL-SPIKE-MEETING-AUDIO.md` §5.3.
>
> A primeira medição, contra a saída padrão da máquina, não mostrou diferença — e teria
> levado à conclusão errada. Aquela saída é um dispositivo virtual (Elgato) cujo driver
> mantém stream ativo permanentemente e alimenta o loopback sozinho. **O teste só é válido
> contra um endpoint que nada esteja usando.**

### 5.5 Timing: evento ou polling

A documentação da Microsoft diz:

> Em versões anteriores ao Windows 10 1703, o cliente de captura em pull-mode não recebe
> eventos quando o stream é inicializado com buffering por evento e loopback. (...) No Windows
> 10 1703 e superiores, clientes de loopback por evento são suportados.

Estamos no 26200, e havia relato de campo consistente de que o evento nunca dispara mesmo em
versões suportadas. **O spike mediu: no 26200 ele dispara.** Maior intervalo entre pacotes de
11 ms nos dois canais, zero fallbacks. O polling também funciona, com intervalo de 16 ms e
2,4× a CPU — pior nos dois eixos. `EventsShared` é o padrão por medição, não por documentação.

O mecanismo de degradação continua, porque "funciona nesta máquina" não é "funciona":

- o spike mede as duas: `EventsShared` e polling com waitable timer no período do device;
- o modo efetivamente usado é **gravado no `session.json` da sessão**;
- **não há degradação silenciosa.** Se o evento não vier dentro de 3× o período do device, o
  canal registra o fato e troca para polling, e a troca aparece na sessão.

### 5.6 Formato na captura

Pedimos `16 kHz, mono, i16` diretamente ao WASAPI, com `autoconvert: true` — que corresponde
a `AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM` e deixa o motor de áudio fazer a conversão.

Três razões:

1. **É o que o consumidor quer.** O Whisper trabalha em 16 kHz mono. Guardar 48 kHz estéreo
   float seria 12× o disco para informação que nenhum consumidor lê — e o áudio é apagado
   depois do processamento de qualquer forma (§16).
2. **Resample no fio da captura é risco.** Deixar o motor do sistema converter tira um
   algoritmo nosso de uma thread de tempo real.
3. **Torna o orçamento de disco trivial:** 32 kB/s por canal, 64 kB/s no total, ~230 MB por
   hora de reunião com os dois canais.

**Medido em 2026-08-18: `autoconvert` funciona junto com loopback.** Os dois canais aceitaram
`16000/1/i16`. Sem ele, o formato nativo é `48000/2/f32` e 30 segundos ocupam 23,0 MB contra
0,96 MB — **24× o disco** para informação que nenhum consumidor lê.

O fallback continua no desenho: se o `autoconvert` for recusado, a captura usa o mix format e
a conversão vira um estágio próprio, fora da thread de captura. Ele deixou de ser provável,
não de ser necessário.

### 5.7 O que nunca acontece

- MIC e SYSTEM **nunca** são misturados na gravação. A separação é a única fonte de
  "eu prometi" versus "outra pessoa disse", e a regra de decisão do brief é explícita: entre
  identificar todos os speakers e preservar YOU vs SYSTEM, preserva-se o segundo.
- Nenhum dos dois streams usa exclusive mode. Além de loopback não permitir, exclusive tomaria
  o dispositivo do Meet.
- A thread de captura nunca toca o banco, nunca serializa JSON e nunca emite evento Tauri.
  Ela escreve bytes num arquivo e empurra fatos por um canal.

---

## 6. Meeting Lifecycle

### 6.1 Uma máquina de estados, não quatro booleanos

O brief pede `audioState`, `transcriptionState` e `analysisState` como campos. Este documento
usa **um** enum, pela razão que o próprio brief enuncia ("não espalhar booleans se uma state
machine resolver melhor") e pelo precedente da ADR-015: dimensões só se separam quando são de
fato ortogonais.

Elas não são. Uma reunião não pode estar transcrevendo e analisando ao mesmo tempo; a análise
não pode começar antes da transcrição terminar. Três campos permitiriam representar estados
impossíveis.

O que **é** ortogonal — e por isso continua separado — é `lifecycle_state`, exatamente como
em Capture, Task, Resource e Reminder.

```text
                  ┌───────────┐
   Start ────────►│ Recording │
                  └─────┬─────┘
              Stop      │       queda do processo
        ┌───────────────┴──────────────────┐
        ▼                                  ▼
  ┌───────────┐                     ┌─────────────┐
  │ Stopping  │                     │ Interrupted │
  └─────┬─────┘                     └──────┬──────┘
        │                      [Processar] │  │ [Descartar]
        ▼                                  │  ▼
  ┌───────────┐◄───────────────────────────┘  ┌───────────┐
  │ Recorded  │                               │ Cancelled │
  └─────┬─────┘                               └───────────┘
        ▼
  ┌──────────────┐   falha    ┌───────────────────────┐
  │ Transcribing │───────────►│ Failed{transcription} │──► retry ──► Recorded
  └─────┬────────┘            └───────────────────────┘
        ▼
  ┌──────────────┐   estado de repouso legitimo: a transcricao e dado da reuniao
  │ Transcribed  │   mesmo que a analise nunca aconteca
  └─────┬────────┘
        ▼
  ┌──────────────┐   falha    ┌──────────────────┐
  │  Analyzing   │───────────►│ Failed{analysis} │──► retry ──► Transcribed
  └─────┬────────┘            └──────────────────┘
        ▼
  ┌───────────┐
  │   Ready   │
  └───────────┘

lifecycle_state: active <-> archived
                 active <-> trashed
```

**Dez estados, cada um com transição explícita e testável.** Quatro regras que o código impõe:

1. **`Transcribed` é estado de repouso, não de passagem.** Se o Hermes estiver offline, a
   reunião fica ali com transcrição completa e utilizável. `Failed{analysis}` é para falha
   real de contrato, não para ausência de rede. Ver §20.
2. **`Failed` nunca é terminal.** Ele guarda o estágio e volta ao repouso anterior no retry.
   Uma reunião nunca sai de existência por falha técnica.
3. **`Interrupted` é estado real, não ausência.** Ele existe no banco, com a duração recuperada
   medida em disco. Ver §9.
4. **`Cancelled` apaga o áudio, e só ele.** A linha da Meeting fica, com `cancelled_at`, para
   que "descartei uma reunião de 1h18" seja um fato consultável e não um buraco.

### 6.2 Onde a máquina mora

Em `mos-core/src/meeting.rs`, como função pura `apply(status, transition) -> Result<Status>`,
no mesmo formato de `attention::apply`. Testável sem janela, sem SQLite, sem WASAPI — o que é
obrigatório, e não estético: ver §21 e a §0.5.

---

## 7. Data Model

Migration `0017_meetings.sql`. Cinco tabelas.

### 7.1 `Meeting`

```rust
pub struct Meeting {
    pub id: MeetingId,                     // UUID v7, gerado no cliente
    pub title: String,                     // editavel; nasce de um default temporal
    pub status: MeetingStatus,
    pub lifecycle_state: LifecycleState,   // reusa o enum existente
    pub source: MeetingSource,             // Manual na V1; Calendar/Detected reservados

    pub started_at: OffsetDateTime,        // UTC
    pub ended_at: Option<OffsetDateTime>,
    pub duration_ms: i64,                  // medido em frames gravados, nao em relogio

    pub project_id: Option<ProjectId>,

    pub audio_dir: String,                 // relativo ao diretorio de dados
    pub retention: AudioRetention,
    pub audio_deleted_at: Option<OffsetDateTime>,

    pub mic: ChannelOutcome,
    pub system: ChannelOutcome,

    pub failure: Option<MeetingFailure>,   // estagio + mensagem legivel
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

`duration_ms` é medido em **frames gravados**, não pela diferença de relógio. Se um canal
perdeu 4 segundos, a duração precisa refletir o que existe. Um número derivado do relógio
mentiria justamente no caso em que a verdade importa.

`mic` e `system` são campos separados — e isso **não** é o antipadrão de booleanos espalhados.
São dois dispositivos físicos independentes cujos destinos divergem de verdade: o headset cai
e o áudio do sistema continua. É exatamente a distinção que a §20 exige mostrar.

```rust
pub enum ChannelOutcome {
    Captured,
    Unavailable { reason: String },        // nunca abriu
    Lost { at_ms: i64, reason: String },   // abriu e caiu; tudo ate at_ms esta gravado
}

pub enum AudioRetention { DeleteAfterProcessing, Keep24h, Keep }
```

### 7.2 `TranscriptSegment`

```rust
pub struct TranscriptSegment {
    pub id: SegmentId,
    pub meeting_id: MeetingId,
    pub seq: i64,                 // ordem de leitura, ja intercalada entre canais
    pub start_ms: i64,            // relativo ao inicio da reuniao
    pub end_ms: i64,
    pub channel: Channel,         // Mic | System
    pub text: String,
    pub speaker: Option<String>,  // None na V1; reservado
    pub confidence: Option<f32>,
}
```

`start_ms` é relativo ao início da reunião e **comum aos dois canais**, o que só é verdade por
causa do keep-alive da §5.4. É essa âncora única que faz `14:04` significar a mesma coisa nos
dois lados.

### 7.3 `MeetingAnalysis`

```rust
pub struct MeetingAnalysis {
    pub meeting_id: MeetingId,   // uma por Meeting; reanalise substitui
    pub summary: String,
    pub model: String,           // o que o gateway reportou
    pub produced_at: OffsetDateTime,
    pub windows: u32,            // quantas janelas de transcricao foram enviadas (§11.4)
}
```

### 7.4 `MeetingInsight` — uma tabela, não oito

```rust
pub enum InsightKind {
    Decision, MyAction, OtherAction, Deadline,
    FollowUp, OpenQuestion, Risk, Topic,
}

pub struct MeetingInsight {
    pub id: InsightId,
    pub meeting_id: MeetingId,
    pub kind: InsightKind,
    pub seq: i64,
    pub text: String,
    pub owner: Option<String>,        // como foi dito; nao e FK para pessoa
    pub due_hint: Option<String>,     // "amanha", "sexta"; resolvido so na confirmacao
    pub confidence: Confidence,       // High | Medium | Low
    pub status: InsightStatus,        // Proposed | Accepted | Dismissed
    pub created_task_id: Option<TaskId>,
    pub created_reminder_id: Option<ReminderId>,
}
```

**Oito tabelas foram consideradas e recusadas**, pelo argumento textual da ADR-025, que
enfrentou a mesma escolha e escolheu três tabelas em vez de nove:

> Anexo, artifact, citação e execução de ferramenta entram como `kind` de parte, com payload
> JSON validado pelo domínio, e só viram tabela própria quando precisarem de lifecycle ou
> consulta própria.

Nenhum dos oito tipos tem lifecycle próprio nem consulta própria. `MeetingInsight` sozinho
preserva a capacidade de promover cada um depois.

`due_hint` guarda o texto natural e **não** um instante. Resolver "amanhã" no momento da
análise congelaria uma interpretação; resolver no momento da confirmação põe a interpretação
na tela, que é o que o `UX-PRINCIPLES` §19 pede.

### 7.5 `MeetingEvidence` — referência, nunca cópia

```rust
pub struct MeetingEvidence {
    pub insight_id: InsightId,
    pub segment_id: SegmentId,
    pub seq: i64,
    pub char_start: Option<u32>,   // recorte dentro do texto do segmento
    pub char_end: Option<u32>,
}
```

O texto da citação **não é guardado**. Ele é o texto do segmento. Isso atende ao pedido
explícito do brief ("reference ao transcript é preferível") e compra algo melhor: a evidência
não pode divergir da transcrição, porque ela **é** a transcrição.

### 7.6 Relações e integridade

| Origem | Destino | Cardinalidade | V1 |
|---|---|---|---|
| Meeting | Project | zero ou um | Sim |
| Meeting | TranscriptSegment | um para muitos | Sim |
| Meeting | MeetingAnalysis | zero ou um | Sim |
| MeetingInsight | Task | zero ou um, criada por confirmação | Sim |
| MeetingInsight | Reminder | zero ou um, criado por confirmação | Sim |
| MeetingInsight | TranscriptSegment | muitos para muitos, via Evidence | Sim |
| Meeting | CalendarEvent | — | Não: `Event` não existe (§0.3) |

Arquivar um Project **não** arquiva suas Meetings, pelo mesmo princípio que já vale para Tasks
(`CORE-FOUNDATION.md` §3.4). Apagar uma Task criada a partir de um Insight não apaga o Insight:
ele volta a `Proposed` com o vínculo marcado como perdido, exatamente como o Attention System
trata target órfão.

---

## 8. Audio Storage

### 8.1 Layout

```text
%APPDATA%/…/m-os/meetings/<meeting-id>/
├── session.json          escrito no inicio, atualizado em mudanca de estado
├── mic/
│   ├── 000000.pcm        10 s cada
│   ├── 000001.pcm
│   └── …
└── system/
    ├── 000000.pcm
    └── …
```

### 8.2 Por que PCM cru, e não WAV

WAV foi a primeira escolha e foi recusada por um motivo específico: **o cabeçalho RIFF carrega
o tamanho dos dados, e ele só é conhecido no fechamento.** Um processo que morre no meio deixa
um arquivo cujo cabeçalho afirma um tamanho que o arquivo não tem — ou seja, um arquivo que
**mente sobre si mesmo** exatamente no cenário em que precisamos confiar nele.

Consertar isso é possível (recalcular o tamanho pelo tamanho do arquivo), mas seria escrever
código de reparo para um problema que não precisamos ter. PCM cru não tem cabeçalho para
mentir: o formato mora uma vez em `session.json`, e um chunk truncado é simplesmente truncado
até o último frame inteiro.

Quando o usuário quiser ouvir, o `ffmpeg` já presente na máquina monta um WAV/M4A sob demanda.

### 8.3 Por que 10 segundos

O chunk define a janela de perda. 10 s a 16 kHz mono i16 = **320 kB**, e o arquivo é fechado e
liberado ao sistema a cada rotação. Uma reunião de 1h20 dá 480 chunks por canal — número que o
filesystem lida sem cerimônia e que a recuperação varre em milissegundos.

Chunks de 1 s dariam 4.800 arquivos por canal para ganhar 9 segundos de janela; chunks de 60 s
trocariam um minuto de gravação por menos arquivos. 10 s é o joelho da curva.

### 8.4 `session.json`

```json
{
  "meetingId": "0198...",
  "startedAt": "2026-08-18T17:02:11Z",
  "format":  { "sampleRate": 16000, "channels": 1, "sample": "i16le" },
  "chunkMs": 10000,
  "timing":  { "mic": "events", "system": "polling" },
  "channels": {
    "mic":    { "device": "Headset Microphone (Jabra)", "opened": true },
    "system": { "device": "Speakers (Realtek)", "opened": true, "keepAlive": true }
  }
}
```

Escrito **de forma atômica** (arquivo temporário + rename), pelo mesmo motivo que a §8.2
recusou WAV: um `session.json` escrito pela metade seria o único arquivo cuja corrupção
impediria a recuperação de tudo o mais.

**O diretório é a fonte de verdade da duração.** `session.json` diz como ler os bytes; quantos
bytes existem é uma pergunta para o filesystem. Nenhum contador é mantido em disco durante a
gravação, porque um contador é mais uma coisa que pode divergir.

---

## 9. Crash Recovery

### 9.1 O que é detectado

Na abertura do M/OS, `MeetingService::reconcile_on_open()` procura Meetings em `Recording` ou
`Stopping`. Uma linha nesses estados com o processo recém-nascido significa, necessariamente,
que o processo anterior morreu sem terminar — não existe outro caminho.

Para cada uma:

1. varre `mic/` e `system/`, soma os bytes, trunca o último chunk de cada canal ao último frame
   inteiro;
2. calcula a duração recuperada a partir dos frames, não do relógio;
3. move a Meeting para `Interrupted` com essa duração;
4. **não apaga nada.**

### 9.2 O que o usuário vê

```text
  Reuniao interrompida
  18 de agosto, 14:02

  Recuperado    1h18m
  Microfone     completo
  Sistema       completo

  [Processar]              [Descartar]
```

`[Descartar]` é ação secundária, e apaga o áudio depois de uma confirmação — `UX-PRINCIPLES`
§54 exige que destruir pareça diferente de arquivar.

A instrução do brief é literal e é honrada: *"Não simplesmente apagar arquivos temporários."*
Uma limpeza automática de "arquivos órfãos" na inicialização é exatamente o comportamento que
transformaria 1h18 de reunião em zero sem ninguém perceber. **Não existe rotina que apague
áudio de uma Meeting que o usuário não decidiu descartar.**

### 9.3 O caso em que a queda foi no meio do processamento

Meetings em `Transcribing` ou `Analyzing` na abertura voltam ao repouso anterior (`Recorded` /
`Transcribed`) e são reprocessadas sob demanda. É o mesmo tratamento que
`settle_unfinished_messages` já faz com mensagens do Hermes que ficaram `streaming` — e pelo
mesmo motivo registrado lá: sem isso, elas voltariam eternamente em curso.

---

## 10. Transcription

### 10.1 A porta

```rust
pub trait TranscriptionProvider: Send + Sync {
    fn name(&self) -> &str;
    fn transcribe(
        &self,
        audio: ChannelAudio,          // 16 kHz mono i16, ja concatenado
        channel: Channel,
        language: Option<&str>,
        progress: &dyn Fn(f32),
    ) -> Result<Vec<RawSegment>, TranscriptionError>;
}
```

Meeting não conhece provider. Provider não conhece Meeting. É a mesma fronteira que `ports.rs`
já desenha para persistência.

### 10.2 Local primeiro

**whisper.cpp, como sidecar.** A decisão D-6 está tomada, e o que decidiu foi o terreno.

| Rota | A favor | Contra |
|---|---|---|
| `whisper-rs` como crate | um processo, sem IPC, progresso nativo | traz build de C++ com `cmake` para dentro do `cargo build` de todo mundo |
| **`whisper-cli.exe` como sidecar** ✔ | o `cargo build` continua puro Rust; o binário é trocável sem recompilar | IPC, parsing de saída, e um binário a assinar e distribuir |

O `SETUP-MAQUINA.md` §2 documenta que a máquina principal já perdeu uma tarde porque
faltava `windres`, e §4 registra que `cargo test -p mos-desktop` **não roda lá** por
incompatibilidade de DLL entre mingws. Acrescentar um build de C++ a essa cadeia é a
próxima tarde perdida — e ela cairia sobre o `cargo build` de quem só quisesse mexer numa
tela.

O sidecar também transforma a D-7 numa escolha do **usuário** em vez de uma escolha de
build: trocar de CPU para cuBLAS é trocar um binário em Settings, não recompilar o M/OS.

### D-7, medida em 2026-08-19

whisper.cpp **v1.9.2**, build `whisper-bin-x64` (CPU), modelo
`ggml-large-v3-turbo-q5_0` (547 MB), áudio pt-BR de 28,3 s:

| Threads | Tempo | Velocidade |
|---:|---:|---:|
| 8 | 7.682 ms | **3,7× tempo real** |
| 16 | 5.051 ms | **5,6× tempo real** |

**A conta que importa, e ela não é confortável:** uma reunião de 60 minutos produz
**120 minutos de áudio**, porque os dois canais são transcritos separadamente. A 5,6×,
isso é **cerca de 21 minutos de processamento**. Aceitável para pós-reunião, longe de
instantâneo — e é um número que precisa aparecer na interface como progresso, não como
espera silenciosa.

Qualidade em pt-BR: boa. Termos técnicos e nomes próprios saem corretos, com acentuação
restaurada, e os limites de segmento caem em fronteiras de frase.

**O caminho para acelerar existe e não exige o CUDA Toolkit.** O release publica
`whisper-cublas-12.4.0-bin-x64.zip` (639 MB), que **empacota as DLLs do CUDA** — a RTX 5070
Ti seria usada trocando o binário em Settings, sem instalar nada e sem recompilar o M/OS.
Não foi medido, e entra como enhancement quando os 21 minutos incomodarem.

Os áudios são **pt-BR**, então o modelo precisa ser multilíngue — as variantes `.en` estão
fora por construção.

### 10.3 Nuvem como possibilidade, não como plano

A porta permite um `OpenAITranscriptionProvider` no futuro. Ele **não** será construído na V1,
e a razão não é preguiça: mandar o áudio para fora contradiz a preferência declarada e, pior,
criaria um caminho em que a feature *funciona* sem o local funcionar — e um caminho que
funciona é um caminho que vira o padrão.

### 10.4 Speakers na V1: a regra determinística ganha

```text
canal MIC     → o usuario local        → "VOCE"
canal SYSTEM  → participantes remotos  → "REMOTO"
```

Sem diarização. Esta é a aplicação direta da regra de decisão do brief — entre IA e regra
determinística confiável, usa-se a determinística — e ela entrega o valor que importa: separar
"o que EU prometi" de "o que outros disseram" com **certeza**, não com probabilidade.

Diarização dentro do canal SYSTEM (quem, entre os remotos, falou) é enhancement da V2. Ela
nunca pode alterar a atribuição de canal: se um diarizador disser que um trecho do MIC é de
outra pessoa, ele está errado por construção, porque aquele microfone é o do usuário.

### 10.5 Intercalação

Cada canal é transcrito independentemente. Os dois conjuntos de segmentos são ordenados por
`start_ms` num único `seq`. Empate resolve com MIC primeiro — arbitrário, mas determinístico,
que é o que uma ordenação precisa ser.

---

## 11. Hermes Analysis

### 11.1 Nenhum agente novo

`ADR-024` é categórica: Hermes é superfície, não segundo agente. A análise usa `session.create`
e `prompt.submit` do gateway já contratado em `HERMES-GATEWAY-CONTRACT.md`, pelo túnel que já
existe. **Zero infraestrutura nova. Zero porta nova. Zero credencial nova.**

### 11.2 A análise não é uma conversa

A análise roda numa sessão própria e efêmera, e **não** vira uma `Conversation` na lista do
Hermes. Despejar uma transcrição de uma hora na thread do usuário seria ruído, e a resposta —
um bloco estruturado — não é feita para ser lida como prosa.

"Ask Jarvis about this meeting" é outra coisa (§15.3): ali sim nasce uma `Conversation` normal,
com a reunião injetada como contexto e o chip da ADR-027 visível.

### 11.3 O consentimento e o registro

Decisão do proprietário (D-A): consentimento **uma vez**, análise automática depois.

Isso não afrouxa a ADR-027, que exige *"chip visível e registro do que foi enviado"*. Ela é
cumprida assim:

- antes da primeira análise da vida, uma tela explica em português o que sai da máquina, e nada
  acontece sem ela;
- cada análise grava um `context_ref` com **o que efetivamente foi enviado** — contagem de
  segmentos, intervalo de tempo, tamanho em caracteres e número de janelas;
- Settings mostra o consentimento e permite revogá-lo. Revogado, reuniões param em
  `Transcribed` e a análise vira ação manual;
- a página da reunião mostra, de forma permanente, que a análise saiu para o Hermes e quando.

A pergunta *"o que exatamente foi para a VPS?"* tem resposta **depois** do envio, não só antes.
Esse é o teste que a ADR-027 define, e ele é atendido.

### 11.4 Orçamento, e o corte que não é silencioso

A ADR-028 registra a limitação que herdamos: *"o contexto é fixo no envio, e essa limitação é
real e conhecida (...) o Context Service precisa orçar o que envia, porque não há segunda
chance."*

Uma reunião de uma hora dá algo em torno de 20 mil caracteres úteis por canal. Quando a
transcrição couber no orçamento, ela vai inteira, numa janela. Quando não couber:

- é dividida em janelas com sobreposição, cada uma analisada separadamente;
- uma passada final consolida, recebendo **os resultados das janelas** e não a transcrição;
- o número de janelas é gravado em `MeetingAnalysis.windows` e **aparece na interface**.

Isto é a regra do "no silent caps": um corte de cobertura que não aparece na tela lê-se como
"cobriu tudo" quando não cobriu.

---

## 12. Structured Output Contract

### 12.1 A forma

O modelo responde em texto, e dentro dele um bloco cercado — o mesmo mecanismo que
`SPEC-ACOES-ENTRE-APPS.md` já usa para ações, e pelo mesmo motivo: o protocolo do gateway **não
tem registro de ferramenta do lado do cliente**, verificado em `tui_gateway/server.py`.

````text
```mos-meeting
{
  "summary": "…",
  "topics": ["…"],
  "items": [
    {
      "kind": "my_action",
      "text": "Finalizar a apresentacao",
      "owner": "Matheus",
      "dueHint": "amanha",
      "confidence": "high",
      "evidence": [{ "segment": "0198c4…", "charStart": 0, "charEnd": 34 }]
    }
  ]
}
```
````

`kind` ∈ `decision | my_action | other_action | deadline | follow_up | open_question | risk`.
`confidence` ∈ `high | medium | low`.

### 12.2 Validação: recusa, não conserto

A regra vem inteira da spec de ações, §3 passo 4:

> **Argumento fora do esquema = proposta recusada, não corrigida.**

Aplicada aqui, ela produz quatro recusas duras, em `mos-core`:

1. `kind` desconhecido → item descartado, contagem registrada.
2. `segment` que não existe na transcrição daquela reunião → **evidência descartada**. É a
   defesa contra o modelo inventar uma citação, e é barata: um `HashSet` de ids reais.
3. `charStart`/`charEnd` fora do texto do segmento → recorte descartado, segmento mantido.
4. JSON malformado ou bloco ausente → uma reprompt com o erro concreto; na segunda falha,
   `Failed{analysis}`, com a resposta crua preservada para diagnóstico e **fora dos logs**.

### 12.2.1 O que a primeira análise real ensinou

Medido em 2026-08-19, contra o Hermes de produção.

A classificação veio **excelente de primeira**: 8 itens, `my_action` separado de
`other_action` corretamente, `decision` para o que foi fechado, `open_question` para a
dúvida em aberto, e `confidence: low` exatamente na frase que começava com "talvez".

E **8 de 8 evidências foram recusadas.** O modelo copiou a linha inteira no campo `segment`:

```json
"segment": "[01a0186f-…] 00:00:11 VOCE — Sobre o orcamento, acho que precisamos…"
```

A culpa era do prompt, que dizia *"um `segment` COPIADO das linhas acima"* — convite
literal para copiar a linha. Duas correções saíram disso:

**O prompt passou a mostrar o formato do id** e a dizer, com todas as letras, que é só o
que está entre colchetes.

**O leitor passou a extrair o UUID de dentro de um texto maior** — e isso **não viola** a
regra de "recusado, não corrigido". A regra existe para impedir que um argumento inválido
seja adivinhado; aqui nada é adivinhado. O id extraído continua sendo conferido contra os
segmentos reais, e um id inexistente continua caindo e sendo contado. Muda a forma de ler,
não o que é aceito. **A defesa contra citação inventada nunca foi o formato do campo — é o
mapa dos ids reais.**

Depois das duas: **7 itens, zero recusas, toda evidência resolvendo para uma fala que
existe.**

### 12.3 As regras determinísticas que sobrepõem o modelo

> **Item de ação sem evidência válida não pode virar Task com um clique.**

Ele aparece, marcado como sem evidência, e a criação de Task exige abrir e editar. Isso não é
desconfiança do modelo: é a promessa da §1 em forma executável — o Meeting Agent não apresenta
inferência como fato sem proveniência.

> **`confidence: low` nunca entra numa criação em lote.**

"Talvez a gente possa revisar isso amanhã" pode virar Task. Não vira Task junto com outras seis
num único clique. É o exemplo literal do brief, resolvido por regra e não por prompt.

---

## 13. Tasks / Projects Integration

### 13.1 O Meeting Agent não escreve em Tasks

```text
MeetingInsight (Proposed)
      │  usuario clica [Criar Task]
      ▼
ActionPreview  ── a mesma peca que o Hermes ja usa
      │  confirma
      ▼
WorkService::create_task  ── o mesmo servico que a interface usa
      │
      ▼
Task + recibo + Undo de 5 s (ADR-035: desfazer arquiva, nunca apaga)
```

Nenhuma linha nova de execução. `meeting.accept_insight` entra em `functions.rs` com o mesmo
`risk`/`confirmation` das criações locais existentes.

**Entregue em 2026-08-19, com uma correção de desenho.** A `MeetingRepository::accept_insight`
faz os três — criar Task, criar Reminder, ligar o item — **numa transação só**, reusando os
mesmos `insert_task` e `insert_reminder` que os repositórios de Task e de Attention já usavam.
Fazê-los em sequência deixaria um instante em que a Task existe e o lembrete dela não, e uma
queda ali deixaria o compromisso sem aviso — que é exatamente o modo de falhar que esta
feature existe para não ter.

Três guardas que os testes prendem:

- **aceitar duas vezes é recusado**, com a checagem dentro da transação. Sem ela, duas
  confirmações rápidas criariam duas Tasks para o mesmo compromisso e a segunda deixaria a
  primeira órfã;
- **um lembrete no passado derruba a aceitação inteira**, e nenhuma Task fica. A validação
  acontece no domínio antes da transação;
- **o corpo do Reminder cita a reunião**, e não a Task. Quando ele tocar amanhã às 9h, "de
  onde veio isto?" precisa ter resposta sem abrir mais nada.

O desfazer é um `UndoStep::UndoMeetingInsight` único, e a ordem dentro dele é deliberada:
cancela o Reminder **primeiro**. Um lembrete que disparasse no meio do desfazer avisaria
sobre uma Task que a pessoa acabou de dizer que não queria.

### 13.2 Preview antes do lote

`[Criar 3 Tasks]` mostra as três antes, com título, Project e prazo editáveis — porque, como a
spec de ações corrigiu na implementação:

> O risco classifica a **consequência da ação**. O preview responde a outra coisa: a
> **incerteza da interpretação**. Quem clica "Criar Task" na interface escolheu aquilo; quem
> falou uma frase pode ter sido mal entendido.

Uma reunião é o caso extremo disso: ninguém escolheu nada, alguém só falou.

### 13.3 Project

`Meeting.project_id`, zero ou um, editável a qualquer momento. Quando presente:

- a Task criada nasce naquele Project;
- a reunião aparece no contexto do Project;
- o lançamento de tempo (§14.3) sabe onde lançar.

O vínculo pode ser **sugerido** pela análise (o nome do Project aparece na transcrição), e
sugestão nunca é aplicação: ela chega como um campo pré-preenchido no preview.

---

## 14. Calendar / Attention Integration

### 14.1 Calendar

`CalendarKind` ganha `Meeting`. Ele é retrospectivo, e uma reunião que aconteceu é exatamente o
material dele. Nenhuma dependência é criada: `calendar.rs` continua compondo o que já sabe.

Relacionar Meeting a **Event** fica fora, porque `Event` não existe (§0.3). O campo
`calendarEventId` do brief **não é criado** — uma coluna que só pode ser nula é uma promessa sem
lastro, e a ADR-034 já estabeleceu a doutrina: *"um anel bonito preenchido com número inventado
é pior que a ausência."*

### 14.2 Attention

Decisão do proprietário (D-C): **prazo vira Reminder, não `Task.due_at`.**

```text
  SUA ACAO

  Finalizar a apresentacao

  Prazo      amanha, 09:00      ← interpretado de dueHint, editavel
  Evidencia  14:04              ← clicavel

  [ Criar Task ]  [ + Lembrete 09:00 ]
```

Marcados os dois, uma transação cria a Task, cria o Reminder com
`ReminderTarget::Task(task_id)` e liga o Insight aos dois. O agendador que já existe entrega.

`ReminderTarget` ganha um sétimo braço, `Meeting(MeetingId)`, para o caso "me lembra de revisar
essa reunião". Custo: uma migration e uma linha no `match` — que é exatamente a consequência que
a ADR-012 aceitou ao recusar tabela genérica de arestas.

### 14.3 Tempo

Decisão do proprietário (D-D): **oferece lançar, nunca lança sozinho.**

Ao parar, se a reunião tem Project e houve duração relevante:

```text
  Lancar 1h12 em NexoDoc como reuniao?        [ Lancar ]  [ Agora nao ]
```

`ActivityType::Meeting` já existe no domínio. `time.record` já é risco **médio com confirmação
explícita**, com a justificativa escrita em `functions.rs`:

> Encerrar e lançar escrevem hora COBRAVEL a partir de uma frase.

O que vale mais aqui, porque a "frase" é uma gravação que o sistema decidiu sozinho que durou
1h12. E a ADR-037 já fixou a doutrina para observação: *"observação não vira hora sozinha."*

### 14.4 O que isso desbloqueia, e que não construímos agora

Com Meeting no sistema, o Smart Snooze "após a reunião" do `ATTENTION-SYSTEM.md` §13.1 deixa de
estar bloqueado. **Não é escopo desta V1.** Fica registrado aqui para que quem for implementá-lo
encontre a âncora em vez de reabrir a investigação.

---

## 15. Search

### 15.1 Global: a reunião, não os segmentos

`SearchItem` ganha `Meeting { meeting, project }`. O FTS global indexa:

- título da reunião;
- resumo;
- texto dos Insights.

**Não indexa segmentos de transcrição no índice global.** A instrução do brief é explícita, e a
razão é aritmética: uma reunião de uma hora tem ~600 segmentos, e três reuniões dominariam
qualquer busca por qualquer palavra comum.

### 15.2 Transcrição: índice próprio, consulta própria

Um segundo FTS, `meeting_transcript_fts`, serve dois consumidores e nenhum outro:

1. a busca **dentro** de uma reunião, na view de transcrição;
2. a pergunta atravessando reuniões — *"quando falamos sobre usar Hermes no M/OS?"* — que entra
   na Search global **promovendo a Meeting**, com o trecho como snippet, deduplicada por
   reunião. Uma reunião, um resultado, mesmo que a palavra apareça 40 vezes.

Os dois índices são reconstruíveis e nenhum é fonte de verdade, como manda o
`CORE-FOUNDATION.md` §7.

### 15.3 Jarvis: a verdade sobre as read tools

O brief pede `mos_search_meetings`, `mos_get_meeting`, `mos_get_meeting_transcript`,
`mos_get_meeting_actions`, `mos_get_meeting_decisions` e `mos_get_meeting_commitments`.

**Elas não podem existir como ferramentas na V1, e isso precisa ser dito em vez de contornado.**
A ADR-028 registra a razão, verificada no código do gateway:

> O protocolo WebSocket do gateway **não expõe registro de ferramenta do lado do cliente**
> (...) Não há como o agente chamar o M/OS de volta no meio do turno sem MCP ou fork.

O que existe é o caminho que a mesma ADR escolheu: **injeção de contexto**. Então os seis nomes
viram a **forma da projeção injetada**, não chamadas:

| Nome pedido | Como existe na V1 |
|---|---|
| `mos_search_meetings` | o usuário anexa reuniões pelo `@`, e a Search resolve quais |
| `mos_get_meeting` | projeção "cabeçalho + resumo" no bloco injetado |
| `mos_get_meeting_actions` / `_decisions` | projeções dos Insights por `kind` |
| `mos_get_meeting_transcript` | janela orçada da transcrição, com aviso quando cortada |
| `mos_get_meeting_commitments` | consulta local sobre Insights `my_action` ainda `Proposed` ou com Task aberta — **respondida pelo M/OS**, não pelo modelo |

O último merece nota: *"quais compromissos de reuniões eu ainda não concluí?"* é uma query SQL,
não uma pergunta de linguagem. Responder por SQL é a aplicação da regra do brief — onde a regra
determinística serve, ela ganha da IA.

Quando a ADR de MCP local existir, os seis viram ferramentas de verdade sem que o domínio mude:
as projeções já terão a forma certa.

---

## 16. Privacy

### 16.1 O áudio

- fica em `%APPDATA%`, sob as mesmas ACLs do banco (`ARCHITECTURE.md` §15.1);
- **nunca** sai da máquina — nem para o Hermes, nem para lugar nenhum;
- padrão de retenção: **apagar após processamento bem-sucedido**, conforme preferência
  declarada. "Bem-sucedido" significa `Ready` ou `Transcribed` com análise recusada por escolha
  — nunca `Failed`;
- alternativas por reunião: manter 24 h, ou manter até decisão manual;
- a limpeza roda na abertura e após cada processamento, e é **idempotente e conservadora**: ela
  só apaga diretório de Meeting cujo `status` autoriza. Diretório sem linha no banco é
  **relatado, nunca apagado** — ver §9.2.

### 16.2 A transcrição

Fica no banco, como dado da reunião, e entra em backup e export. `ARCHITECTURE.md` §16 já avisa
que ambos podem conter dado pessoal em texto claro; o aviso passa a cobrir mais coisa, como já
aconteceu quando conversas entraram (ADR-025).

Ela **sai da máquina** na análise, sob o consentimento da §11.3.

### 16.3 Os logs

Regra dura, sem exceção:

> **Nenhum log técnico contém texto de transcrição, texto de Insight, nome de participante ou
> bytes de áudio.**

O que pode ser logado: `meeting_id`, estado, durações, contagem de segmentos, contagem de
frames, código de erro, nome de dispositivo. O que não pode: qualquer coisa que uma pessoa tenha
dito. É a mesma linha que `ARCHITECTURE.md` §18 já traça para Captures, aplicada a um conteúdo
mais sensível.

### 16.4 Backup

| Dado | Em backup? | Por quê |
|---|---|---|
| Meeting, transcrição, análise, Insights, relações | **Sim** | é o dado da reunião |
| Chunks de áudio | **Não** | é temporário por política, é grande, e é apagado por padrão |

Consequência honesta, e ela precisa estar escrita: **restaurar um backup não devolve o áudio de
uma reunião que ainda não foi processada.** Um backup feito com uma reunião em `Recorded`
restaura a linha e não os bytes; a reunião volta como `Failed{audio}` com a causa dita. É
preferível a inchar todo backup com centenas de MB de dado que a própria política apaga em
seguida.

---

## 17. Consent

### 17.1 Uma vez, e sério

Antes da primeira gravação da vida, e só dela:

```text
  Meeting Notes grava audio

  Enquanto estiver gravando, o M/OS captura o seu microfone e o
  audio que sai pelos alto-falantes — o que inclui a voz das outras
  pessoas na chamada.

  O audio fica neste computador e e apagado depois de processado.
  A transcricao e feita aqui. Para a analise, ela e enviada ao
  Hermes; voce pode desligar isso em Settings.

  Obter o consentimento dos outros participantes, quando necessario,
  e responsabilidade sua.

  [ Entendi, gravar ]                              [ Cancelar ]
```

Uma vez. Não a cada reunião — `UX-PRINCIPLES` §21 é explícito: *"Confirmações constantes tornam
o sistema cansativo e ensinam o usuário a clicar sem ler."* Uma tela jurídica repetida seria
pior que nenhuma, porque ninguém a leria na décima vez.

### 17.2 O que substitui a confirmação repetida

Estado visível, sempre:

- barra de gravação persistente na janela, com ponto vermelho e cronômetro;
- ícone e tooltip no tray, com o cronômetro;
- ao fechar a janela durante uma gravação, o tray mostra que ela continua.

**Não existe caminho de código que inicie gravação sem o usuário ter clicado.** Não há gravação
agendada, não há gravação por detecção, não há gravação por atalho global. A V1 não tem essa
capacidade — e essa é a garantia mais forte que se pode dar.

---

## 18. Security

- **Nenhuma superfície de rede nova.** A análise usa o túnel SSH existente para
  `127.0.0.1:9119`. Nenhuma porta, nenhum servidor, nenhuma credencial nova.
- **Nenhuma capability nova no renderer.** A captura é inteiramente Rust; o WebView continua sem
  acesso a filesystem, a dispositivos e a rede, como `ARCHITECTURE.md` §15.3 exige.
  Notavelmente, **`getUserMedia` não é usado** — a instrução do brief ("não utilize hacks
  baseados em browser") coincide com o modelo de segurança.
- Paths de áudio são derivados do `MeetingId` e validados como filhos do diretório de dados.
  Nenhum path vem do renderer.
- Se o binário do transcritor local for sidecar, ele entra na cadeia de assinatura do
  instalador, como qualquer outro executável distribuído.
- Um bloco `mos-meeting` vindo do modelo é **dado não confiável**. Ele é parseado por
  `serde_json` em tipos fechados, e `char_start`/`char_end` são checados contra os limites reais
  do segmento antes de qualquer fatiamento — um índice fora do texto seria um panic em Rust, e
  um panic dentro de um comando derruba o turno.

---

## 19. Performance

Orçamentos, a validar no spike. São orçamentos de engenharia, não garantias de produto — mesma
ressalva que `ARCHITECTURE.md` §12 faz.

| Item | Orçamento | Medido no spike (2026-08-18) |
|---|---|---|
| CPU durante gravação, processo inteiro | < 2% | **0,29 %** de um núcleo em 15 min |
| RSS adicional durante gravação | < 60 MB | **11 MB** de pico |
| Escrita em disco | ~64 kB/s (dois canais) | **61 kB/s** (55,0 MB em 15 min) |
| Latência do clique em Stop até estado `Recorded` | < 500 ms | não medido: o spike não tem estado |
| Eventos para o renderer | 1/s, payload < 300 bytes | não medido: o spike não tem renderer |
| Drift entre MIC e SYSTEM ao fim de 60 min | < 200 ms | **20 ms em 15 min**; 1 ms contra o relógio do dispositivo |

Regras que sustentam os números:

- as threads de captura são threads de SO dedicadas, registradas com
  `AvSetMmThreadCharacteristics("Pro Audio")`, e **não** tarefas do runtime assíncrono
  compartilhado — o mesmo cuidado que `monitor.rs` já toma ao não varrer processos no fio da
  interface;
- a transcrição roda fora dessas threads e pode usar todos os núcleos, porque acontece depois;
- nenhuma consulta ao banco no caminho do áudio (garantido por compilação, §4.2);
- nada de waveform, nada de polling agressivo, nada de PCM no renderer.

---

## 20. Failure Modes

A separação abaixo é o requisito central desta seção: **o usuário precisa distinguir "perdi a
gravação" de "a gravação está segura e outra coisa falhou".** São situações que pedem respostas
opostas.

| Falha | O que o sistema faz | O que o usuário lê |
|---|---|---|
| Microfone desconecta | uma tentativa de religar no novo default; falhando, canal → `Lost{at_ms}`, o outro continua | *"Microfone desconectado às 32:10. O áudio do sistema continua gravando."* |
| Áudio do sistema para | idem, espelhado | *"A captura do áudio do sistema parou. O microfone continua gravando."* |
| Os dois caem | gravação para, tudo até ali é preservado, Meeting → `Recorded` | *"A gravação parou. 41 minutos foram preservados."* |
| Default device muda | evento de transição gravado no `session.json`; religa se conseguir | *"Dispositivo de saída mudou para Alto-falantes."* |
| Disco cheio | gravação para imediatamente; nada é sobrescrito | *"Sem espaço em disco. 1h04 foi preservada."* |
| Transcrição falha | `Failed{transcription}`, áudio **não** apagado | *"A gravação está segura. Tentar transcrever de novo."* |
| Hermes offline | Meeting fica em `Transcribed`. **Não é falha.** | *"Transcrição pronta. A análise continua quando o Hermes voltar."* |
| Bloco `mos-meeting` inválido | uma reprompt; depois `Failed{analysis}` | *"O Hermes respondeu num formato que não deu para ler. Tentar de novo."* |
| Queda do processo | §9 | *"Reunião interrompida — 1h18 recuperada."* |
| Loopback sem pacotes | canal → `Unavailable` no início, ou `Lost` no meio | *"Não foi possível capturar o áudio do sistema."* |

Duas regras atravessam a tabela:

1. **Nunca fingir que continua gravando.** Um canal caído aparece como caído em menos de dois
   segundos. É a instrução literal do brief, e é a única coisa que torna a barra de gravação
   confiável.
2. **Falha de uma etapa nunca destrói o insumo da anterior.** Transcrição que falha não apaga
   áudio; análise que falha não apaga transcrição.

---

## 21. Testing

### 21.1 A restrição que decide onde o teste mora

`SETUP-MAQUINA.md` §4 registra que **`cargo test -p mos-desktop` não roda** na máquina
principal, e conclui:

> a lógica precisa morar em `mos-core` ou `mos-storage-sqlite`, onde os testes rodam. O crate do
> desktop deve ficar com casca fina — comandos, laços e adaptação — porque teste que não roda
> não protege nada.

Portanto: máquina de estados, parser do contrato, validação de evidência, intercalação de
segmentos, política de retenção e cálculo de duração recuperada moram em `mos-core`. O
`meeting.rs` do desktop é casca.

### 21.2 O que é testado

**`mos-core`**
- máquina de estados: todas as transições válidas; todas as inválidas recusadas
- `Recording → Interrupted → Recorded` e `→ Cancelled`
- `Failed{stage}` volta ao repouso correto no retry
- parser de `mos-meeting`: válido; `kind` desconhecido; JSON quebrado; bloco ausente; dois blocos
- evidência com `segment` inexistente é descartada, e o Insight sobrevive
- `charStart`/`charEnd` fora do texto não fatia e não entra em pânico
- Insight sem evidência não é elegível a criação em lote
- `confidence: low` fora do lote
- intercalação de dois canais, incluindo empate de `start_ms`
- duração a partir de frames, com chunk final truncado
- política de retenção: o que autoriza apagar, e o que nunca autoriza

**`mos-storage-sqlite`**
- migration `0017` sobre banco vazio e sobre banco na `0016`
- criação de Task e Reminder a partir de Insight, numa transação; falha em qualquer etapa não
  deixa nada pela metade
- Task apagada deixa o Insight órfão e vivo
- Meeting entra na Search global; segmento **não** entra
- transcrição encontra por FTS próprio e deduplica por reunião
- rebuild dos dois índices
- `reconcile_on_open` marca `Recording` como `Interrupted`

**`mos-audio`** (partes sem WASAPI)
- rotação de chunk, nome e ordem
- chunk truncado é lido até o último frame inteiro
- `session.json` escrito atomicamente; escrita interrompida não corrompe o anterior

### 21.3 O que a Fase 2 encontrou

Três defeitos que só apareceram porque os testes foram escritos junto, e não depois.

**Um `CHECK` que não checava nada.** A restrição que garante que uma reunião `failed`
sempre carregue o estágio da falha estava escrita assim:

```sql
(status = 'failed' AND failed_stage IN ('audio', 'transcription', 'analysis'))
OR (status <> 'failed' AND failed_stage IS NULL)
```

Com `status = 'failed'` e `failed_stage = NULL`, o `IN` vale NULL, o primeiro ramo vira
`TRUE AND NULL` = NULL, o segundo vira FALSE, e `NULL OR FALSE` = NULL. **No SQLite, um
CHECK que avalia para NULL passa.** A guarda existia e não guardava — e o único jeito de
descobrir isso era um teste que tentasse gravar a linha inválida. O conserto é um
`failed_stage IS NOT NULL` explícito, que parece redundante e não é.

**Um deadlock no caminho da análise.** `replace_analysis` travava o mutex da conexão,
abria a transação, comitava, e travava de novo para reindexar. `Mutex` do std não é
reentrante, e o segundo `lock()` no mesmo escopo trava para sempre. O conserto move a
reindexação para dentro da transação — que é onde ela deveria estar de qualquer forma,
porque `ARCHITECTURE.md` §11.2 exige entidade e projeção na mesma transação.

**`snippet()` do FTS5 não vale em contexto agregado.** A busca na transcrição precisa de
`GROUP BY` para deduplicar por reunião, e o SQLite recusa a combinação com
`unable to use function snippet in the requested context`. A troca acabou sendo melhor
que a intenção original: o trecho passa a vir do próprio segmento, e uma fala inteira é
mais contexto que um fragmento cortado no meio de uma frase.

### 21.4 O que a ponte para o áudio encontrou

Um defeito, e ele é do tipo que nenhum teste de unidade pegaria.

**O manifesto mentia sobre o keep-alive.** Ao parar, `Recording::stop` reescreve o
`session.json` com o que as threads de captura observaram — dispositivo, timing, formato
efetivo. Mas a thread de captura **não sabe que o keep-alive existe**: ele roda numa
thread própria, e ela nem precisa saber. O merge do fim sobrescrevia `keep_alive: true`
por `false`, e o arquivo passava a negar a única coisa que diz se a linha do tempo do
canal remoto é confiável.

Só o teste contra hardware real podia pegar isso, porque só ele produz um manifesto de
fim. O conserto separa as autoridades: a thread manda no que observou, quem abriu a
gravação manda no keep-alive, e o merge preserva os dois.

O mesmo teste também obriga o `started_at` a sobreviver ao merge — ele estava vazio, e um
manifesto sem instante de início é um manifesto que não datava a gravação que descreve.

### 21.5 Teste contra hardware

`crates/mos-audio/tests/hardware.rs`, marcado `#[ignore]` porque abre dispositivos reais:

```powershell
cargo test -p mos-audio --test hardware -- --ignored --nocapture
```

Ele grava 12 segundos — o mínimo que cruza uma fronteira de chunk — e verifica que a
duração cresce durante a gravação, que os dois canais produziram áudio, que a rotação
aconteceu, que nenhum arquivo termina no meio de um frame, que os canais divergem menos
de 500 ms e que o manifesto conta a verdade.

Medido em 2026-08-19: 2 chunks por canal, **10 ms de divergência**, 0 bytes soltos,
`timing: Events`, `16000/1/i16` nos dois.

### 21.6 Teste com modelo real

`crates/mos-transcribe/tests/real_model.rs`, `#[ignore]` e **configurado por ambiente** em
vez de caminho fixo — um teste que exigisse 550 MB em disco para `cargo test` passar seria
um teste que todo mundo desliga:

```powershell
$env:MOS_WHISPER_BIN   = "...\whisper-cli.exe"
$env:MOS_WHISPER_MODEL = "...\ggml-large-v3-turbo-q5_0.bin"
$env:MOS_WHISPER_WAV   = "...\mic-16k.wav"
$env:MOS_WHISPER_WAV2  = "...\system-16k.wav"
cargo test -p mos-transcribe --test real_model -- --ignored --nocapture
```

Ele prova o que o teste de unidade não pode: que o comando montado pelo crate roda, que a
saída que o binário escreve é a que o parser espera, que o progresso chega a `1.0`, e que
a intercalação preserva a origem. A saída medida em 2026-08-19:

```text
    0 ms  VOCE    Bom dia pessoal, vamos comecar o alinhamento do NexoDoc.
    0 ms  REMOTO  Perfeito, bom dia. Eu revisei o documento ontem à noite.
 5360 ms  VOCE    Eu termino os slides ... amanhã de manhã e mando para vocês.
 6640 ms  REMOTO  Combinado, eu reviso os slides na sexta-feira pela manhã.
```

É exatamente a distinção que a §10.4 protege acima de qualquer outra, funcionando.

### 21.7 Teste contra o Hermes real

`apps/desktop/src-tauri/src/hermes.rs`, módulo `gate_d`, `#[ignore]`:

```powershell
cargo test -p mos-desktop --lib gate_d -- --ignored --nocapture
```

Ele **não pede senha e não a lê**: usa `Credentials::load()`, exatamente o caminho que o
aplicativo usa. Depende de o túnel estar aberto.

Um defeito de arquitetura apareceu aqui e vale registrar, porque ele custaria caro em
produção. A primeira versão de `ask_once` esperava `gateway.ready` antes de abrir a sessão
— e travava por cinco minutos até o teto. `gateway.ready` **e** o `result` do
`session.create` são ambos absorvidos por `Bridge::absorb` sem produzir saída visível, e
`next()` continua lendo em vez de devolver. Quem espera por eles fica parado num socket que
não vai mandar mais nada, porque a próxima coisa a acontecer depende de **nós** enviarmos a
pergunta. O conserto usa o mesmo `select!` que o laço da conversa já usava — a lição é que
o padrão existente estava certo e eu inventei outro.

### 21.8 Manual QA

O que só a máquina responde. Cada item exige registro de resultado, como a matriz de evidências
do `TECHNICAL-SPIKE-DESKTOP-SHELL.md` §4:

- reuniões de 5, 15 e 60 minutos;
- microfone de headset e microfone de mesa; saída em headset e em caixas;
- YouTube como fonte de áudio de sistema, e um Meet real;
- desconectar o headset no meio;
- trocar o dispositivo padrão de saída no meio;
- 10 minutos de silêncio absoluto (o teste que valida a §5.4);
- navegar pelo M/OS durante a gravação;
- fechar a janela e ficar no tray durante a gravação;
- matar o processo pelo Gerenciador de Tarefas após 20 minutos;
- Hermes desligado durante a gravação inteira;
- sem rede durante a gravação inteira.

---

## 22. UX Surfaces

Nenhum redesenho. Tokens, `Panel`, `Surface`, list-row e a linguagem de widgets existentes.

### 22.1 Estado inicial e gravação

```text
   Start Meeting Notes
```

Vira, e nada além disso:

```text
   ● Gravando         32:18

     Microfone        ✓
     Sistema          ✓

     [ Parar ]
```

Sem waveform, sem medidor grande, sem cockpit. Um nível de microfone discreto é permitido
porque ele responde a uma pergunta real ("está me ouvindo?"); qualquer coisa maior que isso
seria o "waveform showcase" que o brief proíbe.

O ponto vermelho é o único uso de `--danger` fora de contexto destrutivo neste desenho, e ele se
justifica: gravação em curso é a convenção universal, e `UX-PRINCIPLES` §63 manda ser familiar
onde é útil. O sódio continua reservado para carga (ADR-034).

### 22.2 Tray

```text
   M/OS
   ● Meeting Notes · 32:18
   ─────────────────
   Abrir
   Parar gravacao
```

O tray já existe com três itens (`lib.rs::setup_tray`). Ele ganha os dois de cima **apenas
enquanto grava**, e os perde depois.

### 22.3 Lista

```text
   REUNIOES

   HOJE
   14:00   NexoDoc — Comercial          1h12   3 acoes · 4 decisoes
   10:30   M/OS Design                    42m   2 acoes

   ONTEM
   16:15   Escadas Minarum                28m   pronta

   ─────
   09:40   Alinhamento                    51m   transcricao pronta · analise pendente
```

Agrupada por tempo, pelo mesmo argumento da ADR-030 para conversas: a maioria morre no mesmo
dia, e o que sobrevive delas são Tasks e Resources.

### 22.4 A página da reunião, e por que não são quatro abas

O brief manda não aceitar abas automaticamente. Avaliado, a resposta é **duas views num controle
segmentado, não quatro abas**:

```text
   NexoDoc — Comercial              [ Visao geral | Transcricao ]
   18 ago · 14:00 · 1h12 · NexoDoc
```

**Visão geral** contém, em seções e não em abas: Resumo, Suas ações, Decisões, Ações de outros,
Prazos, Follow-ups, Questões em aberto.

O motivo é `DESIGN-FOUNDATIONS.md` §7: controle segmentado troca **projeção da mesma
informação**; abas escondem coisas diferentes. Ações e Decisões não são outra informação — são o
resumo em outro nível de detalhe, e são exatamente o que a pessoa veio ver. Escondê-las atrás de
uma aba faria a tela abrir vazia do conteúdo que a justifica.

Só a transcrição merece view própria, e por três razões concretas: ela é longa, tem busca própria
e tem um modo de leitura diferente (linear, cronológico).

### 22.5 Ação e evidência

```text
   SUA ACAO                                          alta confianca

   Finalizar a apresentacao

   Prazo        amanha, 09:00
   Evidencia    14:04  ›

   [ Criar Task ]   [ + Lembrete ]
```

Clicar na evidência abre a transcrição no ponto:

```text
   14:04:12   VOCE
   Eu termino os slides amanha de manha e mando pra voces.
```

Este é o requisito `WHY?` do brief, literal.

### 22.6 Transcrição

```text
   [ buscar na transcricao ]                              612 segmentos

   14:02:14   VOCE
   Precisamos revisar isso amanha.

   14:02:28   REMOTO
   Eu consigo fazer pela manha.
```

Timestamp, origem, busca, salto e cópia. `VOCÊ` e `REMOTO` são o que o sistema **sabe**; nomes de
pessoas só aparecerão quando existir diarização confiável (§10.4).

### 22.7 Estados

Processando, recuperada (§9.2), erro (§20) e vazia — e a vazia segue `DESIGN-FOUNDATIONS.md` §10:
uma frase, uma ação, sem ilustração.

### 22.8 Rail

**Entregue.** Reuniões é o décimo segundo destino, com a **ADR-044** justificando não
retirar ninguém — o critério da ADR-036 é "renda ou memória", e uma reunião gravada é
literalmente memória. Grupo `TRABALHO`, depois de Finance.

A barra de gravação **não** é um destino: ela vive no shell, ao lado do estado de sistema e
do Argos. A promessa de que nunca se grava sem indicação visível (§17.2) não pode depender
de qual tela está aberta — se a barra morasse na página de Reuniões, ir para a Home apagaria
da vista o fato de que o microfone está aberto.

---

## 23. Roadmap

### V1 — este documento

Gravação manual, dois canais, gravação incremental, recuperação, transcrição local, análise
estruturada, Insights com evidência, Task e Reminder por confirmação, Project, Search e Jarvis
por contexto injetado.

### V2

- detecção automática de reunião: Calendário diz 14:00 **+** o Chrome começa a usar o microfone
  **→** *"Iniciar Meeting Notes?"*. **Sugestão, nunca início.** A arquitetura já comporta:
  `MeetingSource` tem o braço, e `monitor.rs` já observa nomes de processo dentro da fronteira da
  ADR-037 — que não seria alargada, porque "o Chrome está aberto" é nome de processo, e nada
  além;
- captura por processo: `AudioClient::new_application_loopback_client(process_id, include_tree)`
  **já existe no crate escolhido**. Isolar o áudio do Meet do resto do sistema deixa de ser
  pesquisa e vira trabalho;
- diarização dentro do canal SYSTEM;
- casar Meeting com evento de calendário — depende de `Event` existir.

### V3

Transcrição em tempo real, detecção de ação ao vivo, Jarvis durante a reunião, inteligência entre
reuniões. Nada disso antes de a V1 ter sido usada em reuniões de verdade.

---

## 24. Open Decisions

### Fechadas pelo proprietário em 2026-08-18

| # | Decisão |
|---|---|
| **D-A** | A transcrição sai para o Hermes com **consentimento uma vez**, análise automática depois, com registro por envio e revogação em Settings (§11.3) |
| **D-B** | Meetings entra no rail como **décimo segundo destino**, sem retirar ninguém, com ADR-044 (§22.8) |
| **D-C** | Prazo de Action Item vira **Task + Reminder**; `Task.due_at` **não** é introduzido agora (§14.2) |
| **D-D** | Meeting **oferece** lançar o tempo como `ActivityType::Meeting`; nunca lança sozinho (§14.3) |

### Fechadas pelo spike da Fase 1 em 2026-08-18

Evidência completa em `TECHNICAL-SPIKE-MEETING-AUDIO.md`.

| # | Pergunta | Resposta medida |
|---|---|---|
| **D-1** | Loopback por evento dispara no Windows 11 26200? | **Sim.** Intervalo máximo de 11 ms, zero fallbacks. Polling custa 2,4× a CPU para 16 ms. `EventsShared` fica |
| **D-2** | O keep-alive de silêncio é necessário? | **Sim, obrigatório.** Endpoint ocioso, 25 s de silêncio: 2.498 pacotes com ele, **zero** sem ele |
| **D-3** | `autoconvert` funciona junto com loopback? | **Sim.** `16000/1/i16` aceito nos dois canais. Sem ele, 24× o disco |
| **D-5** | Qual a deriva real? | 1–2 ms em 30 s contra o relógio do dispositivo; 10–22 ms entre canais |

E dois achados que não eram perguntas:

- **`silentPackets` não mede o que parecia medir.** O keep-alive escreve zeros de verdade, e o
  loopback os entrega como áudio comum — a flag `AUDCLNT_BUFFERFLAGS_SILENT` nunca aparece. O
  sinal honesto da D-2 é a ausência de **frames**;
- **`BufferInfo.index` conta em frames do dispositivo**, não nos convertidos. Comparar os dois
  sem escalar pela razão de taxas inventa frames perdidos — a primeira versão do spike reportou
  639.071 deles num teste de 20 segundos.

### Ainda abertas

| # | Pergunta | Como será respondida |
|---|---|---|
| **D-4** | Reconexão automática de dispositivo é confiável? | não exercitada no spike. Arrancar o headset 10 vezes; se falhar uma, degrada para falha explícita (§20). Bloqueia o Gate G, não a Fase 2 |

### Fechadas pela Fase 3 em 2026-08-19

| # | Resposta medida |
|---|---|
| **D-6** | **Sidecar.** `whisper-cli.exe`, sem dependência de C++ no `cargo build` (§10.2) |
| **D-7** | `large-v3-turbo-q5_0` em CPU: **5,6× tempo real** com 16 threads. Qualidade boa em pt-BR. cuBLAS empacota as DLLs do CUDA e fica como enhancement (§10.2) |

### Ainda abertas

| # | Pergunta |
|---|---|
| **D-8** | Qual o orçamento real de uma janela de transcrição no prompt? (§11.4) |
| **D-9** | 21 minutos para processar uma reunião de uma hora é aceitável, ou a build cuBLAS entra na V1? Só o uso real responde |

### Deliberadamente adiadas

- `Task.due_at` e `Event` — pré-requisitos que o `ATTENTION-SYSTEM.md` §34 já reservou;
- MCP local, que transformaria as projeções da §15.3 em ferramentas de verdade;
- provider de transcrição em nuvem;
- diarização.

---

## 25. Gates

Nenhum é pulado. Cada um exige evidência registrada, não percepção.

| Gate | Prova exigida |
|---|---|
| **A** ✔ | mic e sistema capturam juntos; arquivos válidos; D-1, D-2, D-3 e D-5 respondidas |
| **B** ✔ | processo morto aos 45 s deixou 44 s recuperáveis nos dois canais, 0 bytes soltos |
| **C** ◐ | a cadeia transcreve os dois canais com timestamps e origem corretos, provada com áudio pt-BR sintético; **falta uma reunião real de 15 min** |
| **D** ✔ | o Hermes real devolveu `mos-meeting` válido: 7 itens, 0 recusas, toda evidência resolvendo para segmento existente |
| **E** ✔ | migration aplica sobre banco vazio e sobre v16 povoado; Search acha a reunião e não o segmento; Task e Reminder nascem de um Insight **numa transação**, com desfazer |
| **F** ◐ | as superfícies existem, compilam e usam só tokens do design system; os dez itens do §16 não foram exercitados |
| **G** | uma reunião de verdade, ponta a ponta, sem perda silenciosa |

**Se o Gate A falhar, o trabalho para e este documento é reaberto.** Não haverá UI construída
sobre uma captura que não funciona.
