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
    Channel, CoreError, DueDelivery, DueReason, LifecycleState, Reminder, ReminderId,
    ReminderSource, ReminderTarget, VisualLevel,
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
/// O atalho global do Quick Reminder.
///
/// `Ctrl+Shift+R` de "reminder". Nao colide com nada que o M/OS ja registra —
/// a Captura rapida usa o que estiver no `settings.json`, a voz o dela, e a
/// faixa e `Ctrl+Shift+U`. Fixo e nao configuravel de proposito: um atalho a
/// mais na tela de Settings e uma decisao a mais para quem so queria um
/// lembrete, e o §13 do pedido pede captura em segundos.
///
/// Se ele falhar ao registrar — outro programa ja o tomou —, o caminho pela
/// interface continua inteiro. Um atalho e o terceiro caminho, nunca o unico.
pub const QUICK_SHORTCUT: &str = "CommandOrControl+Shift+R";

const SUBJECT_DUE: &str = "reminder-due";
const SUBJECT_MISSED: &str = "reminder-missed";
/// A insistencia. Assunto proprio e com o degrau colado, para o dedupe da
/// entrega anterior nao calar a proxima.
const SUBJECT_RETRY: &str = "reminder-retry";

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
    /// Por que ele esta aparecendo: venceu, se perdeu, ou esta insistindo.
    pub reason: String,
    pub persistent: bool,
    /// `standard` ou `follow_up`. Decide a pergunta que o toast faz.
    pub kind: String,
    pub waiting_for: String,
    /// Para onde o clique leva. `None` e lembrete solto, e ai o clique abre o
    /// proprio Attention Center.
    pub target: Option<DeepLink>,
}

/// O par (tipo, id) que a interface usa para abrir a coisa certa.
///
/// Existe porque abrir um Attention Center generico depois de clicar numa
/// notificacao sobre uma Task especifica e fazer a pessoa procurar de novo o que
/// o sistema ja sabia — e o §44 do pedido recusa isso por escrito.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepLink {
    pub kind: String,
    pub id: String,
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

        // Dorme ate o proximo vencimento OU ate alguem tocar o sino.
        //
        // O sino e o que faz "me lembra daqui a um minuto" tocar em um minuto.
        // Sem ele, o laco dormia com o prazo calculado antes de o lembrete
        // existir, e o toque saia no proximo despertar — ate quinze minutos
        // depois. Ver `AppState::attention_wake`.
        let sino = {
            let state = app.state::<AppState>();
            state.attention_wake.clone()
        };
        tokio::select! {
            _ = tokio::time::sleep(sleep_for) => {}
            _ = sino.notified() => {}
        }

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
    // UMA chamada, e ela e a unica fonte de regras: o que venceu, o que se
    // perdeu, o que precisa insistir e o que a pilha disparou saem todos de
    // `sweep`. Este arquivo so ENTREGA. E a fronteira do §56 do pedido, e ela e
    // o que impede a regra de escorregar para dentro do Tauri.
    let pedidos = {
        let state = app.state::<AppState>();
        match state.attention.sweep() {
            Ok(pedidos) => pedidos,
            Err(error) => {
                eprintln!("[attention] varredura falhou: {}", error.message);
                return;
            }
        }
    };

    for pedido in pedidos {
        deliver(app, &pedido);
    }

    notify_badge(app);
}

