//! O Hermes como agente operacional do M/OS.
//!
//! # O problema que este modulo resolve
//!
//! O `action.rs` ja dava ao Hermes um catalogo de acoes. Faltava o resto da
//! frase: **o modelo nao sabia onde estava.** Uma mensagem como *"cria lembrete
//! para a task que ja esta no kanban"* chegava a VPS como texto solto, e
//! "kanban" era, para o modelo, um conceito de metodologia — nao a coluna de
//! Tasks que o usuario tinha aberta na tela. O agente respondia explicando como
//! criar um lembrete, em vez de criar.
//!
//! Tres ausencias produziam isso, e este modulo cobre as tres:
//!
//! | Ausencia | O que entra aqui |
//! |---|---|
//! | Ninguem dizia que ele opera o M/OS | [`system_context`] |
//! | Ninguem dizia que horas sao, nem onde o usuario esta | [`now_block`], [`here_block`] |
//! | Ninguem dizia quais entidades existem | [`candidates_block`], [`search_terms`], [`query_contract`] |
//!
//! # Por que a busca acontece ANTES do envio
//!
//! A ADR-028 escolheu injecao de contexto porque o protocolo do gateway nao tem
//! registro de ferramenta do lado do cliente: o agente **nao consegue pedir
//! dados no meio do turno**. A consequencia registrada la era "o contexto e
//! fixo no envio" — e ate agora o unico contexto era o que o usuario anexava a
//! mao com `@`.
//!
//! A saida nao e mudar a topologia da rede: e o M/OS fazer a busca **por conta
//! propria, antes de enviar**, com os proprios servicos de leitura. Quem
//! escreve "a task do Victor" nao precisa anexar nada — o M/OS le a frase,
//! procura no FTS local e desce os candidatos junto. E o §7 do pedido em forma
//! de codigo: *resolve → search → infer → act*, e nao *ask → ask → ask*.
//!
//! Quando a primeira busca nao acha, sobra o [`query_contract`]: o modelo pede
//! UMA busca extra por bloco de texto, o M/OS executa localmente e reenvia. E
//! um segundo salto, nao um servidor exposto — a ADR-028 continua de pe.
//!
//! # O que este modulo NAO faz
//!
//! Nada aqui le banco, abre rede ou executa acao. Ele monta texto e resolve
//! referencia sobre dados que alguem ja leu — a mesma disciplina do `action.rs`,
//! pelo mesmo motivo: o `mos-core` nao conhece SQLite nem o Tauri.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{voice_when::fold_text, CoreError, ErrorCode};

// ------------------------------------------------------------------ identidade

/// O que o Hermes precisa saber antes de qualquer mensagem.
///
/// Em portugues, e nao em ingles, porque a conversa e em portugues e o resto do
/// prompt tambem: `action_contract` e `assemble_context` ja descem assim. Um
/// preambulo em outra lingua empurraria a resposta para ela.
///
/// A frase sobre nao duplicar e a mais importante do bloco. Sem ela, "a task ja
/// cadastrada no kanban" vira uma segunda Task com o mesmo titulo — o modelo
/// prestativo cria o que nao encontra, e o usuario acaba com duas.
pub fn system_context() -> String {
    "[Quem você é]\n\
     Você é o Hermes operando DENTRO do M/OS, o sistema pessoal do usuário. \
     O chat é uma das superfícies do M/OS, não uma janela de um app externo.\n\n\
     Quando o usuário falar de task, Kanban, projeto, capture, resource, \
     lembrete, agenda, calendário, pessoa, arquivo, decisão, waiting for, \
     timeline, inbox ou busca SEM nomear outro sistema, ele está falando das \
     entidades do M/OS. Só assuma ferramenta externa quando ele disser o nome \
     dela (\"task do Notion\", \"evento no Google Calendar\", \"issue do GitHub\").\n\n\
     Seu papel é OPERAR o M/OS, não explicar como operá-lo. Quando houver \
     contexto suficiente, proponha a ação em vez de descrever o caminho na \
     interface.\n\n\
     Ordem de trabalho: entenda a intenção, identifique as entidades citadas, \
     use o que o M/OS já pesquisou e mandou abaixo, proponha a ação preservando \
     os vínculos e confirme em uma frase.\n\n\
     Nunca crie entidade nova quando o usuário disser que ela já existe. \
     \"a task que já está no kanban\" é uma Task existente: encontre o id dela \
     nos candidatos e aponte para ele.\n\
     [Fim de quem você é]\n\n"
        .to_owned()
}

// ----------------------------------------------------------------------- agora

const DIAS: [&str; 7] = [
    "segunda-feira",
    "terça-feira",
    "quarta-feira",
    "quinta-feira",
    "sexta-feira",
    "sábado",
    "domingo",
];

const MESES: [&str; 12] = [
    "janeiro",
    "fevereiro",
    "março",
    "abril",
    "maio",
    "junho",
    "julho",
    "agosto",
    "setembro",
    "outubro",
    "novembro",
    "dezembro",
];

/// A data e a hora de quem está falando, por extenso.
///
/// Existe porque "hoje às 20:30" nao significa nada sem ancora, e o modelo do
/// outro lado nao tem relogio confiavel — muito menos o FUSO de quem falou. O
/// `now_local` chega ja com o offset do renderer, que e a mesma regra normativa
/// que o `ReminderComposer` e o `voice_when` seguem (`CORE-FOUNDATION.md` §5).
///
/// O offset aparece escrito porque o modelo pode propor `at` em ISO: sem ele,
/// um instante sem fuso seria lido como UTC e o lembrete tocaria tres horas
/// depois.
pub fn now_block(now_local: OffsetDateTime) -> String {
    let dia = DIAS[now_local.weekday().number_days_from_monday() as usize];
    let mes = MESES[now_local.month() as usize - 1];
    let (horas, minutos, _) = now_local.offset().as_hms();
    format!(
        "[Agora]\n\
         {dia}, {} de {mes} de {}, {:02}:{:02} (UTC{}{:02}:{:02})\n\
         Datas relativas — \"hoje\", \"amanhã\", \"sexta\", \"hoje à noite\" — \
         contam a partir deste instante, neste fuso.\n\
         [Fim de agora]\n\n",
        now_local.day(),
        now_local.year(),
        now_local.hour(),
        now_local.minute(),
        if horas < 0 { '-' } else { '+' },
        horas.abs(),
        minutos.abs(),
    )
}

