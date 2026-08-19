# Voice Inbox — design

Data: 2026-08-19
Branch: `feat/voice-inbox`

---

## 0. As cinco descobertas que moldaram o desenho

Antes de qualquer decisão, a auditoria do repositório. Cinco achados mudaram o
que esta feature é.

**1. Universal Drop Zone e "ingestion pipeline" não existem em código.**
`grep -ril "drop.zone|dropzone|universal.capture|ingest"` devolve apenas
`docs/`. O que existe de verdade é a cadeia
`CreateCaptureInput → CaptureService::create → CaptureRepository::create`, e a
proveniência já modelada em `tasks.source_capture_id` (0002) com
`create_task_from_capture` marcando a Capture como `processed` na mesma
transação. **Esse é o pipeline de ingestão.** Voice Inbox entra nele como
origem nova, e não constrói um segundo.

**2. O HUD já existe, e a voz já tem lugar reservado nele.** A janela
`quick-capture` (`tauri.conf.json`) é 640×126, `alwaysOnTop`, `skipTaskbar`,
posicionada a 34% da altura por `reveal_window`, e já é aberta por um atalho
global. O componente `QuickCapture` em `App.tsx` já carrega
`<span className="amplitude"><i/><i/><i/><i/></span>` com o comentário:

> *"Os tres tracos de amplitude sao a unica presenca da voz em repouso — sem
> icone de microfone. Ficam apagados ate a voz existir (fase adiada)."*

A fase adiada é esta. **Voice Inbox não ganha janela nova.**

**3. O design system já especifica a interação.** `mos-design-system.md` §Voz:

> Voz **não é um modo, é uma forma de digitar**. Desktop: segurar `⌥` enquanto
> fala (não alternar). Estados: repouso (três traços apagados) → ouvindo
> (traços em sódio reagindo à amplitude + timer) → transcrevendo (palavras
> provisórias em `--text-system`, confirmadas em `--text`) → interpretado
> (idêntico ao Capture digitado) → falhou (linguagem de warning, campo continua
> utilizável).

Isso decide a pergunta §6 do brief (segurar vs. alternar): **segurar**. E decide
a §7: a superfície é o campo de Capture que já está lá.

**4. Áudio e transcrição locais já existem, prontos e testados.** `mos-audio`
captura mic por WASAPI em chunks, com recuperação de queda, manifesto atômico e
RMS por segundo; `mos-transcribe` implementa a porta `TranscriptionProvider` do
core com `WhisperCliProvider` (sidecar `whisper-cli.exe`, decisão D-6). O
whisper **já está instalado nesta máquina** com build cuBLAS e
`ggml-large-v3-turbo-q5_0.bin`, e o `settings.json` já aponta para os dois.
Voice Inbox não escolhe provider nem adiciona runtime: usa a porta que existe.

**5. O whisper alucina em áudio quase-silencioso, e a alucinação propaga.**
Medido em 19/08 no Meeting Agent: um canal com picos de 1639 contra 27763 do
mic transcreveu `"Legenda por Sônia Ruberti"` — ninguém disse isso —, e o
resumo do Hermes incorporou o nome inventado. Numa reunião isso é ruído; **numa
Voice Inbox isso é uma Task nascida de silêncio.** O piso de energia deixa de
ser refinamento e vira requisito desta feature.

---

## 1. O que Voice Inbox é

Uma forma de digitar no campo que já existe.

```text
segura o atalho → fala → solta → acabou
```

O que ela **não** é: assistente always-listening, wake word, conversa por voz,
TTS, agente. `ROADMAP.md` §16 pede uma coisa só — *"o usuário consegue capturar
uma ideia sem precisar parar para digitar ou navegar"*.

---

## 2. Arquitetura