/// Entrega. Dois canais, e o de dentro nunca depende do de fora.
///
/// **In-app primeiro, sempre.** O toast do M/OS é o canal que não pode falhar:
/// ele não depende de permissão do Windows, de Focus Assist nem de o Centro de
/// Notificações estar de bom humor. O canal do sistema operacional é o que
/// alcança a pessoa quando a janela está fechada — e é o que pode falhar.
///
/// **Falha de canal nunca resolve a intenção** (`ATTENTION-SYSTEM.md` §27). Se
/// os dois canais falharem, o Reminder continua exatamente onde estava: vencido,
/// contando o atraso e visível no Attention Center. É essa a promessa inteira.
fn deliver<R: Runtime>(app: &AppHandle<R>, pedido: &DueDelivery) {
    let reminder = &pedido.reminder;
    let subject = match pedido.reason {
        DueReason::DueNow => SUBJECT_DUE,
        DueReason::MissedWhileAway => SUBJECT_MISSED,
        // A insistência precisa do degrau na chave: sem ele, o dedupe da
        // primeira entrega bloquearia a segunda para sempre — e um lembrete
        // persistente que não consegue insistir é um lembrete comum com um
        // rótulo mentiroso.
        DueReason::Retry => &format!("{SUBJECT_RETRY}-{}", reminder.escalation_step),
    };

    let state = app.state::<AppState>();
    let nivel = visual_level(reminder, pedido.reason);
    let (title, body) = reminder
        .policy
        .privacy
        .redact(&reminder.title, &reminder.body);

    let overdue_seconds = reminder
        .overdue_by(state.clock.now())
        .map(|span| span.whole_seconds())
        .unwrap_or(0);

    let evento = DeliveryEvent {
        reminder_id: reminder.id.to_string(),
        title: title.clone(),
        body: body.clone(),
        missed: pedido.reason == DueReason::MissedWhileAway,
        overdue_seconds,
        level: nivel.as_str().to_owned(),
        reason: pedido.reason.as_str().to_owned(),
        persistent: reminder.persistent,
        kind: reminder.kind.as_str().to_owned(),
        waiting_for: reminder.waiting_for.clone(),
        target: reminder.target.map(|alvo| {
            let (tipo, id) = alvo.as_columns();
            DeepLink {
                kind: tipo.to_owned(),
                id,
            }
        }),
    };

    deliver_in_app(app, reminder, subject, nivel, &evento);
    deliver_os(app, reminder, subject, &title, &body);
    notify_badge(app);
}

/// Quanto a entrega se impõe.
///
/// Determinístico, e derivado do que já se sabe: prioridade e motivo. Nada aqui
/// consulta modelo nem histórico de uso — a pessoa consegue prever o resultado,
/// e é isso que permite confiar (§30 do pedido).
fn visual_level(reminder: &Reminder, reason: DueReason) -> VisualLevel {
    match (reminder.priority, reason) {
        (mos_core::Priority::Urgent, _) => VisualLevel::Critical,
        (mos_core::Priority::High, _) => VisualLevel::Important,
        (_, DueReason::MissedWhileAway | DueReason::Retry) => VisualLevel::Important,
        (mos_core::Priority::Low, _) => VisualLevel::Quiet,
        _ => VisualLevel::Normal,
    }
}

