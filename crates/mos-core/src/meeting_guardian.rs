//! O Recording Guardian: perceber que a reuniao provavelmente acabou.
//!
//! # O problema que ele resolve
//!
//! A pessoa sai do Teams e esquece o M/OS gravando. Horas depois, a gravacao
//! continua — ocupando disco, e depois custando meia hora de whisper sobre um
//! escritorio vazio. Esquecer a gravacao ligada deixa de ser problema da pessoa
//! e passa a ser do Meeting Agent.
//!
//! # O que ele NAO faz
//!
//! - **nao le conteudo.** Nenhum sinal aqui e o que foi dito: e QUEM tem o
//!   microfone (ADR-047), SE ha energia sonora (nivel RMS, o mesmo numero que ja
//!   pinta a onda), e se a pessoa esta no computador;
//! - **nao para por silencio.** Reuniao tem silencio. Silencio sozinho, com o
//!   app da chamada ainda no microfone, nunca passa de pergunta — e so depois de
//!   vinte minutos;
//! - **nao para porque o Calendar acabou.** Reuniao atrasa. E `Event` nem
//!   existe (`MEETING-AGENT.md` §0.3);
//! - **nao pergunta de novo a cada silencio.** Cooldown que escala, e so um
//!   sinal NOVO e forte fura o cooldown.
//!
//! Puro: recebe o instante por parametro e nao conhece Windows. O laco do
//! desktop coleta os sinais e obedece o veredito.
//!
//! Spec: `docs/superpowers/specs/2026-09-16-meeting-agent-v2-design.md` §3.

use serde::{Deserialize, Serialize};

use crate::StopReason;

/// As tres caixas de Settings, e o limite da reuniao longa.
///
/// Os limiares tecnicos NAO estao aqui: eles sao constantes do modulo, e nao
/// preferencia. Uma tela com "segundos de tolerancia" transformaria a pessoa
/// em quem calibra a heuristica.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardianConfig {
    /// Avisar quando a reuniao parecer ter terminado.
    pub warn_on_end: bool,
    /// Encerrar sozinho, com contagem regressiva, quando a confianca for alta.
    pub auto_stop: bool,
    /// Avisar sobre gravacoes anormalmente longas.
    pub warn_long: bool,
}

impl Default for GuardianConfig {
    fn default() -> Self {
        Self {
            warn_on_end: true,
            auto_stop: false,
            warn_long: true,
        }
    }
}

/// O app associado liberou o microfone ha pelo menos isto: sinal forte.
pub const GRACE_SECS: i64 = 90;
/// Silencio de um canal ha pelo menos isto: sinal medio.
pub const QUIET_SECS: i64 = 90;
/// Silencio longo de um canal.
pub const LONG_QUIET_SECS: i64 = 300;
/// Atividade mais recente que isto veta a suspeita (enquanto o app tem o mic).
pub const RECENT_ACTIVITY_SECS: i64 = 45;
/// Depois disto sem microfone, som remoto deixa de vetar: e musica, e nao a
/// chamada.
pub const RELEASE_STALE_SECS: i64 = 600;
/// Sem app associado, ou com o app ainda no microfone: silencio dos dois canais
/// por tanto tempo vira pergunta.
pub const SILENCE_ASK_SECS: i64 = 1_200;
/// Sem app: silencio por tanto tempo, com a pessoa longe, vira confianca alta.
pub const SILENCE_AUTO_SECS: i64 = 2_700;
/// Ausencia (tela bloqueada ou sem input) que conta como sinal.
pub const AWAY_SECS: i64 = 300;
/// Ausencia longa, para a regra sem app.
pub const LONG_AWAY_SECS: i64 = 1_800;
/// Primeira pergunta de reuniao longa.
pub const LONG_MEETING_MS: i64 = 2 * 60 * 60 * 1000;
/// Silencio que acompanha a pergunta de reuniao longa.
pub const LONG_MEETING_QUIET_SECS: i64 = 600;
/// A contagem regressiva do auto-stop.
pub const COUNTDOWN_SECS: i64 = 20;
/// O primeiro cooldown depois de "continuar gravando". Dobra a cada vez.
pub const COOLDOWN_SECS: i64 = 15 * 60;
pub const MAX_COOLDOWN_SECS: i64 = 60 * 60;
/// Janela, desde o inicio, em que o Guardian ainda adota um app associado.
pub const ADOPT_WINDOW_SECS: i64 = 300;
/// Margem somada ao fim provavel: cortar no instante exato da ultima fala come
/// a despedida.
pub const END_MARGIN_MS: i64 = 15_000;
/// Excesso minimo para valer sugerir corte.
pub const TRIM_MIN_EXCESS_MS: i64 = 10 * 60 * 1000;
/// Microfone em zero digital por tanto tempo, com a chamada ativa: aviso.
pub const MIC_DEAD_SECS: i64 = 120;

pub const ASK_THRESHOLD: f32 = 0.55;
pub const CLEAR_THRESHOLD: f32 = 0.40;
pub const AUTO_THRESHOLD: f32 = 0.80;

/// Quem esta com o microfone aberto agora, sem o proprio M/OS.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MicUser {
    pub process: String,
    pub seconds_open: i64,
}

