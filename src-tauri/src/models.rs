//! Modelos de dados e entradas validadas para clientes e projetos.
//!
//! Structs de leitura implementam `sqlx::FromRow` (mapeamento por nome de
//! coluna, snake_case) e serializam em camelCase para o frontend. As entradas
//! (`*Input`) sao validadas no backend (secao 19/20) antes de qualquer escrita.

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Normaliza um campo de texto opcional: `None` quando vazio/em branco.
fn clean_opt(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Status validos de projeto (secao 8).
pub const PROJECT_STATUSES: [&str; 4] = ["active", "paused", "completed", "archived"];

/// Tipos de atividade validos (secao 8).
pub const ACTIVITY_TYPES: [&str; 6] =
    ["drawing", "detailing", "revision", "meeting", "study", "other"];

// ---------------------------------------------------------------------------
// Clients
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    pub id: String,
    pub name: String,
    pub company_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInput {
    pub name: String,
    pub company_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub notes: Option<String>,
}

/// Entrada de cliente ja validada e normalizada.
pub struct ValidClient {
    pub name: String,
    pub company_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub notes: Option<String>,
}

impl ClientInput {
    pub fn validate(self) -> Result<ValidClient, AppError> {
        let name = self.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Validation("o nome do cliente e obrigatorio".into()));
        }
        let email = clean_opt(self.email);
        if let Some(ref e) = email {
            // Validacao minima: um "@" com texto de ambos os lados.
            let ok = e
                .split_once('@')
                .is_some_and(|(a, b)| !a.is_empty() && b.contains('.') && !b.starts_with('.'));
            if !ok {
                return Err(AppError::Validation("e-mail invalido".into()));
            }
        }
        Ok(ValidClient {
            name,
            company_name: clean_opt(self.company_name),
            email,
            phone: clean_opt(self.phone),
            notes: clean_opt(self.notes),
        })
    }
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub client_id: Option<String>,
    pub name: String,
    pub code: Option<String>,
    pub description: Option<String>,
    pub hourly_rate_cents: i64,
    pub budget_minutes: i64,
    pub status: String,
    pub color: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInput {
    pub client_id: Option<String>,
    pub name: String,
    pub code: Option<String>,
    pub description: Option<String>,
    pub hourly_rate_cents: i64,
    pub budget_minutes: Option<i64>,
    pub color: Option<String>,
}

/// Entrada de projeto ja validada e normalizada.
pub struct ValidProject {
    pub client_id: Option<String>,
    pub name: String,
    pub code: Option<String>,
    pub description: Option<String>,
    pub hourly_rate_cents: i64,
    pub budget_minutes: i64,
    pub color: Option<String>,
}

impl ProjectInput {
    pub fn validate(self) -> Result<ValidProject, AppError> {
        let name = self.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Validation("o nome do projeto e obrigatorio".into()));
        }
        if self.hourly_rate_cents < 0 {
            return Err(AppError::Validation(
                "o valor/hora nao pode ser negativo".into(),
            ));
        }
        let budget_minutes = self.budget_minutes.unwrap_or(0).max(0);
        Ok(ValidProject {
            client_id: clean_opt(self.client_id),
            name,
            code: clean_opt(self.code),
            description: clean_opt(self.description),
            hourly_rate_cents: self.hourly_rate_cents,
            budget_minutes,
            color: clean_opt(self.color),
        })
    }
}

/// Valida um status de projeto recebido do frontend.
pub fn validate_status(status: &str) -> Result<(), AppError> {
    if PROJECT_STATUSES.contains(&status) {
        Ok(())
    } else {
        Err(AppError::Validation(format!("status invalido: {status}")))
    }
}

/// Valida um tipo de atividade.
pub fn validate_activity_type(activity_type: &str) -> Result<(), AppError> {
    if ACTIVITY_TYPES.contains(&activity_type) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "tipo de atividade invalido: {activity_type}"
        )))
    }
}

