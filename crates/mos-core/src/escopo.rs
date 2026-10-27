//! O que pertence a um Workspace (ADR-093).
//!
//! O Workspace e um filtro do M/OS inteiro: estar em PROSUL e ver PROSUL. A
//! pergunta "este item aparece aqui?" tem UMA resposta, e ela mora neste
//! arquivo. O front nao reimplementa a regra: ele recebe o `EscopoDoWorkspace`
//! — os ids que aparecem — e so testa pertinencia.
//!
//! # A regra
//!
//! Um item declara Workspaces de dois jeitos: o PROPRIO (a coluna da 0043, na
//! Task e na Capture) e os HERDADOS (os do Project da Task; as juncoes de
//! Project e de Resource). Num Workspace ativo W, o item aparece quando:
//!
//! - nao declara nenhum — sem Workspace aparece em todos, e e assim que nada
//!   some so por nao ter sido classificado; ou
//! - W esta entre os declarados que continuam ATIVOS.
//!
//! O arquivado nao conta. Um item cujos declarados estao todos arquivados
//! aparece so em TUDO: ele tem dono, o dono saiu de cena, e espalha-lo por
//! todos os outros seria o contrario de separar.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::{CaptureId, ProjectId, ResourceId, TaskId, WorkspaceId};

/// Os Workspaces que um item diz ter.
pub struct Declarados<'a> {
    pub proprio: Option<WorkspaceId>,
    pub herdados: &'a [WorkspaceId],
}

/// O item aparece no Workspace `alvo`? `None` e TUDO, que mostra tudo.
pub fn pertence(
    declarados: &Declarados<'_>,
    alvo: Option<WorkspaceId>,
    ativos: &HashSet<WorkspaceId>,
) -> bool {
    let Some(alvo) = alvo else {
        return true;
    };
    let mut todos = declarados
        .proprio
        .iter()
        .chain(declarados.herdados.iter())
        .peekable();
    if todos.peek().is_none() {
        return true;
    }
    todos.any(|workspace| *workspace == alvo && ativos.contains(workspace))
}

/// Tudo o que o escopo precisa ler, em lote.
///
/// Uma leitura so, e nao uma por item: o escopo e recalculado a cada
/// `data-changed`, e perguntar ao banco pelo Project de cada Task seria o N+1
/// que o quadro do Kanban ja recusou uma vez.
#[derive(Clone, Debug, Default)]
pub struct VinculosDeWorkspace {
    pub ativos: Vec<WorkspaceId>,
    /// (Task, Workspace proprio, Project).
    pub tasks: Vec<(TaskId, Option<WorkspaceId>, Option<ProjectId>)>,
    /// (Capture, Workspace proprio).
    pub captures: Vec<(CaptureId, Option<WorkspaceId>)>,
    pub projects: Vec<ProjectId>,
    pub project_workspaces: Vec<(ProjectId, WorkspaceId)>,
    pub resources: Vec<ResourceId>,
    pub resource_workspaces: Vec<(ResourceId, WorkspaceId)>,
}

/// Os ids que aparecem num Workspace.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EscopoDoWorkspace {
    pub tasks: Vec<String>,
    pub captures: Vec<String>,
    pub projects: Vec<String>,
    pub resources: Vec<String>,
}

