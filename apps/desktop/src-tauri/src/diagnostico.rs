//! O caderno de ocorrencias do M/OS.
//!
//! # Por que existe
//!
//! Ate 2026-08-25 o M/OS nao deixava rastro de nada. Quando ele fechava
//! sozinho no meio do uso, o que sobrava era a lembranca de quem estava na
//! frente: nenhuma mensagem, nenhum arquivo, nenhum jeito de saber se foi
//! panico do Rust, aborto do processo ou a webview morrendo por fora. Duas
//! ocorrencias relatadas — "abre e fecha sozinho" e "a janelinha do canto
//! aparece com erro 404" — nao tinham COMO ser investigadas: as duas acontecem
//! no logon, longe de qualquer terminal aberto.
//!
//! Este modulo nao conserta nenhuma das duas. Ele faz a coisa que precisava
//! existir ANTES do conserto: transformar "aconteceu de novo" em uma linha de
//! texto com hora, origem e causa.
//!
//! # O que ele grava, e o que ele nao grava
//!
//! Grava: panico do Rust (com a linha onde estourou), erro nao tratado da
//! interface, promessa rejeitada sem `catch`, e a janela declarada que abriu
//! mas nunca deu sinal de vida.
//!
//! **Nao grava conteudo do usuario.** Nenhuma Capture, nenhum trecho de
//! transcricao, nenhum titulo de reuniao passa por aqui. O que interessa e a
//! FALHA, e o texto da falha ja e escrito por nos ou pelo runtime. Isso nao e
//! zelo abstrato: `CONTRIBUTING.md` proibe log com conteudo pessoal no
//! repositorio, e o caminho mais curto entre um caderno de ocorrencias e um
//! vazamento e alguem colar o arquivo inteiro num relatorio de bug.
//!
//! # Onde ele mora
//!
//! `%APPDATA%/com.codedbym.mos/logs/ocorrencias.log`, ao lado do banco. Nao
//! vai para o `stderr` de proposito: um M/OS aberto pelo autostart no logon
//! nao tem `stderr` para onde escrever.

use std::{
    fs::{self, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::Duration,
};

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};

/// Teto do arquivo. Passou disso, a metade velha vai embora.
///
/// Duzentos e cinquenta kilobytes sao alguns milhares de ocorrencias — muito
/// mais do que qualquer investigacao precisa, e pouco o bastante para o arquivo
/// nunca virar um problema em si. Um log que cresce sem limite e uma segunda
/// falha esperando a primeira.
const TETO: u64 = 250 * 1024;

/// O destino, decidido uma vez na abertura.
///
/// `OnceLock` e nao um parametro em toda chamada porque quem mais precisa
/// escrever aqui e o hook de panico, que nao recebe argumento nenhum: ele e uma
/// closure global instalada no runtime.
static ARQUIVO: OnceLock<PathBuf> = OnceLock::new();

/// Uma escrita por vez.
///
/// Duas threads em panico simultaneo sao raras, mas e exatamente na hora do
/// panico que duas linhas entrelacadas destroem o valor do registro.
static CANETA: Mutex<()> = Mutex::new(());

/// A gravidade da ocorrencia, como ela aparece no arquivo.
#[derive(Clone, Copy)]
pub enum Nivel {
    /// O processo morreu, ou estava morrendo.
    Fatal,
    /// Algo falhou, mas o M/OS seguiu de pe.
    Erro,
    /// Vale saber depois. Nao interrompeu ninguem.
    Aviso,
}

impl Nivel {
    fn rotulo(self) -> &'static str {
        match self {
            Nivel::Fatal => "FATAL",
            Nivel::Erro => "ERRO ",
            Nivel::Aviso => "AVISO",
        }
    }

    fn de_texto(valor: &str) -> Self {
        match valor {
            "fatal" => Nivel::Fatal,
            "aviso" => Nivel::Aviso,
            _ => Nivel::Erro,
        }
    }
}

/// Prepara o caderno e instala o hook de panico.
///
/// Chamado uma vez, no `setup`, assim que o diretorio de dados existe. Falhar
/// aqui NAO impede o M/OS de abrir: um sistema que se recusa a funcionar porque
/// nao conseguiu abrir o proprio log seria pior que um sistema sem log.
pub fn instalar(diretorio_de_dados: &Path) {
    let pasta = diretorio_de_dados.join("logs");
    if fs::create_dir_all(&pasta).is_err() {
        return;
    }
    if ARQUIVO.set(pasta.join("ocorrencias.log")).is_err() {
        // Ja instalado. Acontece em teste, e nao e erro.
        return;
    }

    // O hook ANTERIOR continua rodando depois do nosso: ele e quem imprime o
    // backtrace no terminal durante `tauri dev`, e trocar diagnostico de
    // desenvolvimento por diagnostico de producao seria um mau negocio.
    let anterior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let onde = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "origem desconhecida".to_owned());
        let causa = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "sem mensagem".to_owned());
        let thread = std::thread::current()
            .name()
            .unwrap_or("sem nome")
            .to_owned();
        escrever(
            Nivel::Fatal,
            "rust",
            &format!("panico em {onde} (thread `{thread}`): {causa}"),
        );
        anterior(info);
    }));

    escrever(
        Nivel::Aviso,
        "abertura",
        &format!("M/OS {} subiu.", versao()),
    );
}

