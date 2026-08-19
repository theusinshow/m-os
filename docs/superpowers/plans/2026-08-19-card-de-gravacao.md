# O card de gravação — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dar à reunião em curso um lugar de trabalho — anotações que sobem ao Hermes, onda sonora ao vivo e Pausar — no painel de detalhe da página Reuniões.

**Architecture:** Três frentes independentes que se encontram numa tela. O estado `Paused` entra na máquina pura de `mos-core`, testado sem janela. A onda ganha evento próprio (`meeting-level`, 15 Hz, dois números) para não inflar o `meeting-tick` de 1 Hz. As notas viram coluna em `meetings` e um bloco de contexto no prompt — sem tocar na validação de evidência.

**Tech Stack:** Rust (Tauri 2, rusqlite, SQLite STRICT), React 18 + TypeScript (Vite, Vitest), CSS com tokens de `packages/design-system/tokens.css`.

## Global Constraints

- **A aba "Anotações" entra no `segmented` que já existe** em `MeetingsPage.tsx:535`, ao lado de "Visão geral" e "Transcrição". Não crie um segundo controle de abas.
- **As notas NÃO geram itens.** O prompt exige *"pelo menos um `segment`"* por item, e a validação em `parse_analysis` **não muda**. Uma nota é contexto; ver §6.1 da spec.
- **A transcrição continua sendo pós-reunião.** Nada nesta implementação transcreve durante a gravação.
- **O keep-alive de silêncio para junto com a pausa.** Se ele continuar escrevendo, o canal SYSTEM acumula frames que o MIC não tem — a torção de linha do tempo que o spike mediu em 4710 ms.
- **Pausado, o ponto vermelho para de pulsar.** Um ponto pulsando com o microfone fechado é a mentira que a §17.2 existe para impedir.
- **Comentários e commits em português, sem acento dentro de `.rs`.** Em `.ts`, `.tsx`, `.md` e `.sql` o acento é normal.
- **Não existe teste de DOM neste repo** (`vitest.config.ts`): o que for testado tem de ser função pura.
- Antes de qualquer `cargo`: `export TMP="<scratchpad>/tmp"; export TEMP="$TMP"`.
- Verificação visual pela skill `ver-o-app`; `orca computer` não funciona nesta máquina.

---

### Task 1: O estado `Paused` na máquina

**Files:**
- Modify: `crates/mos-core/src/meeting.rs:152` (enum), `:170` (`as_str`), `:241` (`Transition`), `:306` (`apply`)
- Test: `crates/mos-core/src/meeting.rs` (módulo `tests`)

**Interfaces:**
- Produces: `MeetingStatus::Paused`; `Transition::Pause`; `Transition::Resume`; `MeetingStatus::as_str` devolvendo `"paused"`.

- [ ] **Step 1: Escreva os testes que falham**

No `mod tests` de `crates/mos-core/src/meeting.rs`:

```rust
    #[test]
    fn pausar_e_retomar_andam_entre_recording_e_paused() {
        let agora = OffsetDateTime::now_utc();
        let gravando = meeting_em(MeetingStatus::Recording);

        let pausada = apply(&gravando, Transition::Pause, agora).unwrap();
        assert_eq!(pausada.status, MeetingStatus::Paused);
        // Pausar NAO carimba fim: a reuniao nao acabou, ela esta esperando.
        assert!(pausada.ended_at.is_none());

        let retomada = apply(&pausada, Transition::Resume, agora).unwrap();
        assert_eq!(retomada.status, MeetingStatus::Recording);
    }

    #[test]
    fn parar_funciona_a_partir_de_paused() {
        let agora = OffsetDateTime::now_utc();
        let pausada = meeting_em(MeetingStatus::Paused);
        let parando = apply(&pausada, Transition::Stop, agora).unwrap();
        assert_eq!(parando.status, MeetingStatus::Stopping);
    }

    #[test]
    fn pausa_recusada_fora_de_recording() {
        let agora = OffsetDateTime::now_utc();
        for estado in [
            MeetingStatus::Recorded,
            MeetingStatus::Transcribed,
            MeetingStatus::Ready,
            MeetingStatus::Paused,
        ] {
            assert!(
                apply(&meeting_em(estado), Transition::Pause, agora).is_err(),
                "Pause deveria ser recusado em {}",
                estado.as_str()
            );
        }
        // E retomar so faz sentido a partir de Paused.
        assert!(apply(&meeting_em(MeetingStatus::Recording), Transition::Resume, agora).is_err());
    }
```

Se não houver um helper `meeting_em(status)` no módulo, crie-o ao lado dos testes existentes, montando uma `Meeting` mínima com o status pedido — leia como os testes vizinhos constroem a struct e repita a forma.

- [ ] **Step 2: Rode e confirme que falha**

```bash
export TMP="$SCRATCH/tmp"; export TEMP="$TMP"; mkdir -p "$TMP"
cargo test -p mos-core pausar_e_retomar
```
Esperado: FAIL com `no variant named 'Paused'`.

- [ ] **Step 3: Acrescente o estado e as transições**

Em `crates/mos-core/src/meeting.rs`, no `enum MeetingStatus`, depois de `Recording`:

```rust
    /// Gravacao suspensa pela pessoa. Os dois canais param de escrever JUNTOS, e
    /// o tempo pausado nao vira frame — entao nao vira duracao, porque
    /// `duration_ms` e medida em frames gravados e nunca por diferenca de
    /// relogio. Nao ha vao para reconstruir.
    Paused,
```

Em `as_str`, `Self::Paused => "paused",`. Acrescente também o caminho de volta em `parse` (a função que lê a string do banco), ao lado de `"recording"`.

No `enum Transition`:

```rust
    /// O usuario clicou em Pausar.
    Pause,
    /// O usuario clicou em Retomar.
    Resume,
```

E em `Transition::name()`, `Self::Pause => "pausar"` e `Self::Resume => "retomar"` — a mensagem de recusa usa esse nome.

