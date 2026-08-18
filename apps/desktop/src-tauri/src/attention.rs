//! O agendador do Attention System, e os comandos que a interface chama.
//!
//! **Um timer, e nao um por Reminder** (`ATTENTION-SYSTEM.md` §7.3). O laco
//! pergunta ao banco qual e o proximo vencimento, dorme ate la, acorda,
//! processa TODOS os vencidos e volta a perguntar. Um timer por lembrete nao
//! escala e nao precisa existir.
//!
//! **O renderer nunca e dono do tempo.** Nada de `setTimeout`: um lembrete tem
//! de sobreviver a reload do front, janela fechada e navegacao. O precedente ja
//! estava no `PendingReminder` do monitor — a janela pode nascer depois do
//! evento, entao quem guarda o estado e o backend.
//!
//! **Sono e detectado por divergencia de relogio**, e nao por evento do
//! sistema. A stack nao expoe sleep/resume: Tauri nao oferece, e o
//! `WM_POWERBROADCAST` do Windows nao chega ate aqui por nenhuma dependencia
//! presente. Entao o laco compara quanto o relogio de parede andou com quanto
//! o monotonico andou, e trata a diferenca como sono ou ajuste de relogio.

use std::time::Duration as StdDuration;

use mos_core::{
    Channel, CoreError, LifecycleState, ReconcileReason, Reminder, ReminderId,
    ReminderSource, ReminderTarget, Transition, VisualLevel,
};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use time::OffsetDateTime;

use crate::AppState;

/// Teto de sono do laco.
///
/// Nao existe por causa de lembrete: existe para o laco voltar a olhar a
/// realidade de vez em quando. E acordando periodicamente que ele descobre que
/// a maquina dormiu ou que o relogio mudou — sem isso, um lembrete para daqui a
/// seis horas dormiria seis horas confiando num prazo calculado antes do sono.
const SANITY_CAP: StdDuration = StdDuration::from_secs(15 * 60);

/// Piso de sono, para o laco nao virar polling quando algo vence agora.
const MIN_SLEEP: StdDuration = StdDuration::from_millis(250);

/// Acima disto, a diferenca entre parede e monotonico e sono ou ajuste de
/// relogio, e nao a impressao normal do agendamento.
const DRIFT_TOLERANCE: StdDuration = StdDuration::from_secs(30);

/// O assunto da entrega. Compoe a `dedupe_key` junto do id do Reminder.
///
/// Dois assuntos e nao um: "venceu" e "foi perdido" sao avisos diferentes sobre
/// o mesmo Reminder, e um nao deve bloquear o outro.
const SUBJECT_DUE: &str = "reminder-due";
const SUBJECT_MISSED: &str = "reminder-missed";

/// O que a interface recebe quando algo precisa aparecer.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryEvent {
    pub reminder_id: String,
    pub title: String,
    pub body: String,
    /// `true` quando venceu enquanto o M/OS nao estava olhando.
    pub missed: bool,
    /// Segundos de atraso. Zero quando venceu agora.
    pub overdue_seconds: i64,
    pub level: String,
}

/// Roda para sempre. Um por processo.
pub async fn run<R: Runtime>(app: AppHandle<R>) {
    // A abertura reconcilia antes de qualquer coisa: o que venceu com o app
    // fechado precisa aparecer, e precisa aparecer como perdido.
    tick(&app).await;

    loop {
        let (state_now, monotonic_before) = {
            let state = app.state::<AppState>();
            (state.clock.now(), state.clock.monotonic())
        };

        let sleep_for = next_sleep(&app, state_now);
        tokio::time::sleep(sleep_for).await;

        // Quanto o mundo achou que passou, contra quanto passou de verdade.
        let (wall_after, monotonic_after) = {
            let state = app.state::<AppState>();
            (state.clock.now(), state.clock.monotonic())
        };
        let monotonic_span = monotonic_after.duration_since(monotonic_before);
        let wall_span = (wall_after - state_now).unsigned_abs();

        if wall_span.saturating_sub(monotonic_span) > DRIFT_TOLERANCE {
            // Nao ha o que "corrigir": a reconciliacao abaixo ja trata o
            // vencido pelo instante original. Isto so registra, para "por que
            // este lembrete chegou atrasado?" ter resposta.
            log_drift(wall_span, monotonic_span);
        }

        tick(&app).await;
    }
}

fn log_drift(wall: StdDuration, monotonic: StdDuration) {
    eprintln!(
        "[attention] salto de relogio: parede {}s, monotonico {}s — provavel sono ou ajuste",
        wall.as_secs(),
        monotonic.as_secs()
    );
}

