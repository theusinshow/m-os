//! A cota de verdade, perguntada ao servidor da Anthropic.
//!
//! # Por que este módulo existe
//!
//! O [`crate`] inteiro foi desenhado em cima de um fato: o transcript não traz
//! teto de cota nem hora de reset, e por isso a régua do anel era o próprio pico
//! observado nesta máquina. Era a régua honesta possível — e deixou de ser a
//! única.
//!
//! O Claude Code guarda um token OAuth em `~/.claude/.credentials.json`, e com
//! ele o servidor responde quanto da janela de cinco horas e quanto da semana já
//! foram, com a hora exata em que cada uma reseta. É o gatilho escrito no
//! "Revisar quando" da ADR-059, e o que ele dispara está na ADR-062.
//!
//! # Este módulo NÃO fala com rede
//!
//! A promessa do crate continua de pé. O que mora aqui é o **protocolo** — o
//! endereço, os cabeçalhos, o formato da resposta — e a leitura da credencial do
//! disco. Quem faz o pedido é o app, que já tem `reqwest` por causa do sync.
//!
//! A divisão não é cerimônia: ela deixa [`ler_resposta`] testável contra um
//! corpo gravado, que é o único jeito de o teste apontar para cá quando a
//! Anthropic mudar o formato.
//!
//! # A fronteira de espectador
//!
//! Este módulo **lê** a credencial e nunca a escreve. Não renova token, não
//! reescreve o arquivo, não toca em nada que o Claude Code considere seu. Com o
//! token vencido a resposta é [`None`] — e a faixa volta para a régua do pico,
//! que continua ali para exatamente esta hora.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use time::{Duration, OffsetDateTime};

/// O endereço que o próprio CLI consulta.
///
/// Não é uma API publicada, e é por isso que ela está isolada aqui: quando
/// sumir, o teste que quebra aponta para um arquivo, e a faixa cai na régua do
/// pico sem que nada mais precise saber.
pub const ENDERECO: &str = "https://api.anthropic.com/api/oauth/usage";

/// Os cabeçalhos que o servidor exige. Sem eles a resposta é 401 ou 403.
pub const CABECALHOS: [(&str, &str); 5] = [
    ("Accept", "application/json"),
    ("anthropic-beta", "oauth-2025-04-20"),
    ("anthropic-version", "2023-06-01"),
    ("x-app", "cli"),
    ("User-Agent", "claude-cli/2.1.246 (external, cli)"),
];

/// O token do Claude Code, lido do disco.
#[derive(Debug, Clone)]
pub struct Credencial {
    pub token: String,
    pub expira_em: OffsetDateTime,
}

impl Credencial {
    pub fn vencida(&self, agora: OffsetDateTime) -> bool {
        agora >= self.expira_em
    }
}

#[derive(Deserialize)]
struct ArquivoDeCredencial {
    #[serde(rename = "claudeAiOauth")]
    oauth: Option<OauthGravado>,
}

#[derive(Deserialize)]
struct OauthGravado {
    #[serde(rename = "accessToken")]
    token: String,
    /// Milissegundos desde a época. É o que o Claude Code grava.
    #[serde(rename = "expiresAt")]
    expira_em: i64,
}

/// Onde o Claude Code guarda a credencial nesta máquina.
pub fn caminho_da_credencial() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".claude").join(".credentials.json"))
}

/// Lê a credencial, se ela existir e for legível.
///
/// Todo caminho de falha devolve [`None`], e nenhum deles é erro: máquina sem
/// Claude Code, arquivo de formato novo, sessão deslogada. A ausência de cota é
/// um estado previsto, e quem o trata é a régua do pico.
pub fn credencial_em(caminho: &Path) -> Option<Credencial> {
    let bruto = std::fs::read_to_string(caminho).ok()?;
    let arquivo: ArquivoDeCredencial = serde_json::from_str(&bruto).ok()?;
    let oauth = arquivo.oauth?;
    if oauth.token.is_empty() {
        return None;
    }
    let expira_em =
        OffsetDateTime::from_unix_timestamp_nanos(oauth.expira_em as i128 * 1_000_000).ok()?;
    Some(Credencial {
        token: oauth.token,
        expira_em,
    })
}

pub fn credencial() -> Option<Credencial> {
    credencial_em(&caminho_da_credencial()?)
}

