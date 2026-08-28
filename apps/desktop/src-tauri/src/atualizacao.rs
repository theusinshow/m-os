//! O que a tela precisa para responder "estou atualizado?".
//!
//! # Por que este modulo existe
//!
//! O painel de atualizacoes sabia responder a pergunta **so durante os segundos
//! seguintes ao clique**. Verificar, sair de Settings e voltar apagava tudo: o
//! estado morava em `useState`, e desmontar o componente levava junto a unica
//! prova de que a verificacao tinha acontecido. Na pratica isso queria dizer que
//! o M/OS nunca sabia dizer se estava em dia — so sabia dizer se voce tinha
//! clicado ha pouco.
//!
//! Pior: **"nao havia versao nova" e "nao consegui verificar" tinham a mesma
//! cara** — nenhuma. Uma verificacao que falhou por falta de rede deixava a tela
//! igualzinha a uma que deu certo, e e exatamente dai que sai o "as vezes ele
//! nao funciona": o app parecia ter conferido quando nao tinha.
//!
//! Entao o resultado da verificacao passa a ser um FATO GRAVADO, e nao estado de
//! tela. Ele mora no `settings.json`, ao lado das outras preferencias deste
//! aparelho, porque e disso que se trata: uma anotacao local sobre esta
//! instalacao. Ele nao sincroniza — a versao instalada e um fato do aparelho, e
//! mandar isso para o hub faria o celular achar que tambem esta desatualizado.
//!
//! # O dia da versao que esta rodando
//!
//! Vem do carimbo do proprio executavel no disco. Isso responde **de quando e a
//! versao que esta rodando**, e nao "desde quando este computador esta nela": o
//! NSIS PRESERVA o carimbo dos arquivos que empacota, entao o `mos-desktop.exe`
//! mantem a hora em que o CI o compilou, e nao a hora em que o instalador
//! passou por aqui. Conferido em 28/08/2026: o executavel marcava 25/08 23:30,
//! que e o build do release; o `uninstall.exe`, que o NSIS gera na hora, marcava
//! 26/08 09:42, que e a instalacao.
//!
//! A pergunta que a tela faz e a primeira — "de que dia e a atualizacao que ele
//! esta executando" —, entao o carimbo serve. O nome do campo diz isso, e nao
//! "instalada em": um rotulo que promete a data da instalacao e entrega a da
//! compilacao e uma mentira pequena que ninguem tem como perceber.

use std::path::Path;

use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::{load_settings, save_settings, AppState};

/// O que a tela desenha embaixo do botao.
///
/// Tudo aqui e fato, e nenhuma frase: quem escreve a frase e a interface, que e
/// quem conhece o idioma e o espaco disponivel. Um campo `mensagem` pronto aqui
/// obrigaria o Rust a decidir o texto de uma tela que ele nao ve.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EstadoDaAtualizacao {
    /// A versao que esta rodando agora.
    pub versao: String,
    /// De quando E esta versao, em RFC 3339: o carimbo do executavel, que o
    /// instalador preserva do build. Vazio quando ele nao pode ser lido — que
    /// acontece, e nao e erro. Ver o cabecalho para por que nao e "instalada em".
    pub versao_de: String,
    /// Quando a ultima verificacao BEM-SUCEDIDA aconteceu. Vazio significa
    /// nunca, e "nunca" e uma resposta que a tela precisa poder dar.
    pub verificada_em: String,
    /// A versao nova que a ultima verificacao encontrou. Vazio significa que ela
    /// encontrou o que se espera encontrar: nada.
    pub disponivel: String,
    /// Quando a versao disponivel foi publicada, em RFC 3339.
    pub publicada_em: String,
    /// O motivo da ultima verificacao que FALHOU, quando a mais recente falhou.
    ///
    /// Separado de `verificada_em` de proposito: as duas coexistem, e a tela
    /// precisa das duas para dizer "voce estava em dia ontem, e hoje eu nao
    /// consegui conferir" em vez de escolher uma das metades.
    pub falha: String,
    pub falha_em: String,
    /// De onde a verificacao busca. Aparece na tela porque, quando ela falha, a
    /// primeira pergunta util e "falha ao falar com quem?".
    pub endpoint: String,
}

