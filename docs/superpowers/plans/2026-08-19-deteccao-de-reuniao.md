# A detecção de reunião pelo microfone — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Oferecer a gravação quando uma reunião começa, detectando qual processo abriu o microfone — sem ler título de janela, conteúdo ou áudio.

**Architecture:** A leitura do registro fica isolada num módulo Windows-only que devolve fatos (`quem` e `desde quando`). A **decisão** de oferecer é função pura em `mos-core`, testada sem Windows. O laço do `monitor.rs` liga as duas e mostra uma janela nova, irmã da do lembrete.

**Tech Stack:** Rust (Tauri 2, `winreg` — já na árvore por caminho transitivo), React 18 + TypeScript, CSS com tokens de `packages/design-system/tokens.css`.

## Global Constraints

- **Nada de título de janela, conteúdo de aba ou áudio.** O único dado novo é *qual processo tem o microfone aberto, e desde quando*. Se um passo pedir mais que isso, ele está errado.
- **Três exclusões, sempre:** o próprio `mos-desktop.exe`; qualquer processo enquanto o M/OS já grava; processo silenciado.
- **20 segundos contínuos** de microfone aberto antes de oferecer. O contador zera quando o microfone fecha.
- **Com mais de um processo, ganha o aberto há mais tempo** — e o alvo importa só para o `Não neste app`.
- **A janela não diz "IA".** O que se inicia é uma gravação; a análise vem depois, com consentimento próprio.
- **Ligada de fábrica**, com toggle em Settings → REUNIÕES, fácil de achar e não enterrado em Avançado.
- **Comentários e commits em português, sem acento dentro de `.rs`.** Em `.ts`, `.tsx`, `.md` e `.sql` o acento é normal.
- **Não existe teste de DOM neste repo**: o que for testado tem de ser função pura.
- Antes de qualquer `cargo`: `export TMP="<scratchpad>/tmp"; export TEMP="$TMP"`.
- Verificação visual pela skill `ver-o-app`; `orca computer` não funciona nesta máquina.

---

### Task 1: A regra de oferecer, em função pura

**Files:**
- Modify: `crates/mos-core/src/monitoring.rs` (ao lado de `diff_transitions`)
- Modify: `crates/mos-core/src/lib.rs` (re-export)
- Test: `crates/mos-core/src/monitoring.rs` (módulo `tests`)

**Interfaces:**
- Produces: `MicrofoneAberto { processo: String, segundos_aberto: i64 }`; `DecisaoDeOferta`; `decidir_oferta(abertos: &[MicrofoneAberto], contexto: &ContextoDaOferta) -> DecisaoDeOferta`; `ContextoDaOferta { gravando: bool, silenciados: BTreeSet<String>, ligado: bool, espera_segundos: i64 }`.

- [ ] **Step 1: Escreva os testes que falham**

No `mod tests` de `crates/mos-core/src/monitoring.rs`:

```rust
    fn contexto() -> ContextoDaOferta {
        ContextoDaOferta {
            gravando: false,
            silenciados: std::collections::BTreeSet::new(),
            ligado: true,
            espera_segundos: 20,
        }
    }

    fn aberto(processo: &str, segundos: i64) -> MicrofoneAberto {
        MicrofoneAberto {
            processo: processo.to_string(),
            segundos_aberto: segundos,
        }
    }

    #[test]
    fn oferece_quando_um_processo_passa_da_espera() {
        let decisao = decidir_oferta(&[aberto("chrome.exe", 25)], &contexto());
        assert_eq!(decisao, DecisaoDeOferta::Oferecer("chrome.exe".into()));
    }

    #[test]
    fn nao_oferece_antes_da_espera() {
        // Microfone que abre por dois segundos e teste de som, atalho de
        // push-to-talk, notificacao. Reuniao mantem aberto.
        assert_eq!(
            decidir_oferta(&[aberto("chrome.exe", 5)], &contexto()),
            DecisaoDeOferta::Nada
        );
    }

    #[test]
    fn o_proprio_mos_nao_conta() {
        // `mos-desktop.exe` esta no ConsentStore, e gravar abre o microfone. Sem
        // esta exclusao o detector se veria gravando e ofereceria gravar.
        assert_eq!(
            decidir_oferta(&[aberto("mos-desktop.exe", 300)], &contexto()),
            DecisaoDeOferta::Nada
        );
        // E nem com maiuscula diferente, que e como o Windows costuma devolver.
        assert_eq!(
            decidir_oferta(&[aberto("MOS-Desktop.EXE", 300)], &contexto()),
            DecisaoDeOferta::Nada
        );
    }

    #[test]
    fn nao_oferece_durante_gravacao() {
        let mut ctx = contexto();
        ctx.gravando = true;
        assert_eq!(
            decidir_oferta(&[aberto("chrome.exe", 300)], &ctx),
            DecisaoDeOferta::Nada
        );
    }

    #[test]
    fn nao_oferece_para_processo_silenciado() {
        let mut ctx = contexto();
        ctx.silenciados.insert("chrome.exe".into());
        assert_eq!(
            decidir_oferta(&[aberto("chrome.exe", 300)], &ctx),
            DecisaoDeOferta::Nada
        );
        // Mas o silencio de um nao cala o outro.
        assert_eq!(
            decidir_oferta(&[aberto("chrome.exe", 300), aberto("zoom.exe", 100)], &ctx),
            DecisaoDeOferta::Oferecer("zoom.exe".into())
        );
    }

    #[test]
    fn desligado_nao_oferece_nada() {
        let mut ctx = contexto();
        ctx.ligado = false;
        assert_eq!(
            decidir_oferta(&[aberto("chrome.exe", 300)], &ctx),
            DecisaoDeOferta::Nada
        );
    }

    #[test]
    fn com_varios_ganha_o_aberto_ha_mais_tempo() {
        // O Discord fica aberto ao lado do Meet. Quem abriu primeiro
        // provavelmente e a reuniao; quem abriu depois costuma ser o acessorio.
        let decisao = decidir_oferta(
            &[aberto("discord.exe", 40), aberto("chrome.exe", 120)],
            &contexto(),
        );
        assert_eq!(decisao, DecisaoDeOferta::Oferecer("chrome.exe".into()));
    }
```

- [ ] **Step 2: Rode e confirme que falha**

```bash
export TMP="$SCRATCH/tmp"; export TEMP="$TMP"; mkdir -p "$TMP"
cargo test -p mos-core oferece
```
Esperado: FAIL com `cannot find function 'decidir_oferta'`.

- [ ] **Step 3: Implemente**

Em `crates/mos-core/src/monitoring.rs`, ao lado de `diff_transitions`:

```rust
/// Um processo com o microfone aberto, e ha quanto tempo.
///
/// So isto atravessa a fronteira: **quem** e **desde quando**. Nao ha titulo de
/// janela, nao ha conteudo, nao ha audio — e a ausencia deles e a feature.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MicrofoneAberto {
    pub processo: String,
    pub segundos_aberto: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisaoDeOferta {
    Oferecer(String),
    Nada,
}

/// O que o laco sabe quando pergunta.
#[derive(Clone, Debug)]
pub struct ContextoDaOferta {
    pub gravando: bool,
    pub silenciados: std::collections::BTreeSet<String>,
    pub ligado: bool,
    pub espera_segundos: i64,
}

/// O processo do proprio M/OS, que nunca dispara a oferta.
const EU_MESMO: &str = "mos-desktop.exe";

/// Decide se ha oferta a fazer, e para qual processo.
///
/// Pura, e por isso testavel sem Windows: o que decide e um conjunto de fatos, e
/// nao o sistema operacional. O laco que a chama e a unica parte que precisa de
/// uma maquina de verdade — mesma divisao de `diff_transitions`.
pub fn decidir_oferta(
    abertos: &[MicrofoneAberto],
    contexto: &ContextoDaOferta,
) -> DecisaoDeOferta {
    if !contexto.ligado || contexto.gravando {
        return DecisaoDeOferta::Nada;
    }

    let alvo = abertos
        .iter()
        .filter(|entrada| !entrada.processo.eq_ignore_ascii_case(EU_MESMO))
        .filter(|entrada| !contexto.silenciados.contains(&entrada.processo.to_lowercase()))
        .filter(|entrada| entrada.segundos_aberto >= contexto.espera_segundos)
        // Ganha o aberto ha MAIS tempo. `max_by_key` devolve o ultimo em caso de
        // empate, e empate aqui e irrelevante: os dois sao candidatos iguais.
        .max_by_key(|entrada| entrada.segundos_aberto);

    match alvo {
        Some(entrada) => DecisaoDeOferta::Oferecer(entrada.processo.clone()),
        None => DecisaoDeOferta::Nada,
    }
}
```

**Atenção ao silenciamento:** o teste insere `"chrome.exe"` em minúscula e o filtro compara com `to_lowercase()`. O `monitor.rs` já guarda os silenciados em minúscula (`suppress` faz `to_lowercase()`), então quem preencher o contexto precisa manter essa convenção.

- [ ] **Step 4: Exporte**