- [ ] **Step 4: Acrescente as regras em `apply`**

No `match (meeting.status, transition)`, junto de `(Recording, Transition::Stop)`:

```rust
        (Recording, Transition::Pause) => {
            next.status = Paused;
        }

        (Paused, Transition::Resume) => {
            next.status = Recording;
        }

        // Parar a partir de Paused vai direto para Stopping, igual a Recording:
        // os arquivos ainda precisam ser fechados, e `ended_at` continua sendo
        // carimbado no AudioSettled e nao aqui.
        (Paused, Transition::Stop) => {
            next.status = Stopping;
        }
```

Tudo o que não casar cai no `refused()` que já existe, que é o comportamento que o terceiro teste cobra.

- [ ] **Step 5: Rode e confirme que passa**

```bash
cargo test -p mos-core meeting
```
Esperado: PASS, incluindo os três testes novos.

- [ ] **Step 6: Commit**

```bash
git add crates/mos-core/src/meeting.rs
git commit -m "feat(reuniao): o estado Paused, e o tempo pausado que nao vira duracao"
```

---

### Task 2: A pausa no áudio, com o keep-alive parando junto

**Files:**
- Modify: `crates/mos-audio/src/lib.rs` (método de pausa na sessão)
- Modify: `crates/mos-audio/src/capture.rs:392-402` (o keep-alive)
- Test: `crates/mos-audio/src/lib.rs` (módulo `tests`)

**Interfaces:**
- Consumes: `MeetingStatus::Paused` (Task 1).
- Produces: `Session::set_paused(&self, paused: bool)`; `Session::is_paused(&self) -> bool`.

- [ ] **Step 1: Escreva o teste que falha**

No `mod tests` de `crates/mos-audio/src/lib.rs`:

```rust
    #[test]
    fn pausar_para_de_contar_frames_e_desliga_o_keep_alive() {
        let sessao = sessao_de_teste();
        assert!(!sessao.is_paused());

        sessao.set_paused(true);
        assert!(sessao.is_paused());
        let antes = sessao.snapshot().mic_frames;
        // Pausado, nenhum frame novo entra na conta, venha audio ou nao.
        sessao.alimentar_para_teste(&[0u8; 3200]);
        assert_eq!(sessao.snapshot().mic_frames, antes, "frame gravado durante a pausa");

        sessao.set_paused(false);
        sessao.alimentar_para_teste(&[0u8; 3200]);
        assert!(sessao.snapshot().mic_frames > antes, "retomar precisa voltar a contar");
    }
```

`sessao_de_teste()` e `alimentar_para_teste()` provavelmente não existem. Leia o `mod tests` deste arquivo: se ele já testa a sessão sem hardware, use o mesmo mecanismo. Se **não houver** teste de sessão sem hardware — o que é provável, já que a captura depende do WASAPI —, então **não invente um duplo**: mova a decisão para uma função pura e teste ela:

```rust
    #[test]
    fn frame_so_conta_quando_nao_esta_pausado() {
        assert_eq!(frames_a_contar(3200, false), 3200);
        assert_eq!(frames_a_contar(3200, true), 0);
    }
```

com

```rust
/// Quantos frames de um pacote entram na conta. Pausado, nenhum.
///
/// E funcao e nao `if` no meio do laco de captura porque e a regra que sustenta
/// "duracao medida em frames": ela precisa ser verificavel sem WASAPI.
pub fn frames_a_contar(frames: usize, pausado: bool) -> usize {
    if pausado { 0 } else { frames }
}
```

Escolha um dos dois caminhos e siga só ele.

- [ ] **Step 2: Rode e confirme que falha**

```bash
cargo test -p mos-audio pausa
```
Esperado: FAIL por símbolo inexistente.

- [ ] **Step 3: Implemente a pausa na sessão**

Acrescente um `AtomicBool` `paused` à struct da sessão, ao lado dos atômicos de nível que já existem, com:

```rust
    /// Suspende a escrita nos DOIS canais.
    ///
    /// Os dois juntos, e nunca um so: parar apenas o MIC deixaria o SYSTEM
    /// acumulando frames que o outro nao tem, e a linha do tempo torceria — a
    /// mesma falha de 4710 ms que o spike mediu, chegando pelo outro lado.
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
```

No laço de captura de `capture.rs`, onde hoje o payload é escrito no chunk, descarte o pacote quando `paused` — **sem** parar o stream do WASAPI. Parar o stream forçaria reabrir o dispositivo ao retomar, e reabrir pode devolver outro formato efetivo, que é uma troca silenciosa que o `ChannelInfo::timing` existe para tornar visível.

- [ ] **Step 4: Desligue o keep-alive junto**

Em `capture.rs`, o `spawn_keep_alive` recebe hoje um `Arc<AtomicBool>` de parada. Passe também o `paused` e faça o laço dele **não escrever silêncio enquanto pausado**:

```rust
            if pausado.load(Ordering::Relaxed) {
                // Silencio escrito durante a pausa entra no chunk do SYSTEM e
                // desalinha os canais — exatamente o que o keep-alive existe
                // para evitar, invertido.
                continue;
            }
```

- [ ] **Step 5: Rode e confirme que passa**

```bash
cargo test -p mos-audio
```
Esperado: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mos-audio/
git commit -m "feat(audio): pausar suspende os dois canais e o keep-alive junto"
```

---

### Task 3: Os comandos de pausar e retomar

**Files:**
- Modify: `apps/desktop/src-tauri/src/meeting.rs` (ao lado de `meeting_stop`, linha 158)
- Modify: `apps/desktop/src-tauri/src/lib.rs` (`invoke_handler`)
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/types.ts` (`MeetingStatus` ganha `"paused"`)

**Interfaces:**
- Consumes: `Transition::Pause`/`Resume` (Task 1); `Session::set_paused` (Task 2).
- Produces: `api.meetingPause(): Promise<Meeting>`; `api.meetingResume(): Promise<Meeting>`.

