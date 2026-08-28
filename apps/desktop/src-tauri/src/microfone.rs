//! Quem esta com o microfone aberto, segundo o Windows.
//!
//! **Somente leitura de registro.** Sem hook, sem injecao, sem captura. O unico
//! dado que sai daqui e QUEM e DESDE QUANDO — nunca titulo de janela, nunca
//! conteudo de aba, nunca audio. A ADR-047 depende dessa estreiteza, e e ela que
//! separa isto de software de vigilancia.
//!
//! O Windows mantem DOIS caminhos, e os dois importam: apps da Store ficam
//! direto sob `microphone`, apps Win32 sob `microphone\NonPackaged`, com o
//! caminho do executavel e as barras trocadas por `#`. Ler so um deixaria
//! buracos que se parecem com "as vezes nao funciona".
//!
//! `LastUsedTimeStop == 0` significa EM USO AGORA. `LastUsedTimeStart` e
//! FILETIME — 100 ns desde 1601 —, e e dele que sai ha quanto tempo.

#[cfg(windows)]
use std::collections::BTreeSet;

use mos_core::MicrofoneAberto;

#[cfg(windows)]
const CONSENT: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone";

/// Segundos entre 1601-01-01 e 1970-01-01, para converter FILETIME em epoch.
#[cfg(windows)]
const EPOCH_1601_PARA_1970: i64 = 11_644_473_600;

#[cfg(windows)]
pub fn abertos_agora() -> Vec<MicrofoneAberto> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    let agora = time::OffsetDateTime::now_utc().unix_timestamp();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(raiz) = hkcu.open_subkey_with_flags(CONSENT, KEY_READ) else {
        // Sem a chave nao ha o que observar. Devolver vazio e o certo: a oferta
        // simplesmente nao acontece, e nada finge ter observado.
        return Vec::new();
    };

    let mut encontrados = Vec::new();
    let mut vistos = BTreeSet::new();

    for nome in raiz.enum_keys().flatten() {
        if nome.eq_ignore_ascii_case("NonPackaged") {
            let Ok(sub) = raiz.open_subkey_with_flags(&nome, KEY_READ) else {
                continue;
            };
            for chave in sub.enum_keys().flatten() {
                if let Some(entrada) = ler(&sub, &chave, agora) {
                    if vistos.insert(entrada.processo.clone()) {
                        encontrados.push(entrada);
                    }
                }
            }
        } else if let Some(entrada) = ler(&raiz, &nome, agora) {
            if vistos.insert(entrada.processo.clone()) {
                encontrados.push(entrada);
            }
        }
    }
    encontrados
}

#[cfg(not(windows))]
pub fn abertos_agora() -> Vec<MicrofoneAberto> {
    // Fora do Windows nao ha ConsentStore. Vazio, e nao erro: a feature
    // simplesmente nao existe la, e a ADR-001 ja limita a plataforma.
    Vec::new()
}

#[cfg(windows)]
fn ler(pai: &winreg::RegKey, chave: &str, agora: i64) -> Option<MicrofoneAberto> {
    use winreg::enums::KEY_READ;

    let entrada = pai.open_subkey_with_flags(chave, KEY_READ).ok()?;
    let parou: u64 = entrada.get_value("LastUsedTimeStop").ok()?;
    // Zero e o unico valor que significa "aberto agora".
    if parou != 0 {
        return None;
    }
    let comecou: u64 = entrada.get_value("LastUsedTimeStart").ok()?;
    let inicio = (comecou as i64) / 10_000_000 - EPOCH_1601_PARA_1970;
    Some(MicrofoneAberto {
        processo: nome_do_processo(chave),
        // `max(0)` porque relogio que anda para tras nao pode virar tempo
        // negativo aberto — isso passaria a espera de 20 s ao contrario, e a
        // oferta apareceria no instante em que o microfone abrisse.
        segundos_aberto: (agora - inicio).max(0),
    })
}

/// O nome do executavel a partir da chave do registro.
///
/// Win32: `C:#Program Files#Google#Chrome#Application#chrome.exe` vira
/// `chrome.exe`. App da Store: o nome da familia do pacote fica como esta — ele
/// ja e um identificador, e nao ha executavel a extrair.
#[cfg(windows)]
fn nome_do_processo(chave: &str) -> String {
    match chave.rsplit('#').next() {
        Some(ultimo) if ultimo.to_lowercase().ends_with(".exe") => ultimo.to_string(),
        _ => chave.to_string(),
    }
}
