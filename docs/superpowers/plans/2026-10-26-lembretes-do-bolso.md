# Lembretes e avisos do bolso — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** No iPhone, o toque no aviso abre o lembrete com Concluir/Adiar à mão, criar e adiar falam a mesma língua do desktop, o bolso avisa prova/task/compromisso — e cala quando o PC está em uso — e o aviso "N itens vieram do computador" deixa de existir.

**Architecture:** Nada de regra nova no domínio: o `mos-web` passa a rodar o motor de avisos que já existe em `mos-core::piloto::avisos` (`candidatos` + `decidir`) e entrega por Web Push. "PC em uso" é um sinal operacional novo, fora do log de operações: o desktop bate em `/sync/presenca` com os segundos ociosos, o hub guarda `em_uso_em` na tabela `aparelhos`, e o `mos-web` lê pela `/sync/aparelhos` que já usa. Presets de instante (criar e adiar) saem das duas UIs para um módulo TS comum em `packages/instantes`.

**Tech Stack:** Rust (axum 0.8, rusqlite, reqwest blocking, time), React 19 + TypeScript + Vitest no PWA (`apps/mos-web/ui`) e no desktop (`apps/desktop`), Service Worker puro (`apps/mos-web/ui/public/sw.js`).

**Spec:** `docs/superpowers/specs/2026-10-26-lembretes-do-bolso-design.md` — leia antes de começar.

## Global Constraints

- Nada de plataforma em `mos-core` ou `mos-sync` (AGENTS.md, regra 1). `GetLastInputInfo` mora no desktop (`apps/desktop/src-tauri/src/monitor.rs::idle_seconds`, que já existe).
- "PC em uso" = algum aparelho com `plataforma == "windows"` cujo `em_uso_em` está a **5 minutos** ou menos do relógio do servidor.
- Com PC em uso, o bolso segura **Academic, Overdue, ForgottenTask e Upcoming** (compromisso inclusive). **Lembretes não calam** — tocam nos dois.
- Tipos que o bolso **nunca** avisa: `DayNotStarted`, `UnfinishedDay`, `Sync`, `WaitingFor`.
- `em_uso_em` é calculado **no hub** (`agora_do_servidor - ocioso_segundos`): relógio de cliente não é confiável (mesma regra do `visto_em`).
- Aviso segurado **não** é registrado em `autopilot_avisos` — é isso que faz ele sair na rodada seguinte se o PC deixar de estar em uso.
- Web Push no iOS **não tem botões de ação**. Não implemente `actions` na notificação.
- Comentários e mensagens em português, no tom do arquivo vizinho (comentário diz *por quê*). Commits direto no `master` (preferência do dono).
- Cada commit termina com `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`.

## Antes de começar (PC de casa)

- [ ] `git pull` no `master` e confira que `4dff7cc` (o spec) e `8b62417` (ADR-090) estão no histórico.
- [ ] Se `cargo` falhar com erro de diretório temporário, exporte `TMP`/`TEMP` para uma pasta sua antes (ver `docs/SETUP-MAQUINA.md`).
- [ ] `cargo test -p mos-web` só compila a porta com OpenSSL; sem ele, rode sem `--features passkey` (é o padrão). Os testes do desktop (`cargo test -p mos-desktop`) podem não executar no Windows local (morrem com `0xC0000139`) — nesse caso confie no CI e rode só `cargo check -p mos-desktop`.

## Mapa de arquivos

| arquivo | o que muda |
|---|---|
| `crates/mos-sync-server/src/hub.rs` | coluna `em_uso_em`, `marcar_em_uso`, campo em `AparelhoRegistrado` |
| `crates/mos-sync-server/src/http.rs` | rota `POST /sync/presenca`, `emUsoEm` na resposta de `/sync/aparelhos` |
| `crates/mos-sync-http/src/lib.rs` | `HttpTransport::marcar_presenca`, `AparelhoNaMalha::em_uso_em` |
| `apps/desktop/src-tauri/src/sync.rs` | laço de presença de 60 s |
| `apps/desktop/src-tauri/src/lib.rs` | inicia o laço de presença |
| `apps/mos-web/src/presenca.rs` (novo) | `pc_em_uso` — função pura |
| `apps/mos-web/src/avisos_do_dia.rs` (novo) | motor do piloto no bolso: filtro de tipos, URL do alvo, passada |
| `apps/mos-web/src/avisos.rs` | URL do lembrete, sai `chegou_do_pc` |
| `apps/mos-web/src/sync.rs` | para de disparar o aviso de sync |
| `apps/mos-web/src/api.rs` | adiar pelo `AttentionService::snooze`; docs de `transitar` |
| `apps/mos-web/src/main.rs`, `src/lib.rs` | inicia o laço novo, declara módulos |
| `packages/instantes/index.ts` (novo) | `whenChoices`, `snoozeChoices`, `SnoozeChoice` |
| `apps/desktop/src/lembretes.ts` | reexporta de `packages/instantes` |
| `apps/mos-web/ui/src/instantes.ts` | `ESCOLHAS`/`disponiveis` passam a vir do módulo comum |
| `apps/mos-web/ui/src/paginas/Lembrete.tsx` | adiar com `snoozeChoices` |
| `apps/mos-web/ui/src/Quando.tsx` | erro de horário no passado |
| `apps/mos-web/ui/src/App.tsx` | folha só fecha quando salvou; abre destino da URL e da mensagem do SW |
| `apps/mos-web/ui/src/navegacao.ts` | `destinoDaUrl` |
| `apps/mos-web/ui/public/sw.js` | clique manda o destino para a janela aberta |
| docs | README do mos-web, `PILOTO.md` §7, ADR-092 em `DECISIONS.md` |

---

### Task 1: O hub sabe quando cada aparelho está em uso

**Files:**
- Modify: `crates/mos-sync-server/src/hub.rs` (schema ~linha 95, `AparelhoRegistrado` ~195, `registrar_aparelho` ~212, `aparelhos` ~235, testes ~319)
- Modify: `crates/mos-sync-server/src/http.rs` (rotas ~61, `AparelhoJson` ~278, `registrar_aparelho` ~305, `aparelhos` ~341)
- Test: `crates/mos-sync-server/src/hub.rs` (`mod tests`)

**Interfaces:**
- Produces: `Hub::marcar_em_uso(&mut self, id: &str, em_uso_em: &str) -> Resultado<bool>` (false = aparelho desconhecido); `AparelhoRegistrado { …, em_uso_em: Option<String> }`; rota `POST /sync/presenca` com corpo `{"id": String, "ociosoSegundos": i64}` → `204`, `404` se o aparelho nunca bateu, `401` sem token; `GET /sync/aparelhos` passa a trazer `"emUsoEm": "<rfc3339>" | null`.

- [ ] **Step 1: Teste que falha**

No `mod tests` de `hub.rs`, depois de `o_hub_guarda_quem_e_cada_aparelho`:

```rust
    #[test]
    fn o_hub_guarda_quando_o_aparelho_esteve_em_uso() {
        let mut hub = Hub::em_memoria().unwrap();
        hub.registrar_aparelho(&aparelho(), "2026-10-26T12:00:00Z")
            .unwrap();
        assert_eq!(hub.aparelhos().unwrap()[0].em_uso_em, None);

        assert!(hub
            .marcar_em_uso(&aparelho().id, "2026-10-26T12:04:30Z")
            .unwrap());
        assert_eq!(
            hub.aparelhos().unwrap()[0].em_uso_em.as_deref(),
            Some("2026-10-26T12:04:30Z")
        );

        // A batida seguinte NAO apaga o uso: sao duas perguntas diferentes.
        hub.registrar_aparelho(&aparelho(), "2026-10-26T12:10:00Z")
            .unwrap();
        assert_eq!(
            hub.aparelhos().unwrap()[0].em_uso_em.as_deref(),
            Some("2026-10-26T12:04:30Z")
        );
    }

    #[test]
    fn uso_de_aparelho_desconhecido_nao_cria_linha() {
        let mut hub = Hub::em_memoria().unwrap();
        assert!(!hub.marcar_em_uso("ninguem", "2026-10-26T12:00:00Z").unwrap());
        assert!(hub.aparelhos().unwrap().is_empty());
    }
```

E acrescente `em_uso_em: None,` no `fn aparelho()` do teste.

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test -p mos-sync-server o_hub_guarda_quando`
Expected: erro de compilação — `no field em_uso_em` / `no method marcar_em_uso`.

- [ ] **Step 3: Implementar no `hub.rs`**

1. No `CREATE TABLE IF NOT EXISTS aparelhos`, acrescente a coluna (bancos novos):

```sql
                visto_em    TEXT NOT NULL,
                em_uso_em   TEXT
```

2. Logo depois do `execute_batch(...)?;` do construtor e antes do `Ok(Self { conexao })`, a migração aditiva para o banco que já existe na VPS:

```rust
        // `em_uso_em` chegou depois da tabela. O hub da VPS ja tem `aparelhos`,
        // e `CREATE TABLE IF NOT EXISTS` nao acrescenta coluna em tabela que
        // existe — entao ela entra aqui, uma vez, e nunca mais.
        let tem_coluna: bool = conexao
            .prepare("SELECT 1 FROM pragma_table_info('aparelhos') WHERE name = 'em_uso_em'")?
            .exists([])?;
        if !tem_coluna {
            conexao.execute("ALTER TABLE aparelhos ADD COLUMN em_uso_em TEXT", [])?;
        }
```

3. Em `AparelhoRegistrado`, depois de `visto_em`:

```rust
    /// A ultima vez que alguem MEXEU neste aparelho, na hora do servidor.
    ///
    /// Nao e o mesmo que `visto_em`: um PC esquecido ligado bate no hub o dia
    /// inteiro sem ninguem na frente dele. E esta a pergunta que o bolso faz
    /// antes de avisar — ver `apps/mos-web/src/presenca.rs`.
    pub em_uso_em: Option<String>,
```

4. `registrar_aparelho` não muda o SQL (o `ON CONFLICT ... DO UPDATE` não toca em `em_uso_em`).

5. Novo método, depois de `registrar_aparelho`:

```rust
    /// Alguem mexeu neste aparelho em `em_uso_em`.
    ///
    /// So atualiza quem ja bateu: presenca de um aparelho que o hub nao conhece
    /// nao tem nome nem plataforma, e uma linha sem eles aparece na malha como
    /// um fantasma. Devolve se achou.
    pub fn marcar_em_uso(&mut self, id: &str, em_uso_em: &str) -> Resultado<bool> {
        let mudou = self.conexao.execute(
            "UPDATE aparelhos SET em_uso_em = ?2 WHERE id = ?1",
            rusqlite::params![id, em_uso_em],
        )?;
        Ok(mudou > 0)
    }
```

6. Em `aparelhos()`, selecione e leia a coluna nova:

```rust
        let mut consulta = self.conexao.prepare(
            "SELECT id, nome, plataforma, versao, contrato, visto_em, em_uso_em \
             FROM aparelhos ORDER BY visto_em DESC",
        )?;
        let linhas = consulta.query_map([], |linha| {
            Ok(AparelhoRegistrado {
                id: linha.get(0)?,
                nome: linha.get(1)?,
                plataforma: linha.get(2)?,
                versao: linha.get(3)?,
                contrato: linha.get::<_, i64>(4)? as u32,
                visto_em: linha.get(5)?,
                em_uso_em: linha.get(6)?,
            })
        })?;
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cargo test -p mos-sync-server`
Expected: falha de compilação em `http.rs` (struct literal sem `em_uso_em`). Siga para o Step 5 antes de esperar verde.

- [ ] **Step 5: A rota no `http.rs`**

1. No literal de `registrar_aparelho`, acrescente `em_uso_em: None,`.
2. Em `AparelhoJson`, depois de `visto_em: String,`: `em_uso_em: Option<String>,` — e no `.map(...)` de `aparelhos`, `em_uso_em: aparelho.em_uso_em,`.
3. A rota, junto das outras (`~linha 64`): `.route("/sync/presenca", post(marcar_presenca))`.
4. O handler, depois de `registrar_aparelho`:

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresencaPedido {
    id: String,
    /// Ha quantos segundos ninguem toca no teclado ou no mouse.
    ///
    /// Segundos e nao um instante: o relogio do cliente nao entra na conta. O
    /// hub subtrai do PROPRIO relogio, pela mesma razao do `visto_em`.
    ociosoSegundos: i64,
}

/// "Alguem esta mexendo neste aparelho."
///
/// Fora do log de operacoes, como a batida: e metadado operacional, e nao
/// entidade. Sem contrato conferido, pelo mesmo motivo da batida.
async fn marcar_presenca(
    State(estado): State<Estado>,
    cabecalhos: HeaderMap,
    Json(pedido): Json<PresencaPedido>,
) -> Result<StatusCode, Erro> {
    if !autorizado(&cabecalhos, &estado.token) {
        return Err(negado());
    }
    let ocioso = time::Duration::seconds(pedido.ociosoSegundos.max(0));
    let em_uso_em = (time::OffsetDateTime::now_utc() - ocioso)
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    let mut hub = estado.hub.lock().expect("hub envenenado");
    let achou = hub
        .marcar_em_uso(&pedido.id, &em_uso_em)
        .map_err(falha_no_banco)?;
    Ok(if achou {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    })
}
```

Se o `#[serde(rename_all = "camelCase")]` reclamar do campo já em camelCase, renomeie o campo para `ocioso_segundos` (o `rename_all` produz `ociosoSegundos`). Confira se `time` já é dependência do crate (`grep time crates/mos-sync-server/Cargo.toml`); se não for, use o mesmo helper que gera `agora_iso()` e subtraia com `time` adicionando `time = { workspace = true }` ao `Cargo.toml`.

- [ ] **Step 6: Rodar e ver passar**

Run: `cargo test -p mos-sync-server`
Expected: PASS, incluindo os dois testes novos e `tests/contrato.rs`.

- [ ] **Step 7: Commit**

```bash
git add crates/mos-sync-server
git commit -m "feat(hub): o hub sabe quando cada aparelho esteve em uso — /sync/presenca e em_uso_em"
```

---

### Task 2: O cliente HTTP manda e lê a presença

**Files:**
- Modify: `crates/mos-sync-http/src/lib.rs` (`AparelhoNaMalha` ~182, `impl HttpTransport` ~213)

**Interfaces:**
- Consumes: rota da Task 1.
- Produces: `HttpTransport::marcar_presenca(&self, id: &str, ocioso_segundos: i64) -> Resultado<()>`; `AparelhoNaMalha::em_uso_em: Option<String>` (`#[serde(default)]`).

- [ ] **Step 1: Campo novo em `AparelhoNaMalha`**, depois de `visto_em`:

```rust
    /// A ultima vez que alguem mexeu neste aparelho, na hora do servidor.
    /// `None` para aparelho que nunca mandou presenca — o bolso, um PC antigo.
    #[serde(default)]
    pub em_uso_em: Option<String>,
```

- [ ] **Step 2: Método novo**, depois de `malha()`:

```rust
    /// Diz ao hub ha quanto tempo ninguem mexe neste aparelho.
    pub fn marcar_presenca(&self, id: &str, ocioso_segundos: i64) -> Resultado<()> {
        let resposta = self
            .cliente
            .post(format!("{}/sync/presenca", self.base))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "id": id, "ociosoSegundos": ocioso_segundos }))
            .send()
            .map_err(erro_de_rede)?;
        if !resposta.status().is_success() {
            return Err(erro_da_resposta(resposta));
        }
        Ok(())
    }
```

- [ ] **Step 3: Compilar e rodar os testes do crate**

Run: `cargo test -p mos-sync-http && cargo check -p mos-web`
Expected: PASS / sem erros.

- [ ] **Step 4: Commit**

```bash
git add crates/mos-sync-http
git commit -m "feat(sync-http): marcar_presenca e em_uso_em na malha"
```

---

### Task 3: O desktop diz ao hub quando está em uso

**Files:**
- Modify: `apps/desktop/src-tauri/src/sync.rs` (perto de `anunciar`, ~533)
- Modify: `apps/desktop/src-tauri/src/lib.rs` (onde o daemon de sync é iniciado — procure `sync::` no `setup`)

**Interfaces:**
- Consumes: `HttpTransport::marcar_presenca` (Task 2); `crate::monitor::idle_seconds() -> Option<i64>` (já existe, `monitor.rs:171`).
- Produces: `pub async fn presenca(app: tauri::AppHandle)` em `sync.rs`.

- [ ] **Step 1: O laço em `sync.rs`**

```rust
/// De quanto em quanto tempo o PC diz ao hub que esta em uso.
///
/// Um minuto, porque o bolso considera "em uso" ate cinco minutos depois da
/// ultima tecla — com folga para uma batida perdida.
const PULSO_DA_PRESENCA: Duration = Duration::from_secs(60);

/// Diz ao hub, a cada minuto, ha quanto tempo ninguem mexe neste PC.
///
/// E o que permite ao iPhone calar o aviso de prova enquanto a pessoa esta na
/// mesa — e voltar a avisar quando ela levanta (spec 2026-10-26, §4). Ocioso
/// ha mais de cinco minutos nao manda nada: a ausencia ja e a resposta, e
/// poupa uma ida a rede por minuto com o PC parado.
pub async fn presenca(app: tauri::AppHandle) {
    loop {
        tokio::time::sleep(PULSO_DA_PRESENCA).await;
        let Some(ocioso) = crate::monitor::idle_seconds() else {
            continue;
        };
        if ocioso > 5 * 60 {
            continue;
        }
        let (storage, settings_path) = {
            let state = app.state::<crate::AppState>();
            (Arc::clone(&state.storage), state.settings_path.clone())
        };
        let endpoint = crate::load_settings(&settings_path).sync_endpoint;
        if endpoint.is_empty() {
            continue;
        }
        let Some(token) = token_guardado() else {
            continue;
        };
        let _ = tauri::async_runtime::spawn_blocking(move || {
            use mos_sync::DeviceRepository;
            let nome = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Este PC".to_owned());
            let eu = storage
                .este_dispositivo(&nome, "windows", env!("CARGO_PKG_VERSION"))
                .map_err(|c| c.mensagem)?;
            let transporte = mos_sync_http::HttpTransport::novo(&endpoint, &token)
                .map_err(|c| c.mensagem)?;
            transporte
                .marcar_presenca(&eu.id.to_string(), ocioso)
                .map_err(|c| c.mensagem)
        })
        .await;
    }
}
```

Ajuste os nomes se `rodar` usar outros (`endpoint`/`token` vêm de `load_settings` e `token_guardado`, exatamente como em `rodar`, `sync.rs:239-249`). Um 404 (o hub ainda não recebeu a batida deste PC) é esperado no primeiro minuto e não precisa de log.

- [ ] **Step 2: Iniciar no `lib.rs`**

Ao lado de onde o daemon de sync é iniciado (procure `tauri::async_runtime::spawn(sync::` ou `sync::daemon`), acrescente:

```rust
            tauri::async_runtime::spawn(sync::presenca(app.handle().clone()));
```

- [ ] **Step 3: Compilar**

Run: `cargo check -p mos-desktop` (o nome do pacote está em `apps/desktop/src-tauri/Cargo.toml`)
Expected: sem erros. `monitor::idle_seconds` é `pub(crate)` — já serve.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/src/sync.rs apps/desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): o PC diz ao hub, a cada minuto, que esta em uso"
```

---

### Task 4: O bolso sabe se o PC está em uso

**Files:**
- Create: `apps/mos-web/src/presenca.rs`
- Modify: `apps/mos-web/src/lib.rs` (declarar `pub mod presenca;`)

**Interfaces:**
- Consumes: `mos_sync_http::AparelhoNaMalha` (Task 2).
- Produces: `pub const JANELA_DE_USO: time::Duration` (5 min); `pub fn pc_em_uso(malha: &[mos_sync_http::AparelhoNaMalha], agora: time::OffsetDateTime) -> bool`.

- [ ] **Step 1: Teste que falha** — crie `apps/mos-web/src/presenca.rs` só com os testes:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mos_sync_http::AparelhoNaMalha;
    use time::macros::datetime;

    fn aparelho(plataforma: &str, em_uso_em: Option<&str>) -> AparelhoNaMalha {
        AparelhoNaMalha {
            id: "x".into(),
            nome: "PC".into(),
            plataforma: plataforma.into(),
            versao: "0.7.3".into(),
            contrato: 1,
            manifesto: vec![],
            visto_em: "2026-10-26T12:00:00Z".into(),
            em_uso_em: em_uso_em.map(String::from),
        }
    }

    const AGORA: time::OffsetDateTime = datetime!(2026-10-26 12:10:00 UTC);

    #[test]
    fn pc_mexido_ha_quatro_minutos_esta_em_uso() {
        let malha = [aparelho("windows", Some("2026-10-26T12:06:00Z"))];
        assert!(pc_em_uso(&malha, AGORA));
    }

    #[test]
    fn pc_parado_ha_seis_minutos_nao_esta() {
        let malha = [aparelho("windows", Some("2026-10-26T12:04:00Z"))];
        assert!(!pc_em_uso(&malha, AGORA));
    }

    /// PC antigo, que nao manda presenca, nunca cala o bolso: na duvida, avisa.
    #[test]
    fn pc_sem_presenca_nao_cala() {
        assert!(!pc_em_uso(&[aparelho("windows", None)], AGORA));
    }

    /// O proprio bolso aparece na malha como `web`; ele nao conta.
    #[test]
    fn o_bolso_nao_conta_como_pc() {
        let malha = [aparelho("web", Some("2026-10-26T12:09:00Z"))];
        assert!(!pc_em_uso(&malha, AGORA));
    }

    #[test]
    fn instante_ilegivel_nao_cala() {
        assert!(!pc_em_uso(&[aparelho("windows", Some("ontem"))], AGORA));
    }
}
```