```text
Atalho global (segurar)          Alt dentro do HUD (segurar)
        │                                   │
        └────────────────┬──────────────────┘
                         ▼
              janela `quick-capture`
              (a que já existe)
                         │
                         ▼
          voice.rs  ·  orquestração no desktop
          (único lugar onde mos-audio e mos-core se encontram,
           mesmo desenho de meeting.rs)
                         │
                         ▼
              mos_audio::Recording::start_mic
              (mic-only, sem loopback, sem keep-alive)
                         │
                         ▼
              voice_notes  ·  o registro durável do áudio
                         │
                         ▼
              piso de energia + piso de duração
                         │
                         ▼
              TranscriptionProvider  (a porta que já existe)
                         │
                         ▼
              mos_core::voice::understand()   ← puro, determinístico, testado
                         │
                         ▼
              Capture (source = voice)   ────────────► Inbox + Search
                         │
                         ▼ só com confiança alta
              create_task_from_capture_with_reminder()
                         │
                    ┌────┴────┐
                    ▼         ▼
                  Task     Reminder
```

Nenhuma caixa nova onde já havia uma. `voice_notes` é a única tabela nova, e a
§6 justifica por que ela precisa existir.

---

## 3. Atalho

**`Ctrl+Alt+Space`, segurado.** Configurável em Settings, ao lado do atalho de
captura que já existe, pelo mesmo mecanismo (`UserSettings.voice_shortcut`,
`set_voice_shortcut`, registro no `global_shortcut()` com rollback).

Por que este:

- irmão do `Ctrl+Shift+Space` que já abre o Quick Capture — mesmo `Space`, mesma
  família, mesma memória muscular;
- `Alt` ecoa o `⌥` que o design system pede;
- `Ctrl+Shift+V` foi **recusado**: como atalho global ele roubaria "colar sem
  formatação" de todos os programas da máquina;
- `Alt+Space` foi recusado: é o menu de janela do Windows.

Semântica de segurar, com o plugin `tauri-plugin-global-shortcut`, que entrega
`ShortcutState::Pressed` e `Released`:

```text
Pressed   → revela o HUD  →  começa a gravar
Released  → para          →  transcreve em background
```

O auto-repeat do Windows dispara `Pressed` repetidas vezes enquanto a tecla
está afundada. A guarda é o próprio estado: já gravando, `Pressed` é ignorado.

**A rede de segurança do microfone.** Se o `Released` se perder — janela
trocada, sessão bloqueada, plugin engasgado —, o microfone ficaria aberto. Três
guardas independentes o fecham:

1. teto rígido de 120 s por gravação, num watchdog em thread própria;
2. `Esc` no HUD cancela e descarta;
3. o HUD perder o foco encerra a gravação (não descarta: o que foi dito é
   preservado).

---

## 4. Estados

Uma máquina de estados, e não booleanos soltos — a mesma regra que
`MEETING-AGENT.md` aplicou a `Meeting.status` para não permitir estados
impossíveis.

```text
Idle
 ├─ Recording      { started_at, duration_ms, level }
 ├─ TooQuiet                      ← terminal, nada persistido
 ├─ Transcribing   { note_id }
 ├─ Captured       { capture_id, understanding }
 ├─ Acted          { capture_id, task_id, reminder_id, undo }
 ├─ NeedsRetry     { note_id, message }   ← áudio preservado
 └─ Failed         { message }
```

`requesting_permission` do brief **não existe** neste desenho, e a ausência é
deliberada: a captura acontece no processo Rust por WASAPI, não no WebView. Não
há prompt de permissão de navegador a esperar. Dispositivo ausente ou tomado
aparece como `Failed` com a frase do sistema, que é honesto — inventar um
estado de permissão que a plataforma não tem seria ensinar uma cerimônia falsa.

---

## 5. O piso de energia, e o piso de duração

Duas recusas antes de qualquer transcrição, e **nenhuma delas grava nada**:

| guarda | limiar | por quê |
|---|---|---|
| duração | < 400 ms | tecla tocada sem querer |
| energia | pico RMS < 120 (de 1000) | o achado 5 — silêncio vira texto inventado |