/// O que o laco observou neste instante.
#[derive(Clone, Debug, Default)]
pub struct Observation {
    /// Epoch em segundos.
    pub now: i64,
    /// A duracao gravada, em ms — a regua do corte. Nao e relogio: nao conta
    /// pausa.
    pub duration_ms: i64,
    pub paused: bool,
    /// O MAIOR nivel desde a ultima observacao, em milesimos. Amostrar o
    /// instantaneo cairia nas pausas entre palavras.
    pub mic_level: u64,
    pub system_level: u64,
    /// Os canais ainda capturam? Um canal perdido nao produz nivel, e o silencio
    /// dele nao pode ser lido como silencio da sala.
    pub mic_alive: bool,
    pub system_alive: bool,
    pub mic_users: Vec<MicUser>,
    /// O processo do app associado ainda existe. `None` = nao da para saber
    /// (app da Store, identificado por familia de pacote).
    pub app_running: Option<bool>,
    pub locked: bool,
    pub idle_secs: Option<i64>,
}

/// Uma pergunta em aberto.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptKind {
    /// "Parece que sua reuniao terminou."
    Ended,
    /// "Esta reuniao esta sendo gravada ha 2h37. Ela ainda esta acontecendo?"
    Long,
}

/// O que a tela deve mostrar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuardianView {
    Idle,
    Ask {
        prompt: PromptKind,
        /// Qual sinal pesou mais. Diagnostico, e nao frase.
        trigger: String,
        /// Onde a conversa provavelmente acabou, em ms relativos.
        suggested_end_ms: Option<i64>,
        /// Quanto foi gravado depois disso.
        excess_ms: i64,
    },
    Countdown {
        seconds_left: i64,
        trigger: String,
        suggested_end_ms: Option<i64>,
        excess_ms: i64,
    },
}

/// O que aconteceu nesta volta, para metrica e para agir.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuardianEvent {
    Suggested {
        confidence: f32,
        trigger: String,
    },
    LongPrompted {
        confidence: f32,
    },
    /// A suspeita se desfez sozinha: o som voltou, o app voltou ao microfone.
    Cleared,
    CountdownStarted {
        confidence: f32,
        trigger: String,
    },
    CountdownCancelled,
    /// Encerre agora, por este motivo.
    AutoStop {
        reason: StopReason,
        confidence: f32,
        trigger: String,
    },
    /// O microfone esta em zero digital com a chamada ativa.
    MicSilent,
}

/// O estado do Guardian durante uma gravacao.
///
/// Serializavel porque e ele que a tela de diagnostico mostra, e porque o
/// fim provavel sobrevive na reuniao depois de parar.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardianState {
    pub started_at: i64,
    pub associated_app: Option<String>,
    pub app_mic_active: bool,
    /// Quando o app associado liberou o microfone (epoch s) — e em que ponto
    /// da gravacao (ms).
    pub app_released_at: Option<i64>,
    pub app_released_at_ms: Option<i64>,
    pub app_process_gone_at: Option<i64>,
    pub mic_floor: f32,
    pub system_floor: f32,
    pub last_mic_activity_at: Option<i64>,
    pub last_system_activity_at: Option<i64>,
    pub last_mic_activity_ms: Option<i64>,
    pub last_system_activity_ms: Option<i64>,
    /// A ancora do silencio: inicio, ou a ultima vez que a pausa acabou. Pausa
    /// nao e silencio da sala.
    pub quiet_anchor_at: i64,
    pub away_since: Option<i64>,
    pub mic_zero_since: Option<i64>,
    pub mic_dead_warned: bool,

    pub confidence: f32,
    pub trigger: String,
    pub prompt: Option<PromptKind>,
    pub prompt_since: Option<i64>,
    pub countdown_until: Option<i64>,
    pub dismissed_at: Option<i64>,
    pub dismiss_count: u32,
    pub cooldown_until: Option<i64>,
    /// Duracao (ms) em que a proxima pergunta de reuniao longa pode acontecer.
    pub next_long_prompt_ms: i64,
    /// O Guardian ja mandou encerrar. Depois disso ele se cala: quem obedece
    /// para a gravacao, e uma segunda ordem so repetiria a primeira.
    pub auto_stopped: bool,
}

impl GuardianState {
    /// Uma gravacao nova. `associated_app` e o alvo da oferta, quando houve.
    pub fn start(now: i64, associated_app: Option<String>) -> Self {
        Self {
            started_at: now,
            app_mic_active: associated_app.is_some(),
            associated_app,
            mic_floor: 1.0,
            system_floor: 1.0,
            quiet_anchor_at: now,
            next_long_prompt_ms: LONG_MEETING_MS,
            ..Self::default()
        }
    }

    /// Onde a conversa provavelmente acabou, em ms relativos.
    ///
    /// Com app: o instante em que ele largou o microfone — a chamada acabou ali.
    /// Sem app: a ultima atividade de qualquer canal. Nos dois, com a margem da
    /// despedida.
    pub fn suggested_end_ms(&self) -> Option<i64> {
        let base = match self.app_released_at_ms {
            Some(released) => Some(released),
            None => self.last_mic_activity_ms.max(self.last_system_activity_ms),
        }?;
        Some(base + END_MARGIN_MS)
    }