/// Quanto dormir ate a proxima acordada.
fn next_sleep<R: Runtime>(app: &AppHandle<R>, now: OffsetDateTime) -> StdDuration {
    let state = app.state::<AppState>();
    let next = match state.attention.next_wake() {
        Ok(next) => next,
        // Banco ocupado ou em migration: tenta de novo no teto. Desistir
        // mataria o agendador ate o proximo restart.
        Err(_) => return SANITY_CAP,
    };

    match next {
        None => SANITY_CAP,
        Some(instant) if instant <= now => MIN_SLEEP,
        Some(instant) => {
            let span = (instant - now).unsigned_abs();
            span.min(SANITY_CAP).max(MIN_SLEEP)
        }
    }
}

/// Uma passada: reconcilia, entrega o que precisa, avisa a interface.
async fn tick<R: Runtime>(app: &AppHandle<R>) {
    let changed = {
        let state = app.state::<AppState>();
        match state.attention.reconcile() {
            Ok(changed) => changed,
            Err(error) => {
                eprintln!("[attention] reconciliacao falhou: {}", error.message);
                return;
            }
        }
    };

    for (reminder, reason) in changed {
        deliver(app, &reminder, reason);
    }

    notify_badge(app);
}

/// Entrega in-app. O canal Windows chega em P1.
fn deliver<R: Runtime>(app: &AppHandle<R>, reminder: &Reminder, reason: ReconcileReason) {
    let missed = reason == ReconcileReason::MissedWhileAway;
    let subject = if missed { SUBJECT_MISSED } else { SUBJECT_DUE };
    let state = app.state::<AppState>();

    let queued = match state
        .attention
        .queue_delivery(reminder.id, Channel::InApp, subject, VisualLevel::Normal)
    {
        Ok(Some(queued)) => queued,
        // `None` e o dedupe funcionando: ja existe entrega viva com esta
        // chave, e criar outra e exatamente o que produz fadiga.
        Ok(None) => return,
        Err(error) => {
            eprintln!("[attention] nao consegui enfileirar: {}", error.message);
            return;
        }
    };

    let (title, body) = reminder
        .policy
        .privacy
        .redact(&reminder.title, &reminder.body);

    let overdue_seconds = reminder
        .overdue_by(state.clock.now())
        .map(|span| span.whole_seconds())
        .unwrap_or(0);

    let event = DeliveryEvent {
        reminder_id: reminder.id.to_string(),
        title,
        body,
        missed,
        overdue_seconds,
        level: VisualLevel::Normal.as_str().to_owned(),
    };

    match app.emit("attention-delivered", &event) {
        Ok(()) => {
            let _ = state.attention.mark_delivered(&queued);
        }
        Err(error) => {
            // O Reminder continua vivo e visivel no Attention Center. Falha de
            // canal nunca resolve uma intencao — e a §27 inteira.
            let _ = state
                .attention
                .mark_failed(&queued, &format!("emit falhou: {error}"));
        }
    }
}

/// O badge conta itens que esperam acao, e nao notificacoes nao lidas.
fn notify_badge<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    if let Ok(count) = state.attention.needs_attention_count() {
        let _ = app.emit("attention-count", count);
    }
}

// ------------------------------------------------------------------ comandos

/// Acorda o agendador agora.
///
/// Chamado depois de criar ou adiar: sem isto, um lembrete para daqui a dois
/// minutos esperaria o laco acordar pelo teto de quinze.
fn poke<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tick(&handle).await;
    });
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReminderInput {
    pub title: String,
    #[serde(default)]
    pub body: String,
    /// Instante em RFC 3339, ja resolvido pelo renderer.
    ///
    /// O calculo de "amanha de manha" acontece na interface de proposito: e o
    /// unico lado que conhece o fuso de quem clicou. O backend guarda UTC e nao
    /// adivinha. Mesmo padrao do `muted_until` do monitor.
    pub at: String,
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub target_id: Option<String>,
}

fn parse_instant(value: &str) -> Result<OffsetDateTime, CoreError> {
    OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339).map_err(|_| {
        CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            "Instante invalido.",
            false,
        )
    })
}

fn parse_target(
    kind: Option<String>,
    id: Option<String>,
) -> Result<Option<ReminderTarget>, CoreError> {
    match (kind, id) {
        (Some(kind), Some(id)) => ReminderTarget::from_columns(&kind, &id).map(Some),
        (None, None) => Ok(None),
        _ => Err(CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            "Alvo incompleto: tipo e id andam juntos.",
            false,
        )),
    }
}