/// Uma janela de cota, com denominador de verdade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limite {
    /// Quanto da janela já foi. Vem inteiro do servidor.
    ///
    /// Não é limitado a 100: se o servidor disser 103, a faixa mostra 103. Um
    /// teto aplicado aqui esconderia justamente a hora em que a informação mais
    /// importa.
    pub percentual: u16,
    /// Quando esta janela zera. `None` quando o servidor não sabe — e aí a
    /// faixa não inventa prazo, pela mesma regra de sempre.
    pub reseta_em: Option<OffsetDateTime>,
}

/// As duas janelas que o servidor conhece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cota {
    /// A janela de cinco horas.
    pub sessao: Option<Limite>,
    /// A janela de sete dias. É a que morde na quinta-feira, e a que o
    /// transcript nunca teve como mostrar.
    pub semana: Option<Limite>,
}

impl Cota {
    /// Uma cota sem nenhuma das duas janelas não é cota. Devolvê-la seria
    /// substituir a régua do pico por nada.
    pub fn vazia(&self) -> bool {
        self.sessao.is_none() && self.semana.is_none()
    }
}

#[derive(Deserialize)]
struct Resposta {
    #[serde(default)]
    limits: Vec<LimiteGravado>,
    #[serde(default)]
    five_hour: Option<JanelaGravada>,
    #[serde(default)]
    seven_day: Option<JanelaGravada>,
}

#[derive(Deserialize)]
struct LimiteGravado {
    kind: String,
    percent: Option<f64>,
    resets_at: Option<String>,
}

