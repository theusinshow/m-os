//! O Next Action Engine: "o que faco agora?" com resposta e razao.
//!
//! Deterministico e explicavel. Cada Task aberta recebe uma pontuacao a partir
//! de pesos fixos — vencida, vence hoje, prioridade, ja comecada, planejada
//! para hoje, objetivo do dia, cabe no tempo livre — e a maior vence. As razoes
//! saem junto, em texto, porque um "faca isto" sem "porque" nao merece o clique.
//!
//! Nao ha aprendizado. Quando houver sinais de habito, eles entram pelo
//! [`super::Habitos`] como mais um peso, e nada aqui precisa mudar de forma.

use serde::{Deserialize, Serialize};

use super::{dias_entre, estimativa_curta, Retrato};
use crate::{ObjectivePriority, ObjectiveStatus, Priority, Task, TaskState};

/// Uma Task pontuada, com as razoes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidata {
    pub task_id: String,
    pub titulo: String,
    /// Nome do Project, ou vazio.
    pub projeto: String,
    /// "~25 min", ou vazio.
    pub estimativa: String,
    pub estimate_minutes: Option<i64>,
    pub prioridade: String,
    /// Se ja tem `started_at`.
    pub comecada: bool,
    pub pontos: i32,
    pub razoes: Vec<String>,
}

/// A resposta inteira: a recomendada e as seguintes, para "proxima tarefa".
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recomendacao {
    pub agora: Option<Candidata>,
    pub seguintes: Vec<Candidata>,
    /// Minutos livres ate o proximo compromisso, quando ha um.
    pub minutos_livres: Option<i64>,
    /// Quando nao ha nada a recomendar, por que.
    pub vazio: Option<String>,
}

const MAX_SEGUINTES: usize = 4;

