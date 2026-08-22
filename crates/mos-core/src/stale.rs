//! O que esta parado ha tempo demais.
//!
//! Dominio PURO e sem persistencia nenhuma: nao ha tabela, nao ha migration e
//! nao ha operacao de sync. Obsolescencia e uma LEITURA de `updated_at`, que ja
//! existe em toda Task e todo Project — inventar estado para ela seria guardar
//! uma conta que o banco ja sabe fazer.
//!
//! **O limiar e por coluna, e nao um numero unico.** Um limiar so transformaria
//! o backlog inteiro num alerta permanente: num sistema com meses de uso o
//! backlog domina a lista e afoga o sinal. Por coluna, o resultado tipico e
//! tres paradas, e nao quarenta e sete.
//!
//! **Nao ha fuso aqui.** `updated_at` e UTC e a conta e de DURACAO, e nao de
//! data civil — diferente do `Day` da Daily Session, que precisou do offset da
//! tela porque um dia e um lugar no calendario de quem olha.

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::{LifecycleState, Project, ProjectId, Task, TaskState};

/// Quanto tempo uma Task pode ficar parada naquela coluna antes de virar
/// pergunta. `None` significa que a coluna nao tem limite — e nao que o limite
/// e infinito por descuido.
pub fn tolerancia(state: TaskState) -> Option<Duration> {
    match state {
        // Comecou e largou.
        TaskState::Doing => Some(Duration::days(7)),
        // Esperando alguem.
        TaskState::Review => Some(Duration::days(7)),
        // Foi planejada e nao andou.
        TaskState::Planned => Some(Duration::days(21)),
        // Entrou e nunca foi decidida.
        TaskState::Inbox => Some(Duration::days(14)),
        // Backlog e onde as coisas esperam; done acabou. Os dois `None` sao
        // decisao, e nao lacuna.
        TaskState::Backlog | TaskState::Done => None,
    }
}

/// Project se move em semanas, e Task em dias.
pub const TOLERANCIA_PROJECT: Duration = Duration::days(21);

/// A atividade de um Project e a atividade das Tasks dele — `max(updated_at)`,
/// caindo no campo do proprio Project so quando ele nao tem Task nenhuma.
///
/// # Por que nao `project.updated_at`
///
/// Porque aquela coluna so muda quando o Project e EDITADO: so `update_project`
/// e `set_project_lifecycle` escrevem nela. Criar Task, mover no Kanban,
/// concluir — nada disso a toca. Usa-la como sinal de obsolescencia marcaria
/// como "parado" o Project em que se trabalhou ontem, e como "vivo" o que foi
/// renomeado e abandonado.
///
/// Recebe a lista COMPLETA e filtra por dentro de proposito: quem chama ja tem
/// todas as Tasks na mao, e montar uma fatia por Project so criaria trabalho
/// para desfazer.
pub fn atividade_do_project(project: &Project, tasks: &[Task]) -> OffsetDateTime {
    tasks
        .iter()
        .filter(|task| {
            task.project_id == Some(project.id) && task.lifecycle_state == LifecycleState::Active
        })
        .map(|task| task.updated_at)
        .max()
        .unwrap_or(project.updated_at)
}

/// Quantas Tasks ativas e nao concluidas o Project tem.
///
/// Project sem trabalho aberto NAO esta travado: ele acabou e ninguem arquivou,
/// que e outra pergunta e merece outra resposta.
pub fn trabalho_aberto(project: &Project, tasks: &[Task]) -> usize {
    tasks
        .iter()
        .filter(|task| {
            task.project_id == Some(project.id)
                && task.lifecycle_state == LifecycleState::Active
                && task.state != TaskState::Done
        })
        .count()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaleKind {
    Task,
    Project,
}

impl StaleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Project => "project",
        }
    }
}