/// Um instante escrito como alguem leria em voz alta.
///
/// Absoluto, e nao relativo: o cartao de preview e onde o usuario confere o que
/// vai ser agendado, e "hoje às 20:30" so confere se ele ja souber que dia e
/// hoje. "quinta-feira, 20 de agosto, 20:30" nao depende de nada.
pub fn spoken_moment(instant: OffsetDateTime) -> String {
    let dia = DIAS[instant.weekday().number_days_from_monday() as usize];
    let mes = MESES[instant.month() as usize - 1];
    format!(
        "{dia}, {} de {mes}, {:02}:{:02}",
        instant.day(),
        instant.hour(),
        instant.minute()
    )
}

// ------------------------------------------------------------------------ onde

/// Uma entidade citada pelo nome, com o id ao lado.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Named {
    pub id: String,
    pub label: String,
}

impl Named {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// O que a interface esta mostrando agora.
///
/// Vem da tela e nao do banco: e a resposta para "me lembra disso sexta", em
/// que **"disso" e o que esta aberto**. Sem isto o modelo teria de perguntar o
/// que ja esta na frente do usuario — exatamente o fluxo que o §8 do pedido
/// chama de excecao, e nao de padrao.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Here {
    /// Nome legivel da tela: "Kanban", "Projects", "Inbox".
    #[serde(default)]
    pub screen: String,
    #[serde(default)]
    pub project: Option<Named>,
    #[serde(default)]
    pub task: Option<Named>,
    #[serde(default)]
    pub workspace: Option<Named>,
}

impl Here {
    pub fn is_empty(&self) -> bool {
        self.screen.trim().is_empty()
            && self.project.is_none()
            && self.task.is_none()
            && self.workspace.is_none()
    }
}

/// O bloco de "onde o usuario esta".
///
/// Vazio quando nao ha nada a dizer: um bloco que so anuncia ausencia gastaria
/// token em toda mensagem para informar nada.
pub fn here_block(here: &Here) -> String {
    if here.is_empty() {
        return String::new();
    }
    let mut linhas = Vec::new();
    if !here.screen.trim().is_empty() {
        linhas.push(format!("Tela aberta: {}", here.screen.trim()));
    }
    if let Some(workspace) = &here.workspace {
        linhas.push(format!(
            "Workspace atual: {} (id {})",
            workspace.label,
            short_id(&workspace.id)
        ));
    }
    if let Some(project) = &here.project {
        linhas.push(format!(
            "Project aberto: {} (id {})",
            project.label,
            short_id(&project.id)
        ));
    }
    if let Some(task) = &here.task {
        linhas.push(format!(
            "Task aberta: {} (id {})",
            task.label,
            short_id(&task.id)
        ));
    }
    format!(
        "[Onde o usuário está no M/OS]\n{}\n\
         Pronomes soltos — \"isso\", \"essa task\", \"esse projeto\", \"aqui\" — \
         quase sempre apontam para o que está aberto.\n\
         [Fim de onde]\n\n",
        linhas.join("\n")
    )
}

/// O dia de hoje, quando ele existe.
///
/// # Por que ele desce no preambulo, e nao por acao
///
/// *"O que falta dos meus objetivos de hoje?"* e uma PERGUNTA, e nao um comando.
/// Responde-la por acao gastaria um turno inteiro — proposta, preview,
/// confirmacao — para devolver tres linhas que o M/OS ja tem na mao. E o mesmo
/// criterio do §15.3 do `MEETING-AGENT.md`: onde a regra deterministica serve,
/// ela ganha da IA.
///
/// **Vazio quando nao ha sessao aberta**, e nao "nenhum objetivo definido". O
/// preambulo desce em toda mensagem, e um bloco que so anuncia ausencia gastaria
/// token em toda conversa para informar nada — mesma regra do `here_block`.
/// Quem quer saber que o dia nao comecou pergunta, e a acao `mos.day.start`
/// existe justamente para o que vem depois da resposta.
pub fn today_block(day: &str, objectives: &[(String, String, bool)]) -> String {
    if objectives.is_empty() {
        return String::new();
    }
    let linhas = objectives
        .iter()
        .map(|(titulo, peso, feito)| {
            format!(
                "- {} {titulo} ({peso})",
                // Marca de texto e nao so estado escrito: a linha e lida de
                // relance, e o simbolo faz o "falta" aparecer antes da palavra.
                if *feito { "[x]" } else { "[ ]" }
            )
        })
        .collect::<Vec<_>>()
        .join("
");
    let feitos = objectives.iter().filter(|(_, _, feito)| *feito).count();
    format!(
        "[Os objetivos de hoje ({day})]
{linhas}
         {feitos} de {} concluídos. Objetivo é a decisão sobre o que importa          hoje — nunca crie Task para representar um.
         [Fim dos objetivos]

",
        objectives.len()
    )
}

// ------------------------------------------------------------------ candidatos

/// Os tipos de entidade que a resolucao alcanca.
///
/// Enum fechado e nao string livre: um `kind` inventado pelo modelo precisa ser
/// recusado na borda, e nao virar busca vazia la dentro.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Task,
    Project,
    Capture,
    Resource,
    Workspace,
    Meeting,
    Reminder,
    /// Um objetivo do dia. Entra no vocabulario porque o Hermes precisa poder
    /// apontar para um — "troca meu segundo objetivo" so resolve se o objetivo
    /// for uma entidade citavel.
    DailyObjective,
}

impl EntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Project => "project",
            Self::Capture => "capture",
            Self::Resource => "resource",
            Self::Workspace => "workspace",
            Self::Meeting => "meeting",
            Self::Reminder => "reminder",
            Self::DailyObjective => "objetivo",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "task" | "tarefa" => Some(Self::Task),
            "project" | "projeto" => Some(Self::Project),
            "capture" | "captura" => Some(Self::Capture),
            "resource" | "recurso" => Some(Self::Resource),
            "workspace" => Some(Self::Workspace),
            "meeting" | "reuniao" | "reunião" => Some(Self::Meeting),
            "reminder" | "lembrete" => Some(Self::Reminder),
            "objetivo" | "objective" | "daily_objective" => Some(Self::DailyObjective),
            _ => None,
        }
    }
}

/// Uma entidade que a busca automatica encontrou.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub kind: EntityKind,
    /// O id completo. O bloco mostra so o prefixo, mas a resolucao aceita os dois.
    pub id: String,
    pub label: String,
    /// Estado, Project, data — o que distingue este candidato dos outros.
    #[serde(default)]
    pub detail: String,
}

/// Os oito primeiros caracteres do UUID.
///
/// O bloco mostra o prefixo em vez do id inteiro por uma razao de custo: o
/// catalogo desce em TODA mensagem, e trinta e seis caracteres por linha, vezes
/// doze candidatos, e um paragrafo inteiro gasto em hifens. Oito digitos hex
/// sao 4 bilhoes de combinacoes — colisao dentro de uma lista de doze e uma
/// hipotese que nao precisa de defesa. E a resolucao aceita o id inteiro
/// tambem, para o dia em que o modelo copiar de outro lugar.
pub fn short_id(id: &str) -> &str {
    let corte = id
        .char_indices()
        .nth(8)
        .map(|(indice, _)| indice)
        .unwrap_or(id.len());
    &id[..corte]
}