/// Pontua uma Task para AGORA. Publico porque o planejador reusa a escala.
pub fn pontuar(r: &Retrato<'_>, task: &Task) -> Option<Candidata> {
    if task.lifecycle_state != crate::LifecycleState::Active || task.state == TaskState::Done {
        return None;
    }
    // Quem espera terceiro nao e acao sua.
    if !task.waiting_for.trim().is_empty() {
        return None;
    }
    if r.bloqueada(task) {
        return None;
    }
    // Pai com subtasks abertas: o trabalho esta nos filhos.
    let tem_filhos_abertos = r.tasks_abertas().any(|t| t.parent_task_id == Some(task.id));
    if tem_filhos_abertos {
        return None;
    }

    let hoje = r.hoje();
    let now = r.now_local;
    let mut pontos = 0i32;
    let mut razoes = Vec::new();

    if task.started_at.is_some() || task.state == TaskState::Doing {
        pontos += 50;
        razoes.push("já começada".into());
    }

    if let Some(due) = task.due_at {
        let dias = dias_entre(&hoje, &r.dia_de(due));
        if due <= now && dias <= 0 {
            pontos += 60;
            razoes.push(if dias == 0 {
                "venceu hoje".into()
            } else {
                format!(
                    "vencida há {} dia{}",
                    -dias,
                    if dias == -1 { "" } else { "s" }
                )
            });
        } else if dias == 0 {
            pontos += 45;
            razoes.push("vence hoje".into());
        } else if dias == 1 {
            pontos += 25;
            razoes.push("vence amanhã".into());
        } else if dias <= 7 {
            pontos += 10;
            razoes.push(format!("vence em {dias} dias"));
        }
    }

    if let Some(planejada) = &task.scheduled_for {
        let d = dias_entre(planejada, &hoje);
        if d == 0 {
            pontos += 30;
            razoes.push("planejada para hoje".into());
        } else if d > 0 {
            pontos += 35;
            razoes.push("ficou de um dia anterior".into());
        }
    }

    match task.priority {
        Priority::Urgent => {
            pontos += 30;
            razoes.push("prioridade urgente".into());
        }
        Priority::High => {
            pontos += 18;
            razoes.push("prioridade alta".into());
        }
        Priority::Normal => {}
        Priority::Low => {
            pontos -= 10;
        }
    }

    if let Some(daily) = r.daily {
        if let Some(obj) = daily.objectives.iter().find(|o| {
            o.link.as_ref().and_then(|l| l.task_id()) == Some(task.id)
                && o.status == ObjectiveStatus::Pending
        }) {
            if obj.priority == ObjectivePriority::Main {
                pontos += 35;
                razoes.push("objetivo principal do dia".into());
            } else {
                pontos += 25;
                razoes.push("objetivo de hoje".into());
            }
        }
    }

    // Cabe no tempo livre?
    if let (Some(est), Some(livres)) = (task.estimate_minutes, r.minutos_livres()) {
        if est > 0 {
            if est <= livres {
                pontos += 10;
                razoes.push(format!(
                    "leva {} e você tem {} livres",
                    estimativa_curta(Some(est)),
                    estimativa_curta(Some(livres)).trim_start_matches('~')
                ));
            } else if livres < 120 {
                pontos -= 15;
                razoes.push(format!(
                    "não cabe nos {} min até o próximo compromisso",
                    livres
                ));
            }
        }
    } else if let Some(est) = task.estimate_minutes {
        if est > 0 {
            razoes.push(format!("leva {}", estimativa_curta(Some(est))));
        }
    }

    pontos += (task.postponed_count as i32 * 3).min(15);
    if task.postponed_count >= 3 {
        razoes.push(format!("já adiada {} vezes", task.postponed_count));
    }

    if task.state == TaskState::Inbox {
        pontos -= 5;
    }
    if task.state == TaskState::Review {
        pontos += 5;
    }

    // Academico com prazo perto, via a Task da atividade.
    if let Some(c) = r.academic.iter().find(|c| {
        c.task_id.as_deref() == Some(task.id.to_string().as_str())
            && c.decision == crate::Decision::None
    }) {
        match c.horizonte {
            crate::Horizonte::Overdue | crate::Horizonte::Today => {
                pontos += 30;
                razoes.push(format!(
                    "{} de {}",
                    if c.kind == "exam" { "prova" } else { "entrega" },
                    c.subject
                ));
            }
            crate::Horizonte::Tomorrow => {
                pontos += 20;
                razoes.push(format!("entrega de {} amanhã", c.subject));
            }
            _ => {}
        }
    }

    // Desempate por recencia: entre iguais, o que foi mexido por ultimo.
    let horas_desde = (now - task.updated_at.to_offset(now.offset()))
        .whole_hours()
        .max(0);
    pontos -= (horas_desde / 24).min(20) as i32;

    if razoes.is_empty() {
        razoes.push("a mais recente sem prazo".into());
    }

    Some(Candidata {
        task_id: task.id.to_string(),
        titulo: task.title.clone(),
        projeto: task
            .project_id
            .and_then(|p| r.projects.iter().find(|x| x.id == p))
            .map(|p| p.name.clone())
            .unwrap_or_default(),
        estimativa: estimativa_curta(task.estimate_minutes),
        estimate_minutes: task.estimate_minutes,
        prioridade: task.priority.as_str().to_owned(),
        comecada: task.started_at.is_some(),
        pontos,
        razoes,
    })
}