fn versao() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// O instante, em texto ordenavel.
///
/// UTC com sufixo explicito. Um log sem fuso declarado e um log que mente na
/// primeira vez que alguem o le em outro lugar.
fn agora() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "instante indisponivel".to_owned())
}

/// Uma linha no caderno.
///
/// Nunca entra em panico e nunca devolve erro: quem chama daqui muitas vezes JA
/// esta tratando uma falha, e uma falha ao registrar a falha nao pode virar a
/// terceira.
pub fn escrever(nivel: Nivel, origem: &str, mensagem: &str) {
    let Some(caminho) = ARQUIVO.get() else {
        return;
    };
    let Ok(_caneta) = CANETA.lock() else {
        return;
    };
    // A mensagem cabe em uma linha, sempre. Um `\n` vindo de um erro de
    // terceiro quebraria o formato e faria a proxima leitura tratar meia
    // ocorrencia como uma ocorrencia inteira.
    let limpa = mensagem.replace(['\n', '\r'], " ⏎ ");
    let linha = format!("{}\t{}\t{origem}\t{limpa}\n", agora(), nivel.rotulo());
    let Ok(mut arquivo) = OpenOptions::new().create(true).append(true).open(caminho) else {
        return;
    };
    let _ = arquivo.write_all(linha.as_bytes());
    let _ = arquivo.flush();
    if arquivo.metadata().map(|m| m.len()).unwrap_or(0) > TETO {
        podar(caminho);
    }
}

/// Corta o arquivo pela metade, mantendo o fim.
///
/// O fim, e nao o comeco: a ocorrencia que interessa e sempre a ultima.
fn podar(caminho: &Path) {
    let Ok(mut arquivo) = OpenOptions::new().read(true).open(caminho) else {
        return;
    };
    let mut texto = String::new();
    if arquivo.read_to_string(&mut texto).is_err() {
        return;
    }
    let corte = texto.len() / 2;
    // Do corte ate a proxima quebra: comecar no meio de uma linha deixaria a
    // primeira entrada do arquivo ilegivel.
    let inicio = texto[corte..]
        .find('\n')
        .map(|i| corte + i + 1)
        .unwrap_or(texto.len());
    let _ = fs::write(caminho, &texto[inicio..]);
}

// ===========================================================================
// A janela que abriu e nao deu sinal
// ===========================================================================

/// Quem ja disse "estou viva".
static VIVAS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Quem ja foi acusada de nao ter montado. Uma acusacao por janela, e so uma.
static ACUSADAS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Quanto tempo uma janela visivel tem para montar antes de virar ocorrencia.
///
/// Doze segundos e generoso para uma webview local, e curto o bastante para o
/// registro ainda estar perto do evento. A janelinha do canto que aparece com
/// erro 404 nunca chega a montar o React — entao ela nunca reporta, e e
/// exatamente isso que este relogio transforma em prova.
const PACIENCIA: Duration = Duration::from_secs(12);

/// A interface avisando que montou.
///
/// Chamado por `main.tsx` de toda janela, no primeiro render.
#[tauri::command]
pub fn diagnostico_janela_viva(rotulo: String) {
    if let Ok(mut vivas) = VIVAS.lock() {
        if !vivas.contains(&rotulo) {
            vivas.push(rotulo);
        }
    }
}