fn deliver_in_app<R: Runtime>(
    app: &AppHandle<R>,
    reminder: &Reminder,
    subject: &str,
    nivel: VisualLevel,
    evento: &DeliveryEvent,
) {
    let state = app.state::<AppState>();
    let queued = match state
        .attention
        .queue_delivery(reminder.id, Channel::InApp, subject, nivel)
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

    match app.emit("attention-delivered", evento) {
        Ok(()) => {
            let _ = state.attention.record_delivered(&queued);
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

/// O canal do sistema operacional.
///
/// # O que o Windows entrega, e o que ele não entrega
///
/// O `tauri-plugin-notification` mostra título e corpo, e o clique **ativa o
/// app**. Ele não oferece botões de ação na notificação: "Concluir" e "Adiar"
/// direto do toast exigiriam uma AUMID registrada e o caminho WinRT, que a
/// ADR-043 e a §11.3 do documento já examinaram e deixaram fora.
///
/// A consequência é honesta e está desenhada: o clique abre o M/OS na coisa
/// certa — a Task, o Project, ou o Attention Center —, e a ação acontece lá, com
/// um clique. Prometer botões que a plataforma não dá seria prometer o que não
/// se cumpre.
fn deliver_os<R: Runtime>(
    app: &AppHandle<R>,
    reminder: &Reminder,
    subject: &str,
    title: &str,
    body: &str,
) {
    use tauri_plugin_notification::NotificationExt;

    let state = app.state::<AppState>();

    // A pessoa pode ter desligado o canal. Desligado ele fica: o lembrete
    // continua no Attention Center, e é lá que ela escolheu vê-lo.
    match state.attention.settings() {
        Ok(ajustes) if !ajustes.os_channel_enabled => return,
        Ok(_) => {}
        Err(error) => {
            eprintln!("[attention] ajustes ilegiveis: {}", error.message);
            return;
        }
    }

    // Chave própria por canal: o toast do Windows e o toast de dentro do app são
    // duas entregas diferentes da mesma cobrança, e uma não pode bloquear a
    // outra pelo dedupe.
    let chave = format!("os-{subject}");
    let queued = match state.attention.queue_delivery(
        reminder.id,
        Channel::Windows,
        &chave,
        VisualLevel::Normal,
    ) {
        Ok(Some(queued)) => queued,
        Ok(None) => return,
        Err(error) => {
            eprintln!(
                "[attention] nao consegui enfileirar (SO): {}",
                error.message
            );
            return;
        }
    };

    let corpo = if body.trim().is_empty() {
        quando_em_palavras(reminder, state.clock.now())
    } else {
        body.to_owned()
    };

    match app
        .notification()
        .builder()
        .title(title)
        .body(&corpo)
        .show()
    {
        Ok(()) => {
            // `record_channel_delivery` e nao `record_delivered`: o toast do
            // Windows e o toast de dentro do app sao a MESMA cobranca saindo
            // por dois canais, e contar os dois faria `delivered_count` dobrar
            // — o que transformava "avisado uma vez" em "ignorado" na tela.
            let _ = state.attention.record_channel_delivery(&queued);
        }
        Err(error) => {
            // Permissão negada, Centro de Notificações indisponível, sessão sem
            // shell. Nada disso resolve o lembrete: ele continua na tela.
            let _ = state
                .attention
                .mark_failed(&queued, &format!("notificacao do sistema falhou: {error}"));
        }
    }
}

/// "atrasado 2 h", "agora". Vai no corpo do toast quando não há nota.
///
/// Um toast que diz só o título não conta o que a pessoa precisa saber para
/// decidir se levanta agora ou daqui a pouco.
fn quando_em_palavras(reminder: &Reminder, now: OffsetDateTime) -> String {
    match reminder.overdue_by(now) {
        None => "Agora".to_owned(),
        Some(atraso) if atraso.whole_minutes() < 1 => "Agora".to_owned(),
        Some(atraso) if atraso.whole_hours() < 1 => {
            format!("Atrasado {} min", atraso.whole_minutes())
        }
        Some(atraso) if atraso.whole_days() < 1 => {
            format!("Atrasado {} h", atraso.whole_hours())
        }
        Some(atraso) => format!("Atrasado {} d", atraso.whole_days()),
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
pub fn poke<R: Runtime>(app: &AppHandle<R>) {
    // Toca o sino ANTES do tick avulso. As duas coisas são necessárias e são
    // diferentes: o tick entrega o que já venceu agora; o sino faz o laço
    // recalcular quando ele precisa acordar da próxima vez.
    //
    // Só o tick era o defeito: o laço continuava dormindo com o prazo de antes,
    // e um lembrete para daqui a um minuto esperava o teto de quinze.
    if let Some(state) = app.try_state::<AppState>() {
        state.attention_wake.notify_one();
    }
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

/// O que se pode mudar num lembrete. Ausente significa "nao mexi".
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditReminderInput {
    pub id: String,
    pub title: Option<String>,
    pub body: Option<String>,
    /// RFC 3339. Reagendar, e nao adiar — `attention_snooze` continua sendo o
    /// adiar, com a contagem de fadiga que ele carrega.
    pub at: Option<String>,
    pub priority: Option<String>,
}

/// Editar um lembrete existente.
///
/// Nasceu junto com a mesma operacao no `mos-web`: as duas telas editam o mesmo
/// dado, e uma edicao que so existisse numa delas seria um sistema paralelo.
#[tauri::command]
pub fn attention_edit<R: Runtime>(
    app: AppHandle<R>,
    input: EditReminderInput,
) -> Result<Reminder, CoreError> {
    let id = mos_core::ReminderId::parse(&input.id)?;
    let instant = match input.at.as_deref() {
        Some(texto) => Some(parse_instant(texto)?),
        None => None,
    };
    let priority = match input.priority.as_deref() {
        Some(texto) => Some(mos_core::Priority::parse(texto)?),
        None => None,
    };
    let editado = app.state::<AppState>().attention.update(
        id,
        mos_core::EditReminder {
            title: input.title,
            body: input.body,
            instant,
            priority,
        },
    )?;
    // O agendador precisa saber: a hora pode ter mudado, e o proximo despertar
    // dele foi calculado com a antiga.
    poke(&app);
    Ok(editado)
}

#[tauri::command]
pub fn attention_list<R: Runtime>(app: AppHandle<R>) -> Result<Vec<Reminder>, CoreError> {
    crate::services(&app)?.attention.open()
}

/// O historico: o que ja foi resolvido, do mais recente para tras.
#[tauri::command]
pub fn attention_resolved<R: Runtime>(
    app: AppHandle<R>,
    limit: Option<usize>,
) -> Result<Vec<Reminder>, CoreError> {
    crate::services(&app)?
        .attention
        .resolved(limit.unwrap_or(50))
}

#[tauri::command]
pub fn attention_count<R: Runtime>(app: AppHandle<R>) -> Result<usize, CoreError> {
    // A PRIMEIRA chamada que a Home faz ao montar, e por isso a que descobriu a
    // corrida de abertura: `state()` aqui abortava o processo quando a webview
    // chegava antes do `setup`.
    crate::services(&app)?.attention.needs_attention_count()
}

#[tauri::command]
pub fn attention_snooze<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    until: String,
) -> Result<Reminder, CoreError> {
    let id = ReminderId::parse(&id)?;
    let until = parse_instant(&until)?;
    let updated = crate::services(&app)?.attention.snooze(id, until)?;
    poke(&app);
    Ok(updated)
}

#[tauri::command]
pub fn attention_complete<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> Result<Reminder, CoreError> {
    let updated = crate::services(&app)?
        .attention
        .complete(ReminderId::parse(&id)?)?;
    // `poke` e nao so `notify_badge`: uma serie recorrente acabou de ganhar
    // proxima ocorrencia, e o agendador calculou o proximo despertar com a
    // antiga.
    poke(&app);
    Ok(updated)
}

#[tauri::command]
pub fn attention_acknowledge<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> Result<Reminder, CoreError> {
    let updated = crate::services(&app)?
        .attention
        .acknowledge(ReminderId::parse(&id)?)?;
    notify_badge(&app);
    Ok(updated)
}

/// Cancelar e desistir da intencao. Continua consultavel: a ADR-035 diz que
/// desfazer arquiva e nunca apaga, e o mesmo vale aqui.
#[tauri::command]
pub fn attention_cancel<R: Runtime>(app: AppHandle<R>, id: String) -> Result<Reminder, CoreError> {
    let updated = crate::services(&app)?
        .attention
        .cancel(ReminderId::parse(&id)?)?;
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

// ------------------------------------------------------ comandos novos

/// O que a interface manda para criar um lembrete.
///
/// Um comando só para os oito casos, e não oito comandos: criar um lembrete
/// simples, um persistente, um recorrente, um com pilha de alertas e um
/// follow-up são o MESMO gesto com campos diferentes. Oito comandos seriam oito
/// lugares para a regra divergir.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewReminderInput {
    pub title: String,
    #[serde(default)]
    pub body: String,
    /// RFC 3339, já resolvido pelo renderer. `None` é "algum dia".
    ///
    /// O cálculo de "amanhã de manhã" acontece na interface de propósito: é o
    /// único lado que conhece o fuso de quem clicou (`CORE-FOUNDATION.md` §5).
    #[serde(default)]
    pub at: Option<String>,
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub persistent: bool,
    /// A regra de repetição, no formato do domínio.
    #[serde(default)]
    pub recurrence: Option<mos_core::Recurrence>,
    #[serde(default)]
    pub waiting_for: Option<String>,
    /// Adiantamentos em minutos, para a pilha de alertas.
    #[serde(default)]
    pub leads: Vec<u32>,
}

#[tauri::command]
pub fn attention_new<R: Runtime>(
    app: AppHandle<R>,
    input: NewReminderInput,
) -> Result<Reminder, CoreError> {
    let at = match input.at.as_deref() {
        Some(texto) => Some(parse_instant(texto)?),
        None => None,
    };
    let target = parse_target(input.target_type, input.target_id)?;
    let priority = match input.priority.as_deref() {
        Some(texto) => mos_core::Priority::parse(texto)?,
        None => mos_core::Priority::Normal,
    };

    let criado = crate::services(&app)?
        .attention
        .create(mos_core::CreateReminder {
            title: input.title,
            body: input.body,
            at,
            target,
            priority,
            source: ReminderSource::User,
            persistent: input.persistent,
            recurrence: input.recurrence,
            waiting_for: input.waiting_for,
            leads: input.leads,
        })?;

    poke(&app);
    Ok(criado)
}

/// Remarcar: muda a hora PLANEJADA. Não é adiar, e não conta fadiga.
#[tauri::command]
pub fn attention_reschedule<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    at: String,
) -> Result<Reminder, CoreError> {
    let atualizado = crate::services(&app)?
        .attention
        .reschedule(ReminderId::parse(&id)?, parse_instant(&at)?)?;
    poke(&app);
    Ok(atualizado)
}

/// O que está sendo esquecido, e por quê.
///
/// Devolve o lembrete junto com os motivos: a tela precisa DIZER por que aquilo
/// está ali, e uma lista sem o porquê é uma lista que a pessoa aprende a não
/// abrir.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionRow {
    pub reminder: Reminder,
    pub reasons: Vec<String>,
    pub weight: u32,
}

#[tauri::command]
pub fn attention_needs<R: Runtime>(app: AppHandle<R>) -> Result<Vec<AttentionRow>, CoreError> {
    Ok(crate::services(&app)?
        .attention
        .attention_list()?
        .into_iter()
        .map(|(reminder, item)| AttentionRow {
            reminder,
            reasons: item
                .reasons
                .iter()
                .map(|motivo| motivo.as_str().to_owned())
                .collect(),
            weight: item.weight,
        })
        .collect())
}

#[tauri::command]
pub fn attention_triggers<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> Result<Vec<mos_core::ReminderTrigger>, CoreError> {
    crate::services(&app)?
        .attention
        .triggers(ReminderId::parse(&id)?)
}

#[tauri::command]
pub fn attention_add_trigger<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    at: Option<String>,
    lead_minutes: Option<u32>,
) -> Result<Reminder, CoreError> {
    let instante = match at.as_deref() {
        Some(texto) => parse_instant(texto)?,
        // Sem instante, o adiantamento manda — e ele é medido a partir do prazo
        // que o lembrete já tem.
        None => OffsetDateTime::now_utc(),
    };
    let atualizado = crate::services(&app)?.attention.add_trigger(
        ReminderId::parse(&id)?,
        instante,
        lead_minutes,
    )?;
    poke(&app);
    Ok(atualizado)
}

#[tauri::command]
pub fn attention_cancel_trigger<R: Runtime>(
    app: AppHandle<R>,
    reminder: String,
    trigger: String,
) -> Result<Reminder, CoreError> {
    let atualizado = crate::services(&app)?.attention.cancel_trigger(
        ReminderId::parse(&reminder)?,
        mos_core::ReminderTriggerId::parse(&trigger)?,
    )?;
    poke(&app);
    Ok(atualizado)
}

#[tauri::command]
pub fn attention_history<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    limit: Option<usize>,
) -> Result<Vec<mos_core::ReminderEvent>, CoreError> {
    crate::services(&app)?
        .attention
        .history(ReminderId::parse(&id)?, limit.unwrap_or(30))
}

#[tauri::command]
pub fn attention_settings<R: Runtime>(
    app: AppHandle<R>,
) -> Result<mos_core::AttentionSettings, CoreError> {
    crate::services(&app)?.attention.settings()
}

#[tauri::command]
pub fn attention_save_settings<R: Runtime>(
    app: AppHandle<R>,
    settings: mos_core::AttentionSettings,
) -> Result<mos_core::AttentionSettings, CoreError> {
    let gravado = crate::services(&app)?.attention.save_settings(settings)?;
    // O silêncio pode ter mudado: o que estava segurado talvez já possa sair.
    poke(&app);
    Ok(gravado)
}

/// O renderer conta ao backend em que fuso ele está.
///
/// Chamado na abertura, e é o que faz "silêncio das 00h às 08h" significar a
/// meia-noite de quem olha. O processo do agendador não tem como perguntar
/// sozinho: o `time` sem a feature `local-offset` não sabe o fuso, e ligá-la
/// traria um caminho não-Windows que este app não precisa.
#[tauri::command]
pub fn attention_set_offset<R: Runtime>(app: AppHandle<R>, minutes: i16) -> Result<(), CoreError> {
    let servicos = crate::services(&app)?;
    let atual = servicos.attention.settings()?;
    if atual.local_offset_minutes == minutes {
        return Ok(());
    }
    servicos
        .attention
        .save_settings(mos_core::AttentionSettings {
            local_offset_minutes: minutes,
            ..atual
        })?;
    Ok(())
}

/// Lê uma frase e devolve o lembrete que ela pede, SEM gravar nada.
///
/// # Por que determinístico, e não o Hermes
///
/// "amanhã 9h" não precisa de um modelo de linguagem, e mandá-la para um custa
/// latência, dinheiro e a possibilidade de a resposta ser diferente da de
/// ontem. O `resolve_when` já existia para a voz e entende as formas que
/// importam — `daqui a 30 minutos`, `sexta`, `dia 10 às 14h`, `às nove da
/// noite`. O §12 do pedido é explícito: parser determinístico sempre que
/// possível.
///
/// O que sobra para o Hermes é o que este não resolve, e aí a pessoa escolhe
/// mandar. Não há chamada automática: um campo de texto que sai para a rede a
/// cada tecla seria um campo de texto lento.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedReminder {
    /// O que sobrou da frase depois de tirar o tempo. É o título.
    pub title: String,
    /// RFC 3339, ou `None` quando a frase não disse quando.
    pub at: Option<String>,
    /// O trecho que virou hora, com a grafia que a pessoa usou. A tela mostra
    /// isto grifado, para ela conferir antes de confirmar.
    pub when_text: Option<String>,
    /// A frase pediu para não deixar esquecer.
    pub persistent: bool,
}