Em `crates/mos-core/src/lib.rs`, no bloco `pub use monitoring::{...}`, acrescente `decidir_oferta, ContextoDaOferta, DecisaoDeOferta, MicrofoneAberto`.

- [ ] **Step 5: Rode e confirme que passa**

```bash
cargo test -p mos-core
```
Esperado: PASS, com os sete testes novos.

- [ ] **Step 6: Commit**

```bash
git add crates/mos-core/src
git commit -m "feat(deteccao): a regra de oferecer gravacao, pura e testada"
```

---

### Task 2: Ler o ConsentStore do microfone

**Files:**
- Create: `apps/desktop/src-tauri/src/microfone.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs` (declarar o módulo)
- Modify: `apps/desktop/src-tauri/Cargo.toml` (dependência `winreg`)

**Interfaces:**
- Consumes: `mos_core::MicrofoneAberto` (Task 1).
- Produces: `microfone::abertos_agora() -> Vec<MicrofoneAberto>`.

- [ ] **Step 1: Declare a dependência**

Em `apps/desktop/src-tauri/Cargo.toml`, junto de `windows-sys`:

```toml
winreg = "0.52"
```

O crate já aparece na `Cargo.lock` por caminho transitivo; declarar torna o uso direto explícito, em vez de depender de uma dependência de terceiro continuar existindo.

- [ ] **Step 2: Escreva o módulo**

Crie `apps/desktop/src-tauri/src/microfone.rs`:

```rust
//! Quem esta com o microfone aberto, segundo o Windows.
//!
//! **Somente leitura de registro.** Sem hook, sem injecao, sem captura. O unico
//! dado que sai daqui e QUEM e DESDE QUANDO — nunca titulo de janela, nunca
//! conteudo, nunca audio. A ADR-046 depende dessa estreiteza.
//!
//! O Windows mantem dois caminhos, e os DOIS importam: apps da Store ficam
//! direto sob `microphone`, apps Win32 sob `microphone\NonPackaged` com o
//! caminho do executavel e as barras trocadas por `#`. Ler so um deixaria
//! buracos que se parecem com "as vezes nao funciona".
//!
//! `LastUsedTimeStop == 0` significa EM USO AGORA. `LastUsedTimeStart` e
//! FILETIME — 100 ns desde 1601 —, e e dele que sai ha quanto tempo.

use std::collections::BTreeSet;

use mos_core::MicrofoneAberto;

const CONSENT: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone";

/// Segundos entre 1601-01-01 e 1970-01-01, para converter FILETIME em epoch.
const EPOCH_1601_PARA_1970: i64 = 11_644_473_600;

#[cfg(windows)]
pub fn abertos_agora() -> Vec<MicrofoneAberto> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    let agora = time::OffsetDateTime::now_utc().unix_timestamp();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(raiz) = hkcu.open_subkey_with_flags(CONSENT, KEY_READ) else {
        return Vec::new();
    };

    let mut encontrados = Vec::new();
    let mut vistos = BTreeSet::new();

    for nome in raiz.enum_keys().flatten() {
        if nome.eq_ignore_ascii_case("NonPackaged") {
            let Ok(sub) = raiz.open_subkey_with_flags(&nome, KEY_READ) else {
                continue;
            };
            for chave in sub.enum_keys().flatten() {
                if let Some(entrada) = ler(&sub, &chave, agora) {
                    if vistos.insert(entrada.processo.clone()) {
                        encontrados.push(entrada);
                    }
                }
            }
        } else if let Some(entrada) = ler(&raiz, &nome, agora) {
            if vistos.insert(entrada.processo.clone()) {
                encontrados.push(entrada);
            }
        }
    }
    encontrados
}

#[cfg(not(windows))]
pub fn abertos_agora() -> Vec<MicrofoneAberto> {
    // Fora do Windows nao ha ConsentStore. Devolver vazio e o certo: a oferta
    // simplesmente nao acontece, e nada finge ter observado.
    Vec::new()
}

#[cfg(windows)]
fn ler(pai: &winreg::RegKey, chave: &str, agora: i64) -> Option<MicrofoneAberto> {
    use winreg::enums::KEY_READ;

    let entrada = pai.open_subkey_with_flags(chave, KEY_READ).ok()?;
    let parou: u64 = entrada.get_value("LastUsedTimeStop").ok()?;
    // Zero e o unico valor que significa "aberto agora".
    if parou != 0 {
        return None;
    }
    let comecou: u64 = entrada.get_value("LastUsedTimeStart").ok()?;
    let inicio = (comecou as i64) / 10_000_000 - EPOCH_1601_PARA_1970;
    Some(MicrofoneAberto {
        processo: nome_do_processo(chave),
        // `max(0)` porque relogio que anda para tras nao pode virar tempo
        // negativo aberto — isso passaria a espera de 20 s ao contrario.
        segundos_aberto: (agora - inicio).max(0),
    })
}

