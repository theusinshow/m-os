//! O provedor Univirtus: sessao, cliente HTTP e coleta.
//!
//! Tudo que e peculiar ao AVA da UNINTER mora aqui. Acima deste arquivo so
//! existem os tipos neutros de `mos_core::academic_sync` — nenhuma tela, nenhum
//! comando de dominio e nenhuma camada do M/Academic sabe que `/ava/bqs/` existe.
//!
//! A investigacao que produziu os endereços esta em
//! `docs/UNIVIRTUS-INTEGRATION.md`. O que segue e o que ela virou.
//!
//! # A sessao
//!
//! Duas pecas, e so elas: o cookie `HttpOnly` da sessao e o header `X-time`.
//! Medido na investigacao: sem header nenhum a API responde 401; so com
//! `X-Requested-With` responde 401; **so com `X-time` responde 200**. E o
//! `X-time` nao e um relogio — sao 18 digitos em formato de ticks .NET, mas
//! ticks calculados na hora sao recusados. Ele e emitido no login.
//!
//! Por isso **nao existe `authenticate(usuario, senha)` aqui**. Nao ha endpoint
//! que troque credencial por token; a sessao nasce de um login de navegador de
//! verdade, na pagina oficial, e o M/OS so a guarda. Um formulario proprio
//! pedindo RU e senha seria uma promessa que o portal nao cumpre — e faria o
//! M/OS carregar a senha de alguem sem nenhuma necessidade.
//!
//! # Onde os segredos ficam
//!
//! No Credential Manager do Windows, pelo mesmo caminho de
//! `mos-hermes/src/auth.rs` e de `finance.rs`. Nunca no banco, nunca em log,
//! nunca no renderer. O que atravessa a fronteira para a tela e o `bool` de
//! "esta conectado" e as contagens — jamais o valor.

