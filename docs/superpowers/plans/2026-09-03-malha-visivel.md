# A malha visível — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fazer a malha de aparelhos ser visível e a identidade de cada um sobreviver, trocar o backfill booleano por geração, e deixar um endereço só para todo mundo alcançar o hub.

**Architecture:** Três mudanças independentes no cliente (`mos-storage-sqlite`), uma tabela e duas rotas no hub (`mos-sync-server`), um método fora do trait `Transport` no `mos-sync-http`, uma seção nova em `SettingsPage.tsx`, e um Caddyfile que separa `/sync/*` do resto. Nada disso encosta no motor de reconciliação nem no modelo local-first.

**Tech Stack:** Rust (rusqlite, axum 0.8, reqwest blocking), React 19 + TypeScript no desktop, Caddy na VPS. Nenhuma dependência nova em nenhum crate.

**Spec:** `docs/superpowers/specs/2026-09-03-malha-visivel-design.md`

## Global Constraints

- **O desktop continua funcionando com a VPS fora do ar.** Nada aqui pode tornar o hub necessário para abrir o app ou gravar localmente.
- **Nenhuma dependência nova**, em nenhum crate nem no `package.json`.
- **A batida de aparelho NÃO entra no trait `Transport`.** O trait espelha o que o motor precisa (`push`, `pull`); identidade de aparelho não é assunto do motor.
- **Falha na batida não interrompe a rodada.** O erro vai para o log; a sincronização continua.
- **O hub não decide nada com a versão** — nenhuma operação é recusada, nenhum cliente bloqueado. Ele grava o que o aparelho diz de si e devolve a lista.
- Comentários e nomes em português, como o resto do repositório. Comentário explica **por quê**, não o quê.
- `cargo fmt --all` e `cargo clippy --workspace --all-targets -- -D warnings` limpos: são portões do CI.
- Testar só o que a mudança cobre (`cargo test -p <crate>`), não a suíte inteira.

---

## File Structure

| Arquivo | Responsabilidade | Task |
| --- | --- | --- |
| `crates/mos-storage-sqlite/src/device_repository.rs` | âncora do `device_id` em `app_metadata` | 1 |
| `crates/mos-storage-sqlite/src/sync_backfill.rs` | marca vira geração | 2 |
| `crates/mos-storage-sqlite/src/sync_cobertura.rs` | teste que amarra cobertura à geração | 2 |
| `crates/mos-sync-server/src/hub.rs` | tabela `aparelhos`, `registrar_aparelho`, `aparelhos` | 3 |
| `crates/mos-sync-server/src/http.rs` | rotas `POST /sync/aparelho` e `GET /sync/aparelhos` | 3 |
| `crates/mos-sync-http/src/lib.rs` | `anunciar` e `malha`, fora do trait | 4 |
| `apps/desktop/src-tauri/src/sync.rs` | chama a batida; comando `sync_malha` | 4 |
| `apps/mos-web/src/sync.rs` | chama a batida | 4 |
| `apps/desktop/src/SettingsPage.tsx`, `api.ts`, `types.ts` | a seção "A MALHA" | 5 |
| `deploy/bootstrap-vps.sh`, `deploy/README.md` | Caddyfile com `/sync/*` | 6 |
| `scripts/sync-tunnel.ps1`, `scripts/install-sync-tunnel.ps1` | **removidos** | 6 |

---

### Task 1: A identidade que não se perde

**Files:**
- Modify: `crates/mos-storage-sqlite/src/device_repository.rs:50-90`
- Test: no próprio arquivo, em `#[cfg(test)] mod tests`

**Interfaces:**
- Produces: nenhuma assinatura nova — `este_dispositivo(nome, plataforma, versao) -> Resultado<Device>` passa a reusar o id ancorado.
- Chave usada: `app_metadata.key = 'sync_device_id'`.

- [ ] **Step 1: Write the failing test**

Acrescentar ao fim de `device_repository.rs`:

```rust
#[cfg(test)]
mod tests {
    use mos_sync::DeviceRepository;

    use crate::SqliteStorage;

    fn storage() -> (SqliteStorage, tempfile::TempDir) {
        let pasta = tempfile::tempdir().unwrap();
        let storage =
            SqliteStorage::open(pasta.path().join("mos.db"), pasta.path().join("backups")).unwrap();
        (storage, pasta)
    }

    /// O caso que aconteceu de verdade, em 02/09/2026.
    ///
    /// O PC do trabalho apareceu no hub com uma identidade NOVA, e com ela um
    /// relogio novo e um cursor zerado — comecou a baixar tudo de novo. A linha
    /// de `devices` pode sumir; o id nao pode.
    #[test]
    fn um_dispositivo_que_perdeu_a_linha_volta_com_o_mesmo_id() {
        let (storage, _guarda) = storage();
        let antes = storage.este_dispositivo("PC", "windows", "0.3.5").unwrap();

        // O sumico, direto no banco: e o que um banco recriado ou uma limpeza
        // por fora produzem.
        storage
            .escrita()
            .unwrap()
            .execute("DELETE FROM devices WHERE is_this_device = 1", [])
            .unwrap();

        let depois = storage.este_dispositivo("PC", "windows", "0.3.5").unwrap();
        assert_eq!(
            antes.id, depois.id,
            "o dispositivo nasceu de novo: o cursor e o relogio iriam junto"
        );
    }

    /// A ancora e a linha nascem juntas, ou nenhuma das duas.
    ///
    /// Ancora sem linha faria a proxima abertura ressuscitar um id que nunca
    /// existiu no hub.
    #[test]
    fn a_ancora_guarda_o_id_do_primeiro_registro() {
        let (storage, _guarda) = storage();
        let device = storage.este_dispositivo("PC", "windows", "0.3.5").unwrap();

        let ancora: String = storage
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT value FROM app_metadata WHERE key = 'sync_device_id'",
                [],
                |linha| linha.get(0),
            )
            .expect("a ancora nao foi gravada");
        assert_eq!(ancora, device.id.to_string());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mos-storage-sqlite --lib device_repository`