    fn excess_ms(&self, duration_ms: i64) -> i64 {
        self.suggested_end_ms()
            .map(|end| (duration_ms - end).max(0))
            .unwrap_or(0)
    }

    fn quiet_secs(&self, last: Option<i64>, now: i64) -> i64 {
        let anchor = last.unwrap_or(self.started_at).max(self.quiet_anchor_at);
        (now - anchor).max(0)
    }

    fn in_cooldown(&self, now: i64) -> bool {
        self.cooldown_until.is_some_and(|until| now < until)
    }

    /// Um sinal forte que nasceu DEPOIS de a pessoa dizer "continuar".
    ///
    /// Silencio nunca e sinal novo: ele so cresce. O que fura o cooldown e um
    /// fato que nao existia no momento do clique.
    fn new_strong_signal_since_dismiss(&self) -> bool {
        let Some(dismissed) = self.dismissed_at else {
            return false;
        };
        self.app_released_at.is_some_and(|at| at > dismissed)
            || self.app_process_gone_at.is_some_and(|at| at > dismissed)
    }

    /// A pessoa clicou "Continuar gravando".
    pub fn keep_recording(&mut self, now: i64) -> Vec<GuardianEvent> {
        let mut events = Vec::new();
        if self.countdown_until.take().is_some() {
            events.push(GuardianEvent::CountdownCancelled);
        }
        if self.prompt == Some(PromptKind::Long) {
            // A pergunta longa volta daqui a uma hora, e nao daqui a quinze
            // minutos: quem disse "ainda esta acontecendo" disse sobre as
            // proximas dezenas de minutos.
            self.next_long_prompt_ms += 60 * 60 * 1000;
        }
        self.prompt = None;
        self.prompt_since = None;
        self.dismissed_at = Some(now);
        self.dismiss_count += 1;
        let cooldown = (COOLDOWN_SECS << (self.dismiss_count - 1).min(4)).min(MAX_COOLDOWN_SECS);
        self.cooldown_until = Some(now + cooldown);
        events
    }

    /// O que a tela mostra agora.
    pub fn view(&self, now: i64, duration_ms: i64) -> GuardianView {
        if let Some(until) = self.countdown_until {
            return GuardianView::Countdown {
                seconds_left: (until - now).max(0),
                trigger: self.trigger.clone(),
                suggested_end_ms: self.suggested_end_ms(),
                excess_ms: self.excess_ms(duration_ms),
            };
        }
        match self.prompt {
            Some(prompt) => GuardianView::Ask {
                prompt,
                trigger: self.trigger.clone(),
                suggested_end_ms: self.suggested_end_ms(),
                excess_ms: self.excess_ms(duration_ms),
            },
            None => GuardianView::Idle,
        }
    }

    /// O corte automatico ao parar, quando ele e seguro.
    ///
    /// Seguro significa: o app da chamada largou o microfone, nenhuma atividade
    /// veio depois disso, e o excesso passa de dez minutos. Sem sinal de app, a
    /// resposta e `None` — a sugestao continua possivel depois de transcrever,
    /// mas cortar sozinho sem saber quando a chamada acabou seria palpite.
    pub fn confident_trim_end(&self, duration_ms: i64) -> Option<i64> {
        let released_ms = self.app_released_at_ms?;
        let last_activity = self
            .last_mic_activity_ms
            .max(self.last_system_activity_ms)
            .unwrap_or(0);
        // Uma fala um minuto depois de largar o microfone ainda pode ser a
        // despedida. Mais que isso, alguem continuou falando: nao e seguro.
        if last_activity > released_ms + 60_000 {
            return None;
        }
        let end = self.suggested_end_ms()?;
        (duration_ms - end >= TRIM_MIN_EXCESS_MS).then_some(end)
    }
}

/// Atualiza o piso de ruido: cai rapido, sobe devagar.
///
/// Cai rapido porque silencio real e o piso verdadeiro; sobe devagar porque
/// uma fala longa nao pode ensinar o piso a achar que fala e ruido.
fn update_floor(floor: &mut f32, level: f32) {
    if level < *floor {
        *floor = *floor * 0.7 + level * 0.3;
    } else {
        *floor = *floor * 0.995 + level * 0.005;
    }
    // Com teto. Sem ele, um som constante — musica depois da chamada, um
    // ventilador perto do microfone — viraria "piso" em um minuto e meio, e a
    // sala passaria a contar como silenciosa com o som tocando.
    *floor = floor.min(FLOOR_CAP);
}

/// O maior piso de ruido que se admite, em milesimos (≈ −50 dBFS).
const FLOOR_CAP: f32 = 10.0;

/// Houve atividade neste canal?
///
/// **Relativo ao piso, e nao limiar fixo.** O microfone desta casa fala a
/// −44 dBFS (≈6‰): um limiar absoluto razoavel para outro microfone apagaria
/// a pessoa. O loopback em silencio e zero digital por causa do keep-alive,
/// entao ali qualquer som conta.
fn is_active(level: u64, floor: f32) -> bool {
    level as f32 > floor * 3.0 + 2.0
}