/// Aplica `pertence` a cada item.
pub fn escopo(vinculos: &VinculosDeWorkspace, alvo: WorkspaceId) -> EscopoDoWorkspace {
    let ativos: HashSet<WorkspaceId> = vinculos.ativos.iter().copied().collect();
    let alvo = Some(alvo);

    let mut do_project: HashMap<ProjectId, Vec<WorkspaceId>> = HashMap::new();
    for (project, workspace) in &vinculos.project_workspaces {
        do_project.entry(*project).or_default().push(*workspace);
    }
    let mut do_resource: HashMap<ResourceId, Vec<WorkspaceId>> = HashMap::new();
    for (resource, workspace) in &vinculos.resource_workspaces {
        do_resource.entry(*resource).or_default().push(*workspace);
    }
    let nenhum: Vec<WorkspaceId> = Vec::new();

    EscopoDoWorkspace {
        tasks: vinculos
            .tasks
            .iter()
            .filter(|(_, proprio, project)| {
                let herdados = project
                    .and_then(|project| do_project.get(&project))
                    .unwrap_or(&nenhum);
                pertence(
                    &Declarados {
                        proprio: *proprio,
                        herdados,
                    },
                    alvo,
                    &ativos,
                )
            })
            .map(|(id, _, _)| id.to_string())
            .collect(),
        captures: vinculos
            .captures
            .iter()
            .filter(|(_, proprio)| {
                pertence(
                    &Declarados {
                        proprio: *proprio,
                        herdados: &[],
                    },
                    alvo,
                    &ativos,
                )
            })
            .map(|(id, _)| id.to_string())
            .collect(),
        projects: vinculos
            .projects
            .iter()
            .filter(|project| {
                pertence(
                    &Declarados {
                        proprio: None,
                        herdados: do_project.get(project).unwrap_or(&nenhum),
                    },
                    alvo,
                    &ativos,
                )
            })
            .map(|id| id.to_string())
            .collect(),
        resources: vinculos
            .resources
            .iter()
            .filter(|resource| {
                pertence(
                    &Declarados {
                        proprio: None,
                        herdados: do_resource.get(resource).unwrap_or(&nenhum),
                    },
                    alvo,
                    &ativos,
                )
            })
            .map(|id| id.to_string())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ativos(lista: &[WorkspaceId]) -> HashSet<WorkspaceId> {
        lista.iter().copied().collect()
    }

    fn so(proprio: Option<WorkspaceId>) -> Declarados<'static> {
        Declarados {
            proprio,
            herdados: &[],
        }
    }

    #[test]
    fn tudo_ve_tudo() {
        let a = WorkspaceId::new();
        assert!(pertence(&so(None), None, &ativos(&[a])));
        assert!(pertence(&so(Some(a)), None, &ativos(&[])));
    }

    #[test]
    fn sem_declarado_aparece_em_todos() {
        let (a, b) = (WorkspaceId::new(), WorkspaceId::new());
        let vivos = ativos(&[a, b]);
        assert!(pertence(&so(None), Some(a), &vivos));
        assert!(pertence(&so(None), Some(b), &vivos));
    }

    #[test]
    fn o_proprio_decide() {
        let (a, b) = (WorkspaceId::new(), WorkspaceId::new());
        let vivos = ativos(&[a, b]);
        assert!(pertence(&so(Some(a)), Some(a), &vivos));
        assert!(!pertence(&so(Some(a)), Some(b), &vivos));
    }

    #[test]
    fn o_herdado_do_project_conta_junto_do_proprio() {
        let (a, b, c) = (WorkspaceId::new(), WorkspaceId::new(), WorkspaceId::new());
        let vivos = ativos(&[a, b, c]);
        let herdados = [b];
        let declarados = Declarados {
            proprio: Some(a),
            herdados: &herdados,
        };
        assert!(pertence(&declarados, Some(a), &vivos));
        assert!(pertence(&declarados, Some(b), &vivos));
        assert!(!pertence(&declarados, Some(c), &vivos));
    }

    #[test]
    fn o_arquivado_nao_conta() {
        let (a, b, c) = (WorkspaceId::new(), WorkspaceId::new(), WorkspaceId::new());
        let vivos = ativos(&[b, c]);
        let herdados = [b];
        let declarados = Declarados {
            proprio: Some(a),
            herdados: &herdados,
        };
        assert!(pertence(&declarados, Some(b), &vivos));
        assert!(!pertence(&declarados, Some(c), &vivos));
    }

    #[test]
    fn so_com_arquivados_fica_so_em_tudo() {
        let (a, b) = (WorkspaceId::new(), WorkspaceId::new());
        let vivos = ativos(&[b]);
        assert!(!pertence(&so(Some(a)), Some(b), &vivos));
        assert!(pertence(&so(Some(a)), None, &vivos));
    }

    #[test]
    fn o_escopo_aplica_a_regra_a_cada_tipo() {
        let (prosul, dev) = (WorkspaceId::new(), WorkspaceId::new());
        let (obra, site) = (ProjectId::new(), ProjectId::new());
        let solto = ProjectId::new();
        let (t_obra, t_site, t_prosul, t_solta) =
            (TaskId::new(), TaskId::new(), TaskId::new(), TaskId::new());
        let (c_prosul, c_dev, c_solta) = (CaptureId::new(), CaptureId::new(), CaptureId::new());
        let (r_prosul, r_solto) = (ResourceId::new(), ResourceId::new());

        let vinculos = VinculosDeWorkspace {
            ativos: vec![prosul, dev],
            tasks: vec![
                (t_obra, None, Some(obra)),
                (t_site, None, Some(site)),
                (t_prosul, Some(prosul), None),
                (t_solta, None, None),
            ],
            captures: vec![
                (c_prosul, Some(prosul)),
                (c_dev, Some(dev)),
                (c_solta, None),
            ],
            projects: vec![obra, site, solto],
            project_workspaces: vec![(obra, prosul), (site, dev)],
            resources: vec![r_prosul, r_solto],
            resource_workspaces: vec![(r_prosul, prosul)],
        };

        let visto = escopo(&vinculos, prosul);
        let ids = |lista: &[String]| lista.iter().cloned().collect::<HashSet<_>>();
        let esperado = |lista: &[String]| lista.iter().cloned().collect::<HashSet<_>>();
        assert_eq!(
            ids(&visto.tasks),
            esperado(&[
                t_obra.to_string(),
                t_prosul.to_string(),
                t_solta.to_string()
            ])
        );
        assert_eq!(
            ids(&visto.captures),
            esperado(&[c_prosul.to_string(), c_solta.to_string()])
        );
        assert_eq!(
            ids(&visto.projects),
            esperado(&[obra.to_string(), solto.to_string()])
        );
        assert_eq!(
            ids(&visto.resources),
            esperado(&[r_prosul.to_string(), r_solto.to_string()])
        );
    }
}
