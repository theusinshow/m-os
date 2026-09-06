//! Gasto da organizacao OpenAI, por projeto, para a faixa de uso.
//!
//! A Admin API Key vive no Credential Manager. O retrato vive somente em
//! memoria: a OpenAI ja e a fonte autoritativa e persistir uma copia faria um
//! numero velho atravessar reinicios com aparencia de atual.

use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration as StdDuration,
};

use keyring::Entry;
use mos_usage::openai;
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use time::OffsetDateTime;

const SERVICE: &str = "m-os";
const ACCOUNT: &str = "openai-admin-api-key";
const INTERVALO: StdDuration = StdDuration::from_secs(5 * 60);

#[derive(Default)]
pub struct OpenAiUsage {
    retrato: Mutex<Option<ResumoOpenAi>>,
    ultimo_erro: Mutex<Option<String>>,
    atualizando: AtomicBool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjetoOpenAi {
    pub id: String,
    pub nome: String,
    pub gasto_micros: i64,
    pub limite_centavos: Option<i64>,
    pub restante_micros: Option<i64>,
    pub limite_ativo: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumoOpenAi {
    pub gasto_micros: i64,
    pub limite_centavos: Option<i64>,
    pub restante_micros: Option<i64>,
    pub limite_ativo: bool,
    pub mes_inicio: String,
    pub atualizado_em: String,
    pub obsoleto: bool,
    pub projetos: Vec<ProjetoOpenAi>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAiUsageStatus {
    pub tem_chave: bool,
    pub atualizando: bool,
    pub ultimo_erro: Option<String>,
    pub resumo: Option<ResumoOpenAi>,
}

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE, ACCOUNT).map_err(|erro| format!("Credential Manager indisponivel: {erro}"))
}

fn chave() -> Result<String, String> {
    entry()?
        .get_password()
        .map_err(|_| "Admin API Key da OpenAI nao configurada.".to_owned())
}

fn tem_chave() -> bool {
    chave().is_ok()
}

fn estado<R: Runtime>(app: &AppHandle<R>) -> Result<tauri::State<'_, OpenAiUsage>, String> {
    app.try_state::<OpenAiUsage>()
        .ok_or_else(|| "Leitura da OpenAI ainda nao iniciou.".to_owned())
}

fn status_de<R: Runtime>(app: &AppHandle<R>) -> OpenAiUsageStatus {
    let Some(estado) = app.try_state::<OpenAiUsage>() else {
        return OpenAiUsageStatus {
            tem_chave: tem_chave(),
            atualizando: false,
            ultimo_erro: None,
            resumo: None,
        };
    };
    let ultimo_erro = estado.ultimo_erro.lock().ok().and_then(|erro| erro.clone());
    let mut resumo = estado.retrato.lock().ok().and_then(|valor| valor.clone());
    if let Some(resumo) = &mut resumo {
        resumo.obsoleto = ultimo_erro.is_some();
    }
    OpenAiUsageStatus {
        tem_chave: tem_chave(),
        atualizando: estado.atualizando.load(Ordering::Relaxed),
        ultimo_erro,
        resumo,
    }
}

#[tauri::command]
pub fn openai_usage_status<R: Runtime>(app: AppHandle<R>) -> OpenAiUsageStatus {
    status_de(&app)
}

#[tauri::command]
pub async fn openai_usage_set_key<R: Runtime>(
    app: AppHandle<R>,
    key: String,
) -> Result<OpenAiUsageStatus, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("A Admin API Key nao pode ficar vazia.".into());
    }
    // Confere ANTES de guardar. Uma chave errada nao deve substituir a ultima
    // que funcionava e deixar a tela afirmando "configurada".
    let resumo = consultar(key).await?;
    entry()?
        .set_password(key)
        .map_err(|erro| format!("Nao foi possivel guardar a chave: {erro}"))?;
    guardar_sucesso(&app, resumo)?;
    Ok(status_de(&app))
}

#[tauri::command]
pub fn openai_usage_clear_key<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => {}
        Err(erro) => return Err(format!("Nao foi possivel remover a chave: {erro}")),
    }
    if let Some(estado) = app.try_state::<OpenAiUsage>() {
        if let Ok(mut retrato) = estado.retrato.lock() {
            *retrato = None;
        }
        if let Ok(mut erro) = estado.ultimo_erro.lock() {
            *erro = None;
        }
    }
    crate::usage::emitir(&app);
    Ok(())
}