#[derive(Deserialize)]
struct JanelaGravada {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

fn instante(bruto: &Option<String>) -> Option<OffsetDateTime> {
    let texto = bruto.as_ref()?;
    OffsetDateTime::parse(texto, &time::format_description::well_known::Rfc3339).ok()
}

fn percentual(valor: Option<f64>) -> Option<u16> {
    let valor = valor?;
    if !valor.is_finite() || valor < 0.0 {
        return None;
    }
    Some(valor.round().min(u16::MAX as f64) as u16)
}

/// Lê o corpo da resposta.
///
/// A lista `limits` vem primeiro porque é a que o servidor mantém: ela NOMEIA a
/// janela (`session`, `weekly_all`) em vez de depender da posição de um campo.
/// `five_hour` e `seven_day` ficam como reserva — hoje trazem o mesmo número, e
/// não custa nada continuar aceitando os dois.
///
/// Devolve [`None`] quando não dá para tirar NENHUMA das duas janelas. Meia
/// resposta é resposta: sessão sem semana é comum, e é válida.
pub fn ler_resposta(corpo: &str) -> Option<Cota> {
    let resposta: Resposta = serde_json::from_str(corpo).ok()?;

    let mut cota = Cota::default();
    for limite in &resposta.limits {
        let alvo = match limite.kind.as_str() {
            "session" => &mut cota.sessao,
            // `weekly_scoped` fica de fora de propósito: ele é o limite de UM
            // modelo, e mostrá-lo como "a semana" diria menos do que o nome
            // promete.
            "weekly_all" => &mut cota.semana,
            _ => continue,
        };
        if let Some(percentual) = percentual(limite.percent) {
            *alvo = Some(Limite {
                percentual,
                reseta_em: instante(&limite.resets_at),
            });
        }
    }

    if cota.sessao.is_none() {
        if let Some(janela) = &resposta.five_hour {
            if let Some(percentual) = percentual(janela.utilization) {
                cota.sessao = Some(Limite {
                    percentual,
                    reseta_em: instante(&janela.resets_at),
                });
            }
        }
    }
    if cota.semana.is_none() {
        if let Some(janela) = &resposta.seven_day {
            if let Some(percentual) = percentual(janela.utilization) {
                cota.semana = Some(Limite {
                    percentual,
                    reseta_em: instante(&janela.resets_at),
                });
            }
        }
    }

    (!cota.vazia()).then_some(cota)
}

/// O que um provedor QUALQUER precisa imprimir para virar um anel.
///
/// # O que isto desfaz
///
/// A ADR-059 deixou OpenAI e os outros de fora com um argumento que continua
/// certo: *"o mockup mostra três anéis, e três anéis com dois deles inventados
/// seria o erro que esta ADR recusa, em triplicado"*. O motivo da recusa era a
/// **invenção**, não a quantidade — e a saída de um comando que o dono escolheu
/// não é invenção nossa.
///
/// # O contrato
///
/// Um comando que imprime, no stdout:
///
/// ```json
/// { "sessionUsedPercent": 25, "weeklyUsedPercent": 60 }
/// ```
///
/// Os nomes vêm do `agent-notch`, que já os usa para a mesma coisa: um
/// contrato que alguém já escreveu vale mais que um contrato melhor que só o
/// M/OS entende. `sessionResetsAt` e `weeklyResetsAt` em RFC 3339 são opcionais,
/// e sem eles a barra mostra o valor sem prazo — que é o que a faixa já faz
/// quando não sabe.
///
/// Qualquer campo que falte ou não seja número vira ausência, nunca zero: um
/// anel em 0% diz "não consumi nada", e é uma frase diferente de "não sei".
pub fn ler_fonte_externa(stdout: &str) -> Option<Cota> {
    #[derive(Deserialize)]
    struct Externa {
        #[serde(rename = "sessionUsedPercent")]
        sessao: Option<f64>,
        #[serde(rename = "weeklyUsedPercent")]
        semana: Option<f64>,
        #[serde(rename = "sessionResetsAt")]
        sessao_reseta: Option<String>,
        #[serde(rename = "weeklyResetsAt")]
        semana_reseta: Option<String>,
    }

    // Um comando costuma imprimir mais que o JSON — banner, aviso, linha em
    // branco. O objeto e o que esta entre a primeira chave e a ultima; exigir o
    // stdout inteiro limpo faria o contrato falhar por causa de uma quebra de
    // linha a mais.
    let inicio = stdout.find('{')?;
    let fim = stdout.rfind('}')?;
    let externa: Externa = serde_json::from_str(stdout.get(inicio..=fim)?).ok()?;

    let cota = Cota {
        sessao: percentual(externa.sessao).map(|percentual| Limite {
            percentual,
            reseta_em: instante(&externa.sessao_reseta),
        }),
        semana: percentual(externa.semana).map(|percentual| Limite {
            percentual,
            reseta_em: instante(&externa.semana_reseta),
        }),
    };
    (!cota.vazia()).then_some(cota)
}

/// A variável que liga o modo de demonstração.
pub const VAR_DEMO: &str = "MOS_FAIXA_DEMO";

/// Uma cota inventada, para VER a faixa sem esperar o consumo chegar lá.
///
/// # Por que isto existe
///
/// Para conferir o anel em 95% era preciso chegar a 95%. Conferir a faixa
/// custava horas de espera ou nada — e "nada" foi o que aconteceu por padrão.
///
/// # E por que ele grita
///
/// Um modo de demonstração que se pareça com o real é exatamente o número
/// inventado que o `Ring.tsx` proíbe por escrito. Por isso a [`Cota`] que sai
/// daqui viaja com um aviso até a tela, e o painel diz que aquilo não é
/// consumo de verdade. Um dado falso indistinguível do verdadeiro é pior que
/// nenhum dado.
///
/// `MOS_FAIXA_DEMO=95,60` — sessão e semana, em por cento. A semana é
/// opcional: `MOS_FAIXA_DEMO=95` deixa a faixa com uma janela só, que é o
/// estado de quem não tem plano semanal.
pub fn demo_de(bruto: &str, agora: OffsetDateTime) -> Option<Cota> {
    let mut partes = bruto.split(',').map(str::trim);
    let sessao = partes.next()?.parse::<u16>().ok()?;
    let semana = partes.next().and_then(|valor| valor.parse::<u16>().ok());
    Some(Cota {
        sessao: Some(Limite {
            percentual: sessao,
            // Relativo ao agora, e não a um instante fixo. A primeira versão
            // gravava um carimbo constante — bom para a bancada, que assim
            // captura sempre o mesmo texto, e a foto mostrou o preço: passada a
            // data, o painel dizia "reseta agora" para sempre.
            reseta_em: Some(agora + Duration::hours(2) + Duration::minutes(26)),
        }),
        semana: semana.map(|percentual| Limite {
            percentual,
            reseta_em: Some(agora + Duration::days(5) + Duration::hours(23)),
        }),
    })
}

pub fn demo() -> Option<Cota> {
    demo_de(&std::env::var(VAR_DEMO).ok()?, OffsetDateTime::now_utc())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    /// A resposta real desta máquina, aparada no que a faixa usa. Os campos que
    /// sobraram no servidor — `spend`, `extra_usage`, as janelas de nome de
    /// fantasia — continuam chegando e continuam sendo ignorados: um `Deserialize`
    /// que exigisse conhecê-los quebraria a cada nome novo que a Anthropic
    /// inventasse.
    const RESPOSTA: &str = r#"{
      "five_hour": { "utilization": 23.0, "resets_at": "2026-08-31T03:50:00.018221+00:00" },
      "seven_day": { "utilization": 3.0, "resets_at": "2026-09-06T17:00:00.018249+00:00" },
      "nimbus_quill": { "utilization": 0.0, "resets_at": null },
      "extra_usage": { "is_enabled": false, "monthly_limit": 10000 },
      "limits": [
        { "kind": "session", "group": "session", "percent": 23, "severity": "normal",
          "resets_at": "2026-08-31T03:50:00.018221+00:00", "is_active": true },
        { "kind": "weekly_all", "group": "weekly", "percent": 3, "severity": "normal",
          "resets_at": "2026-09-06T17:00:00.018249+00:00", "is_active": false },
        { "kind": "weekly_scoped", "group": "weekly", "percent": 97, "severity": "normal",
          "resets_at": null, "is_active": false }
      ]
    }"#;