- [ ] **Step 1: Escreva os comandos**

Em `apps/desktop/src-tauri/src/meeting.rs`, seguindo a forma de `meeting_stop`:

```rust
/// Suspende a gravacao em curso.
///
/// A ordem importa: primeiro o audio para de contar, depois o estado muda. Se
/// fosse ao contrario, o intervalo entre as duas linhas gravaria frames numa
/// reuniao que a tela ja mostra como pausada.
#[tauri::command]
pub fn meeting_pause(app: AppHandle) -> Result<Meeting, CoreError> {
    aplicar_pausa(&app, true, mos_core::meeting::Transition::Pause)
}

#[tauri::command]
pub fn meeting_resume(app: AppHandle) -> Result<Meeting, CoreError> {
    aplicar_pausa(&app, false, mos_core::meeting::Transition::Resume)
}
```

E o corpo compartilhado:

```rust
/// A ordem importa e nao e arbitraria.
///
/// PAUSAR: o audio para PRIMEIRO, o estado muda depois. Se fosse ao contrario, o
/// intervalo entre as duas linhas gravaria frames numa reuniao que a tela ja
/// mostra como pausada.
///
/// RETOMAR: o estado muda primeiro, o audio volta depois — pelo mesmo motivo
/// invertido. Frame gravado antes de a tela dizer "gravando" e a mesma mentira,
/// so que pior, porque ninguem esta olhando.
fn aplicar_pausa(
    app: &AppHandle,
    pausar: bool,
    transicao: mos_core::meeting::Transition,
) -> Result<Meeting, CoreError> {
    let recorder = app.state::<RecordingState>();

    if pausar {
        let active = recorder.active.lock().map_err(|_| trava())?;
        let sessao = active.as_ref().ok_or_else(nenhuma_gravacao)?;
        sessao.set_paused(true);
    }

    let atualizada = aplicar_transicao(app, transicao)?;

    if !pausar {
        let active = recorder.active.lock().map_err(|_| trava())?;
        let sessao = active.as_ref().ok_or_else(nenhuma_gravacao)?;
        sessao.set_paused(false);
    }

    // Tick imediato: sem ele a barra levaria ate um segundo para dizer PAUSADO,
    // e um segundo de ponto vermelho pulsando depois do clique e a mentira que a
    // §17.2 proibe.
    if let Ok(active) = recorder.active.lock() {
        if let Some(atual) = active.as_ref() {
            let frame = tick(atual);
            drop(active);
            let _ = app.emit("meeting-tick", &frame);
        }
    }
    Ok(atualizada)
}
```

`aplicar_transicao`, `trava()` e `nenhuma_gravacao()` provavelmente já existem neste arquivo com outros nomes — `meeting_stop` faz as três coisas. **Leia `meeting_stop` primeiro e reuse os nomes que ele usa**, em vez de criar helpers paralelos.

- [ ] **Step 2: Registre no `invoke_handler`**

Em `apps/desktop/src-tauri/src/lib.rs`, junto de `meeting::meeting_stop`, acrescente `meeting::meeting_pause,` e `meeting::meeting_resume,`.

- [ ] **Step 3: Ligue o cliente**

Em `apps/desktop/src/types.ts`, acrescente `"paused"` à união `MeetingStatus`.

Em `apps/desktop/src/api.ts`, junto de `meetingStop`:

```ts
  meetingPause() {
    return invoke<Meeting>("meeting_pause");
  },
  meetingResume() {
    return invoke<Meeting>("meeting_resume");
  },
```

- [ ] **Step 4: Confirme que compila dos dois lados**

```bash
cd apps/desktop && npx tsc --noEmit
export TMP="$SCRATCH/tmp"; export TEMP="$TMP"
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
```
Esperado: sem erros. O `tsc` vai apontar todo `switch` sobre `MeetingStatus` que não trata `"paused"` — trate cada um, e é de propósito que ele reclame.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src apps/desktop/src/api.ts apps/desktop/src/types.ts
git commit -m "feat(reuniao): comandos de pausar e retomar"
```

---

### Task 4: O evento de nível a 15 Hz

**Files:**
- Modify: `apps/desktop/src-tauri/src/meeting.rs:356` (o laço `run`)
- Modify: `apps/desktop/src/types.ts`

**Interfaces:**
- Produces: evento Tauri `meeting-level` com `{ mic: number; system: number }`; tipo `MeetingLevel` em `types.ts`.

- [ ] **Step 1: Separe os dois laços**

Em `apps/desktop/src-tauri/src/meeting.rs`, o `run` atual dorme 1 s e emite `meeting-tick`. Acrescente um segundo laço, em tarefa própria:

```rust
/// O nivel, quinze vezes por segundo.
///
/// Laco separado do `run` porque as duas coisas mudam em ritmos diferentes:
/// estado, duracao e saude dos canais mudam uma vez por segundo, e mandar o
/// `MeetingTick` inteiro a 15 Hz seria repetir quinze vezes um objeto que mudou
/// zero. Aqui vao DOIS numeros, e nada mais — nunca PCM.
pub async fn run_levels(app: AppHandle) {
    loop {
        tokio::time::sleep(Duration::from_millis(66)).await;

        let recorder = app.state::<RecordingState>();
        let Ok(active) = recorder.active.lock() else { continue };
        let Some(current) = active.as_ref() else { continue };
        let estado = current.snapshot();
        drop(active);

        let _ = app.emit(
            "meeting-level",
            serde_json::json!({ "mic": estado.mic_level, "system": estado.system_level }),
        );
    }
}
```

Use o mesmo nome de método que `tick()` usa para ler o estado da sessão — leia a função `tick` na linha 209 e copie a chamada, em vez de assumir `snapshot()`.

- [ ] **Step 2: Suba a tarefa**

Em `apps/desktop/src-tauri/src/lib.rs`, ao lado de `tauri::async_runtime::spawn(meeting::run(app.handle().clone()));`:

```rust
            tauri::async_runtime::spawn(meeting::run_levels(app.handle().clone()));