// ---------------------------------------------------------------------------
// Cronometro (active_timer) e sessoes (time_entries)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTimer {
    pub id: String,
    pub project_id: String,
    pub started_at: String,
    pub last_resumed_at: String,
    pub accumulated_seconds: i64,
    pub idle_seconds: i64,
    pub status: String,
    pub description: Option<String>,
    pub activity_type: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Total trabalhado (segundos) por projeto — para acompanhamento de metas.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTotal {
    pub project_id: String,
    pub seconds: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntry {
    pub id: String,
    pub project_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_seconds: i64,
    pub idle_seconds: i64,
    pub description: Option<String>,
    pub activity_type: String,
    pub billable: bool,
    pub hourly_rate_snapshot_cents: i64,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTimerInput {
    pub project_id: String,
    pub activity_type: String,
    pub description: Option<String>,
}

/// Entrada de inicio de cronometro ja validada.
pub struct ValidStart {
    pub project_id: String,
    pub activity_type: String,
    pub description: Option<String>,
}

impl StartTimerInput {
    pub fn validate(self) -> Result<ValidStart, AppError> {
        let project_id = self.project_id.trim().to_string();
        if project_id.is_empty() {
            return Err(AppError::Validation("selecione um projeto".into()));
        }
        validate_activity_type(&self.activity_type)?;
        Ok(ValidStart {
            project_id,
            activity_type: self.activity_type,
            description: clean_opt(self.description),
        })
    }
}

// ---------------------------------------------------------------------------
// Edicao/criacao manual de sessoes (secao 13)
// ---------------------------------------------------------------------------

/// Calcula a duracao (segundos) entre dois timestamps ISO. Aceita sessoes que
/// atravessam a meia-noite (timestamps absolutos) e rejeita horarios invalidos
/// ou fim anterior/igual ao inicio.
fn timing(started_at: &str, ended_at: &str) -> Result<(String, String, i64), AppError> {
    let parse = |s: &str| {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.timestamp())
            .map_err(|_| AppError::Validation(format!("horario invalido: {s}")))
    };
    let start = parse(started_at)?;
    let end = parse(ended_at)?;
    let duration = end - start;
    if duration <= 0 {
        return Err(AppError::Validation(
            "o horario final deve ser posterior ao inicial".into(),
        ));
    }
    Ok((started_at.to_string(), ended_at.to_string(), duration))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualEntryInput {
    pub project_id: String,
    pub started_at: String,
    pub ended_at: String,
    pub description: Option<String>,
    pub activity_type: String,
    pub billable: bool,
    pub idle_seconds: i64,
    /// "manual" (padrao) ou "reconstructed" (linha do tempo — secao 14).
    pub source: Option<String>,
}

pub struct ValidManualEntry {
    pub project_id: String,
    pub started_at: String,
    pub ended_at: String,
    pub duration_seconds: i64,
    pub idle_seconds: i64,
    pub description: Option<String>,
    pub activity_type: String,
    pub billable: bool,
    pub source: String,
}

impl ManualEntryInput {
    pub fn validate(self) -> Result<ValidManualEntry, AppError> {
        let project_id = self.project_id.trim().to_string();
        if project_id.is_empty() {
            return Err(AppError::Validation("selecione um projeto".into()));
        }
        validate_activity_type(&self.activity_type)?;
        let source = match self.source.as_deref() {
            None | Some("manual") => "manual".to_string(),
            Some("reconstructed") => "reconstructed".to_string(),
            Some(other) => {
                return Err(AppError::Validation(format!("origem invalida: {other}")))
            }
        };
        let (started_at, ended_at, duration_seconds) =
            timing(&self.started_at, &self.ended_at)?;
        let idle_seconds = self.idle_seconds.clamp(0, duration_seconds);
        Ok(ValidManualEntry {
            project_id,
            started_at,
            ended_at,
            duration_seconds,
            idle_seconds,
            description: clean_opt(self.description),
            activity_type: self.activity_type,
            billable: self.billable,
            source,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryUpdateInput {
    pub started_at: String,
    pub ended_at: String,
    pub description: Option<String>,
    pub activity_type: String,
    pub billable: bool,
    pub idle_seconds: i64,
}

pub struct ValidEntryUpdate {
    pub started_at: String,
    pub ended_at: String,
    pub duration_seconds: i64,
    pub idle_seconds: i64,
    pub description: Option<String>,
    pub activity_type: String,
    pub billable: bool,
}

impl EntryUpdateInput {
    pub fn validate(self) -> Result<ValidEntryUpdate, AppError> {
        validate_activity_type(&self.activity_type)?;
        let (started_at, ended_at, duration_seconds) =
            timing(&self.started_at, &self.ended_at)?;
        let idle_seconds = self.idle_seconds.clamp(0, duration_seconds);
        Ok(ValidEntryUpdate {
            started_at,
            ended_at,
            duration_seconds,
            idle_seconds,
            description: clean_opt(self.description),
            activity_type: self.activity_type,
            billable: self.billable,
        })
    }
}

// ---------------------------------------------------------------------------
// Configuracoes e programas monitorados
// ---------------------------------------------------------------------------

pub const ROUNDING_MODES: [&str; 3] = ["nearest", "up", "down"];

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub idle_detection_enabled: bool,
    pub idle_threshold_minutes: i64,
    pub process_monitoring_enabled: bool,
    pub process_check_interval_seconds: i64,
    pub remind_when_monitored_app_opens: bool,
    pub remind_when_monitored_app_closes: bool,
    pub rounding_enabled: bool,
    pub rounding_interval_minutes: i64,
    pub rounding_mode: String,
    pub start_with_windows: bool,
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
    pub currency: String,
    pub locale: String,
    pub issuer_name: String,
    pub issuer_document: String,
    pub issuer_contact: String,
}

impl Settings {
    /// Valida e normaliza limites minimos antes de persistir.
    pub fn validated(mut self) -> Result<Settings, AppError> {
        if !ROUNDING_MODES.contains(&self.rounding_mode.as_str()) {
            return Err(AppError::Validation("modo de arredondamento invalido".into()));
        }
        if self.process_check_interval_seconds < 1 {
            return Err(AppError::Validation(
                "o intervalo de verificacao deve ser >= 1s".into(),
            ));
        }
        if self.idle_threshold_minutes < 1 {
            return Err(AppError::Validation(
                "o limite de inatividade deve ser >= 1 min".into(),
            ));
        }
        if self.rounding_interval_minutes < 1 {
            self.rounding_interval_minutes = 1;
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub id: String,
    pub event_type: String,
    pub process_name: Option<String>,
    pub detected_at: String,
    pub metadata_json: Option<String>,
    pub processed: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MonitoredApp {
    pub id: String,
    pub display_name: String,
    pub process_name: String,
    pub enabled: bool,
    pub remind_on_open: bool,
    pub remind_on_close: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitoredAppInput {
    pub display_name: String,
    pub process_name: String,
    pub enabled: bool,
    pub remind_on_open: bool,
    pub remind_on_close: bool,
}

pub struct ValidMonitoredApp {
    pub display_name: String,
    pub process_name: String,
    pub enabled: bool,
    pub remind_on_open: bool,
    pub remind_on_close: bool,
}

impl MonitoredAppInput {
    pub fn validate(self) -> Result<ValidMonitoredApp, AppError> {
        let display_name = self.display_name.trim().to_string();
        // Nomes de processo no Windows sao comparados sem diferenciar caixa.
        let process_name = self.process_name.trim().to_lowercase();
        if display_name.is_empty() {
            return Err(AppError::Validation("informe o nome de exibicao".into()));
        }
        if process_name.is_empty() {
            return Err(AppError::Validation(
                "informe o nome do executavel (ex.: acad.exe)".into(),
            ));
        }
        Ok(ValidMonitoredApp {
            display_name,
            process_name,
            enabled: self.enabled,
            remind_on_open: self.remind_on_open,
            remind_on_close: self.remind_on_close,
        })
    }
}