Expected: FAIL — `um_dispositivo_que_perdeu_a_linha_volta_com_o_mesmo_id` com ids diferentes, e `a_ancora_guarda_o_id_do_primeiro_registro` com "a ancora nao foi gravada".

- [ ] **Step 3: Implement**

Substituir o bloco `let id = match existente { ... };` de `este_dispositivo` por:

```rust
        // A ANCORA, e por que ela existe.
        //
        // A linha de `devices` pode sumir — banco recriado, limpeza por fora — e
        // com ela ia o id. Id novo significa relogio novo e cursor zerado: o
        // aparelho volta a baixar tudo, e aparece no hub como um segundo
        // dispositivo com os mesmos dados. Aconteceu em 02/09/2026, no PC do
        // trabalho, e custou uma manha para ser entendido.
        //
        // `app_metadata` sobrevive a isso porque nada no sync a apaga.
        let ancorado: Option<String> = connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = 'sync_device_id'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(erro_sql)?;

        let id = match existente {
            Some(id) => {
                connection
                    .execute(
                        "UPDATE devices SET name = ?2, platform = ?3, app_version = ?4, \
                         updated_at = ?5 WHERE id = ?1",
                        params![id, nome, plataforma, versao, agora],
                    )
                    .map_err(erro_sql)?;
                id
            }
            None => {
                // Sem linha: o id vem da ancora, e so nasce novo quando nem ela
                // existe. As duas gravacoes acontecem na MESMA transacao — uma
                // ancora sem linha ressuscitaria um id que o hub nunca viu.
                let id = ancorado.unwrap_or_else(|| DeviceId::novo().to_string());
                let transacao = connection.unchecked_transaction().map_err(erro_sql)?;
                transacao
                    .execute(
                        "INSERT INTO devices (id, name, platform, app_version, last_sync_at, \
                         is_this_device, created_at, updated_at) \
                         VALUES (?1, ?2, ?3, ?4, '', 1, ?5, ?5)",
                        params![id, nome, plataforma, versao, agora],
                    )
                    .map_err(erro_sql)?;
                transacao
                    .execute(
                        "INSERT INTO app_metadata (key, value) VALUES ('sync_device_id', ?1) \
                         ON CONFLICT(key) DO UPDATE SET value = ?1",
                        params![id],
                    )
                    .map_err(erro_sql)?;
                transacao.commit().map_err(erro_sql)?;
                id
            }
        };
```

Se `app_metadata` não tiver `key` como chave primária, o `ON CONFLICT(key)` falha ao compilar o SQL em tempo de execução — conferir com `PRAGMA table_info(app_metadata)` e usar `INSERT OR REPLACE` nesse caso.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p mos-storage-sqlite --lib device_repository`
Expected: PASS — 2 testes.

- [ ] **Step 5: Commit**

```bash
git add crates/mos-storage-sqlite/src/device_repository.rs
git commit -m "fix(sync): a identidade do aparelho passa a ter ancora

A linha de devices pode sumir, e com ela ia o id — relogio novo, cursor zerado,
e o aparelho aparecendo no hub como um segundo dispositivo com os mesmos dados.
Aconteceu no PC do trabalho em 02/09 e custou uma manha.

A ancora mora em app_metadata, que nada no sync apaga, e nasce na mesma
transacao da linha: ancora sem linha ressuscitaria um id que o hub nunca viu."
```

---

### Task 2: O backfill por geração

**Files:**
- Modify: `crates/mos-storage-sqlite/src/sync_backfill.rs:31` (a marca) e a função `backfill_do_sync`
- Modify: `crates/mos-storage-sqlite/src/sync_cobertura.rs` (o teste que amarra cobertura e geração)

**Interfaces:**
- Produces: `pub(crate) const GERACAO_ATUAL: u32 = 2;` em `sync_backfill.rs`; a chave `app_metadata.sync_backfill_geracao` guarda a geração já passada.

- [ ] **Step 1: Write the failing test**

Acrescentar ao `mod tests` de `sync_backfill.rs`:

```rust
    /// A armadilha que ja disparou uma vez.
    ///
    /// A marca era um booleano. Quando a cobertura cresceu de 12 para 26 tipos,
    /// quem ja tinha passado pelo backfill NUNCA re-emitiu o que passou a ser
    /// sincronizavel — e o dado velho ficou parado num PC so.
    #[test]
    fn geracao_menor_faz_o_backfill_rodar_de_novo() {
        let (storage, _guarda) = storage_ligado();
        semear_um_projeto(&storage);

        assert!(storage.backfill_do_sync().unwrap() > 0, "a primeira passagem nao emitiu");
        assert_eq!(storage.backfill_do_sync().unwrap(), 0, "passou duas vezes na mesma geracao");

        // O aparelho que parou na geracao anterior.
        storage
            .escrita()
            .unwrap()
            .execute(
                "INSERT INTO app_metadata (key, value) VALUES ('sync_backfill_geracao', '1') \
                 ON CONFLICT(key) DO UPDATE SET value = '1'",
                [],
            )
            .unwrap();

        assert!(
            storage.backfill_do_sync().unwrap() > 0,
            "a geracao nova nao re-emitiu: o dado velho ficaria parado"
        );
    }

    /// Quem vinha da marca antiga entra como geracao 1, e portanto re-emite.
    #[test]
    fn a_marca_antiga_conta_como_geracao_um() {
        let (storage, _guarda) = storage_ligado();
        semear_um_projeto(&storage);
        storage
            .escrita()
            .unwrap()
            .execute(
                "INSERT INTO app_metadata (key, value) VALUES ('sync_backfill_v1', '140')",
                [],
            )
            .unwrap();

        assert!(
            storage.backfill_do_sync().unwrap() > 0,
            "quem tinha a marca antiga precisa re-emitir na geracao 2"
        );
    }