#[tauri::command]
pub async fn openai_usage_refresh<R: Runtime>(
    app: AppHandle<R>,
) -> Result<OpenAiUsageStatus, String> {
    atualizar(&app).await?;
    Ok(status_de(&app))
}

pub fn resumo<R: Runtime>(app: &AppHandle<R>) -> Option<ResumoOpenAi> {
    status_de(app).resumo
}

fn guardar_sucesso<R: Runtime>(app: &AppHandle<R>, resumo: ResumoOpenAi) -> Result<(), String> {
    let estado = estado(app)?;
    *estado
        .retrato
        .lock()
        .map_err(|erro| format!("Estado da OpenAI indisponivel: {erro}"))? = Some(resumo);
    *estado
        .ultimo_erro
        .lock()
        .map_err(|erro| format!("Estado da OpenAI indisponivel: {erro}"))? = None;
    crate::usage::emitir(app);
    Ok(())
}

async fn atualizar<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    // O bloco existe para o emprestimo de `app` acabar ANTES do `await`: um
    // `State` vivo atravessando ponto de espera prende a referencia por toda a
    // consulta a rede. `drop()` nao servia — `State` nao implementa `Drop`, e a
    // chamada era um no-op que so enganava quem lesse.
    {
        let estado_atual = estado(app)?;
        if estado_atual.atualizando.swap(true, Ordering::AcqRel) {
            return Err("A leitura da OpenAI ja esta em andamento.".into());
        }
    }

    let resultado = match chave() {
        Ok(chave) => consultar(&chave).await,
        Err(erro) => Err(erro),
    };

    {
        let estado_atual = estado(app)?;
        estado_atual.atualizando.store(false, Ordering::Release);
    }
    match resultado {
        Ok(resumo) => guardar_sucesso(app, resumo),
        Err(erro) => {
            if let Some(estado) = app.try_state::<OpenAiUsage>() {
                if let Ok(mut ultimo) = estado.ultimo_erro.lock() {
                    *ultimo = Some(erro.clone());
                }
            }
            crate::usage::emitir(app);
            Err(erro)
        }
    }
}

pub async fn run<R: Runtime>(app: AppHandle<R>) {
    // Fora do caminho de abertura. O banco e a primeira pintura terminam antes
    // de uma integracao remota poder disputar runtime com eles.
    tokio::time::sleep(StdDuration::from_secs(3)).await;
    loop {
        if tem_chave() {
            let _ = atualizar(&app).await;
        }
        tokio::time::sleep(INTERVALO).await;
    }
}

async fn corpo(
    key: &str,
    pedido: reqwest::RequestBuilder,
    ausente_e_valido: bool,
) -> Result<Option<String>, String> {
    let resposta = pedido
        .bearer_auth(key)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|erro| format!("Nao foi possivel falar com a OpenAI: {erro}"))?;
    let status = resposta.status();
    if ausente_e_valido && status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let texto = resposta
        .text()
        .await
        .map_err(|erro| format!("Nao foi possivel ler a resposta da OpenAI: {erro}"))?;
    if !status.is_success() {
        let detalhe: String = texto.chars().take(240).collect();
        return Err(format!("OpenAI respondeu {status}: {detalhe}"));
    }
    Ok(Some(texto))
}

async fn projetos(cliente: &reqwest::Client, key: &str) -> Result<Vec<openai::Projeto>, String> {
    let mut todos = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let mut query = vec![
            ("limit", "100".to_owned()),
            ("include_archived", "true".to_owned()),
        ];
        if let Some(cursor) = &after {
            query.push(("after", cursor.clone()));
        }
        let texto = corpo(key, cliente.get(openai::PROJECTS_URL).query(&query), false)
            .await?
            .expect("resposta obrigatoria");
        let pagina = openai::ler_projetos(&texto)?;
        todos.extend(pagina.projetos);
        if !pagina.tem_mais {
            break;
        }
        after = pagina.ultimo_id;
        if after.is_none() {
            return Err("A OpenAI indicou mais projetos sem devolver cursor.".into());
        }
    }
    Ok(todos)
}

