//! O normalizador do Univirtus: JSON cru do AVA -> tipos de `academic_sync`.
//!
//! **Puro.** Nao abre socket, nao le keyring, nao toca em Tauri. Recebe
//! `serde_json::Value` como o portal devolveu e produz `External*`. E o que
//! permite testar as duas armadilhas do contrato — `peso` que nao e peso, e
//! `id: 0` que nao e identidade — sem sessao autenticada e sem rede.
//!
//! Toda a fonte destes campos esta em `docs/UNIVIRTUS-INTEGRATION.md` §4.

use serde_json::Value;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

use crate::academic_sync::{
    ExternalAcademicContext, ExternalAssessment, ExternalAssessmentStatus, ExternalAssignment,
    ExternalAssignmentStatus, ExternalMaterial, ExternalSemester, ExternalSubject,
};
use crate::daily::Day;
use crate::error::{CoreError, ErrorCode};

/// O envelope que TODA resposta do Univirtus usa.
///
/// `totalRegistros` vem no envelope e **mente** — veio `0` em respostas com 17
/// itens. Por isso esta funcao ignora o contador e devolve o array; quem conta
/// e o `len()`.
pub fn envelope<'a>(body: &'a Value, chave: &str) -> &'a [Value] {
    body.get(chave)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

// ===========================================================================
// Datas
// ===========================================================================

/// O Univirtus devolve `2026-08-24T23:59:00` — sem fuso.
///
/// O instante e local de Brasilia, e o M/OS guarda tudo em UTC. Interpretar
/// como UTC atrasaria todo prazo em 3 horas, e "vence 23h59" viraria "vence
/// 20h59" na tela. O offset entra como parametro em vez de ser lido do relogio
/// porque funcao que le relogio nao se testa.
pub fn instante(bruto: &str, offset: UtcOffset) -> Option<OffsetDateTime> {
    let bruto = bruto.trim();
    if bruto.is_empty() {
        return None;
    }
    // `0001-01-01T00:00:00` e o "nulo" do .NET, e aparece em `dataInicio` de
    // sala. Tratar como data real colocaria compromissos no ano 1.
    if bruto.starts_with("0001-01-01") {
        return None;
    }
    let (data, hora) = bruto.split_once('T')?;
    let mut partes = data.split('-');
    let ano: i32 = partes.next()?.parse().ok()?;
    let mes: u8 = partes.next()?.parse().ok()?;
    let dia: u8 = partes.next()?.parse().ok()?;
    let mut hp = hora.split(':');
    let h: u8 = hp.next()?.parse().ok()?;
    let m: u8 = hp.next().unwrap_or("0").parse().ok()?;
    let s: u8 = hp
        .next()
        .unwrap_or("0")
        .split('.')
        .next()
        .unwrap_or("0")
        .parse()
        .ok()?;
    let date = Date::from_calendar_date(ano, Month::try_from(mes).ok()?, dia).ok()?;
    let time = Time::from_hms(h, m, s).ok()?;
    Some(PrimitiveDateTime::new(date, time).assume_offset(offset))
}