/// Teto de candidatos no prompt.
///
/// Doze e o numero que cabe sem empurrar a pergunta para fora da janela. O
/// `CONTEXT_BUDGET` do `assemble_context` protege o contexto ANEXADO; este
/// protege o automatico, que o usuario nao escolheu e por isso nao percebe
/// crescer.
pub const MAX_CANDIDATES: usize = 12;

/// O bloco que diz o que existe no M/OS para esta mensagem.
///
/// Vazio quando nada foi encontrado — e a ausencia e informacao: um bloco
/// dizendo "nenhum resultado" convidaria o modelo a concluir que a entidade nao
/// existe, quando o que houve foi busca fraca. Sem bloco, ele ainda pode pedir
/// uma busca melhor pelo `mos-query`.
pub fn candidates_block(candidates: &[Candidate]) -> String {
    if candidates.is_empty() {
        return String::new();
    }
    let linhas = candidates
        .iter()
        .take(MAX_CANDIDATES)
        .map(|candidate| {
            let detail = if candidate.detail.trim().is_empty() {
                String::new()
            } else {
                format!(" · {}", candidate.detail.trim())
            };
            format!(
                "- {} {} · {}{}",
                candidate.kind.as_str(),
                short_id(&candidate.id),
                candidate.label,
                detail
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "[Entidades do M/OS encontradas para esta mensagem]\n\
         O M/OS pesquisou a base local antes de enviar. Estas existem AGORA:\n\
         {linhas}\n\
         Ao propor uma ação sobre uma delas, use o id mostrado acima. \
         Não crie uma entidade nova que já esteja nesta lista.\n\
         [Fim das entidades]\n\n"
    )
}

// --------------------------------------------------------------------- resolver

/// O resultado de procurar uma entidade por uma referencia humana.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Resolved<T> {
    /// Um acerto forte e sozinho. E o caso em que se age sem perguntar.
    One(T),
    /// Varios acertos do mesmo peso. Perguntar aqui e legitimo (§8).
    Many(Vec<T>),
    None,
}

impl<T> Resolved<T> {
    pub fn one(self) -> Option<T> {
        match self {
            Self::One(item) => Some(item),
            _ => None,
        }
    }
}

/// Encontra a entidade a que uma referencia aponta.
///
/// # Por que em degraus, e nao numa comparacao so
///
/// A referencia pode chegar de tres jeitos diferentes na mesma conversa: o id
/// que o proprio M/OS mandou no bloco de candidatos, o titulo inteiro como o
/// modelo copiou, ou um pedaco do titulo como a pessoa falou. Uma comparacao
/// unica teria de escolher qual atender, e perderia as outras duas.
///
/// Os degraus tambem resolvem a ambiguidade sem sorteio: **o primeiro degrau
/// que acerta decide**. Se o id casa, casou — nao importa que o titulo dele
/// tambem apareca dentro de outros tres. E so quando o degrau que acertou
/// devolve mais de um e que existe duvida de verdade, e ai o chamador pergunta
/// em vez de escolher pelo primeiro da lista.
pub fn resolve<'a, T, I, L>(items: &'a [T], reference: &str, id: I, label: L) -> Resolved<&'a T>
where
    I: Fn(&T) -> String,
    L: Fn(&T) -> String,
{
    let needle = fold_text(reference.trim());
    if needle.is_empty() {
        return Resolved::None;
    }

    // Ponteiros de funcao, e nao closures em caixa: nenhum degrau captura nada,
    // e um `Box<dyn Fn>` aqui alocaria cinco vezes por resolucao para guardar
    // cinco comparacoes de string.
    let degraus: [fn(&str, &str) -> bool; 5] = [
        // Id inteiro.
        |campo, needle| campo == needle,
        // Prefixo de id, como o bloco de candidatos mostra. Seis digitos e o
        // piso: com menos, "1f4" casaria com meia base por acaso.
        |campo, needle| needle.len() >= 6 && needle.len() <= campo.len() && campo.starts_with(needle),
        // Titulo exato.
        |campo, needle| campo == needle,
        // Comeco do titulo. Vem antes de "contem" porque quem cita uma Task
        // costuma citar o comeco dela.
        |campo, needle| campo.starts_with(needle),
        |campo, needle| campo.contains(needle),
    ];

    for (indice, casa) in degraus.iter().enumerate() {
        let por_id = indice < 2;
        let achados: Vec<&T> = items
            .iter()
            .filter(|item| {
                let campo = if por_id {
                    fold_text(&id(item))
                } else {
                    fold_text(&label(item))
                };
                casa(&campo, &needle)
            })
            .collect();
        match achados.len() {
            0 => continue,
            1 => return Resolved::One(achados[0]),
            _ => return Resolved::Many(achados),
        }
    }

    Resolved::None
}

/// Traduz o resultado da resolucao em erro legivel, quando nao houve um so.
///
/// Existe para as mensagens serem iguais em todas as acoes: "bate com 3 Tasks"
/// escrito de tres jeitos em tres lugares vira tres bugs de copia.
pub fn resolution_error<T>(
    resolved: &Resolved<&T>,
    kind: EntityKind,
    reference: &str,
    label: impl Fn(&T) -> String,
) -> Option<CoreError> {
    let nome = match kind {
        EntityKind::Task => "Task",
        EntityKind::Project => "Project",
        EntityKind::Capture => "Capture",
        EntityKind::Resource => "Resource",
        EntityKind::Workspace => "Workspace",
        EntityKind::Meeting => "Reunião",
        EntityKind::Reminder => "Lembrete",
        EntityKind::DailyObjective => "Objetivo do dia",
    };
    match resolved {
        Resolved::One(_) => None,
        Resolved::None => Some(CoreError::new(
            ErrorCode::NotFound,
            format!("Não achei {nome} para \"{reference}\"."),
            false,
        )),
        Resolved::Many(achados) => {
            // Os titulos entram no erro de proposito: "bate com 3" manda o
            // usuario procurar, e a lista deixa ele responder na propria frase.
            let amostra = achados
                .iter()
                .take(4)
                .map(|item| format!("\"{}\"", label(item)))
                .collect::<Vec<_>>()
                .join(", ");
            Some(CoreError::new(
                ErrorCode::InvalidInput,
                format!(
                    "\"{reference}\" bate com {} {nome}s: {amostra}. Diga qual.",
                    achados.len()
                ),
                false,
            ))
        }
    }
}

