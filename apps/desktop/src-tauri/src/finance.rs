//! Credenciais e cliente HTTP do M-Finance: a Action API (escrita) e o
//! Intelligence Gateway (leitura) — ADR-073.
//!
//! Mesmo padrao de `mos-hermes/src/auth.rs`: os segredos vivem so no Windows
//! Credential Manager, nunca na memoria do renderer nem em disco em texto
//! claro. Diferente do Hermes, aqui nao ha sessao — cada chamada manda o
//! segredo no header `Authorization`, como o proprio M-Finance ja faz para o
//! cron do Vercel (`app/api/cron/reminders`).
//!
//! Dois segredos, dois escopos. O de ACAO escreve (e tambem le); o de LEITURA
//! so le, e a Action API nunca o aceita. Sem o de leitura configurado, a
//! leitura usa o de acao — funciona, e a divida esta escrita na ADR.

use std::time::Duration;

use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "m-os";
const ACTION_ACCOUNT: &str = "finance-action-secret";
const READ_ACCOUNT: &str = "finance-read-secret";
/// O host do M-Finance, escrito uma vez so. O gate de permissao em
/// `hermes.rs` acha o App do Registry por ele, e as URLs abaixo sao montadas a
/// partir dele — se pudessem divergir, o M/OS acabaria pedindo permissao para
/// um destino e escrevendo em outro.
macro_rules! finance_host {
    () => {
        "m-finance-silk.vercel.app"
    };
}

pub const ACTION_HOST: &str = finance_host!();
const ACTION_API_URL: &str = concat!("https://", finance_host!(), "/api/mos/actions");
const QUERY_API_URL: &str = concat!("https://", finance_host!(), "/api/mos/finance/query");

/// Leitura comum volta em segundos; a analise pesada pode levar quase um
/// minuto do lado do M-Finance.
const QUERY_TIMEOUT: Duration = Duration::from_secs(15);
const ANALYZE_TIMEOUT: Duration = Duration::from_secs(70);
const ACTION_TIMEOUT: Duration = Duration::from_secs(30);

fn entry(account: &str) -> Result<Entry, String> {
    Entry::new(SERVICE, account)
        .map_err(|error| format!("Credential Manager indisponivel: {error}"))
}

fn store(account: &str, secret: &str) -> Result<(), String> {
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return Err("O secret nao pode ficar vazio.".into());
    }
    entry(account)?
        .set_password(trimmed)
        .map_err(|error| format!("Nao foi possivel guardar: {error}"))
}

fn clear(account: &str) -> Result<(), String> {
    match entry(account)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("Nao foi possivel remover: {error}")),
    }
}

fn load(account: &str) -> Option<String> {
    entry(account).ok()?.get_password().ok()
}

#[tauri::command]
pub fn finance_set_action_secret(secret: String) -> Result<(), String> {
    store(ACTION_ACCOUNT, &secret)
}

#[tauri::command]
pub fn finance_clear_action_secret() -> Result<(), String> {
    clear(ACTION_ACCOUNT)
}

#[tauri::command]
pub fn finance_action_secret_configured() -> bool {
    load(ACTION_ACCOUNT).is_some()
}

#[tauri::command]
pub fn finance_set_read_secret(secret: String) -> Result<(), String> {
    store(READ_ACCOUNT, &secret)
}

#[tauri::command]
pub fn finance_clear_read_secret() -> Result<(), String> {
    clear(READ_ACCOUNT)
}

#[tauri::command]
pub fn finance_read_secret_configured() -> bool {
    load(READ_ACCOUNT).is_some()
}

/// Da para LER o M-Finance? O secret de leitura, ou o de acao no lugar dele.
pub fn can_read() -> bool {
    load(READ_ACCOUNT).is_some() || load(ACTION_ACCOUNT).is_some()
}

fn read_secret() -> Result<String, String> {
    load(READ_ACCOUNT)
        .or_else(|| load(ACTION_ACCOUNT))
        .ok_or_else(|| "Secret do M-Finance nao configurado. Cole-o em Settings.".to_owned())
}

