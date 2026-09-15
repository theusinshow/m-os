//! A saude da sincronizacao: o que uma falha significa, quanto esperar antes
//! de insistir, e como isso vira um estado que a tela consegue dizer.
//!
//! # Por que isto mora no motor
//!
//! Desktop e bolso tinham cada um o seu laco, e nenhum dos dois distinguia
//! "sem rede" de "segredo errado": os dois esperavam o mesmo tempo fixo e
//! tentavam de novo. O resultado era o pior dos dois mundos — a rede que voltou
//! esperava quinze minutos, e a credencial errada batia na mesma parede para
//! sempre. A regra de quanto esperar e uma so, e por isso vive aqui, sem
//! plataforma, com teste.
//!
//! # O que e retry e o que e desistencia
//!
//! `SyncError::retriavel` ja viajava desde o transporte e era descartado na
//! chegada. Agora ele decide: falha retriavel entra no backoff; falha permanente
//! (credencial, contrato, resposta ilegivel) espera a rede de seguranca e
//! aparece no Sync Health como algo que a pessoa precisa resolver.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// O que uma falha de rodada significa, para quem vai decidir o que fazer.
///
/// Derivado da MENSAGEM e do `retriavel`, e nao de um codigo novo no
/// transporte: o transporte fala HTTP, o motor fala "o que aconteceu", e o
/// texto e o unico contrato que os dois ja compartilhavam.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoDeFalha {
    /// Nao alcancou o hub: DNS, conexao recusada, sem rede.
    Offline,
    /// Alcancou e o outro lado nao respondeu a tempo.
    Timeout,
    /// O hub respondeu com erro que ele mesmo chama de passageiro (5xx).
    Hub,
    /// Segredo recusado. Insistir nao resolve.
    Credencial,
    /// Formato incompativel entre as duas pontas. So atualizar resolve.
    Contrato,
    /// Este aparelho nao conseguiu gravar ou ler o proprio banco.
    Local,
    /// Nao deu para dizer mais que "falhou".
    Desconhecida,
}

impl TipoDeFalha {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Offline => "offline",
            Self::Timeout => "timeout",
            Self::Hub => "hub",
            Self::Credencial => "credencial",
            Self::Contrato => "contrato",
            Self::Local => "local",
            Self::Desconhecida => "desconhecida",
        }
    }

    pub fn parse(valor: &str) -> Option<Self> {
        Some(match valor {
            "offline" => Self::Offline,
            "timeout" => Self::Timeout,
            "hub" => Self::Hub,
            "credencial" => Self::Credencial,
            "contrato" => Self::Contrato,
            "local" => Self::Local,
            "desconhecida" => Self::Desconhecida,
            _ => return None,
        })
    }

    /// Se vale a pena tentar de novo sem ninguem mexer em nada.
    pub fn retriavel(self) -> bool {
        matches!(
            self,
            Self::Offline | Self::Timeout | Self::Hub | Self::Local
        )
    }

    /// Se a pessoa precisa fazer algo para o sync voltar.
    pub fn exige_acao(self) -> bool {
        !self.retriavel()
    }
}

/// Classifica uma falha pela mensagem e pelo `retriavel` que o transporte deu.
///
/// A mensagem e olhada por PALAVRAS conhecidas do proprio transporte e do
/// `reqwest`; o que nao casar cai em `Desconhecida` mantendo o `retriavel`
/// como veio — nunca promove uma falha permanente a passageira.
pub fn classificar(mensagem: &str, retriavel: bool) -> TipoDeFalha {
    let m = mensagem.to_ascii_lowercase();
    let contem = |trechos: &[&str]| trechos.iter().any(|t| m.contains(t));

    if contem(&["contrato"]) {
        return TipoDeFalha::Contrato;
    }
    if contem(&[
        "401",
        "403",
        "segredo",
        "credencial",
        "unauthorized",
        "forbidden",
    ]) {
        return TipoDeFalha::Credencial;
    }
    if contem(&["timed out", "timeout", "tempo esgotado", "408"]) {
        return TipoDeFalha::Timeout;
    }
    if contem(&[
        "sem alcancar",
        "dns",
        "connection refused",
        "recusou a conexao",
        "no such host",
        "network",
        "sem rede",
        "unreachable",
        "connect error",
        "error sending request",
    ]) {
        return TipoDeFalha::Offline;
    }
    if contem(&["banco local", "database is locked", "sqlite", "disco"]) {
        return TipoDeFalha::Local;
    }
    if contem(&[
        "o hub respondeu 5",
        "hub respondeu 50",
        "502",
        "503",
        "504",
        "500",
    ]) {
        return TipoDeFalha::Hub;
    }
    if retriavel {
        TipoDeFalha::Desconhecida
    } else if contem(&["ilegivel"]) {
        TipoDeFalha::Contrato
    } else {
        TipoDeFalha::Desconhecida
    }
}