O pico vem de `Live.level_milli`, que `mos-audio` já mantém e que a thread de
captura já reduz a `0..1000` dentro dela mesma. Nada de PCM cruza a fronteira.

Recusado, o HUD diz **"Não ouvi nada"** e volta a repouso. Nenhuma linha em
`voice_notes`, nenhum arquivo em disco, nenhuma Capture. É o §23 do brief — *não
crie lixo no banco para gravações canceladas*.

Terceira guarda, depois da transcrição: `mos_core::voice::is_hallucination`
descarta a família de créditos de legenda que o whisper inventa em português
(`legenda por…`, `legendas pela comunidade…`, `amara.org`, `subtitles by…`).
Descartado, o resultado é tratado como transcrição vazia — áudio preservado,
`NeedsRetry`.

---

## 6. Persistência

### 6.1 `voice_notes` — por que a tabela existe

`Capture.content` é `NOT NULL CHECK (length(trim(content)) > 0)` e o domínio não
tem operação de reescrever conteúdo. Então uma Capture **não pode nascer antes
da transcrição** sem que se invente um conteúdo falso e uma mutação que hoje
não existe — e reescrever conteúdo depois destruiria exatamente a garantia que
o brief §11 pede, a de que a transcrição original é preservada.

`voice_notes` é o registro durável do áudio entre "parei de falar" e "existe
texto". É o mesmo desenho que `meetings` usa, e pela mesma razão.

```sql
CREATE TABLE voice_notes (
    id                TEXT PRIMARY KEY NOT NULL,
    status            TEXT NOT NULL,     -- recording|recorded|transcribing|captured|failed|cancelled
    audio_dir         TEXT NOT NULL,     -- relativo ao app_data_dir, derivado do id
    duration_ms       INTEGER NOT NULL DEFAULT 0,
    peak_level        INTEGER NOT NULL DEFAULT 0,
    transcript        TEXT NOT NULL DEFAULT '',
    provider          TEXT NOT NULL DEFAULT '',
    capture_id        TEXT REFERENCES captures(id) ON DELETE SET NULL,
    context_project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    context_task_id   TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    failure_message   TEXT NOT NULL DEFAULT '',
    audio_deleted_at  TEXT,
    started_at        TEXT NOT NULL,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
) STRICT;
```

O contrato de estado, imposto por `CHECK` e não por documentação: `captured`
exige `capture_id NOT NULL` e transcrição não vazia; `failed` exige
`failure_message` não vazia.

### 6.2 O áudio

**Apagado assim que a Capture existe.** A informação relevante — o texto — já
está preservada e indexada nesse instante, então guardar os bytes só aumenta a
superfície de privacidade sem comprar nada. Preferência declarada do produto:
privacidade e pouco armazenamento (brief §12).

O áudio **sobrevive** exatamente enquanto a informação ainda não foi
preservada: `recorded`, `transcribing`, `failed`. É o que torna
`voice_retry` honesto, e é a resposta ao critério de aceite F.

Sem enum de retenção configurável nesta versão. `AudioRetention` existe para
Meeting porque uma reunião de uma hora tem valor de reescuta; oito segundos de
"comprar café" não têm. Um seletor de três opções para isso seria cerimônia sem
decisão real por trás.

### 6.3 `CaptureSource::Voice`

O `CHECK (source_kind IN ('home','quick_capture'))` de 0001 recusa `voice`, e
SQLite não altera `CHECK`. A migration 0022 recria `captures` pelo procedimento
de doze passos, **preservando `rowid`** — o que mantém `capture_search` (FTS5
de conteúdo externo) apontando para as linhas certas.

