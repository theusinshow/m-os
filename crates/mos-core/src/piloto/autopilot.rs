//! O Autopilot: o observador que junta os motores num so olhar.
//!
//! Ele nao inventa regra — chama o Attention Engine, o Next Action, o
//! Planner e o Rescue sobre o MESMO retrato e devolve o [`Panorama`] que a Home
//! desenha inteira numa chamada so. E o mesmo panorama que o laco de fundo le
//! para decidir avisos, e que o Hermes le no preambulo.
//!
//! # Acoes seguras e acoes sugeridas
//!
//! O Autopilot AUTOMATIZA o que nao destroi nada e SUGERE o resto:
//!
//! | acao | modo |
//! | --- | --- |
//! | montar a proposta do dia | automatica (nada e gravado ate o clique) |
//! | encerrar o dia de ontem que ficou aberto | sugerida (um clique) |
//! | mover Task planejada para amanha no encerramento | sugerida, marcada por default |
//! | reagendar Task vencida | sugerida |
//! | arquivar Capture velha no resgate | sugerida |
//! | avisar | automatica, com a politica anti-spam |
//!
//! O que muda dado de verdade passa sempre pela pessoa.

use serde::{Deserialize, Serialize};

use super::{
    atencao::{compor_atencao, resumir, ItemDeAtencao, ResumoDeAtencao},
    planejador::{agenda_do_dia, propor_dia, saudacao, LinhaDaAgenda, PropostaDoDia},
    proximo::{recomendar, Recomendacao},
    resgate::{detectar, Ausencia},
    Retrato, SyncNoRetrato,
};
use crate::{ObjectiveStatus, SessionStatus, TaskState};

/// Em que estado o dia esta, para a Home. Espelha `SessionStatus` com o caso
/// "ontem aberto" que a tela ja distinguia em `daily.ts`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EstadoDoDia {
    NotStarted,
    /// Hoje nao comecou e um dia anterior ficou aberto.
    StaleOpen {
        day: String,
        pendentes: usize,
    },
    Active {
        started_at: String,
        feitos: usize,
        total: usize,
    },
    Ended {
        ended_at: String,
        feitos: usize,
        total: usize,
    },
}

/// A Task ativa agora, para o indicador global.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskAtiva {
    pub task_id: String,
    pub titulo: String,
    pub started_at: String,
    pub minutos: i64,
}

