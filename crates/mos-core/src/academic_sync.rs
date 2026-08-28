//! O motor de sincronizacao academica — **neutro de provedor**.
//!
//! Este modulo nao sabe o que e Univirtus. Ele conhece cinco tipos externos
//! (`External*`), um retrato (`ProviderSnapshot`) e uma reconciliacao pura que
//! compara o retrato com o que o M/OS ja guardou. Quem traduz JSON de um AVA
//! especifico para estes tipos e o normalizador daquele AVA — hoje
//! `academic_univirtus`, amanha outro — e quem persiste e o storage.
//!
//! # Por que a reconciliacao e pura
//!
//! Ela recebe o retrato e as referencias externas ja gravadas, e devolve um
//! PLANO. Nao abre transacao, nao chama rede e nao le relogio a nao ser pelo
//! parametro. E o que permite testar "duas avaliacoes com `id: 0` continuam
//! sendo duas" sem banco e sem sessao — que e exatamente o caso que o
//! `docs/UNIVIRTUS-INTEGRATION.md` §5 aponta como a armadilha do contrato.
//!
//! # Por que existe `payload_hash`
//!
//! O Univirtus nao tem ETag, nem `If-Modified-Since`, nem cursor. `dataModificacao`
//! existe em algumas entidades e nao em outras. O unico criterio que funciona
//! para todas e comparar a impressao digital do que o provedor mandou com a da
//! ultima vez. Igual, nada a fazer; diferente, atualizar.
//!
//! # Por que ausencia nao e exclusao
//!
//! Uma avaliacao some da lista quando a janela dela fecha. Um trabalho some
//! quando o semestre vira. Apagar por ausencia jogaria fora o historico da
//! pessoa por causa de uma decisao de exibicao do portal. O plano marca
//! `Missing`, e quem aplica so registra `unavailable_since`.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::daily::Day;
use crate::error::{CoreError, ErrorCode};

/// O identificador do provedor no banco. Uma constante porque ele entra em
/// chave primaria: um erro de digitacao criaria um segundo provedor invisivel,
/// e a proxima sincronizacao duplicaria tudo.
pub const PROVIDER_UNIVIRTUS: &str = "univirtus";

// ===========================================================================
// Os tipos externos
// ===========================================================================