use keyring::Entry;
use mos_core::academic_sync::{
    ExternalAssessment, ExternalAssignment, ExternalMaterial, ExternalSemester, ExternalSubject,
    ProviderSnapshot, PROVIDER_UNIVIRTUS,
};
use mos_core::{univirtus as norm, CoreError, ErrorCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::UtcOffset;

const SERVICE: &str = "m-os";
const ACCOUNT: &str = "univirtus-session";

/// O host, escrito uma vez so.
///
/// Um macro, e nao tres strings soltas: o cookie e lido para este dominio, as
/// URLs sao montadas a partir dele e a janela de login abre nele. Se os tres
/// pudessem divergir, o M/OS acabaria pedindo cookie de um lugar e falando com
/// outro — e o mesmo cuidado que o `finance_host!` de `finance.rs` ja toma.
macro_rules! univirtus_host {
    () => {
        "univirtus.uninter.com"
    };
}

pub const LOGIN_URL: &str = concat!("https://", univirtus_host!(), "/ava/web/");
const BASE: &str = concat!("https://", univirtus_host!());

/// Quanto tempo esperar por uma resposta. O AVA e lento em horario de pico, e
/// 30s e o ponto em que insistir passa a ser pior que avisar.
const TIMEOUT_SEGUNDOS: u64 = 30;

// ===========================================================================
// A allowlist
// ===========================================================================

/// As UNICAS rotas que este cliente sabe montar.
///
/// # Por que allowlist, e nao blacklist
///
/// No Univirtus existe **GET que altera estado**. A acao `Iniciar` de uma
/// avaliacao consome uma das tentativas e faz a prova comecar a correr;
/// `AvisoDestinatarioPopup/{id}/Lido/true` marca aviso como lido;
/// `GetAtividadeItemAprendizagemAcessado` registra acesso e mexe no
/// `porcentagemAcessado` — que e como a pessoa sabe o que ja estudou.
///
/// Uma blacklist protegeria contra os tres que a investigacao encontrou, e
/// contra nenhum dos que ela nao encontrou. Um enum fechado protege contra
/// todos: **nao existe `request(path)` neste arquivo**. Para chamar algo novo e
/// preciso acrescentar uma variante aqui, e escrever a variante e o momento em
/// que alguem tem de pensar se aquilo e mesmo somente leitura.
#[derive(Clone, Debug)]
enum Rota {
    /// O ping mais barato que prova que a sessao vive.
    Sessao,
    /// Curso e situacao do aluno.
    Curso,
    /// Historico academico: semestre, situacao e media oficial.
    Historico { chave_aluno: String },
    /// As ofertas em que o aluno esta inscrito.
    Ofertas,
    /// Avaliacoes de uma disciplina.
    Avaliacoes { id_sala: String, id_oferta: String },
    /// Trabalhos de uma disciplina.
    Trabalhos { id_oferta: String },
    /// As aulas do roteiro (`TipoOferta/1`) ou o material complementar (`2`).
    Estrutura {
        id_sala: String,
        id_oferta: String,
        tipo: u8,
    },
    /// As atividades de uma aula do roteiro.
    AtividadesDaAula {
        id_oferta: String,
        id_estrutura: String,
    },
    /// As atividades de uma secao de material complementar.
    AtividadesComplementares {
        id_oferta: String,
        id_estrutura: String,
    },
    /// Os arquivos de uma atividade.
    Arquivos {
        id_atividade: String,
        complementar: bool,
    },
}

impl Rota {
    fn caminho(&self) -> String {
        match self {
            Self::Sessao => "/ava/sistema/Escola/0/Usuario".into(),
            Self::Curso => "/ava/sistema/UsuarioCurso/0/GetCursosAproveitamento?idUsuario=0".into(),
            Self::Historico { chave_aluno } => format!(
                "/ava/integracao/UsuarioIntegracaoSistemaAcademico/0/GetDisciplinasAproveitamento?sidCdAluno={}",
                urlencode(chave_aluno)
            ),
            Self::Ofertas => "/ava/sistema/UsuarioHistoricoCursoOferta/false/Usuario/".into(),
            Self::Avaliacoes { id_sala, id_oferta } => format!(
                "/ava/bqs/AvaliacaoUsuario/1/paginacao/true?numRegistros=100&filtro=&ordenacao=\
                 &idSalaVirtual={id_sala}&idSalaVirtualOferta={id_oferta}\
                 &ajustarDatasMatriculaCurso=false"
            ),
            Self::Trabalhos { id_oferta } => format!(
                "/ava/interacao/TrabalhoEtapa/{id_oferta}/GetEtapasByOfertaInscrito/false\
                 ?master=true&idSalaVirtualOfertaAproveitamento={id_oferta}"
            ),
            Self::Estrutura {
                id_sala,
                id_oferta,
                tipo,
            } => format!(
                "/ava/ava/SalaVirtualEstrutura/{id_sala}/TipoOferta/{tipo}\
                 ?idSalaVirtualOferta={id_oferta}&idSalaVirtualOfertaAproveitamento=\
                 &idSalaVirtualOfertaPai="
            ),
            Self::AtividadesDaAula {
                id_oferta,
                id_estrutura,
            } => format!(
                "/ava/ava/salaVirtualAtividade/0/EstruturaOferta/{id_oferta}/?id={id_estrutura}\
                 &editar=false&idSalaVirtualOfertaPai=&idSalaVirtualOfertaAproveitamento="
            ),
            Self::AtividadesComplementares {
                id_oferta,
                id_estrutura,
            } => format!(
                "/ava/ava/SalaVirtualAtividade/{id_estrutura}/EstruturaOferta/{id_oferta}\
                 ?idSalaVirtualOfertaPai=null&idSalaVirtualOfertaAproveitamento=null\
                 &buscarItemAprendizagem=true&ocultarAtividadeSemItem=true"
            ),
            Self::Arquivos {
                id_atividade,
                complementar,
            } => format!(
                "/ava/atv/AtividadeItemAprendizagem/{id_atividade}/Atividade?complementar={complementar}"
            ),
        }
    }
}

/// As unicas rotas do Univirtus que o M/OS pode tocar, por prefixo.
///
/// Segunda camada, e nao a primeira: quem protege e o enum `Rota`. Esta lista
/// existe porque uma variante nova pode ser escrita errada, e um assert em
/// tempo de execucao pega o erro antes de ele virar uma tentativa de prova
/// iniciada. Ver `caminho_permitido`.
const PREFIXOS_PERMITIDOS: [&str; 8] = [
    "/ava/sistema/Escola/",
    "/ava/sistema/UsuarioCurso/",
    "/ava/sistema/UsuarioHistoricoCursoOferta/",
    "/ava/integracao/UsuarioIntegracaoSistemaAcademico/",
    "/ava/bqs/AvaliacaoUsuario/",
    "/ava/interacao/TrabalhoEtapa/",
    "/ava/ava/",
    "/ava/atv/AtividadeItemAprendizagem/",
];

/// Fragmentos que NUNCA podem aparecer numa URL montada aqui.
///
/// Sao os GET com efeito colateral que a investigacao encontrou. Eles ja estao
/// fora do enum; esta lista e o cinto sobre o suspensorio, e o teste que a
/// acompanha e o que impede alguem de reintroduzi-los sem perceber.
const FRAGMENTOS_PROIBIDOS: [&str; 4] = [
    "AvisoDestinatarioPopup",
    "GetAtividadeItemAprendizagemAcessado",
    "/Iniciar",
    "IniciarAvaliacao",
];

fn caminho_permitido(caminho: &str) -> bool {
    if FRAGMENTOS_PROIBIDOS
        .iter()
        .any(|proibido| caminho.contains(proibido))
    {
        return false;
    }
    PREFIXOS_PERMITIDOS
        .iter()
        .any(|prefixo| caminho.starts_with(prefixo))
}

fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            outro => format!("%{outro:02X}"),
        })
        .collect()
}