/// O nome do executavel a partir da chave do registro.
///
/// Win32: `C:#Program Files#Google#Chrome#Application#chrome.exe` vira
/// `chrome.exe`. App da Store: o nome da familia do pacote fica como esta — ele
/// ja e um identificador, e nao ha executavel a extrair.
#[cfg(windows)]
fn nome_do_processo(chave: &str) -> String {
    match chave.rsplit('#').next() {
        Some(ultimo) if ultimo.to_lowercase().ends_with(".exe") => ultimo.to_string(),
        _ => chave.to_string(),
    }
}
```

- [ ] **Step 3: Declare o módulo**

Em `apps/desktop/src-tauri/src/lib.rs`, junto dos outros `mod`:

```rust
mod microfone;
```

- [ ] **Step 4: Confirme que compila e olhe o resultado de verdade**

```bash
export TMP="$SCRATCH/tmp"; export TEMP="$TMP"
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
```
Esperado: sem avisos.

Não há teste unitário aqui **de propósito**: o resultado depende de qual app está com o microfone aberto na máquina de quem roda, e um teste que depende disso falha em CI e passa localmente por acaso. A regra que se pode testar é a da Task 1, e ela já está testada. O que se verifica aqui é comportamento real, na Task 6.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/microfone.rs apps/desktop/src-tauri/src/lib.rs apps/desktop/src-tauri/Cargo.toml
git commit -m "feat(deteccao): le o ConsentStore do microfone, e nada alem disso"
```

---

### Task 3: A janela da oferta

**Files:**
- Modify: `apps/desktop/src-tauri/tauri.conf.json` (janela `reuniao-detectada`)
- Create: `apps/desktop/src/ReuniaoDetectada.tsx`
- Modify: `apps/desktop/src/App.tsx` (o `switch` de `App()`, ~linha 3249)
- Modify: `apps/desktop/src/App.css`

**Interfaces:**
- Produces: janela de label `reuniao-detectada`; componente `<ReuniaoDetectada />`; evento Tauri `reuniao-detectada` com `{ processo: string; nome: string }`.

- [ ] **Step 1: Declare a janela**

Em `apps/desktop/src-tauri/tauri.conf.json`, no array `app.windows`, depois da entrada `lembrete`:

```json
      {
        "label": "reuniao-detectada",
        "title": "M/OS",
        "width": 380,
        "height": 132,
        "transparent": true,
        "visible": false,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "resizable": false,
        "maximizable": false,
        "minimizable": false,
        "decorations": false,
        "focus": false,
        "shadow": false
      }
```

`shadow: false` não é cópia distraída: sem ele o Windows desenha a borda branca de 1px que o commit `ddec664` removeu do lembrete, e a janela nasceria com o defeito já corrigido uma vez.

Janela **própria**, e não a do lembrete: compartilhar faria as duas disputarem o mesmo espaço no pior momento possível, que é durante uma reunião.

- [ ] **Step 2: Escreva o componente**

Crie `apps/desktop/src/ReuniaoDetectada.tsx`:

```tsx
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Button } from "./Button";

type Detectada = { processo: string; nome: string };

/**
 * A oferta que aparece quando um microfone abre.
 *
 * Ela **não rouba o foco** — quem está entrando numa reunião está clicando em
 * outra coisa, e capturar o teclado nesse instante é um acidente.
 *
 * E ela não diz "IA". O que se inicia aqui é uma GRAVAÇÃO; a análise vem depois,
 * por botão separado e com consentimento próprio. O Notion escreve "Iniciar
 * Anotações IA" e promete na hora errada.
 */
export function ReuniaoDetectada() {
  const [alvo, setAlvo] = useState<Detectada | null>(null);
  const [erro, setErro] = useState("");

  useEffect(() => {
    const off = listen<Detectada>("reuniao-detectada", (evento) => {
      setAlvo(evento.payload);
      setErro("");
    });
    return () => { void off.then((fn) => fn()); };
  }, []);

  async function agir(run: () => Promise<unknown>) {
    try {
      await run();
      await api.fecharReuniaoDetectada();
    } catch (causa) {
      // O erro fica AQUI. Mandar procurar o motivo no M/OS desfaz o motivo de a
      // janelinha existir.
      setErro(causa instanceof Error ? causa.message : String(causa));
    }
  }

  if (!alvo) return null;

  return (
    <main className="oferta-shell">
      <header className="oferta-head">
        <span className="micro-label">M/OS · REUNIÕES</span>
        <strong>{alvo.nome} abriu o microfone</strong>
      </header>

      {erro ? <p className="support-copy" role="alert">{erro}</p> : null}

      <div className="oferta-acoes">
        <Button variant="primary" size="sm" onClick={() => void agir(() => api.meetingStart("", null))}>
          Gravar reunião
        </Button>
        <Button variant="ghost" size="sm" onClick={() => void agir(async () => undefined)}>
          Agora não
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => void agir(() => api.silenciarDeteccao(alvo.processo))}
        >
          Não neste app
        </Button>
      </div>
    </main>
  );
}
```