/// O que uma entidade externa precisa saber sobre si para ser reconciliada.
pub trait ExternalEntity {
    /// A chave estavel no provedor. **Nunca** um id de tentativa, nunca uma URL
    /// assinada — ver `docs/UNIVIRTUS-INTEGRATION.md` §5 e §21.
    fn external_id(&self) -> &str;
    /// A impressao digital do conteudo que o provedor manda. Muda quando algo
    /// que nos importa mudou, e so entao.
    fn fingerprint(&self) -> String;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAcademicContext {
    /// Curso, como o provedor o nomeia.
    pub course_name: String,
    pub course_external_id: String,
    /// A situacao do aluno no curso ("ATIVO"), quando o provedor a informa.
    pub enrollment_status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSemester {
    pub external_id: String,
    pub name: String,
    pub institution: String,
    pub starts_on: Day,
    pub ends_on: Day,
    /// Verdadeiro para o periodo que o provedor considera em andamento.
    pub current: bool,
}

impl ExternalEntity for ExternalSemester {
    fn external_id(&self) -> &str {
        &self.external_id
    }
    fn fingerprint(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.name,
            self.institution,
            self.starts_on.as_str(),
            self.ends_on.as_str()
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSubject {
    pub external_id: String,
    pub semester_external_id: String,
    pub name: String,
    pub code: String,
    /// O professor, quando o provedor tem. O Univirtus nao tem: vem vazio, e
    /// vazio nunca sobrescreve o que a pessoa escreveu (ver `merge_subject`).
    pub teacher: String,
    /// A situacao academica do provedor: "EM CURSO", "APR.MEDIA", "EM EXAME".
    pub situation: String,
    /// A media que a INSTITUICAO calcula (`aproveitamentoMD` no Univirtus).
    ///
    /// Guardada como dado do provedor, e NUNCA escrita por cima da media que o
    /// M/OS deriva em `academic::desempenho`. As duas discordam de proposito:
    /// a instituicao tem regra de exame e recuperacao que o M/OS nao modela, e
    /// deixar uma sobrescrever a outra recriaria a terceira fonte que o ADR-058
    /// eliminou.
    pub official_grade: Option<f64>,
}

impl ExternalEntity for ExternalSubject {
    fn external_id(&self) -> &str {
        &self.external_id
    }
    fn fingerprint(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}",
            self.semester_external_id,
            self.name,
            self.code,
            self.teacher,
            self.situation,
            fmt_num(self.official_grade)
        )
    }
}

/// Prova, APOL, simulado — o que ocupa um instante marcado.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAssessment {
    pub external_id: String,
    pub subject_external_id: String,
    pub title: String,
    /// Categoria do provedor ("APOL Objetiva", "Prova Objetiva"). Vira `topics`
    /// na `Exam`, que e onde o M/Academic guarda "sobre o que e".
    pub category: String,
    /// Quando a janela abre. Informativo; nao e o instante do compromisso.
    pub available_at: Option<OffsetDateTime>,
    /// O instante que importa: ate quando da para fazer.
    pub due_at: OffsetDateTime,
    /// Quanto ela vale NA MEDIA. Vem de `pesoMedia`, e nunca de `peso`.
    pub weight: f64,
    /// O TETO da avaliacao. Vem de `peso`, que se chama peso e nao e peso.
    pub max_score: Option<f64>,
    pub score: Option<f64>,
    pub status: ExternalAssessmentStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalAssessmentStatus {
    /// Ainda nao feita — a janela pode nem ter aberto.
    Pending,
    /// Feita, sem nota publicada.
    Done,
    /// Feita e com nota.
    Graded,
    Cancelled,
}

impl ExternalEntity for ExternalAssessment {
    fn external_id(&self) -> &str {
        &self.external_id
    }
    fn fingerprint(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{:?}",
            self.subject_external_id,
            self.title,
            self.category,
            self.due_at.unix_timestamp(),
            self.weight,
            fmt_num(self.max_score),
            fmt_num(self.score),
            self.status
        )
    }
}

/// Trabalho, atividade pratica — o que se entrega.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAssignment {
    pub external_id: String,
    pub subject_external_id: String,
    pub title: String,
    pub description: String,
    pub due_at: Option<OffsetDateTime>,
    pub submitted_at: Option<OffsetDateTime>,
    pub weight: f64,
    pub max_score: Option<f64>,
    pub score: Option<f64>,
    pub status: ExternalAssignmentStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalAssignmentStatus {
    Pending,
    Submitted,
    Graded,
    Cancelled,
}

impl ExternalEntity for ExternalAssignment {
    fn external_id(&self) -> &str {
        &self.external_id
    }
    fn fingerprint(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{:?}",
            self.subject_external_id,
            self.title,
            self.description,
            self.due_at.map(|d| d.unix_timestamp()).unwrap_or_default(),
            self.submitted_at
                .map(|d| d.unix_timestamp())
                .unwrap_or_default(),
            self.weight,
            fmt_num(self.max_score),
            fmt_num(self.score),
            self.status
        )
    }
}

/// Um arquivo da disciplina.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalMaterial {
    /// `sistemaRepositorio.id`. Numerico e estavel.
    ///
    /// **Nunca a URL.** A URL de download e assinada pelo CloudFront e expira em
    /// horas; usa-la como identidade faria cada sincronizacao criar um Resource
    /// novo para o mesmo PDF, e o antigo apontaria para um link morto.
    pub external_id: String,
    pub subject_external_id: String,
    pub title: String,
    /// A extensao que o provedor declara ("pdf"), em minusculas.
    pub extension: String,
    /// Verdadeiro para o material complementar (Plano de Ensino e afins), que
    /// no Univirtus mora numa estrutura separada do roteiro.
    pub complementary: bool,
    /// A URL de agora, se houver. **Nao e identidade e nao se guarda como tal**:
    /// o storage a grava como endereco corrente do Resource, ciente de que ela
    /// caduca, e a sincronizacao seguinte a substitui.
    pub temporary_url: Option<String>,
}