async fn custos(
    cliente: &reqwest::Client,
    key: &str,
    inicio: i64,
    fim: i64,
) -> Result<BTreeMap<Option<String>, i64>, String> {
    let mut todos = BTreeMap::new();
    let mut page: Option<String> = None;
    loop {
        let mut query = vec![
            ("start_time", inicio.to_string()),
            ("end_time", fim.to_string()),
            ("bucket_width", "1d".to_owned()),
            ("group_by", "project_id".to_owned()),
            ("limit", "31".to_owned()),
        ];
        if let Some(cursor) = &page {
            query.push(("page", cursor.clone()));
        }
        let texto = corpo(key, cliente.get(openai::COSTS_URL).query(&query), false)
            .await?
            .expect("resposta obrigatoria");
        let pagina = openai::ler_custos(&texto)?;
        for (id, valor) in pagina.por_projeto {
            *todos.entry(id).or_insert(0) += valor;
        }
        if !pagina.tem_mais {
            break;
        }
        page = pagina.proxima;
        if page.is_none() {
            return Err("A OpenAI indicou mais custos sem devolver cursor.".into());
        }
    }
    Ok(todos)
}

async fn limite(
    cliente: &reqwest::Client,
    key: &str,
    url: &str,
) -> Result<Option<openai::LimiteMensal>, String> {
    let Some(texto) = corpo(key, cliente.get(url), true).await? else {
        return Ok(None);
    };
    openai::ler_limite(&texto).map(Some)
}

async fn consultar(key: &str) -> Result<ResumoOpenAi, String> {
    let cliente = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(15))
        .build()
        .map_err(|erro| format!("Nao foi possivel iniciar o cliente OpenAI: {erro}"))?;
    let agora = OffsetDateTime::now_utc();
    let inicio = agora
        .date()
        .replace_day(1)
        .map_err(|erro| format!("Nao foi possivel calcular o inicio do mes: {erro}"))?
        .midnight()
        .assume_utc();

    let projetos = projetos(&cliente, key).await?;
    let custos = custos(
        &cliente,
        key,
        inicio.unix_timestamp(),
        agora.unix_timestamp(),
    )
    .await?;
    let limite_organizacao = limite(&cliente, key, openai::ORGANIZATION_LIMIT_URL).await?;

    let nomes: HashMap<String, String> = projetos
        .iter()
        .map(|projeto| (projeto.id.clone(), projeto.nome.clone()))
        .collect();
    let mut limites = HashMap::new();
    for projeto in &projetos {
        let encontrado = limite(&cliente, key, &openai::project_limit_url(&projeto.id)).await?;
        limites.insert(projeto.id.clone(), encontrado);
    }

    let mut linhas: Vec<ProjetoOpenAi> = projetos
        .into_iter()
        .map(|projeto| {
            let gasto = custos.get(&Some(projeto.id.clone())).copied().unwrap_or(0);
            let limite = limites.remove(&projeto.id).flatten();
            ProjetoOpenAi {
                id: projeto.id,
                nome: projeto.nome,
                gasto_micros: gasto,
                limite_centavos: limite.as_ref().map(|valor| valor.centavos),
                restante_micros: limite
                    .as_ref()
                    .map(|valor| openai::restante_micros(gasto, valor.centavos)),
                limite_ativo: limite.is_some_and(|valor| valor.impondo),
            }
        })
        .collect();

    // Custos de projeto arquivado/desconhecido continuam visiveis. E custo sem
    // project_id nao e repartido: a proveniencia ausente vira uma linha propria.
    for (id, gasto) in &custos {
        match id {
            Some(id) if !nomes.contains_key(id) => linhas.push(ProjetoOpenAi {
                id: id.clone(),
                nome: id.clone(),
                gasto_micros: *gasto,
                limite_centavos: None,
                restante_micros: None,
                limite_ativo: false,
            }),
            None => linhas.push(ProjetoOpenAi {
                id: "sem-projeto".into(),
                nome: "Sem projeto".into(),
                gasto_micros: *gasto,
                limite_centavos: None,
                restante_micros: None,
                limite_ativo: false,
            }),
            _ => {}
        }
    }
    linhas.sort_by(|a, b| {
        b.gasto_micros
            .cmp(&a.gasto_micros)
            .then_with(|| a.nome.cmp(&b.nome))
    });

    let gasto_total = custos.values().copied().sum();
    let restante = limite_organizacao
        .as_ref()
        .map(|valor| openai::restante_micros(gasto_total, valor.centavos));
    Ok(ResumoOpenAi {
        gasto_micros: gasto_total,
        limite_centavos: limite_organizacao.as_ref().map(|valor| valor.centavos),
        restante_micros: restante,
        limite_ativo: limite_organizacao.is_some_and(|valor| valor.impondo),
        mes_inicio: inicio
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|erro| format!("Nao foi possivel formatar o inicio do mes: {erro}"))?,
        atualizado_em: agora
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|erro| format!("Nao foi possivel formatar a leitura: {erro}"))?,
        obsoleto: false,
        projetos: linhas,
    })
}
