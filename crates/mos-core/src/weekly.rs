//! Weekly Review: o fecho da semana sobre as Daily Sessions.

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    CoreError, DailyObjective, DailyObjectiveId, DailyReflection, DailySession, Day, DayMood,
    ErrorCode, ObjectiveLink, ObjectivePriority, ObjectiveStatus,
};

/// Uma semana civil, identificada pela **data da segunda-feira**.
///
/// # Por que a segunda, e nao o numero ISO
///
/// Numero ISO tem duas armadilhas que a data da segunda simplesmente nao tem:
/// **semanas 53**, e o 1º de janeiro que pertence a semana 52 do ano anterior.
/// Guardar `2026-W01` obrigaria a escolher uma convencao de virada de ano e a
/// acerta-la em todo lugar que compara; guardar `2026-08-17` nao obriga a nada.
///
/// # Por que civil e fixa, e nao sete dias deslizantes
///
/// Um "fecho" de janela deslizante nao fecha nada, e a unicidade precisa de uma
/// chave. E a mesma razao pela qual [`Day`] existe como campo em vez de ser
/// decidido por cada leitor.
///
/// **Esta e a unica copia da regra.** Nada de `date(day, 'weekday 0', '-6
/// days')` em SQL: a semana calculada em dois lugares e como as duas versoes
/// divergem.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Week(Day);

impl Week {
    /// A semana que contem este dia.
    pub fn containing(day: &Day) -> Result<Self, CoreError> {
        let date = day.date()?;
        // `number_days_from_monday` da 0 na segunda e 6 no domingo, entao a
        // segunda nao anda e o domingo volta seis. As duas bordas sao o caso
        // que uma conta ingenua erra.
        let recuo = i64::from(date.weekday().number_days_from_monday());
        Day::from_date(date - Duration::days(recuo)).map(Self)
    }

    /// Le a data de uma segunda-feira. Recusa qualquer outro dia.
    pub fn parse(value: &str) -> Result<Self, CoreError> {
        let day = Day::parse(value)?;
        let date = day.date()?;
        if date.weekday().number_days_from_monday() != 0 {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "A semana e identificada pela segunda-feira.",
                false,
            ));
        }
        Ok(Self(day))
    }

    pub fn start(&self) -> &Day {
        &self.0
    }

    pub fn end(&self) -> Result<Day, CoreError> {
        Day::from_date(self.0.date()? + Duration::days(6))
    }

    pub fn previous(&self) -> Result<Self, CoreError> {
        Day::from_date(self.0.date()? - Duration::days(7)).map(Self)
    }

    pub fn next(&self) -> Result<Self, CoreError> {
        Day::from_date(self.0.date()? + Duration::days(7)).map(Self)
    }

    /// As duas bordas entram.
    pub fn contains(&self, day: &Day) -> Result<bool, CoreError> {
        Ok(*day >= *self.start() && *day <= self.end()?)
    }
}

impl std::fmt::Display for Week {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeeklyReviewId(Uuid);

impl WeeklyReviewId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        Uuid::parse_str(value).map(Self).map_err(|_| {
            CoreError::new(
                ErrorCode::InvalidInput,
                "ID de fecho de semana invalido.",
                false,
            )
        })
    }

    /// O UUID cru, para a sincronizacao. Ver `docs/SYNC.md`.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for WeeklyReviewId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WeeklyReviewId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// O fecho de uma semana.
///
/// **Minusculo de proposito: a narrativa inteira e DERIVADA.** Guardar o resumo
/// duplicaria dado para exibir noutra superficie, que o `CORE-FOUNDATION.md` §2
/// principio 6 proibe — e ele envelheceria: reabrir um objetivo de terca
/// mudaria a semana, e o texto gravado continuaria dizendo o contrario.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyReview {
    pub id: WeeklyReviewId,
    pub week: Week,
    /// Vazio e legitimo: fechar a semana e o gesto, escrever e opcional.
    pub summary: String,
    #[serde(with = "time::serde::rfc3339")]
    pub closed_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug)]
pub struct NewWeeklyReview {
    pub id: WeeklyReviewId,
    pub week: Week,
    pub summary: String,
    pub closed_at: OffsetDateTime,
}

