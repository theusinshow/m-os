//! A triagem da Inbox sem administração (F6 do MOS-OVERHAUL-PLAN, ADR-084).
//!
//! Captura digitada não recebia leitura nenhuma: a voz tinha `understand`, o
//! texto não. Aqui a MESMA leitura determinística da voz — verbo, quando, qual
//! projeto, hesitação — vira uma sugestão com motivo, e um link vira
//! referência. Sem LLM, sem rede.
//!
//! A política é a do plano §11: **nada é aplicado sozinho.** A sugestão aparece
//! pronta, a pessoa confirma uma ou um lote, e cada item volta com resultado e
//! caminho de volta. Hesitação ("talvez", "acho que") fica sem sugestão de
//! ação — não confundir linguagem natural com autorização.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{Capture, Confidence, CoreError, ProjectHint, Servicos, UndoStep, VoiceAction};

/// O que a sugestão propõe fazer com a Capture.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "acao", rename_all = "snake_case")]
pub enum AcaoDeTriagem {
    /// Tem um endereço: guardar como referência.
    Referencia { url: String, titulo: String },
    /// Tem verbo de trabalho.
    Task {
        titulo: String,
        #[serde(default)]
        project_id: Option<String>,
    },
    /// Tem verbo e QUANDO: Task com lembrete na hora dita.
    TaskComLembrete {
        titulo: String,
        #[serde(default)]
        project_id: Option<String>,
        #[serde(with = "time::serde::rfc3339")]
        quando: OffsetDateTime,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SugestaoDeTriagem {
    pub capture_id: String,
    pub acao: AcaoDeTriagem,
    pub confianca: Confidence,
    /// Por que esta sugestão, em frases curtas. Nunca vazio.
    pub razoes: Vec<String>,
}

/// O primeiro endereço http(s) do texto, sem a pontuação que o cerca.
pub fn endereco_em(texto: &str) -> Option<String> {
    texto.split_whitespace().find_map(|bruto| {
        let palavra = bruto
            .trim_start_matches(['(', '[', '<', '"', '\''])
            .trim_end_matches(['.', ',', ';', ':', ')', ']', '>', '"', '\'']);
        (palavra.starts_with("http://") || palavra.starts_with("https://"))
            .then(|| palavra.to_owned())
    })
}

/// Um título a partir do conteúdo cru: a primeira linha, até 80 caracteres.
pub fn titulo_curto(conteudo: &str) -> String {
    let primeira = conteudo.lines().next().unwrap_or("").trim();
    if primeira.chars().count() <= 80 {
        return primeira.to_owned();
    }
    let cortado: String = primeira.chars().take(79).collect();
    format!("{}…", cortado.trim_end())
}

/// Lê uma Capture e propõe um destino, ou nada.
///
/// `None` é resposta legítima e comum: a maioria das capturas é nota, e
/// inventar uma ação para cada uma seria ruído disfarçado de ajuda.
pub fn sugerir(
    capture: &Capture,
    now_local: OffsetDateTime,
    projetos: &[ProjectHint],
) -> Option<SugestaoDeTriagem> {
    if let Some(url) = endereco_em(&capture.content) {
        let sem_link = capture.content.replace(&url, " ");
        let titulo = titulo_curto(
            sem_link
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .trim_end_matches(['.', ',', ';', ':', '-', '(', ')'])
                .trim(),
        );
        return Some(SugestaoDeTriagem {
            capture_id: capture.id.to_string(),
            acao: AcaoDeTriagem::Referencia {
                titulo: if titulo.is_empty() {
                    url.clone()
                } else {
                    titulo
                },
                url,
            },
            confianca: Confidence::High,
            razoes: vec!["tem um link".to_owned()],
        });
    }
    let lido = crate::understand(
        &capture.content,
        now_local,
        crate::VoiceContext::default(),
        projetos,
    );
    if lido.hedged || lido.action == VoiceAction::Keep {
        return None;
    }
    let mut razoes = Vec::new();
    let project_id = lido.project_id.map(|id| id.to_string());
    if let Some(nome) = lido
        .project_id
        .and_then(|id| projetos.iter().find(|p| p.id == id))
        .map(|p| p.name.clone())
    {
        razoes.push(format!("cita {nome}"));
    }
    let acao = match (lido.action, lido.when) {
        (VoiceAction::CreateTaskWithReminder, Some(quando)) if quando > now_local => {
            razoes.insert(0, format!("diz quando: \"{}\"", lido.when_raw));
            AcaoDeTriagem::TaskComLembrete {
                titulo: lido.title.clone(),
                project_id,
                quando,
            }
        }
        _ => {
            razoes.insert(0, "descreve algo a fazer".to_owned());
            AcaoDeTriagem::Task {
                titulo: lido.title.clone(),
                project_id,
            }
        }
    };
    Some(SugestaoDeTriagem {
        capture_id: capture.id.to_string(),
        acao,
        confianca: lido.confidence,
        razoes,
    })
}

/// O desfecho de um item do lote.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultadoDaTriagem {
    pub capture_id: String,
    pub ok: bool,
    pub mensagem: String,
    pub undo: Option<UndoStep>,
}

/// Aplica as sugestões CONFIRMADAS, uma a uma.
///
/// Cada item é independente: um que falha (a Capture já foi convertida em
/// outro aparelho, o lembrete caiu no passado) não desfaz os outros, e volta
/// dizendo por quê. Repetir o lote não duplica: converter uma Capture que já
/// tem Task é recusado pelo repositório.
pub fn aplicar(servicos: &Servicos, itens: Vec<SugestaoDeTriagem>) -> Vec<ResultadoDaTriagem> {
    itens
        .into_iter()
        .map(|item| {
            let capture_id = item.capture_id.clone();
            match aplicar_um(servicos, item) {
                Ok((mensagem, undo)) => ResultadoDaTriagem {
                    capture_id,
                    ok: true,
                    mensagem,
                    undo,
                },
                Err(erro) => ResultadoDaTriagem {
                    capture_id,
                    ok: false,
                    mensagem: erro.message,
                    undo: None,
                },
            }
        })
        .collect()
}

fn aplicar_um(
    s: &Servicos,
    item: SugestaoDeTriagem,
) -> Result<(String, Option<UndoStep>), CoreError> {
    let parse_projeto =
        |id: &Option<String>| id.as_deref().map(crate::ProjectId::parse).transpose();
    match item.acao {
        AcaoDeTriagem::Referencia { url, titulo } => {
            let recurso = s.memory.create_resource(crate::CreateResourceInput {
                kind: crate::ResourceKind::Site,
                title: titulo,
                url,
                note: String::new(),
                source_capture_id: Some(item.capture_id.clone()),
            })?;
            Ok((
                format!("Referência guardada: {}", recurso.title),
                Some(UndoStep::ArchiveResource {
                    id: recurso.id.to_string(),
                }),
            ))
        }
        AcaoDeTriagem::Task { titulo, project_id } => {
            let (task, _) = s.work.create_task_from_capture_with_reminder(
                &item.capture_id,
                &titulo,
                "",
                parse_projeto(&project_id)?,
                None,
            )?;
            Ok((
                format!("Virou Task: {}", task.title),
                Some(UndoStep::UndoCaptureToTask {
                    capture_id: item.capture_id,
                    task_id: task.id.to_string(),
                }),
            ))
        }
        AcaoDeTriagem::TaskComLembrete {
            titulo,
            project_id,
            quando,
        } => {
            let lembrete =
                s.attention
                    .draft_at(&titulo, "", quando, crate::ReminderSource::Capture)?;
            let (task, lembrete) = s.work.create_task_from_capture_with_reminder(
                &item.capture_id,
                &titulo,
                "",
                parse_projeto(&project_id)?,
                Some(lembrete),
            )?;
            Ok((
                format!("Virou Task com lembrete: {}", task.title),
                // Mesma inversa da voz: arquiva a Task, cancela o lembrete e
                // devolve a Capture à Inbox.
                Some(UndoStep::UndoVoiceAction {
                    capture_id: item.capture_id,
                    task_id: task.id.to_string(),
                    reminder_id: lembrete.map(|r| r.id.to_string()),
                }),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CaptureId, CaptureSource, LifecycleState, ProcessingState, ProjectId};
    use time::macros::datetime;

    fn captura(texto: &str) -> Capture {
        Capture {
            id: CaptureId::new(),
            content: texto.to_owned(),
            source: CaptureSource::QuickCapture,
            processing_state: ProcessingState::Inbox,
            lifecycle_state: LifecycleState::Active,
            captured_at: datetime!(2026-09-24 10:00 UTC),
            updated_at: datetime!(2026-09-24 10:00 UTC),
        }
    }

    const AGORA: OffsetDateTime = datetime!(2026-09-24 10:00 -03:00);

    #[test]
    fn link_vira_referencia_com_o_texto_como_titulo() {
        let s = sugerir(
            &captura("manual do Allplan https://allplan.com/doc."),
            AGORA,
            &[],
        )
        .unwrap();
        assert_eq!(
            s.acao,
            AcaoDeTriagem::Referencia {
                url: "https://allplan.com/doc".into(),
                titulo: "manual do Allplan".into()
            }
        );
    }

    #[test]
    fn nota_solta_nao_ganha_sugestao() {
        assert!(sugerir(&captura("ideia: blocos de concreto celular"), AGORA, &[]).is_none());
    }

    #[test]
    fn hesitacao_nao_vira_acao() {
        assert!(sugerir(
            &captura("talvez eu devesse revisar o memorial amanhã"),
            AGORA,
            &[]
        )
        .is_none());
    }

    #[test]
    fn verbo_com_quando_vira_task_com_lembrete_e_diz_por_que() {
        let projetos = [ProjectHint {
            id: ProjectId::new(),
            name: "Jaboticatuba".into(),
        }];
        let s = sugerir(
            &captura("enviar as bases de Jaboticatuba amanhã às 9h"),
            AGORA,
            &projetos,
        )
        .unwrap();
        match &s.acao {
            AcaoDeTriagem::TaskComLembrete {
                project_id, quando, ..
            } => {
                assert_eq!(
                    project_id.as_deref(),
                    Some(projetos[0].id.to_string().as_str())
                );
                assert!(*quando > AGORA);
            }
            outra => panic!("esperava Task com lembrete, veio {outra:?}"),
        }
        assert!(s.razoes[0].starts_with("diz quando"));
        assert!(s.razoes.iter().any(|r| r.contains("Jaboticatuba")));
    }
}
