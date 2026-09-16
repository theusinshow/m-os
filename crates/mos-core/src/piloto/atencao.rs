//! O Attention Engine: uma lista so do que precisa da pessoa.
//!
//! Cada item carrega tipo, severidade, alvo, razao e a acao recomendada. E a
//! lista que a Home mostra em "Precisa de atencao", que o Autopilot observa, e
//! de onde o motor de avisos tira candidatos. Nenhum outro lugar do M/OS decide
//! "isto e urgente" por conta propria.
//!
//! Deduplicado por (tipo, alvo): a mesma Task vencida nao vira dois itens por
//! ter dois motivos — ela vira um item com dois motivos.

use super::{dias_entre, Alvo, Retrato, SyncNoRetrato};
use crate::{Horizonte, LifecycleState, Priority, ReminderKind, ReminderStatus};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severidade {
    Baixa,
    Media,
    Alta,
    Urgente,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoDeAtencao {
    Overdue,
    UpcomingDeadline,
    StaleWaitingFor,
    UnprocessedCapture,
    UnsyncedChanges,
    AcademicDeadline,
    UnfinishedDay,
    DayNotStarted,
    StaleTask,
    SchedulingConflict,
    ReminderDue,
    /// Reuniao pronta com acoes por revisar.
    MeetingReview,
    /// Reuniao que nao anda sozinha: transcritor ausente, sem audio, falha.
    MeetingNeedsAttention,
}

impl TipoDeAtencao {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Overdue => "overdue",
            Self::UpcomingDeadline => "upcoming_deadline",
            Self::StaleWaitingFor => "stale_waiting_for",
            Self::UnprocessedCapture => "unprocessed_capture",
            Self::UnsyncedChanges => "unsynced_changes",
            Self::AcademicDeadline => "academic_deadline",
            Self::UnfinishedDay => "unfinished_day",
            Self::DayNotStarted => "day_not_started",
            Self::StaleTask => "stale_task",
            Self::SchedulingConflict => "scheduling_conflict",
            Self::ReminderDue => "reminder_due",
            Self::MeetingReview => "meeting_review",
            Self::MeetingNeedsAttention => "meeting_needs_attention",
        }
    }
}