fn texto(v: &Value, chave: &str) -> String {
    v.get(chave)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn numero(v: &Value, chave: &str) -> Option<f64> {
    v.get(chave).and_then(Value::as_f64)
}

fn inteiro_como_texto(v: &Value, chave: &str) -> String {
    match v.get(chave) {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.trim().to_owned(),
        _ => String::new(),
    }
}

// ===========================================================================
// Contexto e semestre
// ===========================================================================

/// O curso, de `GetCursosAproveitamento`.
pub fn context(body: &Value) -> Option<ExternalAcademicContext> {
    let primeiro = envelope(body, "cursosAproveitamento").first()?;
    let curso = primeiro.get("curso")?;
    Some(ExternalAcademicContext {
        course_name: texto(curso, "nome"),
        course_external_id: inteiro_como_texto(curso, "id"),
        enrollment_status: primeiro
            .get("tipoSituacaoAlunoCurso")
            .map(|s| texto(s, "nome"))
            .unwrap_or_default(),
    })
}

/// O `sCdAluno` cifrado que `GetDisciplinasAproveitamento` exige.
///
/// **E um segredo de sessao**: nao deve ser logado, nem gravado no banco
/// academico. Sai daqui e vai direto para a query da chamada seguinte.
pub fn student_key(body: &Value) -> Option<String> {
    let primeiro = envelope(body, "cursosAproveitamento").first()?;
    let chave = texto(primeiro, "sCdAluno");
    (!chave.is_empty()).then_some(chave)
}

/// Os semestres, derivados de `nomeModuloPOrdenacao` no historico academico.
///
/// O Univirtus nao tem entidade "semestre" com datas. O que ele tem e o rotulo
/// `2026B2`, que ordena lexicograficamente. As datas do `Semester` do M/OS sao
/// **inferidas do rotulo**, e nao inventadas: o M/OS precisa de intervalo porque
/// `Semester::status_em` deriva o estado das datas (ADR-058 §3), e um intervalo
/// aproximado que respeita a ordem e melhor que um campo "ativo" que mente.
pub fn semesters(historico: &Value, institution: &str) -> Vec<ExternalSemester> {
    let mut rotulos: Vec<String> = envelope(historico, "aproveitamento")
        .iter()
        .map(|d| texto(d, "nomeModuloPOrdenacao"))
        .filter(|r| !r.is_empty())
        .collect();
    rotulos.sort();
    rotulos.dedup();

    let corrente = rotulos.last().cloned().unwrap_or_default();
    rotulos
        .iter()
        .filter_map(|rotulo| {
            let (starts_on, ends_on) = intervalo_do_rotulo(rotulo)?;
            Some(ExternalSemester {
                external_id: rotulo.clone(),
                name: rotulo.clone(),
                institution: institution.to_owned(),
                starts_on,
                ends_on,
                current: *rotulo == corrente,
            })
        })
        .collect()
}

/// `2026B2` -> (inicio, fim).
///
/// O ciclo da UNINTER e bimestral dentro de uma letra: A, B e C sao os tercos do
/// ano, e o digito e a metade do terco. Sao quatro meses por letra, dois por
/// digito. O intervalo nao precisa bater com o calendario oficial ao dia — ele
/// precisa **ordenar certo e conter o presente**, que e do que
/// `semestre_corrente` depende.
fn intervalo_do_rotulo(rotulo: &str) -> Option<(Day, Day)> {
    let bytes = rotulo.as_bytes();
    if bytes.len() < 6 {
        return None;
    }
    let ano: i32 = rotulo.get(0..4)?.parse().ok()?;
    let letra = bytes[4] as char;
    let digito = bytes[5] as char;
    let base = match letra {
        'A' => 1u8,
        'B' => 5,
        'C' => 9,
        _ => return None,
    };
    let inicio_mes = match digito {
        '1' => base,
        '2' => base + 2,
        _ => return None,
    };
    let fim_mes = inicio_mes + 1;
    let ultimo_dia = ultimo_dia_do_mes(ano, fim_mes)?;
    Some((
        Day::parse(&format!("{ano:04}-{inicio_mes:02}-01")).ok()?,
        Day::parse(&format!("{ano:04}-{fim_mes:02}-{ultimo_dia:02}")).ok()?,
    ))
}

fn ultimo_dia_do_mes(ano: i32, mes: u8) -> Option<u8> {
    let mes = Month::try_from(mes).ok()?;
    Some(mes.length(ano))
}

// ===========================================================================
// Disciplinas
// ===========================================================================

/// As disciplinas, cruzando as ofertas do AVA com o historico academico.
///
/// A juncao e por `codigoOferta` == `cdOfertaDisciplina` — a unica chave que
/// aparece nos DOIS lados. Sem ela nao haveria como saber a que semestre uma
/// oferta pertence, porque o AVA nao diz.
///
/// `codigoOferta == 0` marca **sala de apoio** ("Duvidas sobre Estagio",
/// "Pesquisa e extensao"), e nao disciplina: elas nao tem contrapartida no
/// historico e virariam duas Subjects fantasma sem nota nem prazo.
pub fn subjects(ofertas: &Value, historico: &Value) -> Vec<ExternalSubject> {
    let por_codigo: std::collections::HashMap<String, &Value> =
        envelope(historico, "aproveitamento")
            .iter()
            .map(|d| (inteiro_como_texto(d, "cdOfertaDisciplina"), d))
            .collect();

    envelope(ofertas, "usuarioHistoricoCursoOfertas")
        .iter()
        .filter_map(|o| {
            let codigo = inteiro_como_texto(o, "codigoOferta");
            if codigo.is_empty() || codigo == "0" {
                return None;
            }
            let hist = por_codigo.get(&codigo)?;
            Some(ExternalSubject {
                external_id: codigo.clone(),
                semester_external_id: texto(hist, "nomeModuloPOrdenacao"),
                // O nome do historico e o oficial ("Estatica dos Corpos"); o do
                // AVA e o da sala ("Estatica dos corpos"). O oficial ganha.
                name: {
                    let oficial = texto(hist, "nomeDisciplina");
                    if oficial.is_empty() {
                        texto(o, "nomeSalaVirtual")
                    } else {
                        oficial
                    }
                },
                code: codigo,
                // O Univirtus nao publica professor: `salaVirtual.nomeProfessor`
                // vem `null`. Vazio aqui e honesto, e o merge no storage nunca
                // apaga o que a pessoa escreveu com um vazio.
                teacher: String::new(),
                situation: texto(hist, "tipoSituacaoAluno"),
                official_grade: numero(hist, "aproveitamentoMD"),
            })
        })
        .collect()
}

/// Os pares (idSalaVirtual, idSalaVirtualOferta) necessarios para consultar uma
/// disciplina. Nao viram `external_id` — sao endereco de consulta, e mudam
/// quando a mesma disciplina e refeita noutro semestre (§5 do relatorio).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectQueryRef {
    pub codigo_oferta: String,
    pub id_sala_virtual: String,
    pub id_sala_virtual_oferta: String,
    pub nome: String,
}