/// A recomendacao para agora.
pub fn recomendar(r: &Retrato<'_>) -> Recomendacao {
    let mut candidatas: Vec<Candidata> = r.tasks.iter().filter_map(|t| pontuar(r, t)).collect();
    candidatas.sort_by(|a, b| b.pontos.cmp(&a.pontos).then(a.titulo.cmp(&b.titulo)));
    let minutos_livres = r.minutos_livres();
    if candidatas.is_empty() {
        let vazio = match r.proximo_compromisso() {
            Some(c) => {
                let local = c.at.to_offset(r.now_local.offset());
                format!(
                    "Nada precisa da sua atenção agora. Próximo compromisso às {:02}:{:02}.",
                    local.hour(),
                    local.minute()
                )
            }
            None => "Nada precisa da sua atenção agora.".into(),
        };
        return Recomendacao {
            agora: None,
            seguintes: Vec::new(),
            minutos_livres,
            vazio: Some(vazio),
        };
    }
    let mut iter = candidatas.into_iter();
    let agora = iter.next();
    Recomendacao {
        agora,
        seguintes: iter.take(MAX_SEGUINTES).collect(),
        minutos_livres,
        vazio: None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::*;
    use super::*;
    use crate::CalendarKind;
    use time::Duration;

    #[test]
    fn vencida_ganha_de_planejada_que_ganha_de_solta() {
        let mut c = Cenario::default();
        let mut a = task("vencida");
        a.due_at = Some(agora() - Duration::days(1));
        let mut b = task("planejada");
        b.scheduled_for = Some(crate::Day::from_local(agora()));
        let s = task("solta");
        c.tasks.extend([s, b, a]);
        let rec = recomendar(&c.retrato());
        assert_eq!(rec.agora.unwrap().titulo, "vencida");
        assert_eq!(rec.seguintes[0].titulo, "planejada");
        assert_eq!(rec.seguintes[1].titulo, "solta");
    }

    #[test]
    fn comecada_vem_primeiro() {
        let mut c = Cenario::default();
        let mut a = task("comecada");
        a.started_at = Some(agora() - Duration::minutes(20));
        let mut b = task("vence hoje");
        b.due_at = Some(agora() + Duration::hours(5));
        c.tasks.extend([b, a]);
        let rec = recomendar(&c.retrato());
        let agora_ = rec.agora.unwrap();
        assert_eq!(agora_.titulo, "comecada");
        assert!(agora_.razoes.contains(&"já começada".to_string()));
    }

    #[test]
    fn aguardando_e_bloqueada_ficam_de_fora() {
        let mut c = Cenario::default();
        let trava = task("trava");
        let mut travada = task("travada");
        travada.blocked_by_task_id = Some(trava.id);
        let mut esperando = task("esperando");
        esperando.waiting_for = "Victor".into();
        c.tasks.extend([trava.clone(), travada, esperando]);
        let rec = recomendar(&c.retrato());
        assert_eq!(rec.agora.unwrap().titulo, "trava");
        assert!(rec.seguintes.is_empty());
    }

    #[test]
    fn cabe_no_tempo_livre_conta_e_explica() {
        let mut c = Cenario::default();
        c.agenda.push(item(
            CalendarKind::Meeting,
            agora() + Duration::minutes(45),
            "Reunião",
        ));
        let mut curta = task("curta");
        curta.estimate_minutes = Some(25);
        let mut longa = task("longa");
        longa.estimate_minutes = Some(180);
        c.tasks.extend([longa, curta]);
        let rec = recomendar(&c.retrato());
        assert_eq!(rec.minutos_livres, Some(45));
        let agora_ = rec.agora.unwrap();
        assert_eq!(agora_.titulo, "curta");
        assert!(
            agora_.razoes.iter().any(|r| r.contains("45")),
            "{:?}",
            agora_.razoes
        );
    }

    #[test]
    fn sem_tasks_diz_o_proximo_compromisso() {
        let mut c = Cenario::default();
        c.agenda.push(item(
            CalendarKind::Meeting,
            agora()
                .replace_hour(16)
                .unwrap()
                .replace_minute(30)
                .unwrap(),
            "Reunião",
        ));
        let rec = recomendar(&c.retrato());
        assert!(rec.agora.is_none());
        assert_eq!(
            rec.vazio.as_deref(),
            Some("Nada precisa da sua atenção agora. Próximo compromisso às 16:30.")
        );
    }

    #[test]
    fn objetivo_principal_do_dia_pesa() {
        let mut c = Cenario::default();
        let alvo = task("objetivo");
        let outra = task("outra");
        let mut dia = sessao_ativa(crate::Day::from_local(agora()));
        dia.objectives.push(crate::DailyObjective {
            id: crate::DailyObjectiveId::new(),
            session_id: dia.session.as_ref().unwrap().id,
            title: "objetivo".into(),
            description: String::new(),
            link: Some(
                crate::ObjectiveLink::new(crate::LinkKind::Task, &alvo.id.to_string()).unwrap(),
            ),
            priority: ObjectivePriority::Main,
            status: ObjectiveStatus::Pending,
            position: 0,
            carried_from: None,
            created_at: agora(),
            updated_at: agora(),
            completed_at: None,
        });
        c.daily = Some(dia);
        c.tasks.extend([outra, alvo]);
        let rec = recomendar(&c.retrato());
        assert_eq!(rec.agora.unwrap().titulo, "objetivo");
    }
}