// ===========================================================================
// O cofre da sessao
// ===========================================================================

/// O que o login produziu. **Nunca serializado para fora do keyring.**
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnivirtusSession {
    /// O header `Cookie` inteiro, como o navegador o mandaria.
    cookie: String,
    /// O `X-time` emitido no login.
    x_time: String,
}

impl UnivirtusSession {
    pub fn new(cookie: String, x_time: String) -> Result<Self, CoreError> {
        let cookie = cookie.trim().to_owned();
        let x_time = x_time.trim().to_owned();
        if cookie.is_empty() {
            return Err(erro(
                "A sessao veio sem cookie. Entre no portal e tente de novo.",
            ));
        }
        // 18 digitos de ticks .NET. A checagem e de formato, e nao de valor: o
        // valor so o servidor sabe validar, e tentar adivinha-lo foi o que a
        // investigacao provou nao funcionar.
        if x_time.len() < 10 || !x_time.chars().all(|c| c.is_ascii_digit()) {
            return Err(erro(
                "A sessao veio sem o X-time. Conclua o login no portal antes de fechar a janela.",
            ));
        }
        Ok(Self { cookie, x_time })
    }
}

// `Debug` manual: o derive imprimiria cookie e X-time em qualquer `{:?}`, e um
// `{:?}` num log e tudo que separa "segredo guardado" de "segredo vazado".
impl std::fmt::Display for UnivirtusSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UnivirtusSession(<redigida>)")
    }
}

fn entry() -> Result<Entry, CoreError> {
    Entry::new(SERVICE, ACCOUNT)
        .map_err(|error| erro(&format!("Credential Manager indisponivel: {error}")))
}

pub fn guardar_sessao(session: &UnivirtusSession) -> Result<(), CoreError> {
    let payload = serde_json::to_string(session)
        .map_err(|_| erro("Nao foi possivel preparar a sessao para guardar."))?;
    entry()?
        .set_password(&payload)
        .map_err(|error| erro(&format!("Nao foi possivel guardar a sessao: {error}")))
}

pub fn ler_sessao() -> Option<UnivirtusSession> {
    let bruto = entry().ok()?.get_password().ok()?;
    serde_json::from_str(&bruto).ok()
}

pub fn esquecer_sessao() -> Result<(), CoreError> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(erro(&format!("Nao foi possivel remover a sessao: {error}"))),
    }
}

pub fn ha_sessao() -> bool {
    ler_sessao().is_some()
}

// ===========================================================================
// O cliente
// ===========================================================================

pub struct UnivirtusClient {
    http: reqwest::Client,
    session: UnivirtusSession,
}