```

Os dois auxiliares (`storage_ligado`, `semear_um_projeto`) já existem no `mod tests` do arquivo com outros nomes — reusar os que estiverem lá; se os nomes forem outros, ajustar as chamadas, sem criar helpers novos.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mos-storage-sqlite --lib sync_backfill`
Expected: FAIL — a terceira asserção de `geracao_menor_faz_o_backfill_rodar_de_novo` devolve 0.

- [ ] **Step 3: Implement**

Em `sync_backfill.rs`, trocar a constante e a leitura da marca:

```rust
/// A geracao da cobertura ja aplicada neste banco.
const MARCA: &str = "sync_backfill_geracao";

/// A marca de antes, quando isto era um booleano. Quem a tem passou pela
/// cobertura de 12 tipos, e portanto esta na geracao 1.
const MARCA_ANTIGA: &str = "sync_backfill_v1";

/// A geracao da cobertura ATUAL.
///
/// Sobe quando `sync_cobertura.rs` passa a incluir tipos que antes nao
/// atravessavam — e e isso que faz o backfill rodar de novo em quem ja tinha
/// passado por ele. A 1 cobria 12 tipos; a 2 cobre 26.
pub(crate) const GERACAO_ATUAL: u32 = 2;
```

E, dentro de `backfill_do_sync`, trocar o bloco `ja_passou` por:

```rust
        let geracao: u32 = transacao
            .query_row(
                "SELECT value FROM app_metadata WHERE key = ?1",
                params![MARCA],
                |linha| linha.get::<_, String>(0),
            )
            .optional()
            .map_err(map_sql_error)?
            .and_then(|valor| valor.parse().ok())
            .unwrap_or_else(|| {
                // Sem geracao gravada: quem tem a marca antiga esta na 1, e quem
                // nao tem nenhuma das duas nunca passou — geracao 0.
                let antiga: bool = transacao
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM app_metadata WHERE key = ?1)",
                        params![MARCA_ANTIGA],
                        |linha| linha.get(0),
                    )
                    .unwrap_or(false);
                u32::from(antiga)
            });

        if geracao >= GERACAO_ATUAL {
            return Ok(0);
        }
```

E a gravação no fim da transação:

```rust
        transacao
            .execute(
                "INSERT INTO app_metadata (key, value) VALUES (?1, ?2) \
                 ON CONFLICT(key) DO UPDATE SET value = ?2",
                params![MARCA, GERACAO_ATUAL.to_string()],
            )
            .map_err(map_sql_error)?;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p mos-storage-sqlite --lib sync_backfill`
Expected: PASS.

- [ ] **Step 5: O teste que impede a armadilha de voltar**

Acrescentar ao `mod tests` de `sync_cobertura.rs`:

```rust
    /// A cobertura da geracao 2, copiada a mao.
    ///
    /// A duplicacao E o mecanismo: mudar `SINCRONIZAVEIS` sem tocar aqui quebra
    /// o teste, e a mensagem manda subir `GERACAO_ATUAL`. Sem isto, a cobertura
    /// cresce e quem ja passou pelo backfill nunca re-emite — que foi
    /// exatamente o que aconteceu entre a v1 e a v0.3.4.
    const COBERTURA_DA_GERACAO_2: &[&str] = &[
        // Copiar aqui, na ordem, o conteudo de SINCRONIZAVEIS no momento da
        // implementacao. A lista tem 26 entradas hoje.
    ];

    #[test]
    fn mudar_a_cobertura_obriga_a_subir_a_geracao() {
        assert_eq!(
            super::SINCRONIZAVEIS,
            COBERTURA_DA_GERACAO_2,
            "A cobertura mudou. Suba `GERACAO_ATUAL` em `sync_backfill.rs` e \
             atualize `COBERTURA_DA_GERACAO_N` aqui — sem isso, quem ja passou \
             pelo backfill nunca re-emite o que voce acabou de incluir."
        );
        assert_eq!(crate::sync_backfill::GERACAO_ATUAL, 2);
    }
```

Preencher `COBERTURA_DA_GERACAO_2` com a lista real de `SINCRONIZAVEIS` (copiar do próprio arquivo, na ordem em que está).

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p mos-storage-sqlite --lib sync_cobertura`
Expected: PASS. Se falhar, a cópia da lista está errada — corrigir a cópia, e **não** a lista original.

- [ ] **Step 7: Commit**

```bash
git add crates/mos-storage-sqlite/src/sync_backfill.rs crates/mos-storage-sqlite/src/sync_cobertura.rs
git commit -m "fix(sync): o backfill passa a ter geracao, e a cobertura a exige

A marca era um booleano. Quando a cobertura cresceu de 12 para 26 tipos, quem ja
tinha passado pelo backfill nunca re-emitiu o que passou a ser sincronizavel — e
o dado velho ficou parado num PC so.

Agora a marca guarda a geracao, e marca menor que a atual faz o backfill rodar
de novo. Quem tinha a marca antiga entra como geracao 1.