/// Os numeros de "Hoje".
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hoje {
    pub concluidas: usize,
    pub restantes: usize,
    /// 0..=100
    pub progresso: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Panorama {
    pub day: String,
    pub saudacao: String,
    pub estado_do_dia: EstadoDoDia,
    pub hoje: Hoje,
    pub agora: Recomendacao,
    pub task_ativa: Option<TaskAtiva>,
    pub proximos: Vec<LinhaDaAgenda>,
    pub atencao: Vec<ItemDeAtencao>,
    pub resumo_de_atencao: ResumoDeAtencao,
    /// Preenchido quando o dia nao comecou: a proposta pronta para um clique.
    pub proposta: Option<PropostaDoDia>,
    pub resgate: Option<Ausencia>,
    pub sync: SyncNoRetrato,
    /// "Nada precisa da sua atencao agora. Proximo compromisso as 16:30."
    pub vazio: Option<String>,
}

/// Quantos proximos compromissos a Home mostra.
pub const PROXIMOS_NA_HOME: usize = 3;

pub fn estado_do_dia(r: &Retrato<'_>) -> EstadoDoDia {
    let Some(daily) = r.daily else {
        return EstadoDoDia::NotStarted;
    };
    let (feitos, total) = daily.progress();
    match daily.status {
        SessionStatus::NotStarted => match &daily.stale {
            Some(s) => EstadoDoDia::StaleOpen {
                day: s.day.as_str().to_owned(),
                pendentes: daily
                    .stale_objectives
                    .iter()
                    .filter(|o| o.status == ObjectiveStatus::Pending)
                    .count(),
            },
            None => EstadoDoDia::NotStarted,
        },
        SessionStatus::Active => EstadoDoDia::Active {
            started_at: daily
                .session
                .as_ref()
                .and_then(|s| {
                    s.started_at
                        .format(&time::format_description::well_known::Rfc3339)
                        .ok()
                })
                .unwrap_or_default(),
            feitos,
            total,
        },
        SessionStatus::Completed => EstadoDoDia::Ended {
            ended_at: daily
                .session
                .as_ref()
                .and_then(|s| s.ended_at)
                .and_then(|e| {
                    e.format(&time::format_description::well_known::Rfc3339)
                        .ok()
                })
                .unwrap_or_default(),
            feitos,
            total,
        },
    }
}

/// Os numeros de hoje: objetivos do dia quando ha sessao; senao, as Tasks
/// planejadas para hoje.
pub fn hoje(r: &Retrato<'_>) -> Hoje {
    let dia = r.hoje();
    let (feitos, total) = match r.daily.filter(|d| d.status != SessionStatus::NotStarted) {
        Some(d) => d.progress(),
        None => {
            let planejadas: Vec<_> = r
                .tasks
                .iter()
                .filter(|t| {
                    t.lifecycle_state == crate::LifecycleState::Active
                        && t.scheduled_for.as_ref() == Some(&dia)
                })
                .collect();
            (
                planejadas
                    .iter()
                    .filter(|t| t.state == TaskState::Done)
                    .count(),
                planejadas.len(),
            )
        }
    };
    Hoje {
        concluidas: feitos,
        restantes: total.saturating_sub(feitos),
        progresso: (feitos * 100).checked_div(total).unwrap_or(0) as u8,
    }
}

pub fn task_ativa(r: &Retrato<'_>) -> Option<TaskAtiva> {
    r.tasks_abertas()
        .filter(|t| t.started_at.is_some())
        .max_by_key(|t| t.started_at)
        .map(|t| {
            let inicio = t.started_at.unwrap_or(t.updated_at);
            TaskAtiva {
                task_id: t.id.to_string(),
                titulo: t.title.clone(),
                started_at: inicio
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
                minutos: (r.now_local - inicio.to_offset(r.now_local.offset()))
                    .whole_minutes()
                    .max(0),
            }
        })
}

/// O panorama inteiro. Uma chamada, uma leitura do retrato.
pub fn panorama(r: &Retrato<'_>) -> Panorama {
    let estado = estado_do_dia(r);
    let atencao = compor_atencao(r);
    let resumo_de_atencao = resumir(&atencao);
    let agora = recomendar(r);
    let proposta = matches!(
        estado,
        EstadoDoDia::NotStarted | EstadoDoDia::StaleOpen { .. }
    )
    .then(|| propor_dia(r));
    let mut proximos = agenda_do_dia(r, &r.hoje());
    let agora_rfc = r
        .now_local
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    proximos.retain(|l| l.at > agora_rfc);
    proximos.truncate(PROXIMOS_NA_HOME);
    let vazio = if atencao.is_empty() && agora.agora.is_none() {
        agora.vazio.clone()
    } else {
        None
    };

    Panorama {
        day: r.hoje().as_str().to_owned(),
        saudacao: saudacao(r.minuto_local()).to_owned(),
        estado_do_dia: estado,
        hoje: hoje(r),
        agora,
        task_ativa: task_ativa(r),
        proximos,
        atencao,
        resumo_de_atencao,
        proposta,
        resgate: detectar(r),
        sync: r.sync.clone(),
        vazio,
    }
}

/// Mediana dos minutos de inicio das sessoes anteriores — o unico habito que
/// ja da para aprender sem modelo. `None` com menos de tres dias.
pub fn inicio_habitual(inicios_minuto: &mut [u16]) -> Option<u16> {
    if inicios_minuto.len() < 3 {
        return None;
    }
    inicios_minuto.sort_unstable();
    Some(inicios_minuto[inicios_minuto.len() / 2])
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::*;
    use super::*;
    use time::Duration;

    #[test]
    fn panorama_com_dia_nao_iniciado_traz_proposta() {
        let mut c = Cenario::default();
        c.daily = Some(nao_iniciado(crate::Day::from_local(agora())));
        let mut t = task("Revisar");
        t.due_at = Some(agora() + Duration::hours(3));
        c.tasks.push(t);
        let p = panorama(&c.retrato());
        assert_eq!(p.estado_do_dia, EstadoDoDia::NotStarted);
        assert!(p.proposta.is_some());
        assert_eq!(p.agora.agora.as_ref().unwrap().titulo, "Revisar");
        assert_eq!(p.saudacao, "Bom dia.");
    }

    #[test]
    fn panorama_com_dia_ativo_nao_traz_proposta_e_conta_progresso() {
        let mut c = Cenario::default();
        let mut dia = sessao_ativa(crate::Day::from_local(agora()));
        let sid = dia.session.as_ref().unwrap().id;
        for (i, st) in [
            ObjectiveStatus::Completed,
            ObjectiveStatus::Pending,
            ObjectiveStatus::Pending,
        ]
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
                completed_at: (*st == ObjectiveStatus::Completed).then_some(agora()),
            });
        }
        c.daily = Some(dia);
        let p = panorama(&c.retrato());
        assert!(p.proposta.is_none());
        assert_eq!(
            p.hoje,
            Hoje {
                concluidas: 1,
                restantes: 2,
                progresso: 33
            }
        );
        assert!(matches!(
            p.estado_do_dia,
            EstadoDoDia::Active {
                feitos: 1,
                total: 3,
                ..
            }
        ));
    }

    #[test]
    fn vazio_util_quando_nada_ha() {
        let mut c = Cenario::default();
        c.agenda.push(item(
            crate::CalendarKind::Meeting,
            agora()
                .replace_hour(16)
                .unwrap()
                .replace_minute(30)
                .unwrap(),
            "Reunião",
        ));
        let p = panorama(&c.retrato());
        assert_eq!(
            p.vazio.as_deref(),
            Some("Nada precisa da sua atenção agora. Próximo compromisso às 16:30.")
        );
        assert_eq!(p.proximos.len(), 1);
    }

    #[test]
    fn task_ativa_e_a_comecada_mais_recente() {
        let mut c = Cenario::default();
        let mut a = task("velha");
        a.started_at = Some(agora() - Duration::hours(3));
        let mut b = task("nova");
        b.started_at = Some(agora() - Duration::minutes(18));
        c.tasks.extend([a, b]);
        let ativa = task_ativa(&c.retrato()).unwrap();
        assert_eq!(ativa.titulo, "nova");
        assert_eq!(ativa.minutos, 18);
    }

    #[test]
    fn inicio_habitual_e_mediana_com_tres_ou_mais() {
        assert_eq!(inicio_habitual(&mut vec![500, 520]), None);
        assert_eq!(inicio_habitual(&mut vec![540, 500, 520, 900]), Some(540));
    }
}