    #[test]
    fn a_resposta_da_maquina_vira_as_duas_janelas() {
        let cota = ler_resposta(RESPOSTA).expect("a resposta tem as duas janelas");
        let sessao = cota.sessao.expect("sessão");
        assert_eq!(sessao.percentual, 23);
        assert_eq!(sessao.reseta_em, Some(datetime!(2026-08-31 03:50:00.018221 UTC)));
        let semana = cota.semana.expect("semana");
        assert_eq!(semana.percentual, 3);
        assert_eq!(semana.reseta_em, Some(datetime!(2026-09-06 17:00:00.018249 UTC)));
    }

    /// O `weekly_scoped` de 97% não pode virar "a semana". Ele é de um modelo só,
    /// e trocá-lo pelo total transformaria 3% em 97% na tela.
    #[test]
    fn o_limite_de_um_modelo_nao_e_a_semana() {
        let cota = ler_resposta(RESPOSTA).unwrap();
        assert_eq!(cota.semana.unwrap().percentual, 3);
    }

    /// Sem a lista, as janelas soltas respondem.
    #[test]
    fn as_janelas_soltas_sao_a_reserva() {
        let corpo = r#"{
          "five_hour": { "utilization": 61.4, "resets_at": "2026-08-31T03:50:00Z" },
          "seven_day": { "utilization": 12.0, "resets_at": null }
        }"#;
        let cota = ler_resposta(corpo).unwrap();
        // 61,4 arredonda para 61: o anel não tem casa decimal para gastar.
        assert_eq!(cota.sessao.unwrap().percentual, 61);
        assert_eq!(cota.semana.unwrap().reseta_em, None);
    }

    /// Meia resposta é resposta.
    #[test]
    fn sessao_sem_semana_continua_valendo() {
        let corpo = r#"{ "limits": [
          { "kind": "session", "percent": 40, "resets_at": null } ] }"#;
        let cota = ler_resposta(corpo).unwrap();
        assert_eq!(cota.sessao.unwrap().percentual, 40);
        assert!(cota.semana.is_none());
    }

    /// Nenhuma janela é o mesmo que não ter perguntado — e aí a régua do pico
    /// tem de continuar valendo, em vez de ser substituída por um anel vazio.
    #[test]
    fn resposta_sem_janela_nenhuma_e_none() {
        assert!(ler_resposta(r#"{"limits": [], "spend": {"percent": 12}}"#).is_none());
        assert!(ler_resposta("nao e json").is_none());
        assert!(ler_resposta(r#"{"limits":[{"kind":"weekly_scoped","percent":9}]}"#).is_none());
    }

    /// Acima de 100 passa. É a hora em que o número mais importa.
    #[test]
    fn o_percentual_nao_tem_teto() {
        let corpo = r#"{ "limits": [ { "kind": "session", "percent": 103 } ] }"#;
        assert_eq!(ler_resposta(corpo).unwrap().sessao.unwrap().percentual, 103);
    }

    const AGORA: OffsetDateTime = datetime!(2026-08-31 12:00:00 UTC);

    #[test]
    fn a_demonstracao_aceita_uma_ou_duas_janelas() {
        let duas = demo_de("95,60", AGORA).unwrap();
        assert_eq!(duas.sessao.unwrap().percentual, 95);
        assert_eq!(duas.semana.unwrap().percentual, 60);

        // Uma janela só: o estado de quem não tem plano semanal.
        let uma = demo_de("40", AGORA).unwrap();
        assert_eq!(uma.sessao.unwrap().percentual, 40);
        assert!(uma.semana.is_none());

        assert_eq!(demo_de(" 7 , 8 ", AGORA).unwrap().sessao.unwrap().percentual, 7);
    }

    /// O prazo da demonstração é sempre no FUTURO. A primeira versão usava um
    /// instante fixo, e a foto do painel mostrou "reseta agora" — o defeito que
    /// este teste existe para não deixar voltar.
    #[test]
    fn o_prazo_da_demonstracao_nunca_ja_passou() {
        let cota = demo_de("95,72", AGORA).unwrap();
        assert!(cota.sessao.unwrap().reseta_em.unwrap() > AGORA);
        assert!(cota.semana.unwrap().reseta_em.unwrap() > AGORA + Duration::days(5));
    }

    #[test]
    fn demonstracao_ilegivel_nao_vira_zero() {
        // Zero seria um número, e um número errado é pior que a ausência.
        assert!(demo_de("", AGORA).is_none());
        assert!(demo_de("muito", AGORA).is_none());
        assert!(demo_de("-5", AGORA).is_none());
    }

    #[test]
    fn o_contrato_externo_e_o_do_agent_notch() {
        let cota = ler_fonte_externa(r#"{"sessionUsedPercent": 25, "weeklyUsedPercent": 60}"#)
            .unwrap();
        assert_eq!(cota.sessao.unwrap().percentual, 25);
        assert_eq!(cota.semana.unwrap().percentual, 60);
        assert_eq!(cota.sessao.unwrap().reseta_em, None);
    }

    /// Um comando raramente imprime só o JSON.
    #[test]
    fn o_json_e_achado_no_meio_do_barulho() {
        let saida = "codex v1.2 (build 9)
lendo credenciais...
                     {\"sessionUsedPercent\": 8}
feito.
";
        assert_eq!(ler_fonte_externa(saida).unwrap().sessao.unwrap().percentual, 8);
    }

    #[test]
    fn o_prazo_externo_e_opcional_e_lido_quando_vem() {
        let cota = ler_fonte_externa(
            r#"{"sessionUsedPercent": 5, "sessionResetsAt": "2026-08-31T03:50:00Z"}"#,
        )
        .unwrap();
        assert_eq!(
            cota.sessao.unwrap().reseta_em,
            Some(datetime!(2026-08-31 03:50:00 UTC))
        );
        assert!(cota.semana.is_none());
    }

    /// Campo que falta é ausência, e nunca zero. "Não sei" e "não consumi nada"
    /// são frases diferentes, e um anel vazio diz a segunda.
    #[test]
    fn campo_ausente_ou_torto_nao_vira_zero() {
        assert!(ler_fonte_externa(r#"{"sessionUsedPercent": "muito"}"#).is_none());
        assert!(ler_fonte_externa(r#"{"outraCoisa": 5}"#).is_none());
        assert!(ler_fonte_externa("erro: nao autenticado").is_none());
        assert!(ler_fonte_externa("").is_none());
        assert!(ler_fonte_externa(r#"{"sessionUsedPercent": null}"#).is_none());
    }

    #[test]
    fn a_credencial_sai_do_arquivo_do_claude_code() {
        let pasta = tempfile::tempdir().unwrap();
        let caminho = pasta.path().join(".credentials.json");
        // 2026-08-31T02:00:00Z em milissegundos.
        std::fs::write(
            &caminho,
            r#"{"claudeAiOauth":{"accessToken":"tok","expiresAt":1788141600000,
               "refreshToken":"r","scopes":[]}}"#,
        )
        .unwrap();
        let credencial = credencial_em(&caminho).expect("le a credencial");
        assert_eq!(credencial.token, "tok");
        assert!(credencial.vencida(datetime!(2026-08-31 03:00:00 UTC)));
        assert!(!credencial.vencida(datetime!(2026-08-31 01:00:00 UTC)));
    }

    #[test]
    fn arquivo_ausente_ou_de_outro_formato_e_none() {
        let pasta = tempfile::tempdir().unwrap();
        assert!(credencial_em(&pasta.path().join("nao-existe.json")).is_none());
        let vazio = pasta.path().join("vazio.json");
        std::fs::write(&vazio, "{}").unwrap();
        assert!(credencial_em(&vazio).is_none());
        let sem_token = pasta.path().join("sem.json");
        std::fs::write(&sem_token, r#"{"claudeAiOauth":{"accessToken":"","expiresAt":1}}"#).unwrap();
        assert!(credencial_em(&sem_token).is_none());
    }
}
