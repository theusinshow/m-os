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
}