pub fn subject_query_refs(ofertas: &Value) -> Vec<SubjectQueryRef> {
    envelope(ofertas, "usuarioHistoricoCursoOfertas")
        .iter()
        .filter_map(|o| {
            let codigo = inteiro_como_texto(o, "codigoOferta");
            if codigo.is_empty() || codigo == "0" {
                return None;
            }
            Some(SubjectQueryRef {
                codigo_oferta: codigo,
                id_sala_virtual: inteiro_como_texto(o, "idSalaVirtual"),
                id_sala_virtual_oferta: inteiro_como_texto(o, "idSalaVirtualOferta"),
                nome: texto(o, "nomeSalaVirtual"),
            })
        })
        .collect()
}

// ===========================================================================
// Avaliacoes
// ===========================================================================

/// As avaliacoes de uma disciplina, de `bqs/AvaliacaoUsuario`.
///
/// # As duas regras que nao podem ser esquecidas
///
/// **1. `external_id` e `idAvaliacao`, nunca `id`.** O `id` e da *tentativa do
/// usuario*, e vale `0` enquanto a prova nao foi iniciada — cinco provas da
/// mesma disciplina chegam com `0` ao mesmo tempo. Chavear por ele colidiria as
/// cinco numa so.
///
/// **2. `peso` e o TETO, `pesoMedia` e o peso.** `avaliacao.peso` vale 100 e
/// significa "a prova vale de 0 a 100". Quem pondera a media e
/// `avaliacao.pesoMedia`, que vale 15. Mapear `peso -> weight` faria a media da
/// disciplina inteira errada, e ela erraria em silencio — o numero apareceria,
/// so estaria errado.
pub fn assessments(
    body: &Value,
    subject_external_id: &str,
    offset: UtcOffset,
) -> Vec<ExternalAssessment> {
    envelope(body, "avaliacaoUsuarios")
        .iter()
        .filter_map(|a| {
            let avaliacao = a.get("avaliacao")?;
            let external_id = inteiro_como_texto(a, "idAvaliacao");
            if external_id.is_empty() || external_id == "0" {
                return None;
            }
            let due_at = instante(&texto(a, "dataFim"), offset)?;
            Some(ExternalAssessment {
                external_id,
                subject_external_id: subject_external_id.to_owned(),
                title: {
                    let nome = texto(avaliacao, "nome");
                    // O portal deixa espaco duplo em "Prova Objetiva  (Regular)".
                    nome.split_whitespace().collect::<Vec<_>>().join(" ")
                },
                category: texto(avaliacao, "nomeAvaliacaoTipo"),
                available_at: instante(&texto(a, "dataInicio"), offset),
                due_at,
                // `pesoMedia`, e NAO `peso`. Ver o doc-comment acima.
                weight: numero(avaliacao, "pesoMedia").unwrap_or(0.0).max(0.0),
                // `peso`, que se chama peso e e o teto.
                max_score: numero(avaliacao, "peso").filter(|v| *v > 0.0),
                score: numero(a, "nota"),
                status: status_avaliacao(a),
            })
        })
        .collect()
}