```

- [ ] **Step 3: Declare o tipo no front**

Em `apps/desktop/src/types.ts`, junto de `MeetingTick`:

```ts
/** O nível cru, a 15 Hz. Dois números e nada mais — nunca PCM. */
export type MeetingLevel = {
  mic: number;
  system: number;
};
```

- [ ] **Step 4: Confirme que compila**

```bash
export TMP="$SCRATCH/tmp"; export TEMP="$TMP"
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cd apps/desktop && npx tsc --noEmit
```

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src apps/desktop/src/types.ts
git commit -m "feat(reuniao): evento de nivel a 15 Hz, separado do tick"
```

---

### Task 5: A janela da onda, em função pura

**Files:**
- Create: `apps/desktop/src/ondaSonora.ts`
- Test: `apps/desktop/src/ondaSonora.test.ts`

**Interfaces:**
- Produces: `BARRAS = 30`; `DEGRAUS = 8`; `empurrar(janela: number[], nivel: number): number[]`; `alturaDaBarra(nivel: number): number`; `degrausAcesos(nivel: number): number`.

- [ ] **Step 1: Escreva os testes que falham**

Crie `apps/desktop/src/ondaSonora.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { BARRAS, DEGRAUS, alturaDaBarra, degrausAcesos, empurrar } from "./ondaSonora";

describe("a janela", () => {
  it("nasce cheia de silêncio, para a onda não crescer da esquerda", () => {
    const janela = empurrar([], 500);
    expect(janela).toHaveLength(BARRAS);
    expect(janela[BARRAS - 1]).toBe(500);
    expect(janela[0]).toBe(0);
  });

  it("empurra pela direita e descarta a mais velha", () => {
    let janela = Array.from({ length: BARRAS }, (_, i) => i);
    janela = empurrar(janela, 999);
    expect(janela).toHaveLength(BARRAS);
    expect(janela[BARRAS - 1]).toBe(999);
    expect(janela[0]).toBe(1);
  });

  it("nunca cresce além de BARRAS, por mais que se empurre", () => {
    let janela: number[] = [];
    for (let i = 0; i < BARRAS * 3; i += 1) janela = empurrar(janela, i);
    expect(janela).toHaveLength(BARRAS);
  });
});

describe("o modo sem movimento", () => {
  it("silêncio acende pelo menos um degrau", () => {
    // Mesma razão do PISO: zero degraus leria como "morreu".
    expect(degrausAcesos(0)).toBe(1);
  });

  it("cresce por degraus e satura em DEGRAUS", () => {
    expect(degrausAcesos(1000)).toBe(DEGRAUS);
    expect(degrausAcesos(5000)).toBe(DEGRAUS);
    expect(degrausAcesos(500)).toBeGreaterThan(degrausAcesos(100));
    expect(degrausAcesos(500)).toBeLessThan(DEGRAUS);
  });
});

describe("a altura", () => {
  it("silêncio ainda desenha um traço, e não some", () => {
    // Uma barra de altura zero leria como "morreu", e silêncio não é queda.
    expect(alturaDaBarra(0)).toBeGreaterThan(0);
  });

  it("cresce com o nível e satura no teto", () => {
    expect(alturaDaBarra(500)).toBeGreaterThan(alturaDaBarra(100));
    expect(alturaDaBarra(1000)).toBe(1);
    expect(alturaDaBarra(5000)).toBe(1, "nivel acima do esperado nao estoura a barra");
  });
});
```

- [ ] **Step 2: Rode e confirme que falha**

```bash
cd apps/desktop && npx vitest run src/ondaSonora.test.ts
```
Esperado: FAIL — `Failed to resolve import "./ondaSonora"`.

- [ ] **Step 3: Escreva o módulo**

Crie `apps/desktop/src/ondaSonora.ts`:

```ts
/**
 * A onda do card de gravação: a janela de níveis e a altura de cada barra.
 *
 * Vive fora do componente para poder ser testada — não há teste de DOM neste
 * repo, então a regra tem de ser função pura. O componente desenha; aqui está o
 * que ele desenha.
 */

/** Trinta barras a 15 Hz são dois segundos de história. Menos que isso não
 *  mostra a cadência da fala; mais vira gráfico, e gráfico é o cockpit que o
 *  desenho recusa. */
export const BARRAS = 30;

/** Os degraus do modo sem movimento. Oito, os mesmos que a barra da topbar
 *  usava antes de o nível mudar de casa. */
export const DEGRAUS = 8;

/** O nível que o backend chama de cheio. RMS em milésimos, como o tick. */
const TETO = 1000;

/** A menor altura visível. Silêncio precisa desenhar ALGUMA coisa: uma barra de
 *  altura zero leria como "o áudio morreu", e silêncio não é queda — distinguir
 *  os dois é a razão de a onda existir. */
const PISO = 0.08;

/**
 * Empurra um nível pela direita e descarta o mais velho.
 *
 * Uma janela curta demais é preenchida com silêncio à esquerda, e não deixada
 * curta: uma onda que cresce da esquerda nos dois primeiros segundos parece
 * animação de entrada, e não medida.
 */
export function empurrar(janela: number[], nivel: number): number[] {
  const base = janela.length >= BARRAS
    ? janela.slice(janela.length - BARRAS + 1)
    : [...Array.from({ length: BARRAS - 1 - janela.length }, () => 0), ...janela];
  return [...base, nivel];
}

/** A altura da barra, de 0 a 1. Satura no teto em vez de estourar: um pico
 *  acima do esperado não deve desenhar fora da caixa. */
export function alturaDaBarra(nivel: number): number {
  const bruto = Math.max(0, nivel) / TETO;
  return Math.min(1, Math.max(PISO, bruto));
}

/** Quantos degraus acender, para quem pediu menos movimento.
 *
 *  Oito, como os da barra antiga — e nao e nostalgia: com
 *  `prefers-reduced-motion` a onda deixa de mostrar HISTORIA e passa a mostrar
 *  so o agora, porque a historia e justamente o que rola. O que sobra ainda
 *  distingue silencio de queda, que e a razao de a onda existir. */
export function degrausAcesos(nivel: number): number {
  const bruto = Math.max(0, nivel) / TETO;
  return Math.min(DEGRAUS, Math.max(1, Math.round(bruto * DEGRAUS)));
}
```