impl ExternalEntity for ExternalMaterial {
    fn external_id(&self) -> &str {
        &self.external_id
    }
    /// A URL **nao entra** na impressao digital. Ela muda a cada resposta do
    /// provedor porque e assinada com validade curta; se entrasse, todo material
    /// apareceria como "atualizado" em toda sincronizacao, e o relatorio de sync
    /// viraria ruido permanente.
    fn fingerprint(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.subject_external_id, self.title, self.extension, self.complementary
        )
    }
}

/// O retrato que um provedor produz numa sincronizacao.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSnapshot {
    pub provider: String,
    pub context: Option<ExternalAcademicContext>,
    pub semesters: Vec<ExternalSemester>,
    pub subjects: Vec<ExternalSubject>,
    pub assessments: Vec<ExternalAssessment>,
    pub assignments: Vec<ExternalAssignment>,
    pub materials: Vec<ExternalMaterial>,
    /// Disciplinas que falharam individualmente. A sincronizacao continua sem
    /// elas — uma disciplina quebrada nao derruba as outras seis (§47 do
    /// pedido) — e o que elas ja tinham no M/OS fica como estava.
    pub warnings: Vec<String>,
}

impl ProviderSnapshot {
    pub fn new(provider: &str) -> Self {
        Self {
            provider: provider.to_owned(),
            ..Default::default()
        }
    }

    /// As disciplinas que falharam nao podem ser confundidas com disciplinas
    /// que sumiram. Esta lista alimenta a protecao de `reconcile_with_scope`.
    pub fn is_partial(&self) -> bool {
        !self.warnings.is_empty()
    }
}

// ===========================================================================
// A referencia externa gravada
// ===========================================================================

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalKind {
    Semester,
    Subject,
    Assignment,
    Exam,
    Material,
}

impl ExternalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Semester => "semester",
            Self::Subject => "subject",
            Self::Assignment => "assignment",
            Self::Exam => "exam",
            Self::Material => "material",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value.trim() {
            "semester" => Ok(Self::Semester),
            "subject" => Ok(Self::Subject),
            "assignment" => Ok(Self::Assignment),
            "exam" => Ok(Self::Exam),
            "material" => Ok(Self::Material),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Tipo de referencia externa desconhecido no banco local.",
                false,
            )),
        }
    }
}

/// A ponte entre um id do provedor e um id do M/OS.
///
/// Tabela propria em vez de colunas `provider`/`external_id` espalhadas pelas
/// cinco tabelas academicas: a faculdade e um contexto sobre os primitivos
/// (ADR-058), e o mesmo raciocinio vale aqui — o Univirtus nao pode acrescentar
/// coluna a `academic_exams`, senao o dia em que um segundo AVA existir cada
/// tabela ganharia mais um par de colunas.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalRef {
    pub provider: String,
    pub kind: ExternalKind,
    pub external_id: String,
    /// O id da entidade correspondente no M/OS.
    pub local_id: String,
    pub payload_hash: String,
    /// Desde quando o provedor parou de listar isto. `None` significa presente.
    #[serde(with = "time::serde::rfc3339::option")]
    pub unavailable_since: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub first_synced_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub last_synced_at: OffsetDateTime,
}

// ===========================================================================
// A reconciliacao
// ===========================================================================