/// As formas de dizer "não me deixa esquecer".
///
/// Lista curta e literal de propósito (§29 do pedido). Reconhecer intenção por
/// semelhança faria "não esqueci de nada" virar um lembrete persistente — e um
/// sistema que insiste por engano é um sistema que se desliga.
const NAO_ESQUECER: &[&str] = &[
    "nao me deixa esquecer",
    "nao me deixe esquecer",
    "nao deixa eu esquecer",
    "nao me deixar esquecer",
    "sem falta",
    "nao posso esquecer",
];

#[tauri::command]
pub fn attention_parse<R: Runtime>(
    app: AppHandle<R>,
    text: String,
    offset_minutes: i16,
) -> Result<ParsedReminder, CoreError> {
    let agora = crate::services(&app)?.clock.now();
    let deslocamento = time::UtcOffset::from_whole_seconds(i32::from(offset_minutes) * 60)
        .unwrap_or(time::UtcOffset::UTC);
    Ok(parse_reminder_phrase(&text, agora.to_offset(deslocamento)))
}

/// A parte pura, para o teste não precisar de janela.
fn parse_reminder_phrase(text: &str, now_local: OffsetDateTime) -> ParsedReminder {
    let persistent = frase_pede_persistencia(text);
    let achado = mos_core::resolve_when(text, now_local);

    let (at, when_text, titulo) = match achado {
        None => (None, None, text.trim().to_owned()),
        Some(resolvido) => {
            let instante = resolvido.instant;
            let trecho = resolvido.raw.clone();
            let titulo = text.replace(&trecho, " ");
            (
                Some(
                    instante
                        .to_offset(time::UtcOffset::UTC)
                        .format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_default(),
                ),
                Some(trecho),
                titulo,
            )
        }
    };

    ParsedReminder {
        title: limpar_titulo(&titulo),
        at,
        when_text,
        persistent,
    }
}