`tasks.source_capture_id` e `resources.source_capture_id` apontam para
`captures` com `ON DELETE RESTRICT`. Com `foreign_keys=ON`, `DROP TABLE` numa
tabela-pai com filhos é recusado; e `PRAGMA foreign_keys` é *no-op* dentro de
transação, então o desligamento não cabe dentro do `BEGIN` do arquivo `.sql`.
Ele acontece **no Rust, em volta da migration**, com `PRAGMA foreign_key_check`
antes do `ON` de volta — quem desliga a guarda tem de provar que não a
precisava.

---

## 7. `mos_core::voice` — o resolvedor de intenção

Puro, determinístico, sem rede, sem IA, e é **a única parte com regra de
verdade**, portanto é onde os testes moram. `SETUP-MAQUINA.md` §4 registra que
`cargo test -p mos-desktop` não roda na máquina principal; teste que não roda
não protege nada, então o desktop fica casca fina e o core carrega a decisão.

```rust
pub fn understand(
    transcript: &str,
    now_local: OffsetDateTime,   // já no fuso de quem falou
    context: VoiceContext,       // Project/Task na tela quando o atalho tocou
    projects: &[ProjectHint],
) -> Understanding
```

### 7.1 Fuso

`now_local` chega **do renderer**, em RFC 3339 com offset. É a regra normativa
de `CORE-FOUNDATION.md` §5 — *"a interpretação de datas naturais deve respeitar
timezone e locale do usuário"* — e o padrão que `ReminderComposer` e
`calendar_window` já seguem: quem conhece o fuso é a tela, o banco guarda UTC.

O instante resolvido é gravado em UTC, **e a frase original também é gravada**.
"Amanhã" resolvido não pode depender de quando alguém lê depois.

### 7.2 Datas naturais em pt-BR

`hoje`, `amanhã`, `depois de amanhã`, dias da semana (`sexta`, `segunda`, …,
sempre a **próxima** ocorrência), `semana que vem`, `dia 25`, `às nove` /
`às 9` / `9h` / `09:00` / `nove e meia`, `daqui a duas horas` / `em 2 horas`,
e os períodos `de manhã` (09:00), `de tarde` (14:00), `de noite` (20:00).

Sem hora dita, o padrão é 09:00 do dia — o mesmo default que
`ReminderComposer` já usa em "Amanhã 9h". Data no passado depois de resolvida
rola para a próxima ocorrência válida.

### 7.3 Project

Três caminhos, em ordem de força:

1. **código falado** — `063-26`, e também `zero sessenta e três barra vinte e
   seis` normalizado; casa contra o nome do Project por código embutido;
2. **nome falado** — `no projeto NexoDoc`, `do projeto …`, casamento por
   token normalizado (sem acento, minúsculo), exigindo casamento **único**;
   dois Projects candidatos não escolhem nenhum;
3. **contexto** — o Project da tela quando o atalho tocou.

O contexto é **sinal, não verdade** (brief §13): ele só entra quando nada foi
dito, e quando entra, entra com confiança um degrau abaixo.

### 7.4 Intenção e confiança

```text
Reminder  ← verbo explícito de lembrete  E  instante resolvido
Task      ← verbo de trabalho explícito ("coloca…no projeto", "adiciona",
            "criar task") ou imperativo simples reconhecido
Capture   ← todo o resto
```

E, acima de tudo, o **marcador de hesitação**: `talvez`, `acho que`,
`quem sabe`, `seria bom`, `eu devia`, `não sei se`, `um dia`, `qualquer hora`.
Presente, a intenção cai para `Capture` com confiança baixa,
**independentemente de haver verbo e data na frase**. É o brief §17 em forma de
código: *não confunda linguagem natural com autorização.*

| confiança | o que acontece |
|---|---|
| alta | executa, com Desfazer no recibo |
| média | Capture criada; o HUD **oferece** a ação por ⏎ durante o recibo |
| baixa | Capture na Inbox, e nada mais |