Adicione `pub mod presenca;` em `apps/mos-web/src/lib.rs`. Se `time::macros` não estiver habilitado, acrescente a feature `macros` ao `time` em `[dev-dependencies]` do `apps/mos-web/Cargo.toml` (ou construa os instantes com `OffsetDateTime::parse`).

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test -p mos-web --lib presenca`
Expected: FAIL — `cannot find function pc_em_uso`.

- [ ] **Step 3: Implementar, acima dos testes**

```rust
//! O PC esta em uso agora?
//!
//! E a pergunta que o bolso faz antes de avisar prova, task e compromisso. Com
//! a pessoa na mesa, o PC ja mostra e ja avisa — o iPhone tocando junto e o
//! mesmo aviso duas vezes (spec 2026-10-26, §4).
//!
//! "Em uso" e nao "ligado": um PC esquecido ligado em casa bate no hub o dia
//! inteiro, e ele nao pode calar o aviso da prova na rua. Por isso a pergunta
//! e sobre `em_uso_em` — a ultima tecla —, e nao sobre `visto_em`.
//!
//! Na duvida, avisa: instante ilegivel, PC antigo sem o campo, hub fora — tudo
//! isso responde "nao esta em uso". Um aviso a mais custa menos que uma prova
//! perdida.

use mos_sync_http::AparelhoNaMalha;
use time::OffsetDateTime;

/// Quanto tempo depois da ultima tecla o PC ainda conta como em uso.
pub const JANELA_DE_USO: time::Duration = time::Duration::minutes(5);

pub fn pc_em_uso(malha: &[AparelhoNaMalha], agora: OffsetDateTime) -> bool {
    malha
        .iter()
        .filter(|aparelho| aparelho.plataforma == "windows")
        .filter_map(|aparelho| aparelho.em_uso_em.as_deref())
        .filter_map(|texto| {
            OffsetDateTime::parse(texto, &time::format_description::well_known::Rfc3339).ok()
        })
        .any(|quando| agora - quando <= JANELA_DE_USO)
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cargo test -p mos-web --lib presenca`
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/mos-web/src/presenca.rs apps/mos-web/src/lib.rs apps/mos-web/Cargo.toml
git commit -m "feat(mos-web): o bolso sabe se o PC esta em uso — cinco minutos desde a ultima tecla"
```

---

### Task 5: O bolso roda o motor de avisos

**Files:**
- Create: `apps/mos-web/src/avisos_do_dia.rs`
- Modify: `apps/mos-web/src/piloto.rs` (tornar `ler` e `Leitura` visíveis ao crate: `pub(crate) fn ler`)
- Modify: `apps/mos-web/src/lib.rs` (`pub mod avisos_do_dia;`)
- Modify: `apps/mos-web/src/main.rs` (iniciar o laço, ao lado de `avisos::iniciar`, ~linha 121)

**Interfaces:**
- Consumes: `presenca::pc_em_uso` (Task 4); `mos_core::{candidatos_a_aviso, decidir_avisos, PoliticaDeAvisos, VereditoDeAviso, TipoDeAviso, Alvo}`; `piloto::ler(estado: &Estado, now_local: OffsetDateTime, ultima_presenca: Option<OffsetDateTime>) -> Result<Leitura, CoreError>` (já existe, `piloto.rs:157`, hoje privada); `SqliteStorage::{autopilot_ligado, avisos_desde, registrar_aviso, podar_avisos}` (`crates/mos-storage-sqlite/src/piloto_repository.rs`); `avisos::{Avisador, Aviso}`.
- Produces: `pub fn do_bolso(tipo: TipoDeAviso) -> bool`; `pub fn url_do_alvo(alvo: &mos_core::Alvo) -> String`; `pub fn iniciar(estado: Estado, avisador: Arc<Avisador>)`.

- [ ] **Step 1: Testes que falham** — crie `apps/mos-web/src/avisos_do_dia.rs` com:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mos_core::TipoDeAviso;

    #[test]
    fn o_bolso_avisa_prova_task_e_compromisso() {
        for tipo in [
            TipoDeAviso::Academic,
            TipoDeAviso::Overdue,
            TipoDeAviso::ForgottenTask,
            TipoDeAviso::Upcoming,
        ] {
            assert!(do_bolso(tipo), "{tipo:?} devia avisar no bolso");
        }
    }

    #[test]
    fn coisas_de_mesa_ficam_no_pc() {
        for tipo in [
            TipoDeAviso::DayNotStarted,
            TipoDeAviso::UnfinishedDay,
            TipoDeAviso::Sync,
            TipoDeAviso::WaitingFor,
        ] {
            assert!(!do_bolso(tipo), "{tipo:?} nao devia avisar no bolso");
        }
    }

    #[test]
    fn o_toque_leva_ao_alvo() {
        let alvo = |kind: &str| mos_core::Alvo { kind: kind.into(), id: "abc".into() };
        assert_eq!(url_do_alvo(&alvo("task")), "/?task=abc");
        assert_eq!(url_do_alvo(&alvo("reminder")), "/?lembrete=abc");
        assert_eq!(url_do_alvo(&alvo("academic")), "/?pagina=academico");
        assert_eq!(url_do_alvo(&alvo("meeting")), "/");
        assert_eq!(url_do_alvo(&alvo("")), "/");
    }
}
```

Antes de escrever o `url_do_alvo`, confirme o `kind` que os candidatos acadêmicos usam: `grep -n "Alvo {" -A3 crates/mos-core/src/piloto/avisos.rs`. Se for outro texto (ex.: `"academic_exam"`), use esse no `match` e no teste.

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test -p mos-web --lib avisos_do_dia`
Expected: FAIL — funções inexistentes.

- [ ] **Step 3: Implementar, acima dos testes**