- [ ] **Step 3: Ligue no roteamento**

Em `apps/desktop/src/App.tsx`, no `switch` de `App()` (perto da linha 3249), ao lado de `case "lembrete":`:

```tsx
    case "reuniao-detectada":
      return <ReuniaoDetectada />;
```

E o import no topo, junto dos outros de componente.

- [ ] **Step 4: Escreva o CSS**

No fim de `apps/desktop/src/App.css`, antes de `@media (max-width: 960px)`:

```css
/* --- A oferta de gravar (ADR-046) ----------------------------------------- */

/* Mesma forma do `.reminder-shell`: a margem e onde a sombra em CSS cabe, e
   cada pixel dela e transparente e engole clique — por isso ela e curta. */
.oferta-shell {
  display: grid;
  gap: var(--space-3);
  margin: 4px 10px 16px;
  padding: var(--space-3) var(--space-4);
  background: var(--surface-raised);
  border: var(--line) solid var(--border-strong);
  border-radius: var(--radius);
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.45);
}

[data-theme='light'] .oferta-shell {
  box-shadow: 0 6px 16px rgba(20, 24, 26, 0.16);
}

.oferta-head {
  display: grid;
  gap: var(--space-1);
}

.oferta-head .micro-label {
  color: var(--signal-ink);
}

.oferta-acoes {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}
```

- [ ] **Step 5: Confirme que compila**

```bash
cd apps/desktop && npx tsc --noEmit
```
Esperado: erros em `api.fecharReuniaoDetectada` e `api.silenciarDeteccao`, que a Task 4 cria. **Isto é esperado nesta etapa** — não invente os métodos aqui.

- [ ] **Step 6: Commit**

Comite junto com a Task 4, porque o componente não compila sem os comandos dela. Pule este passo.

---

### Task 4: Os comandos da janela

**Files:**
- Modify: `apps/desktop/src-tauri/src/monitor.rs` (dois comandos)
- Modify: `apps/desktop/src-tauri/src/lib.rs` (`invoke_handler`)
- Modify: `apps/desktop/src/api.ts`

**Interfaces:**
- Consumes: a janela `reuniao-detectada` (Task 3); `Monitor::suppress` já existente.
- Produces: `api.fecharReuniaoDetectada(): Promise<void>`; `api.silenciarDeteccao(processo: string): Promise<void>`.

- [ ] **Step 1: Escreva os comandos**

Em `apps/desktop/src-tauri/src/monitor.rs`, junto dos comandos que já existem:

```rust
/// Fecha a janelinha da oferta.
///
/// `hide`, e nao `close`: a janela sobrevive entre ofertas, como a do lembrete.
/// Fechar de verdade obrigaria a recria-la, e recriar custa o tempo em que a
/// oferta precisa aparecer.
#[tauri::command]
pub fn fechar_reuniao_detectada<R: Runtime>(app: AppHandle<R>) {
    if let Some(window) = app.get_webview_window("reuniao-detectada") {
        let _ = window.hide();
    }
}

/// Silencia a deteccao para um processo, pelo mesmo caminho do lembrete.
///
/// Silencia o AVISO, e nunca a observacao — o mesmo criterio que o `suppress` do
/// lembrete usa. Quem pediu silencio pediu para nao ser interrompido.
#[tauri::command]
pub fn silenciar_deteccao<R: Runtime>(app: AppHandle<R>, processo: String) {
    // Ate o fim do dia de quem clicou, como o "nao lembrar hoje" faz.
    let ate = time::OffsetDateTime::now_utc().unix_timestamp() + 60 * 60 * 12;
    app.state::<Monitor>().suppress(&processo, ate);
    if let Some(window) = app.get_webview_window("reuniao-detectada") {
        let _ = window.hide();
    }
}
```