/// O que fazer com uma entidade externa.
#[derive(Clone, Debug, PartialEq)]
pub enum SyncAction<'a, T> {
    /// Nao existe no M/OS. Criar, e gravar a referencia.
    Create(&'a T),
    /// Existe e mudou. Atualizar SO os campos do provedor.
    Update { item: &'a T, local_id: String },
    /// Existe e esta igual. Nada a fazer alem de tocar `last_synced_at`.
    Unchanged { item: &'a T, local_id: String },
}

impl<'a, T> SyncAction<'a, T> {
    pub fn item(&self) -> &'a T {
        match self {
            Self::Create(item) => item,
            Self::Update { item, .. } | Self::Unchanged { item, .. } => item,
        }
    }
}

/// Uma referencia que o provedor deixou de listar.
#[derive(Clone, Debug, PartialEq)]
pub struct Missing {
    pub kind: ExternalKind,
    pub external_id: String,
    pub local_id: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reconciliation<'a, T> {
    pub actions: Vec<SyncAction<'a, T>>,
    pub missing: Vec<Missing>,
}

impl<T> Reconciliation<'_, T> {
    pub fn created(&self) -> usize {
        self.actions
            .iter()
            .filter(|a| matches!(a, SyncAction::Create(_)))
            .count()
    }
    pub fn updated(&self) -> usize {
        self.actions
            .iter()
            .filter(|a| matches!(a, SyncAction::Update { .. }))
            .count()
    }
    pub fn unchanged(&self) -> usize {
        self.actions
            .iter()
            .filter(|a| matches!(a, SyncAction::Unchanged { .. }))
            .count()
    }
}

/// Compara o que veio com o que ja existe.
///
/// `existing` sao as referencias do MESMO provedor e do MESMO `kind`. O escopo
/// e responsabilidade de quem chama porque so ele sabe se o retrato e completo:
/// sincronizar apenas o semestre corrente e legitimo, e nesse caso as
/// avaliacoes dos semestres passados nao estao "faltando" — elas nem foram
/// pedidas. Ver `reconcile_scoped`.
pub fn reconcile<'a, T: ExternalEntity>(
    kind: ExternalKind,
    items: &'a [T],
    existing: &[ExternalRef],
) -> Reconciliation<'a, T> {
    let known: HashMap<&str, &ExternalRef> = existing
        .iter()
        .filter(|r| r.kind == kind)
        .map(|r| (r.external_id.as_str(), r))
        .collect();

    let mut actions = Vec::with_capacity(items.len());
    let mut vistos: Vec<&str> = Vec::with_capacity(items.len());

    for item in items {
        let id = item.external_id();
        vistos.push(id);
        match known.get(id) {
            None => actions.push(SyncAction::Create(item)),
            Some(reference) => {
                let local_id = reference.local_id.clone();
                // Uma referencia que estava marcada como ausente e reapareceu
                // conta como atualizacao mesmo com o hash igual: o que mudou nao
                // e o conteudo, e o fato de ela existir de novo.
                if reference.payload_hash == item.fingerprint()
                    && reference.unavailable_since.is_none()
                {
                    actions.push(SyncAction::Unchanged { item, local_id });
                } else {
                    actions.push(SyncAction::Update { item, local_id });
                }
            }
        }
    }

    let missing = existing
        .iter()
        .filter(|r| r.kind == kind)
        .filter(|r| !vistos.contains(&r.external_id.as_str()))
        .filter(|r| r.unavailable_since.is_none())
        .map(|r| Missing {
            kind,
            external_id: r.external_id.clone(),
            local_id: r.local_id.clone(),
        })
        .collect();

    Reconciliation { actions, missing }
}

/// A reconciliacao de um retrato PARCIAL.
///
/// Quando o provedor so foi consultado sobre parte do universo — o semestre
/// corrente, ou as disciplinas que responderam sem erro —, o que ficou de fora
/// nao esta ausente: nao foi perguntado. `in_scope` decide, por `external_id`
/// da referencia, se aquela linha estava no recorte desta rodada.
///
/// Sem isto, uma sincronizacao do semestre corrente marcaria como
/// `unavailable` todas as avaliacoes dos semestres anteriores, e a proxima
/// sincronizacao completa as "ressuscitaria" — dois eventos falsos por rodada.
pub fn reconcile_scoped<'a, T: ExternalEntity>(
    kind: ExternalKind,
    items: &'a [T],
    existing: &[ExternalRef],
    in_scope: impl Fn(&ExternalRef) -> bool,
) -> Reconciliation<'a, T> {
    let recorte: Vec<ExternalRef> = existing
        .iter()
        .filter(|r| r.kind == kind && in_scope(r))
        .cloned()
        .collect();
    let mut resultado = reconcile(kind, items, &recorte);
    // As acoes precisam enxergar TODAS as referencias, e nao so as do recorte:
    // uma avaliacao de semestre passado que reaparece no retrato tem de virar
    // Update da linha antiga, e nao Create de uma segunda.
    let fora: HashMap<&str, &ExternalRef> = existing
        .iter()
        .filter(|r| r.kind == kind && !in_scope(r))
        .map(|r| (r.external_id.as_str(), r))
        .collect();
    for acao in &mut resultado.actions {
        if let SyncAction::Create(item) = acao {
            if let Some(reference) = fora.get(item.external_id()) {
                *acao = SyncAction::Update {
                    item,
                    local_id: reference.local_id.clone(),
                };
            }
        }
    }
    resultado
}