O grau médio **não faz pergunta**. Ele mostra o que teria feito e deixa a
Capture salva atrás — quem não responde nada fica com a Capture, que é o
comportamento correto do brief §19, não um erro. É assim que §17 (sugerir) e
§18 (não perguntar) coexistem sem se contradizer.

---

## 8. Ação, e o caminho de volta

Confiança alta em `Reminder` cria **Task e Reminder na mesma transação**, com o
Reminder apontando para a Task — o precedente é `accept_insight`, e a razão é a
mesma: existe um instante entre as duas escritas em que a Task existe e o aviso
dela não, e uma queda ali deixa o compromisso mudo.

`WorkRepository::create_task_from_capture_with_reminder` é a operação nova. Ela
faz o que `create_task_from_capture` já fazia — cria a Task ligada à Capture e
marca a Capture como `processed` — e acrescenta o Reminder opcional, tudo sob
uma transação.

Desfazer entra pelo mecanismo que já existe, `UndoStep`, com um braço novo:

```rust
UndoVoiceAction { capture_id, task_id, reminder_id: Option<String> }
```

Arquiva a Task, cancela o Reminder e devolve a Capture para a Inbox. **Não
apaga** — ADR-035, todo Undo do M/OS é restauração de estado. A Capture
permanece: ela é a origem, e o histórico do que foi dito não é o que se está
desfazendo.

---

## 9. Search e Inbox

Nada novo. A Capture nasce `processing_state = inbox`, entra em `capture_search`
pela mesma escrita que toda Capture faz, e aparece em `SearchItem::Capture` com
`derived_task` quando virou Task. `source` já viaja no `Capture` serializado,
então filtrar por voz é leitura de campo, não índice novo.

O que muda na tela é uma etiqueta: a linha da Inbox e o visualizador de Capture
mostram `VOZ` em `--font-system`, do mesmo jeito que já mostram a origem.

---

## 10. Privacidade

- o microfone só abre depois de gesto explícito — segurar uma tecla;
- enquanto grava, o HUD está na tela, com os traços em sódio reagindo e o timer
  correndo: não existe caminho em que o microfone esteja aberto sem isso;
- soltar, `Esc`, perder o foco, o teto de 120 s e o encerramento do processo
  fecham o stream — cinco caminhos, e o `Drop` da `Recording` é o último;
- nada sobe para lugar nenhum: transcrição é local, e o Hermes **não participa**
  desta feature;
- log nenhum carrega transcrição ou áudio. Erro de provider carrega o stderr do
  binário, que é técnico e nunca contém fala — a mesma fronteira do §16.3 do
  Meeting Agent;
- áudio apagado assim que o texto existe.

---

## 11. Testes

No core, que é onde eles rodam:

- datas naturais, uma por forma, incluindo a virada de mês e o dia da semana
  que cai hoje (pede a próxima);
- hesitação vencendo verbo + data;
- Project por código, por nome, por contexto, e o empate que não escolhe;
- título gerado sem o verbo, sem a data e sem o trecho de Project;
- alucinação de legenda reconhecida;
- a máquina de estados de `VoiceNote` recusando transições impossíveis.

No storage: migration preservando `rowid` e FTS, `voice` aceito em
`source_kind`, `create_task_from_capture_with_reminder` atômico, retry
encontrando a nota depois de reabrir.

No renderer: o que é função pura — formatação do recibo, e o mapa de estado do
HUD.

Fora de teste, por decisão: animação, e o caminho que exige microfone real.
`mos-audio` já tem `tests/hardware.rs` ignorado por padrão para isso.

---

## 12. O que fica de fora, explicitamente

Always-listening, wake word, conversa contínua, TTS, voz no Hermes, Universal
Timeline, Tool Gateway, mobile, sincronização de áudio, treinamento de modelo,
e a intenção `waiting_for` — que o brief §15 exemplo D cobre com "caso
contrário, Capture", e é isso que acontece: não há entidade Waiting For no
M/OS, então a frase do João vira Capture, que é o comportamento correto.
