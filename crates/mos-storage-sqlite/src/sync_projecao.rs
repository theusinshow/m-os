//! A ponte entre operacao e dominio: a `Projecao` de verdade.
//!
//! O motor sabe reconciliar, mas nao sabe o que e uma Task. Este modulo e o
//! unico lugar do M/OS que sabe as duas coisas — e por isso ele e curto de
//! proposito.
//!
//! # As duas metades
//!
//! - `estado_de` devolve o que este dispositivo ja sabe da entidade, com o
//!   instante de cada campo. Sai da tabela sombra `sync_state`, e nao das
//!   tabelas de dominio, porque o dominio nao guarda instante por campo — ver o
//!   comentario da migration 0035.
//! - `guardar` grava o estado reconciliado e MATERIALIZA nas colunas do
//!   dominio, para as telas, a busca e os relatorios enxergarem.
//!
//! # Tipo desconhecido nao e erro
//!
//! Um cliente antigo pode receber uma operacao sobre um tipo que ele ainda nao
//! conhece (§27 e §74: versoes N e N-1 convivem). Ele guarda o estado e nao
//! materializa nada — e no dia em que atualizar, a materializacao acontece com
//! o dado que ja estava aqui. Descartar seria perder dado por ser velho.
//!
//! # Por que o `UPSERT` e nao um `UPDATE`
//!
//! A operacao que cria uma Task e um `Update` com titulo, descricao e project,
//! e ela pode chegar depois de um `Update` que so mexe no `workState`. Quem
//! recebe fora de ordem precisa terminar com a linha certa nas duas ordens, e
//! um `UPDATE` puro perderia a primeira metade.

use mos_core::CoreError;
use mos_sync::{EstadoDaEntidade, Op, Projecao, Resultado, SyncError};
use rusqlite::{params, Connection};
use time::OffsetDateTime;

use crate::{map_sql_error, SqliteStorage};

/// Como um tipo de entidade vira linha.
struct Mapa {
    tabela: &'static str,
    /// (campo emitido, coluna). O nome do campo e o que `emitir_update` usa; a
    /// coluna e a do banco.
    colunas: &'static [(&'static str, &'static str)],
    /// (coluna, literal SQL) para as colunas `NOT NULL` sem `DEFAULT`.
    ///
    /// Sao valores PROVISORIOS, e existem por causa da ordem de chegada: a
    /// operacao que traz o titulo pode chegar depois da que mexe no
    /// `workState`, e sem um valor aqui o `INSERT` bateria no `NOT NULL` e a
    /// entidade nunca apareceria. O `UPDATE` logo em seguida os substitui assim
    /// que o campo de verdade chega.
    obrigatorias: &'static [(&'static str, &'static str)],
}

/// Os tipos que este M/OS sabe materializar.
///
/// Cresce um por um, junto com a emissao. Um tipo fora desta lista continua
/// sincronizando — o estado e guardado e reenviado —, so nao aparece nas telas
/// ate alguem escrever a linha dele aqui.
fn mapa_de(kind: &str) -> Option<Mapa> {
    match kind {
        "task" => Some(Mapa {
            tabela: "tasks",
            colunas: &[
                ("title", "title"),
                ("description", "description"),
                ("projectId", "project_id"),
                ("workState", "work_state"),
                ("lifecycleState", "lifecycle_state"),
            ],
            obrigatorias: &[("title", "'(sem titulo)'"), ("description", "''")],
        }),
        "project" => Some(Mapa {
            tabela: "projects",
            colunas: &[
                ("name", "name"),
                ("description", "description"),
                ("lifecycleState", "lifecycle_state"),
            ],
            obrigatorias: &[("name", "'(sem nome)'"), ("description", "''")],
        }),
        _ => None,
    }
}

fn falha(causa: CoreError) -> SyncError {
    SyncError::novo(causa.message, causa.retryable)
}

impl SqliteStorage {
    /// O estado reconciliado guardado para esta entidade.
    pub(crate) fn estado_guardado(
        conexao: &Connection,
        kind: &str,
        id: uuid::Uuid,
    ) -> Result<EstadoDaEntidade, CoreError> {
        let json: Option<String> = conexao
            .query_row(
                "SELECT estado FROM sync_state WHERE entity_kind = ?1 AND entity_id = ?2",
                params![kind, id.to_string()],
                |linha| linha.get(0),
            )
            .ok();
        match json {
            Some(texto) => serde_json::from_str(&texto).map_err(|causa| {
                crate::sync_emit::erro_de_sync(format!("Estado de sync ilegivel: {causa}"))
            }),
            None => Ok(EstadoDaEntidade::default()),
        }
    }