- [ ] **Step 4: Rode e confirme que passa**

```bash
cd apps/desktop && npx vitest run src/ondaSonora.test.ts
```
Esperado: PASS, 7 testes.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/ondaSonora.ts apps/desktop/src/ondaSonora.test.ts
git commit -m "feat(onda): a janela de niveis e a altura da barra, testadas"
```

---

### Task 6: A coluna `notes` e o autosave

**Files:**
- Create: `crates/mos-storage-sqlite/migrations/0022_meeting_notes.sql`
- Modify: `crates/mos-storage-sqlite/src/lib.rs` (constante e aplicação, e `SCHEMA_VERSION` para 22)
- Modify: `crates/mos-storage-sqlite/src/meeting_repository.rs` (SELECT e um `set_notes`)
- Modify: `crates/mos-core/src/meeting.rs` (campo `notes` na struct `Meeting`)
- Modify: `apps/desktop/src-tauri/src/meeting.rs`, `apps/desktop/src/api.ts`, `apps/desktop/src/types.ts`

**Interfaces:**
- Produces: `Meeting.notes: String`; `api.meetingSetNotes(id: string, notes: string): Promise<Meeting>`.

- [ ] **Step 1: Escreva a migration**

Crie `crates/mos-storage-sqlite/migrations/0022_meeting_notes.sql`:

```sql
-- As anotacoes de quem gravou.
--
-- `DEFAULT ''` e NOT NULL, e nao nullable: aqui a ausencia NAO significa "o que
-- o desenho escolheu", como na 0021 — significa que ninguem escreveu nada. Uma
-- nota vazia e um fato comum e completo, e um NULL so acrescentaria um segundo
-- jeito de dizer a mesma coisa.
--
-- Reuniao gravada antes desta coluna le string vazia, e nao erro.
--
-- Texto puro, sem formatacao: o M/OS nao tem editor rico em lugar nenhum, e
-- introduzir um aqui seria a maior peca da feature pela menor razao.

BEGIN IMMEDIATE;

ALTER TABLE meetings ADD COLUMN notes TEXT NOT NULL DEFAULT '';

PRAGMA user_version = 22;

COMMIT;
```

- [ ] **Step 2: Registre a migration e suba a versão**

Em `crates/mos-storage-sqlite/src/lib.rs`, depois da `MIGRATION_021`:

```rust
const MIGRATION_022: &str = include_str!("../migrations/0022_meeting_notes.sql");
```

Depois do bloco `if current <= 20`:

```rust
    if current <= 21 {
        connection
            .execute_batch(MIGRATION_022)
            .map_err(map_sql_error)?;
    }
```

E `const SCHEMA_VERSION: u32 = 22;` — os testes de upgrade comparam com esta constante e falham em bloco se ela ficar para trás.

- [ ] **Step 3: Escreva o teste que falha**

No `mod tests` de `crates/mos-storage-sqlite/src/meeting_repository.rs`:

```rust
    #[test]
    fn notas_gravam_leem_e_nascem_vazias() {
        let (_dir, storage) = storage();
        let reuniao = storage.create_meeting(nova_reuniao_de_teste()).unwrap();
        // Nasce vazia, e nao nula: ninguem escreveu ainda.
        assert_eq!(reuniao.notes, "");

        let salva = storage.set_meeting_notes(reuniao.id, "orcamento ate sexta").unwrap();
        assert_eq!(salva.notes, "orcamento ate sexta");

        let relida = storage.meeting(reuniao.id).unwrap().unwrap();
        assert_eq!(relida.notes, "orcamento ate sexta");
    }
```

Use os helpers que os testes vizinhos deste arquivo já usam para criar reunião — leia o `mod tests` e repita a forma, em vez de inventar `nova_reuniao_de_teste`.

- [ ] **Step 4: Rode e confirme que falha**

```bash
cargo test -p mos-storage-sqlite notas_gravam
```
Esperado: FAIL — campo `notes` inexistente.

- [ ] **Step 5: Implemente**

Acrescente `pub notes: String` à struct `Meeting` em `crates/mos-core/src/meeting.rs`, inclua a coluna em todos os `SELECT` de `meeting_repository.rs`, e escreva:

```rust
    fn set_meeting_notes(&self, id: MeetingId, notes: &str) -> Result<Meeting, CoreError> {
        {
            let connection = self.connection.lock().map_err(map_lock_error)?;
            connection
                .execute(
                    "UPDATE meetings SET notes = ?2, updated_at = ?3 WHERE id = ?1",
                    params![id.to_string(), notes, format_time(OffsetDateTime::now_utc())?],
                )
                .map_err(map_sql_error)?;
        }
        self.meeting(id)?.ok_or_else(|| {
            CoreError::new(mos_core::ErrorCode::NotFound, "Reuniao nao encontrada.", false)
        })
    }
```

Declare no port e exponha no serviço, seguindo a forma das funções vizinhas de meeting.

- [ ] **Step 6: Ligue o comando e o cliente**

Comando `meeting_set_notes(id: String, notes: String)` em `apps/desktop/src-tauri/src/meeting.rs`, registrado no `invoke_handler`, e no `api.ts`:

```ts
  // Autosave: chamado com debounce pela tela. Sem botao de salvar, porque um
  // botao de salvar numa nota de reuniao e uma chance de perder o que se
  // escreveu.
  meetingSetNotes(id: string, notes: string) {
    return invoke<Meeting>("meeting_set_notes", { id, notes });
  },