/// O que a tela pode oferecer com um clique. Texto no tag, para a interface
/// nunca precisar adivinhar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "acao", rename_all = "snake_case")]
pub enum AcaoRecomendada {
    ComecarTask {
        id: String,
    },
    AbrirTask {
        id: String,
    },
    ReagendarTask {
        id: String,
        para: String,
    },
    Cobrar {
        id: String,
        quem: String,
    },
    ProcessarInbox,
    AbrirSync,
    AbrirAcademico {
        tipo: String,
        id: String,
    },
    EncerrarDia,
    IniciarDia,
    AbrirLembrete {
        id: String,
    },
    /// Abre a reuniao — na revisao das acoes, quando houver.
    AbrirReuniao {
        id: String,
    },
    Nenhuma,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDeAtencao {
    pub tipo: TipoDeAtencao,
    pub severidade: Severidade,
    pub titulo: String,
    pub descricao: String,
    pub alvo: Alvo,
    /// Por que este item existe, em frases curtas. Nunca vazio.
    pub razoes: Vec<String>,
    /// Quando o motivo nasceu (RFC3339), para "ha 3 dias" na tela.
    pub desde: Option<String>,
    pub acao: AcaoRecomendada,
    /// Peso para ordenar entre severidades iguais. Maior primeiro.
    pub peso: i32,
}

fn rfc(instante: time::OffsetDateTime) -> Option<String> {
    instante
        .format(&time::format_description::well_known::Rfc3339)
        .ok()
}

/// Quantos dias sem cobrar ate um "aguardando" virar item.
pub const DIAS_ESPERANDO: i64 = 3;
/// Quantas horas uma Task comecada pode ficar sem desfecho.
pub const HORAS_COMECADA: i64 = 4;
/// Quantas horas offline com fila ate virar atencao.
pub const HORAS_OFFLINE: i64 = 2;
/// A partir de que hora (minuto local) um dia aberto pede encerramento.
pub const MINUTO_NOITE: u16 = 21 * 60;
/// A partir de que hora um dia nao iniciado pede inicio (se nada habitual).
pub const MINUTO_MANHA: u16 = 9 * 60 + 30;
/// Ate que hora ainda faz sentido pedir inicio.
pub const MINUTO_TARDE: u16 = 18 * 60;

/// Monta a lista. Ordenada por severidade, depois peso.
pub fn compor_atencao(r: &Retrato<'_>) -> Vec<ItemDeAtencao> {
    let mut itens: Vec<ItemDeAtencao> = Vec::new();
    let hoje = r.hoje();
    let now = r.now_local;
    let offset = now.offset();

    // ---- tasks ------------------------------------------------------------
    let mut task_ids_com_item = std::collections::HashSet::new();
    for task in r.tasks_abertas() {
        let mut razoes = Vec::new();
        let mut severidade = None;
        let mut tipo = None;
        let mut acao = AcaoRecomendada::AbrirTask {
            id: task.id.to_string(),
        };
        let mut desde = None;
        let mut peso = 0;

        // Aguardando alguem: e pergunta diferente de "atrasada".
        if !task.waiting_for.trim().is_empty() {
            let cobrar_venceu = task.follow_up_at.map(|f| f <= now).unwrap_or(false);
            let dias = dias_entre(&r.dia_de(task.updated_at), &hoje);
            if cobrar_venceu || (task.follow_up_at.is_none() && dias >= DIAS_ESPERANDO) {
                razoes.push(format!(
                    "aguardando {} há {} dia{}",
                    task.waiting_for.trim(),
                    dias.max(0),
                    if dias == 1 { "" } else { "s" }
                ));
                if cobrar_venceu {
                    razoes.push("o follow-up já passou".into());
                }
                tipo = Some(TipoDeAtencao::StaleWaitingFor);
                severidade = Some(if cobrar_venceu {
                    Severidade::Alta
                } else {
                    Severidade::Media
                });
                acao = AcaoRecomendada::Cobrar {
                    id: task.id.to_string(),
                    quem: task.waiting_for.trim().to_owned(),
                };
                desde = task.follow_up_at.or(Some(task.updated_at)).and_then(rfc);
                peso = dias as i32;
            }
            // Quem espera terceiro nao esta "vencida" por culpa propria.
            if let (Some(t), Some(s)) = (tipo, severidade) {
                itens.push(ItemDeAtencao {
                    tipo: t,
                    severidade: s,
                    titulo: task.title.clone(),
                    descricao: format!("Aguardando {}", task.waiting_for.trim()),
                    alvo: Alvo::task(task.id),
                    razoes,
                    desde,
                    acao,
                    peso,
                });
                task_ids_com_item.insert(task.id);
            }
            continue;
        }

        if let Some(due) = task.due_at {
            let dia_do_prazo = r.dia_de(due);
            let dias = dias_entre(&hoje, &dia_do_prazo);
            if due <= now && dias <= 0 {
                let atraso = dias_entre(&dia_do_prazo, &hoje);
                razoes.push(if atraso == 0 {
                    "venceu hoje".into()
                } else {
                    format!(
                        "vencida há {atraso} dia{}",
                        if atraso == 1 { "" } else { "s" }
                    )
                });
                tipo = Some(TipoDeAtencao::Overdue);
                severidade = Some(if atraso >= 3 || task.priority >= Priority::High {
                    Severidade::Urgente
                } else {
                    Severidade::Alta
                });
                acao = AcaoRecomendada::ComecarTask {
                    id: task.id.to_string(),
                };
                desde = rfc(due);
                peso = 100 + atraso as i32 * 5;
            } else if dias == 0 {
                razoes.push("vence hoje".into());
                tipo = Some(TipoDeAtencao::UpcomingDeadline);
                severidade = Some(Severidade::Alta);
                acao = AcaoRecomendada::ComecarTask {
                    id: task.id.to_string(),
                };
                desde = rfc(due);
                peso = 80;
            } else if dias == 1 {
                razoes.push("vence amanhã".into());
                tipo = Some(TipoDeAtencao::UpcomingDeadline);
                severidade = Some(if task.priority >= Priority::High {
                    Severidade::Alta
                } else {
                    Severidade::Media
                });
                acao = AcaoRecomendada::ComecarTask {
                    id: task.id.to_string(),
                };
                desde = rfc(due);
                peso = 60;
            }
        }

        // Comecada e esquecida.
        if tipo.is_none() {
            if let Some(inicio) = task.started_at {
                let horas = (now - inicio.to_offset(offset)).whole_hours();
                if horas >= HORAS_COMECADA {
                    razoes.push(format!("começada há {horas}h e sem desfecho"));
                    tipo = Some(TipoDeAtencao::StaleTask);
                    severidade = Some(Severidade::Media);
                    acao = AcaoRecomendada::AbrirTask {
                        id: task.id.to_string(),
                    };
                    desde = rfc(inicio);
                    peso = horas as i32;
                }
            }
        }

        // Planejada para um dia que ja passou, sem ter sido movida.
        if tipo.is_none() {
            if let Some(planejada) = &task.scheduled_for {
                let atraso = dias_entre(planejada, &hoje);
                if atraso > 0 {
                    razoes.push(format!(
                        "planejada para {} e não foi movida",
                        planejada.as_str()
                    ));
                    if task.postponed_count >= 3 {
                        razoes.push(format!("adiada {} vezes", task.postponed_count));
                    }
                    tipo = Some(TipoDeAtencao::StaleTask);
                    severidade = Some(if task.postponed_count >= 3 {
                        Severidade::Alta
                    } else {
                        Severidade::Baixa
                    });
                    acao = AcaoRecomendada::ReagendarTask {
                        id: task.id.to_string(),
                        para: hoje.as_str().to_owned(),
                    };
                    peso = atraso as i32 + task.postponed_count as i32 * 2;
                }
            }
        }

        if let (Some(t), Some(s)) = (tipo, severidade) {
            // Ja comecada: "Comecar" de novo nao faz sentido — abre.
            if task.started_at.is_some() {
                acao = AcaoRecomendada::AbrirTask {
                    id: task.id.to_string(),
                };
            }
            if task.priority == Priority::Urgent {
                razoes.push("prioridade urgente".into());
            } else if task.priority == Priority::High {
                razoes.push("prioridade alta".into());
            }
            let projeto = task
                .project_id
                .and_then(|p| r.projects.iter().find(|x| x.id == p))
                .map(|p| p.name.clone())
                .unwrap_or_default();
            itens.push(ItemDeAtencao {
                tipo: t,
                severidade: s,
                titulo: task.title.clone(),
                descricao: projeto,
                alvo: Alvo::task(task.id),
                razoes,
                desde,
                acao,
                peso,
            });
            task_ids_com_item.insert(task.id);
        }
    }

    // ---- reminders --------------------------------------------------------
    for reminder in r.reminders_abertos() {
        let Some(due) = reminder.next_due_at else {
            continue;
        };
        if due > now {
            continue;
        }
        let vencido = matches!(
            reminder.status,
            ReminderStatus::Due
                | ReminderStatus::Delivered
                | ReminderStatus::Missed
                | ReminderStatus::Scheduled
                | ReminderStatus::Acknowledged
        );
        if !vencido {
            continue;
        }
        // Um lembrete que aponta para uma Task ja listada nao vira item a mais.
        if let Some(crate::ReminderTarget::Task(id)) = reminder.target {
            if task_ids_com_item.contains(&id) {
                continue;
            }
        }
        let horas = (now - due.to_offset(offset)).whole_hours();
        let mut razoes = vec![if horas < 1 {
            "venceu agora".into()
        } else {
            format!("venceu há {horas}h")
        }];
        if reminder.snooze_count >= 3 {
            razoes.push(format!("adiado {} vezes", reminder.snooze_count));
        }
        let (tipo, descricao, acao) = if reminder.kind == ReminderKind::FollowUp {
            (
                TipoDeAtencao::StaleWaitingFor,
                format!("Cobrar {}", reminder.waiting_for),
                AcaoRecomendada::AbrirLembrete {
                    id: reminder.id.to_string(),
                },
            )
        } else {
            (
                TipoDeAtencao::ReminderDue,
                reminder.body.clone(),
                AcaoRecomendada::AbrirLembrete {
                    id: reminder.id.to_string(),
                },
            )
        };
        let severidade = match (reminder.priority, horas) {
            (Priority::Urgent, _) => Severidade::Urgente,
            (Priority::High, _) | (_, 24..) => Severidade::Alta,
            _ => Severidade::Media,
        };
        itens.push(ItemDeAtencao {
            tipo,
            severidade,
            titulo: reminder.title.clone(),
            descricao,
            alvo: Alvo::reminder(reminder.id),
            razoes,
            desde: rfc(due),
            acao,
            peso: horas as i32 + reminder.snooze_count as i32,
        });
    }

    // ---- reunioes ---------------------------------------------------------
    //
    // So duas perguntas: ha acoes esperando revisao, e ha reuniao que nao anda
    // sozinha. Processando nao e atencao — o M/OS esta cuidando, e dizer isso
    // na lista do "precisa de voce" ensinaria a pessoa a ignorar a lista.
    for reuniao in r.reunioes {
        use crate::MeetingPhase::*;
        match reuniao.fase {
            NeedsAttention | FailedRecoverable => itens.push(ItemDeAtencao {
                tipo: TipoDeAtencao::MeetingNeedsAttention,
                severidade: Severidade::Media,
                titulo: reuniao.titulo.clone(),
                descricao: reuniao.atencao.clone(),
                alvo: Alvo::meeting(&reuniao.id),
                razoes: vec![if reuniao.atencao.is_empty() {
                    "a reunião precisa de você".into()
                } else {
                    reuniao.atencao.clone()
                }],
                desde: rfc(reuniao.quando),
                acao: AcaoRecomendada::AbrirReuniao {
                    id: reuniao.id.clone(),
                },
                peso: 10,
            }),
            Ready | PartiallyReady if reuniao.acoes_pendentes > 0 => {
                let dias = dias_entre(&r.dia_de(reuniao.quando), &hoje);
                itens.push(ItemDeAtencao {
                    tipo: TipoDeAtencao::MeetingReview,
                    // Revisar logo e barato; revisar uma semana depois e
                    // reconstruir de memoria. A severidade sobe com o tempo, e
                    // nunca passa de media: ninguem se atrasa por nao revisar.
                    severidade: if dias >= 2 {
                        Severidade::Media
                    } else {
                        Severidade::Baixa
                    },
                    titulo: reuniao.titulo.clone(),
                    descricao: if reuniao.acoes_pendentes == 1 {
                        "1 ação encontrada".into()
                    } else {
                        format!("{} ações encontradas", reuniao.acoes_pendentes)
                    },
                    alvo: Alvo::meeting(&reuniao.id),
                    razoes: vec!["reunião pronta, com ações por revisar".into()],
                    desde: rfc(reuniao.quando),
                    acao: AcaoRecomendada::AbrirReuniao {
                        id: reuniao.id.clone(),
                    },
                    peso: reuniao.acoes_pendentes as i32 + dias as i32,
                });
            }
            _ => {}
        }
    }

    // ---- academico --------------------------------------------------------
    for c in r.academic {
        if c.decision != crate::Decision::None {
            continue;
        }
        // A Task que executa ja esta listada? Entao a entrega e o mesmo item.
        if let Some(tid) = c
            .task_id
            .as_deref()
            .and_then(|t| crate::TaskId::parse(t).ok())
        {
            if task_ids_com_item.contains(&tid) {
                continue;
            }
        }
        let (severidade, razao) = match c.horizonte {
            Horizonte::Overdue => (Severidade::Urgente, "prazo passou".to_owned()),
            Horizonte::Today => (Severidade::Urgente, "vence hoje".to_owned()),
            Horizonte::Tomorrow => (Severidade::Alta, "vence amanhã".to_owned()),
            Horizonte::ThisWeek => (Severidade::Media, "vence esta semana".to_owned()),
            Horizonte::Later => continue,
        };
        let o_que = if c.kind == "exam" { "prova" } else { "entrega" };
        itens.push(ItemDeAtencao {
            tipo: TipoDeAtencao::AcademicDeadline,
            severidade,
            titulo: c.title.clone(),
            descricao: format!("{o_que} · {}", c.subject),
            alvo: Alvo::academic(&c.kind, &c.id),
            razoes: vec![razao, "ainda pendente".into()],
            desde: rfc(c.at),
            acao: match c.task_id.as_ref() {
                Some(t) => AcaoRecomendada::ComecarTask { id: t.clone() },
                None => AcaoRecomendada::AbrirAcademico {
                    tipo: c.kind.clone(),
                    id: c.id.clone(),
                },
            },
            peso: match c.horizonte {
                Horizonte::Overdue => 90,
                Horizonte::Today => 85,
                Horizonte::Tomorrow => 60,
                _ => 20,
            },
        });
    }

    // ---- inbox ------------------------------------------------------------
    let inbox: Vec<_> = r
        .inbox
        .iter()
        .filter(|c| c.lifecycle_state == LifecycleState::Active)
        .collect();
    if !inbox.is_empty() {
        let mais_velha = inbox.iter().map(|c| c.captured_at).min();
        let dias = mais_velha
            .map(|m| dias_entre(&r.dia_de(m), &hoje))
            .unwrap_or(0);
        let n = inbox.len();
        itens.push(ItemDeAtencao {
            tipo: TipoDeAtencao::UnprocessedCapture,
            severidade: if n >= 10 || dias >= 7 {
                Severidade::Media
            } else {
                Severidade::Baixa
            },
            titulo: format!(
                "{n} capture{} para organizar",
                if n == 1 { "" } else { "s" }
            ),
            descricao: if dias > 0 {
                format!(
                    "a mais antiga há {dias} dia{}",
                    if dias == 1 { "" } else { "s" }
                )
            } else {
                "chegaram hoje".into()
            },
            alvo: Alvo::nenhum(),
            razoes: vec![format!("{n} na Inbox")],
            desde: mais_velha.and_then(rfc),
            acao: AcaoRecomendada::ProcessarInbox,
            peso: n as i32,
        });
    }

    // ---- sync -------------------------------------------------------------
    match r.sync {
        SyncNoRetrato::Erro {
            pendentes,
            mensagem,
        } => itens.push(ItemDeAtencao {
            tipo: TipoDeAtencao::UnsyncedChanges,
            severidade: Severidade::Alta,
            titulo: "A sincronização parou".into(),
            descricao: mensagem.clone(),
            alvo: Alvo::nenhum(),
            razoes: vec![
                format!(
                    "{pendentes} alteraç{} esperando",
                    if *pendentes == 1 { "ão" } else { "ões" }
                ),
                "precisa de você para voltar".into(),
            ],
            desde: None,
            acao: AcaoRecomendada::AbrirSync,
            peso: *pendentes as i32,
        }),
        SyncNoRetrato::Offline { pendentes, desde } if *pendentes > 0 => {
            let horas = desde
                .as_deref()
                .and_then(|d| {
                    time::OffsetDateTime::parse(d, &time::format_description::well_known::Rfc3339)
                        .ok()
                })
                .map(|d| (now - d.to_offset(offset)).whole_hours())
                .unwrap_or(0);
            if horas >= HORAS_OFFLINE {
                itens.push(ItemDeAtencao {
                    tipo: TipoDeAtencao::UnsyncedChanges,
                    severidade: Severidade::Media,
                    titulo: format!("Sem sincronizar há {horas}h"),
                    descricao: "Suas alterações estão salvas aqui e sobem quando a conexão voltar."
                        .into(),
                    alvo: Alvo::nenhum(),
                    razoes: vec![format!(
                        "{pendentes} alteraç{} na fila",
                        if *pendentes == 1 { "ão" } else { "ões" }
                    )],
                    desde: desde.clone(),
                    acao: AcaoRecomendada::AbrirSync,
                    peso: horas as i32,
                });
            }
        }
        _ => {}
    }

    // ---- o dia ------------------------------------------------------------
    if let Some(daily) = r.daily {
        if let Some(stale) = &daily.stale {
            let pendentes = daily
                .stale_objectives
                .iter()
                .filter(|o| o.status == crate::ObjectiveStatus::Pending)
                .count();
            itens.push(ItemDeAtencao {
                tipo: TipoDeAtencao::UnfinishedDay,
                severidade: Severidade::Media,
                titulo: format!("O dia {} ficou aberto", stale.day.as_str()),
                descricao: format!(
                    "{pendentes} objetivo{} sem desfecho",
                    if pendentes == 1 { "" } else { "s" }
                ),
                alvo: Alvo::nenhum(),
                razoes: vec!["encerrar mantém o histórico honesto".into()],
                desde: rfc(stale.started_at),
                acao: AcaoRecomendada::EncerrarDia,
                peso: pendentes as i32,
            });
        }
        let minuto = r.minuto_local();
        match daily.status {
            crate::SessionStatus::Active if minuto >= MINUTO_NOITE => {
                let (feitos, total) = daily.progress();
                itens.push(ItemDeAtencao {
                    tipo: TipoDeAtencao::UnfinishedDay,
                    severidade: Severidade::Baixa,
                    titulo: "Hora de encerrar o dia".into(),
                    descricao: format!(
                        "{feitos} de {total} concluído{}",
                        if total == 1 { "" } else { "s" }
                    ),
                    alvo: Alvo::nenhum(),
                    razoes: vec!["já passou das 21h".into()],
                    desde: None,
                    acao: AcaoRecomendada::EncerrarDia,
                    peso: 0,
                });
            }
            crate::SessionStatus::NotStarted if daily.stale.is_none() => {
                let limiar = r
                    .habitos
                    .inicio_habitual_minuto
                    .map(|m| m + 30)
                    .unwrap_or(MINUTO_MANHA);
                if minuto >= limiar && minuto < MINUTO_TARDE {
                    itens.push(ItemDeAtencao {
                        tipo: TipoDeAtencao::DayNotStarted,
                        severidade: Severidade::Baixa,
                        titulo: "Seu dia ainda não foi iniciado".into(),
                        descricao: "Posso montar tudo automaticamente.".into(),
                        alvo: Alvo::nenhum(),
                        razoes: vec![match r.habitos.inicio_habitual_minuto {
                            Some(m) => {
                                format!("você costuma começar às {:02}:{:02}", m / 60, m % 60)
                            }
                            None => "já passou das 9h30".into(),
                        }],
                        desde: None,
                        acao: AcaoRecomendada::IniciarDia,
                        peso: 0,
                    });
                }
            }
            _ => {}
        }
    }

    // ---- conflitos de agenda ------------------------------------------------
    let compromissos: Vec<_> = r
        .agenda
        .iter()
        .filter(|i| super::e_compromisso(i) && i.ends_at.is_some())
        .collect();
    for (a, b) in compromissos.iter().zip(compromissos.iter().skip(1)) {
        if let (Some(fim_a), true) = (a.ends_at, b.at > a.at) {
            if b.at < fim_a && r.dia_de(a.at) == hoje {
                itens.push(ItemDeAtencao {
                    tipo: TipoDeAtencao::SchedulingConflict,
                    severidade: Severidade::Media,
                    titulo: format!("{} e {} se sobrepõem", a.title, b.title),
                    descricao: String::new(),
                    alvo: Alvo::nenhum(),
                    razoes: vec!["dois compromissos no mesmo horário".into()],
                    desde: rfc(a.at),
                    acao: AcaoRecomendada::Nenhuma,
                    peso: 0,
                });
            }
        }
    }

    // ---- dedup + ordem ------------------------------------------------------
    let mut vistos = std::collections::HashSet::new();
    itens.retain(|i| i.alvo.id.is_empty() || vistos.insert((i.tipo, i.alvo.clone())));
    itens.sort_by(|a, b| {
        b.severidade
            .cmp(&a.severidade)
            .then(b.peso.cmp(&a.peso))
            .then(a.titulo.cmp(&b.titulo))
    });
    itens
}

/// Contagens curtas para o cabecalho da Home e para o Hermes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumoDeAtencao {
    pub urgentes: usize,
    pub altos: usize,
    pub total: usize,
}