`Monitor::suppress` está em `monitor.rs:97` com a assinatura `(&self, process_name: &str, until_epoch: i64)` — verificado. Ele já normaliza para minúscula por dentro, então não faça `to_lowercase()` antes de chamar.

- [ ] **Step 2: Registre no `invoke_handler`**

Em `apps/desktop/src-tauri/src/lib.rs`, junto dos outros comandos de `monitor::`:

```rust
            monitor::fechar_reuniao_detectada,
            monitor::silenciar_deteccao,
```

- [ ] **Step 3: Escreva o cliente**

Em `apps/desktop/src/api.ts`, junto dos outros de monitoramento:

```ts
  // --- A oferta de gravar (ADR-046) ---
  fecharReuniaoDetectada() {
    return invoke<void>("fechar_reuniao_detectada");
  },
  // Silencia o AVISO para aquele processo, e nunca a observação.
  silenciarDeteccao(processo: string) {
    return invoke<void>("silenciar_deteccao", { processo });
  },
```

- [ ] **Step 4: Confirme que compila dos dois lados**

```bash
cd apps/desktop && npx tsc --noEmit
export TMP="$SCRATCH/tmp"; export TEMP="$TMP"
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
```
Esperado: limpo nos dois.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src apps/desktop/src-tauri
git commit -m "feat(deteccao): a janela da oferta e os comandos dela"
```

---

### Task 5: O laço, o toggle e a ADR

**Files:**
- Modify: `apps/desktop/src-tauri/src/monitor.rs` (o `run`, ~linha 197)
- Modify: `crates/mos-core/src/monitoring.rs` (campo em `MonitoringSettings`)
- Modify: `crates/mos-storage-sqlite/migrations/` (migration `0023`)
- Modify: `apps/desktop/src/MeetingSettings.tsx` (o toggle)
- Modify: `docs/DECISIONS.md` (ADR-046)

**Interfaces:**
- Consumes: `decidir_oferta` (Task 1); `microfone::abertos_agora` (Task 2); a janela e os comandos (Tasks 3 e 4).
- Produces: `MonitoringSettings.meeting_detection_enabled: bool`, default `true`.

- [ ] **Step 1: A migration do campo**

Crie `crates/mos-storage-sqlite/migrations/0023_meeting_detection.sql`:

```sql
-- O toggle da deteccao de reuniao (ADR-046).
--
-- `DEFAULT 1`: LIGADA de fabrica, por decisao do proprietario. A ADR-046 admite
-- o custo com todas as letras — a fronteira da ADR-037 passa a ser atravessada
-- COM AVISO e nao com pedido. O toggle e a mitigacao, e ele mora em
-- Settings > REUNIOES, e nao enterrado em Avancado.

BEGIN IMMEDIATE;

ALTER TABLE tracking_settings ADD COLUMN meeting_detection_enabled INTEGER NOT NULL DEFAULT 1;

PRAGMA user_version = 23;

COMMIT;
```

A tabela e `tracking_settings`, e nao `monitoring_settings` — verificado no banco real. E la que moram `process_monitoring_enabled`, `remind_on_monitored_open` e as outras chaves de observacao, apesar do nome falar de tracking.

Registre a migration em `crates/mos-storage-sqlite/src/lib.rs` (constante `MIGRATION_023`, bloco `if current <= 22`, e `SCHEMA_VERSION = 23`), acrescente o campo à struct `MonitoringSettings` e ao `SELECT`/`UPDATE` do repositório.

- [ ] **Step 2: Ligue o laço**

Em `apps/desktop/src-tauri/src/monitor.rs`, dentro do `run`, **depois** do bloco `if settings.process_monitoring_enabled`:

```rust
        // A deteccao de reuniao anda no MESMO laco, e nao num proprio: as duas
        // perguntam ao sistema no mesmo ritmo, e dois lacos acordando a cada
        // poucos segundos custam bateria por nada.
        {
            let gravando = app
                .state::<crate::meeting::RecordingState>()
                .active
                .lock()
                .map(|guard| guard.is_some())
                .unwrap_or(false);
            let silenciados = app
                .state::<Monitor>()
                .silenced_now(time::OffsetDateTime::now_utc().unix_timestamp())
                .into_iter()
                .map(|(processo, _)| processo)
                .collect();

            let contexto = mos_core::ContextoDaOferta {
                gravando,
                silenciados,
                ligado: settings.meeting_detection_enabled,
                espera_segundos: ESPERA_DE_OFERTA,
            };
            let abertos = crate::microfone::abertos_agora();
            if let mos_core::DecisaoDeOferta::Oferecer(processo) =
                mos_core::decidir_oferta(&abertos, &contexto)
            {
                oferecer(&app, &processo);
            }
        }