/// Uma volta do Guardian.
pub fn observe(
    state: &mut GuardianState,
    observation: &Observation,
    config: &GuardianConfig,
) -> (GuardianView, Vec<GuardianEvent>) {
    let now = observation.now;
    let mut events = Vec::new();
    if state.auto_stopped {
        return (GuardianView::Idle, events);
    }

    track_app(state, observation);
    track_activity(state, observation);
    track_away(state, observation);
    if let Some(event) = track_mic_health(state, observation) {
        events.push(event);
    }

    // Pausado nao se julga. A ancora do silencio anda junto, para a volta da
    // pausa nao chegar com dez minutos de "silencio" que ninguem viveu.
    if observation.paused {
        state.quiet_anchor_at = now;
        if state.countdown_until.take().is_some() {
            events.push(GuardianEvent::CountdownCancelled);
        }
        if state.prompt == Some(PromptKind::Ended) {
            state.prompt = None;
            state.prompt_since = None;
            events.push(GuardianEvent::Cleared);
        }
        state.confidence = 0.0;
        return (state.view(now, observation.duration_ms), events);
    }

    let (confidence, trigger, auto_reason) = score(state, observation);
    state.confidence = confidence;

    // --- contagem regressiva em curso ------------------------------------
    if let Some(until) = state.countdown_until {
        if confidence < ASK_THRESHOLD {
            state.countdown_until = None;
            state.prompt = None;
            state.prompt_since = None;
            events.push(GuardianEvent::CountdownCancelled);
            events.push(GuardianEvent::Cleared);
        } else if now >= until {
            state.countdown_until = None;
            state.auto_stopped = true;
            events.push(GuardianEvent::AutoStop {
                reason: auto_reason.unwrap_or(StopReason::AutoMeetingEnded),
                confidence,
                trigger: state.trigger.clone(),
            });
        }
        return (state.view(now, observation.duration_ms), events);
    }

    // --- a suspeita se desfez --------------------------------------------
    if state.prompt == Some(PromptKind::Ended) && confidence < CLEAR_THRESHOLD {
        state.prompt = None;
        state.prompt_since = None;
        events.push(GuardianEvent::Cleared);
    }

    let cooldown_blocks = state.in_cooldown(now) && !state.new_strong_signal_since_dismiss();

    // --- auto-stop ---------------------------------------------------------
    if config.auto_stop && confidence >= AUTO_THRESHOLD && auto_reason.is_some() && !cooldown_blocks
    {
        state.trigger = trigger.clone();
        state.countdown_until = Some(now + COUNTDOWN_SECS);
        state.prompt = Some(PromptKind::Ended);
        state.prompt_since.get_or_insert(now);
        events.push(GuardianEvent::CountdownStarted {
            confidence,
            trigger,
        });
        return (state.view(now, observation.duration_ms), events);
    }

    // --- perguntar se acabou -----------------------------------------------
    if config.warn_on_end
        && state.prompt.is_none()
        && confidence >= ASK_THRESHOLD
        && !cooldown_blocks
    {
        state.trigger = trigger.clone();
        state.prompt = Some(PromptKind::Ended);
        state.prompt_since = Some(now);
        events.push(GuardianEvent::Suggested {
            confidence,
            trigger,
        });
        return (state.view(now, observation.duration_ms), events);
    }

    // --- reuniao longa -----------------------------------------------------
    if config.warn_long
        && state.prompt.is_none()
        && observation.duration_ms >= state.next_long_prompt_ms
        && !state.in_cooldown(now)
    {
        let quiet_mic = state.quiet_secs(state.last_mic_activity_at, now);
        let quiet_system = state.quiet_secs(state.last_system_activity_at, now);
        let inactive = (quiet_mic >= LONG_MEETING_QUIET_SECS
            && quiet_system >= LONG_MEETING_QUIET_SECS)
            || (state.associated_app.is_some() && !state.app_mic_active);
        if inactive {
            state.trigger = "long_meeting".into();
            state.prompt = Some(PromptKind::Long);
            state.prompt_since = Some(now);
            events.push(GuardianEvent::LongPrompted { confidence });
        }
    }

    (state.view(now, observation.duration_ms), events)
}

/// Acompanha o app associado: adocao, liberacao, retomada, encerramento.
fn track_app(state: &mut GuardianState, observation: &Observation) {
    let now = observation.now;

    if state.associated_app.is_none() && now - state.started_at <= ADOPT_WINDOW_SECS {
        // Gravacao manual sem chamada aberta: adota o primeiro que abrir o
        // microfone logo depois. Ganha o aberto ha mais tempo, pelo mesmo
        // criterio da oferta (Discord ao lado do Meet).
        if let Some(user) = observation
            .mic_users
            .iter()
            .max_by_key(|user| user.seconds_open)
        {
            state.associated_app = Some(user.process.clone());
            state.app_mic_active = true;
        }
    }

    let Some(app) = state.associated_app.clone() else {
        return;
    };
    let has_mic = observation
        .mic_users
        .iter()
        .any(|user| user.process.eq_ignore_ascii_case(&app));

    if state.app_mic_active && !has_mic {
        state.app_released_at = Some(now);
        state.app_released_at_ms = Some(observation.duration_ms);
    } else if !state.app_mic_active && has_mic {
        // Voltou ao microfone: a chamada nao tinha acabado (trocou de
        // dispositivo, reconectou). O fim provavel anterior deixa de valer.
        state.app_released_at = None;
        state.app_released_at_ms = None;
        state.app_process_gone_at = None;
    }
    state.app_mic_active = has_mic;

    if !has_mic && observation.app_running == Some(false) {
        state.app_process_gone_at.get_or_insert(now);
    } else if observation.app_running == Some(true) {
        state.app_process_gone_at = None;
    }
}