// ===========================================================================
// O resultado
// ===========================================================================

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncCounts {
    pub created: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub unavailable: usize,
}

impl SyncCounts {
    pub fn total_touched(self) -> usize {
        self.created + self.updated
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOutcome {
    Completed,
    CompletedWithWarnings,
    /// A sessao caiu. **Nada foi marcado como ausente** — ver `SyncReport`.
    RequiresAuthentication,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub provider: String,
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub finished_at: OffsetDateTime,
    pub outcome: SyncOutcome,
    pub semesters: SyncCounts,
    pub subjects: SyncCounts,
    pub assessments: SyncCounts,
    pub assignments: SyncCounts,
    pub materials: SyncCounts,
    pub warnings: Vec<String>,
}

impl SyncReport {
    pub fn empty(provider: &str, at: OffsetDateTime, outcome: SyncOutcome) -> Self {
        Self {
            provider: provider.to_owned(),
            started_at: at,
            finished_at: at,
            outcome,
            semesters: SyncCounts::default(),
            subjects: SyncCounts::default(),
            assessments: SyncCounts::default(),
            assignments: SyncCounts::default(),
            materials: SyncCounts::default(),
            warnings: Vec::new(),
        }
    }

    /// A frase curta que o toast mostra. Vazia quando nada mudou — o §35 do
    /// pedido nao quer modal, e "tudo em dia" nao merece nem toast.
    pub fn resumo(&self) -> String {
        let mut partes: Vec<String> = Vec::new();
        let mut add = |n: usize, singular: &str, plural: &str| {
            if n > 0 {
                partes.push(format!("+{n} {}", if n == 1 { singular } else { plural }));
            }
        };
        add(self.subjects.created, "disciplina", "disciplinas");
        add(self.assessments.created, "avaliação", "avaliações");
        add(self.assignments.created, "trabalho", "trabalhos");
        add(self.materials.created, "material", "materiais");
        let atualizados = self.assessments.updated + self.assignments.updated;
        if atualizados > 0 {
            partes.push(format!(
                "~{atualizados} {}",
                if atualizados == 1 {
                    "atualizado"
                } else {
                    "atualizados"
                }
            ));
        }
        partes.join(" · ")
    }
}

/// O estado de conexao de um provedor, como a tela precisa ver.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderConnection {
    /// Nunca conectado, ou desconectado pela pessoa.
    Disconnected,
    /// Ha sessao guardada e ela respondeu.
    Connected,
    /// Ha sessao guardada e ela expirou. Os dados sincronizados **continuam**.
    Expired,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub provider: String,
    pub connection: ProviderConnection,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_sync_at: Option<OffsetDateTime>,
    pub last_outcome: Option<SyncOutcome>,
    pub course_name: String,
    /// Quantas linhas o provedor ja trouxe, por tipo. Serve a tela de
    /// integracao, e nao a logica.
    pub tracked: BTreeMap<String, usize>,
}

fn fmt_num(value: Option<f64>) -> String {
    // Duas casas fixas: `0.1 + 0.2` nao pode virar mudanca de conteudo.
    match value {
        Some(v) => format!("{v:.2}"),
        None => "-".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn ref_de(kind: ExternalKind, external: &str, local: &str, hash: &str) -> ExternalRef {
        ExternalRef {
            provider: PROVIDER_UNIVIRTUS.into(),
            kind,
            external_id: external.into(),
            local_id: local.into(),
            payload_hash: hash.into(),
            unavailable_since: None,
            first_synced_at: datetime!(2026-08-01 10:00 UTC),
            last_synced_at: datetime!(2026-08-01 10:00 UTC),
        }
    }

    fn avaliacao(external: &str, titulo: &str, quando: OffsetDateTime) -> ExternalAssessment {
        ExternalAssessment {
            external_id: external.into(),
            subject_external_id: "905706".into(),
            title: titulo.into(),
            category: "Prova Objetiva".into(),
            available_at: None,
            due_at: quando,
            weight: 30.0,
            max_score: Some(100.0),
            score: None,
            status: ExternalAssessmentStatus::Pending,
        }
    }

    /// A armadilha do §5 do relatorio: cinco avaliacoes nao iniciadas chegam com
    /// `id: 0` ao mesmo tempo. O que as distingue e `idAvaliacao`, e e ele que
    /// vira `external_id` — este teste morre se alguem trocar a chave.
    #[test]
    fn avaliacoes_com_id_zero_continuam_sendo_avaliacoes_diferentes() {
        let quando = datetime!(2026-09-14 23:59 UTC);
        let itens = vec![
            avaliacao("2713958", "Prova Objetiva (Regular)", quando),
            avaliacao("2713960", "Simulado 3", quando),
            avaliacao("2713961", "Prova (Substitutiva)", quando),
        ];
        let plano = reconcile(ExternalKind::Exam, &itens, &[]);
        assert_eq!(plano.created(), 3);
        let ids: Vec<&str> = plano
            .actions
            .iter()
            .map(|a| a.item().external_id())
            .collect();
        assert_eq!(ids, ["2713958", "2713960", "2713961"]);
    }

    #[test]
    fn rodar_duas_vezes_sem_mudanca_nao_cria_nada() {
        let quando = datetime!(2026-09-14 23:59 UTC);
        let itens = vec![avaliacao("2713958", "Prova Objetiva", quando)];
        let refs = vec![ref_de(
            ExternalKind::Exam,
            "2713958",
            "exam-local-1",
            &itens[0].fingerprint(),
        )];
        let plano = reconcile(ExternalKind::Exam, &itens, &refs);
        assert_eq!(plano.created(), 0);
        assert_eq!(plano.updated(), 0);
        assert_eq!(plano.unchanged(), 1);
        assert!(plano.missing.is_empty());
    }

    #[test]
    fn prazo_alterado_vira_update_e_nao_um_segundo_registro() {
        let antes = avaliacao("2713958", "Prova", datetime!(2026-09-14 23:59 UTC));
        let refs = vec![ref_de(
            ExternalKind::Exam,
            "2713958",
            "exam-local-1",
            &antes.fingerprint(),
        )];
        let depois = vec![avaliacao(
            "2713958",
            "Prova",
            datetime!(2026-09-30 23:59 UTC),
        )];
        let plano = reconcile(ExternalKind::Exam, &depois, &refs);
        assert_eq!(plano.created(), 0);
        assert_eq!(plano.updated(), 1);
        match &plano.actions[0] {
            SyncAction::Update { local_id, .. } => assert_eq!(local_id, "exam-local-1"),
            outro => panic!("esperava Update, veio {outro:?}"),
        }
    }

    #[test]
    fn o_que_sumiu_do_provedor_e_marcado_e_nunca_apagado() {
        let refs = vec![
            ref_de(ExternalKind::Exam, "2713958", "exam-1", "hash-a"),
            ref_de(ExternalKind::Exam, "2713960", "exam-2", "hash-b"),
        ];
        let agora = vec![avaliacao(
            "2713958",
            "Prova",
            datetime!(2026-09-14 23:59 UTC),
        )];
        let plano = reconcile(ExternalKind::Exam, &agora, &refs);
        assert_eq!(plano.missing.len(), 1);
        assert_eq!(plano.missing[0].local_id, "exam-2");
        assert_eq!(plano.missing[0].external_id, "2713960");
    }

    #[test]
    fn o_que_reaparece_volta_como_update_mesmo_com_hash_igual() {
        let item = avaliacao("2713958", "Prova", datetime!(2026-09-14 23:59 UTC));
        let mut referencia = ref_de(ExternalKind::Exam, "2713958", "exam-1", &item.fingerprint());
        referencia.unavailable_since = Some(datetime!(2026-08-10 10:00 UTC));
        let plano = reconcile(
            ExternalKind::Exam,
            std::slice::from_ref(&item),
            &[referencia],
        );
        assert_eq!(plano.updated(), 1);
        assert_eq!(plano.unchanged(), 0);
    }

    /// Sincronizar so o semestre corrente nao pode marcar como ausente o que
    /// nem foi perguntado.
    #[test]
    fn fora_do_recorte_nao_conta_como_ausente() {
        let refs = vec![
            ref_de(ExternalKind::Exam, "corrente-1", "exam-1", "hash-a"),
            ref_de(ExternalKind::Exam, "antigo-1", "exam-2", "hash-b"),
        ];
        let agora = vec![avaliacao(
            "corrente-1",
            "Prova",
            datetime!(2026-09-14 23:59 UTC),
        )];
        let plano = reconcile_scoped(ExternalKind::Exam, &agora, &refs, |r| {
            r.external_id.starts_with("corrente")
        });
        assert!(plano.missing.is_empty());
        assert_eq!(plano.updated(), 1);
    }

    /// E o que estava fora do recorte mas voltou a aparecer tem de casar com a
    /// linha antiga, nunca criar uma segunda.
    #[test]
    fn fora_do_recorte_que_reaparece_casa_com_a_linha_antiga() {
        let refs = vec![ref_de(ExternalKind::Exam, "antigo-1", "exam-2", "hash-b")];
        let agora = vec![avaliacao(
            "antigo-1",
            "Prova",
            datetime!(2026-09-14 23:59 UTC),
        )];
        let plano = reconcile_scoped(ExternalKind::Exam, &agora, &refs, |_| false);
        assert_eq!(plano.created(), 0);
        assert_eq!(plano.updated(), 1);
        match &plano.actions[0] {
            SyncAction::Update { local_id, .. } => assert_eq!(local_id, "exam-2"),
            outro => panic!("esperava Update, veio {outro:?}"),
        }
    }

    /// A URL assinada muda a cada resposta. Se ela entrasse na impressao
    /// digital, todo material apareceria como atualizado em toda rodada.
    #[test]
    fn a_url_assinada_nao_muda_a_impressao_digital_do_material() {
        let base = ExternalMaterial {
            external_id: "60634399".into(),
            subject_external_id: "905706".into(),
            title: "PLANO DE ENSINO.pdf".into(),
            extension: "pdf".into(),
            complementary: true,
            temporary_url: Some("https://cdn/a?Signature=aaa".into()),
        };
        let outra = ExternalMaterial {
            temporary_url: Some("https://cdn/a?Signature=bbb".into()),
            ..base.clone()
        };
        assert_eq!(base.fingerprint(), outra.fingerprint());
    }

    #[test]
    fn a_media_oficial_entra_na_impressao_digital_da_disciplina() {
        let base = ExternalSubject {
            external_id: "905706".into(),
            semester_external_id: "2026B2".into(),
            name: "Projeto Arquitetônico".into(),
            code: "905706".into(),
            teacher: String::new(),
            situation: "EM CURSO".into(),
            official_grade: None,
        };
        let com_nota = ExternalSubject {
            official_grade: Some(8.5),
            ..base.clone()
        };
        assert_ne!(base.fingerprint(), com_nota.fingerprint());
    }

    #[test]
    fn o_resumo_fica_vazio_quando_nada_mudou() {
        let relatorio = SyncReport::empty(
            PROVIDER_UNIVIRTUS,
            datetime!(2026-08-22 13:40 UTC),
            SyncOutcome::Completed,
        );
        assert_eq!(relatorio.resumo(), "");
    }
}