```

E `notes: string;` no tipo `Meeting` de `types.ts`.

- [ ] **Step 7: Rode tudo**

```bash
cargo test -p mos-core -p mos-storage-sqlite
cd apps/desktop && npx tsc --noEmit
```
Esperado: verde nos dois.

- [ ] **Step 8: Commit**

```bash
git add crates/ apps/desktop/src-tauri/src apps/desktop/src/api.ts apps/desktop/src/types.ts
git commit -m "feat(reuniao): a coluna de anotacoes, e o comando que a grava"
```

---

### Task 7: As notas no prompt, como contexto e não como fonte de item

**Files:**
- Modify: `crates/mos-core/src/meeting_analysis.rs:441` (`instructions`) e a montagem das janelas
- Test: `crates/mos-core/src/meeting_analysis.rs` (módulo `tests`)

**Interfaces:**
- Consumes: `Meeting.notes` (Task 6).
- Produces: `instructions(title: &str, notes: &str) -> String`.

- [ ] **Step 1: Escreva o teste que falha**

```rust
    #[test]
    fn as_notas_entram_como_contexto_e_a_regra_de_evidencia_fica() {
        let com = instructions("Obra X", "cliente quer o orcamento ate sexta");
        assert!(com.contains("NOTAS DE QUEM GRAVOU"));
        assert!(com.contains("cliente quer o orcamento ate sexta"));
        // A regra que sustenta o "aceitar num clique" nao pode afrouxar.
        assert!(com.contains("pelo menos um `segment`"));
        assert!(
            com.contains("nao servem de evidencia"),
            "o prompt precisa dizer que a nota NAO ancora item"
        );

        // Sem notas, o bloco nao existe: um cabecalho vazio ensinaria o modelo a
        // procurar conteudo que nao esta la.
        let sem = instructions("Obra X", "   ");
        assert!(!sem.contains("NOTAS DE QUEM GRAVOU"));
    }
```

- [ ] **Step 2: Rode e confirme que falha**

```bash
cargo test -p mos-core as_notas_entram
```
Esperado: FAIL — `instructions` recebe um argumento só.

- [ ] **Step 3: Implemente**

Mude a assinatura para `pub fn instructions(title: &str, notes: &str) -> String` e monte o bloco antes do resto:

```rust
    let bloco = if notes.trim().is_empty() {
        String::new()
    } else {
        format!(
            "NOTAS DE QUEM GRAVOU (contexto, nao transcricao):\n\
             {}\n\
             \n\
             Elas dizem o que importou para quem estava na reuniao. Use para o\n\
             resumo e para desambiguar. Elas NAO foram ditas em voz alta, entao\n\
             nao servem de evidencia e nao geram item sozinhas.\n\
             \n",
            notes.trim()
        )
    };
```

E acrescente à lista de regras, junto das que já existem:

```rust
         - as notas acima sao contexto: nenhum item pode ter como unica base\n\
         \x20 uma nota, porque nota nao tem `segment`.\n\
```

**Não mexa em `parse_analysis`.** A validação que exige evidência é o que garante a regra mesmo quando o modelo ignora a instrução — é por isso que ela fica onde está.

- [ ] **Step 4: Ajuste quem chama**

O `tsc` não ajuda aqui; o compilador do Rust sim. Rode `cargo check -p mos-core` e corrija cada chamador de `instructions`, passando as notas da reunião.

- [ ] **Step 5: Rode e confirme que passa**

```bash
cargo test -p mos-core
```
Esperado: PASS, incluindo os testes de janela que já existiam.

- [ ] **Step 6: Commit**

```bash
git add crates/mos-core/src/meeting_analysis.rs
git commit -m "feat(analise): as notas sobem como contexto, sem afrouxar a evidencia"
```

---

### Task 8: O card na tela

**Files:**
- Create: `apps/desktop/src/CardGravacao.tsx`
- Modify: `apps/desktop/src/MeetingsPage.tsx:318` (o `view`), `:521` (o detalhe), `:535` (o `segmented`)
- Modify: `apps/desktop/src/App.css`

**Interfaces:**
- Consumes: `empurrar`, `alturaDaBarra`, `BARRAS` (Task 5); `api.meetingPause`, `meetingResume` (Task 3); `api.meetingSetNotes` (Task 6); evento `meeting-level` (Task 4).
- Produces: `<CardGravacao meeting={Meeting} onMudou={(m: Meeting) => void} />`.

- [ ] **Step 1: Escreva o componente**

Crie `apps/desktop/src/CardGravacao.tsx`. Ele monta a onda ouvindo `meeting-level`, faz o autosave das notas com debounce de 800 ms, e oferece Pausar/Retomar e Parar.

```tsx
import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Button } from "./Button";
import { BARRAS, DEGRAUS, alturaDaBarra, degrausAcesos, empurrar } from "./ondaSonora";
import type { Meeting, MeetingLevel } from "./types";

/**
 * O card da reunião em curso.
 *
 * Só existe em `recording` e `paused` — é o posto de trabalho da reunião, e não
 * um indicador. O indicador continua na topbar, porque a §17.2 promete que ele
 * apareça em QUALQUER tela, e esta é uma só.
 */