/// O estado da avaliacao.
///
/// `idAvaliacaoUsuarioStatus` e a fonte, e nao o texto de `status`: o texto e
/// para leitura humana e ja apareceu em duas grafias. `3` e finalizada, `4` e
/// aguardando inicio. Nota presente promove para `Graded`.
fn status_avaliacao(a: &Value) -> ExternalAssessmentStatus {
    let tem_nota = a.get("nota").and_then(Value::as_f64).is_some();
    match a.get("idAvaliacaoUsuarioStatus").and_then(Value::as_i64) {
        Some(3) if tem_nota => ExternalAssessmentStatus::Graded,
        Some(3) => ExternalAssessmentStatus::Done,
        Some(4) => ExternalAssessmentStatus::Pending,
        _ if tem_nota => ExternalAssessmentStatus::Graded,
        _ => ExternalAssessmentStatus::Pending,
    }
}

// ===========================================================================
// Trabalhos
// ===========================================================================

/// Os trabalhos de uma disciplina, de `interacao/TrabalhoEtapa`.
///
/// A identidade e `idTrabalho:id` — o par, e nao so `idTrabalho`. Um trabalho
/// tem N etapas ("Regular", "2a Chamada", "Exame", "RCP"), cada uma com prazo
/// proprio, e todas compartilham o mesmo `idTrabalho`. Chavear so por ele
/// colapsaria as quatro entregas numa.
pub fn assignments(
    body: &Value,
    subject_external_id: &str,
    offset: UtcOffset,
) -> Vec<ExternalAssignment> {
    envelope(body, "trabalhoEtapas")
        .iter()
        .filter_map(|t| {
            let trabalho = inteiro_como_texto(t, "idTrabalho");
            let etapa = inteiro_como_texto(t, "id");
            if trabalho.is_empty() && etapa.is_empty() {
                return None;
            }
            let submitted_at = instante(&texto(t, "dataEntrega"), offset);
            let score = numero(t, "notaEtapa").or_else(|| numero(t, "notaTrabalho"));
            Some(ExternalAssignment {
                external_id: format!("{trabalho}:{etapa}"),
                subject_external_id: subject_external_id.to_owned(),
                title: {
                    let nome = texto(t, "nome");
                    if nome.is_empty() {
                        texto(t, "nomeTrabalhoTipo")
                    } else {
                        nome
                    }
                },
                description: texto(t, "nomeTrabalhoTipo"),
                due_at: instante(&texto(t, "dataFim"), offset),
                submitted_at,
                // O Univirtus nao publica peso de trabalho na media. Zero e o
                // que o M/Academic ja entende como "nao entra na media quando ha
                // outra avaliacao com peso" (ADR-058 §4).
                weight: 0.0,
                max_score: score.map(|_| 10.0),
                score,
                status: if score.is_some() {
                    ExternalAssignmentStatus::Graded
                } else if submitted_at.is_some() {
                    ExternalAssignmentStatus::Submitted
                } else {
                    ExternalAssignmentStatus::Pending
                },
            })
        })
        .collect()
}

// ===========================================================================
// Materiais
// ===========================================================================