fn track_activity(state: &mut GuardianState, observation: &Observation) {
    if observation.paused {
        return;
    }
    let now = observation.now;
    let at_ms = observation.duration_ms;

    if observation.mic_alive {
        if is_active(observation.mic_level, state.mic_floor) {
            state.last_mic_activity_at = Some(now);
            state.last_mic_activity_ms = Some(at_ms);
        }
        update_floor(&mut state.mic_floor, observation.mic_level as f32);
    }
    if observation.system_alive {
        if is_active(observation.system_level, state.system_floor) {
            state.last_system_activity_at = Some(now);
            state.last_system_activity_ms = Some(at_ms);
        }
        update_floor(&mut state.system_floor, observation.system_level as f32);
    }
}

fn track_away(state: &mut GuardianState, observation: &Observation) {
    let now = observation.now;
    let idle = observation.idle_secs.unwrap_or(0);
    let away = observation.locked || idle >= AWAY_SECS;
    if away {
        // Com input ocioso, a ausencia comecou quando o input parou, e nao
        // quando o laco percebeu.
        let since = if observation.locked { now } else { now - idle };
        state.away_since = Some(state.away_since.map_or(since, |at| at.min(since)));
    } else {
        state.away_since = None;
    }
}

fn track_mic_health(state: &mut GuardianState, observation: &Observation) -> Option<GuardianEvent> {
    if observation.paused || !observation.mic_alive || !state.app_mic_active {
        state.mic_zero_since = None;
        return None;
    }
    if observation.mic_level > 0 {
        state.mic_zero_since = None;
        return None;
    }
    let since = *state.mic_zero_since.get_or_insert(observation.now);
    if !state.mic_dead_warned && observation.now - since >= MIC_DEAD_SECS {
        state.mic_dead_warned = true;
        return Some(GuardianEvent::MicSilent);
    }
    None
}

/// A confianca de que a reuniao acabou, o sinal que mais pesou, e — quando a
/// confianca sustenta encerrar sozinho — por qual motivo.
fn score(state: &GuardianState, observation: &Observation) -> (f32, String, Option<StopReason>) {
    let now = observation.now;
    let quiet_mic = if observation.mic_alive {
        state.quiet_secs(state.last_mic_activity_at, now)
    } else {
        // Canal perdido nao e sala em silencio. Ele nao soma, e nao veta.
        0
    };
    let quiet_system = if observation.system_alive {
        state.quiet_secs(state.last_system_activity_at, now)
    } else {
        0
    };
    let away_secs = state
        .away_since
        .map(|since| (now - since).max(0))
        .unwrap_or(0);

    // --- sem app associado: so silencio, e com paciencia --------------------
    let Some(_) = &state.associated_app else {
        let quiet = quiet_mic.min(quiet_system);
        if quiet >= SILENCE_AUTO_SECS && away_secs >= LONG_AWAY_SECS {
            return (
                0.85,
                "long_inactivity".into(),
                Some(StopReason::AutoInactivity),
            );
        }
        if quiet >= SILENCE_ASK_SECS {
            return (ASK_THRESHOLD, "long_silence".into(), None);
        }
        return (0.0, String::new(), None);
    };

    // --- o app ainda esta no microfone: a chamada esta de pe ----------------
    if state.app_mic_active {
        // So o silencio muito longo dos dois lados vira pergunta — a pessoa
        // pode ter esquecido de sair da chamada. Nunca auto-stop: o app ainda
        // diz que ha chamada.
        if quiet_mic.min(quiet_system) >= SILENCE_ASK_SECS {
            let bonus = if away_secs >= AWAY_SECS { 0.1 } else { 0.0 };
            return (ASK_THRESHOLD + bonus, "long_silence_in_call".into(), None);
        }
        return (0.0, String::new(), None);
    }

    // --- o app largou o microfone -------------------------------------------
    let released_for = state
        .app_released_at
        .map(|at| (now - at).max(0))
        .unwrap_or(0);
    if released_for < GRACE_SECS {
        return (0.0, String::new(), None);
    }

    let recent_activity = quiet_mic < RECENT_ACTIVITY_SECS || quiet_system < RECENT_ACTIVITY_SECS;
    if recent_activity && released_for < RELEASE_STALE_SECS {
        // Alguem ainda fala, e o microfone saiu ha pouco: pode ser troca de
        // dispositivo, ou a conversa continuando na sala. Espera.
        return (0.0, String::new(), None);
    }

    let mut confidence: f32 = 0.5;
    let mut trigger = "app_released_mic".to_owned();
    if state.app_process_gone_at.is_some() {
        confidence += 0.15;
        trigger = "app_closed".into();
    }
    if quiet_system >= QUIET_SECS {
        confidence += 0.15;
    }
    if quiet_system >= LONG_QUIET_SECS {
        confidence += 0.1;
    }
    if quiet_mic >= QUIET_SECS {
        confidence += 0.15;
    }
    if quiet_mic >= LONG_QUIET_SECS {
        confidence += 0.05;
    }
    if away_secs >= AWAY_SECS {
        confidence += 0.1;
    }

    // Com atividade recente (so depois de dez minutos sem microfone chega
    // aqui), a pergunta pode acontecer, mas encerrar sozinho nao: alguem ainda
    // esta fazendo barulho, e isso pede uma pessoa decidindo.
    if recent_activity {
        return (confidence.min(0.7), trigger, None);
    }
    let confidence = confidence.min(1.0);
    (confidence, trigger, Some(StopReason::AutoMeetingEnded))
}

