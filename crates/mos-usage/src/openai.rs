//! Os fatos financeiros que a API administrativa da OpenAI devolve.
//!
//! Este modulo conhece JSON e aritmetica, mas nao rede nem credencial. O shell
//! escolhe quando e como pedir; aqui a resposta de terceiro vira tipos pequenos
//! e testaveis. E a mesma fronteira que `cota` usa para a Anthropic.

use std::collections::BTreeMap;

use serde::Deserialize;

pub const PROJECTS_URL: &str = "https://api.openai.com/v1/organization/projects";
pub const COSTS_URL: &str = "https://api.openai.com/v1/organization/costs";
pub const ORGANIZATION_LIMIT_URL: &str =
    "https://api.openai.com/v1/organization/spend_limit";

pub fn project_limit_url(project_id: &str) -> String {
    format!(
        "https://api.openai.com/v1/organization/projects/{project_id}/spend_limit"
    )
}

/// US$ 1 em micros. Custo de modelo pode ser menor que um centavo; centavos
/// seriam uma unidade destrutiva para a soma.
pub const MICROS_POR_DOLAR: i64 = 1_000_000;
pub const MICROS_POR_CENTAVO: i64 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projeto {
    pub id: String,
    pub nome: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginaDeProjetos {
    pub projetos: Vec<Projeto>,
    pub tem_mais: bool,
    pub ultimo_id: Option<String>,
}

#[derive(Deserialize)]
struct ProjectsResponse {
    #[serde(default)]
    data: Vec<ProjectResponse>,
    #[serde(default)]
    has_more: bool,
    #[serde(default)]
    last_id: Option<String>,
}

#[derive(Deserialize)]
struct ProjectResponse {
    id: String,
    #[serde(default)]
    name: Option<String>,
}

pub fn ler_projetos(corpo: &str) -> Result<PaginaDeProjetos, String> {
    let resposta: ProjectsResponse = serde_json::from_str(corpo)
        .map_err(|erro| format!("lista de projetos inesperada: {erro}"))?;
    let projetos = resposta
        .data
        .into_iter()
        .map(|projeto| Projeto {
            nome: projeto.name.unwrap_or_else(|| projeto.id.clone()),
            id: projeto.id,
        })
        .collect();
    Ok(PaginaDeProjetos {
        projetos,
        tem_mais: resposta.has_more,
        ultimo_id: resposta.last_id,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginaDeCustos {
    /// Micros de USD por `project_id`. `None` e custo que a OpenAI nao atribuiu
    /// a projeto nenhum, e continua explicito em vez de ser repartido.
    pub por_projeto: BTreeMap<Option<String>, i64>,
    pub tem_mais: bool,
    pub proxima: Option<String>,
}

#[derive(Deserialize)]
struct CostsResponse {
    #[serde(default)]
    data: Vec<CostBucket>,
    #[serde(default)]
    has_more: bool,
    #[serde(default)]
    next_page: Option<String>,
}

#[derive(Deserialize)]
struct CostBucket {
    #[serde(default)]
    results: Vec<CostResult>,
}

#[derive(Deserialize)]
struct CostResult {
    #[serde(default)]
    amount: Option<Amount>,
    #[serde(default)]
    project_id: Option<String>,
}

#[derive(Deserialize)]
struct Amount {
    value: f64,
    currency: String,
}

fn para_micros(valor: f64) -> Result<i64, String> {
    if !valor.is_finite() {
        return Err("custo nao finito".into());
    }
    let micros = valor * MICROS_POR_DOLAR as f64;
    if micros < i64::MIN as f64 || micros > i64::MAX as f64 {
        return Err("custo fora do intervalo suportado".into());
    }
    Ok(micros.round() as i64)
}

pub fn ler_custos(corpo: &str) -> Result<PaginaDeCustos, String> {
    let resposta: CostsResponse = serde_json::from_str(corpo)
        .map_err(|erro| format!("custos inesperados: {erro}"))?;
    let mut por_projeto = BTreeMap::new();
    for bucket in resposta.data {
        for resultado in bucket.results {
            let Some(amount) = resultado.amount else { continue };
            if !amount.currency.eq_ignore_ascii_case("usd") {
                return Err(format!("moeda de custo nao suportada: {}", amount.currency));
            }
            let valor = para_micros(amount.value)?;
            *por_projeto.entry(resultado.project_id).or_insert(0) += valor;
        }
    }
    Ok(PaginaDeCustos {
        por_projeto,
        tem_mais: resposta.has_more,
        proxima: resposta.next_page,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimiteMensal {
    /// O endpoint documenta `threshold_amount` em centavos.
    pub centavos: i64,
    pub impondo: bool,
}

#[derive(Deserialize)]
struct SpendLimitResponse {
    threshold_amount: i64,
    currency: String,
    interval: String,
    enforcement: Enforcement,
}

#[derive(Deserialize)]
struct Enforcement {
    status: String,
}

pub fn ler_limite(corpo: &str) -> Result<LimiteMensal, String> {
    let resposta: SpendLimitResponse = serde_json::from_str(corpo)
        .map_err(|erro| format!("limite inesperado: {erro}"))?;
    if !resposta.currency.eq_ignore_ascii_case("usd") {
        return Err(format!("moeda de limite nao suportada: {}", resposta.currency));
    }
    if resposta.interval != "month" {
        return Err(format!("intervalo de limite nao suportado: {}", resposta.interval));
    }
    if resposta.threshold_amount < 0 {
        return Err("limite mensal negativo".into());
    }
    Ok(LimiteMensal {
        centavos: resposta.threshold_amount,
        impondo: resposta.enforcement.status == "enforcing",
    })
}

pub fn restante_micros(gasto_micros: i64, limite_centavos: i64) -> i64 {
    limite_centavos
        .saturating_mul(MICROS_POR_CENTAVO)
        .saturating_sub(gasto_micros)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_projetos_e_preserva_cursor() {
        let pagina = ler_projetos(
            r#"{"data":[{"id":"proj_nexo","name":"NexoDoc"},{"id":"proj_sem_nome"}],"has_more":true,"last_id":"proj_sem_nome"}"#,
        )
        .unwrap();
        assert_eq!(pagina.projetos[0].nome, "NexoDoc");
        assert_eq!(pagina.projetos[1].nome, "proj_sem_nome");
        assert!(pagina.tem_mais);
        assert_eq!(pagina.ultimo_id.as_deref(), Some("proj_sem_nome"));
    }

    #[test]
    fn soma_buckets_por_projeto_sem_arredondar_para_centavos() {
        let pagina = ler_custos(
            r#"{"data":[{"results":[{"amount":{"value":0.004321,"currency":"usd"},"project_id":"proj_nexo"},{"amount":{"value":1.25,"currency":"usd"},"project_id":"proj_truss"}]},{"results":[{"amount":{"value":0.005679,"currency":"usd"},"project_id":"proj_nexo"},{"amount":{"value":0.5,"currency":"usd"},"project_id":null}]}],"has_more":false,"next_page":null}"#,
        )
        .unwrap();
        assert_eq!(pagina.por_projeto[&Some("proj_nexo".into())], 10_000);
        assert_eq!(pagina.por_projeto[&Some("proj_truss".into())], 1_250_000);
        assert_eq!(pagina.por_projeto[&None], 500_000);
    }

    #[test]
    fn custo_em_outra_moeda_falha_em_vez_de_somar_coisas_diferentes() {
        let erro = ler_custos(
            r#"{"data":[{"results":[{"amount":{"value":1,"currency":"brl"},"project_id":"p"}]}]}"#,
        )
        .unwrap_err();
        assert!(erro.contains("moeda"));
    }

    #[test]
    fn le_limite_em_centavos_e_estado_de_aplicacao() {
        let limite = ler_limite(
            r#"{"threshold_amount":10000,"currency":"USD","interval":"month","enforcement":{"status":"enforcing"}}"#,
        )
        .unwrap();
        assert_eq!(limite.centavos, 10_000);
        assert!(limite.impondo);
    }

    #[test]
    fn restante_pode_ficar_negativo_sem_esconder_o_estouro() {
        assert_eq!(restante_micros(12_000_000, 1_000), -2_000_000);
    }
}