// -------------------------------------------------------------------- leitura

#[derive(Serialize)]
struct QueryRequest<'a> {
    tool: &'a str,
    args: &'a serde_json::Value,
}

#[derive(Deserialize)]
struct QueryError {
    #[serde(default)]
    code: String,
    #[serde(default)]
    message: String,
}

#[derive(Deserialize)]
struct QueryResponse {
    ok: bool,
    #[serde(default, rename = "asOf")]
    as_of: String,
    #[serde(default)]
    data: serde_json::Value,
    #[serde(default)]
    error: Option<QueryError>,
}

/// O que uma ferramenta de leitura devolveu.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinanceReading {
    pub tool: String,
    pub as_of: String,
    pub data: serde_json::Value,
}

/// Chama UMA ferramenta do Intelligence Gateway. O nome ja passou pela
/// allowlist do `mos-core` (`FinanceTool`); o M-Finance confere de novo.
pub async fn query(
    tool: mos_core::FinanceTool,
    args: &serde_json::Value,
) -> Result<FinanceReading, String> {
    let secret = read_secret()?;
    let timeout = if tool == mos_core::FinanceTool::Analyze {
        ANALYZE_TIMEOUT
    } else {
        QUERY_TIMEOUT
    };
    let response = reqwest::Client::new()
        .post(QUERY_API_URL)
        .bearer_auth(secret)
        .timeout(timeout)
        .json(&QueryRequest {
            tool: tool.as_str(),
            args,
        })
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                "O M-Finance demorou demais para responder.".to_owned()
            } else {
                // Sem a URL e sem o header: o erro do reqwest pode trazer os dois.
                "Nao foi possivel falar com o M-Finance.".to_owned()
            }
        })?;
    let status = response.status();
    let body: QueryResponse = response.json().await.map_err(|_| {
        format!(
            "Resposta inesperada do M-Finance (HTTP {}).",
            status.as_u16()
        )
    })?;
    if body.ok {
        Ok(FinanceReading {
            tool: tool.as_str().to_owned(),
            as_of: body.as_of,
            data: body.data,
        })
    } else {
        let error = body.error.unwrap_or(QueryError {
            code: String::new(),
            message: String::new(),
        });
        Err(match (status.as_u16(), error.code.as_str()) {
            (401, _) => "O M-Finance recusou o secret. Confira em Settings.".to_owned(),
            (_, "ai_not_configured") => {
                "A IA financeira não está configurada no M-Finance; use as ferramentas determinísticas.".to_owned()
            }
            _ if !error.message.is_empty() => error.message,
            _ => format!("O M-Finance recusou a consulta (HTTP {}).", status.as_u16()),
        })
    }
}

/// O resumo da Home: o context pack, lido pelo Rust. O renderer recebe os
/// numeros — que sao dele — e nunca o secret.
#[tauri::command]
pub async fn finance_home_summary() -> Result<FinanceReading, String> {
    if !can_read() {
        return Err("not_configured".into());
    }
    query(mos_core::FinanceTool::ContextPack, &serde_json::json!({})).await
}

// --------------------------------------------------------------------- escrita

#[derive(Serialize)]
struct ActionRequest<'a> {
    #[serde(rename = "actionId")]
    action_id: &'a str,
    args: serde_json::Value,
    #[serde(rename = "idempotencyKey")]
    idempotency_key: &'a str,
}

#[derive(Deserialize, Default)]
struct ReceiptEntity {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    label: String,
}

#[derive(Deserialize, Default)]
struct Receipt {
    #[serde(default)]
    message: String,
    #[serde(default)]
    entities: Vec<ReceiptEntity>,
}

#[derive(Deserialize)]
struct ActionResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default, rename = "billId")]
    bill_id: Option<String>,
    #[serde(default)]
    receipt: Option<Receipt>,
    #[serde(default)]
    replayed: bool,
}