/// Uma coisa parada, ja pronta para a tela.
///
/// `context` e `state` sao Strings e nao tipos porque as duas variantes os
/// preenchem com coisas diferentes: a Task traz o nome do Project e a coluna, e
/// o Project traz "N tasks abertas" e nada. Um enum com dois formatos custaria
/// mais do que a ambiguidade que ele evitaria.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parada {
    pub kind: StaleKind,
    pub id: String,
    pub title: String,
    /// Nome do Project, para Task. "N tasks abertas", para Project. Vazio
    /// quando a Task nao tem Project.
    pub context: String,
    /// A coluna do Kanban, para Task. Vazio para Project.
    pub state: String,
    /// Dias inteiros parados.
    pub days: i64,
}

/// O que [`compose_stale`] precisa.
///
/// Estrutura em vez de quatro parametros soltos, como no `ComposeInput` do
/// calendario: duas das entradas sao colecoes, e troca-las de lugar por engano
/// compilaria sem reclamacao nenhuma.
pub struct StaleInput<'a> {
    /// O agora vem de fora para o teste poder fixa-lo. Nao ha `now_utc()` aqui
    /// dentro: funcao pura que le o relogio nao se testa.
    pub now: OffsetDateTime,
    pub tasks: &'a [Task],
    pub projects: &'a [Project],
    /// Como achar o nome de um Project. Fechamento e nao mapa pronto, pelo mesmo
    /// motivo do `calendar::ComposeInput`.
    pub project_name: &'a dyn Fn(ProjectId) -> String,
}

/// Dias inteiros entre dois instantes, nunca negativo.
///
/// Truncar para dias inteiros e o que faz a comparacao casar com o rotulo: com
/// limiar de 7, o primeiro item que aparece diz "8d". Comparar `Duration` cheia
/// deixaria entrar um item de 7 dias e uma hora exibindo "7d" — o numero na tela
/// contradiria o criterio.
fn dias_parado(agora: OffsetDateTime, desde: OffsetDateTime) -> i64 {
    (agora - desde).whole_days().max(0)
}