pub fn resumir(itens: &[ItemDeAtencao]) -> ResumoDeAtencao {
    ResumoDeAtencao {
        urgentes: itens
            .iter()
            .filter(|i| i.severidade == Severidade::Urgente)
            .count(),
        altos: itens
            .iter()
            .filter(|i| i.severidade == Severidade::Alta)
            .count(),
        total: itens.len(),
    }
}

#[cfg(test)]
// O cenario nasce padrao e cada teste muda so o que importa para ele: montar
// o `Cenario` inteiro num literal esconderia justamente essa diferenca.
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::super::fixtures::*;
    use super::*;
    use crate::Day;
    use time::Duration;

    fn reuniao(
        fase: crate::MeetingPhase,
        acoes: usize,
        dias: i64,
    ) -> super::super::ReuniaoNoRetrato {
        super::super::ReuniaoNoRetrato {
            id: "r1".into(),
            titulo: "Revisão estrutural".into(),
            fase,
            acoes_pendentes: acoes,
            atencao: "O transcritor local não foi encontrado.".into(),
            quando: agora() - Duration::days(dias),
        }
    }

    #[test]
    fn reuniao_pronta_com_acoes_vira_revisao_e_processando_nao_aparece() {
        let mut c = Cenario::default();
        c.reunioes.push(reuniao(crate::MeetingPhase::Ready, 3, 0));
        c.reunioes
            .push(reuniao(crate::MeetingPhase::Processing, 0, 0));
        c.reunioes.push(reuniao(crate::MeetingPhase::Ready, 0, 0));
        let itens = compor_atencao(&c.retrato());
        let revisoes: Vec<_> = itens
            .iter()
            .filter(|i| i.tipo == TipoDeAtencao::MeetingReview)
            .collect();
        assert_eq!(revisoes.len(), 1);
        assert_eq!(revisoes[0].descricao, "3 ações encontradas");
        assert_eq!(revisoes[0].severidade, Severidade::Baixa);
        assert!(matches!(
            revisoes[0].acao,
            AcaoRecomendada::AbrirReuniao { .. }
        ));
        assert_eq!(itens.len(), 1);
    }

    #[test]
    fn reuniao_travada_pede_a_pessoa_com_a_frase_do_problema() {
        let mut c = Cenario::default();
        c.reunioes
            .push(reuniao(crate::MeetingPhase::NeedsAttention, 0, 1));
        let itens = compor_atencao(&c.retrato());
        assert_eq!(itens[0].tipo, TipoDeAtencao::MeetingNeedsAttention);
        assert!(itens[0].descricao.contains("transcritor"));
    }

    #[test]
    fn task_vencida_e_urgente_depois_de_tres_dias() {
        let mut c = Cenario::default();
        let mut t = task("Enviar arquivos");
        t.due_at = Some(agora() - Duration::days(4));
        c.tasks.push(t);
        let itens = compor_atencao(&c.retrato());
        assert_eq!(itens.len(), 1);
        assert_eq!(itens[0].tipo, TipoDeAtencao::Overdue);
        assert_eq!(itens[0].severidade, Severidade::Urgente);
        assert!(itens[0].razoes[0].contains("4 dias"));
    }

    #[test]
    fn vence_amanha_e_medio_sem_prioridade() {
        let mut c = Cenario::default();
        let mut t = task("Trabalho");
        t.due_at = Some(agora() + Duration::days(1));
        c.tasks.push(t);
        let itens = compor_atencao(&c.retrato());
        assert_eq!(itens[0].tipo, TipoDeAtencao::UpcomingDeadline);
        assert_eq!(itens[0].severidade, Severidade::Media);
    }

    #[test]
    fn aguardando_ha_tres_dias_vira_cobranca_e_nao_atraso() {
        let mut c = Cenario::default();
        let mut t = task("Tipos de base");
        t.waiting_for = "Victor".into();
        t.updated_at = agora() - Duration::days(3);
        t.due_at = Some(agora() - Duration::days(1));
        c.tasks.push(t);
        let itens = compor_atencao(&c.retrato());
        assert_eq!(itens.len(), 1);
        assert_eq!(itens[0].tipo, TipoDeAtencao::StaleWaitingFor);
        assert!(
            matches!(itens[0].acao, AcaoRecomendada::Cobrar { ref quem, .. } if quem == "Victor")
        );
    }

    #[test]
    fn a_mesma_task_nao_aparece_duas_vezes() {
        let mut c = Cenario::default();
        let mut t = task("X");
        t.due_at = Some(agora() - Duration::hours(1));
        t.started_at = Some(agora() - Duration::hours(6));
        c.tasks.push(t);
        let itens = compor_atencao(&c.retrato());
        assert_eq!(itens.len(), 1);
    }

    #[test]
    fn item_resolvido_some_da_lista() {
        let mut c = Cenario::default();
        let mut t = task("X");
        t.due_at = Some(agora() - Duration::hours(1));
        t.state = crate::TaskState::Done;
        c.tasks.push(t);
        assert!(compor_atencao(&c.retrato()).is_empty());
    }

    #[test]
    fn inbox_vira_um_item_so_com_contagem() {
        let mut c = Cenario::default();
        for i in 0..4 {
            c.inbox.push(crate::Capture {
                id: crate::CaptureId::new(),
                content: format!("c{i}"),
                source: crate::CaptureSource::QuickCapture,
                captured_at: agora() - Duration::days(i),
                updated_at: agora(),
                processing_state: crate::ProcessingState::Inbox,
                lifecycle_state: crate::LifecycleState::Active,
            });
        }
        let itens = compor_atencao(&c.retrato());
        assert_eq!(itens.len(), 1);
        assert_eq!(itens[0].titulo, "4 captures para organizar");
        assert_eq!(itens[0].acao, AcaoRecomendada::ProcessarInbox);
    }

    #[test]
    fn dia_nao_iniciado_so_depois_das_nove_e_meia() {
        let mut c = Cenario::default();
        c.daily = Some(nao_iniciado(Day::from_local(agora())));
        c.now = agora().replace_hour(8).unwrap();
        assert!(compor_atencao(&c.retrato()).is_empty());
        c.now = agora();
        let itens = compor_atencao(&c.retrato());
        assert_eq!(itens[0].tipo, TipoDeAtencao::DayNotStarted);
        c.now = agora().replace_hour(19).unwrap();
        assert!(
            compor_atencao(&c.retrato()).is_empty(),
            "à noite não pede início"
        );
    }

    #[test]
    fn dia_aberto_a_noite_pede_encerramento() {
        let mut c = Cenario::default();
        c.daily = Some(sessao_ativa(Day::from_local(agora())));
        c.now = agora().replace_hour(21).unwrap();
        let itens = compor_atencao(&c.retrato());
        assert_eq!(itens[0].tipo, TipoDeAtencao::UnfinishedDay);
        assert_eq!(itens[0].acao, AcaoRecomendada::EncerrarDia);
    }

    #[test]
    fn sync_offline_so_depois_de_duas_horas_com_fila() {
        let mut c = Cenario::default();
        c.sync = SyncNoRetrato::Offline {
            pendentes: 2,
            desde: Some(
                (agora() - Duration::hours(1))
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ),
        };
        assert!(compor_atencao(&c.retrato()).is_empty());
        c.sync = SyncNoRetrato::Offline {
            pendentes: 2,
            desde: Some(
                (agora() - Duration::hours(3))
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ),
        };
        assert_eq!(
            compor_atencao(&c.retrato())[0].tipo,
            TipoDeAtencao::UnsyncedChanges
        );
        c.sync = SyncNoRetrato::Erro {
            pendentes: 1,
            mensagem: "401".into(),
        };
        assert_eq!(compor_atencao(&c.retrato())[0].severidade, Severidade::Alta);
    }

    #[test]
    fn ordena_por_severidade_depois_peso() {
        let mut c = Cenario::default();
        let mut a = task("amanhã");
        a.due_at = Some(agora() + Duration::days(1));
        let mut b = task("vencida");
        b.due_at = Some(agora() - Duration::days(5));
        c.tasks.push(a);
        c.tasks.push(b);
        let itens = compor_atencao(&c.retrato());
        assert_eq!(itens[0].titulo, "vencida");
        let resumo = resumir(&itens);
        assert_eq!(resumo.urgentes, 1);
        assert_eq!(resumo.total, 2);
    }
}