/// Os arquivos de uma atividade, de `atv/AtividadeItemAprendizagem`.
///
/// `external_id` e `sistemaRepositorio.id` — numerico e estavel. A URL vem
/// junto como `temporary_url` e **nunca** como identidade: ela e assinada pelo
/// CloudFront com validade de horas, e usa-la como chave criaria um Resource
/// novo por sincronizacao.
pub fn materials(
    body: &Value,
    subject_external_id: &str,
    complementary: bool,
) -> Vec<ExternalMaterial> {
    let mut saida = Vec::new();
    for item in envelope(body, "atividadeItemAprendizagens") {
        let etiquetas = item
            .get("itemAprendizagemEtiquetas")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        for etiqueta in etiquetas {
            let Some(repo) = etiqueta.get("sistemaRepositorio") else {
                continue;
            };
            if repo.is_null() {
                continue;
            }
            let id = inteiro_como_texto(repo, "id");
            if id.is_empty() || id == "0" {
                continue;
            }
            let nome = texto(repo, "nome");
            if nome.is_empty() {
                continue;
            }
            saida.push(ExternalMaterial {
                external_id: id,
                subject_external_id: subject_external_id.to_owned(),
                title: nome,
                extension: texto(repo, "extensao").to_lowercase(),
                complementary,
                temporary_url: {
                    let url = texto(repo, "url");
                    (!url.is_empty()).then_some(url)
                },
            });
        }
    }
    saida
}

/// As aulas do roteiro, de `ava/SalaVirtualEstrutura`. Sao endereco de consulta
/// para chegar as atividades — o M/Academic nao tem entidade "Aula" hoje, e
/// inventar uma agora seria a feature que o §9 do `ACADEMIC.md` deixou fora.
pub fn structure_ids(body: &Value) -> Vec<String> {
    envelope(body, "salaVirtualEstruturas")
        .iter()
        .map(|e| inteiro_como_texto(e, "id"))
        .filter(|id| !id.is_empty() && id != "0")
        .collect()
}

/// Os `idAtividade` de uma estrutura, de `ava/salaVirtualAtividade`.
pub fn activity_ids(body: &Value) -> Vec<String> {
    envelope(body, "salaVirtualAtividades")
        .iter()
        .map(|a| inteiro_como_texto(a, "idAtividade"))
        .filter(|id| !id.is_empty() && id != "0")
        .collect()
}

/// A sessao caiu?
///
/// O Univirtus responde 401 em toda chamada quando a sessao morre. Distinguir
/// isso de erro generico e o que permite a tela dizer "Reconectar" em vez de
/// "algo deu errado", e o que impede o sync de concluir que a pessoa trancou o
/// curso (§30 do pedido).
pub fn is_session_expired(status: u16) -> bool {
    status == 401 || status == 403
}