/// Sugestao de corte a partir da transcricao, quando o Guardian nao sabe.
///
/// A ultima fala termina dez minutos ou mais antes do fim da gravacao: o fim
/// provavel e ela, com a margem da despedida.
pub fn trim_suggestion_from_segments(
    segment_ends_ms: impl IntoIterator<Item = i64>,
    duration_ms: i64,
) -> Option<i64> {
    let last = segment_ends_ms.into_iter().max()?;
    let end = last + END_MARGIN_MS;
    (duration_ms - end >= TRIM_MIN_EXCESS_MS).then_some(end)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_800_000_000;

    fn teams(seconds_open: i64) -> Vec<MicUser> {
        vec![MicUser {
            process: "ms-teams.exe".into(),
            seconds_open,
        }]
    }

    /// Um relogio de teste: cada `tick` e um segundo, e a gravacao anda junto.
    struct Cena {
        state: GuardianState,
        config: GuardianConfig,
        t: i64,
        views: Vec<GuardianView>,
        events: Vec<GuardianEvent>,
    }

    impl Cena {
        fn new(app: Option<&str>, config: GuardianConfig) -> Self {
            Self {
                state: GuardianState::start(T0, app.map(str::to_owned)),
                config,
                t: 0,
                views: Vec::new(),
                events: Vec::new(),
            }
        }

        fn tick(&mut self, obs: impl Fn(i64) -> Observation) -> GuardianView {
            self.t += 1;
            let mut observation = obs(self.t);
            observation.now = T0 + self.t;
            observation.duration_ms = self.t * 1000;
            let (view, events) = observe(&mut self.state, &observation, &self.config);
            self.events.extend(events);
            self.views.push(view.clone());
            view
        }

        fn run(&mut self, secs: i64, obs: impl Fn(i64) -> Observation) -> GuardianView {
            let mut last = GuardianView::Idle;
            for _ in 0..secs {
                last = self.tick(&obs);
            }
            last
        }

        fn count(&self, pred: impl Fn(&GuardianEvent) -> bool) -> usize {
            self.events.iter().filter(|event| pred(event)).count()
        }
    }

    fn conversa(users: Vec<MicUser>) -> impl Fn(i64) -> Observation {
        move |t| Observation {
            mic_level: if t % 7 < 3 { 40 } else { 1 },
            system_level: if t % 5 < 2 { 60 } else { 0 },
            mic_alive: true,
            system_alive: true,
            mic_users: users.clone(),
            app_running: Some(true),
            ..Observation::default()
        }
    }

    fn silencio(users: Vec<MicUser>, app_running: Option<bool>) -> impl Fn(i64) -> Observation {
        move |_| Observation {
            mic_level: 1,
            system_level: 0,
            mic_alive: true,
            system_alive: true,
            mic_users: users.clone(),
            app_running,
            ..Observation::default()
        }
    }

    fn ask(view: &GuardianView) -> bool {
        matches!(
            view,
            GuardianView::Ask {
                prompt: PromptKind::Ended,
                ..
            }
        )
    }

    #[test]
    fn reuniao_normal_teams_libera_e_a_pergunta_aparece_depois_da_tolerancia() {
        let mut cena = Cena::new(Some("ms-teams.exe"), GuardianConfig::default());
        cena.run(1_800, conversa(teams(60)));
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::Suggested { .. })),
            0
        );

        // Teams larga o microfone. Nos primeiros 90 s, nada.
        let view = cena.run(80, silencio(vec![], Some(true)));
        assert_eq!(view, GuardianView::Idle);

        let view = cena.run(30, silencio(vec![], Some(true)));
        assert!(ask(&view), "{view:?}");
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::Suggested { .. })),
            1
        );

        // A pergunta NAO se repete enquanto esta aberta.
        cena.run(600, silencio(vec![], Some(true)));
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::Suggested { .. })),
            1
        );
    }

    #[test]
    fn continuar_gravando_aplica_cooldown_que_escala() {
        let mut cena = Cena::new(Some("ms-teams.exe"), GuardianConfig::default());
        cena.run(600, conversa(teams(60)));
        let view = cena.run(120, silencio(vec![], Some(true)));
        assert!(ask(&view));

        cena.state.keep_recording(T0 + cena.t);
        assert_eq!(cena.state.view(T0 + cena.t, 0), GuardianView::Idle);

        // Quinze minutos de silencio: nenhuma pergunta nova.
        cena.run(14 * 60, silencio(vec![], Some(true)));
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::Suggested { .. })),
            1
        );

        // Passado o cooldown, volta a perguntar.
        cena.run(2 * 60, silencio(vec![], Some(true)));
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::Suggested { .. })),
            2
        );

        // O segundo "continuar" dura o dobro.
        cena.state.keep_recording(T0 + cena.t);
        cena.run(25 * 60, silencio(vec![], Some(true)));
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::Suggested { .. })),
            2
        );
        cena.run(6 * 60, silencio(vec![], Some(true)));
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::Suggested { .. })),
            3
        );
    }

    #[test]
    fn sinal_novo_e_forte_fura_o_cooldown() {
        let mut cena = Cena::new(Some("ms-teams.exe"), GuardianConfig::default());
        cena.run(600, conversa(teams(60)));
        cena.run(120, silencio(vec![], Some(true)));
        cena.state.keep_recording(T0 + cena.t);

        // A chamada voltou (o Teams readquiriu o microfone) e acabou de novo.
        cena.run(300, conversa(teams(60)));
        let view = cena.run(120, silencio(vec![], Some(true)));
        assert!(
            ask(&view),
            "o Teams largou o microfone DEPOIS do clique: {view:?}"
        );
    }

    #[test]
    fn auto_stop_com_contagem_regressiva() {
        let config = GuardianConfig {
            auto_stop: true,
            ..GuardianConfig::default()
        };
        let mut cena = Cena::new(Some("ms-teams.exe"), config);
        cena.run(1_200, conversa(teams(60)));

        // Teams larga o microfone e fecha. Remoto e local em silencio.
        cena.run(95, silencio(vec![], Some(false)));
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::CountdownStarted { .. })),
            1
        );
        assert!(matches!(
            cena.state.view(T0 + cena.t, 0),
            GuardianView::Countdown { .. }
        ));

        cena.run(COUNTDOWN_SECS + 1, silencio(vec![], Some(false)));
        let stops: Vec<_> = cena
            .events
            .iter()
            .filter_map(|e| match e {
                GuardianEvent::AutoStop { reason, .. } => Some(*reason),
                _ => None,
            })
            .collect();
        assert_eq!(stops, vec![StopReason::AutoMeetingEnded]);
    }

    #[test]
    fn a_contagem_se_desfaz_quando_o_app_volta_ao_microfone() {
        let config = GuardianConfig {
            auto_stop: true,
            ..GuardianConfig::default()
        };
        let mut cena = Cena::new(Some("ms-teams.exe"), config);
        cena.run(600, conversa(teams(60)));
        cena.run(95, silencio(vec![], Some(false)));
        assert!(matches!(
            cena.state.view(T0 + cena.t, 0),
            GuardianView::Countdown { .. }
        ));

        // Falso positivo: microfone temporariamente liberado, a chamada voltou.
        cena.run(5, conversa(teams(3)));
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::CountdownCancelled)),
            1
        );
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::AutoStop { .. })),
            0
        );
        assert_eq!(cena.state.view(T0 + cena.t, 0), GuardianView::Idle);
    }

    #[test]
    fn falso_positivo_mic_liberado_por_pouco_com_audio_voltando_nao_interrompe() {
        let config = GuardianConfig {
            auto_stop: true,
            ..GuardianConfig::default()
        };
        let mut cena = Cena::new(Some("ms-teams.exe"), config);
        cena.run(600, conversa(teams(60)));
        // O microfone some por 60 s (troca de headset) com a conversa seguindo.
        cena.run(60, conversa(vec![]));
        cena.run(600, conversa(teams(10)));
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::Suggested { .. })),
            0
        );
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::CountdownStarted { .. })),
            0
        );
    }

    #[test]
    fn reuniao_de_tres_horas_com_audio_ativo_nao_e_interrompida() {
        let config = GuardianConfig {
            auto_stop: true,
            ..GuardianConfig::default()
        };
        let mut cena = Cena::new(Some("ms-teams.exe"), config);
        cena.run(3 * 60 * 60, conversa(teams(60)));
        assert!(cena.events.iter().all(|e| !matches!(
            e,
            GuardianEvent::Suggested { .. }
                | GuardianEvent::LongPrompted { .. }
                | GuardianEvent::AutoStop { .. }
                | GuardianEvent::CountdownStarted { .. }
        )));
    }

    #[test]
    fn reuniao_longa_com_inatividade_pergunta_se_ainda_acontece() {
        let config = GuardianConfig {
            warn_on_end: false,
            ..GuardianConfig::default()
        };
        let mut cena = Cena::new(None, config);
        cena.run(2 * 60 * 60 - 600, conversa(vec![]));
        cena.run(900, silencio(vec![], None));
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::LongPrompted { .. })),
            1
        );
        cena.state.keep_recording(T0 + cena.t);
        assert_eq!(
            cena.state.next_long_prompt_ms,
            LONG_MEETING_MS + 60 * 60 * 1000
        );
    }

    #[test]
    fn reuniao_esquecida_teams_fechado_e_45_minutos_depois_sugere_corte() {
        let mut cena = Cena::new(Some("ms-teams.exe"), GuardianConfig::default());
        cena.run(40 * 60, conversa(teams(60)));
        let fim_da_chamada_ms = cena.t * 1000;
        cena.run(45 * 60, silencio(vec![], Some(false)));

        let fim = cena.state.suggested_end_ms().unwrap();
        assert!(
            (fim - fim_da_chamada_ms).abs() <= END_MARGIN_MS + 1000,
            "{fim}"
        );
        let corte = cena.state.confident_trim_end(cena.t * 1000);
        assert!(
            corte.is_some(),
            "45 min depois do Teams fechar e corte seguro"
        );
        match cena.state.view(T0 + cena.t, cena.t * 1000) {
            GuardianView::Ask { excess_ms, .. } => assert!(excess_ms >= 40 * 60 * 1000),
            outra => panic!("{outra:?}"),
        }
    }

    #[test]
    fn browser_meet_termina_e_o_chrome_continua_aberto() {
        let chrome = |s| {
            vec![MicUser {
                process: "chrome.exe".into(),
                seconds_open: s,
            }]
        };
        let mut cena = Cena::new(Some("chrome.exe"), GuardianConfig::default());
        cena.run(30 * 60, conversa(chrome(60)));
        // O Chrome segue rodando (app_running = true), mas largou o microfone.
        let view = cena.run(120, silencio(vec![], Some(true)));
        assert!(ask(&view), "{view:?}");
    }

    #[test]
    fn musica_depois_da_chamada_nao_veta_para_sempre_mas_nao_encerra_sozinha() {
        let config = GuardianConfig {
            auto_stop: true,
            ..GuardianConfig::default()
        };
        let mut cena = Cena::new(Some("ms-teams.exe"), config);
        cena.run(600, conversa(teams(60)));
        let musica = |_| Observation {
            mic_level: 1,
            system_level: 80,
            mic_alive: true,
            system_alive: true,
            app_running: Some(false),
            ..Observation::default()
        };
        cena.run(300, musica);
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::Suggested { .. })),
            0
        );
        cena.run(400, musica);
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::Suggested { .. })),
            1
        );
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::CountdownStarted { .. })),
            0
        );
    }

    #[test]
    fn silencio_com_o_app_na_chamada_so_pergunta_depois_de_vinte_minutos() {
        let config = GuardianConfig {
            auto_stop: true,
            ..GuardianConfig::default()
        };
        let mut cena = Cena::new(Some("ms-teams.exe"), config);
        cena.run(600, conversa(teams(60)));
        cena.run(19 * 60, silencio(teams(600), Some(true)));
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::Suggested { .. })),
            0
        );
        cena.run(2 * 60, silencio(teams(600), Some(true)));
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::Suggested { .. })),
            1
        );
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::CountdownStarted { .. })),
            0
        );
    }

    #[test]
    fn pausa_nao_vira_silencio() {
        let mut cena = Cena::new(None, GuardianConfig::default());
        cena.run(60, conversa(vec![]));
        cena.run(40 * 60, |_| Observation {
            paused: true,
            mic_alive: true,
            system_alive: true,
            ..Observation::default()
        });
        let view = cena.run(60, conversa(vec![]));
        assert_eq!(view, GuardianView::Idle);
        assert_eq!(
            cena.count(|e| matches!(e, GuardianEvent::Suggested { .. })),
            0
        );
    }

    #[test]
    fn gravacao_manual_adota_o_app_que_abre_o_microfone_logo_depois() {
        let mut cena = Cena::new(None, GuardianConfig::default());
        cena.run(30, silencio(vec![], None));
        cena.run(10, conversa(teams(5)));
        assert_eq!(cena.state.associated_app.as_deref(), Some("ms-teams.exe"));
    }

    #[test]
    fn sem_app_e_com_a_pessoa_longe_a_inatividade_longa_encerra() {
        let config = GuardianConfig {
            auto_stop: true,
            ..GuardianConfig::default()
        };
        let mut cena = Cena::new(None, config);
        cena.run(ADOPT_WINDOW_SECS + 60, conversa(vec![]));
        let longe = |_| Observation {
            mic_level: 1,
            system_level: 0,
            mic_alive: true,
            system_alive: true,
            locked: true,
            ..Observation::default()
        };
        cena.run(SILENCE_AUTO_SECS + COUNTDOWN_SECS + 5, longe);
        assert!(cena.events.iter().any(|e| matches!(
            e,
            GuardianEvent::AutoStop {
                reason: StopReason::AutoInactivity,
                ..
            }
        )));
    }

    #[test]
    fn microfone_em_zero_digital_com_a_chamada_ativa_avisa_uma_vez() {
        let mut cena = Cena::new(Some("ms-teams.exe"), GuardianConfig::default());
        let mudo = |_| Observation {
            mic_level: 0,
            system_level: 50,
            mic_alive: true,
            system_alive: true,
            mic_users: teams(60),
            app_running: Some(true),
            ..Observation::default()
        };
        cena.run(MIC_DEAD_SECS + 300, mudo);
        assert_eq!(cena.count(|e| matches!(e, GuardianEvent::MicSilent)), 1);
    }

    #[test]
    fn corte_pela_transcricao_so_com_dez_minutos_de_folga() {
        assert_eq!(
            trim_suggestion_from_segments([10_000, 60_000], 30 * 60 * 1000),
            Some(60_000 + END_MARGIN_MS)
        );
        assert_eq!(
            trim_suggestion_from_segments([10_000, 25 * 60 * 1000], 30 * 60 * 1000),
            None
        );
        assert_eq!(trim_suggestion_from_segments([], 30 * 60 * 1000), None);
    }
}
