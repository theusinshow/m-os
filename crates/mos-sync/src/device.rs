//! Quem e cada instalacao.
//!
//! O §9 da missao pede que o usuario consiga ver os proprios dispositivos. Mas
//! a identidade serve antes disso a tres coisas que nao aparecem na tela: ela
//! desempata o HLC, ela e a origem que a Timeline mostra, e ela e o que permite
//! a um dispositivo saber o que **ele** ja enviou.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A identidade de uma instalacao do M/OS.
///
/// Nasce uma vez, na primeira abertura, e vive no banco local. Nao e o hardware
/// e nao e a conta: reinstalar o app cria um dispositivo novo, e isso e o certo
/// — o banco local tambem e outro.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceId(pub Uuid);

impl DeviceId {
    pub fn novo() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A plataforma de um dispositivo.
///
/// `Outra` existe para o §68: um cliente futuro — macOS, iPad, Android, web —
/// tem que conseguir se apresentar a um M/OS que ainda nao o conhece, e ser
/// listado com o proprio nome em vez de sumir da lista de dispositivos.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    Windows,
    Ios,
    Macos,
    Android,
    Outra(String),
}

impl Platform {
    pub fn as_str(&self) -> &str {
        match self {
            Platform::Windows => "windows",
            Platform::Ios => "ios",
            Platform::Macos => "macos",
            Platform::Android => "android",
            Platform::Outra(nome) => nome,
        }
    }

    /// Le de texto, e nunca falha: um nome desconhecido vira `Outra`, e nao
    /// erro. Ver a nota em `Outra`.
    pub fn ler(texto: &str) -> Self {
        match texto {
            "windows" => Platform::Windows,
            "ios" => Platform::Ios,
            "macos" => Platform::Macos,
            "android" => Platform::Android,
            outro => Platform::Outra(outro.to_owned()),
        }
    }
}

/// O registro de um dispositivo, como o usuario o ve e como o sync o usa.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: DeviceId,
    /// O nome que a pessoa reconhece: "PC Principal", "iPhone 14 Pro".
    pub name: String,
    pub platform: Platform,
    /// Versao do app que rodou por ultimo neste dispositivo. E o que permite ao
    /// §27 decidir se um cliente esta velho demais para uma operacao.
    pub app_version: String,
    /// Ultima sincronizacao concluida, em ISO-8601. Vazio significa "nunca".
    pub last_sync_at: String,
    /// Verdadeiro no dispositivo em que este registro esta sendo lido.
    #[serde(default)]
    pub is_this_device: bool,
}