/// Teto do texto da semana. Sete dias cabem em dois paragrafos; o que passa
/// disso e journaling, que este desenho recusou por nome.
const MAX_SUMMARY: usize = 4_000;

impl NewWeeklyReview {
    /// Texto vazio NAO impede o fecho.
    ///
    /// Difere do `NewDailyReflection::create`, que devolve `None` quando nao ha
    /// nada a guardar: la a reflexao e acessorio do encerramento; aqui ela e o
    /// unico campo, e a linha precisa existir para a semana constar como
    /// fechada.
    pub fn create(week: Week, summary: &str, now: OffsetDateTime) -> Self {
        let summary = summary.trim();
        Self {
            id: WeeklyReviewId::new(),
            week,
            summary: summary.chars().take(MAX_SUMMARY).collect(),
            closed_at: now,
        }
    }
}

/// O que ocupou a semana: um Project, ou um objetivo que se repetiu.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dominant {
    pub label: String,
    /// Em quantos dias isto foi o objetivo PRINCIPAL.
    pub main_days: usize,
    /// Em quantos dias apareceu, de qualquer peso.
    pub days: usize,
}

/// Um objetivo que atravessou a semana sendo adiado.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recurring {
    pub title: String,
    pub times_carried: usize,
}

/// A semana em narrativa. **Nenhum placar.**
///
/// `ATTENTION-SYSTEM.md` §19 proibe resumo de produtividade em digest semanal,
/// e a razao vale aqui: um numero que soma sete dias de decisoes numa fracao
/// ensina a inflar o denominador na segunda e a evitar objetivo dificil na
/// quinta. A unica contagem que sobrevive e `days_with_session`, que e fato
/// sobre o uso do sistema e nao sobre o trabalho.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekSummary {
    pub week: Week,
    pub days_with_session: usize,
    pub dominated: Vec<Dominant>,
    pub recurring: Vec<Recurring>,
    pub dropped: Vec<String>,
    pub blocked_days: Vec<Day>,
    pub review: Option<WeeklyReview>,
    /// Nenhuma sessao na semana. A tela usa isto para NAO oferecer o fecho.
    pub empty: bool,
}

/// Tudo o que [`compose_week`] precisa ler.
///
/// Estrutura em vez de seis parametros soltos, pela mesma razao do
/// `ComposeInput` do calendario: trocar duas colecoes de lugar por engano
/// compilaria sem reclamacao nenhuma.
pub struct WeekInput<'a> {
    pub week: Week,
    pub sessions: &'a [DailySession],
    pub objectives: &'a [DailyObjective],
    pub reflections: &'a [DailyReflection],
    /// Como achar o Project de um vinculo. Fechamento e nao mapa pronto porque
    /// so o comando do desktop conhece Tasks e Projects.
    pub project_of: &'a dyn Fn(&ObjectiveLink) -> Option<String>,
    pub carry_depth: &'a dyn Fn(DailyObjectiveId) -> usize,
}

/// A partir de quantos elos um carry-over vira assunto.
///
/// Dois, e o mesmo corte do `avisoDeCarregado` no front: quase todo carry-over
/// veio de ontem, e "veio de ontem" e ruido.
const MIN_CORRENTE: usize = 2;