O teste novo duplica a lista de proposito: mudar a cobertura sem subir a geracao
quebra o teste, e a mensagem diz o que fazer. A armadilha ja disparou uma vez."
```

---

### Task 3: A malha no hub

**Files:**
- Modify: `crates/mos-sync-server/src/hub.rs` (tabela + dois métodos + testes)
- Modify: `crates/mos-sync-server/src/http.rs` (duas rotas + o comentário do topo)

**Interfaces:**
- Produces: `Hub::registrar_aparelho(&mut self, aparelho: &AparelhoRegistrado, visto_em: &str) -> Resultado<()>` e `Hub::aparelhos(&self) -> Resultado<Vec<AparelhoRegistrado>>`, com

```rust
pub struct AparelhoRegistrado {
    pub id: String,
    pub nome: String,
    pub plataforma: String,
    pub versao: String,
    pub contrato: u32,
    pub visto_em: String,
}
```

  Rotas: `POST /sync/aparelho` (corpo `{id, nome, plataforma, versao, contrato}`) e `GET /sync/aparelhos` (devolve `{aparelhos: [...]}`), as duas com `Authorization: Bearer`.

- [ ] **Step 1: Write the failing test**

Acrescentar ao `mod tests` de `hub.rs`:

```rust
    #[test]
    fn o_hub_guarda_quem_e_cada_aparelho() {
        let mut hub = Hub::em_memoria().unwrap();
        let aparelho = AparelhoRegistrado {
            id: "01a0279d-18e1-78c2-991f-9e894e7214be".into(),
            nome: "DESKTOP-634TJR1".into(),
            plataforma: "windows".into(),
            versao: "0.3.5".into(),
            contrato: 1,
            visto_em: String::new(),
        };
        hub.registrar_aparelho(&aparelho, "2026-09-03T12:00:00Z").unwrap();

        let lista = hub.aparelhos().unwrap();
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].nome, "DESKTOP-634TJR1");
        assert_eq!(lista[0].versao, "0.3.5");
        // A hora e a do SERVIDOR: relogio de cliente errado e comum, e um
        // "visto ha 3 dias" que na verdade foi agora manda a investigacao para
        // o lado errado.
        assert_eq!(lista[0].visto_em, "2026-09-03T12:00:00Z");
    }

    #[test]
    fn a_batida_seguinte_atualiza_em_vez_de_duplicar() {
        let mut hub = Hub::em_memoria().unwrap();
        let mut aparelho = AparelhoRegistrado {
            id: "01a0279d-18e1-78c2-991f-9e894e7214be".into(),
            nome: "PC".into(),
            plataforma: "windows".into(),
            versao: "0.3.4".into(),
            contrato: 1,
            visto_em: String::new(),
        };
        hub.registrar_aparelho(&aparelho, "2026-09-03T12:00:00Z").unwrap();
        aparelho.versao = "0.3.5".into();
        hub.registrar_aparelho(&aparelho, "2026-09-03T12:30:00Z").unwrap();

        let lista = hub.aparelhos().unwrap();
        assert_eq!(lista.len(), 1, "a batida duplicou o aparelho");
        assert_eq!(lista[0].versao, "0.3.5");
        assert_eq!(lista[0].visto_em, "2026-09-03T12:30:00Z");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mos-sync-server`
Expected: FAIL — `AparelhoRegistrado` não existe.

- [ ] **Step 3: Implement no hub**

Em `hub.rs`, acrescentar ao `execute_batch` da abertura:

```sql
            CREATE TABLE IF NOT EXISTS aparelhos (
                -- O mesmo DeviceId que assina as operacoes: e ele que liga esta
                -- linha ao que aparece no log.
                id          TEXT PRIMARY KEY,
                nome        TEXT NOT NULL,
                plataforma  TEXT NOT NULL,
                versao      TEXT NOT NULL,
                contrato    INTEGER NOT NULL,
                visto_em    TEXT NOT NULL
            );
```

E os dois métodos:

```rust
/// O que um aparelho diz de si.
///
/// Nao e regra, e metadado: o hub grava e devolve, sem decidir nada com isso —
/// nenhuma operacao e recusada por versao, nenhum cliente e bloqueado.
#[derive(Debug, Clone)]
pub struct AparelhoRegistrado {
    pub id: String,
    pub nome: String,
    pub plataforma: String,
    pub versao: String,
    pub contrato: u32,
    pub visto_em: String,
}

impl Hub {
    /// A batida. `visto_em` e a hora do SERVIDOR, e nao a que o cliente mandou.
    pub fn registrar_aparelho(
        &mut self,
        aparelho: &AparelhoRegistrado,
        visto_em: &str,
    ) -> Resultado<()> {
        self.conexao.execute(
            "INSERT INTO aparelhos (id, nome, plataforma, versao, contrato, visto_em) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(id) DO UPDATE SET nome = ?2, plataforma = ?3, versao = ?4, \
             contrato = ?5, visto_em = ?6",
            rusqlite::params![
                aparelho.id,
                aparelho.nome,
                aparelho.plataforma,
                aparelho.versao,
                aparelho.contrato,
                visto_em,
            ],
        )?;
        Ok(())
    }

    /// Quem esta na malha, o mais recente primeiro.
    pub fn aparelhos(&self) -> Resultado<Vec<AparelhoRegistrado>> {
        let mut consulta = self.conexao.prepare(
            "SELECT id, nome, plataforma, versao, contrato, visto_em FROM aparelhos \
             ORDER BY visto_em DESC",
        )?;
        let linhas = consulta.query_map([], |linha| {
            Ok(AparelhoRegistrado {
                id: linha.get(0)?,
                nome: linha.get(1)?,
                plataforma: linha.get(2)?,
                versao: linha.get(3)?,
                contrato: linha.get::<_, i64>(4)? as u32,
                visto_em: linha.get(5)?,
            })
        })?;
        let mut aparelhos = Vec::new();
        for linha in linhas {
            aparelhos.push(linha?);
        }
        Ok(aparelhos)
    }
}
```

Se o campo da conexão em `Hub` tiver outro nome que `conexao`, usar o nome real — conferir na `struct Hub` (linha 36).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p mos-sync-server`
Expected: PASS.

- [ ] **Step 5: As rotas**

Em `http.rs`, acrescentar ao `rotas`:

```rust
        .route("/sync/aparelho", post(registrar_aparelho))
        .route("/sync/aparelhos", get(aparelhos))
```

E os handlers:

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AparelhoPedido {
    id: String,
    nome: String,
    plataforma: String,
    versao: String,
    contrato: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AparelhoJson {
    id: String,
    nome: String,
    plataforma: String,
    versao: String,
    contrato: u32,
    visto_em: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MalhaResposta {
    aparelhos: Vec<AparelhoJson>,
}

/// A batida de um aparelho.
///
/// O contrato NAO e conferido aqui, e a ausencia e a decisao: um aparelho velho
/// demais para sincronizar ainda precisa conseguir se anunciar — e ver "versao
/// 0.2.9, visto ha 3 dias" na tela e exatamente como se descobre isso.
async fn registrar_aparelho(
    State(estado): State<Estado>,
    cabecalhos: HeaderMap,
    Json(pedido): Json<AparelhoPedido>,
) -> Result<StatusCode, Erro> {
    if !autorizado(&cabecalhos, &estado.token) {
        return Err(negado());
    }
    let aparelho = crate::hub::AparelhoRegistrado {
        id: pedido.id,
        nome: pedido.nome,
        plataforma: pedido.plataforma,
        versao: pedido.versao,
        contrato: pedido.contrato,
        visto_em: String::new(),
    };
    let agora = agora_iso();
    let mut hub = estado.hub.lock().expect("hub envenenado");
    hub.registrar_aparelho(&aparelho, &agora)
        .map_err(falha_no_banco)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn aparelhos(
    State(estado): State<Estado>,
    cabecalhos: HeaderMap,
) -> Result<Json<MalhaResposta>, Erro> {
    if !autorizado(&cabecalhos, &estado.token) {
        return Err(negado());
    }
    let hub = estado.hub.lock().expect("hub envenenado");
    let lista = hub.aparelhos().map_err(falha_no_banco)?;
    Ok(Json(MalhaResposta {
        aparelhos: lista
            .into_iter()
            .map(|a| AparelhoJson {
                id: a.id,
                nome: a.nome,
                plataforma: a.plataforma,
                versao: a.versao,
                contrato: a.contrato,
                visto_em: a.visto_em,
            })
            .collect(),
    }))
}
```

- [ ] **Step 6: Atualizar o comentário do topo do `http.rs`**

O topo do arquivo diz que uma terceira rota que o `Transport` não pede significa "alguém colocou regra no servidor". Substituir esse parágrafo por:

```rust
//! # Duas rotas de contrato, e duas de metadado
//!
//! `push` e `pull` sao a traducao literal do `Transport`, e nenhuma regra de
//! dominio pode entrar nelas — o `SYNC.md` § "o servidor coordena e persiste"
//! continua valendo palavra por palavra.
//!
//! `/sync/aparelho` e `/sync/aparelhos` sao a excecao consciente: elas nao
//! carregam regra nenhuma. O hub grava o que o aparelho diz de si — nome,
//! plataforma, versao, contrato — e devolve a lista. Nenhuma operacao e
//! recusada por causa disso, nenhum cliente e bloqueado. Elas existem porque a
//! pergunta "quem esta na malha, e em que versao" nao tinha onde ser
//! respondida, e responde-la custou uma manha de investigacao em 02/09/2026.
```

- [ ] **Step 7: Conferir que compila e que os testes passam**

Run: `cargo test -p mos-sync-server`
Expected: PASS, sem avisos de import não usado.

- [ ] **Step 8: Commit**

```bash
git add crates/mos-sync-server/src
git commit -m "feat(sync): o hub passa a saber quem esta na malha

O hub tinha uma tabela so, o log. A pergunta 'o outro PC esta atrasado?' nao
tinha onde ser respondida — nem no servidor nem na tela —, e responde-la custou
uma manha de investigacao com curl no tunel.

Agora ha `aparelhos` e duas rotas. Elas nao carregam regra: o hub grava o que o
aparelho diz de si e devolve a lista. Nenhuma operacao e recusada por versao,
nenhum cliente e bloqueado — e o comentario do topo do http.rs foi reescrito
para dizer isso, em vez de ser contrariado em silencio.

A hora e a do servidor: relogio de cliente errado e comum, e um 'visto ha 3
dias' que na verdade foi agora manda a investigacao para o lado errado."
```

---

### Task 4: A batida, no cliente

**Files:**
- Modify: `crates/mos-sync-http/src/lib.rs` (dois métodos inerentes, fora do trait)
- Modify: `apps/desktop/src-tauri/src/sync.rs` (chamar antes da rodada; comando `sync_malha`)
- Modify: `apps/desktop/src-tauri/src/lib.rs` (registrar o comando)
- Modify: `apps/mos-web/src/sync.rs` (chamar antes da rodada)

**Interfaces:**
- Consumes: as rotas da Task 3.
- Produces:

```rust
pub struct Anuncio<'a> {
    pub id: &'a str,
    pub nome: &'a str,
    pub plataforma: &'a str,
    pub versao: &'a str,
    pub contrato: u32,
}

pub struct AparelhoNaMalha {
    pub id: String,
    pub nome: String,
    pub plataforma: String,
    pub versao: String,
    pub contrato: u32,
    pub visto_em: String,
}

impl HttpTransport {
    pub fn anunciar(&self, anuncio: &Anuncio<'_>) -> Resultado<()>;
    pub fn malha(&self) -> Resultado<Vec<AparelhoNaMalha>>;
}
```

  E o comando Tauri `sync_malha() -> Result<Vec<AparelhoNaMalha>, String>`, serializado em camelCase.

- [ ] **Step 1: Implementar no transporte**

Em `mos-sync-http/src/lib.rs`, acrescentar depois do `impl HttpTransport` existente:

```rust
/// O que este aparelho diz de si ao hub.
///
/// Emprestado, e nao dono: quem chama ja tem as quatro coisas na mao, e clonar
/// para anunciar seria alocar por batida sem motivo.
pub struct Anuncio<'a> {
    pub id: &'a str,
    pub nome: &'a str,
    pub plataforma: &'a str,
    pub versao: &'a str,
    pub contrato: u32,
}

/// Um aparelho, como o hub o conhece.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AparelhoNaMalha {
    pub id: String,
    pub nome: String,
    pub plataforma: String,
    pub versao: String,
    pub contrato: u32,
    /// RFC3339, na hora do servidor.
    pub visto_em: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MalhaResposta {
    aparelhos: Vec<AparelhoNaMalha>,
}

/// A batida e a lista vivem FORA do trait `Transport`, e de proposito.
///
/// O trait espelha o que o motor precisa: `push` e `pull`. Identidade de
/// aparelho nao e assunto do motor — enfia-la la dentro obrigaria toda
/// implementacao futura (um transporte de teste, um por arquivo) a fingir que
/// sabe o que e uma versao de app.
impl HttpTransport {
    pub fn anunciar(&self, anuncio: &Anuncio<'_>) -> Resultado<()> {
        let resposta = self
            .cliente
            .post(format!("{}/sync/aparelho", self.base))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({
                "id": anuncio.id,
                "nome": anuncio.nome,
                "plataforma": anuncio.plataforma,
                "versao": anuncio.versao,
                "contrato": anuncio.contrato,
            }))
            .send()
            .map_err(sem_alcance)?;
        if !resposta.status().is_success() {
            return Err(erro_de_status(resposta));
        }
        Ok(())
    }

    pub fn malha(&self) -> Resultado<Vec<AparelhoNaMalha>> {
        let resposta = self
            .cliente
            .get(format!("{}/sync/aparelhos", self.base))
            .bearer_auth(&self.token)
            .send()
            .map_err(sem_alcance)?;
        if !resposta.status().is_success() {
            return Err(erro_de_status(resposta));
        }
        let corpo: MalhaResposta = resposta
            .json()
            .map_err(|causa| SyncError::novo(format!("Malha ilegivel: {causa}"), false))?;
        Ok(corpo.aparelhos)
    }
}
```

Os nomes `sem_alcance` e `erro_de_status` são os helpers que já existem no arquivo (linhas ~82 e ~91) — conferir os nomes reais e usar os que estiverem lá. Se `serde_json` não estiver nas dependências do crate, montar o corpo com uma struct `#[derive(Serialize)]` em vez do `json!`, para não acrescentar dependência.

- [ ] **Step 2: Chamar no desktop**

Em `apps/desktop/src-tauri/src/sync.rs`, dentro da closure de `spawn_blocking` de `rodar`, antes do `sincronizar_agora`:

```rust
        // A batida ANTES da rodada, e o erro dela nao interrompe nada: quem nao
        // conseguiu se anunciar ainda tem trabalho a sincronizar, e trocar dado
        // por metadado seria pessimo negocio.
        if let Ok(eu) = mos_sync::DeviceRepository::este_dispositivo(
            storage.as_ref(),
            &std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Este PC".to_owned()),
            "windows",
            env!("CARGO_PKG_VERSION"),
        ) {
            if let Err(causa) = transporte.anunciar(&mos_sync_http::Anuncio {
                id: &eu.id.to_string(),
                nome: &eu.name,
                plataforma: "windows",
                versao: env!("CARGO_PKG_VERSION"),
                contrato: mos_sync::CONTRACT_VERSION,
            }) {
                eprintln!("[sync] a batida nao chegou: {}", causa.mensagem);
            }
        }
```

E o comando novo, ao lado de `sync_now`:

```rust
/// Quem esta na malha, como o hub conhece.
///
/// Vazio nao e erro: um hub que ainda nao recebeu batida de ninguem responde
/// lista vazia, e a tela sabe dizer isso melhor que uma mensagem de falha.
#[tauri::command]
pub async fn sync_malha(app: tauri::AppHandle) -> Result<Vec<mos_sync_http::AparelhoNaMalha>, String> {
    let state = app.state::<crate::AppState>();
    let settings = crate::load_settings(&state.settings_path);
    let endpoint = settings.sync_endpoint.clone();
    if endpoint.is_empty() {
        return Ok(Vec::new());
    }
    let Some(token) = token_guardado() else {
        return Ok(Vec::new());
    };
    tauri::async_runtime::spawn_blocking(move || {
        let transporte = mos_sync_http::HttpTransport::novo(endpoint, token)
            .map_err(|erro| erro.mensagem)?;
        transporte.malha().map_err(|erro| erro.mensagem)
    })
    .await
    .map_err(|erro| format!("A consulta a malha nao terminou: {erro}"))?
}
```

Registrar `sync::sync_malha` no `invoke_handler` de `lib.rs`, ao lado de `sync::sync_now`.

- [ ] **Step 3: Chamar no bolso**

Em `apps/mos-web/src/sync.rs`, dentro de `rodar`, logo depois de montar o transporte:

```rust
    // Mesma batida do desktop, e pelo mesmo motivo: sem ela, o M/OS de bolso e
    // um aparelho que sincroniza e nao aparece em lugar nenhum.
    if let Ok(eu) = mos_sync::DeviceRepository::este_dispositivo(storage, "M/OS de bolso", "web", env!("CARGO_PKG_VERSION")) {
        if let Err(causa) = transporte.anunciar(&mos_sync_http::Anuncio {
            id: &eu.id.to_string(),
            nome: &eu.name,
            plataforma: "web",
            versao: env!("CARGO_PKG_VERSION"),
            contrato: mos_sync::CONTRACT_VERSION,
        }) {
            eprintln!("[web] a batida nao chegou: {}", causa.mensagem);
        }
    }
```

- [ ] **Step 4: Compilar e rodar os testes**

Run: `cargo build --workspace`
Expected: sem erros.

Run: `cargo test -p mos-sync-http -p mos-sync-server`
Expected: PASS.

- [ ] **Step 5: Provar contra um hub de verdade**

Com o hub rodando local (ou pelo túnel, que ainda existe nesta altura):

```bash
curl -s -X POST http://127.0.0.1:9120/sync/aparelho \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"id":"01a0279d-18e1-78c2-991f-9e894e7214be","nome":"TESTE","plataforma":"windows","versao":"0.3.5","contrato":1}' \
  -o /dev/null -w '%{http_code}\n'
curl -s http://127.0.0.1:9120/sync/aparelhos -H "Authorization: Bearer $TOKEN"
```

Expected: `204`, e depois um JSON com `TESTE` dentro de `aparelhos`.

- [ ] **Step 6: Commit**

```bash
git add crates/mos-sync-http/src/lib.rs apps/desktop/src-tauri/src apps/mos-web/src/sync.rs
git commit -m "feat(sync): cada aparelho passa a se anunciar ao hub

A batida vive FORA do trait Transport: o trait espelha o que o motor precisa —
push e pull —, e identidade de aparelho nao e assunto do motor. Enfia-la la
obrigaria toda implementacao futura a fingir que sabe o que e uma versao de app.

O erro da batida nao interrompe a rodada. Quem nao conseguiu se anunciar ainda
tem trabalho a sincronizar, e trocar dado por metadado seria pessimo negocio."
```

---

### Task 5: A malha na tela

**Files:**
- Modify: `apps/desktop/src/types.ts` (o tipo, e dois campos em `SyncStatus`)
- Modify: `apps/desktop/src/api.ts` (a chamada)
- Modify: `apps/desktop/src/SettingsPage.tsx` (a seção, dentro de `SyncSettings`)
- Modify: `apps/desktop/src-tauri/src/sync.rs` (`SyncStatus` ganha `device_id` e `app_version`)
- Modify: `apps/desktop/src/App.css` (as regras `.malha`)

**Interfaces:**
- Consumes: `sync_malha` da Task 4.
- Produces: `type AparelhoNaMalha = { id: string; nome: string; plataforma: string; versao: string; contrato: number; vistoEm: string }`.

- [ ] **Step 1: O tipo e a chamada**

Em `types.ts`:

```ts
/** Um aparelho da malha, como o hub o conhece.
 *
 *  `vistoEm` é a hora do SERVIDOR: relógio de cliente errado é comum, e um
 *  "visto há 3 dias" que na verdade foi agora manda a investigação para o lado
 *  errado. */
export type AparelhoNaMalha = {
  id: string;
  nome: string;
  plataforma: string;
  versao: string;
  contrato: number;
  vistoEm: string;
};
```

Em `api.ts`, ao lado de `syncStatus`:

```ts
  /** Quem está na malha. Lista vazia quando o hub não foi configurado. */
  syncMalha() {
    return invoke<AparelhoNaMalha[]>("sync_malha");
  },
```

- [ ] **Step 2: A seção**

Em `SettingsPage.tsx`, dentro de `SyncSettings`, acrescentar o estado e a carga:

```tsx
  const [malha, setMalha] = useState<AparelhoNaMalha[]>([]);
```

E dentro de `refresh`, depois de `setStatus(next)`:

```tsx
    // A malha falha em silêncio de propósito: o hub pode estar fora, e uma
    // seção vazia é melhor que a tela inteira de sincronização recusando abrir.
    setMalha(await api.syncMalha().catch(() => []));
```

E o bloco, logo depois da `<dl className="fact-grid">`:

```tsx
    {malha.length > 0 ? <>
      <p className="rotulo">A MALHA</p>
      <ul className="malha">
        {malha.map((aparelho) => {
          const euMesmo = aparelho.id === status?.deviceId;
          const divergente = aparelho.versao !== status?.appVersion;
          return <li key={aparelho.id} data-divergente={divergente || undefined}>
            <span className="malha-nome">{aparelho.nome}</span>
            <span className="malha-versao">{aparelho.versao}</span>
            <span className="malha-visto">
              {euMesmo ? "este aparelho" : relativeTime(aparelho.vistoEm)}
              {/* Aviso, e não bloqueio: versão diferente não impede
                  sincronizar, e a frase é o que encerra a investigação. */}
              {divergente ? ` · em versão diferente` : ""}
            </span>
          </li>;
        })}
      </ul>
    </> : null}
```

Isto exige dois campos novos em `SyncStatus` (`deviceId` e `appVersion`), que o comando `sync_status` passa a devolver — acrescentar em `apps/desktop/src-tauri/src/sync.rs`:

```rust
    /// O id deste aparelho, para a tela saber qual linha da malha e ela mesma.
    pub device_id: String,
    /// A versao deste app, para a tela marcar quem esta em versao diferente.
    pub app_version: String,
```

preenchidos com `mos_sync::DeviceRepository::este_dispositivo(...)` (ou string vazia se falhar) e `env!("CARGO_PKG_VERSION")`. E os mesmos dois campos em `types.ts`, no tipo `SyncStatus`.

- [ ] **Step 3: O CSS**

Em `apps/desktop/src/App.css`, junto das outras regras de Settings:

```css
/* A MALHA.
   Uma linha por aparelho, em grade de três colunas para o nome, a versão e o
   quando ficarem alinhados entre linhas — desalinhados, comparar duas versões
   exige varrer o texto. */
.malha {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-2);
}

.malha li {
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: var(--space-3);
  align-items: baseline;
  font-size: 13px;
}

.malha-versao {
  font-family: var(--font-system);
  color: var(--text-secondary);
}

.malha-visto {
  color: var(--text-system);
  font-size: 12px;
}

.malha li[data-divergente] .malha-versao {
  color: var(--signal);
}
```

- [ ] **Step 4: Conferir na janela de verdade**

Run: `cd apps/desktop && npm run typecheck && npm test`
Expected: sem erros de tipo; testes passam.

Subir o app (`npm run tauri dev`), abrir Settings → SINCRONIZAÇÃO e fotografar com a skill `ver-o-app`. Conferir: os aparelhos aparecem, o desta máquina diz "este aparelho", e quem estiver em versão diferente aparece em âmbar.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src apps/desktop/src-tauri/src/sync.rs
git commit -m "feat(sync): a malha aparece na tela de sincronizacao

Quem sao os aparelhos, em que versao cada um esta, e quando cada um falou pela
ultima vez. E a frase que teria encerrado em dois segundos a investigacao de
02/09 — a que custou uma manha e um curl dentro do tunel.

Versao diferente vira aviso em ambar, e nao bloqueio: sincronizar entre versoes
diferentes continua permitido, mas deixa de ser invisivel."
```

---

### Task 6: Um endereço só

**Files:**
- Modify: `deploy/bootstrap-vps.sh` (o Caddyfile)
- Modify: `deploy/README.md` (o passo do túnel sai)
- Delete: `scripts/sync-tunnel.ps1`, `scripts/install-sync-tunnel.ps1`

- [ ] **Step 1: O Caddyfile**

Em `deploy/bootstrap-vps.sh`, trocar o bloco `cat >/etc/caddy/Caddyfile` por:

```bash
cat >/etc/caddy/Caddyfile <<CADDY
$DOMINIO {
	# O hub, FORA do basic_auth. Nao e descuido: o cliente manda
	# \`Authorization: Bearer\`, e o basic_auth do Caddy recusaria antes de o hub
	# ver o token. A protecao aqui e o segredo de 64 caracteres, comparado em
	# tempo constante no proprio hub.
	@sync path /sync/*
	handle @sync {
		reverse_proxy 127.0.0.1:9120
	}

	# Todo o resto e o M/OS de bolso, atras da senha.
	handle {
		basic_auth {
			$USUARIO $HASH
		}
		reverse_proxy 127.0.0.1:9130
	}
}
CADDY
```

- [ ] **Step 2: Aplicar na VPS**

O `bootstrap-vps.sh` é idempotente. Rodar (pede sudo, então é passo do dono):

```
ssh hermes@167.233.43.1 -t "sudo bash /caminho/do/bootstrap-vps.sh"
```

Ou, se preferir só o Caddy: editar `/etc/caddy/Caddyfile` com o bloco acima e `sudo systemctl reload caddy`.

- [ ] **Step 3: Provar da máquina de fora**

```bash
# Sem token: 401 do HUB, e nao o desafio do basic_auth.
curl -s -o /dev/null -w '%{http_code}\n' "https://167-233-43-1.sslip.io/sync/pull?contrato=1&cursor=&limite=1"

# Com token: 200 e um lote.
curl -s -H "Authorization: Bearer $TOKEN" "https://167-233-43-1.sslip.io/sync/pull?contrato=1&cursor=&limite=1" | head -c 200

# A raiz continua pedindo senha.
curl -s -o /dev/null -w '%{http_code}\n' https://167-233-43-1.sslip.io/
```

Expected: `401`, depois um JSON com `ops`, depois `401` na raiz (desafio do basic_auth).

Se o primeiro devolver `401` **com** cabeçalho `WWW-Authenticate: Basic`, o matcher não pegou — o `handle @sync` precisa vir antes do `handle` genérico.

- [ ] **Step 4: Trocar o endereço nos dois PCs**

Em Settings → SINCRONIZAÇÃO, endereço vira `https://167-233-43-1.sslip.io`. O segredo continua o mesmo. Clicar em **Sincronizar agora** e conferir que a rodada acontece com o túnel **derrubado**.

Depois, remover a tarefa em cada PC:

```powershell
Unregister-ScheduledTask -TaskName "M-OS Sync Tunnel" -Confirm:$false
```

- [ ] **Step 5: Apagar os scripts e o passo do README**

```bash
git rm scripts/sync-tunnel.ps1 scripts/install-sync-tunnel.ps1
```

Em `deploy/README.md`, substituir a instrução do túnel por:

```markdown
Os PCs alcançam o hub pelo mesmo endereço do bolso —
`https://167-233-43-1.sslip.io` — porque o Caddy manda `/sync/*` para o hub,
fora do `basic_auth`. Não há túnel: ele existia porque o hub só escutava em
`127.0.0.1`, e era a peça que quebrava calada.
```

- [ ] **Step 6: Commit**

```bash
git add deploy scripts
git commit -m "feat(sync): um endereco so, e o tunel morre

O Caddy passa a mandar /sync/* para o hub, fora do basic_auth — o cliente manda
Bearer, e o basic_auth recusaria antes de o hub ver o token. A protecao e o
segredo de 64 caracteres, comparado em tempo constante.

O que isso expoe, dito com todas as letras: o hub passa a ser alcancavel da
internet, e quem tiver o segredo le e escreve o log inteiro. Antes exigia o
segredo E uma chave SSH. A troca e deliberada: some a unica peca que quebrava
calada, e o endereco passa a ser o mesmo nos tres aparelhos.

Os scripts do tunel saem do repositorio. Script que sobra e script que alguem
roda de novo."
```

---

## Self-Review

**Cobertura do spec:**

| Seção do spec | Task |
| --- | --- |
| 1. Identidade que não se perde (âncora em `app_metadata`, mesma transação) | 1 |
| 2. A malha no hub (tabela, duas rotas, hora do servidor, comentário do `http.rs`) | 3 |
| 2. A malha no cliente (fora do trait, falha não interrompe) | 4 |
| 2. A malha na tela (lista, aviso de versão divergente) | 5 |
| 3. Backfill por geração (constante, migração da marca antiga) | 2 |
| 3. Teste que amarra cobertura e geração | 2 |
| 4. Caddy com `/sync/*`, endereço único, túnel e scripts removidos | 6 |
| Verificação por `curl` contra o servidor de verdade | 4 (hub local) e 6 (público) |

**Nomes entre tasks:** `sync_device_id` (Task 1) é lido só pela Task 1. `GERACAO_ATUAL` (Task 2) é lido pelo teste da própria Task 2. `AparelhoRegistrado` (Task 3, servidor) e `AparelhoNaMalha` (Task 4, cliente) são tipos distintos de propósito — um é a linha do banco do hub, o outro é o que viaja; `Anuncio` (Task 4) é o corpo do POST. `sync_malha` (Task 4) é consumido pela Task 5, que também depende dos campos `deviceId` e `appVersion` acrescentados a `SyncStatus` na própria Task 5.

**O que este plano NÃO faz:** não mexe no motor de reconciliação, não muda o contrato (`CONTRACT_VERSION` segue 1), não toca no modelo local-first, e não investiga por que o CronoCAD não apareceu no PC2 — isso espera o diagnóstico daquela máquina e ganha spec próprio.