```

E, no topo do arquivo:

```rust
/// Vinte segundos de microfone aberto antes de oferecer.
///
/// Nao e conservadorismo: microfone que abre por dois segundos e teste de som,
/// atalho de push-to-talk, notificacao. Reuniao mantem aberto. Sem a espera o
/// popup vira ruido, e popup ruidoso e desligado no primeiro dia — o que custa a
/// feature inteira em troca de nada.
const ESPERA_DE_OFERTA: i64 = 20;
```

- [ ] **Step 3: Mostre a janela**

Ainda em `monitor.rs`, no molde de `show_reminder` (linha ~345), que já resolve posição e "não roubar foco":

```rust
/// Mostra a oferta, sem roubar o foco.
///
/// Uma janela que captura o teclado no instante em que alguem entra numa reuniao
/// e um acidente — a pessoa esta clicando em "entrar", nao aqui.
///
/// So oferece UMA VEZ por abertura de microfone: `ja_ofereceu` guarda o processo
/// e so limpa quando ele some da lista de abertos. Sem isso a janela reapareceria
/// a cada volta do laco, que sao poucos segundos.
fn oferecer<R: Runtime>(app: &AppHandle<R>, processo: &str) {
    let monitor = app.state::<Monitor>();
    {
        let Ok(mut observed) = monitor.observed.lock() else {
            return;
        };
        if observed.ja_ofereceu.as_deref() == Some(processo) {
            return;
        }
        observed.ja_ofereceu = Some(processo.to_string());
    }

    let Some(window) = app.get_webview_window("reuniao-detectada") else {
        return;
    };
    if !window.is_visible().unwrap_or(false) {
        if let Ok(Some(monitor)) = window.current_monitor() {
            let screen = monitor.size();
            let scale = monitor.scale_factor();
            if let Ok(size) = window.outer_size() {
                let margin = (24.0 * scale) as u32;
                let x = screen.width.saturating_sub(size.width + margin);
                let y = screen.height.saturating_sub(size.height + margin * 3);
                let _ = window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
            }
        }
        let _ = window.show();
        let _ = window.set_always_on_top(true);
    }
    let _ = window.emit(
        "reuniao-detectada",
        serde_json::json!({ "processo": processo, "nome": nome_amigavel(processo) }),
    );
}

/// O nome que a pessoa le. Sem lista de reunioes conhecidas: o que se sabe e o
/// nome do executavel, e inventar "Google Meet" a partir de `chrome.exe` seria
/// afirmar o que nao se observou.
fn nome_amigavel(processo: &str) -> String {
    processo
        .strip_suffix(".exe")
        .unwrap_or(processo)
        .to_string()
}
```

Acrescente `ja_ofereceu: Option<String>` à struct que `observed` guarda (perto da linha 55), e **limpe-o** quando o processo sumir dos abertos — no mesmo bloco da Task 5 Step 2, antes de decidir:

```rust
            if let Ok(mut observed) = app.state::<Monitor>().observed.lock() {
                let ainda_aberto = observed
                    .ja_ofereceu
                    .as_ref()
                    .map(|alvo| abertos.iter().any(|e| &e.processo == alvo))
                    .unwrap_or(false);
                if !ainda_aberto {
                    // Microfone fechou: a proxima abertura oferece de novo, e
                    // isso e desejado — pode ser outra reuniao.
                    observed.ja_ofereceu = None;
                }
            }
```

- [ ] **Step 4: O toggle**

Em `apps/desktop/src/MeetingSettings.tsx`, logo depois do `setting-row` do consentimento (linha ~129), no mesmo molde:

```tsx
        <div className="setting-row">
          <div>
            <strong>Oferecer gravação quando uma reunião começa</strong>
            {/* O segundo parágrafo não é decoração: é o que a pessoa precisa
                para decidir, e ele diz o que a feature NÃO faz. Sem ele, o
                toggle pede confiança em vez de informar. */}
            <p>
              O M/OS observa qual programa abriu o microfone — nunca o título da janela,
              o conteúdo da tela ou o áudio.
            </p>
          </div>
          <label className="switch">
            <input
              aria-label="Oferecer gravação quando uma reunião começa"
              type="checkbox"
              checked={deteccao}
              onChange={(event) => {
                const ligado = event.currentTarget.checked;
                void api.setMonitoringSettings({ ...configuracoes, meetingDetectionEnabled: ligado })
                  .then(() => setDeteccao(ligado))
                  .catch((error) => setNote(String(error)));
              }}
            />
            <span />
          </label>
        </div>
