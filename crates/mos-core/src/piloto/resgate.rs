//! O Rescue Mode: a pessoa sumiu por dias, e o sistema nao pode puni-la com
//! uma parede.
//!
//! O ciclo que este modulo existe para quebrar:
//!
//! ```text
//! nao usei → acumulou → ficou bagunçado → nao quero abrir → abandonei
//! ```
//!
//! [`detectar`] diz se houve ausencia; [`plano`] monta uma sequencia guiada,
//! poucos passos, poucas decisoes por passo, com a acao sugerida ja escolhida.
//! Nada e alterado aqui — quem aplica e quem tem repositorio, e aplica o que a
//! pessoa confirmou.

use serde::{Deserialize, Serialize};

use super::{
    atencao::{compor_atencao, Severidade, TipoDeAtencao},
    dia_seguinte, dias_entre, Retrato,
};

/// A partir de quantos dias sem abrir o M/OS o resgate entra.
pub const DIAS_DE_AUSENCIA: i64 = 3;
/// Acima disto, o plano deixa de propor "hoje" para tudo e espalha.
pub const DIAS_DE_AUSENCIA_LONGA: i64 = 10;
/// Quantos itens por passo, no maximo. Poucas decisoes por vez.
pub const ITENS_POR_PASSO: usize = 7;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ausencia {
    pub dias: i64,
    /// RFC3339 da ultima presenca, quando conhecida.
    pub desde: Option<String>,
    pub tarefas_atrasadas: usize,
    pub captures: usize,
    pub waiting_for: usize,
    pub deadlines_proximos: usize,
    pub lembretes_vencidos: usize,
}

/// Detecta se houve ausencia que justifique o resgate.
///
/// A referencia e a ULTIMA PRESENCA antes desta abertura (gravada por quem tem
/// disco), com o dia da ultima sessao como segunda fonte. Sem nenhuma das duas
/// nao ha o que detectar — um M/OS recem-instalado nao foi abandonado.
pub fn detectar(r: &Retrato<'_>) -> Option<Ausencia> {
    let hoje = r.hoje();
    let ultima = r.ultima_presenca.map(|p| r.dia_de(p));
    let dias = ultima.map(|d| dias_entre(&d, &hoje))?;
    if dias < DIAS_DE_AUSENCIA {
        return None;
    }
    let itens = compor_atencao(r);
    let conta = |tipo: TipoDeAtencao| itens.iter().filter(|i| i.tipo == tipo).count();
    Some(Ausencia {
        dias,
        desde: r.ultima_presenca.and_then(|p| {
            p.format(&time::format_description::well_known::Rfc3339)
                .ok()
        }),
        tarefas_atrasadas: conta(TipoDeAtencao::Overdue),
        captures: r.inbox.len(),
        waiting_for: conta(TipoDeAtencao::StaleWaitingFor),
        deadlines_proximos: conta(TipoDeAtencao::UpcomingDeadline)
            + conta(TipoDeAtencao::AcademicDeadline),
        lembretes_vencidos: conta(TipoDeAtencao::ReminderDue),
    })
}