/// A escada do backoff, em segundos.
///
/// Comeca curta porque a falha mais comum e a rede que ainda nao voltou do
/// suspend — dez segundos depois ela costuma estar de pe. Cresce ate quinze
/// minutos, que e a rede de seguranca que ja existia; alem disso e o mesmo que
/// nao tentar.
pub const ESCADA_DO_BACKOFF: [u64; 5] = [10, 30, 120, 300, 900];

/// Quanto esperar depois de `falhas_seguidas` rodadas que falharam.
///
/// Zero falhas nao tem atraso — e a resposta a "acabei de conseguir".
pub fn atraso(falhas_seguidas: u32) -> Duration {
    if falhas_seguidas == 0 {
        return Duration::ZERO;
    }
    let indice = (falhas_seguidas as usize - 1).min(ESCADA_DO_BACKOFF.len() - 1);
    Duration::from_secs(ESCADA_DO_BACKOFF[indice])
}

/// O que ficou registrado da ultima rodada, entre execucoes.
///
/// Persistido: um app que reabre precisa saber que a ultima rodada de ontem
/// falhou por credencial, e nao fingir que nunca tentou. Tudo em RFC3339, texto,
/// porque e assim que o resto do banco fala de tempo.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistroDeSaude {
    /// Quando a ultima rodada terminou INTEIRA.
    pub ultimo_ok_em: Option<String>,
    /// Quando a ultima rodada terminou, bem ou mal.
    pub ultima_rodada_em: Option<String>,
    pub ultimo_erro: Option<String>,
    pub tipo_do_erro: Option<TipoDeFalha>,
    pub falhas_seguidas: u32,
    /// Antes deste instante, o laco nao insiste sozinho. O botao pode.
    pub proxima_tentativa_em: Option<String>,
}

/// O estado que a tela desenha. Seis, e nao cinco — ver `docs/SYNC.md` §10.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EstadoDeSaude {
    /// Sem endereco ou sem segredo. Nao e problema; e feature desligada.
    Desligado,
    /// Uma rodada corre agora.
    Sincronizando { pendentes: usize },
    /// Tudo subiu e desceu; nada espera.
    EmDia,
    /// Ha mudancas locais esperando a proxima rodada, e nada esta errado.
    Pendente { pendentes: usize },
    /// A ultima rodada nao alcancou o hub. Vai tentar de novo sozinho.
    Offline {
        pendentes: usize,
        proxima_tentativa_em: Option<String>,
    },
    /// A ultima rodada falhou por algo que precisa da pessoa.
    Erro {
        pendentes: usize,
        tipo: TipoDeFalha,
        mensagem: String,
    },
}

/// O que a tela precisa para escolher o estado.
pub struct Sinais<'a> {
    pub ligado: bool,
    pub rodando: bool,
    pub pendentes: usize,
    pub registro: &'a RegistroDeSaude,
}