/// Tudo o que passou da tolerancia, do mais excedido para o menos.
pub fn compose_stale(input: StaleInput<'_>) -> Vec<Parada> {
    // A tolerancia viaja ao lado da parada porque a ordenacao precisa dela, e o
    // consumidor nao. Ela morre nesta funcao.
    let mut medidas: Vec<(Parada, i64)> = Vec::new();

    for task in input.tasks {
        if task.lifecycle_state != LifecycleState::Active {
            continue;
        }
        let Some(limite) = tolerancia(task.state) else {
            continue;
        };
        let limite = limite.whole_days();
        let dias = dias_parado(input.now, task.updated_at);
        if dias <= limite {
            continue;
        }
        medidas.push((
            Parada {
                kind: StaleKind::Task,
                id: task.id.to_string(),
                title: task.title.clone(),
                context: task.project_id.map(input.project_name).unwrap_or_default(),
                state: task.state.as_str().to_owned(),
                days: dias,
            },
            limite,
        ));
    }

    let limite_project = TOLERANCIA_PROJECT.whole_days();
    for project in input.projects {
        if project.lifecycle_state != LifecycleState::Active {
            continue;
        }
        let abertas = trabalho_aberto(project, input.tasks);
        if abertas == 0 {
            continue;
        }
        let dias = dias_parado(input.now, atividade_do_project(project, input.tasks));
        if dias <= limite_project {
            continue;
        }
        medidas.push((
            Parada {
                kind: StaleKind::Project,
                id: project.id.to_string(),
                title: project.name.clone(),
                context: if abertas == 1 {
                    "1 task aberta".to_owned()
                } else {
                    format!("{abertas} tasks abertas")
                },
                state: String::new(),
                days: dias,
            },
            limite_project,
        ));
    }

    // Excesso proporcional, por produto cruzado: `a.days / a.limite` contra
    // `b.days / b.limite` sem tocar em float. Empate cai no titulo para a lista
    // da Home nao dancar entre dois refreshes iguais.
    medidas.sort_by(|(a, limite_a), (b, limite_b)| {
        (b.days * limite_a)
            .cmp(&(a.days * limite_b))
            .then_with(|| b.days.cmp(&a.days))
            .then_with(|| a.title.cmp(&b.title))
    });

    medidas.into_iter().map(|(parada, _)| parada).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::TaskId;
    use time::macros::datetime;

    fn instante() -> OffsetDateTime {
        datetime!(2026-08-22 09:00 UTC)
    }

    fn project(nome: &str, atualizado: OffsetDateTime) -> Project {
        Project {
            id: ProjectId::new(),
            name: nome.to_owned(),
            description: String::new(),
            repository: String::new(),
            lifecycle_state: LifecycleState::Active,
            created_at: atualizado,
            updated_at: atualizado,
        }
    }

    fn task(
        titulo: &str,
        state: TaskState,
        atualizada: OffsetDateTime,
        project: Option<&Project>,
    ) -> Task {
        Task {
            id: TaskId::new(),
            title: titulo.to_owned(),
            description: String::new(),
            project_id: project.map(|alvo| alvo.id),
            source_capture_id: None,
            state,
            lifecycle_state: LifecycleState::Active,
            created_at: atualizada,
            updated_at: atualizada,
            completed_at: None,
        }
    }

    #[test]
    fn cada_coluna_tem_a_sua_tolerancia() {
        assert_eq!(tolerancia(TaskState::Doing), Some(Duration::days(7)));
        assert_eq!(tolerancia(TaskState::Review), Some(Duration::days(7)));
        assert_eq!(tolerancia(TaskState::Planned), Some(Duration::days(21)));
        assert_eq!(tolerancia(TaskState::Inbox), Some(Duration::days(14)));
    }

    /// Backlog e onde as coisas esperam, e done acabou. Nenhum dos dois pode
    /// entrar: um limiar unico afogaria o sinal com o backlog inteiro.
    #[test]
    fn backlog_e_done_nunca_entram() {
        assert_eq!(tolerancia(TaskState::Backlog), None);
        assert_eq!(tolerancia(TaskState::Done), None);
    }

    /// O defeito que a §3 do spec encontrou: `projects.updated_at` so muda
    /// quando o Project e EDITADO. Criar Task, mover no Kanban e concluir nao
    /// tocam naquela coluna. Usa-lo como sinal marcaria como parado o Project em
    /// que se trabalhou ontem.
    #[test]
    fn a_atividade_do_project_vem_da_task_mais_recente() {
        let alvo = project("Casa", instante() - Duration::days(30));
        let tasks = vec![
            task(
                "velha",
                TaskState::Doing,
                instante() - Duration::days(20),
                Some(&alvo),
            ),
            task(
                "nova",
                TaskState::Doing,
                instante() - Duration::days(2),
                Some(&alvo),
            ),
        ];
        assert_eq!(
            atividade_do_project(&alvo, &tasks),
            instante() - Duration::days(2)
        );
    }

    #[test]
    fn project_sem_task_nenhuma_cai_no_proprio_campo() {
        let alvo = project("Sozinho", instante() - Duration::days(9));
        assert_eq!(
            atividade_do_project(&alvo, &[]),
            instante() - Duration::days(9)
        );
    }

    /// A Task de outro Project nao pode dar vida a este.
    #[test]
    fn a_task_de_outro_project_nao_conta() {
        let alvo = project("Casa", instante() - Duration::days(30));
        let outro = project("Trabalho", instante() - Duration::days(30));
        let tasks = vec![task("de la", TaskState::Doing, instante(), Some(&outro))];
        assert_eq!(
            atividade_do_project(&alvo, &tasks),
            instante() - Duration::days(30)
        );
    }

    #[test]
    fn trabalho_aberto_ignora_concluida_e_arquivada() {
        let alvo = project("Casa", instante());
        let mut arquivada = task("arquivada", TaskState::Doing, instante(), Some(&alvo));
        arquivada.lifecycle_state = LifecycleState::Archived;
        let tasks = vec![
            task("aberta", TaskState::Doing, instante(), Some(&alvo)),
            task("pronta", TaskState::Done, instante(), Some(&alvo)),
            arquivada,
        ];
        assert_eq!(trabalho_aberto(&alvo, &tasks), 1);
    }
    fn entrada<'a>(
        agora: OffsetDateTime,
        tasks: &'a [Task],
        projects: &'a [Project],
    ) -> StaleInput<'a> {
        StaleInput {
            now: agora,
            tasks,
            projects,
            project_name: &|_| "Casa".to_owned(),
        }
    }

    /// A fronteira exata do limiar de 7 dias. Entra quem PASSOU dele: 7 dias
    /// cravados ainda estao dentro da tolerancia, e o primeiro rotulo que
    /// aparece e "8d".
    #[test]
    fn seis_dias_nao_entra_e_oito_entra() {
        let seis = vec![task(
            "seis",
            TaskState::Doing,
            instante() - Duration::days(6),
            None,
        )];
        assert!(compose_stale(entrada(instante(), &seis, &[])).is_empty());

        let sete = vec![task(
            "sete",
            TaskState::Doing,
            instante() - Duration::days(7),
            None,
        )];
        assert!(compose_stale(entrada(instante(), &sete, &[])).is_empty());

        let oito = vec![task(
            "oito",
            TaskState::Doing,
            instante() - Duration::days(8),
            None,
        )];
        let paradas = compose_stale(entrada(instante(), &oito, &[]));
        assert_eq!(paradas.len(), 1);
        assert_eq!(paradas[0].days, 8);
        assert_eq!(paradas[0].state, "doing");
        assert_eq!(paradas[0].kind, StaleKind::Task);
    }

    #[test]
    fn backlog_e_done_ficam_de_fora_por_mais_velhos_que_sejam() {
        let tasks = vec![
            task(
                "guardada",
                TaskState::Backlog,
                instante() - Duration::days(400),
                None,
            ),
            task(
                "pronta",
                TaskState::Done,
                instante() - Duration::days(400),
                None,
            ),
        ];
        assert!(compose_stale(entrada(instante(), &tasks, &[])).is_empty());
    }

    #[test]
    fn arquivada_e_na_lixeira_ficam_fora_de_tudo() {
        let mut arquivada = task(
            "arquivada",
            TaskState::Doing,
            instante() - Duration::days(90),
            None,
        );
        arquivada.lifecycle_state = LifecycleState::Archived;
        let mut no_lixo = task(
            "no lixo",
            TaskState::Doing,
            instante() - Duration::days(90),
            None,
        );
        no_lixo.lifecycle_state = LifecycleState::Trashed;
        let tasks = vec![arquivada, no_lixo];
        assert!(compose_stale(entrada(instante(), &tasks, &[])).is_empty());
    }

    #[test]
    fn a_task_traz_o_nome_do_project_como_contexto() {
        let alvo = project("Casa", instante());
        let tasks = vec![task(
            "pintar",
            TaskState::Doing,
            instante() - Duration::days(9),
            Some(&alvo),
        )];
        let paradas = compose_stale(entrada(instante(), &tasks, &[]));
        assert_eq!(paradas[0].context, "Casa");
    }

    #[test]
    fn task_sem_project_fica_sem_contexto_em_vez_de_texto_inventado() {
        let tasks = vec![task(
            "solta",
            TaskState::Doing,
            instante() - Duration::days(9),
            None,
        )];
        let paradas = compose_stale(entrada(instante(), &tasks, &[]));
        assert_eq!(paradas[0].context, "");
    }

    #[test]
    fn project_parado_com_trabalho_aberto_entra_com_a_contagem_no_contexto() {
        let alvo = project("Casa", instante() - Duration::days(60));
        let tasks = vec![
            // Planned tem tolerancia de 21: com 30 dias ela tambem entra, e o
            // teste confere que o Project entra ALEM dela.
            task(
                "uma",
                TaskState::Planned,
                instante() - Duration::days(30),
                Some(&alvo),
            ),
            task(
                "outra",
                TaskState::Backlog,
                instante() - Duration::days(30),
                Some(&alvo),
            ),
        ];
        let projects = vec![alvo];
        let paradas = compose_stale(entrada(instante(), &tasks, &projects));
        let parado = paradas
            .iter()
            .find(|parada| parada.kind == StaleKind::Project)
            .expect("o Project parado precisa entrar");
        assert_eq!(parado.days, 30);
        assert_eq!(parado.context, "2 tasks abertas");
        assert_eq!(parado.state, "");
    }

    #[test]
    fn uma_task_aberta_fala_no_singular() {
        let alvo = project("Casa", instante() - Duration::days(60));
        let tasks = vec![task(
            "uma",
            TaskState::Backlog,
            instante() - Duration::days(30),
            Some(&alvo),
        )];
        let projects = vec![alvo];
        let paradas = compose_stale(entrada(instante(), &tasks, &projects));
        assert_eq!(paradas[0].context, "1 task aberta");
    }

    /// Project sem trabalho aberto nao esta travado: ele acabou e ninguem
    /// arquivou, que e outra pergunta.
    #[test]
    fn project_sem_trabalho_aberto_nao_entra() {
        let alvo = project("Acabado", instante() - Duration::days(60));
        let tasks = vec![task(
            "pronta",
            TaskState::Done,
            instante() - Duration::days(60),
            Some(&alvo),
        )];
        let projects = vec![alvo];
        let paradas = compose_stale(entrada(instante(), &tasks, &projects));
        assert!(paradas
            .iter()
            .all(|parada| parada.kind != StaleKind::Project));
    }

    #[test]
    fn project_arquivado_nao_entra() {
        let mut alvo = project("Guardado", instante() - Duration::days(60));
        alvo.lifecycle_state = LifecycleState::Archived;
        let tasks = vec![task(
            "aberta",
            TaskState::Backlog,
            instante() - Duration::days(60),
            Some(&alvo),
        )];
        let projects = vec![alvo];
        assert!(compose_stale(entrada(instante(), &tasks, &projects)).is_empty());
    }

    /// A ordem e o EXCESSO PROPORCIONAL, e nao os dias crus. Uma Task 12 dias
    /// parada num limiar de 7 esta a 171%; uma 24 dias num limiar de 21 esta a
    /// 114%. Ordenar por dias colocaria a segunda primeiro, e ela e a menos
    /// urgente das duas.
    #[test]
    fn a_ordem_e_o_excesso_proporcional_e_nao_os_dias() {
        let tasks = vec![
            task(
                "planejada ha muito",
                TaskState::Planned,
                instante() - Duration::days(24),
                None,
            ),
            task(
                "largada no meio",
                TaskState::Doing,
                instante() - Duration::days(12),
                None,
            ),
        ];
        let paradas = compose_stale(entrada(instante(), &tasks, &[]));
        assert_eq!(paradas[0].title, "largada no meio");
        assert_eq!(paradas[1].title, "planejada ha muito");
    }

    /// Empate no excesso nao pode sair em ordem que muda a cada leitura: a lista
    /// da Home seria diferente a cada refresh sem nada ter mudado.
    #[test]
    fn empate_desempata_pelo_titulo_para_a_ordem_nao_dancar() {
        let tasks = vec![
            task(
                "zebra",
                TaskState::Doing,
                instante() - Duration::days(14),
                None,
            ),
            task(
                "abelha",
                TaskState::Doing,
                instante() - Duration::days(14),
                None,
            ),
        ];
        let paradas = compose_stale(entrada(instante(), &tasks, &[]));
        assert_eq!(paradas[0].title, "abelha");
        assert_eq!(paradas[1].title, "zebra");
    }

    /// Relogio para tras, ou `updated_at` no futuro, nao pode virar dias
    /// negativos entrando na lista por acidente de sinal.
    #[test]
    fn atualizada_no_futuro_nao_entra() {
        let tasks = vec![task(
            "adiantada",
            TaskState::Doing,
            instante() + Duration::days(3),
            None,
        )];
        assert!(compose_stale(entrada(instante(), &tasks, &[])).is_empty());
    }
}