export function CardGravacao({ meeting, onMudou }: {
  meeting: Meeting;
  onMudou: (meeting: Meeting) => void;
}) {
  const [janela, setJanela] = useState<number[]>([]);
  // O repo le `prefers-reduced-motion` so em CSS, em sete blocos. Aqui NAO da:
  // "parar de rolar" e decisao de DADO, nao de estilo — uma onda que se
  // redesenha 15x por segundo e movimento por mais que nenhuma transicao exista.
  // Este e o primeiro leitor em JS, e e por isso que ele carrega esta nota.
  const [semMovimento] = useState(
    () => window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );
  const [notas, setNotas] = useState(meeting.notes);
  const [erro, setErro] = useState("");
  const pausada = meeting.status === "paused";
  const gravado = useRef(meeting.notes);

  // A onda ouve o evento de 15 Hz. Pausada, ela congela em vez de desenhar
  // silêncio: silêncio é "ninguém falou", e pausado é "não estou ouvindo" — a
  // onda não pode dizer a mesma coisa nos dois casos.
  useEffect(() => {
    if (pausada) return;
    const off = listen<MeetingLevel>("meeting-level", (evento) => {
      setJanela((atual) => empurrar(atual, Math.max(evento.payload.mic, evento.payload.system)));
    });
    return () => { void off.then((fn) => fn()); };
  }, [pausada]);

  // Autosave com debounce. `gravado` guarda o que já foi ao banco para não
  // reenviar o mesmo texto quando a tela recarrega por outro motivo.
  useEffect(() => {
    if (notas === gravado.current) return;
    const timer = setTimeout(() => {
      api.meetingSetNotes(meeting.id, notas)
        .then((atualizada) => { gravado.current = notas; onMudou(atualizada); })
        .catch((causa) => setErro(causa instanceof Error ? causa.message : String(causa)));
    }, 800);
    return () => clearTimeout(timer);
  }, [notas, meeting.id, onMudou]);

  const alternarPausa = useCallback(async () => {
    setErro("");
    try {
      onMudou(pausada ? await api.meetingResume() : await api.meetingPause());
    } catch (causa) {
      setErro(causa instanceof Error ? causa.message : String(causa));
    }
  }, [pausada, onMudou]);

  return (
    <section className="card-gravacao" data-pausada={pausada || undefined}>
      <div className="card-gravacao-barra">
        {semMovimento ? (
          /* Sem movimento: oito degraus mostrando o AGORA, sem historia. A
             historia e justamente o que rola, e rolar e o que foi pedido para
             parar. O que sobra ainda distingue silencio de queda. */
          <span className="onda" data-degraus="" aria-hidden="true">
            {Array.from({ length: DEGRAUS }, (_, i) => (
              <i key={i} data-on={i < degrausAcesos(janela[janela.length - 1] ?? 0) || undefined} />
            ))}
          </span>
        ) : (
          <span className="onda" aria-hidden="true">
            {Array.from({ length: BARRAS }, (_, i) => (
              <i key={i} style={{ "--h": String(alturaDaBarra(janela[i] ?? 0)) } as React.CSSProperties} />
            ))}
          </span>
        )}
        {/* O nível é decorativo para quem lê por leitor de tela: o estado que
            importa é gravando ou pausado, e ele é dito em palavras. */}
        <span className="visually-hidden" aria-live="polite">
          {pausada ? "Gravação pausada" : "Gravando"}
        </span>
        <Button variant="outline" size="sm" onClick={() => void alternarPausa()}>
          {pausada ? "Retomar" : "Pausar"}
        </Button>
      </div>

      <label className="visually-hidden" htmlFor="notas-da-reuniao">Anotações</label>
      <textarea
        id="notas-da-reuniao"
        className="card-gravacao-notas"
        value={notas}
        placeholder="O que você escrever aqui sobe junto com a transcrição."
        onChange={(evento) => setNotas(evento.currentTarget.value)}
      />

      {erro ? <p className="support-copy" role="alert">{erro}</p> : null}
    </section>
  );
}
```

- [ ] **Step 2: Escreva o CSS**

No fim de `apps/desktop/src/App.css`, antes do bloco `@media (max-width: 960px)`:

```css
/* --- O card de gravacao --------------------------------------------------- */

.card-gravacao {
  display: grid;
  gap: var(--space-3);
  padding: var(--space-4);
  background: var(--surface-raised);
  border: var(--line) solid var(--border-strong);
  border-radius: var(--radius);
}