/// Vigia, enquanto o app viver, as janelas que aparecem sem montar.
///
/// # So as VISIVEIS
///
/// Tres das quatro janelas do `tauri.conf.json` nascem `visible: false` e
/// passam a vida escondidas — a captura rapida, o lembrete, a oferta de gravar.
/// Acusar janela oculta encheria o caderno de uma ocorrencia por abertura, e um
/// log que sempre acusa e um log que ninguem le. O sinal que interessa e
/// especifico: **uma janela que APARECEU na tela sem interface dentro.**
///
/// # Por que em laco, e nao uma vez
///
/// A janelinha do canto aparece quando um microfone abre — dez minutos depois
/// do logon, ou nunca. Uma conferencia unica na abertura olharia justamente o
/// instante em que ela ainda esta escondida, e nao veria nada. O laco custa uma
/// varredura de quatro janelas a cada quinze segundos.
///
/// `ACUSADAS` garante uma linha por janela por sessao: sem ela, uma janelinha
/// quebrada que fica aberta escreveria quatro linhas por minuto.
///
/// Thread propria porque a espera e longa e o `setup` nao pode bloquear — o
/// Tauri so entrega o laco de eventos depois que ele volta.
pub fn vigiar_janelas<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    std::thread::Builder::new()
        .name("mos-diagnostico".into())
        .spawn(move || loop {
            std::thread::sleep(PACIENCIA);
            for (rotulo, janela) in app.webview_windows() {
                if !janela.is_visible().unwrap_or(false) {
                    continue;
                }
                let (Ok(vivas), Ok(mut acusadas)) = (VIVAS.lock(), ACUSADAS.lock()) else {
                    continue;
                };
                if vivas.contains(&rotulo) || acusadas.contains(&rotulo) {
                    continue;
                }
                acusadas.push(rotulo.clone());
                let endereco = janela
                    .url()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|_| "endereco indisponivel".to_owned());
                escrever(
                    Nivel::Erro,
                    "janela",
                    &format!(
                        "`{rotulo}` esta na tela e nao montou a interface. Endereco: {endereco}"
                    ),
                );
            }
        })
        .ok();
}

// ===========================================================================
// A ponte com a interface
// ===========================================================================

/// A interface registrando o que ela viu quebrar.
///
/// `origem` diz QUAL janela — sem isso, "TypeError: x is not a function" nao
/// distingue o app inteiro da janelinha de 420 pixels.
#[tauri::command]
pub fn diagnostico_registrar(nivel: String, origem: String, mensagem: String) {
    // O texto vem da webview, entao ele e cortado: um erro com um stack trace
    // gigante entope o arquivo sem dizer mais do que as primeiras linhas dizem.
    let corte: String = mensagem.chars().take(1200).collect();
    escrever(Nivel::de_texto(&nivel), &origem, &corte);
}

/// Uma ocorrencia, como a tela de Settings a mostra.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ocorrencia {
    pub quando: String,
    pub nivel: String,
    pub origem: String,
    pub mensagem: String,
}

/// As ultimas ocorrencias, da mais nova para a mais velha.
///
/// Existe para a pessoa nao precisar abrir o `%APPDATA%` a mao. O caminho vai
/// junto (`diagnostico_caminho`) para quando ela quiser o arquivo inteiro.
#[tauri::command]
pub fn diagnostico_recente(limite: usize) -> Vec<Ocorrencia> {
    let Some(caminho) = ARQUIVO.get() else {
        return Vec::new();
    };
    let Ok(mut arquivo) = OpenOptions::new().read(true).open(caminho) else {
        return Vec::new();
    };
    let _ = arquivo.seek(SeekFrom::Start(0));
    let mut texto = String::new();
    if arquivo.read_to_string(&mut texto).is_err() {
        return Vec::new();
    }
    texto
        .lines()
        .rev()
        .take(limite.clamp(1, 500))
        .filter_map(analisar)
        .collect()
}

/// Onde o arquivo mora, para a tela poder abrir a pasta.
#[tauri::command]
pub fn diagnostico_caminho() -> String {
    ARQUIVO
        .get()
        .map(|c| c.display().to_string())
        .unwrap_or_default()
}

fn analisar(linha: &str) -> Option<Ocorrencia> {
    let mut campos = linha.splitn(4, '\t');
    Some(Ocorrencia {
        quando: campos.next()?.to_owned(),
        nivel: campos.next()?.trim().to_owned(),
        origem: campos.next()?.to_owned(),
        mensagem: campos.next()?.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_linha_se_desfaz_nos_mesmos_quatro_campos() {
        let linha = "2026-08-25T12:00:00Z\tERRO \tjanela\t`lembrete` nao montou";
        let ocorrencia = analisar(linha).expect("a linha tem os quatro campos");
        assert_eq!(ocorrencia.nivel, "ERRO");
        assert_eq!(ocorrencia.origem, "janela");
        assert_eq!(ocorrencia.mensagem, "`lembrete` nao montou");
    }

    #[test]
    fn linha_truncada_nao_vira_ocorrencia_pela_metade() {
        assert!(analisar("2026-08-25T12:00:00Z\tERRO").is_none());
    }

    /// A mensagem de varias linhas cabe em uma so.
    ///
    /// Sem isto, um stack trace vindo da webview quebraria o formato e a
    /// proxima leitura trataria cada linha do trace como uma ocorrencia.
    #[test]
    fn a_mensagem_nunca_quebra_o_formato() {
        let mensagem = "TypeError\n  at foo\r\n  at bar";
        let limpa = mensagem.replace(['\n', '\r'], " ⏎ ");
        assert!(!limpa.contains('\n'));
    }
}