// ------------------------------------------------------------------ termos

/// Palavras que nao ajudam a achar nada.
///
/// Duas familias, e a segunda e a que importa aqui: alem dos conectivos, saem
/// os **substantivos do proprio M/OS**. Procurar por "task" numa base em que
/// tudo e task devolve tudo; procurar por "lembrete" numa frase que PEDE um
/// lembrete devolve os lembretes antigos em vez da coisa lembrada.
const RUIDO: [&str; 136] = [
    // conectivos e artigos
    "a", "ao", "aos", "as", "com", "da", "das", "de", "do", "dos", "e", "em", "essa", "esse",
    "esta", "este", "eu", "isso", "isto", "ja", "la", "lhe", "me", "meu", "minha", "na", "nas",
    "no", "nos", "num", "numa", "o", "os", "ou", "para", "pra", "pro", "por", "que", "se", "ser",
    "sobre", "um", "uma", "voce", "aquele", "aquela", "aquilo", "tem", "ter", "vai", "foi",
    // vocabulario do proprio M/OS
    "task", "tasks", "tarefa", "tarefas", "kanban", "board", "quadro", "coluna", "projeto",
    "projetos", "project", "capture", "captura", "capturas", "resource", "recurso", "recursos",
    "lembrete", "lembretes", "reminder", "agenda", "calendario", "inbox", "workspace", "timeline",
    "arquivo", "arquivos", "mos", "hermes", "sistema", "app",
    // verbos de comando, que descrevem a acao e nao o alvo
    "cria", "criar", "adiciona", "adicionar", "coloca", "colocar", "move", "mover", "marca",
    "marcar", "lembra", "lembrar", "procura", "procurar", "busca", "buscar", "cadastrada",
    "cadastrado",
    // pronomes e palavras de tempo: dizem QUANDO e sobre o QUE, nunca QUAL.
    // Sem eles, "me lembra disso hoje de noite" procuraria por "disso",
    // "hoje" e "noite" — tres palavras que aparecem em metade da base.
    "disso", "disto", "daquilo", "dele", "dela", "nele", "nela", "aqui", "ali", "agora", "hoje",
    "amanha", "ontem", "noite", "tarde", "manha", "hora", "horas", "minuto", "minutos", "dia",
    "dias", "semana", "mes", "ainda", "depois", "antes", "quando", "onde", "qual", "todos",
    "tudo", "nada", "mais", "menos",
];

/// Minuscula e sem acento, para comparar texto com termo de busca.
///
/// E o mesmo dobramento que a resolucao e os termos usam. Quem precisa comparar
/// um titulo com um termo tem de passar pela MESMA funcao — duas normalizacoes
/// diferentes fariam "Enviar" bater na resolucao e nao bater na busca.
pub fn normalize(text: &str) -> String {
    fold_text(text)
}

/// Comprimento minimo de um termo util.
///
/// Tres, e nao quatro: nomes proprios curtos existem e sao o melhor sinal que
/// uma frase costuma ter.
const MIN_TERMO: usize = 3;

/// Teto de termos por busca. Cada termo e uma varredura FTS.
const MAX_TERMOS: usize = 8;

/// Quebra a frase do usuario nos termos que valem uma busca.
///
/// O tokenizador aqui NAO e o do `voice_when`: aquele quebra no hifen, porque
/// "segunda-feira" precisa virar "segunda". Este preserva o hifen dentro da
/// palavra, porque `063-26` e um codigo de projeto e parti-lo em `063` e `26`
/// procuraria por dois numeros que aparecem em qualquer lugar.
pub fn search_terms(text: &str) -> Vec<String> {
    let mut termos: Vec<String> = Vec::new();
    let normalizado = fold_text(text);
    let mut atual = String::new();

    let empurra = |atual: &mut String, termos: &mut Vec<String>| {
        let palavra = atual.trim_matches(['-', '/', '.']).to_owned();
        atual.clear();
        if palavra.chars().count() < MIN_TERMO {
            return;
        }
        if RUIDO.contains(&palavra.as_str()) {
            return;
        }
        if termos.iter().any(|existente| existente == &palavra) {
            return;
        }
        termos.push(palavra);
    };

    for caractere in normalizado.chars() {
        if caractere.is_alphanumeric() || caractere == '-' || caractere == '/' || caractere == '.' {
            atual.push(caractere);
        } else {
            empurra(&mut atual, &mut termos);
        }
    }
    empurra(&mut atual, &mut termos);

    termos.truncate(MAX_TERMOS);
    termos
}

// ------------------------------------------------------------------- consulta

/// Uma busca que o modelo pediu.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRequest {
    pub search: String,
    /// Vazio significa todos os tipos.
    #[serde(default)]
    pub kinds: Vec<EntityKind>,
}

/// Quantas buscas extras um turno pode pedir.
///
/// Uma. Cada salto e um `prompt.submit` inteiro sobre um tunel SSH ate uma VPS,
/// e o custo aparece como silencio na tela. Uma busca extra cobre o caso real —
/// a primeira varredura pegou o termo errado — e duas ja seriam o agente
/// tateando enquanto o usuario espera.
pub const MAX_QUERY_HOPS: u8 = 1;

/// O contrato da busca, que so desce enquanto ainda ha salto disponivel.
///
/// Some do prompt no ultimo salto de proposito: oferecer uma ferramenta que
/// nao vai ser executada ensinaria o modelo a pedi-la e receber silencio.
pub fn query_contract() -> String {
    "[Buscar mais no M/OS]\n\
     Se as entidades acima não bastarem para agir, responda APENAS com um bloco:\n\n\
     ```mos-query\n\
     { \"search\": \"termos\", \"kinds\": [\"task\"] }\n\
     ```\n\n\
     O M/OS executa a busca na base local e devolve o resultado para você \
     continuar. `kinds` é opcional; sem ele a busca alcança todos os tipos. \
     Use isto UMA vez, e só quando faltar dado — não para confirmar o que já veio.\n\
     [Fim da busca]\n\n"
        .to_owned()
}

/// Le o bloco de busca. Recusa em vez de adivinhar, igual ao `parse_action`.
pub fn parse_query(raw: &str) -> Result<QueryRequest, CoreError> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|error| {
        CoreError::new(
            ErrorCode::InvalidInput,
            format!("A busca não é um JSON válido: {error}"),
            false,
        )
    })?;

    let search = value
        .get("search")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    if search.is_empty() {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "A busca veio sem `search`.",
            false,
        ));
    }

    // Um `kinds` com nome desconhecido e ignorado, e nao recusado: o pedido
    // ainda e uma busca legivel, e recusar a frase inteira por causa de um
    // rotulo torto gastaria o unico salto disponivel com um erro.
    let kinds = value
        .get("kinds")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter_map(EntityKind::parse)
                .collect()
        })
        .unwrap_or_default();

    Ok(QueryRequest { search, kinds })
}