.card-gravacao-barra {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

/* A onda: trinta barras, dois segundos de historia. Altura vem do `--h` que o
   componente calcula, e nao de uma animacao — o movimento E o dado. */
.onda {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 2px;
  height: var(--height-control);
}

.onda i {
  display: block;
  flex: 1;
  min-height: 2px;
  height: calc(var(--h) * 100%);
  background: var(--signal-ink);
  border-radius: 1px;
  transition: height 80ms linear;
}

/* Pausada, a onda esvazia a cor mas NAO some: sumir diria "acabou", e pausado
   nao e acabado. */
.card-gravacao[data-pausada] .onda i {
  background: var(--border-control);
}

.card-gravacao-notas {
  min-height: 8rem;
  padding: var(--space-3);
  color: var(--text);
  font: var(--text-ui);
  background: var(--surface);
  border: var(--line) solid var(--border);
  border-radius: var(--radius);
  resize: vertical;
}

.card-gravacao-notas:focus-visible {
  border-color: var(--signal-ink);
  outline: none;
}

/* Sem movimento, o componente troca de desenho (nao so de transicao): sao oito
   degraus do agora, no lugar de trinta barras de historia. O CSS so acompanha. */
.onda[data-degraus] i {
  flex: none;
  width: 3px;
  height: 12px;
  background: var(--border-control);
  transition: none;
}

.onda[data-degraus] i[data-on] {
  background: var(--signal-ink);
}

@media (prefers-reduced-motion: reduce) {
  .onda i {
    transition: none;
  }
}
```

- [ ] **Step 3: Monte na página**

Em `apps/desktop/src/MeetingsPage.tsx`, troque a união do estado `view` na linha 318 para incluir a aba nova:

```tsx
  const [view, setView] = useState<"overview" | "transcript" | "notes">("overview");
```

Acrescente o botão da aba ao `segmented` que já existe (linha ~535), **depois** de "Transcrição":

```tsx
                <button
                  role="tab"
                  aria-selected={view === "notes"}
                  onClick={() => setView("notes")}
                >Anotações</button>
```

E monte o card logo antes do `<MeetingActions ...>`, só nos dois estados em que ele existe:

```tsx
              {chosen.status === "recording" || chosen.status === "paused" ? (
                <CardGravacao meeting={chosen} onMudou={(atualizada) => { setChosen(atualizada); void loadList(); }} />
              ) : null}
```

Use o nome real do setter do detalhe escolhido nesta página — leia como `chosen` é obtido antes de escrever `setChosen`.

- [ ] **Step 4: A aba Transcrição explica em vez de ficar vazia**

No corpo do `view === "transcript"`, quando o status for `recording` ou `paused`, mostre a explicação em vez da lista vazia:

```tsx
                {chosen.status === "recording" || chosen.status === "paused" ? (
                  <p className="support-copy">
                    A transcrição chega quando você parar. Ela é feita de uma vez, com a
                    reunião inteira — transcrever pedaços soltos corta palavras na emenda
                    e perde o contexto que desambigua.
                  </p>
                ) : (
                  /* o que a tela ja mostra hoje */
                  null
                )}
```

Substitua o `null` do ramo de baixo pelo conteúdo de transcrição que a página já renderiza — não o duplique, mova-o.

- [ ] **Step 5: Confirme que compila e testa**

```bash
cd apps/desktop && npx tsc --noEmit && npx vitest run
```
Esperado: sem erro; todos os testes passando.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/CardGravacao.tsx apps/desktop/src/MeetingsPage.tsx apps/desktop/src/App.css
git commit -m "feat(reuniao): o card de gravacao, com onda, notas e pausa"
```

---

### Task 9: A barra da topbar encolhe

**Files:**
- Modify: `apps/desktop/src/RecordingBar.tsx`
- Modify: `apps/desktop/src/App.css` (bloco `.recording-bar`)

- [ ] **Step 1: Tire o nível e trate a pausa**

Em `RecordingBar.tsx`, remova o componente `Level` e o `<Channel>` do JSX — eles mudam de casa para o card. **Mantenha** o `data-warning` de `bothGone`: ele é alarme de canal perdido, não medida, e continua valendo em qualquer tela.

Atualize o comentário de topo do arquivo, que hoje justifica o nível discreto, para dizer onde ele foi parar e por quê:

```tsx
 * O nível saiu daqui e virou onda no card da página Reuniões. A razão é a mesma
 * que antes justificava tê-lo: a pergunta que a forma responde. Aqui a barra
 * acompanha você por telas que não são sobre a reunião, e o que importa é
 * "estou gravando?" e "perdi o áudio?" — a primeira o relógio responde, a
 * segunda o `data-warning` responde. "Está me ouvindo agora?" é pergunta de
 * quem está na reunião, e é lá que ela é respondida.
```

E o ponto vermelho para de pulsar quando pausado:

```tsx
      <span className="recording-dot" data-pausada={tick.status === "paused" || undefined} aria-hidden="true" />
```

mais o rótulo:

```tsx
      {tick.status === "paused" ? <span className="micro-label">PAUSADO</span> : null}
```

Isto exige que o `MeetingTick` carregue o status. Se ele ainda não carrega, acrescente o campo no Rust (`apps/desktop/src-tauri/src/meeting.rs`, struct do tick) e em `types.ts` — leia a struct antes de assumir.

- [ ] **Step 2: O CSS da pausa**

```css
/* Pausado, o ponto PARA. Um ponto pulsando com o microfone fechado e a mentira
   exata que a §17.2 existe para impedir — e ela e pior que nao ter indicacao,
   porque ensina a confiar num sinal falso. */
.recording-dot[data-pausada] {
  animation: none;
  background: var(--text-disabled);
}
```

- [ ] **Step 3: Confirme**

```bash
cd apps/desktop && npx tsc --noEmit && npx vitest run
```

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/RecordingBar.tsx apps/desktop/src/App.css
git commit -m "fix(reuniao): a barra encolhe, e o ponto para de pulsar na pausa"
```

---

### Task 10: Fechamento

- [ ] **Step 1: Rode tudo**

```bash
export TMP="$SCRATCH/tmp"; export TEMP="$TMP"
cargo test -p mos-core -p mos-storage-sqlite -p mos-audio
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cd apps/desktop && npx tsc --noEmit && npx vitest run
```

- [ ] **Step 2: Confirme a migration no banco de verdade**

```bash
python -c "import sqlite3,os; db=os.path.expandvars(r'%APPDATA%\com.codedbym.mos\m-os.db'); con=sqlite3.connect(f'file:{db}?mode=ro',uri=True); print(con.execute('PRAGMA user_version').fetchone()); print([r[1] for r in con.execute('PRAGMA table_info(meetings)')])"
```
Esperado: `(22,)` e `notes` na lista, **depois** de abrir o app uma vez.

- [ ] **Step 3: Exercite de verdade — este é o gate**

Pela skill `ver-o-app`, com o app rodando. Há duas reuniões já gravadas no banco e o whisper instalado, então a cadeia inteira pode rodar:

1. inicie uma gravação e confirme que o card aparece, com a onda se mexendo ao falar;
2. **pause**, e confirme as três coisas juntas: a onda congela e esvazia a cor, o ponto da topbar para de pulsar e fica cinza, e a barra diz `PAUSADO`;
3. retome, escreva uma nota, e confirme no banco que ela chegou:
   ```bash
   python -c "import sqlite3,os; db=os.path.expandvars(r'%APPDATA%\com.codedbym.mos\m-os.db'); con=sqlite3.connect(f'file:{db}?mode=ro',uri=True); print(con.execute('select title,status,notes from meetings order by started_at desc limit 1').fetchall())"
   ```
4. pare, transcreva, e **confirme que a duração NÃO conta o tempo pausado** — é a asserção central da Task 1, e só uma gravação real a prova;
5. analise, e confirme que o resumo reflete a nota **sem** que apareça um item ancorado nela;
6. fotografe o card em 1280 e 840, nos dois temas, gravando e pausado, e **olhe cada imagem**.

- [ ] **Step 4: Relate honestamente**

Diga o que foi verificado por foto, o que foi exercitado de verdade, e o que não. Em particular: escalas de 125%/150% não são observáveis nesta máquina.