```rust
//! Os avisos do dia, no bolso: prova, task vencida ou esquecida, compromisso.
//!
//! A regra NAO mora aqui. `mos-core::piloto::avisos` decide o que merece
//! interromper — o mesmo `candidatos` + `decidir` que o desktop roda
//! (`apps/desktop/src-tauri/src/piloto.rs::tick`). Este arquivo so faz o que e
//! do bolso: escolher quais tipos fazem sentido longe da mesa, calar quando o
//! PC esta em uso, e entregar por Web Push.
//!
//! A memoria do que ja foi avisado e a `autopilot_avisos` do banco do
//! `mos-web` — local, nao sincroniza. Cada aparelho lembra do que ELE avisou.

use std::sync::Arc;
use std::time::Duration;

use mos_core::{TipoDeAviso, VereditoDeAviso};
use time::OffsetDateTime;

use crate::avisos::{Avisador, Aviso};
use crate::estado::Estado;

/// O mesmo passo dos lembretes.
const INTERVALO: Duration = Duration::from_secs(60);

/// Quanto historico o `decidir` olha. O mesmo do desktop.
const JANELA_DO_HISTORICO: time::Duration = time::Duration::days(2);

/// O que faz sentido avisar longe da mesa.
///
/// "Dia nao comecou" e "dia nao fechado" sao convites para sentar no PC; no
/// iPhone, na rua, sao so culpa. Sync e WaitingFor sao estado da maquina e de
/// terceiros, nao algo que se resolve com o celular na mao.
pub fn do_bolso(tipo: TipoDeAviso) -> bool {
    matches!(
        tipo,
        TipoDeAviso::Academic
            | TipoDeAviso::Overdue
            | TipoDeAviso::ForgottenTask
            | TipoDeAviso::Upcoming
    )
}

/// Para onde o toque leva. Ver `ui/src/navegacao.ts::destinoDaUrl`.
pub fn url_do_alvo(alvo: &mos_core::Alvo) -> String {
    match alvo.kind.as_str() {
        "task" if !alvo.id.is_empty() => format!("/?task={}", alvo.id),
        "reminder" if !alvo.id.is_empty() => format!("/?lembrete={}", alvo.id),
        "academic" => String::from("/?pagina=academico"),
        _ => String::from("/"),
    }
}

/// O PC esta em uso? Pergunta ao hub. Sem hub, ou com o hub fora, "nao".
fn pc_em_uso_agora(estado: &Estado) -> bool {
    let Some(hub) = &estado.hub else {
        return false;
    };
    let Ok(transporte) = mos_sync_http::HttpTransport::novo(&hub.url, &hub.token) else {
        return false;
    };
    transporte
        .malha()
        .map(|malha| crate::presenca::pc_em_uso(&malha, OffsetDateTime::now_utc()))
        .unwrap_or(false)
}

/// Uma passada. Bloqueante — chame de dentro de `spawn_blocking`.
pub fn passada(estado: &Estado, avisador: &Avisador) {
    let ajustes = estado.attention.settings().ok();
    let offset = ajustes
        .as_ref()
        .map(|a| a.offset())
        .unwrap_or(time::UtcOffset::UTC);
    let agora = OffsetDateTime::now_utc().to_offset(offset);

    let Ok(leitura) = crate::piloto::ler(estado, agora, None) else {
        return;
    };
    let retrato = leitura.retrato();
    let candidatos: Vec<_> = mos_core::candidatos_a_aviso(&retrato)
        .into_iter()
        .filter(|c| do_bolso(c.tipo))
        .collect();
    if candidatos.is_empty() {
        return;
    }

    // Segurar, e nao descartar: nada e registrado enquanto o PC esta em uso,
    // entao o mesmo candidato volta na proxima passada e sai se a pessoa tiver
    // levantado e ele ainda valer.
    if pc_em_uso_agora(estado) {
        return;
    }

    let storage = &estado.storage;
    let silencio = ajustes.map(|a| a.quiet).unwrap_or_default();
    let historico = storage
        .avisos_desde(agora - JANELA_DO_HISTORICO)
        .unwrap_or_default();
    let politica = mos_core::PoliticaDeAvisos {
        silencio: &silencio,
        ligado: storage.autopilot_ligado().unwrap_or(true),
    };
    for decisao in mos_core::decidir_avisos(&retrato, candidatos, &historico, &politica) {
        if decisao.veredito != VereditoDeAviso::Entregar {
            continue;
        }
        let c = decisao.candidato;
        // Registra ANTES de mandar, como o laco dos lembretes: notificacao
        // repetida custa mais caro que notificacao perdida.
        if storage
            .registrar_aviso(
                &c.chave,
                c.tipo,
                (&c.alvo.kind, &c.alvo.id),
                &c.titulo,
                &c.corpo,
                OffsetDateTime::now_utc(),
            )
            .is_err()
        {
            continue;
        }
        avisador.disparar(&Aviso {
            titulo: c.titulo.clone(),
            corpo: c.corpo.clone(),
            tag: format!("aviso-{}", c.chave),
            url: url_do_alvo(&c.alvo),
            badge: None,
        });
    }
    let _ = storage.podar_avisos(OffsetDateTime::now_utc() - time::Duration::days(7));
}

/// O laco. Separado do dos lembretes: um erro lendo a agenda nao pode calar um
/// lembrete vencido.
pub fn iniciar(estado: Estado, avisador: Arc<Avisador>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(INTERVALO).await;
            let estado = estado.clone();
            let avisador = Arc::clone(&avisador);
            let _ = tokio::task::spawn_blocking(move || passada(&estado, &avisador)).await;
        }
    });
}
```

Ajustes que o compilador vai pedir e são esperados:
- `crate::piloto::ler` e o tipo `Leitura` precisam ser `pub(crate)` em `piloto.rs`.
- Confira em `piloto_repository.rs:261` a assinatura exata de `registrar_aviso` (a ordem acima é a que o desktop usa em `apps/desktop/src-tauri/src/piloto.rs:563-570`).
- `ajustes.offset()` existe em `AttentionSettings` (usado em `avisos.rs`). Se o tipo de `quiet` não for `Default`, use `mos_core::QuietHours::default()`.
- Se `VereditoDeAviso` não implementar `PartialEq` para a comparação, use `matches!(decisao.veredito, VereditoDeAviso::Entregar)`.

- [ ] **Step 4: Iniciar no `main.rs`**, logo depois do bloco `avisos::iniciar(...)`:

```rust
    // Os avisos do dia: prova, task, compromisso. Laco proprio, pelo mesmo
    // motivo do laco dos lembretes ser proprio.
    if let Some(push) = &estado.push {
        mos_web::avisos_do_dia::iniciar(estado.clone(), std::sync::Arc::clone(&push.avisador));
    }
```

(Use o caminho de módulo que o `main.rs` já usa para `avisos` — se ele faz `use mos_web::avisos;`, faça o mesmo para `avisos_do_dia`.)

- [ ] **Step 5: Rodar**

Run: `cargo test -p mos-web`
Expected: PASS, com os 3 testes novos.

- [ ] **Step 6: Commit**

```bash
git add apps/mos-web/src
git commit -m "feat(mos-web): o bolso avisa prova, task e compromisso — e cala com o PC em uso"
```

---

### Task 6: O aviso do lembrete leva ao lembrete; sai o aviso de sync; adiar deixa história

**Files:**
- Modify: `apps/mos-web/src/avisos.rs` (`o_que_avisar` ~linha 139, `chegou_do_pc` ~198)
- Modify: `apps/mos-web/src/sync.rs` (~linhas 186-195, onde `chegou_do_pc` é chamado)
- Modify: `apps/mos-web/src/api.rs` (`transitar` doc ~1069, `adiar_lembrete` ~1186)
- Test: `apps/mos-web/src/avisos.rs` (`mod tests`), `apps/mos-web/tests/de_bolso.rs`

**Interfaces:**
- Produces: `Aviso.url == format!("/?lembrete={}", lembrete.id)` para lembretes; `chegou_do_pc` removida.

- [ ] **Step 1: Teste que falha (URL)** — no `mod testes` de `avisos.rs` (~linha 351, que já tem os helpers `lembrete(titulo, corpo, vence)` e `instante(segundos)`), acrescente depois de `o_que_venceu_avisa`:

```rust
    /// O iOS nao poe botao na notificacao; o toque e o atalho que sobra, e
    /// ele tem que cair no lembrete, e nao na Home.
    #[test]
    fn o_toque_no_aviso_leva_ao_lembrete() {
        let agora = instante(1_000_000);
        let vencido = lembrete("Ligar pro dentista", "", instante(999_940));
        let avisos = o_que_avisar(&[vencido.clone()], agora);
        assert_eq!(avisos[0].1.url, format!("/?lembrete={}", vencido.id));
    }
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test -p mos-web --lib o_toque_no_aviso`
Expected: FAIL — `left: "/"`.

- [ ] **Step 3: Implementar** — em `o_que_avisar`, troque `url: String::from("/"),` por:

```rust
                    // O toque abre ESTE lembrete, com Concluir e Adiar em cima.
                    // O iOS nao poe botao dentro da notificacao; este e o atalho
                    // que sobra, e ele custa um toque so.
                    url: format!("/?lembrete={}", lembrete.id),
```

- [ ] **Step 4: Tirar o aviso de sync**

