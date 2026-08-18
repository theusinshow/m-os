//! Credencial e cliente HTTP para a Action API do M-Finance.
//!
//! Mesmo padrao de `mos-hermes/src/auth.rs`: o segredo vive so no Windows
//! Credential Manager, nunca na memoria do renderer nem em disco em texto
//! claro. Diferente do Hermes, aqui nao ha sessao — cada chamada manda o
//! segredo no header `Authorization`, como o proprio M-Finance ja faz para o
//! cron do Vercel (`app/api/cron/reminders`).

use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "m-os";
const ACCOUNT: &str = "finance-action-secret";
/// O host do M-Finance, escrito uma vez so. O gate de permissao em
/// `hermes.rs` acha o App do Registry por ele, e a URL abaixo e montada a
/// partir dele — se os dois pudessem divergir, o M/OS acabaria pedindo
/// permissao para um destino e escrevendo em outro.
macro_rules! finance_host {
    () => {
        "m-finance-silk.vercel.app"
    };
}

pub const ACTION_HOST: &str = finance_host!();
const ACTION_API_URL: &str = concat!("https://", finance_host!(), "/api/mos/actions");

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE, ACCOUNT)
        .map_err(|error| format!("Credential Manager indisponivel: {error}"))
}

#[tauri::command]
pub fn finance_set_action_secret(secret: String) -> Result<(), String> {
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return Err("O secret nao pode ficar vazio.".into());
    }
    entry()?
        .set_password(trimmed)
        .map_err(|error| format!("Nao foi possivel guardar: {error}"))
}

#[tauri::command]
pub fn finance_clear_action_secret() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("Nao foi possivel remover: {error}")),
    }
}

#[tauri::command]
pub fn finance_action_secret_configured() -> bool {
    entry()
        .and_then(|e| e.get_password().map_err(|error| error.to_string()))
        .is_ok()
}

#[derive(Serialize)]
struct ActionRequest {
    #[serde(rename = "actionId")]
    action_id: &'static str,
    args: serde_json::Value,
}

#[derive(Deserialize)]
struct ActionResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default, rename = "billId")]
    bill_id: Option<String>,
}

/// Chama a Action API do M-Finance. Erros de rede, autenticacao e recusa de
/// negocio viram a MESMA `Result<_, String>` — quem chama (jarvis::run_action)
/// converte para `CoreError` e o texto vai direto para o recibo da conversa.
pub async fn execute_create_bill(
    amount_cents: i64,
    description: &str,
    due_day: Option<u8>,
    is_recurring: bool,
) -> Result<String, String> {
    let secret = entry()?.get_password().map_err(|_| {
        "Secret do M-Finance nao configurado. Cole-o em Settings antes de confirmar.".to_owned()
    })?;

    let args = serde_json::json!({
        "amountCents": amount_cents,
        "description": description,
        "dueDay": due_day,
        "isRecurring": is_recurring,
    });

    let response = reqwest::Client::new()
        .post(ACTION_API_URL)
        .bearer_auth(secret)
        .json(&ActionRequest {
            action_id: "m-finance.create_bill",
            args,
        })
        .send()
        .await
        .map_err(|error| format!("Nao foi possivel falar com o M-Finance: {error}"))?;

    let body: ActionResponse = response
        .json()
        .await
        .map_err(|error| format!("Resposta inesperada do M-Finance: {error}"))?;

    if body.ok {
        Ok(body
            .bill_id
            .map(|id| format!("Conta criada no M-Finance (id {id})."))
            .unwrap_or_else(|| "Conta criada no M-Finance.".to_owned()))
    } else {
        Err(body
            .error
            .unwrap_or_else(|| "O M-Finance recusou a acao.".to_owned()))
    }
}