    /// Grava o estado reconciliado. **Nao** materializa: quem materializa
    /// decide separado, porque a mudanca local ja escreveu a linha de dominio
    /// pelo caminho normal e reescrever seria desfazer o que ela acabou de
    /// fazer.
    pub(crate) fn guardar_estado(
        transacao: &Connection,
        kind: &str,
        id: uuid::Uuid,
        estado: &EstadoDaEntidade,
    ) -> Result<(), CoreError> {
        let json = serde_json::to_string(estado).map_err(|causa| {
            crate::sync_emit::erro_de_sync(format!("Estado de sync ilegivel: {causa}"))
        })?;
        let momento = crate::repository::format_time(OffsetDateTime::now_utc())?;
        transacao
            .execute(
                "INSERT INTO sync_state (entity_kind, entity_id, estado, updated_at) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(entity_kind, entity_id) DO UPDATE SET estado = ?3, updated_at = ?4",
                params![kind, id.to_string(), json, momento],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    /// Escreve o estado reconciliado nas colunas do dominio.
    fn materializar(
        transacao: &Connection,
        kind: &str,
        id: uuid::Uuid,
        estado: &EstadoDaEntidade,
    ) -> Result<(), CoreError> {
        let Some(mapa) = mapa_de(kind) else {
            return Ok(());
        };
        let momento = crate::repository::format_time(OffsetDateTime::now_utc())?;

        // O apagamento vira `trashed`, e nao `DELETE`.
        //
        // Sao dois motivos independentes, e os dois ja eram regra do M/OS antes
        // da sincronizacao: um dispositivo offline precisa distinguir "apagado"
        // de "nunca chegou", e todo Undo daqui e restauracao de estado.
        if estado.deleted_at.is_some() {
            transacao
                .execute(
                    &format!(
                        "UPDATE {} SET lifecycle_state = 'trashed', updated_at = ?1 WHERE id = ?2",
                        mapa.tabela
                    ),
                    params![momento, id.to_string()],
                )
                .map_err(map_sql_error)?;
            return Ok(());
        }

        // Garante a linha antes de escrever nela.
        //
        // `INSERT OR IGNORE`: a linha ja existe quando a entidade nasceu neste
        // dispositivo, e sobrescreve-la com os provisorios apagaria o que o
        // usuario acabou de digitar.
        let colunas: String = mapa
            .obrigatorias
            .iter()
            .map(|(coluna, _)| format!(", {coluna}"))
            .collect();
        let literais: String = mapa
            .obrigatorias
            .iter()
            .map(|(_, literal)| format!(", {literal}"))
            .collect();
        transacao
            .execute(
                &format!(
                    "INSERT OR IGNORE INTO {} (id, created_at, updated_at{colunas}) \
                     VALUES (?1, ?2, ?2{literais})",
                    mapa.tabela
                ),
                params![id.to_string(), momento],
            )
            .map_err(map_sql_error)?;

        for (campo, coluna) in mapa.colunas {
            let Some(resolvido) = estado.campos.get(*campo) else {
                continue;
            };
            let valor = valor_sql(&resolvido.valor);
            transacao
                .execute(
                    &format!(
                        "UPDATE {} SET {coluna} = ?1, updated_at = ?2 WHERE id = ?3",
                        mapa.tabela
                    ),
                    params![valor, momento, id.to_string()],
                )
                .map_err(map_sql_error)?;
        }
        Ok(())
    }

    /// Aplica uma operacao LOCAL sobre o estado de sync, na mesma transacao.
    ///
    /// Sem isto, a tabela sombra so saberia do que veio de fora, e a proxima
    /// operacao remota reconciliaria contra um estado sem as edicoes deste
    /// dispositivo — que e o mesmo que perde-las.
    pub(crate) fn absorver_local(&self, transacao: &Connection, op: &Op) -> Result<(), CoreError> {
        let kind = op.entity.kind.as_str();
        let base = Self::estado_guardado(transacao, kind, op.entity.id)?;
        let reconciliado = mos_sync::aplicar(base, std::slice::from_ref(op)).estado;
        Self::guardar_estado(transacao, kind, op.entity.id, &reconciliado)
    }
}

/// Traduz o JSON do campo para o que a coluna aceita.
///
/// `null` continua `null`; o resto vira texto. As colunas do M/OS sao TEXT com
/// `STRICT`, e um numero indo como inteiro numa coluna de texto e um erro de
/// tipo em vez de um valor.
fn valor_sql(valor: &serde_json::Value) -> Option<String> {
    match valor {
        serde_json::Value::Null => None,
        serde_json::Value::String(texto) => Some(texto.clone()),
        outro => Some(outro.to_string()),
    }
}

/// A `Projecao` do motor, sobre o banco de verdade.
pub struct ProjecaoSqlite<'a> {
    pub(crate) storage: &'a SqliteStorage,
}

impl<'a> ProjecaoSqlite<'a> {
    pub fn nova(storage: &'a SqliteStorage) -> Self {
        Self { storage }
    }
}

impl Projecao for ProjecaoSqlite<'_> {
    fn estado_de(&self, op: &Op) -> EstadoDaEntidade {
        let Ok(conexao) = self.storage.connection.lock() else {
            return EstadoDaEntidade::default();
        };
        SqliteStorage::estado_guardado(&conexao, op.entity.kind.as_str(), op.entity.id)
            .unwrap_or_default()
    }

    fn guardar(&mut self, op: &Op, estado: &EstadoDaEntidade) -> Resultado<()> {
        let conexao = self
            .storage
            .connection
            .lock()
            .map_err(|_| SyncError::novo("Banco local ocupado.", true))?;
        let transacao = conexao
            .unchecked_transaction()
            .map_err(map_sql_error)
            .map_err(falha)?;
        let kind = op.entity.kind.as_str();

        // Estado e dominio commitam JUNTOS. Gravar o estado e falhar ao
        // materializar deixaria a tela mostrando o valor antigo enquanto a
        // reconciliacao ja considera o novo — e a proxima operacao nao
        // corrigiria, porque para ela o assunto ja esta resolvido.
        SqliteStorage::guardar_estado(&transacao, kind, op.entity.id, estado).map_err(falha)?;
        SqliteStorage::materializar(&transacao, kind, op.entity.id, estado).map_err(falha)?;
        transacao.commit().map_err(map_sql_error).map_err(falha)?;
        Ok(())
    }
}
