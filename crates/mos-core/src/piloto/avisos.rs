//! O Notification Engine: o que vale uma notificacao, e o que e spam.
//!
//! Duas metades, separadas de proposito:
//!
//! - [`candidatos`] transforma o retrato em CANDIDATOS a aviso — reuniao em 15
//!   minutos, Task planejada e nao comecada, entrega de amanha, follow-up de
//!   hoje, dia aberto a noite, sync parado. Cada um com chave de deduplicacao
//!   e as opcoes de adiar que fazem sentido para ele.
//! - [`decidir`] aplica a POLITICA: deduplicacao pela chave, cooldown por tipo,
//!   silencio noturno, teto por hora, snooze, e "ja resolvido". O que passa e
//!   entregue; o que nao passa vem com o motivo, para o log.
//!
//! O lembrete classico continua com o agendador do Attention System — este
//! motor cobre o que aquele nunca cobriu (Task, Academic, dia, sync) e nao o
//! substitui.

use serde::{Deserialize, Serialize};

use super::{
    atencao::{compor_atencao, Severidade, TipoDeAtencao},
    e_compromisso, Alvo, Retrato, SyncNoRetrato,
};
use crate::{CalendarKind, Priority, QuietHours, TaskState};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoDeAviso {
    Upcoming,
    ForgottenTask,
    Academic,
    WaitingFor,
    Sync,
    UnfinishedDay,
    DayNotStarted,
    Overdue,
}

impl TipoDeAviso {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Upcoming => "upcoming",
            Self::ForgottenTask => "forgotten_task",
            Self::Academic => "academic",
            Self::WaitingFor => "waiting_for",
            Self::Sync => "sync",
            Self::UnfinishedDay => "unfinished_day",
            Self::DayNotStarted => "day_not_started",
            Self::Overdue => "overdue",
        }
    }

    /// Quanto tempo entre dois avisos do mesmo tipo, em minutos.
    pub fn cooldown_minutos(self) -> i64 {
        match self {
            Self::Upcoming => 10,
            Self::ForgottenTask => 120,
            Self::Academic => 240,
            Self::WaitingFor => 240,
            Self::Sync => 360,
            Self::UnfinishedDay => 180,
            Self::DayNotStarted => 240,
            Self::Overdue => 180,
        }
    }

    /// Se pode furar o silencio noturno.
    pub fn urgente(self) -> bool {
        matches!(self, Self::Upcoming)
    }
}

/// As opcoes de adiar, em minutos. `None` no `ate` e "escolher horario".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpcaoDeAdiar {
    pub rotulo: String,
    pub minutos: Option<i64>,
}