/// De onde o `tauri.conf.json` manda buscar.
///
/// Repetido aqui como texto, e a repeticao e deliberada: ler o JSON de
/// configuracao em tempo de execucao so para mostrar uma linha na tela custaria
/// um caminho de arquivo e um parse que podem falhar, para exibir algo que nao
/// muda entre versoes. Se o endpoint mudar, esta linha muda junto — e o teste
/// que compara os dois nao existe porque o custo do erro e uma URL errada num
/// texto de diagnostico, e nao uma atualizacao que nao chega.
const ENDPOINT: &str = "https://github.com/theusinshow/m-os/releases/latest/download/latest.json";

/// Quando o executavel que esta rodando foi escrito no disco.
///
/// Na pratica, a hora em que o CI o compilou: o NSIS preserva o carimbo do que
/// empacota. Ver o cabecalho.
///
/// Falha em silencio — devolvendo vazio — porque nao saber a data e uma
/// informacao a menos, e nao um motivo para o painel inteiro nao abrir.
fn carimbo_do_executavel() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|caminho| carimbo_de(&caminho))
        .unwrap_or_default()
}

fn carimbo_de(caminho: &Path) -> Option<String> {
    let modificado = std::fs::metadata(caminho).ok()?.modified().ok()?;
    OffsetDateTime::from(modificado).format(&Rfc3339).ok()
}

#[tauri::command]
pub fn atualizacao_estado(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> EstadoDaAtualizacao {
    let settings = load_settings(&state.settings_path);
    EstadoDaAtualizacao {
        versao: app.package_info().version.to_string(),
        versao_de: carimbo_do_executavel(),
        verificada_em: settings.atualizacao_verificada_em.clone(),
        disponivel: settings.atualizacao_disponivel.clone(),
        publicada_em: settings.atualizacao_publicada_em.clone(),
        falha: settings.atualizacao_falha.clone(),
        falha_em: settings.atualizacao_falha_em.clone(),
        endpoint: ENDPOINT.to_owned(),
    }
}

/// Anota uma verificacao que DEU CERTO.
///
/// `disponivel` vazio significa "conferi e nao ha versao nova" — que e o
/// resultado comum, e o unico que a tela antiga nao conseguia guardar.
///
/// Apagar a falha aqui e o ponto: uma verificacao bem-sucedida torna a queixa
/// anterior historia, e deixar as duas na tela faria o painel contar duas
/// versoes da mesma coisa.
#[tauri::command]
pub fn atualizacao_anotar_verificacao(
    state: tauri::State<'_, AppState>,
    disponivel: String,
    publicada_em: String,
) -> Result<(), mos_core::CoreError> {
    let mut settings = load_settings(&state.settings_path);
    settings.atualizacao_verificada_em = agora();
    settings.atualizacao_disponivel = disponivel;
    settings.atualizacao_publicada_em = publicada_em;
    settings.atualizacao_falha.clear();
    settings.atualizacao_falha_em.clear();
    save_settings(&state.settings_path, &settings)
}

/// Anota uma verificacao que FALHOU.
///
/// Ela NAO apaga `verificada_em` nem `disponivel`: o que se soube ontem continua
/// sendo o que se sabe. Apagar transformaria uma queda de rede em "voce nunca
/// verificou", que e mentira e e pior — some a unica informacao que ainda valia.
#[tauri::command]
pub fn atualizacao_anotar_falha(
    state: tauri::State<'_, AppState>,
    motivo: String,
) -> Result<(), mos_core::CoreError> {
    let mut settings = load_settings(&state.settings_path);
    settings.atualizacao_falha = motivo;
    settings.atualizacao_falha_em = agora();
    save_settings(&state.settings_path, &settings)
}

fn agora() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_carimbo_de_um_arquivo_que_existe_e_legivel_como_rfc3339() {
        let pasta = tempfile::tempdir().unwrap();
        let arquivo = pasta.path().join("mos-desktop.exe");
        std::fs::write(&arquivo, b"binario de mentira").unwrap();

        let carimbo = carimbo_de(&arquivo).expect("um arquivo recem-escrito tem carimbo");
        // A tela formata isto com `new Date(...)`, entao ele precisa ser um
        // instante que o JavaScript entenda — nao basta ser uma string.
        OffsetDateTime::parse(&carimbo, &Rfc3339).expect("o carimbo precisa ser RFC 3339");
    }

    #[test]
    fn o_carimbo_de_um_arquivo_ausente_e_vazio_e_nao_erro() {
        // Nao saber a data e uma informacao a menos, e nao motivo para o painel
        // inteiro nao abrir.
        assert!(carimbo_de(Path::new("nao/existe/mos-desktop.exe")).is_none());
    }
}