/// O que o plano propoe fazer com um item. Poucas opcoes, uma ja escolhida.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "acao", rename_all = "snake_case")]
pub enum AcaoDeResgate {
    /// Task: planejar para um dia. Nao muda o prazo.
    Planejar { task_id: String, para: String },
    /// Task: tirar de qualquer dia; volta ao backlog do planejamento.
    Backlog { task_id: String },
    /// Task: arquivar.
    Arquivar { task_id: String },
    /// Task: concluir (ja estava feita e ninguem marcou).
    Concluir { task_id: String },
    /// Task aguardando terceiro: cobrar hoje.
    Cobrar { task_id: String },
    /// Capture: processar (abrir a Inbox naquele item).
    Processar { capture_id: String },
    /// Capture: arquivar sem processar.
    ArquivarCapture { capture_id: String },
    /// Lembrete: concluir.
    ConcluirLembrete { reminder_id: String },
    /// Lembrete: adiar para amanha.
    AdiarLembrete { reminder_id: String, para: String },
    /// Compromisso academico: abrir.
    AbrirAcademico { tipo: String, id: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDeResgate {
    pub titulo: String,
    pub descricao: String,
    pub razoes: Vec<String>,
    pub sugerida: AcaoDeResgate,
    pub alternativas: Vec<AcaoDeResgate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassoDeResgate {
    Urgente,
    Atrasadas,
    Captures,
    WaitingFor,
}

impl PassoDeResgate {
    pub fn titulo(self) -> &'static str {
        match self {
            Self::Urgente => "Urgente",
            Self::Atrasadas => "Atrasadas",
            Self::Captures => "Captures",
            Self::WaitingFor => "Waiting For",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Passo {
    pub passo: PassoDeResgate,
    pub titulo: String,
    pub itens: Vec<ItemDeResgate>,
    /// Quantos ficaram de fora deste passo por caberem so `ITENS_POR_PASSO`.
    pub restantes: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanoDeResgate {
    pub passos: Vec<Passo>,
    /// "3 coisas importantes para hoje."
    pub fecho: String,
    pub importantes_hoje: usize,
}

fn dia_mais(r: &Retrato<'_>, n: i64) -> String {
    let mut d = r.hoje();
    for _ in 0..n {
        d = dia_seguinte(&d);
    }
    d.as_str().to_owned()
}

/// Monta o plano guiado. Vazio quando nao ha nada a resgatar.
pub fn plano(r: &Retrato<'_>, ausencia: &Ausencia) -> PlanoDeResgate {
    let itens = compor_atencao(r);
    let hoje = r.hoje().as_str().to_owned();
    let amanha = dia_mais(r, 1);
    let longa = ausencia.dias >= DIAS_DE_AUSENCIA_LONGA;
    let mut passos = Vec::new();

    // 1. Urgente: o que vence hoje/amanha, e o academico pesado.
    let mut urgentes = Vec::new();
    for i in itens
        .iter()
        .filter(|i| i.severidade == Severidade::Urgente && i.tipo != TipoDeAtencao::Overdue)
    {
        let sugerida = match (&i.alvo.kind[..], &i.acao) {
            ("task", _) => AcaoDeResgate::Planejar {
                task_id: i.alvo.id.clone(),
                para: hoje.clone(),
            },
            ("reminder", _) => AcaoDeResgate::ConcluirLembrete {
                reminder_id: i.alvo.id.clone(),
            },
            (k, _) if k.starts_with("academic_") => AcaoDeResgate::AbrirAcademico {
                tipo: k.trim_start_matches("academic_").to_owned(),
                id: i.alvo.id.clone(),
            },
            _ => continue,
        };
        urgentes.push(ItemDeResgate {
            titulo: i.titulo.clone(),
            descricao: i.descricao.clone(),
            razoes: i.razoes.clone(),
            sugerida,
            alternativas: Vec::new(),
        });
    }
    empurrar(&mut passos, PassoDeResgate::Urgente, urgentes);

    // 2. Atrasadas: espalhar, e nao empilhar tudo em hoje.
    let mut atrasadas = Vec::new();
    let mut vaga = 0i64;
    for (n, i) in itens
        .iter()
        .filter(|i| i.tipo == TipoDeAtencao::Overdue)
        .enumerate()
    {
        let task_id = i.alvo.id.clone();
        // Ausencia curta: as tres primeiras hoje, o resto amanha. Longa:
        // uma por dia, para a semana nao nascer impossivel.
        let para = if longa {
            vaga += 1;
            dia_mais(r, (vaga - 1) / 2)
        } else if n < 3 {
            hoje.clone()
        } else {
            amanha.clone()
        };
        atrasadas.push(ItemDeResgate {
            titulo: i.titulo.clone(),
            descricao: i.descricao.clone(),
            razoes: i.razoes.clone(),
            sugerida: AcaoDeResgate::Planejar {
                task_id: task_id.clone(),
                para,
            },
            alternativas: vec![
                AcaoDeResgate::Concluir {
                    task_id: task_id.clone(),
                },
                AcaoDeResgate::Backlog {
                    task_id: task_id.clone(),
                },
                AcaoDeResgate::Arquivar { task_id },
            ],
        });
    }
    empurrar(&mut passos, PassoDeResgate::Atrasadas, atrasadas);

    // 3. Captures: processar as recentes; sugerir arquivar as muito velhas.
    let mut captures = Vec::new();
    let mut ordenadas: Vec<_> = r.inbox.iter().collect();
    ordenadas.sort_by_key(|c| std::cmp::Reverse(c.captured_at));
    for c in ordenadas {
        let dias = dias_entre(&r.dia_de(c.captured_at), &r.hoje());
        let velha = dias >= 30;
        let titulo: String = c
            .content
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(80)
            .collect();
        captures.push(ItemDeResgate {
            titulo,
            descricao: format!("há {dias} dia{}", if dias == 1 { "" } else { "s" }),
            razoes: vec![if velha {
                "muito antiga — provavelmente já passou".into()
            } else {
                "sem processar".into()
            }],
            sugerida: if velha {
                AcaoDeResgate::ArquivarCapture {
                    capture_id: c.id.to_string(),
                }
            } else {
                AcaoDeResgate::Processar {
                    capture_id: c.id.to_string(),
                }
            },
            alternativas: vec![if velha {
                AcaoDeResgate::Processar {
                    capture_id: c.id.to_string(),
                }
            } else {
                AcaoDeResgate::ArquivarCapture {
                    capture_id: c.id.to_string(),
                }
            }],
        });
    }
    empurrar(&mut passos, PassoDeResgate::Captures, captures);

    // 4. Waiting For: cobrar.
    let mut esperando = Vec::new();
    for i in itens
        .iter()
        .filter(|i| i.tipo == TipoDeAtencao::StaleWaitingFor)
    {
        let sugerida = match &i.alvo.kind[..] {
            "task" => AcaoDeResgate::Cobrar {
                task_id: i.alvo.id.clone(),
            },
            "reminder" => AcaoDeResgate::AdiarLembrete {
                reminder_id: i.alvo.id.clone(),
                para: amanha.clone(),
            },
            _ => continue,
        };
        esperando.push(ItemDeResgate {
            titulo: i.titulo.clone(),
            descricao: i.descricao.clone(),
            razoes: i.razoes.clone(),
            sugerida,
            alternativas: Vec::new(),
        });
    }
    empurrar(&mut passos, PassoDeResgate::WaitingFor, esperando);

    let importantes_hoje = passos
        .iter()
        .flat_map(|p| p.itens.iter())
        .filter(|i| matches!(&i.sugerida, AcaoDeResgate::Planejar { para, .. } if para == &hoje))
        .count()
        + passos
            .iter()
            .filter(|p| p.passo == PassoDeResgate::Urgente)
            .map(|p| p.itens.len())
            .sum::<usize>();

    let fecho = match importantes_hoje {
        0 => "Tudo organizado. Nada importante para hoje.".to_owned(),
        1 => "Tudo organizado. 1 coisa importante para hoje.".to_owned(),
        n => format!("Tudo organizado. {n} coisas importantes para hoje."),
    };

    PlanoDeResgate {
        passos,
        fecho,
        importantes_hoje,
    }
}

fn empurrar(passos: &mut Vec<Passo>, passo: PassoDeResgate, mut itens: Vec<ItemDeResgate>) {
    if itens.is_empty() {
        return;
    }
    let restantes = itens.len().saturating_sub(ITENS_POR_PASSO);
    itens.truncate(ITENS_POR_PASSO);
    passos.push(Passo {
        passo,
        titulo: passo.titulo().to_owned(),
        itens,
        restantes,
    });
}

#[cfg(test)]
// O cenario nasce padrao e cada teste muda so o que importa para ele: montar
// o `Cenario` inteiro num literal esconderia justamente essa diferenca.
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::super::fixtures::*;
    use super::*;
    use time::Duration;

    #[test]
    fn menos_de_tres_dias_nao_e_ausencia() {
        let mut c = Cenario::default();
        c.ultima_presenca = Some(agora() - Duration::days(2));
        assert!(detectar(&c.retrato()).is_none());
        c.ultima_presenca = None;
        assert!(
            detectar(&c.retrato()).is_none(),
            "sem referência não há abandono"
        );
    }

    #[test]
    fn ausencia_curta_conta_e_planeja_tres_para_hoje() {
        let mut c = Cenario::default();
        c.ultima_presenca = Some(agora() - Duration::days(4));
        for i in 0..5 {
            let mut t = task(&format!("atrasada {i}"));
            t.due_at = Some(agora() - Duration::days(i + 1));
            c.tasks.push(t);
        }
        let a = detectar(&c.retrato()).unwrap();
        assert_eq!(a.dias, 4);
        assert_eq!(a.tarefas_atrasadas, 5);
        let p = plano(&c.retrato(), &a);
        let atrasadas = p
            .passos
            .iter()
            .find(|p| p.passo == PassoDeResgate::Atrasadas)
            .unwrap();
        let hoje: Vec<_> = atrasadas.itens.iter().filter(|i| matches!(&i.sugerida, AcaoDeResgate::Planejar { para, .. } if para == "2026-09-15")).collect();
        assert_eq!(hoje.len(), 3);
        assert_eq!(p.importantes_hoje, 3);
        assert_eq!(p.fecho, "Tudo organizado. 3 coisas importantes para hoje.");
    }

    #[test]
    fn ausencia_longa_espalha_uma_a_cada_dois_dias() {
        let mut c = Cenario::default();
        c.ultima_presenca = Some(agora() - Duration::days(15));
        for i in 0..4 {
            let mut t = task(&format!("a{i}"));
            t.due_at = Some(agora() - Duration::days(i + 1));
            c.tasks.push(t);
        }
        let a = detectar(&c.retrato()).unwrap();
        let p = plano(&c.retrato(), &a);
        let dias: Vec<String> = p.passos[0]
            .itens
            .iter()
            .filter_map(|i| match &i.sugerida {
                AcaoDeResgate::Planejar { para, .. } => Some(para.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            dias,
            vec!["2026-09-15", "2026-09-15", "2026-09-16", "2026-09-16"]
        );
    }

    #[test]
    fn backlog_grande_cabe_em_sete_por_passo() {
        let mut c = Cenario::default();
        c.ultima_presenca = Some(agora() - Duration::days(5));
        for i in 0..12 {
            c.inbox.push(crate::Capture {
                id: crate::CaptureId::new(),
                content: format!("c{i}"),
                source: crate::CaptureSource::Home,
                captured_at: agora() - Duration::days(i * 5),
                updated_at: agora(),
                processing_state: crate::ProcessingState::Inbox,
                lifecycle_state: crate::LifecycleState::Active,
            });
        }
        let a = detectar(&c.retrato()).unwrap();
        assert_eq!(a.captures, 12);
        let p = plano(&c.retrato(), &a);
        let caps = p
            .passos
            .iter()
            .find(|p| p.passo == PassoDeResgate::Captures)
            .unwrap();
        assert_eq!(caps.itens.len(), ITENS_POR_PASSO);
        assert_eq!(caps.restantes, 5);
        assert!(matches!(
            caps.itens[0].sugerida,
            AcaoDeResgate::Processar { .. }
        ));
        assert!(
            caps.itens
                .iter()
                .any(|i| matches!(i.sugerida, AcaoDeResgate::ArquivarCapture { .. })),
            "as velhas sugerem arquivar"
        );
    }
}