fn opcoes(curtas: bool, noite: bool, amanha: bool) -> Vec<OpcaoDeAdiar> {
    let mut v = Vec::new();
    if curtas {
        v.push(OpcaoDeAdiar {
            rotulo: "10 min".into(),
            minutos: Some(10),
        });
        v.push(OpcaoDeAdiar {
            rotulo: "30 min".into(),
            minutos: Some(30),
        });
        v.push(OpcaoDeAdiar {
            rotulo: "1 hora".into(),
            minutos: Some(60),
        });
    }
    if noite {
        v.push(OpcaoDeAdiar {
            rotulo: "Hoje à noite".into(),
            minutos: Some(-1),
        });
    }
    if amanha {
        v.push(OpcaoDeAdiar {
            rotulo: "Amanhã".into(),
            minutos: Some(-2),
        });
    }
    v.push(OpcaoDeAdiar {
        rotulo: "Escolher horário".into(),
        minutos: None,
    });
    v
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidato {
    /// Deduplicacao: tipo + alvo + janela. O mesmo evento nao avisa duas vezes.
    pub chave: String,
    pub tipo: TipoDeAviso,
    pub titulo: String,
    pub corpo: String,
    pub prioridade: Priority,
    pub alvo: Alvo,
    pub adiar: Vec<OpcaoDeAdiar>,
    /// O rotulo do botao principal, quando ha um ("Fazer agora").
    pub acao_principal: String,
}

/// Minutos antes de um compromisso para avisar.
pub const ANTECEDENCIA_MINUTOS: i64 = 15;
/// A partir de que hora uma Task planejada para hoje e "esquecida".
pub const MINUTO_ESQUECIDA: u16 = 15 * 60;
/// Teto de avisos por hora, contando todos os tipos.
pub const TETO_POR_HORA: usize = 4;

fn janela_dia(r: &Retrato<'_>) -> String {
    r.hoje().as_str().to_owned()
}

/// Os candidatos a aviso, a partir do retrato. Sem politica — ver [`decidir`].
pub fn candidatos(r: &Retrato<'_>) -> Vec<Candidato> {
    let mut lista = Vec::new();
    let now = r.now_local;
    let offset = now.offset();
    let hoje = r.hoje();
    let dia = janela_dia(r);
    let minuto = r.minuto_local();

    // Upcoming: compromisso comecando em ate 15 minutos.
    for item in r.agenda.iter().filter(|i| {
        e_compromisso(i) && i.kind != CalendarKind::TaskDue && i.kind != CalendarKind::AssignmentDue
    }) {
        let local = item.at.to_offset(offset);
        let faltam = (local - now).whole_minutes();
        if (0..=ANTECEDENCIA_MINUTOS).contains(&faltam) {
            let relacionadas = r
                .tasks_abertas()
                .filter(|t| item.project_id.is_some() && t.project_id == item.project_id)
                .count();
            let corpo = if relacionadas > 0 {
                format!(
                    "Existem {relacionadas} Task{} relacionada{}.",
                    if relacionadas == 1 { "" } else { "s" },
                    if relacionadas == 1 { "" } else { "s" }
                )
            } else {
                String::new()
            };
            lista.push(Candidato {
                chave: format!("upcoming:{}:{}", item.at.unix_timestamp(), item.title),
                tipo: TipoDeAviso::Upcoming,
                titulo: format!("{} começa em {} minutos.", item.title, faltam.max(1)),
                corpo,
                prioridade: Priority::High,
                alvo: Alvo::nenhum(),
                adiar: Vec::new(),
                acao_principal: String::new(),
            });
        }
    }

    // Forgotten task: planejada para hoje, nao comecada, ja e tarde.
    if minuto >= MINUTO_ESQUECIDA {
        for t in r.tasks_abertas().filter(|t| {
            t.scheduled_for.as_ref() == Some(&hoje)
                && t.started_at.is_none()
                && t.state != TaskState::Doing
                && t.waiting_for.is_empty()
        }) {
            let est = super::estimativa_curta(t.estimate_minutes);
            lista.push(Candidato {
                chave: format!("forgotten:{}:{dia}", t.id),
                tipo: TipoDeAviso::ForgottenTask,
                titulo: t.title.clone(),
                corpo: if est.is_empty() {
                    "Você planejou fazer isso hoje e ainda não começou.".into()
                } else {
                    format!(
                        "Você planejou fazer isso hoje e ainda não começou. Estimativa: {}.",
                        est.trim_start_matches('~')
                    )
                },
                prioridade: t.priority,
                alvo: Alvo::task(t.id),
                adiar: opcoes(true, true, true),
                acao_principal: "Fazer agora".into(),
            });
        }
    }

    // O resto sai do Attention Engine, para nao haver duas regras de urgencia.
    for i in compor_atencao(r) {
        match i.tipo {
            TipoDeAtencao::Overdue if i.severidade == Severidade::Urgente => {
                lista.push(Candidato {
                    chave: format!("overdue:{}:{dia}", i.alvo.id),
                    tipo: TipoDeAviso::Overdue,
                    titulo: i.titulo.clone(),
                    corpo: format!("{}.", primeira_maiuscula(&i.razoes.join(", "))),
                    prioridade: Priority::High,
                    alvo: i.alvo.clone(),
                    adiar: opcoes(false, true, true),
                    acao_principal: "Fazer agora".into(),
                })
            }
            TipoDeAtencao::AcademicDeadline if i.severidade >= Severidade::Alta => {
                lista.push(Candidato {
                    chave: format!("academic:{}:{dia}", i.alvo.id),
                    tipo: TipoDeAviso::Academic,
                    titulo: format!(
                        "{} {}.",
                        i.titulo,
                        i.razoes.first().cloned().unwrap_or_default()
                    ),
                    corpo: "Ainda está pendente.".into(),
                    prioridade: Priority::High,
                    alvo: i.alvo.clone(),
                    adiar: opcoes(false, true, true),
                    acao_principal: "Abrir".into(),
                })
            }
            TipoDeAtencao::StaleWaitingFor if i.severidade >= Severidade::Alta => {
                let quem = i
                    .descricao
                    .trim_start_matches("Aguardando ")
                    .trim_start_matches("Cobrar ")
                    .to_owned();
                lista.push(Candidato {
                    chave: format!("waiting:{}:{dia}", i.alvo.id),
                    tipo: TipoDeAviso::WaitingFor,
                    titulo: format!("{quem} ainda não respondeu."),
                    corpo: format!("{} — você marcou follow-up para hoje.", i.titulo),
                    prioridade: Priority::Normal,
                    alvo: i.alvo.clone(),
                    adiar: opcoes(false, true, true),
                    acao_principal: "Cobrar".into(),
                });
            }
            TipoDeAtencao::UnfinishedDay
                if i.acao == super::atencao::AcaoRecomendada::EncerrarDia
                    && i.titulo.starts_with("Hora") =>
            {
                lista.push(Candidato {
                    chave: format!("unfinished_day:{dia}"),
                    tipo: TipoDeAviso::UnfinishedDay,
                    titulo: "Seu dia ainda está aberto.".into(),
                    corpo: format!("{}. Encerrar leva um clique.", i.descricao),
                    prioridade: Priority::Low,
                    alvo: Alvo::nenhum(),
                    adiar: opcoes(true, false, true),
                    acao_principal: "Encerrar dia".into(),
                })
            }
            TipoDeAtencao::DayNotStarted => lista.push(Candidato {
                chave: format!("day_not_started:{dia}"),
                tipo: TipoDeAviso::DayNotStarted,
                titulo: "Seu dia ainda não foi iniciado.".into(),
                corpo: "Posso montar tudo automaticamente.".into(),
                prioridade: Priority::Low,
                alvo: Alvo::nenhum(),
                adiar: opcoes(true, false, false),
                acao_principal: "Montar meu dia".into(),
            }),
            TipoDeAtencao::UnsyncedChanges => {
                // So o persistente: erro que exige acao, ou offline com fila
                // ha horas. Falha passageira nunca vira notificacao.
                let persistente = matches!(r.sync, SyncNoRetrato::Erro { .. })
                    || i.severidade >= Severidade::Media;
                if persistente {
                    lista.push(Candidato {
                        chave: format!("sync:{}:{dia}", if matches!(r.sync, SyncNoRetrato::Erro { .. }) { "erro" } else { "offline" }),
                        tipo: TipoDeAviso::Sync,
                        titulo: i.titulo.clone(),
                        corpo: if matches!(r.sync, SyncNoRetrato::Erro { .. }) { "Suas alterações estão salvas neste dispositivo. A sincronização precisa de você para voltar.".into() } else { i.descricao.clone() },
                        prioridade: Priority::Normal, alvo: Alvo::nenhum(),
                        adiar: opcoes(false, false, true), acao_principal: "Ver sync".into(),
                    });
                }
            }
            _ => {}
        }
    }

    lista
}

fn primeira_maiuscula(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Um aviso ja entregue, como o historico guarda.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvisoEntregue {
    pub chave: String,
    pub tipo: TipoDeAviso,
    #[serde(with = "time::serde::rfc3339")]
    pub entregue_em: time::OffsetDateTime,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub adiado_ate: Option<time::OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub resolvido_em: Option<time::OffsetDateTime>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "veredito", rename_all = "snake_case")]
pub enum Veredito {
    Entregar,
    Suprimir { motivo: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decisao {
    pub candidato: Candidato,
    pub veredito: Veredito,
}

pub struct Politica<'a> {
    pub silencio: &'a QuietHours,
    /// Se o Autopilot esta ligado. Desligado, nada e entregue.
    pub ligado: bool,
}

/// Aplica a politica aos candidatos.
///
/// A ordem das perguntas e a ordem de custo: desligado sai antes de qualquer
/// conta; deduplicacao e snooze olham o historico da propria chave; cooldown
/// olha o tipo; silencio olha a hora; o teto por hora olha o conjunto.
pub fn decidir(
    r: &Retrato<'_>,
    candidatos: Vec<Candidato>,
    historico: &[AvisoEntregue],
    politica: &Politica<'_>,
) -> Vec<Decisao> {
    let now = r.now_local;
    let minuto = r.minuto_local();
    let mut entregues_na_hora = historico
        .iter()
        .filter(|h| (now - h.entregue_em.to_offset(now.offset())).whole_minutes() < 60)
        .count();
    let mut decisoes = Vec::new();
    let mut ja_nesta_rodada: std::collections::HashSet<TipoDeAviso> =
        std::collections::HashSet::new();

    // Os mais importantes primeiro, para o teto por hora cortar o que menos importa.
    let mut candidatos = candidatos;
    candidatos.sort_by_key(|c| std::cmp::Reverse(c.prioridade));

    for c in candidatos {
        let veredito = (|| {
            if !politica.ligado {
                return Veredito::Suprimir {
                    motivo: "autopilot desligado".into(),
                };
            }
            if let Some(h) = historico.iter().find(|h| h.chave == c.chave) {
                if h.resolvido_em.is_some() {
                    return Veredito::Suprimir {
                        motivo: "já resolvido".into(),
                    };
                }
                if let Some(ate) = h.adiado_ate {
                    if ate.to_offset(now.offset()) > now {
                        return Veredito::Suprimir {
                            motivo: "adiado".into(),
                        };
                    }
                    // Snooze venceu: pode entregar de novo — mas so uma vez por
                    // vencimento, e o historico e regravado por quem entrega.
                } else {
                    return Veredito::Suprimir {
                        motivo: "já avisado".into(),
                    };
                }
            }
            let ultimo_do_tipo = historico
                .iter()
                .filter(|h| h.tipo == c.tipo)
                .map(|h| h.entregue_em)
                .max();
            if let Some(u) = ultimo_do_tipo {
                if (now - u.to_offset(now.offset())).whole_minutes() < c.tipo.cooldown_minutos() {
                    return Veredito::Suprimir {
                        motivo: format!("cooldown de {} min", c.tipo.cooldown_minutos()),
                    };
                }
            }
            if ja_nesta_rodada.contains(&c.tipo) {
                return Veredito::Suprimir {
                    motivo: "agrupado: outro do mesmo tipo já sai nesta rodada".into(),
                };
            }
            if politica.silencio.contains(minuto)
                && !(c.tipo.urgente() && politica.silencio.allow_urgent)
            {
                return Veredito::Suprimir {
                    motivo: "silêncio noturno".into(),
                };
            }
            if entregues_na_hora >= TETO_POR_HORA && !c.tipo.urgente() {
                return Veredito::Suprimir {
                    motivo: format!("teto de {TETO_POR_HORA} por hora"),
                };
            }
            Veredito::Entregar
        })();
        if veredito == Veredito::Entregar {
            entregues_na_hora += 1;
            ja_nesta_rodada.insert(c.tipo);
        }
        decisoes.push(Decisao {
            candidato: c,
            veredito,
        });
    }
    decisoes
}

/// Resolve "Hoje à noite" e "Amanhã" em instante, a partir de agora.
pub fn instante_de_adiar(
    now_local: time::OffsetDateTime,
    minutos: Option<i64>,
) -> Option<time::OffsetDateTime> {
    match minutos {
        None => None,
        Some(-1) => Some(
            now_local
                .replace_hour(20)
                .ok()?
                .replace_minute(0)
                .ok()?
                .replace_second(0)
                .ok()?,
        ),
        Some(-2) => Some(
            (now_local + time::Duration::days(1))
                .replace_hour(9)
                .ok()?
                .replace_minute(0)
                .ok()?
                .replace_second(0)
                .ok()?,
        ),
        Some(m) => Some(now_local + time::Duration::minutes(m)),
    }
}

#[cfg(test)]
// O cenario nasce padrao e cada teste muda so o que importa para ele: montar
// o `Cenario` inteiro num literal esconderia justamente essa diferenca.
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::super::fixtures::*;
    use super::*;
    use time::Duration;

    fn politica(silencio: &QuietHours) -> Politica<'_> {
        Politica {
            silencio,
            ligado: true,
        }
    }

    #[test]
    fn reuniao_em_quinze_minutos_vira_upcoming() {
        let mut c = Cenario::default();
        c.agenda.push(item(
            CalendarKind::Meeting,
            agora() + Duration::minutes(14),
            "Reunião",
        ));
        let cands = candidatos(&c.retrato());
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].tipo, TipoDeAviso::Upcoming);
        assert_eq!(cands[0].titulo, "Reunião começa em 14 minutos.");
    }

    #[test]
    fn task_planejada_e_nao_comecada_so_depois_das_quinze() {
        let mut c = Cenario::default();
        let mut t = task("Enviar PDF");
        t.scheduled_for = Some(crate::Day::from_local(agora()));
        t.estimate_minutes = Some(10);
        c.tasks.push(t);
        assert!(candidatos(&c.retrato()).is_empty());
        c.now = agora().replace_hour(15).unwrap();
        let cands = candidatos(&c.retrato());
        assert_eq!(cands[0].tipo, TipoDeAviso::ForgottenTask);
        assert!(cands[0].corpo.contains("10 min"));
        assert_eq!(cands[0].adiar.len(), 6);
    }

    #[test]
    fn a_mesma_chave_nao_entrega_duas_vezes() {
        let mut c = Cenario::default();
        c.agenda.push(item(
            CalendarKind::Meeting,
            agora() + Duration::minutes(10),
            "Reunião",
        ));
        let q = QuietHours::default();
        let cands = candidatos(&c.retrato());
        let hist = vec![AvisoEntregue {
            chave: cands[0].chave.clone(),
            tipo: TipoDeAviso::Upcoming,
            entregue_em: agora() - Duration::minutes(2),
            adiado_ate: None,
            resolvido_em: None,
        }];
        let d = decidir(&c.retrato(), cands, &hist, &politica(&q));
        assert_eq!(
            d[0].veredito,
            Veredito::Suprimir {
                motivo: "já avisado".into()
            }
        );
    }

    #[test]
    fn cooldown_por_tipo_e_agrupamento() {
        let mut c = Cenario::default();
        c.now = agora().replace_hour(15).unwrap();
        for i in 0..3 {
            let mut t = task(&format!("t{i}"));
            t.scheduled_for = Some(crate::Day::from_local(agora()));
            c.tasks.push(t);
        }
        let q = QuietHours::default();
        let cands = candidatos(&c.retrato());
        assert_eq!(cands.len(), 3);
        let d = decidir(&c.retrato(), cands.clone(), &[], &politica(&q));
        assert_eq!(
            d.iter()
                .filter(|x| x.veredito == Veredito::Entregar)
                .count(),
            1,
            "um por tipo por rodada"
        );
        let hist = vec![AvisoEntregue {
            chave: "outra".into(),
            tipo: TipoDeAviso::ForgottenTask,
            entregue_em: c.now - Duration::minutes(30),
            adiado_ate: None,
            resolvido_em: None,
        }];
        let d = decidir(&c.retrato(), cands, &hist, &politica(&q));
        assert!(
            d.iter()
                .all(|x| matches!(x.veredito, Veredito::Suprimir { .. })),
            "cooldown de 120 min"
        );
    }

    #[test]
    fn snooze_segura_ate_vencer_e_resolvido_nunca_volta() {
        let mut c = Cenario::default();
        c.agenda.push(item(
            CalendarKind::Meeting,
            agora() + Duration::minutes(10),
            "Reunião",
        ));
        let q = QuietHours::default();
        let cands = candidatos(&c.retrato());
        let chave = cands[0].chave.clone();
        let adiado = vec![AvisoEntregue {
            chave: chave.clone(),
            tipo: TipoDeAviso::Upcoming,
            entregue_em: agora() - Duration::minutes(20),
            adiado_ate: Some(agora() + Duration::minutes(5)),
            resolvido_em: None,
        }];
        assert!(matches!(
            decidir(&c.retrato(), cands.clone(), &adiado, &politica(&q))[0].veredito,
            Veredito::Suprimir { .. }
        ));
        let vencido = vec![AvisoEntregue {
            chave: chave.clone(),
            tipo: TipoDeAviso::Upcoming,
            entregue_em: agora() - Duration::minutes(20),
            adiado_ate: Some(agora() - Duration::minutes(1)),
            resolvido_em: None,
        }];
        assert_eq!(
            decidir(&c.retrato(), cands.clone(), &vencido, &politica(&q))[0].veredito,
            Veredito::Entregar
        );
        let resolvido = vec![AvisoEntregue {
            chave,
            tipo: TipoDeAviso::Upcoming,
            entregue_em: agora() - Duration::minutes(20),
            adiado_ate: None,
            resolvido_em: Some(agora()),
        }];
        assert_eq!(
            decidir(&c.retrato(), cands, &resolvido, &politica(&q))[0].veredito,
            Veredito::Suprimir {
                motivo: "já resolvido".into()
            }
        );
    }

    #[test]
    fn silencio_noturno_segura_o_que_nao_e_urgente() {
        let mut c = Cenario::default();
        c.now = agora().replace_hour(23).unwrap();
        c.sync = SyncNoRetrato::Erro {
            pendentes: 3,
            mensagem: "401".into(),
        };
        c.agenda.push(item(
            CalendarKind::Meeting,
            c.now + Duration::minutes(10),
            "Plantão",
        ));
        let q = QuietHours {
            enabled: true,
            start_minute: 22 * 60,
            end_minute: 8 * 60,
            allow_urgent: true,
        };
        let d = decidir(&c.retrato(), candidatos(&c.retrato()), &[], &politica(&q));
        let sync = d
            .iter()
            .find(|x| x.candidato.tipo == TipoDeAviso::Sync)
            .unwrap();
        assert!(matches!(sync.veredito, Veredito::Suprimir { .. }));
        let up = d
            .iter()
            .find(|x| x.candidato.tipo == TipoDeAviso::Upcoming)
            .unwrap();
        assert_eq!(up.veredito, Veredito::Entregar);
    }

    #[test]
    fn sync_passageiro_nao_vira_aviso() {
        let mut c = Cenario::default();
        c.sync = SyncNoRetrato::Offline {
            pendentes: 1,
            desde: Some(
                (agora() - Duration::minutes(20))
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            ),
        };
        assert!(candidatos(&c.retrato()).is_empty());
    }

    #[test]
    fn autopilot_desligado_nao_entrega_nada() {
        let mut c = Cenario::default();
        c.agenda.push(item(
            CalendarKind::Meeting,
            agora() + Duration::minutes(5),
            "R",
        ));
        let q = QuietHours::default();
        let d = decidir(
            &c.retrato(),
            candidatos(&c.retrato()),
            &[],
            &Politica {
                silencio: &q,
                ligado: false,
            },
        );
        assert!(matches!(d[0].veredito, Veredito::Suprimir { .. }));
    }

    #[test]
    fn adiar_resolve_noite_e_amanha() {
        assert_eq!(
            instante_de_adiar(agora(), Some(30)).unwrap(),
            agora() + Duration::minutes(30)
        );
        assert_eq!(instante_de_adiar(agora(), Some(-1)).unwrap().hour(), 20);
        let am = instante_de_adiar(agora(), Some(-2)).unwrap();
        assert_eq!((am.day(), am.hour()), (16, 9));
        assert!(instante_de_adiar(agora(), None).is_none());
    }
}
