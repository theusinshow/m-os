use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionCategory {
    Capture,
    Work,
    Memory,
    App,
    Data,
    System,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionRisk {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionConfirmation {
    None,
    Explicit,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: FunctionCategory,
    pub risk: FunctionRisk,
    pub confirmation: FunctionConfirmation,
}

pub fn function_registry() -> Vec<FunctionDefinition> {
    vec![
        function(
            "capture.create",
            "Criar Capture",
            "Registra uma nota local na Inbox.",
            FunctionCategory::Capture,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "capture.quick_open",
            "Abrir Quick Capture",
            "Exibe a janela de captura rapida.",
            FunctionCategory::Capture,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "capture.mark_processed",
            "Processar Capture",
            "Remove uma Capture da Inbox sem apagar o conteudo.",
            FunctionCategory::Capture,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "task.create",
            "Criar Task",
            "Cria uma Task local, opcionalmente vinculada a um Project.",
            FunctionCategory::Work,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "task.create_from_capture",
            "Converter Capture em Task",
            "Cria uma Task preservando a Capture como origem.",
            FunctionCategory::Work,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "task.set_state",
            "Mover Task",
            "Altera uma Task entre Backlog, Doing e Done.",
            FunctionCategory::Work,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "project.create",
            "Criar Project",
            "Cria um Project local para agrupar trabalho.",
            FunctionCategory::Work,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "resource.create",
            "Salvar Resource",
            "Guarda um link com o contexto pelo qual ele merece ser lembrado.",
            FunctionCategory::Memory,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "resource.create_from_capture",
            "Converter Capture em Resource",
            "Cria um Resource preservando a Capture como origem.",
            FunctionCategory::Memory,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "resource.open",
            "Abrir Resource",
            "Abre a URL validada de um Resource ativo no sistema.",
            FunctionCategory::Memory,
            FunctionRisk::Medium,
            FunctionConfirmation::None,
        ),
        function(
            "workspace.create",
            "Criar Workspace",
            "Cria uma lente de contexto sobre Projects e Apps.",
            FunctionCategory::Work,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "workspace.link_project",
            "Vincular Project a Workspace",
            "Inclui ou remove um Project de uma lente de contexto.",
            FunctionCategory::Work,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "workspace.link_app",
            "Vincular App a Workspace",
            "Inclui ou remove um App de uma lente de contexto.",
            FunctionCategory::Work,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "app.register",
            "Registrar App",
            "Adiciona um App conhecido ao registro local.",
            FunctionCategory::App,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "app.open",
            "Abrir App",
            "Abre URL ou caminho local cadastrado no registro de Apps.",
            FunctionCategory::App,
            FunctionRisk::Medium,
            FunctionConfirmation::None,
        ),
        function(
            "data.backup",
            "Criar Backup",
            "Copia o dataset local para um arquivo portavel.",
            FunctionCategory::Data,
            FunctionRisk::Medium,
            FunctionConfirmation::Explicit,
        ),
        function(
            "data.restore",
            "Restaurar Backup",
            "Substitui o dataset local por um backup selecionado.",
            FunctionCategory::Data,
            FunctionRisk::High,
            FunctionConfirmation::Explicit,
        ),
        function(
            "data.export_json",
            "Exportar JSON",
            "Exporta os dados locais para leitura externa.",
            FunctionCategory::Data,
            FunctionRisk::Medium,
            FunctionConfirmation::Explicit,
        ),
        function(
            "system.update_check",
            "Verificar Atualizacao",
            "Consulta GitHub Releases para encontrar uma versao assinada.",
            FunctionCategory::System,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "system.update_install",
            "Instalar Atualizacao",
            "Baixa, instala e reinicia o M/OS usando o updater assinado.",
            FunctionCategory::System,
            FunctionRisk::Medium,
            FunctionConfirmation::Explicit,
        ),
    ]
}

pub fn search_functions(query: &str, limit: usize) -> Vec<FunctionDefinition> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return function_registry().into_iter().take(limit).collect();
    }

    function_registry()
        .into_iter()
        .filter(|definition| {
            definition.id.contains(&query)
                || definition.name.to_lowercase().contains(&query)
                || definition.description.to_lowercase().contains(&query)
        })
        .take(limit)
        .collect()
}

fn function(
    id: &'static str,
    name: &'static str,
    description: &'static str,
    category: FunctionCategory,
    risk: FunctionRisk,
    confirmation: FunctionConfirmation,
) -> FunctionDefinition {
    FunctionDefinition {
        id,
        name,
        description,
        category,
        risk,
        confirmation,
    }
}

#[cfg(test)]
mod tests {
    use super::{function_registry, search_functions};

    #[test]
    fn registry_has_stable_ids() {
        let functions = function_registry();

        assert!(functions.iter().any(|item| item.id == "capture.create"));
        assert!(functions.iter().any(|item| item.id == "resource.create"));
        assert!(functions.iter().any(|item| item.id == "data.restore"));
        assert!(functions.iter().all(|item| item.id.contains('.')));
    }

    #[test]
    fn search_matches_id_name_and_description() {
        assert_eq!(search_functions("capture.create", 10).len(), 1);
        assert!(search_functions("Backup", 10)
            .iter()
            .any(|item| item.id == "data.backup"));
        assert!(search_functions("GitHub", 10)
            .iter()
            .any(|item| item.id == "system.update_check"));
    }

    #[test]
    fn search_limit_is_respected() {
        assert_eq!(search_functions("", 3).len(), 3);
    }
}