/// O que o M/OS devolve ao modelo depois de executar a busca pedida.
///
/// Vai como mensagem de usuario porque o protocolo do gateway so tem
/// `prompt.submit` — nao ha papel `tool` do lado do cliente. O bloco se anuncia
/// como resultado de sistema para o modelo nao confundir com fala da pessoa.
pub fn query_answer(request: &QueryRequest, candidates: &[Candidate]) -> String {
    let corpo = if candidates.is_empty() {
        "Nenhuma entidade do M/OS bate com esses termos.".to_owned()
    } else {
        candidates
            .iter()
            .take(MAX_CANDIDATES)
            .map(|candidate| {
                let detail = if candidate.detail.trim().is_empty() {
                    String::new()
                } else {
                    format!(" · {}", candidate.detail.trim())
                };
                format!(
                    "- {} {} · {}{}",
                    candidate.kind.as_str(),
                    short_id(&candidate.id),
                    candidate.label,
                    detail
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "[Resultado da busca no M/OS — mensagem do sistema, não do usuário]\n\
         Busca: \"{}\"\n\
         {corpo}\n\
         Agora responda ao pedido original. Não peça outra busca: esta foi a \
         única disponível. Se ainda faltar dado, pergunte ao usuário em uma frase.\n\
         [Fim do resultado]",
        request.search
    )
}

// --------------------------------------------------------------------- blocos

/// Separa um bloco cercado do texto da resposta.
///
/// Generalizacao do que o `split_proposal` fazia so para `mos-action`: a mesma
/// leitura serve para `mos-query`, e duas copias divergiriam na primeira vez
/// que uma delas ganhasse um caso de borda.
///
/// So o PRIMEIRO bloco e lido. O contrato pede um por mensagem, e ler o segundo
/// de uma resposta que ja saiu do contrato seria confiar num formato que aquela
/// mesma mensagem provou errado.
pub fn split_fenced(text: &str, fence_name: &str) -> (String, Option<String>) {
    let fence = format!("```{fence_name}");
    let Some(start) = text.find(&fence) else {
        return (text.to_owned(), None);
    };
    let after = start + fence.len();
    let Some(end_offset) = text[after..].find("```") else {
        // Cerca aberta: o turno pode ter sido interrompido no meio do bloco.
        // Sem fechamento nao ha bloco valido, e o texto fica como veio.
        return (text.to_owned(), None);
    };
    let raw = text[after..after + end_offset].trim().to_owned();
    let mut limpo = String::with_capacity(text.len());
    limpo.push_str(&text[..start]);
    limpo.push_str(&text[after + end_offset + 3..]);
    (limpo.trim().to_owned(), Some(raw))
}

// ------------------------------------------------------------------ preambulo

/// Tudo o que o M/OS sabe e que o modelo precisa saber.
pub struct PreambleInput<'a> {
    pub now_local: OffsetDateTime,
    pub here: &'a Here,
    pub candidates: &'a [Candidate],
    pub finance_enabled: bool,
    /// Quantas buscas extras ainda cabem neste turno.
    pub hops_left: u8,
    /// O dia e os objetivos dele: `(titulo, peso, concluido)`. Vazio quando nao
    /// ha sessao aberta — e ai o bloco nao desce.
    pub today: (String, Vec<(String, String, bool)>),
}

/// Monta o prefixo do prompt, na ordem em que ele deve ser lido.
///
/// A ordem nao e estetica. Identidade primeiro, porque ela reenquadra tudo o
/// que vem depois; tempo e lugar em seguida, porque sao o que resolve pronome e
/// data; o catalogo de acoes depois, porque so faz sentido quando ja se sabe
/// sobre o que agir; e os candidatos por ultimo, colados na pergunta, porque
/// sao o dado mais especifico e o mais facil de perder no meio.
pub fn preamble(input: PreambleInput<'_>) -> String {
    let mut blocos = vec![
        system_context(),
        now_block(input.now_local),
        here_block(input.here),
        // Depois de "onde voce esta" e antes do catalogo: o dia e contexto, e
        // nao acao. Ele reenquadra o que "meus objetivos" significa antes de o
        // modelo ler o que da para fazer com eles.
        today_block(&input.today.0, &input.today.1),
        crate::action_contract(input.finance_enabled),
    ];
    if input.hops_left > 0 {
        blocos.push(query_contract());
    }
    blocos.push(candidates_block(input.candidates));
    blocos.concat()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn candidato(kind: EntityKind, id: &str, label: &str) -> Candidate {
        Candidate {
            kind,
            id: id.to_owned(),
            label: label.to_owned(),
            detail: String::new(),
        }
    }

    /// O bloco de identidade tem uma frase que nao pode sair: a que impede a
    /// duplicata. Sem ela, "a task ja cadastrada" vira uma segunda Task.
    #[test]
    fn the_identity_block_forbids_duplicating_what_exists() {
        let context = system_context();
        assert!(context.contains("Kanban"));
        assert!(context.contains("Nunca crie entidade nova"));
        assert!(context.contains("OPERAR"));
    }

    /// O modelo nao tem relogio, e "hoje as 20:30" sem ancora vira palpite.
    #[test]
    fn the_now_block_spells_the_date_the_weekday_and_the_offset() {
        let bloco = now_block(datetime!(2026-08-20 14:32:00 -03:00));
        assert!(bloco.contains("quinta-feira"), "{bloco}");
        assert!(bloco.contains("20 de agosto de 2026"), "{bloco}");
        assert!(bloco.contains("14:32"), "{bloco}");
        assert!(bloco.contains("UTC-03:00"), "{bloco}");
    }

    /// Fuso positivo tambem precisa sair certo: o sinal vem do offset, e nao de
    /// uma suposicao sobre o Brasil.
    #[test]
    fn a_positive_offset_keeps_its_sign() {
        let bloco = now_block(datetime!(2026-01-05 09:00:00 +05:30));
        assert!(bloco.contains("UTC+05:30"), "{bloco}");
        assert!(bloco.contains("segunda-feira"), "{bloco}");
    }

    /// Um bloco que so anuncia ausencia gastaria token para informar nada.
    #[test]
    fn an_empty_here_produces_no_block() {
        assert!(here_block(&Here::default()).is_empty());
    }

    #[test]
    fn the_here_block_names_what_is_open() {
        let here = Here {
            screen: "Kanban".into(),
            project: Some(Named::new("1f4c9a2b-0000-0000-0000-000000000000", "Minarum")),
            task: Some(Named::new("7c3e2b19-0000-0000-0000-000000000000", "Enviar bases")),
            workspace: None,
        };
        let bloco = here_block(&here);
        assert!(bloco.contains("Tela aberta: Kanban"));
        assert!(bloco.contains("Minarum (id 1f4c9a2b)"));
        assert!(bloco.contains("Enviar bases (id 7c3e2b19)"));
        // O id inteiro nao entra: trinta e seis caracteres por linha em toda
        // mensagem e um paragrafo gasto em hifens.
        assert!(!bloco.contains("0000-0000"));
    }

    /// Sem candidato, sem bloco — e a ausencia e deliberada. Um "nenhum
    /// resultado" convidaria o modelo a concluir que a entidade nao existe.
    #[test]
    fn no_candidates_means_no_block() {
        assert!(candidates_block(&[]).is_empty());
    }

    #[test]
    fn the_candidates_block_shows_kind_short_id_and_label() {
        let bloco = candidates_block(&[Candidate {
            kind: EntityKind::Task,
            id: "7c3e2b19-1111-2222-3333-444444444444".into(),
            label: "Enviar tipos de bases faltantes para o Victor".into(),
            detail: "doing · Minarum".into(),
        }]);
        assert!(bloco.contains("- task 7c3e2b19 · Enviar tipos de bases faltantes para o Victor · doing · Minarum"), "{bloco}");
        assert!(bloco.contains("Não crie uma entidade nova"));
    }

    /// O teto existe porque o bloco desce em toda mensagem e o usuario nao
    /// escolheu o que entrou nele.
    #[test]
    fn the_candidates_block_stops_at_the_ceiling() {
        let muitos: Vec<Candidate> = (0..40)
            .map(|indice| candidato(EntityKind::Task, &format!("{indice:08}-aaaa"), "Task"))
            .collect();
        let bloco = candidates_block(&muitos);
        assert_eq!(bloco.matches("- task ").count(), MAX_CANDIDATES);
    }

    // ------------------------------------------------------------- resolucao

    struct Item {
        id: String,
        label: String,
    }

    fn itens() -> Vec<Item> {
        [
            ("7c3e2b19-1111-2222-3333-444444444444", "Enviar tipos de bases faltantes para o Victor"),
            ("1f4c9a2b-1111-2222-3333-444444444444", "Revisar memorial descritivo"),
            ("aa11bb22-1111-2222-3333-444444444444", "Enviar proposta para o Victor"),
        ]
        .into_iter()
        .map(|(id, label)| Item {
            id: id.to_owned(),
            label: label.to_owned(),
        })
        .collect()
    }

    fn resolver(reference: &str) -> Resolved<&'static Item> {
        // Vaza de proposito no teste: o vetor precisa viver enquanto a
        // referencia emprestada existir.
        let items: &'static Vec<Item> = Box::leak(Box::new(itens()));
        resolve(items, reference, |item| item.id.clone(), |item| item.label.clone())
    }

    #[test]
    fn resolves_by_the_short_id_the_block_showed() {
        match resolver("7c3e2b19") {
            Resolved::One(item) => assert!(item.label.contains("bases faltantes")),
            outro => panic!("esperava um acerto, veio {outro:?}", outro = match outro {
                Resolved::Many(itens) => format!("{} acertos", itens.len()),
                _ => "nenhum".to_owned(),
            }),
        }
    }

    #[test]
    fn resolves_by_the_whole_id_too() {
        assert!(resolver("7c3e2b19-1111-2222-3333-444444444444").one().is_some());
    }

    /// Prefixo curto demais casaria com meia base por acaso.
    #[test]
    fn a_short_prefix_is_not_an_id() {
        assert!(matches!(resolver("7c3"), Resolved::None));
    }

    #[test]
    fn resolves_by_the_exact_title_ignoring_case_and_accents() {
        assert!(resolver("REVISAR MEMORIAL DESCRITIVO").one().is_some());
        assert!(resolver("revisar memorial descritivo").one().is_some());
    }

    /// O degrau que acerta decide. "Enviar" bate em dois titulos por "contem",
    /// e isso e duvida de verdade — nao se escolhe o primeiro da lista.
    #[test]
    fn an_ambiguous_fragment_asks_instead_of_guessing() {
        match resolver("Enviar") {
            Resolved::Many(achados) => assert_eq!(achados.len(), 2),
            _ => panic!("dois titulos comecam com Enviar"),
        }
    }

    /// O fragmento que so cabe em um acerta sozinho — e este e o caso do
    /// pedido: "enviar tipos de bases faltantes" tem de virar acao, nao pergunta.
    #[test]
    fn a_fragment_that_fits_only_one_acts_alone() {
        let achado = resolver("tipos de bases faltantes").one();
        assert!(achado.is_some());
        assert!(achado.unwrap().label.contains("Victor"));
    }

    #[test]
    fn an_unknown_reference_resolves_to_nothing() {
        assert!(matches!(resolver("memorial do prédio azul"), Resolved::None));
    }

    /// O erro de ambiguidade lista os titulos: "bate com 3" manda procurar, a
    /// lista deixa responder na propria frase.
    #[test]
    fn the_ambiguity_error_lists_the_titles() {
        let items = itens();
        let resolved = resolve(&items, "Enviar", |item| item.id.clone(), |item| item.label.clone());
        let error = resolution_error(&resolved, EntityKind::Task, "Enviar", |item: &Item| {
            item.label.clone()
        })
        .expect("ambiguidade precisa virar erro");
        assert!(error.message.contains("Victor"), "{}", error.message);
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn a_single_match_produces_no_error() {
        let items = itens();
        let resolved = resolve(&items, "7c3e2b19", |item| item.id.clone(), |item| item.label.clone());
        assert!(resolution_error(&resolved, EntityKind::Task, "7c3e2b19", |item: &Item| item
            .label
            .clone())
        .is_none());
    }

    // ----------------------------------------------------------------- termos

    /// O caso do pedido. O que sobra tem de ser o que identifica a Task — e
    /// "lembrete", "task" e "kanban" tem de sair, porque procurar por eles numa
    /// base em que tudo e task devolve tudo.
    #[test]
    fn the_motivating_sentence_keeps_only_what_identifies_the_task() {
        let termos = search_terms(
            "Criar lembrete para hoje de noite às 20:30 para enviar tipos de bases faltantes para o Victor, task já cadastrada no kanban.",
        );
        assert!(termos.contains(&"victor".to_owned()), "{termos:?}");
        assert!(termos.contains(&"bases".to_owned()), "{termos:?}");
        assert!(termos.contains(&"faltantes".to_owned()), "{termos:?}");
        assert!(!termos.contains(&"lembrete".to_owned()), "{termos:?}");
        assert!(!termos.contains(&"task".to_owned()), "{termos:?}");
        assert!(!termos.contains(&"kanban".to_owned()), "{termos:?}");
        assert!(!termos.contains(&"para".to_owned()), "{termos:?}");
    }

    /// Codigo de projeto nao pode ser partido: `063-26` virando `063` e `26`
    /// procuraria por dois numeros que aparecem em qualquer lugar.
    #[test]
    fn a_project_code_survives_whole() {
        let termos = search_terms("coloca isso no projeto 063-26");
        assert_eq!(termos, vec!["063-26".to_owned()]);
    }

    #[test]
    fn terms_are_deduped_and_capped() {
        let termos = search_terms("victor victor victor um dois tres quatro cinco seis sete oito nove dez onze");
        assert_eq!(termos.iter().filter(|termo| *termo == "victor").count(), 1);
        assert!(termos.len() <= MAX_TERMOS);
    }

    /// Uma frase que so tem ruido nao produz busca. Sem esta guarda, "cria uma
    /// task" viraria uma varredura por nada e traria os doze primeiros itens da
    /// base como se fossem candidatos.
    #[test]
    fn a_sentence_made_of_noise_produces_no_search() {
        assert!(search_terms("cria uma task").is_empty());
        assert!(search_terms("me lembra disso").is_empty());
    }

    // --------------------------------------------------------------- consulta

    #[test]
    fn parses_a_query_with_kinds() {
        let request = parse_query(r#"{"search":"victor bases","kinds":["task","projeto"]}"#).unwrap();
        assert_eq!(request.search, "victor bases");
        assert_eq!(request.kinds, vec![EntityKind::Task, EntityKind::Project]);
    }

    /// Um rotulo torto nao pode gastar o unico salto disponivel com um erro.
    #[test]
    fn an_unknown_kind_is_ignored_not_refused() {
        let request = parse_query(r#"{"search":"x","kinds":["task","quantum"]}"#).unwrap();
        assert_eq!(request.kinds, vec![EntityKind::Task]);
    }

    #[test]
    fn a_query_without_search_is_refused() {
        assert!(parse_query(r#"{"kinds":["task"]}"#).is_err());
        assert!(parse_query(r#"{"search":"   "}"#).is_err());
        assert!(parse_query("nao e json").is_err());
    }

    /// O resultado precisa se anunciar como sistema: sem isso o modelo trata a
    /// lista como fala do usuario e responde a ela em vez de continuar.
    #[test]
    fn the_answer_says_it_is_not_the_user_talking() {
        let request = QueryRequest {
            search: "victor".into(),
            kinds: Vec::new(),
        };
        let resposta = query_answer(&request, &[candidato(EntityKind::Task, "7c3e2b19-x", "Enviar")]);
        assert!(resposta.contains("não do usuário"));
        assert!(resposta.contains("task 7c3e2b19"));
        assert!(resposta.contains("Não peça outra busca"));
    }

    #[test]
    fn an_empty_answer_says_so_plainly() {
        let request = QueryRequest {
            search: "coisa que nao existe".into(),
            kinds: Vec::new(),
        };
        assert!(query_answer(&request, &[]).contains("Nenhuma entidade"));
    }

    // ----------------------------------------------------------------- blocos

    #[test]
    fn splits_a_fenced_block_and_removes_it_from_the_text() {
        let (texto, bloco) = split_fenced(
            "Vou buscar.\n```mos-query\n{\"search\":\"victor\"}\n```\nJa volto.",
            "mos-query",
        );
        assert_eq!(bloco.as_deref(), Some(r#"{"search":"victor"}"#));
        assert!(!texto.contains("mos-query"));
        assert!(texto.contains("Vou buscar."));
    }

    #[test]
    fn a_text_without_the_fence_comes_back_whole() {
        let (texto, bloco) = split_fenced("so texto", "mos-query");
        assert_eq!(texto, "so texto");
        assert!(bloco.is_none());
    }

    /// Cerca aberta e turno interrompido no meio do bloco. Sem fechamento nao
    /// ha bloco valido, e o texto fica como veio.
    #[test]
    fn an_unclosed_fence_is_not_a_block() {
        let (texto, bloco) = split_fenced("```mos-query\n{\"sea", "mos-query");
        assert!(bloco.is_none());
        assert!(texto.contains("mos-query"));
    }

    /// As duas cercas convivem na mesma resposta sem se comerem.
    #[test]
    fn action_and_query_fences_do_not_collide() {
        let texto = "```mos-action\n{\"action\":\"mos.task.create\"}\n```";
        let (_, query) = split_fenced(texto, "mos-query");
        assert!(query.is_none());
        let (_, action) = split_fenced(texto, "mos-action");
        assert!(action.is_some());
    }

    // -------------------------------------------------------------- preambulo

    #[test]
    fn the_preamble_carries_every_block_in_reading_order() {
        let here = Here {
            screen: "Kanban".into(),
            ..Default::default()
        };
        let candidatos = [candidato(EntityKind::Task, "7c3e2b19-x", "Enviar bases")];
        let texto = preamble(PreambleInput {
            now_local: datetime!(2026-08-20 14:32:00 -03:00),
            here: &here,
            candidates: &candidatos,
            finance_enabled: false,
            hops_left: MAX_QUERY_HOPS,
            today: (String::new(), Vec::new()),
        });

        let identidade = texto.find("[Quem você é]").expect("identidade");
        let agora = texto.find("[Agora]").expect("agora");
        let onde = texto.find("[Onde o usuário está").expect("onde");
        let acoes = texto.find("[Ações disponíveis").expect("acoes");
        let entidades = texto.find("[Entidades do M/OS").expect("entidades");

        assert!(identidade < agora);
        assert!(agora < onde);
        assert!(onde < acoes);
        assert!(acoes < entidades);
    }

    /// O caso obrigatorio do pedido, na parte que e deterministica.
    ///
    /// O que este teste NAO cobre e o meio: a escolha do modelo. O que ele
    /// cobre e tudo o que o M/OS controla dos dois lados dessa escolha — que a
    /// Task existente chega ao prompt com id, e que a proposta que volta
    /// apontando para esse id vira um lembrete VINCULADO, e nao uma Task nova.
    ///
    /// Antes deste trabalho os dois lados estavam quebrados: nao havia bloco de
    /// candidatos, entao a Task nao chegava; e nao havia acao de lembrete,
    /// entao a unica coisa que o modelo podia propor era `mos.task.create` —
    /// literalmente a duplicata que o pedido proibe.
    #[test]
    fn the_mandatory_case_carries_the_task_into_the_prompt_and_back_out_as_a_link() {
        let pergunta = "Criar lembrete para hoje de noite às 20:30 para enviar \
                        tipos de bases faltantes para o Victor, task já cadastrada no kanban.";
        let agora = datetime!(2026-08-20 14:32:00 -03:00);

        // O que a busca automatica do M/OS encontraria com os termos da frase.
        let termos = search_terms(pergunta);
        assert!(termos.contains(&"victor".to_owned()), "{termos:?}");

        let task = Candidate {
            kind: EntityKind::Task,
            id: "7c3e2b19-1111-2222-3333-444444444444".into(),
            label: "Enviar tipos de bases faltantes para o Victor".into(),
            detail: "doing · Minarum".into(),
        };
        let here = Here {
            screen: "Kanban".into(),
            ..Default::default()
        };
        let prompt = preamble(PreambleInput {
            now_local: agora,
            here: &here,
            candidates: std::slice::from_ref(&task),
            finance_enabled: false,
            hops_left: MAX_QUERY_HOPS,
            today: (String::new(), Vec::new()),
        });

        // A Task chega ao modelo com id, e o lembrete e uma acao que existe.
        assert!(prompt.contains("task 7c3e2b19"), "{prompt}");
        assert!(prompt.contains("mos.reminder.create"), "{prompt}");
        // E ele sabe que dia e hoje, sem o que "20:30" nao e uma hora.
        assert!(prompt.contains("20 de agosto de 2026"));
        assert!(prompt.contains("Tela aberta: Kanban"));
        // E sabe que nao pode criar a Task de novo.
        assert!(prompt.contains("Nunca crie entidade nova"));

        // A resposta plausivel do modelo, com a cerca que o contrato ensinou.
        let resposta = "Criei o lembrete e liguei à task que já estava no Kanban.\n\n\
             ```mos-action\n\
             { \"action\": \"mos.reminder.create\", \"args\": { \
             \"title\": \"Enviar tipos de bases faltantes para o Victor\", \
             \"when\": \"hoje às 20:30\", \"taskRef\": \"7c3e2b19\" } }\n\
             ```";
        let (texto, bloco) = split_fenced(resposta, "mos-action");
        assert!(!texto.contains("mos-action"));
        let args = crate::parse_action_at(&bloco.expect("a proposta"), agora).unwrap();

        // O ponto do pedido: lembrete, e nao Task.
        assert_eq!(args.kind(), crate::ActionKind::ReminderCreate);
        match &args {
            crate::ActionArgs::ReminderCreate { at, target, .. } => {
                assert!(at.starts_with("2026-08-20T20:30:00"), "{at}");
                assert_eq!(target.as_ref().unwrap().reference, "7c3e2b19");
            }
            outro => panic!("virou outra acao: {outro:?}"),
        }

        // E o id do bloco resolve de volta na Task que o originou.
        let base = [task];
        assert!(matches!(
            resolve(
                &base,
                "7c3e2b19",
                |candidate| candidate.id.clone(),
                |candidate| candidate.label.clone()
            ),
            Resolved::One(_)
        ));
    }

    /// Oferecer uma ferramenta que nao vai ser executada ensinaria o modelo a
    /// pedi-la e receber silencio.
    #[test]
    fn the_query_contract_disappears_on_the_last_hop() {
        let here = Here::default();
        let com_salto = preamble(PreambleInput {
            now_local: datetime!(2026-08-20 14:32:00 -03:00),
            here: &here,
            candidates: &[],
            finance_enabled: false,
            hops_left: 1,
            today: (String::new(), Vec::new()),
        });
        let sem_salto = preamble(PreambleInput {
            now_local: datetime!(2026-08-20 14:32:00 -03:00),
            here: &here,
            candidates: &[],
            finance_enabled: false,
            hops_left: 0,
            today: (String::new(), Vec::new()),
        });
        assert!(com_salto.contains("mos-query"));
        assert!(!sem_salto.contains("mos-query"));
    }

    /// Um bloco que so anuncia ausencia gastaria token em toda mensagem para
    /// informar nada. Mesma regra do `here_block`.
    #[test]
    fn um_dia_sem_objetivos_nao_produz_bloco() {
        assert!(today_block("2026-08-21", &[]).is_empty());
    }

    /// "O que falta dos meus objetivos de hoje?" e uma PERGUNTA. Ela se responde
    /// pelo preambulo, e nao gastando um turno de proposta e confirmacao.
    #[test]
    fn o_bloco_de_hoje_diz_o_que_falta_e_o_que_ja_foi() {
        let bloco = today_block(
            "2026-08-21",
            &[
                ("Finalizar planta de formas".into(), "principal".into(), false),
                ("Revisar memorial".into(), "secundário".into(), true),
            ],
        );
        assert!(bloco.contains("2026-08-21"), "{bloco}");
        assert!(bloco.contains("[ ] Finalizar planta de formas (principal)"), "{bloco}");
        assert!(bloco.contains("[x] Revisar memorial (secundário)"), "{bloco}");
        assert!(bloco.contains("1 de 2 concluídos"), "{bloco}");
        // A frase que impede a duplicata, do mesmo jeito que o bloco de
        // identidade impede "cria a task que ja existe".
        assert!(bloco.contains("nunca crie Task para representar um"), "{bloco}");
    }

    #[test]
    fn o_bloco_de_hoje_entra_no_preambulo_antes_do_catalogo() {
        let prompt = preamble(PreambleInput {
            now_local: datetime!(2026-08-21 09:08:00 -03:00),
            here: &Here::default(),
            candidates: &[],
            finance_enabled: false,
            hops_left: 0,
            today: (
                "2026-08-21".into(),
                vec![("Planta de formas".into(), "principal".into(), false)],
            ),
        });
        let dia = prompt.find("Os objetivos de hoje").expect("o bloco desce");
        let acoes = prompt.find("Ações disponíveis").expect("o catalogo desce");
        assert!(dia < acoes, "o dia e contexto, e contexto vem antes do que da para fazer");
    }

    /// O vocabulario de entidade atravessa a ponte e volta. Um tipo novo sem
    /// `parse` seria um candidato que o modelo cita e o M/OS nao reconhece.
    #[test]
    fn objetivo_do_dia_e_um_tipo_de_entidade_citavel() {
        assert_eq!(EntityKind::parse("objetivo"), Some(EntityKind::DailyObjective));
        assert_eq!(EntityKind::parse("daily_objective"), Some(EntityKind::DailyObjective));
        assert_eq!(EntityKind::DailyObjective.as_str(), "objetivo");
    }
}