```

Use o nome real do comando que grava `MonitoringSettings` neste projeto — leia `api.ts` e procure o que a tela de Tempo usa para gravar as configurações de observação. Se ele exigir o objeto inteiro, carregue-o no `useEffect` desta tela junto com o consentimento, no mesmo `Promise.all`.

- [ ] **Step 5: A ADR-046**

Em `docs/DECISIONS.md`, acrescente à tabela de índice e escreva a ADR no fim, seguindo a estrutura das vizinhas. O que precisa estar escrito:

A fronteira da ADR-037 vai de *"nomes de programa, e nada além disso"* para *"nomes de programa, e qual programa está com o microfone aberto"*.

**Por que microfone e não título de janela:** título expõe conteúdo — nome de documento, de aba, de página. Microfone expõe uma capacidade. E o microfone detecta o fato certo: uma aba do Meet aberta não é uma reunião, um microfone aberto é. É também o que o Notion faz, e a conta oficial deles é explícita: *"It doesn't read your browser content."*

**O custo, admitido:** ligada de fábrica, a fronteira é atravessada **com aviso e não com pedido**. A ADR-037 desenhou a fronteira justamente para que atravessá-la fosse difícil e visível. Foi decisão do proprietário, com o trade-off na mesa, e o argumento a favor é que uma feature que exige ser descoberta não serve a quem não a descobre. O toggle é a mitigação.

**O que esta ADR não consegue prever:** se 20 segundos é cedo, tarde ou irritante. Reveja depois de uma semana de uso; se for irritante, o caminho é subir a espera, e não desligar a feature.

- [ ] **Step 6: Rode tudo**

```bash
export TMP="$SCRATCH/tmp"; export TEMP="$TMP"
cargo test -p mos-core -p mos-storage-sqlite
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cd apps/desktop && npx tsc --noEmit && npx vitest run
```

- [ ] **Step 7: Commit**

```bash
git add crates apps/desktop docs/DECISIONS.md
git commit -m "feat(deteccao): o laco, o toggle e a ADR-046"
```

---

### Task 6: O gate na máquina

- [ ] **Step 1: Suba o app**

```bash
cd apps/desktop && npm run tauri dev
```

- [ ] **Step 2: Confirme a migration**

```bash
python -c "import sqlite3,os; db=os.path.expandvars(r'%APPDATA%\com.codedbym.mos\m-os.db'); con=sqlite3.connect(f'file:{db}?mode=ro',uri=True); print(con.execute('PRAGMA user_version').fetchone())"
```
Esperado: `(23,)`.

- [ ] **Step 3: Confirme a leitura do registro, antes de testar a UI**

Com uma chamada de voz aberta em qualquer app:

```bash
powershell.exe -NoProfile -Command "
\$b = 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone\NonPackaged'
Get-ChildItem \$b | ForEach-Object {
  \$p = Get-ItemProperty \$_.PSPath
  if (\$p.LastUsedTimeStop -eq 0) { Write-Output ('EM USO: ' + (Split-Path \$_.PSPath -Leaf)) }
}"
```
Esperado: pelo menos uma linha. Se não aparecer nada com uma chamada aberta, **pare**: o problema é a leitura, e não a interface, e seguir para a UI esconderia isso.

- [ ] **Step 4: O gate de comportamento**

1. abra um Meet no Chrome e **cronometre**: a janela não pode aparecer antes de ~20 s;
2. com a janela aberta, confirme que ela **não roubou o foco** — continue digitando no Chrome e veja se as letras chegam lá;
3. clique em `Gravar reunião` e confirme que a gravação começa e a janelinha some;
4. **pare a gravação, e comece outra pelo M/OS**: com o M/OS gravando, a oferta **não pode** aparecer, mesmo com o Chrome ainda no microfone. É a exclusão que impede o ridículo;
5. feche a chamada, abra de novo e clique em `Não neste app`; confirme que uma terceira chamada não oferece mais;
6. desligue o toggle em Settings e confirme que nada aparece.

- [ ] **Step 5: O gate visual**

Pela skill `ver-o-app`: fotografe a janelinha nos dois temas e **olhe as imagens**. Ela mede 380×132 — confirme que os três botões cabem numa linha sem estourar, e que a moldura branca do `shadow` não voltou.

- [ ] **Step 6: Relate honestamente**

Diga o que foi exercitado de verdade e o que não. Em particular: se algum dos seis passos do gate de comportamento não foi feito, diga qual e por quê.