#[tauri::command]
pub fn attention_create<R: Runtime>(
    app: AppHandle<R>,
    input: CreateReminderInput,
) -> Result<Reminder, CoreError> {
    let instant = parse_instant(&input.at)?;
    let target = parse_target(input.target_type, input.target_id)?;

    let created = app.state::<AppState>().attention.create_at(
        &input.title,
        &input.body,
        instant,
        target,
        ReminderSource::User,
    )?;

    poke(&app);
    Ok(created)
}

#[tauri::command]
pub fn attention_list<R: Runtime>(app: AppHandle<R>) -> Result<Vec<Reminder>, CoreError> {
    app.state::<AppState>().attention.open()
}

#[tauri::command]
pub fn attention_count<R: Runtime>(app: AppHandle<R>) -> Result<usize, CoreError> {
    app.state::<AppState>().attention.needs_attention_count()
}

#[tauri::command]
pub fn attention_snooze<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    until: String,
) -> Result<Reminder, CoreError> {
    let id = ReminderId::parse(&id)?;
    let until = parse_instant(&until)?;
    let updated = app
        .state::<AppState>()
        .attention
        .transition(id, Transition::Snooze { until })?;
    poke(&app);
    Ok(updated)
}

#[tauri::command]
pub fn attention_complete<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> Result<Reminder, CoreError> {
    let updated = app
        .state::<AppState>()
        .attention
        .transition(ReminderId::parse(&id)?, Transition::Complete)?;
    notify_badge(&app);
    Ok(updated)
}

#[tauri::command]
pub fn attention_acknowledge<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> Result<Reminder, CoreError> {
    let updated = app
        .state::<AppState>()
        .attention
        .transition(ReminderId::parse(&id)?, Transition::Acknowledge)?;
    notify_badge(&app);
    Ok(updated)
}

/// Cancelar e desistir da intencao. Continua consultavel: a ADR-035 diz que
/// desfazer arquiva e nunca apaga, e o mesmo vale aqui.
#[tauri::command]
pub fn attention_cancel<R: Runtime>(app: AppHandle<R>, id: String) -> Result<Reminder, CoreError> {
    let updated = app
        .state::<AppState>()
        .attention
        .transition(ReminderId::parse(&id)?, Transition::Cancel)?;
    notify_badge(&app);
    Ok(updated)
}

#[tauri::command]
pub fn attention_archive<R: Runtime>(app: AppHandle<R>, id: String) -> Result<Reminder, CoreError> {
    let updated = app
        .state::<AppState>()
        .attention
        .set_lifecycle(ReminderId::parse(&id)?, LifecycleState::Archived)?;
    notify_badge(&app);
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_core::ContentPrivacy as _Privacy;

    /// O alvo e um par, e meio par nao resolve nada. Sem isto, um bug na
    /// interface gravaria `target_type` sem id e o vinculo apontaria para lugar
    /// nenhum — que e justamente o que o CHECK do banco tambem recusa.
    #[test]
    fn a_half_target_is_refused() {
        assert!(parse_target(Some("task".into()), None).is_err());
        assert!(parse_target(None, Some("abc".into())).is_err());
        assert!(parse_target(None, None).unwrap().is_none());
    }

    #[test]
    fn an_unknown_target_kind_is_refused() {
        let id = mos_core::TaskId::new().to_string();
        assert!(parse_target(Some("planeta".into()), Some(id)).is_err());
    }

    #[test]
    fn an_instant_must_be_rfc3339() {
        assert!(parse_instant("2026-08-18T15:00:00Z").is_ok());
        assert!(parse_instant("18/08/2026 15:00").is_err());
        assert!(parse_instant("").is_err());
    }

    /// Os dois assuntos existem para nao se bloquearem: "venceu" e "foi
    /// perdido" sao avisos diferentes sobre o mesmo Reminder.
    #[test]
    fn due_and_missed_do_not_share_a_dedupe_key() {
        let id = mos_core::ReminderId::new();
        assert_ne!(
            mos_core::NewNotification::dedupe_key(SUBJECT_DUE, id),
            mos_core::NewNotification::dedupe_key(SUBJECT_MISSED, id)
        );
    }

    /// O payload do evento respeita a privacidade do Reminder. Este e o ponto
    /// onde o conteudo sai do processo, entao e aqui que a politica precisa
    /// valer — e nao so na tela.
    #[test]
    fn the_event_payload_respects_privacy() {
        let (title, body) = _Privacy::Hidden.redact("Pagar boleto", "R$ 1.234,56");
        assert_eq!(title, "M/OS");
        assert!(!body.contains("1.234"));
    }
}