impl UnivirtusClient {
    pub fn new(session: UnivirtusSession) -> Result<Self, CoreError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(TIMEOUT_SEGUNDOS))
            // Redirect desligado: um 302 para a tela de login e a forma como o
            // portal diz "sua sessao morreu". Seguir o redirect transformaria
            // isso num HTML de 200, e o sync concluiria que o aluno nao tem
            // disciplina nenhuma.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| erro(&format!("Nao foi possivel abrir o cliente: {error}")))?;
        Ok(Self { http, session })
    }

    /// A UNICA porta de saida. Toda chamada passa por aqui, e aqui a allowlist
    /// e verificada antes de o socket abrir.
    async fn get(&self, rota: Rota) -> Result<Value, CoreError> {
        let caminho = rota.caminho();
        if !caminho_permitido(&caminho) {
            // Nao e erro de rede: e uma rota que ninguem revisou. Falhar alto e
            // o ponto.
            return Err(erro(
                "Rota do Univirtus fora da allowlist. Nenhuma chamada foi feita.",
            ));
        }
        let resposta = self
            .http
            .get(format!("{BASE}{caminho}"))
            .header("Cookie", &self.session.cookie)
            .header("X-time", &self.session.x_time)
            .header("Accept", "application/json")
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await
            .map_err(|error| {
                CoreError::new(
                    ErrorCode::Io,
                    format!("O Univirtus nao respondeu: {error}"),
                    true,
                )
            })?;

        let status = resposta.status().as_u16();
        if norm::is_session_expired(status) || (300..400).contains(&status) {
            return Err(norm::session_expired_error());
        }
        if status == 404 {
            // 404 e resposta legitima do portal para "nao ha nada aqui" — uma
            // disciplina sem avaliacao publicada devolve isso. Envelope vazio.
            return Ok(Value::Object(Default::default()));
        }
        if !(200..300).contains(&status) {
            return Err(CoreError::new(
                ErrorCode::Io,
                format!("O Univirtus respondeu {status}."),
                true,
            ));
        }
        let corpo = resposta.text().await.unwrap_or_default();
        if corpo.trim().is_empty() {
            return Ok(Value::Object(Default::default()));
        }
        serde_json::from_str(&corpo).map_err(|_| {
            CoreError::new(
                ErrorCode::DataIntegrity,
                String::from("O Univirtus respondeu algo que nao e JSON. A sessao pode ter caido."),
                true,
            )
        })
    }

    /// A sessao vive? Devolve `Ok(())` ou o erro de sessao expirada.
    pub async fn check_session(&self) -> Result<(), CoreError> {
        self.get(Rota::Sessao).await.map(|_| ())
    }

    /// Um retrato do semestre corrente.
    ///
    /// # Por que so o corrente
    ///
    /// Nao ha endpoint consolidado: avaliacoes e trabalhos sao **duas chamadas
    /// por disciplina**. Varrer o historico inteiro custaria ~30 chamadas em
    /// toda sincronizacao para reconfirmar notas de semestres fechados, que nao
    /// mudam. O historico academico completo vem de graca na primeira chamada, e
    /// e ele que da semestre e media oficial de tudo.
    ///
    /// O recorte e declarado no retrato: `subjects` traz so as disciplinas
    /// perguntadas, e a reconciliacao usa isso para nao marcar como ausente o
    /// que nem foi pedido.
    pub async fn snapshot(&self, offset: UtcOffset) -> Result<ProviderSnapshot, CoreError> {
        let mut retrato = ProviderSnapshot::new(PROVIDER_UNIVIRTUS);

        // 1. Curso e a chave cifrada do aluno.
        let curso = self.get(Rota::Curso).await?;
        retrato.context = norm::context(&curso);
        let instituicao = retrato
            .context
            .as_ref()
            .map(|c| c.course_name.clone())
            .unwrap_or_default();
        let Some(chave_aluno) = norm::student_key(&curso) else {
            return Err(erro(
                "O Univirtus nao devolveu o vinculo do aluno. Reconecte e tente de novo.",
            ));
        };

        // 2. Historico: semestres, situacao e media oficial de TODAS.
        let historico = self.get(Rota::Historico { chave_aluno }).await?;
        let semestres: Vec<ExternalSemester> = norm::semesters(&historico, &instituicao);
        let Some(corrente) = semestres.iter().find(|s| s.current).cloned() else {
            // Sem semestre nao ha o que sincronizar, e isso nao e erro: um
            // aluno entre periodos esta nesse estado.
            retrato.semesters = semestres;
            return Ok(retrato);
        };

        // 3. Ofertas e disciplinas.
        let ofertas = self.get(Rota::Ofertas).await?;
        let todas: Vec<ExternalSubject> = norm::subjects(&ofertas, &historico);
        let enderecos = norm::subject_query_refs(&ofertas);
        let do_corrente: Vec<ExternalSubject> = todas
            .iter()
            .filter(|s| s.semester_external_id == corrente.external_id)
            .cloned()
            .collect();

        retrato.semesters = semestres;
        retrato.subjects = do_corrente.clone();

        // 4. Por disciplina: avaliacoes, trabalhos e materiais.
        //
        // Em serie, e nao em paralelo. Sao poucas disciplinas por semestre, e
        // disparar dez requisicoes simultaneas contra um portal academico e o
        // tipo de gentileza que nao se pede de volta.
        for disciplina in &do_corrente {
            let Some(endereco) = enderecos
                .iter()
                .find(|e| e.codigo_oferta == disciplina.external_id)
            else {
                continue;
            };

            match self
                .avaliacoes_e_trabalhos(endereco, &disciplina.external_id, offset)
                .await
            {
                Ok((avaliacoes, trabalhos)) => {
                    retrato.assessments.extend(avaliacoes);
                    retrato.assignments.extend(trabalhos);
                }
                // Sessao caida derruba tudo: continuar produziria um retrato
                // vazio que a reconciliacao leria como "sumiu tudo".
                Err(e) if e.code == ErrorCode::ProviderUnauthorized => return Err(e),
                // Uma disciplina que falhou vira aviso. As outras continuam.
                Err(e) => retrato
                    .warnings
                    .push(format!("{}: {}", disciplina.name, e.message)),
            }

            match self.materiais(endereco, &disciplina.external_id).await {
                Ok(materiais) => retrato.materials.extend(materiais),
                Err(e) if e.code == ErrorCode::ProviderUnauthorized => return Err(e),
                Err(e) => retrato
                    .warnings
                    .push(format!("{} (materiais): {}", disciplina.name, e.message)),
            }
        }

        Ok(retrato)
    }

    async fn avaliacoes_e_trabalhos(
        &self,
        endereco: &norm::SubjectQueryRef,
        subject_external_id: &str,
        offset: UtcOffset,
    ) -> Result<(Vec<ExternalAssessment>, Vec<ExternalAssignment>), CoreError> {
        let avaliacoes = self
            .get(Rota::Avaliacoes {
                id_sala: endereco.id_sala_virtual.clone(),
                id_oferta: endereco.id_sala_virtual_oferta.clone(),
            })
            .await?;
        let trabalhos = self
            .get(Rota::Trabalhos {
                id_oferta: endereco.id_sala_virtual_oferta.clone(),
            })
            .await?;
        Ok((
            norm::assessments(&avaliacoes, subject_external_id, offset),
            norm::assignments(&trabalhos, subject_external_id, offset),
        ))
    }

    /// Os materiais de uma disciplina, das DUAS fontes.
    ///
    /// O roteiro (`TipoOferta/1`) e o material complementar (`TipoOferta/2`) sao
    /// estruturas diferentes, e o **Plano de Ensino mora na segunda**. Varrer so
    /// o roteiro perderia o documento mais util da disciplina inteira — foi
    /// medido: das oito aulas de uma disciplina, seis nao tinham arquivo nenhum.
    async fn materiais(
        &self,
        endereco: &norm::SubjectQueryRef,
        subject_external_id: &str,
    ) -> Result<Vec<ExternalMaterial>, CoreError> {
        let mut saida = Vec::new();

        for (tipo, complementar) in [(1u8, false), (2u8, true)] {
            let estrutura = self
                .get(Rota::Estrutura {
                    id_sala: endereco.id_sala_virtual.clone(),
                    id_oferta: endereco.id_sala_virtual_oferta.clone(),
                    tipo,
                })
                .await?;
            for id_estrutura in norm::structure_ids(&estrutura) {
                let rota = if complementar {
                    Rota::AtividadesComplementares {
                        id_oferta: endereco.id_sala_virtual_oferta.clone(),
                        id_estrutura,
                    }
                } else {
                    Rota::AtividadesDaAula {
                        id_oferta: endereco.id_sala_virtual_oferta.clone(),
                        id_estrutura,
                    }
                };
                let atividades = self.get(rota).await?;
                for id_atividade in norm::activity_ids(&atividades) {
                    let arquivos = self
                        .get(Rota::Arquivos {
                            id_atividade,
                            complementar,
                        })
                        .await?;
                    saida.extend(norm::materials(
                        &arquivos,
                        subject_external_id,
                        complementar,
                    ));
                }
            }
        }

        // O mesmo PDF pode aparecer em duas atividades. O conjunto e por id.
        saida.sort_by(|a, b| a.external_id.cmp(&b.external_id));
        saida.dedup_by(|a, b| a.external_id == b.external_id);
        Ok(saida)
    }
}

