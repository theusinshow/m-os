use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionCategory {
    Capture,
    Work,
    /// Rastreio de tempo. Categoria propria e nao `Work` porque Tempo e um
    /// substantivo central do produto e nao uma faceta do trabalho — e a razao
    /// esta na ADR-036: e dele que sai a renda de quem fatura por hora.
    Time,
    Memory,
    /// Lembretes e o que precisa da atencao da pessoa. Categoria propria
    /// pelo mesmo criterio que deu uma a `Time`: e um substantivo central
    /// do produto, e nao uma faceta de outro. `CORE.md` §1 lista Reminder
    /// entre os onze conceitos fundamentais desde o inicio.
    Attention,
    App,
    /// Reunioes. Categoria propria pelo mesmo criterio que deu uma a `Time` e
    /// outra a `Attention`: e um substantivo central do produto, e nao uma
    /// faceta de outro. `ATTENTION-SYSTEM.md` §0.2 ja dependia dela existir.
    Meeting,
    /// A Daily Session. Categoria propria pelo mesmo criterio que deu uma a
    /// `Time`, outra a `Attention` e outra a `Meeting`: e um substantivo
    /// central do produto, e nao uma faceta de outro. Ela nao e `Work` porque
    /// o dia nao e trabalho — e a decisao sobre qual trabalho importa hoje.
    Daily,
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
        // -------------------------------------------------------- Daily Session
        //
        // Nenhuma delas e de risco alto, e todas sao reversiveis: comecar o dia
        // se desfaz encerrando, encerrar se desfaz reabrindo, e concluir um
        // objetivo se desfaz devolvendo ele a pendente. O que separa `end_day`
        // das outras nao e o risco e sim o PESO — ele resolve varios objetivos
        // de uma vez —, e por isso ele pede confirmacao explicita mantendo o
        // risco baixo. Risco e confirmacao sao campos diferentes de proposito.
        function(
            "daily.start_day",
            "Iniciar meu dia",
            "Monta a sessao do dia com um objetivo principal e ate tres secundarios.",
            FunctionCategory::Daily,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "daily.view_today",
            "Ver a sessao do dia",
            "Abre os objetivos de hoje, o progresso e o que esta vinculado a eles.",
            FunctionCategory::Daily,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "daily.add_objective",
            "Adicionar objetivo do dia",
            "Acrescenta um objetivo ao dia que ja comecou.",
            FunctionCategory::Daily,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "daily.set_objective_status",
            "Resolver objetivo do dia",
            "Conclui, leva para amanha, abandona ou devolve um objetivo a pendente.",
            FunctionCategory::Daily,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "daily.set_main",
            "Definir o objetivo principal",
            "Promove um objetivo a principal, rebaixando o anterior.",
            FunctionCategory::Daily,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "daily.end_day",
            "Encerrar meu dia",
            "Fecha a sessao do dia, resolve os pendentes e guarda a reflexao.",
            FunctionCategory::Daily,
            FunctionRisk::Low,
            FunctionConfirmation::Explicit,
        ),
        function(
            "attention.create_reminder",
            "Criar lembrete",
            "Agenda um lembrete para um instante escolhido.",
            FunctionCategory::Attention,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "attention.resolve_reminder",
            "Resolver lembrete",
            "Conclui ou cancela um lembrete que ja existe.",
            FunctionCategory::Attention,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
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
        // Trocar o Project de uma Task NAO herda o risco de move-la de coluna,
        // e a diferenca aparece no Tempo: as horas lancadas seguem o Project, e
        // um lancamento no Project errado vira uma linha na fatura errada. Nao
        // e risco alto — a hora ja lancada guarda a taxa em snapshot —, mas
        // tambem nao e a mesma coisa que arrastar um cartao entre colunas.
        function(
            "task.set_project",
            "Mover Task de Project",
            "Passa uma Task para outro Project, ou tira ela de todos.",
            FunctionCategory::Work,
            FunctionRisk::Medium,
            FunctionConfirmation::Explicit,
        ),
        function(
            "project.create",
            "Criar Project",
            "Cria um Project local para agrupar trabalho.",
            FunctionCategory::Work,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        // As tres do tempo. O risco NAO e uniforme, e a diferenca e deliberada.
        //
        // Iniciar e barato: o banco recusa um segundo cronometro, entao a pior
        // consequencia de um engano e um cronometro rodando no Project errado,
        // visivel na hora e corrigivel com um clique.
        //
        // Encerrar e lancar escrevem hora COBRAVEL a partir de uma frase. Um
        // "duas horas" ouvido como "doze horas" vira erro de fatura, e o erro so
        // aparece no dia de cobrar. Por isso os dois pedem confirmacao explicita
        // — a mesma razao pela qual a fase 3 desta spec e a que exige mais
        // cuidado: quando a acao vira dinheiro, a cerimonia precisa pesar.
        function(
            "time.start",
            "Iniciar cronometro",
            "Comeca a contar tempo num Project. Recusa se ja houver cronometro.",
            FunctionCategory::Time,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "time.stop",
            "Encerrar cronometro",
            "Encerra o cronometro em curso e grava a sessao.",
            FunctionCategory::Time,
            FunctionRisk::Medium,
            FunctionConfirmation::Explicit,
        ),
        function(
            "time.record",
            "Lancar tempo",
            "Registra tempo trabalhado que o cronometro nao contou.",
            FunctionCategory::Time,
            FunctionRisk::Medium,
            FunctionConfirmation::Explicit,
        ),
        // As sete da reuniao. O risco NAO e uniforme, e a diferenca e a mesma
        // que separa `time.start` de `time.record`: o que so mexe em dado local
        // e barato, o que atravessa a fronteira da maquina ou apaga bytes nao.
        function(
            "meeting.start",
            "Iniciar Meeting Notes",
            "Comeca a gravar microfone e audio do sistema. Nunca acontece sem clique.",
            FunctionCategory::Meeting,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "meeting.stop",
            "Parar Meeting Notes",
            "Encerra a gravacao e fecha os arquivos de audio.",
            FunctionCategory::Meeting,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "meeting.transcribe",
            "Transcrever reuniao",
            "Transcreve o audio localmente. Nada sai da maquina.",
            FunctionCategory::Meeting,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        // Risco medio, e a razao nao e a consequencia local: e que ela ENVIA a
        // transcricao para a VPS do Hermes. O consentimento e dado uma vez e
        // registrado (ADR-027), mas a acao continua marcada como o limite que
        // ela atravessa.
        function(
            "meeting.analyze",
            "Analisar reuniao com o Hermes",
            "Envia a transcricao ao Hermes e recebe resumo, decisoes e acoes.",
            FunctionCategory::Meeting,
            FunctionRisk::Medium,
            FunctionConfirmation::None,
        ),
        function(
            "meeting.accept_insight",
            "Criar Task a partir da reuniao",
            "Cria Task e, quando pedido, Reminder a partir de um item da reuniao.",
            FunctionCategory::Meeting,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        function(
            "meeting.link_project",
            "Vincular reuniao a Project",
            "Relaciona a reuniao ao contexto em que ela pertence.",
            FunctionCategory::Meeting,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
        // A unica da familia que APAGA bytes. Confirmacao explicita pela mesma
        // razao que `data.restore` a exige: o inverso nao existe.
        function(
            "meeting.delete_audio",
            "Apagar audio da reuniao",
            "Remove os arquivos de audio. A transcricao e a analise permanecem.",
            FunctionCategory::Meeting,
            FunctionRisk::Medium,
            FunctionConfirmation::Explicit,
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
            // Nasceu `workspace.set_widget`, dentro do inspetor de Workspace.
            // Mudou de nome junto com o lugar: esconder widget e capacidade da
            // HOME, e vale em qualquer contexto — inclusive em "Todos", que nao
            // e Workspace nenhum. O id nao vai para o banco nem para o Hermes;
            // vive so entre este registro, o mapa de intents e a ancora no DOM.
            "home.set_widget",
            "Escolher widgets da Home",
            "Mostra ou oculta um widget da Home, no contexto atual.",
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
            "m-finance.create_bill",
            "Criar conta no M-Finance",
            "Propõe uma conta (valor, descrição, vencimento) para o M-Finance, App externo. Dinheiro é sempre risco alto.",
            FunctionCategory::App,
            FunctionRisk::High,
            FunctionConfirmation::Explicit,
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
    use super::{function_registry, search_functions, FunctionConfirmation, FunctionRisk};

    #[test]
    fn m_finance_create_bill_is_registered_as_high_risk() {
        let entry = function_registry()
            .into_iter()
            .find(|item| item.id == "m-finance.create_bill")
            .expect("m-finance.create_bill deveria estar registrada");
        assert_eq!(entry.risk, FunctionRisk::High);
        assert_eq!(entry.confirmation, FunctionConfirmation::Explicit);
    }

    #[test]
    fn apagar_audio_de_reuniao_exige_confirmacao_explicita() {
        let entry = function_registry()
            .into_iter()
            .find(|item| item.id == "meeting.delete_audio")
            .expect("meeting.delete_audio deveria estar registrada");
        assert_eq!(entry.confirmation, FunctionConfirmation::Explicit);
    }

    #[test]
    fn analisar_reuniao_nao_e_risco_baixo() {
        // Ela envia a transcricao para fora da maquina. Rebaixa-la para `Low`
        // apagaria o unico lugar do registro onde esse limite aparece.
        let entry = function_registry()
            .into_iter()
            .find(|item| item.id == "meeting.analyze")
            .expect("meeting.analyze deveria estar registrada");
        assert_eq!(entry.risk, FunctionRisk::Medium);
    }

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
    fn widget_visibility_is_a_declared_function() {
        assert!(function_registry()
            .iter()
            .any(|item| item.id == "home.set_widget"));
    }

    #[test]
    fn search_limit_is_respected() {
        assert_eq!(search_functions("", 3).len(), 3);
    }
}