/// Escolhe o estado. A ORDEM das perguntas e o desenho:
///
/// 1. desligado sai primeiro e sai calado;
/// 2. rodando ganha de tudo, porque e o que esta acontecendo AGORA;
/// 3. erro permanente ganha de offline, porque offline se resolve sozinho e o
///    permanente nao;
/// 4. offline ganha de pendente, porque a fila e consequencia;
/// 5. pendente ganha de em dia.
pub fn estado(sinais: Sinais<'_>) -> EstadoDeSaude {
    if !sinais.ligado {
        return EstadoDeSaude::Desligado;
    }
    if sinais.rodando {
        return EstadoDeSaude::Sincronizando {
            pendentes: sinais.pendentes,
        };
    }
    if let (Some(tipo), Some(mensagem)) = (
        sinais.registro.tipo_do_erro,
        sinais.registro.ultimo_erro.as_ref(),
    ) {
        // Um erro so vale enquanto for o ULTIMO acontecimento: uma rodada boa
        // depois dele limpa o registro (ver `RegistroDeSaude::sucesso`).
        if tipo.exige_acao() {
            return EstadoDeSaude::Erro {
                pendentes: sinais.pendentes,
                tipo,
                mensagem: mensagem.clone(),
            };
        }
        return EstadoDeSaude::Offline {
            pendentes: sinais.pendentes,
            proxima_tentativa_em: sinais.registro.proxima_tentativa_em.clone(),
        };
    }
    if sinais.pendentes > 0 {
        return EstadoDeSaude::Pendente {
            pendentes: sinais.pendentes,
        };
    }
    EstadoDeSaude::EmDia
}

impl RegistroDeSaude {
    /// Uma rodada terminou inteira. Zera a escada.
    pub fn sucesso(&mut self, agora: &str) {
        self.ultimo_ok_em = Some(agora.to_owned());
        self.ultima_rodada_em = Some(agora.to_owned());
        self.ultimo_erro = None;
        self.tipo_do_erro = None;
        self.falhas_seguidas = 0;
        self.proxima_tentativa_em = None;
    }

    /// Uma rodada parou. Sobe um degrau e marca quando insistir.
    ///
    /// `proxima_em` e calculado por quem chama a partir de `atraso`, porque
    /// somar duracao a texto RFC3339 e trabalho do adaptador que tem o relogio.
    pub fn falha(
        &mut self,
        agora: &str,
        mensagem: &str,
        retriavel: bool,
        proxima_em: impl FnOnce(Duration) -> String,
    ) -> TipoDeFalha {
        let tipo = classificar(mensagem, retriavel);
        self.ultima_rodada_em = Some(agora.to_owned());
        self.ultimo_erro = Some(mensagem.to_owned());
        self.tipo_do_erro = Some(tipo);
        self.falhas_seguidas = self.falhas_seguidas.saturating_add(1);
        // Falha permanente nao entra na escada: ela espera a rede de
        // seguranca, e o que muda a situacao e a pessoa, nao o tempo.
        let espera = if tipo.retriavel() {
            atraso(self.falhas_seguidas)
        } else {
            Duration::from_secs(*ESCADA_DO_BACKOFF.last().unwrap_or(&900))
        };
        self.proxima_tentativa_em = Some(proxima_em(espera));
        tipo
    }