fn erro(mensagem: &str) -> CoreError {
    CoreError::new(ErrorCode::Io, mensagem.to_owned(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Toda rota que o enum sabe montar tem de passar pela allowlist. Se uma
    /// variante nova nascer com prefixo errado, este teste morre antes de ela
    /// chegar ao portal.
    #[test]
    fn toda_rota_do_enum_esta_na_allowlist() {
        let rotas = [
            Rota::Sessao,
            Rota::Curso,
            Rota::Historico {
                chave_aluno: "abc==".into(),
            },
            Rota::Ofertas,
            Rota::Avaliacoes {
                id_sala: "60236".into(),
                id_oferta: "1161461".into(),
            },
            Rota::Trabalhos {
                id_oferta: "1161461".into(),
            },
            Rota::Estrutura {
                id_sala: "60236".into(),
                id_oferta: "1161461".into(),
                tipo: 1,
            },
            Rota::AtividadesDaAula {
                id_oferta: "1161461".into(),
                id_estrutura: "2758788".into(),
            },
            Rota::AtividadesComplementares {
                id_oferta: "1161461".into(),
                id_estrutura: "1179754".into(),
            },
            Rota::Arquivos {
                id_atividade: "8544442".into(),
                complementar: true,
            },
        ];
        for rota in rotas {
            let caminho = rota.caminho();
            assert!(
                caminho_permitido(&caminho),
                "rota fora da allowlist: {caminho}"
            );
        }
    }

    /// Os GET com efeito colateral que a investigacao encontrou. Nenhum deles
    /// pode passar, nem que alguem os escreva a mao.
    #[test]
    fn os_gets_que_alteram_estado_sao_recusados() {
        for perigoso in [
            "/ava/sistema/AvisoDestinatarioPopup/774975901/Lido/true",
            "/ava/atv/AtividadeItemAprendizagem/8544442/GetAtividadeItemAprendizagemAcessado/",
            "/ava/bqs/AvaliacaoUsuario/2713958/Iniciar",
            "/ava/bqs/IniciarAvaliacao/2713958",
        ] {
            assert!(
                !caminho_permitido(perigoso),
                "deveria ser recusado: {perigoso}"
            );
        }
    }

    #[test]
    fn rota_de_outro_dominio_ou_fora_do_ava_nao_passa() {
        for fora in [
            "/api/qualquer",
            "/ava/sistema/Aviso/1/paginacao/true",
            "https://outro.com/ava/sistema/Escola/0/Usuario",
        ] {
            assert!(!caminho_permitido(fora), "deveria ser recusado: {fora}");
        }
    }

    /// A chave do aluno e cifrada e contem `+`, `/` e `=`. Sem escapar, a query
    /// chega truncada e o portal devolve historico vazio — que a sincronizacao
    /// leria como "o aluno nao tem disciplina".
    #[test]
    fn a_chave_do_aluno_vai_escapada_na_query() {
        let rota = Rota::Historico {
            chave_aluno: "Rv2YM42proH8soc+ruRna/g==".into(),
        };
        let caminho = rota.caminho();
        assert!(caminho.contains("%2B"), "o + tem de virar %2B");
        assert!(caminho.contains("%2F"), "a / tem de virar %2F");
        assert!(caminho.contains("%3D"), "o = tem de virar %3D");
        assert!(!caminho.contains("soc+ruRna"));
    }

    #[test]
    fn a_sessao_recusa_x_time_que_nao_e_ticks() {
        assert!(UnivirtusSession::new("s=1".into(), "638912345678901234".into()).is_ok());
        assert!(UnivirtusSession::new("s=1".into(), String::new()).is_err());
        assert!(UnivirtusSession::new("s=1".into(), "abc".into()).is_err());
        assert!(UnivirtusSession::new(String::new(), "638912345678901234".into()).is_err());
    }

    /// Um `{:?}` num log nao pode imprimir o segredo.
    #[test]
    fn a_sessao_nao_se_imprime() {
        let sessao =
            UnivirtusSession::new("ASPSESSIONID=segredo".into(), "638912345678901234".into())
                .unwrap();
        let texto = format!("{sessao}");
        assert!(!texto.contains("segredo"));
        assert!(!texto.contains("638912345678901234"));
    }
}