pub fn session_expired_error() -> CoreError {
    CoreError::new(
        ErrorCode::ProviderUnauthorized,
        "A sessao do Univirtus expirou. Reconecte para sincronizar de novo.",
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn brasilia() -> UtcOffset {
        UtcOffset::from_hms(-3, 0, 0).unwrap()
    }

    fn avaliacao_json(id_tentativa: i64, id_avaliacao: i64, nome: &str) -> Value {
        json!({
            "id": id_tentativa,
            "idAvaliacao": id_avaliacao,
            "idAvaliacaoUsuarioStatus": 4,
            "status": "Aguardando início",
            "dataInicio": "2026-08-24T00:00:00",
            "dataFim": "2026-09-14T23:59:00",
            "nota": null,
            "avaliacao": {
                "id": id_avaliacao,
                "nome": nome,
                "nomeAvaliacaoTipo": "Prova Objetiva",
                "peso": 100,
                "pesoMedia": 30
            }
        })
    }

    // -----------------------------------------------------------------------
    // A regra critica do §14 do pedido
    // -----------------------------------------------------------------------

    /// `peso` vale 100 e e o TETO. `pesoMedia` vale 15 e e o peso na media.
    /// Este teste existe para morrer no dia em que alguem trocar os dois.
    #[test]
    fn peso_e_teto_e_peso_media_e_peso() {
        let body = json!({ "avaliacaoUsuarios": [ {
            "id": 152558335,
            "idAvaliacao": 2713956,
            "idAvaliacaoUsuarioStatus": 3,
            "dataInicio": "2026-07-13T00:00:00",
            "dataFim": "2026-08-24T23:59:00",
            "nota": 100,
            "avaliacao": {
                "nome": "APOL Objetiva 1 (Regular)",
                "nomeAvaliacaoTipo": "APOL Objetiva",
                "peso": 100,
                "pesoMedia": 15
            }
        } ] });
        let saida = assessments(&body, "905706", brasilia());
        assert_eq!(saida.len(), 1);
        assert_eq!(saida[0].max_score, Some(100.0), "peso deve virar max_score");
        assert_eq!(saida[0].weight, 15.0, "pesoMedia deve virar weight");
        assert_eq!(saida[0].score, Some(100.0));
        assert_eq!(saida[0].status, ExternalAssessmentStatus::Graded);
    }

    /// O caso que quebraria a media em silencio: se `peso` virasse `weight`, uma
    /// prova de peso 30 passaria a pesar 100.
    #[test]
    fn o_peso_da_media_nunca_e_cem_so_porque_o_teto_e_cem() {
        let body = json!({ "avaliacaoUsuarios": [ avaliacao_json(0, 2713958, "Prova") ] });
        let saida = assessments(&body, "905706", brasilia());
        assert_eq!(saida[0].weight, 30.0);
        assert_ne!(saida[0].weight, 100.0);
    }

    // -----------------------------------------------------------------------
    // A regra critica do §15 do pedido
    // -----------------------------------------------------------------------

    /// Tres avaliacoes nao iniciadas chegam com `id: 0`. Elas sao tres.
    #[test]
    fn tres_avaliacoes_com_id_zero_produzem_tres_identidades() {
        let body = json!({ "avaliacaoUsuarios": [
            avaliacao_json(0, 2713958, "Prova Objetiva  (Regular)"),
            avaliacao_json(0, 2713960, "Simulado 3"),
            avaliacao_json(0, 2713961, "Prova Objetiva  (Substitutiva)"),
        ] });
        let saida = assessments(&body, "905706", brasilia());
        assert_eq!(saida.len(), 3);
        let ids: Vec<&str> = saida.iter().map(|a| a.external_id.as_str()).collect();
        assert_eq!(ids, ["2713958", "2713960", "2713961"]);
        assert!(!ids.contains(&"0"), "o id da tentativa nunca e identidade");
    }

    #[test]
    fn o_espaco_duplo_do_portal_nao_vaza_para_o_titulo() {
        let body = json!({ "avaliacaoUsuarios": [
            avaliacao_json(0, 2713958, "Prova Objetiva  (Regular)"),
        ] });
        let saida = assessments(&body, "905706", brasilia());
        assert_eq!(saida[0].title, "Prova Objetiva (Regular)");
    }

    // -----------------------------------------------------------------------
    // Datas
    // -----------------------------------------------------------------------

    /// "vence 23h59" tem de continuar sendo 23h59 na tela de quem esta em
    /// Brasilia. Ler como UTC atrasaria o prazo em tres horas.
    #[test]
    fn a_data_sem_fuso_e_lida_como_local_e_nao_como_utc() {
        let quando = instante("2026-08-24T23:59:00", brasilia()).unwrap();
        assert_eq!(quando.offset(), brasilia());
        assert_eq!(quando.hour(), 23);
        // Em UTC, isso e 2026-08-25T02:59.
        let em_utc = quando.to_offset(UtcOffset::UTC);
        assert_eq!(em_utc.day(), 25);
        assert_eq!(em_utc.hour(), 2);
    }

    #[test]
    fn o_nulo_do_dotnet_nao_vira_compromisso_no_ano_um() {
        assert!(instante("0001-01-01T00:00:00", brasilia()).is_none());
        assert!(instante("", brasilia()).is_none());
    }

    // -----------------------------------------------------------------------
    // Disciplinas e semestres
    // -----------------------------------------------------------------------

    fn ofertas_json() -> Value {
        json!({ "usuarioHistoricoCursoOfertas": [
            { "codigoOferta": 905706, "idSalaVirtual": 60236, "idSalaVirtualOferta": 1161461,
              "nomeSalaVirtual": "Projeto Arquitetônico" },
            { "codigoOferta": 906216, "idSalaVirtual": 57865, "idSalaVirtualOferta": 1041884,
              "nomeSalaVirtual": "Estática dos corpos" },
            // Sala de apoio: codigoOferta 0.
            { "codigoOferta": 0, "idSalaVirtual": 63421, "idSalaVirtualOferta": 388580,
              "nomeSalaVirtual": "Dúvidas sobre Estágio" },
        ] })
    }

    fn historico_json() -> Value {
        json!({ "aproveitamento": [
            { "cdOfertaDisciplina": 905706, "nomeDisciplina": "Projeto Arquitetônico",
              "nomeModuloPOrdenacao": "2026B2", "tipoSituacaoAluno": "EM CURSO",
              "aproveitamentoMD": null },
            { "cdOfertaDisciplina": 906216, "nomeDisciplina": "Estática dos Corpos",
              "nomeModuloPOrdenacao": "2026B2", "tipoSituacaoAluno": "EM CURSO",
              "aproveitamentoMD": null },
            { "cdOfertaDisciplina": 887180, "nomeDisciplina": "Cálculo a Várias Variáveis",
              "nomeModuloPOrdenacao": "2026B1", "tipoSituacaoAluno": "APR.MÉDIA",
              "aproveitamentoMD": 8.5 },
        ] })
    }

    #[test]
    fn a_sala_de_apoio_nao_vira_disciplina() {
        let saida = subjects(&ofertas_json(), &historico_json());
        assert_eq!(saida.len(), 2);
        assert!(saida.iter().all(|s| s.external_id != "0"));
        assert!(!saida.iter().any(|s| s.name.contains("Estágio")));
    }

    #[test]
    fn o_nome_oficial_do_historico_ganha_do_nome_da_sala() {
        let saida = subjects(&ofertas_json(), &historico_json());
        let estatica = saida.iter().find(|s| s.external_id == "906216").unwrap();
        assert_eq!(estatica.name, "Estática dos Corpos");
    }

    /// `aproveitamentoMD` e dado do provedor. Ele chega como `official_grade` e
    /// nao existe caminho por onde ele vire `score` de avaliacao nenhuma.
    #[test]
    fn a_media_oficial_chega_como_dado_do_provedor() {
        let ofertas = json!({ "usuarioHistoricoCursoOfertas": [
            { "codigoOferta": 887180, "idSalaVirtual": 9206, "idSalaVirtualOferta": 1042600,
              "nomeSalaVirtual": "Cálculo" },
        ] });
        let saida = subjects(&ofertas, &historico_json());
        assert_eq!(saida[0].official_grade, Some(8.5));
        assert_eq!(saida[0].situation, "APR.MÉDIA");
    }

    #[test]
    fn o_semestre_corrente_e_o_maior_rotulo() {
        let saida = semesters(&historico_json(), "UNINTER");
        assert_eq!(saida.len(), 2);
        let corrente: Vec<&str> = saida
            .iter()
            .filter(|s| s.current)
            .map(|s| s.external_id.as_str())
            .collect();
        assert_eq!(corrente, ["2026B2"]);
    }

    #[test]
    fn o_rotulo_vira_intervalo_que_ordena_certo() {
        let (i1, f1) = intervalo_do_rotulo("2026B1").unwrap();
        let (i2, f2) = intervalo_do_rotulo("2026B2").unwrap();
        assert!(i1.as_str() < i2.as_str());
        assert!(f1.as_str() < f2.as_str());
        assert_eq!(i2.as_str(), "2026-07-01");
        assert_eq!(f2.as_str(), "2026-08-31");
    }

    // -----------------------------------------------------------------------
    // Trabalhos
    // -----------------------------------------------------------------------

    /// Quatro etapas do mesmo trabalho sao quatro entregas com prazos
    /// diferentes. Chavear so por `idTrabalho` colapsaria as quatro numa.
    #[test]
    fn as_etapas_do_mesmo_trabalho_nao_colapsam() {
        let body = json!({ "trabalhoEtapas": [
            { "id": 394147, "idTrabalho": 352876, "nome": "Atividade Prática - Regular",
              "nomeTrabalhoTipo": "Trabalho", "dataFim": "2026-03-23T23:59:59",
              "dataEntrega": null, "notaEtapa": null },
            { "id": 394148, "idTrabalho": 352876, "nome": "Atividade Prática - 2ª Chamada",
              "nomeTrabalhoTipo": "Trabalho", "dataFim": "2026-05-04T23:59:59",
              "dataEntrega": null, "notaEtapa": null },
        ] });
        let saida = assignments(&body, "906216", brasilia());
        assert_eq!(saida.len(), 2);
        assert_eq!(saida[0].external_id, "352876:394147");
        assert_eq!(saida[1].external_id, "352876:394148");
        assert_ne!(saida[0].external_id, saida[1].external_id);
    }

    #[test]
    fn sem_data_de_entrega_o_trabalho_esta_pendente() {
        let body = json!({ "trabalhoEtapas": [
            { "id": 1, "idTrabalho": 2, "nome": "T", "dataFim": "2026-08-24T23:59:59",
              "dataEntrega": null, "notaEtapa": null },
        ] });
        let saida = assignments(&body, "905706", brasilia());
        assert_eq!(saida[0].status, ExternalAssignmentStatus::Pending);
        assert!(saida[0].submitted_at.is_none());
    }

    // -----------------------------------------------------------------------
    // Materiais
    // -----------------------------------------------------------------------

    /// A identidade e o id numerico do repositorio. A URL assinada entra como
    /// endereco temporario, e nunca como chave.
    #[test]
    fn o_material_e_identificado_pelo_id_do_repositorio_e_nunca_pela_url() {
        let body = json!({ "atividadeItemAprendizagens": [ {
            "itemAprendizagemEtiquetas": [
                { "sistemaRepositorio": null },
                { "sistemaRepositorio": {
                    "id": "60634399",
                    "nome": "PLANO DE ENSINO.pdf",
                    "extensao": "PDF",
                    "url": "JcbQ9Mzjile...?Signature=abc&Expires=1787419297"
                } }
            ]
        } ] });
        let saida = materials(&body, "905706", true);
        assert_eq!(saida.len(), 1);
        assert_eq!(saida[0].external_id, "60634399");
        assert!(!saida[0].external_id.contains("Signature"));
        assert!(!saida[0].external_id.starts_with("http"));
        assert_eq!(saida[0].extension, "pdf");
        assert!(saida[0].complementary);
        assert!(saida[0].temporary_url.is_some());
    }

    #[test]
    fn atividade_sem_arquivo_nao_inventa_material() {
        let body = json!({ "atividadeItemAprendizagens": [
            { "itemAprendizagemEtiquetas": [ { "nomeRotulo": "Título", "sistemaRepositorio": null } ] }
        ] });
        assert!(materials(&body, "905706", false).is_empty());
    }

    // -----------------------------------------------------------------------
    // Envelope e sessao
    // -----------------------------------------------------------------------

    /// `totalRegistros` vem `0` em resposta com itens. Quem conta e o array.
    #[test]
    fn o_contador_mentiroso_do_envelope_e_ignorado() {
        let body = json!({ "aproveitamento": [ {"cdOfertaDisciplina": 1}, {"cdOfertaDisciplina": 2} ],
                           "totalRegistros": 0 });
        assert_eq!(envelope(&body, "aproveitamento").len(), 2);
    }

    #[test]
    fn quatrocentos_e_um_e_sessao_expirada_e_nao_erro_generico() {
        assert!(is_session_expired(401));
        assert!(is_session_expired(403));
        assert!(!is_session_expired(404));
        assert!(!is_session_expired(500));
        assert_eq!(session_expired_error().code, ErrorCode::ProviderUnauthorized);
    }
}