/// O que a Action API confirmou ter feito.
pub struct FinanceEffect {
    pub message: String,
    pub entities: Vec<(String, String, String)>,
}

/// Executa UMA acao ja confirmada. Erros de rede, autenticacao e recusa de
/// negocio viram a MESMA `Result<_, String>` — quem chama (`jarvis::run_action`)
/// converte para `CoreError` e o texto vai direto para o recibo da conversa.
///
/// `idempotency_key` identifica a proposta: um retry com a mesma chave devolve
/// o recibo gravado em vez de lancar duas vezes.
pub async fn execute_action(
    action_id: &str,
    args: serde_json::Value,
    idempotency_key: &str,
) -> Result<FinanceEffect, String> {
    let secret = load(ACTION_ACCOUNT).ok_or_else(|| {
        "Secret do M-Finance nao configurado. Cole-o em Settings antes de confirmar.".to_owned()
    })?;

    let response = reqwest::Client::new()
        .post(ACTION_API_URL)
        .bearer_auth(secret)
        .timeout(ACTION_TIMEOUT)
        .json(&ActionRequest {
            action_id,
            args,
            idempotency_key,
        })
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                // Pode ter gravado: a chave de idempotencia e o que torna seguro
                // confirmar de novo.
                "O M-Finance demorou demais. Confirme de novo: a mesma proposta não é lançada duas vezes.".to_owned()
            } else {
                "Nao foi possivel falar com o M-Finance.".to_owned()
            }
        })?;

    let status = response.status();
    let body: ActionResponse = response.json().await.map_err(|_| {
        format!(
            "Resposta inesperada do M-Finance (HTTP {}).",
            status.as_u16()
        )
    })?;

    if !body.ok {
        return Err(match status.as_u16() {
            401 => "O M-Finance recusou o secret de ação. Confira em Settings.".to_owned(),
            _ => body
                .error
                .unwrap_or_else(|| "O M-Finance recusou a acao.".to_owned()),
        });
    }

    let receipt = body.receipt.unwrap_or_default();
    let mut message = if receipt.message.is_empty() {
        body.bill_id
            .map(|id| format!("Conta criada no M-Finance (id {id})."))
            .unwrap_or_else(|| "Feito no M-Finance.".to_owned())
    } else {
        receipt.message
    };
    if body.replayed {
        message.push_str(" (já tinha sido feito — nada foi lançado de novo)");
    }
    Ok(FinanceEffect {
        message,
        entities: receipt
            .entities
            .into_iter()
            .filter(|entity| !entity.id.is_empty())
            .map(|entity| {
                (
                    format!("m-finance.{}", entity.kind),
                    entity.id,
                    entity.label,
                )
            })
            .collect(),
    })
}

/// A chave de idempotencia de uma proposta: a mensagem que a carregou e um
/// resumo estavel do texto cru. Mesma proposta, mesma chave — inclusive depois
/// de reiniciar o app, porque FNV-1a nao depende de semente.
pub fn idempotency_key(message_id: &str, raw: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in raw.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    let prefix: String = message_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(40)
        .collect();
    format!("mos:{prefix}:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::idempotency_key;

    #[test]
    fn a_chave_e_estavel_e_distingue_propostas() {
        let a = idempotency_key("0191f2c4-aaaa-7bbb-8ccc-1234567890ab", r#"{"action":"x"}"#);
        assert_eq!(
            a,
            idempotency_key("0191f2c4-aaaa-7bbb-8ccc-1234567890ab", r#"{"action":"x"}"#)
        );
        assert_ne!(
            a,
            idempotency_key("0191f2c4-aaaa-7bbb-8ccc-1234567890ab", r#"{"action":"y"}"#)
        );
        // O formato que a Action API aceita: [A-Za-z0-9:._-]{8,128}.
        assert!(a.len() <= 128);
        assert!(a
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ":._-".contains(c)));
    }
}