Apague `pub fn chegou_do_pc(...)` de `avisos.rs` (e o teste dele, se houver). Em `sync.rs`, apague o bloco que chama `avisos::chegou_do_pc` e dispara. Mantenha o resto da rodada. Se o parâmetro `avisador` de `rodar`/`iniciar` ficar sem uso, mantenha-o só se outro código usar; senão remova-o da assinatura e da chamada em `main.rs`. Comentário no lugar onde o aviso era disparado:

```rust
    // Nao avisa mais "N itens vieram do computador" (spec 2026-10-26, §5).
    // Ninguem agia sobre ele: o badge e a inbox ja mostram o que chegou.
```

- [ ] **Step 5: Adiar pelo serviço** — em `api.rs`, troque o corpo de `adiar_lembrete`:

```rust
    let id = mos_core::ReminderId::parse(&id).map_err(de_core)?;
    let ate = instante(&pedido.ate)?;
    // Pelo METODO do servico, como concluir: e ele que escreve o `Snoozed` no
    // historico. A transicao crua adiava sem deixar rastro.
    let lembrete = escrever(&estado, move |estado| estado.attention.snooze(id, ate)).await?;
    Ok(Json(serde_json::to_value(lembrete).unwrap_or_default()))
```

E reescreva o comentário de `transitar` (hoje "Concluir e cancelar, e mais nada... Adiar ... NAO esta aqui de proposito") para:

```rust
/// Uma transicao crua de estado.
///
/// Concluir, cancelar e adiar NAO passam mais por aqui: os tres usam o metodo do
/// servico, que registra historico. Ver `adiar_lembrete` para por que o bolso
/// pode adiar.
```

Se `transitar` ficar sem chamadores, apague-a.

- [ ] **Step 6: Teste de integração do histórico** — em `tests/de_bolso.rs`, no fim de `adiar_no_bolso_empurra_a_hora`:

```rust
    let historico: serde_json::Value = cliente
        .get(format!("http://{web}/api/lembretes/{id}/historico"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let tipos: Vec<&str> = historico
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|evento| evento["kind"].as_str())
        .collect();
    assert!(
        tipos.iter().any(|t| t.eq_ignore_ascii_case("snoozed")),
        "adiar no bolso tem que deixar historico, veio {tipos:?}"
    );
```

(Confira o nome do campo do tipo no JSON do `ReminderEvent` — `kind` ou `type` — com `grep -n "pub struct ReminderEvent" -A10 crates/mos-core/src/*.rs`.)

- [ ] **Step 7: Rodar**

Run: `cargo test -p mos-web`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add apps/mos-web/src apps/mos-web/tests
git commit -m "feat(mos-web): o aviso do lembrete abre o lembrete, adiar deixa historico, e sai o aviso de sync"
```

---

### Task 7: Criar e adiar falam a mesma língua nos dois aparelhos

**Files:**
- Create: `packages/instantes/index.ts`, `packages/instantes/package.json`
- Modify: `apps/desktop/src/lembretes.ts` (~linhas 94-165: `SnoozeChoice`, `snoozeChoices`, `whenChoices` e helpers)
- Modify: `apps/mos-web/ui/src/instantes.ts` (`Escolha`, `ESCOLHAS`, `disponiveis`)
- Modify: `apps/mos-web/ui/src/paginas/Lembrete.tsx` (`ADIAMENTOS`, ~linhas 19-24 e o bloco "ADIAR")
- Test: `apps/mos-web/ui/src/instantes.test.ts`, `apps/desktop/src/lembretes.test.ts` (já existem e devem continuar verdes)

**Interfaces:**
- Produces: `export type SnoozeChoice = { label: string; resolve: () => Date }`; `export function snoozeChoices(now: Date): SnoozeChoice[]`; `export function whenChoices(now: Date): SnoozeChoice[]` em `packages/instantes/index.ts`.

- [ ] **Step 1: O módulo comum** — `packages/instantes/package.json`:

```json
{
  "name": "@m-os/instantes",
  "version": "0.1.0",
  "private": true,
  "description": "Os presets de quando e de adiar, iguais no desktop e no bolso.",
  "type": "module",
  "exports": { ".": "./index.ts" }
}
```

`packages/instantes/index.ts`: **mova** de `apps/desktop/src/lembretes.ts` o tipo `SnoozeChoice`, `snoozeChoices`, `somar`, `emHoraLocal`, `whenChoices` e `proximaSegunda` — com os comentários — para este arquivo, exportando os três públicos. Cabeçalho do arquivo:

```ts
/**
 * Os presets de *quando* e de *adiar*, num lugar só.
 *
 * O desktop e o bolso tinham cada um a sua lista, e elas divergiram: no bolso
 * "amanhã" era +24h, no desktop era amanhã às 9h. Um lembrete adiado no
 * celular às 23h tocava às 23h do dia seguinte. Instante é regra de domínio, e
 * regra de domínio não se duplica (AGENTS.md, regra 3).
 *
 * Tudo aqui resolve no fuso de QUEM TOCA — ver `CORE-FOUNDATION.md` §5.
 */
```

- [ ] **Step 2: O desktop importa** — em `apps/desktop/src/lembretes.ts`, no lugar do que foi movido:

```ts
// Os presets moram em `packages/instantes`, compartilhados com o bolso.
export { snoozeChoices, whenChoices, type SnoozeChoice } from "../../../packages/instantes";
```

(É o mesmo estilo de caminho relativo que `main.tsx` já usa para `packages/design-system`.)

Run: `cd apps/desktop && npx tsc --noEmit && npx vitest run src/lembretes.test.ts`
Expected: PASS sem mudar nenhum teste.

- [ ] **Step 3: Teste que falha no bolso** — em `apps/mos-web/ui/src/instantes.test.ts`, acrescente:

```ts
import { snoozeChoices } from "../../../../packages/instantes";

describe("adiar no bolso", () => {
  it("'Amanhã 9h' é amanhã às nove, e não daqui a 24 horas", () => {
    const agora = new Date(2026, 9, 26, 23, 0);
    const amanha = snoozeChoices(agora).find((c) => c.label === "Amanhã 9h");
    expect(amanha?.resolve()).toEqual(new Date(2026, 9, 27, 9, 0, 0, 0));
  });

  it("às 23h não oferece 'Mais tarde' nem 'À noite'", () => {
    const rotulos = snoozeChoices(new Date(2026, 9, 26, 23, 0)).map((c) => c.label);
    expect(rotulos).not.toContain("Mais tarde");
    expect(rotulos).not.toContain("À noite");
  });
});
```

Run: `cd apps/mos-web/ui && npx vitest run src/instantes.test.ts`
Expected: PASS já (o módulo existe). Este teste guarda o contrato; a mudança de comportamento do bolso vem no Step 5.

- [ ] **Step 4: `instantes.ts` do bolso usa o módulo comum**

Troque `ESCOLHAS` e `disponiveis` por um adaptador sobre `whenChoices`, mantendo o tipo `Escolha` (o `Quando.tsx` e os testes usam `rotulo`/`resolver(base)`):

```ts
import { whenChoices } from "../../../../packages/instantes";

/**
 * As opções que ainda fazem sentido, no formato da folha.
 *
 * A lista em si mora em `packages/instantes`, compartilhada com o desktop.
 */
