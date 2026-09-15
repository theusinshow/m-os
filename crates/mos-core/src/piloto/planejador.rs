//! O Daily Planner: o dia montado sozinho, para comecar e para encerrar.
//!
//! # Start My Day 2.0
//!
//! [`propor_dia`] monta uma proposta pronta — principal, secundarios, agenda e
//! os numeros do cabecalho — usando a MESMA escala do [`super::proximo`]. A
//! pessoa pode editar ou clicar "Começar meu dia" sem tocar em nada. O que sai
//! e um [`StartDayInput`] pronto para o `DailyService`, e nao um segundo
//! modelo de objetivo.
//!
//! # End My Day 2.0
//!
//! [`propor_encerramento`] le o dia como esta e propoe, com um clique: o que
//! foi concluido, o que fica aberto, o que venceu, e o que MOVER PARA AMANHA.
//! Mover muda `scheduled_for`, nunca `due_at` — o planejamento anda, o prazo
//! nao. Nada e alterado aqui; quem grava e quem tem repositorio.

use serde::{Deserialize, Serialize};

use super::{dia_seguinte, dias_entre, e_compromisso, proximo::pontuar, Retrato};
use crate::{
    CalendarKind, Day, LinkKind, ObjectiveDraft, ObjectiveResolution, ObjectiveStatus,
    StartDayInput, SUGGESTED_SECONDARIES,
};

/// Um compromisso da agenda de hoje, na forma que a tela mostra.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinhaDaAgenda {
    /// "14:00"
    pub hora: String,
    pub titulo: String,
    /// `meeting`, `reminder`, `exam`, ...
    pub tipo: String,
    pub at: String,
}

/// Um objetivo proposto, com a razao de estar aqui.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjetivoProposto {
    pub draft: ObjectiveDraft,
    pub razoes: Vec<String>,
    /// "~25 min", ou vazio.
    pub estimativa: String,
    pub projeto: String,
}

/// Os numeros do "Bom dia. Hoje voce tem:".
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContagensDoDia {
    pub compromissos: usize,
    pub tarefas_importantes: usize,
    pub vencidas: usize,
    pub lembretes: usize,
    pub entregas_academicas: usize,
    pub captures_na_inbox: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropostaDoDia {
    pub day: String,
    pub saudacao: String,
    pub contagens: ContagensDoDia,
    pub principal: Option<ObjetivoProposto>,
    pub secundarios: Vec<ObjetivoProposto>,
    pub agenda: Vec<LinhaDaAgenda>,
    /// Pronto para `DailyService::start`. E a mesma coisa que os campos acima,
    /// na forma que o servico aceita — montado aqui para a tela nao remontar.
    pub input: StartDayInput,
    /// A frase curta que vai em `session.note`.
    pub nota: String,
}

pub fn saudacao(minuto: u16) -> &'static str {
    match minuto {
        0..=299 => "Boa noite.",
        300..=719 => "Bom dia.",
        720..=1079 => "Boa tarde.",
        _ => "Boa noite.",
    }
}

fn hora_curta(at: time::OffsetDateTime, offset: time::UtcOffset) -> String {
    let l = at.to_offset(offset);
    format!("{:02}:{:02}", l.hour(), l.minute())
}

fn tipo_da_linha(kind: CalendarKind) -> &'static str {
    match kind {
        CalendarKind::Meeting => "meeting",
        CalendarKind::Reminder => "reminder",
        CalendarKind::ExamScheduled => "exam",
        CalendarKind::AssignmentDue => "assignment",
        CalendarKind::AcademicPlanned => "study",
        CalendarKind::TaskDue => "task_due",
        _ => "other",
    }
}

/// A agenda de um dia, so o que e compromisso, em ordem.
pub fn agenda_do_dia(r: &Retrato<'_>, dia: &Day) -> Vec<LinhaDaAgenda> {
    let offset = r.now_local.offset();
    let mut linhas: Vec<_> = r
        .agenda
        .iter()
        .filter(|i| e_compromisso(i) && &r.dia_de(i.at) == dia)
        .map(|i| LinhaDaAgenda {
            hora: hora_curta(i.at, offset),
            titulo: i.title.clone(),
            tipo: tipo_da_linha(i.kind).to_owned(),
            at: i
                .at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        })
        .collect();
    linhas.sort_by(|a, b| a.at.cmp(&b.at));
    linhas.dedup_by(|a, b| a.at == b.at && a.titulo == b.titulo);
    linhas
}