/// Monta a narrativa da semana a partir do que ja foi lido.
///
/// PURA e sem repositorio, igual ao `calendar::compose` e ao
/// `daily::compose_context`: e ela que carrega as regras que podem estar
/// erradas — como se agrupa o que dominou, o que conta como recorrente, o que
/// fica de fora —, e regra sem teste e regra que ninguem conferiu.
pub fn compose_week(input: WeekInput<'_>) -> Result<WeekSummary, CoreError> {
    use std::collections::{HashMap, HashSet};

    // As sessoes DA SEMANA. O repositorio ja filtra, e filtrar de novo aqui e
    // barato: confiar no chamador seria a semana mostrar o trabalho de outra.
    let mut dias: Vec<&DailySession> = Vec::new();
    for session in input.sessions {
        if input.week.contains(&session.day)? {
            dias.push(session);
        }
    }
    let da_semana: HashSet<_> = dias.iter().map(|sessao| sessao.id).collect();
    let dia_da_sessao: HashMap<_, _> = dias
        .iter()
        .map(|sessao| (sessao.id, sessao.day.clone()))
        .collect();

    let meus: Vec<&DailyObjective> = input
        .objectives
        .iter()
        .filter(|objetivo| da_semana.contains(&objetivo.session_id))
        .collect();

    // ---------------------------------------------------------- o que dominou
    //
    // A chave e o Project quando o vinculo resolve, e o titulo normalizado
    // quando nao resolve. Agrupar so por Project deixaria a unica secao que
    // responde "onde foi meu tempo" vazia numa semana inteira de texto livre —
    // que e o caso mais comum de quem esta comecando.
    struct Acumulado {
        label: String,
        principais: HashSet<Day>,
        dias: HashSet<Day>,
    }
    let mut grupos: HashMap<String, Acumulado> = HashMap::new();
    for objetivo in &meus {
        let Some(dia) = dia_da_sessao.get(&objetivo.session_id).cloned() else {
            continue;
        };
        let project = objetivo
            .link
            .as_ref()
            .and_then(|link| (input.project_of)(link));
        let (chave, rotulo) = match project {
            Some(nome) => (format!("p:{}", crate::normalize(&nome)), nome),
            None => (
                format!("t:{}", crate::normalize(&objetivo.title)),
                objetivo.title.clone(),
            ),
        };
        let entrada = grupos.entry(chave).or_insert_with(|| Acumulado {
            // O rotulo e o PRIMEIRO titulo escrito, e nao a forma normalizada:
            // ninguem quer ler o proprio objetivo sem acento e em caixa baixa.
            label: rotulo,
            principais: HashSet::new(),
            dias: HashSet::new(),
        });
        entrada.dias.insert(dia.clone());
        if objetivo.priority == ObjectivePriority::Main {
            entrada.principais.insert(dia);
        }
    }
    let mut dominated: Vec<Dominant> = grupos
        .into_values()
        .map(|acumulado| Dominant {
            label: acumulado.label,
            main_days: acumulado.principais.len(),
            days: acumulado.dias.len(),
        })
        .collect();
    // Ser principal tres vezes e um fato mais forte que aparecer cinco vezes
    // como secundario. Empate desempata pelo rotulo, para a ordem nao dancar
    // entre duas leituras — `HashMap` nao promete ordem nenhuma.
    dominated.sort_by(|esquerda, direita| {
        direita
            .main_days
            .cmp(&esquerda.main_days)
            .then(direita.days.cmp(&esquerda.days))
            .then(esquerda.label.cmp(&direita.label))
    });

    // ------------------------------------------------ o que voltou toda vez
    //
    // Uma corrente que atravessa a semana aparece UMA vez, com a profundidade
    // do elo mais recente: cinco linhas iguais seriam a mesma informacao
    // repetida cinco vezes.
    let elos_anteriores: HashSet<DailyObjectiveId> = meus
        .iter()
        .filter_map(|objetivo| objetivo.carried_from)
        .collect();
    let mut recurring: Vec<Recurring> = meus
        .iter()
        // Quem e elo intermediario DENTRO da semana sai: o elo final carrega a
        // corrente inteira.
        .filter(|objetivo| !elos_anteriores.contains(&objetivo.id))
        .filter_map(|objetivo| {
            let vezes = (input.carry_depth)(objetivo.id);
            (vezes >= MIN_CORRENTE).then(|| Recurring {
                title: objetivo.title.clone(),
                times_carried: vezes,
            })
        })
        .collect();
    recurring.sort_by(|esquerda, direita| {
        direita
            .times_carried
            .cmp(&esquerda.times_carried)
            .then(esquerda.title.cmp(&direita.title))
    });

    // -------------------------------------------------- o que voce largou
    let mut dropped: Vec<String> = meus
        .iter()
        .filter(|objetivo| objetivo.status == ObjectiveStatus::Dropped)
        .map(|objetivo| objetivo.title.clone())
        .collect();
    dropped.sort();
    dropped.dedup();

    // ------------------------------------------------------ dias travados
    let mut blocked_days: Vec<Day> = input
        .reflections
        .iter()
        .filter(|reflexao| reflexao.mood == Some(DayMood::Blocked))
        .filter_map(|reflexao| dia_da_sessao.get(&reflexao.session_id).cloned())
        .collect();
    blocked_days.sort();
    blocked_days.dedup();

    Ok(WeekSummary {
        week: input.week,
        days_with_session: dias.len(),
        dominated,
        recurring,
        dropped,
        blocked_days,
        review: None,
        empty: dias.is_empty(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        DailyObjective, DailyObjectiveId, DailyReflection, DailySession, DailySessionId, DayMood,
        LinkKind, ObjectiveLink, ObjectivePriority, ObjectiveStatus, SessionStatus,
    };
    use time::macros::datetime;
    use time::OffsetDateTime;

    fn instante() -> OffsetDateTime {
        datetime!(2026-08-17 09:00 -03:00)
    }

    fn sessao(day: &str) -> DailySession {
        DailySession {
            id: DailySessionId::new(),
            day: dia(day),
            status: SessionStatus::Completed,
            note: String::new(),
            started_at: instante(),
            ended_at: Some(instante()),
            created_at: instante(),
            updated_at: instante(),
        }
    }

    fn objetivo(
        sessao: &DailySession,
        title: &str,
        priority: ObjectivePriority,
        status: ObjectiveStatus,
        link: Option<ObjectiveLink>,
    ) -> DailyObjective {
        DailyObjective {
            id: DailyObjectiveId::new(),
            session_id: sessao.id,
            title: title.to_owned(),
            description: String::new(),
            link,
            priority,
            status,
            position: 0,
            carried_from: None,
            created_at: instante(),
            updated_at: instante(),
            completed_at: None,
        }
    }

    fn link_task(id: &str) -> ObjectiveLink {
        ObjectiveLink::new(LinkKind::Task, id).unwrap()
    }

    const TASK_A: &str = "018f0000-0000-7000-8000-0000000000a1";
    const TASK_B: &str = "018f0000-0000-7000-8000-0000000000b2";

    fn semana_de_teste() -> Week {
        Week::containing(&dia("2026-08-19")).unwrap()
    }

    /// Monta a entrada com fechamentos triviais. Os testes que precisam de
    /// Project ou de profundidade sobrescrevem depois.
    fn entrada<'a>(
        semana: &'a Week,
        sessions: &'a [DailySession],
        objectives: &'a [DailyObjective],
        reflections: &'a [DailyReflection],
        project_of: &'a dyn Fn(&ObjectiveLink) -> Option<String>,
        carry_depth: &'a dyn Fn(DailyObjectiveId) -> usize,
    ) -> WeekInput<'a> {
        WeekInput {
            week: semana.clone(),
            sessions,
            objectives,
            reflections,
            project_of,
            carry_depth,
        }
    }

    #[test]
    fn conta_dias_com_sessao_e_ignora_o_que_esta_fora_da_semana() {
        let semana = semana_de_teste();
        let sessoes = [
            sessao("2026-08-17"),
            sessao("2026-08-19"),
            sessao("2026-08-24"),
        ];
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &[],
            &[],
            &sem_project,
            &sem_profundidade,
        ))
        .unwrap();
        assert_eq!(resumo.days_with_session, 2, "24/08 e da semana seguinte");
        assert!(!resumo.empty);
    }

    #[test]
    fn semana_sem_sessao_nenhuma_e_marcada_como_vazia() {
        // A tela usa isto para NAO oferecer o fecho: nao ha o que revisar, e um
        // botao ali ensinaria que o M/OS quer um registro por semana mesmo
        // quando nao houve semana.
        let semana = semana_de_teste();
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &[],
            &[],
            &[],
            &sem_project,
            &sem_profundidade,
        ))
        .unwrap();
        assert!(resumo.empty);
        assert_eq!(resumo.days_with_session, 0);
    }

    #[test]
    fn o_que_dominou_agrupa_por_project_quando_o_vinculo_resolve() {
        let semana = semana_de_teste();
        let segunda = sessao("2026-08-17");
        let terca = sessao("2026-08-18");
        let objetivos = vec![
            objetivo(
                &segunda,
                "Planta de formas",
                ObjectivePriority::Main,
                ObjectiveStatus::Completed,
                Some(link_task(TASK_A)),
            ),
            objetivo(
                &terca,
                "Detalhamento",
                ObjectivePriority::Main,
                ObjectiveStatus::Pending,
                Some(link_task(TASK_B)),
            ),
        ];
        let sessoes = [segunda, terca];
        // As duas Tasks pertencem ao MESMO Project: e essa a agregacao que a
        // tela precisa, e ela e invisivel olhando so para os titulos.
        let project_of = |_: &ObjectiveLink| Some("063-26".to_owned());
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &objetivos,
            &[],
            &project_of,
            &sem_profundidade,
        ))
        .unwrap();
        assert_eq!(resumo.dominated.len(), 1);
        assert_eq!(resumo.dominated[0].label, "063-26");
        assert_eq!(resumo.dominated[0].main_days, 2);
        assert_eq!(resumo.dominated[0].days, 2);
    }

    #[test]
    fn o_que_dominou_cai_no_titulo_quando_nao_ha_vinculo() {
        // O caso mais comum do inicio. Agrupar so por Project deixaria a unica
        // secao que responde "onde foi meu tempo" vazia numa semana inteira de
        // texto livre.
        let semana = semana_de_teste();
        let segunda = sessao("2026-08-17");
        let terca = sessao("2026-08-18");
        let objetivos = vec![
            objetivo(
                &segunda,
                "Resolver pendencias financeiras",
                ObjectivePriority::Main,
                ObjectiveStatus::Completed,
                None,
            ),
            objetivo(
                &terca,
                "resolver PENDENCIAS financeiras",
                ObjectivePriority::Secondary,
                ObjectiveStatus::Pending,
                None,
            ),
        ];
        let sessoes = [segunda, terca];
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &objetivos,
            &[],
            &sem_project,
            &sem_profundidade,
        ))
        .unwrap();
        assert_eq!(
            resumo.dominated.len(),
            1,
            "caixa diferente e o mesmo assunto"
        );
        assert_eq!(resumo.dominated[0].main_days, 1);
        assert_eq!(resumo.dominated[0].days, 2);
        assert_eq!(
            resumo.dominated[0].label, "Resolver pendencias financeiras",
            "o rotulo e o primeiro titulo escrito, e nao a forma normalizada"
        );
    }

    #[test]
    fn dominou_ordena_por_dias_como_principal() {
        // Ser principal tres vezes e um fato mais forte que aparecer cinco
        // vezes como secundario.
        let semana = semana_de_teste();
        let dias: Vec<DailySession> = ["2026-08-17", "2026-08-18", "2026-08-19", "2026-08-20"]
            .iter()
            .map(|valor| sessao(valor))
            .collect();
        let mut objetivos = Vec::new();
        for dia_da_semana in &dias[..2] {
            objetivos.push(objetivo(
                dia_da_semana,
                "Principal duas vezes",
                ObjectivePriority::Main,
                ObjectiveStatus::Pending,
                None,
            ));
        }
        for dia_da_semana in &dias {
            objetivos.push(objetivo(
                dia_da_semana,
                "Secundario sempre",
                ObjectivePriority::Secondary,
                ObjectiveStatus::Pending,
                None,
            ));
        }
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &dias,
            &objetivos,
            &[],
            &sem_project,
            &sem_profundidade,
        ))
        .unwrap();
        assert_eq!(resumo.dominated[0].label, "Principal duas vezes");
        assert_eq!(resumo.dominated[1].label, "Secundario sempre");
        assert_eq!(resumo.dominated[1].days, 4, "e ele apareceu mais vezes");
    }

    #[test]
    fn o_que_voltou_toda_vez_corta_em_dois_e_aparece_uma_vez_so() {
        let semana = semana_de_teste();
        let segunda = sessao("2026-08-17");
        let terca = sessao("2026-08-18");
        let veterano_a = objetivo(
            &segunda,
            "Atualizar documentacao",
            ObjectivePriority::Secondary,
            ObjectiveStatus::CarriedOver,
            None,
        );
        let mut veterano_b = objetivo(
            &terca,
            "Atualizar documentacao",
            ObjectivePriority::Secondary,
            ObjectiveStatus::CarriedOver,
            None,
        );
        // A corrente: o de terca veio do de segunda.
        veterano_b.carried_from = Some(veterano_a.id);
        let novato = objetivo(
            &terca,
            "Veio de ontem",
            ObjectivePriority::Secondary,
            ObjectiveStatus::Pending,
            None,
        );

        let id_a = veterano_a.id;
        let id_b = veterano_b.id;
        let objetivos = vec![veterano_a, veterano_b, novato];
        let sessoes = [segunda, terca];
        let sem_project = |_: &ObjectiveLink| None;
        let profundidade = move |id: DailyObjectiveId| {
            if id == id_a {
                3
            } else if id == id_b {
                4
            } else {
                1
            }
        };
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &objetivos,
            &[],
            &sem_project,
            &profundidade,
        ))
        .unwrap();
        assert_eq!(
            resumo.recurring.len(),
            1,
            "a corrente e uma linha, e nao duas"
        );
        assert_eq!(resumo.recurring[0].title, "Atualizar documentacao");
        assert_eq!(
            resumo.recurring[0].times_carried, 4,
            "a profundidade e a do elo mais recente"
        );
    }

    #[test]
    fn o_que_voce_largou_lista_os_abandonados() {
        let semana = semana_de_teste();
        let segunda = sessao("2026-08-17");
        let objetivos = vec![
            objetivo(
                &segunda,
                "Revisar proposta antiga",
                ObjectivePriority::Secondary,
                ObjectiveStatus::Dropped,
                None,
            ),
            objetivo(
                &segunda,
                "Feito",
                ObjectivePriority::Main,
                ObjectiveStatus::Completed,
                None,
            ),
        ];
        let sessoes = [segunda];
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &objetivos,
            &[],
            &sem_project,
            &sem_profundidade,
        ))
        .unwrap();
        assert_eq!(resumo.dropped, ["Revisar proposta antiga"]);
    }

    #[test]
    fn dias_travados_vem_dos_humores_ja_respondidos() {
        // NAO e uma pergunta nova: perguntar o humor da semana no domingo seria
        // pedir a mesma coisa uma oitava vez, com menos precisao.
        let semana = semana_de_teste();
        let quarta = sessao("2026-08-19");
        let quinta = sessao("2026-08-20");
        let sexta = sessao("2026-08-21");
        let reflexoes = vec![
            DailyReflection {
                session_id: quarta.id,
                mood: Some(DayMood::Blocked),
                summary: String::new(),
                created_at: instante(),
                updated_at: instante(),
            },
            DailyReflection {
                session_id: quinta.id,
                mood: Some(DayMood::Blocked),
                summary: String::new(),
                created_at: instante(),
                updated_at: instante(),
            },
            DailyReflection {
                session_id: sexta.id,
                mood: Some(DayMood::Productive),
                summary: String::new(),
                created_at: instante(),
                updated_at: instante(),
            },
        ];
        let sessoes = [quarta, quinta, sexta];
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &[],
            &reflexoes,
            &sem_project,
            &sem_profundidade,
        ))
        .unwrap();
        let dias: Vec<&str> = resumo
            .blocked_days
            .iter()
            .map(|valor| valor.as_str())
            .collect();
        assert_eq!(dias, ["2026-08-19", "2026-08-20"]);
    }

    #[test]
    fn objetivo_de_sessao_fora_da_semana_nao_entra_em_nada() {
        // As sessoes chegam filtradas pelo repositorio, mas os objetivos podem
        // vir de um lote maior. Confiar no chamador aqui seria a semana mostrar
        // o trabalho de outra.
        let semana = semana_de_teste();
        let dentro = sessao("2026-08-18");
        let fora = sessao("2026-08-25");
        let objetivos = vec![
            objetivo(
                &dentro,
                "Da semana",
                ObjectivePriority::Main,
                ObjectiveStatus::Dropped,
                None,
            ),
            objetivo(
                &fora,
                "De outra semana",
                ObjectivePriority::Main,
                ObjectiveStatus::Dropped,
                None,
            ),
        ];
        let sessoes = [dentro];
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &objetivos,
            &[],
            &sem_project,
            &sem_profundidade,
        ))
        .unwrap();
        assert_eq!(resumo.dropped, ["Da semana"]);
    }

    fn dia(valor: &str) -> Day {
        Day::parse(valor).unwrap()
    }

    #[test]
    fn a_semana_e_a_segunda_que_contem_o_dia() {
        // 2026-08-21 e uma sexta. A semana dela comeca em 17 e termina em 23.
        let semana = Week::containing(&dia("2026-08-21")).unwrap();
        assert_eq!(semana.start().as_str(), "2026-08-17");
        assert_eq!(semana.end().unwrap().as_str(), "2026-08-23");
    }

    #[test]
    fn a_segunda_e_o_domingo_caem_na_mesma_semana() {
        // As duas bordas sao o caso que uma conta de "menos N dias" erra: a
        // segunda nao pode andar para tras, e o domingo nao pode virar a
        // semana seguinte.
        let segunda = Week::containing(&dia("2026-08-17")).unwrap();
        let domingo = Week::containing(&dia("2026-08-23")).unwrap();
        assert_eq!(segunda, domingo);
        assert_eq!(segunda.start().as_str(), "2026-08-17");
    }

    #[test]
    fn a_semana_atravessa_mes_e_ano_sem_numero_iso() {
        // 2026-01-01 e uma quinta, e a semana dela comeca em 2025-12-29. Com
        // numero ISO isto seria "semana 1 de 2026" comecando em 2025, que e a
        // convencao que este desenho recusa por nao precisar dela.
        let virada = Week::containing(&dia("2026-01-01")).unwrap();
        assert_eq!(virada.start().as_str(), "2025-12-29");
        assert_eq!(virada.end().unwrap().as_str(), "2026-01-04");
    }

    #[test]
    fn anterior_e_proxima_andam_sete_dias() {
        let semana = Week::containing(&dia("2026-03-02")).unwrap();
        assert_eq!(semana.previous().unwrap().start().as_str(), "2026-02-23");
        assert_eq!(semana.next().unwrap().start().as_str(), "2026-03-09");
    }

    #[test]
    fn ano_bissexto_nao_quebra_a_conta() {
        // 2028 e bissexto. A semana que contem 29/02 comeca em 28/02.
        let semana = Week::containing(&dia("2028-02-29")).unwrap();
        assert_eq!(semana.start().as_str(), "2028-02-28");
        assert_eq!(semana.next().unwrap().start().as_str(), "2028-03-06");
    }

    #[test]
    fn a_semana_so_aceita_segunda_feira() {
        // Ela e CHAVE: duas representacoes do mesmo intervalo criariam duas
        // linhas para a mesma semana, e o indice unico nao veria a duplicata.
        assert!(Week::parse("2026-08-17").is_ok());
        assert!(
            Week::parse("2026-08-21").is_err(),
            "sexta nao e inicio de semana"
        );
        assert!(
            Week::parse("2026-8-17").is_err(),
            "sem zero a esquerda e outra chave"
        );
        assert!(Week::parse("").is_err());
    }

    #[test]
    fn contains_responde_pelas_duas_bordas() {
        let semana = Week::containing(&dia("2026-08-19")).unwrap();
        assert!(semana.contains(&dia("2026-08-17")).unwrap());
        assert!(semana.contains(&dia("2026-08-23")).unwrap());
        assert!(!semana.contains(&dia("2026-08-16")).unwrap());
        assert!(!semana.contains(&dia("2026-08-24")).unwrap());
    }

    #[test]
    fn a_semana_atravessa_a_ponte_como_a_data_da_segunda() {
        // O nome vai para o TypeScript e para o banco. Um formato diferente de
        // cada lado faria a tela deixar de reconhecer a semana sem erro de
        // compilacao de nenhum dos dois.
        let semana = Week::containing(&dia("2026-08-19")).unwrap();
        assert_eq!(serde_json::to_string(&semana).unwrap(), "\"2026-08-17\"");
        assert_eq!(
            serde_json::from_str::<Week>("\"2026-08-17\"").unwrap(),
            semana
        );
    }

    #[test]
    fn as_semanas_se_ordenam_no_tempo() {
        // `pending_week` escolhe a mais recente com `max()`. Sem Ord correto,
        // ela escolheria por ordem alfabetica — que por sorte coincide neste
        // formato, e e exatamente o tipo de sorte que quebra quando o formato
        // muda.
        let antes = Week::containing(&dia("2025-12-31")).unwrap();
        let depois = Week::containing(&dia("2026-01-05")).unwrap();
        assert!(antes < depois);
    }
}