export function disponiveis(base: Date = new Date()): Escolha[] {
  return whenChoices(base).map((escolha) => ({
    rotulo: escolha.label,
    resolver: () => escolha.resolve(),
  }));
}
```

Apague `ESCOLHAS`. No `instantes.test.ts`, o helper da linha ~15 que fazia `ESCOLHAS.find(...)` passa a usar `disponiveis(base).find((c) => c.rotulo === rotulo)`; ajuste os testes que dependiam de `ESCOLHAS` para resolver contra a `base` do teste. `padrao` não muda.

Run: `npx vitest run src/instantes.test.ts`
Expected: PASS.

- [ ] **Step 5: `Lembrete.tsx` adia com as opções do desktop**

Apague `ADIAMENTOS`. Importe `snoozeChoices` do módulo comum e troque o `.map` do bloco ADIAR:

```tsx
import { snoozeChoices } from "../../../../../packages/instantes";
// ...
            <div className="detalhe-adiar">
              {snoozeChoices(new Date()).map((opcao) => (
                <button
                  key={opcao.label}
                  type="button"
                  disabled={ocupado}
                  onClick={() => aoAdiar(opcao.resolve())}
                >
                  {opcao.label}
                </button>
              ))}
            </div>
```

- [ ] **Step 6: Rodar tudo do PWA**

Run: `cd apps/mos-web/ui && npx tsc --noEmit && npx vitest run && npm run build`
Expected: PASS e build em `../static`. Se o Vite recusar importar fora da raiz em `npm run dev`, acrescente em `vite.config.ts`: `server: { fs: { allow: ["..", "../../../packages"] }, ... }` (o build não precisa).

- [ ] **Step 7: Commit**

```bash
git add packages/instantes apps/desktop/src/lembretes.ts apps/mos-web/ui/src
git commit -m "refactor(instantes): criar e adiar com a mesma lista no desktop e no bolso — amanha e amanha as 9h"
```

---

### Task 8: O toque chega no lugar; criar recusa o passado e não perde a folha

**Files:**
- Modify: `apps/mos-web/ui/src/navegacao.ts` (nova `destinoDaUrl`)
- Modify: `apps/mos-web/ui/src/navegacao.test.ts`
- Modify: `apps/mos-web/ui/public/sw.js` (`notificationclick`, ~linha 83)
- Modify: `apps/mos-web/ui/src/App.tsx` (efeito de abertura; `criarLembrete` ~327)
- Modify: `apps/mos-web/ui/src/Quando.tsx` (erro de passado)

**Interfaces:**
- Consumes: URLs da Task 5 (`/?task=`, `/?lembrete=`, `/?pagina=academico`) e da Task 6.
- Produces: `export type Destino = { pagina: Pagina; lembrete?: string; task?: string }`; `export function destinoDaUrl(busca: string): Destino | null`. Mensagem do SW para a janela: `{ tipo: "abrir", url: string }`.

- [ ] **Step 1: Teste que falha** — em `navegacao.test.ts`:

```ts
import { destinoDaUrl } from "./navegacao";

describe("o destino do toque no aviso", () => {
  it("abre o lembrete", () => {
    expect(destinoDaUrl("?lembrete=abc")).toEqual({ pagina: "lembretes", lembrete: "abc" });
  });
  it("abre a task", () => {
    expect(destinoDaUrl("?task=t1")).toEqual({ pagina: "fazer", task: "t1" });
  });
  it("abre o acadêmico", () => {
    expect(destinoDaUrl("?pagina=academico")).toEqual({ pagina: "academico" });
  });
  it("não inventa destino", () => {
    expect(destinoDaUrl("")).toBeNull();
    expect(destinoDaUrl("?pagina=nao-existe")).toBeNull();
  });
});
```

Run: `npx vitest run src/navegacao.test.ts` → FAIL (`destinoDaUrl` não existe).

- [ ] **Step 2: Implementar em `navegacao.ts`**

```ts
export type Destino = { pagina: Pagina; lembrete?: string; task?: string };

const PAGINAS: ReadonlyArray<Pagina> = [
  "home", "agenda", "capturar", "fazer", "mais",
  "horas", "academico", "lembretes", "biblioteca",
];

/**
 * Para onde o toque num aviso leva.
 *
 * O servidor escreve o destino na URL do aviso (`avisos.rs`,
 * `avisos_do_dia.rs`), e o service worker o entrega aqui — pela URL quando o
 * app abre do zero, por mensagem quando ele já estava aberto. Um parâmetro que
 * não se reconhece não vira página nenhuma: abrir a Home é melhor que abrir
 * uma tela em branco.
 */
export function destinoDaUrl(busca: string): Destino | null {
  const parametros = new URLSearchParams(busca);
  const lembrete = parametros.get("lembrete");
  if (lembrete) return { pagina: "lembretes", lembrete };
  const task = parametros.get("task");
  if (task) return { pagina: "fazer", task };
  const pagina = parametros.get("pagina");
  if (pagina && (PAGINAS as ReadonlyArray<string>).includes(pagina)) {
    return { pagina: pagina as Pagina };
  }
  return null;
}
```

Run: `npx vitest run src/navegacao.test.ts` → PASS.

- [ ] **Step 3: `sw.js` entrega o destino à janela aberta** — substitua o `notificationclick`:

```js
self.addEventListener("notificationclick", (evento) => {
  evento.notification.close();
  const destino = evento.notification.data?.url ?? "/";
  evento.waitUntil(
    // Reaproveita a janela que já existe em vez de abrir outra: no iPhone, uma
    // segunda janela do mesmo PWA é uma segunda cópia do app na tela. Com a
    // janela aberta, o destino vai por MENSAGEM — focar sem navegar deixava a
    // pessoa na tela onde estava, e o toque no aviso não levava a lugar nenhum.
    self.clients
      .matchAll({ type: "window", includeUncontrolled: true })
      .then((janelas) => {
        for (const janela of janelas) {
          if ("focus" in janela) {
            janela.postMessage({ tipo: "abrir", url: destino });
            return janela.focus();
          }
        }
        return self.clients.openWindow(destino);
      }),
  );
});
```

Confira no `push` handler do mesmo arquivo que `data: { url }` é passado ao `showNotification` (o `Aviso.url` do servidor). Se não for, acrescente `data: { url: aviso.url }`.

- [ ] **Step 4: `App.tsx` abre o destino** — perto dos outros `useEffect`, acrescente:

```tsx
  // O toque no aviso. Do zero, o destino vem na URL; com o app aberto, vem por
  // mensagem do service worker (`sw.js`). A URL é limpa depois de lida: sem
  // isso, recarregar a página reabriria o mesmo lembrete para sempre.
  useEffect(() => {
    function abrir(destino: Destino | null) {
      if (!destino) return;
      setPagina(destino.pagina);
      if (destino.lembrete) setLembreteAberto(destino.lembrete);
      if (destino.task) setTaskAberta(destino.task);
    }
    abrir(destinoDaUrl(window.location.search));
    if (window.location.search) window.history.replaceState(null, "", "/");

    function mensagem(evento: MessageEvent) {
      if (evento.data?.tipo !== "abrir" || typeof evento.data.url !== "string") return;
      abrir(destinoDaUrl(new URL(evento.data.url, window.location.origin).search));
    }
    navigator.serviceWorker?.addEventListener("message", mensagem);
    return () => navigator.serviceWorker?.removeEventListener("message", mensagem);
  }, []);