/// Monta a proposta do dia. Nao grava nada.
pub fn propor_dia(r: &Retrato<'_>) -> PropostaDoDia {
    let hoje = r.hoje();
    let now = r.now_local;

    // Carry-over primeiro: o que ficou de ontem tem precedencia sobre o que a
    // escala escolheria, porque a pessoa JA decidiu que aquilo importava.
    let mut carregados: Vec<ObjetivoProposto> = Vec::new();
    let mut tasks_ja_propostas = std::collections::HashSet::new();
    if let Some(daily) = r.daily {
        for obj in daily
            .stale_objectives
            .iter()
            .filter(|o| o.status == ObjectiveStatus::Pending)
        {
            let (link_kind, link_id) = obj
                .link
                .as_ref()
                .map(|l| {
                    let (k, i) = l.as_columns();
                    (k.to_owned(), i.to_owned())
                })
                .unwrap_or_default();
            if let Some(tid) = obj.link.as_ref().and_then(|l| l.task_id()) {
                // A Task ja foi concluida por fora? Entao o objetivo nao volta.
                if r.tasks
                    .iter()
                    .any(|t| t.id == tid && t.state == crate::TaskState::Done)
                {
                    continue;
                }
                tasks_ja_propostas.insert(tid);
            }
            carregados.push(ObjetivoProposto {
                draft: ObjectiveDraft {
                    title: obj.title.clone(),
                    description: obj.description.clone(),
                    link_kind,
                    link_id,
                    carried_from: obj.id.to_string(),
                },
                razoes: vec!["ficou de um dia anterior".into()],
                estimativa: String::new(),
                projeto: String::new(),
            });
        }
    }

    // Depois a escala: as Tasks que mais pesam hoje.
    let mut candidatas: Vec<_> = r
        .tasks
        .iter()
        .filter(|t| !tasks_ja_propostas.contains(&t.id))
        .filter_map(|t| pontuar(r, t))
        .collect();
    candidatas.sort_by(|a, b| b.pontos.cmp(&a.pontos).then(a.titulo.cmp(&b.titulo)));

    let mut propostos: Vec<ObjetivoProposto> = carregados;
    for c in candidatas.into_iter().take(1 + SUGGESTED_SECONDARIES) {
        if propostos.len() > SUGGESTED_SECONDARIES {
            break;
        }
        propostos.push(ObjetivoProposto {
            draft: ObjectiveDraft {
                title: c.titulo.clone(),
                description: String::new(),
                link_kind: LinkKind::Task.as_str().to_owned(),
                link_id: c.task_id.clone(),
                carried_from: String::new(),
            },
            razoes: c.razoes.clone(),
            estimativa: c.estimativa.clone(),
            projeto: c.projeto.clone(),
        });
    }

    let mut iter = propostos.into_iter();
    let principal = iter.next();
    let secundarios: Vec<_> = iter.collect();

    // Contagens do cabecalho.
    let agenda = agenda_do_dia(r, &hoje);
    let mut contagens = ContagensDoDia {
        compromissos: agenda
            .iter()
            .filter(|l| l.tipo == "meeting" || l.tipo == "exam" || l.tipo == "study")
            .count(),
        ..Default::default()
    };
    for t in r.tasks_abertas() {
        if let Some(due) = t.due_at {
            let d = dias_entre(&hoje, &r.dia_de(due));
            if due <= now && d <= 0 {
                contagens.vencidas += 1;
                continue;
            }
            if d == 0 {
                contagens.tarefas_importantes += 1;
                continue;
            }
        }
        if t.priority >= crate::Priority::High || t.scheduled_for.as_ref() == Some(&hoje) {
            contagens.tarefas_importantes += 1;
        }
    }
    contagens.lembretes = r
        .reminders_abertos()
        .filter(|rem| {
            rem.next_due_at
                .map(|d| r.dia_de(d) == hoje)
                .unwrap_or(false)
        })
        .count();
    contagens.entregas_academicas = r
        .academic
        .iter()
        .filter(|c| {
            c.decision == crate::Decision::None
                && matches!(
                    c.horizonte,
                    crate::Horizonte::Today | crate::Horizonte::Overdue
                )
        })
        .count();
    contagens.captures_na_inbox = r.inbox.len();

    let nota = {
        let mut partes = Vec::new();
        if contagens.vencidas > 0 {
            partes.push(format!(
                "{} vencida{}",
                contagens.vencidas,
                if contagens.vencidas == 1 { "" } else { "s" }
            ));
        }
        if contagens.compromissos > 0 {
            partes.push(format!(
                "{} compromisso{}",
                contagens.compromissos,
                if contagens.compromissos == 1 { "" } else { "s" }
            ));
        }
        if contagens.entregas_academicas > 0 {
            partes.push(format!(
                "{} entrega{} da faculdade",
                contagens.entregas_academicas,
                if contagens.entregas_academicas == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }
        if partes.is_empty() {
            "Dia montado automaticamente.".to_owned()
        } else {
            format!("Montado automaticamente: {}.", partes.join(", "))
        }
    };

    let input = StartDayInput {
        main: principal.as_ref().map(|p| p.draft.clone()),
        secondaries: secundarios.iter().map(|s| s.draft.clone()).collect(),
        note: nota.clone(),
    };

    PropostaDoDia {
        day: hoje.as_str().to_owned(),
        saudacao: saudacao(r.minuto_local()).to_owned(),
        contagens,
        principal,
        secundarios,
        agenda,
        input,
        nota,
    }
}

/// Uma Task que o encerramento propoe mover.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Movimento {
    pub task_id: String,
    pub titulo: String,
    /// `AAAA-MM-DD` de destino.
    pub para: String,
    pub razao: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropostaDeEncerramento {
    pub day: String,
    pub concluidas: usize,
    pub abertas: usize,
    pub vencidas: usize,
    /// Objetivos pendentes → `carried_over`. Pronto para `EndDayInput`.
    pub resolutions: Vec<ObjectiveResolution>,
    /// Tasks planejadas para hoje e nao concluidas → amanha.
    pub mover: Vec<Movimento>,
    /// Frase do cartao: "Mover 2 para amanhã".
    pub sugestao: String,
}

/// Le o dia e propoe o encerramento. Nao grava nada.
pub fn propor_encerramento(r: &Retrato<'_>) -> PropostaDeEncerramento {
    let hoje = r.hoje();
    let amanha = dia_seguinte(&hoje);
    let now = r.now_local;
    let mut p = PropostaDeEncerramento {
        day: hoje.as_str().to_owned(),
        ..Default::default()
    };

    if let Some(daily) = r.daily {
        let (feitos, _) = daily.progress();
        p.concluidas = feitos;
        for obj in &daily.objectives {
            if obj.status == ObjectiveStatus::Pending {
                p.abertas += 1;
                p.resolutions.push(ObjectiveResolution {
                    objective_id: obj.id.to_string(),
                    status: ObjectiveStatus::CarriedOver.as_str().to_owned(),
                });
            }
        }
    }

    for t in r.tasks_abertas() {
        if let Some(due) = t.due_at {
            if due <= now {
                p.vencidas += 1;
            }
        }
        let planejada_hoje_ou_antes = t
            .scheduled_for
            .as_ref()
            .map(|d| dias_entre(d, &hoje) >= 0)
            .unwrap_or(false);
        if planejada_hoje_ou_antes {
            p.mover.push(Movimento {
                task_id: t.id.to_string(),
                titulo: t.title.clone(),
                para: amanha.as_str().to_owned(),
                razao: if t.started_at.is_some() {
                    "começada e não concluída".into()
                } else {
                    "planejada para hoje".into()
                },
            });
        }
    }

    let n = p.mover.len();
    p.sugestao = match n {
        0 if p.abertas == 0 => "Tudo resolvido.".into(),
        0 => format!(
            "{} objetivo{} passa{} para amanhã",
            p.abertas,
            if p.abertas == 1 { "" } else { "s" },
            if p.abertas == 1 { "" } else { "m" }
        ),
        1 => "Mover 1 para amanhã".into(),
        _ => format!("Mover {n} para amanhã"),
    };
    p
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::*;
    use super::*;
    use crate::CalendarKind;
    use time::Duration;

    #[test]
    fn a_proposta_tem_principal_secundarios_e_agenda() {
        let mut c = Cenario::default();
        let mut a = task("Revisar Caixa 01");
        a.due_at = Some(agora() + Duration::hours(4));
        a.estimate_minutes = Some(25);
        let mut b = task("Enviar arquivos");
        b.priority = crate::Priority::High;
        let s = task("Solta");
        c.tasks.extend([s, b, a]);
        c.agenda.push(item(
            CalendarKind::Meeting,
            agora().replace_hour(14).unwrap(),
            "reunião",
        ));
        c.agenda.push(item(
            CalendarKind::TaskDone,
            agora().replace_hour(9).unwrap(),
            "rastro",
        ));
        let p = propor_dia(&c.retrato());
        assert_eq!(p.saudacao, "Bom dia.");
        assert_eq!(
            p.principal.as_ref().unwrap().draft.title,
            "Revisar Caixa 01"
        );
        assert_eq!(p.principal.as_ref().unwrap().estimativa, "~25 min");
        assert_eq!(p.secundarios.len(), 2);
        assert_eq!(p.agenda.len(), 1, "rastro não é agenda");
        assert_eq!(p.agenda[0].hora, "14:00");
        assert_eq!(p.contagens.compromissos, 1);
        assert_eq!(p.contagens.tarefas_importantes, 2);
        assert_eq!(
            p.input.main.as_ref().unwrap().link_id,
            p.principal.unwrap().draft.link_id
        );
    }

    #[test]
    fn carry_over_vem_antes_e_nao_repete_task() {
        let mut c = Cenario::default();
        let t = task("De ontem");
        let mut dia = nao_iniciado(crate::Day::from_local(agora()));
        let ontem = crate::Day::from_local(agora() - Duration::days(1));
        let mut stale = sessao_ativa(ontem.clone());
        let sessao = stale.session.take().unwrap();
        dia.stale = Some(sessao.clone());
        dia.stale_objectives.push(crate::DailyObjective {
            id: crate::DailyObjectiveId::new(),
            session_id: sessao.id,
            title: "De ontem".into(),
            description: String::new(),
            link: Some(
                crate::ObjectiveLink::new(crate::LinkKind::Task, &t.id.to_string()).unwrap(),
            ),
            priority: crate::ObjectivePriority::Main,
            status: ObjectiveStatus::Pending,
            position: 0,
            carried_from: None,
            created_at: agora(),
            updated_at: agora(),
            completed_at: None,
        });
        c.daily = Some(dia);
        c.tasks.push(t);
        let p = propor_dia(&c.retrato());
        assert_eq!(p.principal.as_ref().unwrap().draft.title, "De ontem");
        assert!(!p.principal.as_ref().unwrap().draft.carried_from.is_empty());
        assert!(p.secundarios.is_empty(), "a mesma task não vira secundário");
    }

    #[test]
    fn sem_nada_a_proposta_e_vazia_mas_valida() {
        let c = Cenario::default();
        let p = propor_dia(&c.retrato());
        assert!(p.principal.is_none());
        assert!(p.input.main.is_none());
        assert_eq!(p.nota, "Dia montado automaticamente.");
    }

    #[test]
    fn muitas_tasks_cabem_em_quatro() {
        let mut c = Cenario::default();
        for i in 0..20 {
            let mut t = task(&format!("t{i}"));
            t.due_at = Some(agora() + Duration::days(i));
            c.tasks.push(t);
        }
        let p = propor_dia(&c.retrato());
        assert_eq!(1 + p.secundarios.len(), 1 + SUGGESTED_SECONDARIES);
    }

    #[test]
    fn encerramento_move_planejadas_e_carrega_objetivos_sem_tocar_prazo() {
        let mut c = Cenario::default();
        let hoje = crate::Day::from_local(agora());
        let mut a = task("planejada");
        a.scheduled_for = Some(hoje.clone());
        a.due_at = Some(agora() + Duration::days(3));
        let mut b = task("feita");
        b.scheduled_for = Some(hoje.clone());
        b.state = crate::TaskState::Done;
        let mut v = task("vencida");
        v.due_at = Some(agora() - Duration::days(1));
        c.tasks.extend([a, b, v]);
        let mut dia = sessao_ativa(hoje.clone());
        let sid = dia.session.as_ref().unwrap().id;
        for (i, st) in [ObjectiveStatus::Completed, ObjectiveStatus::Pending]
            .iter()
            .enumerate()
        {
            dia.objectives.push(crate::DailyObjective {
                id: crate::DailyObjectiveId::new(),
                session_id: sid,
                title: format!("o{i}"),
                description: String::new(),
                link: None,
                priority: crate::ObjectivePriority::Secondary,
                status: *st,
                position: i as i64,
                carried_from: None,
                created_at: agora(),
                updated_at: agora(),
                completed_at: if *st == ObjectiveStatus::Completed {
                    Some(agora())
                } else {
                    None
                },
            });
        }
        c.daily = Some(dia);
        let p = propor_encerramento(&c.retrato());
        assert_eq!(p.concluidas, 1);
        assert_eq!(p.abertas, 1);
        assert_eq!(p.vencidas, 1);
        assert_eq!(p.mover.len(), 1);
        assert_eq!(p.mover[0].para, "2026-09-16");
        assert_eq!(p.resolutions[0].status, "carried_over");
        assert_eq!(p.sugestao, "Mover 1 para amanhã");
    }
}