    /// Quanto o laco deve esperar antes da proxima rodada automatica.
    pub fn espera(&self) -> Duration {
        match self.tipo_do_erro {
            None => Duration::ZERO,
            Some(tipo) if tipo.retriavel() => atraso(self.falhas_seguidas),
            Some(_) => Duration::from_secs(*ESCADA_DO_BACKOFF.last().unwrap_or(&900)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registro_com_erro(mensagem: &str, retriavel: bool) -> RegistroDeSaude {
        let mut r = RegistroDeSaude::default();
        r.falha("2026-09-15T10:00:00Z", mensagem, retriavel, |_| {
            "depois".into()
        });
        r
    }

    #[test]
    fn classifica_pelos_textos_do_transporte() {
        assert_eq!(
            classificar("Sem alcancar o hub: error sending request", true),
            TipoDeFalha::Offline
        );
        assert_eq!(
            classificar("Sem alcancar o hub: operation timed out", true),
            TipoDeFalha::Timeout
        );
        assert_eq!(classificar("O hub respondeu 503.", true), TipoDeFalha::Hub);
        assert_eq!(
            classificar("O hub respondeu 401.", false),
            TipoDeFalha::Credencial
        );
        assert_eq!(
            classificar("O outro lado fala o contrato 2, e este M/OS fala 1.", false),
            TipoDeFalha::Contrato
        );
        assert_eq!(
            classificar("O banco local recusou a operacao: database is locked", true),
            TipoDeFalha::Local
        );
        assert_eq!(
            classificar("Resposta do hub ilegivel: EOF", false),
            TipoDeFalha::Contrato
        );
    }

    #[test]
    fn a_escada_sobe_e_para_no_teto() {
        assert_eq!(atraso(0), Duration::ZERO);
        assert_eq!(atraso(1), Duration::from_secs(10));
        assert_eq!(atraso(2), Duration::from_secs(30));
        assert_eq!(atraso(3), Duration::from_secs(120));
        assert_eq!(atraso(5), Duration::from_secs(900));
        assert_eq!(atraso(50), Duration::from_secs(900));
    }

    #[test]
    fn sucesso_zera_a_escada() {
        let mut r = registro_com_erro("Sem alcancar o hub", true);
        r.falha("2026-09-15T10:00:10Z", "Sem alcancar o hub", true, |_| {
            "x".into()
        });
        assert_eq!(r.falhas_seguidas, 2);
        r.sucesso("2026-09-15T10:01:00Z");
        assert_eq!(r.falhas_seguidas, 0);
        assert!(r.ultimo_erro.is_none());
        assert_eq!(r.espera(), Duration::ZERO);
    }

    #[test]
    fn falha_permanente_nao_entra_na_escada() {
        let r = registro_com_erro("O hub respondeu 401.", false);
        assert_eq!(r.espera(), Duration::from_secs(900));
        let e = estado(Sinais {
            ligado: true,
            rodando: false,
            pendentes: 3,
            registro: &r,
        });
        assert!(matches!(
            e,
            EstadoDeSaude::Erro {
                tipo: TipoDeFalha::Credencial,
                pendentes: 3,
                ..
            }
        ));
    }

    #[test]
    fn offline_e_estado_proprio_e_nao_erro() {
        let r = registro_com_erro("Sem alcancar o hub: connection refused", true);
        let e = estado(Sinais {
            ligado: true,
            rodando: false,
            pendentes: 2,
            registro: &r,
        });
        assert!(matches!(e, EstadoDeSaude::Offline { pendentes: 2, .. }));
    }

    #[test]
    fn a_ordem_das_perguntas() {
        let limpo = RegistroDeSaude::default();
        assert_eq!(
            estado(Sinais {
                ligado: false,
                rodando: true,
                pendentes: 9,
                registro: &limpo
            }),
            EstadoDeSaude::Desligado
        );
        assert_eq!(
            estado(Sinais {
                ligado: true,
                rodando: true,
                pendentes: 9,
                registro: &limpo
            }),
            EstadoDeSaude::Sincronizando { pendentes: 9 }
        );
        assert_eq!(
            estado(Sinais {
                ligado: true,
                rodando: false,
                pendentes: 4,
                registro: &limpo
            }),
            EstadoDeSaude::Pendente { pendentes: 4 }
        );
        assert_eq!(
            estado(Sinais {
                ligado: true,
                rodando: false,
                pendentes: 0,
                registro: &limpo
            }),
            EstadoDeSaude::EmDia
        );
    }

    #[test]
    fn tipo_de_falha_vai_e_volta_em_texto() {
        for tipo in [
            TipoDeFalha::Offline,
            TipoDeFalha::Timeout,
            TipoDeFalha::Hub,
            TipoDeFalha::Credencial,
            TipoDeFalha::Contrato,
            TipoDeFalha::Local,
            TipoDeFalha::Desconhecida,
        ] {
            assert_eq!(TipoDeFalha::parse(tipo.as_str()), Some(tipo));
        }
    }
}