fn frase_pede_persistencia(text: &str) -> bool {
    let normal = mos_core::normalizar_frase(text);
    NAO_ESQUECER.iter().any(|marca| normal.contains(marca))
}

/// Tira o que sobra depois de recortar o tempo e as fórmulas de pedido.
///
/// "Me lembra de enviar as bases hoje 20:30" tem que virar "Enviar as bases", e
/// não "Me lembra de enviar as bases" — o título é o TRABALHO, e repetir o
/// pedido dentro dele é ruído em toda tela que mostrar aquele lembrete.
fn limpar_titulo(bruto: &str) -> String {
    // O recorte do tempo deixa buracos no meio: "me lembra hoje 20:30 de
    // enviar" vira "me lembra   de enviar". Normalizar antes e o que permite
    // reconhecer a abertura — sem isso, "me lembra " casaria e deixaria um "de"
    // orfao na frente, e o titulo sairia "De enviar as bases".
    let mut texto = bruto.split_whitespace().collect::<Vec<_>>().join(" ");

    // As aberturas, do mais longo para o mais curto. `de` sozinho fica por
    // ultimo e existe justamente para o buraco descrito acima.
    const ABERTURAS: &[&str] = &[
        "nao me deixa esquecer de ",
        "nao me deixe esquecer de ",
        "nao me deixa esquecer ",
        "me lembrar de ",
        "me lembra de ",
        "me lembre de ",
        "lembrar de ",
        "lembrete de ",
        "me lembra ",
        "me lembre ",
        "lembrete ",
        "de ",
    ];

    loop {
        let normal = mos_core::normalizar_frase(&texto);
        let Some(abertura) = ABERTURAS
            .iter()
            .find(|abertura| normal.starts_with(*abertura))
        else {
            break;
        };
        texto = texto[abertura.len()..].trim_start().to_owned();
    }

    // As caudas: "e nao me deixa esquecer" no fim da frase.
    let normal = mos_core::normalizar_frase(&texto);
    for marca in NAO_ESQUECER {
        if let Some(corte) = normal.find(marca) {
            // So corta se estiver perto do fim: no meio da frase, o trecho pode
            // ser o proprio assunto.
            if corte + marca.len() >= normal.len().saturating_sub(3) {
                texto = texto[..corte].to_owned();
            }
        }
    }

    // O "e" solto que sobra dos dois lados de "... e nao me deixa esquecer".
    //
    // PALAVRA, e nao caractere: `trim_start_matches('e')` comia o "e" de
    // "enviar", e o titulo virava "Nviar o PDF". E nas duas pontas, porque a
    // conjuncao pode ficar na frente (quando a cauda saiu do comeco) ou atras
    // (o caso normal, "enviar as bases e nao me deixa esquecer").
    let texto = tirar_conjuncao(texto.trim());

    // A primeira letra em maiuscula. Uma lista em que metade dos titulos comeca
    // minusculo le como uma lista descuidada.
    let mut chars = texto.chars();
    match chars.next() {
        None => String::new(),
        Some(primeira) => primeira.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Tira o "e" solto das pontas, e a pontuacao que sobra com ele.
fn tirar_conjuncao(texto: &str) -> String {
    let pontuacao = |c: char| c == ',' || c == '.' || c == ';' || c.is_whitespace();
    let texto = texto.trim_matches(pontuacao);
    let texto = texto
        .strip_prefix("e ")
        .or_else(|| texto.strip_prefix("E "))
        .unwrap_or(texto)
        .trim_matches(pontuacao);
    let texto = texto
        .strip_suffix(" e")
        .or_else(|| texto.strip_suffix(" E"))
        .unwrap_or(texto);
    texto.trim_matches(pontuacao).to_owned()
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

    // ------------------------------------------------- Quick Reminder

    /// Quinta-feira, 8 de setembro de 2026, às 15:00 em Brasília.
    fn agora() -> OffsetDateTime {
        time::macros::datetime!(2026-09-08 15:00 -3)
    }

    /// O caso do §62, digitado de uma vez só.
    #[test]
    fn the_motivating_phrase_becomes_a_persistent_reminder() {
        let lido = parse_reminder_phrase(
            "Me lembra hoje 20:30 de enviar as bases para o Victor e nao me deixa esquecer",
            agora(),
        );
        assert!(lido.persistent, "a frase pediu para nao deixar esquecer");
        assert!(lido.at.is_some());
        assert!(
            lido.title.to_lowercase().contains("bases"),
            "titulo ficou: {}",
            lido.title
        );
        assert!(
            !lido.title.to_lowercase().contains("me lembra"),
            "o pedido nao pode virar parte do titulo: {}",
            lido.title
        );
        assert!(
            !lido.title.to_lowercase().contains("esquecer"),
            "a formula tambem sai do titulo: {}",
            lido.title
        );
    }

    /// As formas simples resolvem sem modelo nenhum. É o §12: parser
    /// determinístico sempre que possível.
    #[test]
    fn the_simple_forms_never_need_a_model() {
        for frase in [
            "Ligar para a Ana amanha 9h",
            "Revisar a prancha daqui a 30 minutos",
            "Pagar o boleto dia 10 as 14h",
            "Mandar o PDF sexta a tarde",
        ] {
            let lido = parse_reminder_phrase(frase, agora());
            assert!(lido.at.is_some(), "nao resolveu: {frase}");
            assert!(!lido.title.is_empty(), "titulo vazio: {frase}");
            assert!(lido.when_text.is_some(), "sem trecho grifado: {frase}");
        }
    }

    /// Sem tempo na frase, o lembrete nasce sem data — e isso é legítimo.
    /// Inventar uma hora seria inventar um compromisso.
    #[test]
    fn a_phrase_without_a_time_becomes_a_someday() {
        let lido = parse_reminder_phrase("Comprar cabo HDMI", agora());
        assert!(lido.at.is_none());
        assert_eq!(lido.title, "Comprar cabo HDMI");
        assert!(!lido.persistent);
    }

    /// A persistência é reconhecida por marca literal, e não por semelhança:
    /// "não esqueci de nada" não pode virar um lembrete que insiste.
    #[test]
    fn only_the_literal_phrases_ask_for_persistence() {
        assert!(parse_reminder_phrase("Nao me deixe esquecer disso", agora()).persistent);
        assert!(parse_reminder_phrase("Mandar sem falta", agora()).persistent);
        assert!(!parse_reminder_phrase("Nao esqueci de nada", agora()).persistent);
        assert!(!parse_reminder_phrase("Lembrar do esquecimento", agora()).persistent);
    }

    /// O título começa em maiúscula, e sem sobra de pontuação.
    #[test]
    fn the_title_comes_out_clean() {
        assert_eq!(
            limpar_titulo("  me lembra de enviar o PDF  "),
            "Enviar o PDF"
        );
        assert_eq!(
            limpar_titulo("lembrete de regar as plantas"),
            "Regar as plantas"
        );
        assert_eq!(limpar_titulo("enviar as bases ,"), "Enviar as bases");
        assert_eq!(limpar_titulo(""), "");
    }

    /// O caso que a TELA mostrou, e que os testes não pegavam.
    ///
    /// Com a hora NO MEIO da frase — "me lembra **hoje 20:30** de enviar" —, o
    /// recorte do tempo deixa "me lembra   de enviar", e o título saía "De
    /// enviar as bases para o Victor e": um "de" órfão na frente e uma
    /// conjunção solta atrás. Nenhum teste cobria isso porque todos punham a
    /// hora no fim da frase — que é o jeito que ninguém fala.
    #[test]
    fn the_time_in_the_middle_does_not_leave_scraps_in_the_title() {
        let lido = parse_reminder_phrase(
            "Me lembra hoje 20:30 de enviar as bases para o Victor e nao me deixa esquecer",
            agora(),
        );
        assert_eq!(lido.title, "Enviar as bases para o Victor");
        assert!(lido.persistent);
    }

    #[test]
    fn the_leftover_conjunction_goes_from_both_ends() {
        assert_eq!(limpar_titulo("de enviar o PDF e"), "Enviar o PDF");
        assert_eq!(limpar_titulo("e enviar o PDF"), "Enviar o PDF");
        // E não come a palavra que COMEÇA com a conjunção.
        assert_eq!(limpar_titulo("estudar calculo"), "Estudar calculo");
        assert_eq!(limpar_titulo("de-para das camadas"), "De-para das camadas");
    }
}