```

Importe `destinoDaUrl` e `type Destino` de `./navegacao`.

- [ ] **Step 5: A folha de criar não fecha no erro** — em `criarLembrete`, no `catch`, apague `setAgendando(null);` e deixe:

```tsx
    } catch (causa) {
      // A folha FICA aberta: fechar no erro jogava fora a escolha que a pessoa
      // fez, e o recado de erro aparecia sobre uma tela que já não mostrava o
      // que tinha falhado.
      reclamar(causa);
    }
```

- [ ] **Step 6: `Quando.tsx` recusa o passado antes do clique**

Depois de `const escolhas = disponiveis(base.current);`:

```tsx
  // O campo livre aceita digitar uma hora que já passou — `min` só vale no
  // seletor nativo. O servidor recusaria; dizer antes do toque poupa a ida e
  // diz o que fazer.
  const passou = !semData && quando.getTime() <= Date.now();
```

Abaixo do `<p className="folha-instante">`:

```tsx
        {passou ? (
          <p className="folha-erro" role="alert">
            Esse horário já passou. Escolha um horário futuro ou “Algum dia”.
          </p>
        ) : null}
```

E no botão de criar: `disabled={ocupado || passou}`. Em `telas.css`, junto das regras de `.folha-instante`:

```css
/* O erro em sodio, e nao em vermelho: e o unico acento do app (estilo.css). */
.folha-erro {
  margin: 0;
  font-size: 13px;
  color: var(--signal);
}
```

- [ ] **Step 7: Rodar e construir**

Run: `cd apps/mos-web/ui && npx tsc --noEmit && npx vitest run && npm run build`
Expected: PASS, build em `../static`.

- [ ] **Step 8: Commit**

```bash
git add apps/mos-web/ui
git commit -m "feat(mos-web): o toque no aviso abre o lembrete ou a task, e criar recusa o passado sem perder a folha"
```

---

### Task 9: Documentos, ADR e entrega

**Files:**
- Modify: `apps/mos-web/README.md` (tabela "O que avisa" e seção "Adiar não está na porta", ~linhas 180-205)
- Modify: `docs/PILOTO.md` (§7, ~linhas 173-175 e 229-233)
- Modify: `docs/DECISIONS.md` (ADR-092 no fim)
- Modify: `docs/superpowers/specs/2026-10-26-lembretes-do-bolso-design.md` (Estado: Implementado; §5: lembretes **não** passam pelo teto por hora — ver abaixo)

- [ ] **Step 1: README do mos-web**

Troque a tabela "O que avisa" por:

```markdown
| | |
|---|---|
| Lembrete que venceu | laço de 60s; o toque abre o lembrete (`/?lembrete=`) |
| Prova, task vencida/esquecida, compromisso | laço de 60s rodando `mos-core::piloto::avisos`; cala com o PC em uso (`presenca.rs`) |
```

Substitua a seção "**Adiar não está na porta.**" por um parágrafo dizendo que adiar existe, passa por `AttentionService::snooze` (registra histórico) e por que não há disputa com o agendador do desktop (o `mos-web` não tem agendador que escreva). E acrescente: "O iOS não mostra botões dentro da notificação; o toque abre o lembrete com Concluir e Adiar em cima."

- [ ] **Step 2: `PILOTO.md` §7** — troque a frase que diz que o bolso não roda o motor por: "Desde a ADR-092 o `mos-web` roda `candidatos` + `decidir` num laço de 60s e entrega por Web Push os tipos Academic, Overdue, ForgottenTask e Upcoming. Com o PC em uso (última tecla há ≤ 5 min, informada ao hub por `/sync/presenca`), o bolso segura esses avisos."

- [ ] **Step 3: ADR-092** — acrescente ao fim de `docs/DECISIONS.md`, no formato da ADR-089/075 (Contexto, Decisão, os oito eixos, o que ficou de fora). Pontos obrigatórios:
  - Contexto: os quatro pedidos do dono; iOS sem botões na notificação.
  - Decisão: §1–§5 do spec, com os números (5 min; tipos do bolso; tipos só do PC).
  - **Correção do spec:** lembretes **não** passam pelo teto por hora do `decidir` — são compromisso explícito da pessoa e já têm dedupe por vencimento e degrau. Só os avisos do piloto passam pelo teto e pelo cooldown.
  - Eixos: copie os do spec, atualizando `database` com a migração aditiva `em_uso_em` no hub.
  - Fora: botões na notificação (iOS), aviso de abertura de janela da prova, revisão visual do bolso.

- [ ] **Step 4: Spec** — mude `**Estado:**` para `implementado (ADR-092)` e corrija o §5 conforme o ponto acima.

- [ ] **Step 5: Verificação geral**

Run (raiz do repo):
```bash
cargo fmt --all -- --check
cargo test -p mos-sync-server -p mos-sync-http -p mos-web
cargo check -p mos-desktop
cd apps/mos-web/ui && npx tsc --noEmit && npx vitest run && cd ../../..
cd apps/desktop && npx tsc --noEmit && npx vitest run && cd ../..
```
Expected: tudo verde. Se `cargo fmt --check` reclamar, rode `cargo fmt --all` e inclua no commit.

- [ ] **Step 6: Commit e push**

```bash
git add apps/mos-web/README.md docs
git commit -m "docs: ADR-092 — lembretes e avisos do bolso"
git push origin master
```

- [ ] **Step 7: Entrega — três binários, nesta ordem**

A ordem importa: o hub precisa aceitar `/sync/presenca` antes de o desktop bater nela (antes disso o desktop só recebe 404 e ignora — não quebra, mas não cala nada).

1. **Hub** — workflow `Sync server` (`.github/workflows/sync-server.yml`, `workflow_dispatch` se não disparar no push). Baixe e instale:
   ```powershell
   gh run download <run-id> --name mos-sync-server-linux-x86_64 -D $env:TEMP\hub
   scp -i $env:USERPROFILE\.ssh\hermes_work $env:TEMP\hub\mos-sync-server hermes@167.233.43.1:/tmp/mos-sync-server
   ssh -t -i $env:USERPROFILE\.ssh\hermes_work hermes@167.233.43.1 'sudo install -m 755 /tmp/mos-sync-server /usr/local/bin/mos-sync-server && sudo systemctl restart mos-sync && sleep 2 && curl -s http://172.17.0.1:9120/health'
   ```
   A migração da coluna roda sozinha no primeiro start.
2. **mos-web** — workflow `M/OS web`; mesmo caminho, artifact `mos-web-linux-x86_64`, `sudo install ... /usr/local/bin/mos-web && sudo systemctl restart mos-web`.
3. **Desktop** — release normal do M/OS (workflow `package.yml`), ou rode o build de dev **isolado** do banco real (ver memória "Nao matar o M/OS em teste" e `docs/SETUP-MAQUINA.md`).

- [ ] **Step 8: Conferência no mundo real**

1. Com o PC em uso, `ssh ... 'curl -s -H "Authorization: Bearer $(sudo grep MOS_SYNC_TOKEN /etc/mos-sync.env | cut -d= -f2)" http://172.17.0.1:9120/sync/aparelhos'` mostra `emUsoEm` do PC com menos de 1 min.
2. Crie no iPhone um lembrete para daqui a 2 min. Quando o aviso chegar, toque: abre o lembrete, com as opções de adiar do desktop ("Amanhã 9h" etc.).
3. Com o app aberto em outra página, toque num aviso: ele troca para o lembrete.
4. Deixe o PC parado > 5 min com uma task vencida: o aviso chega no iPhone. Mexa no PC: o próximo aviso do dia não chega no iPhone (o lembrete continua chegando).
5. O aviso "N itens novos vieram do computador" não aparece mais.
```
