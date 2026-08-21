//! A operacao: a unidade que viaja entre dispositivos.
//!
//! # Por que operacao, e nao "a entidade inteira"
//!
//! Se o celular mandasse a Task inteira e o PC mandasse a Task inteira, a unica
//! reconciliacao possivel seria escolher uma das duas — e a outra sumiria. Foi
//! isso que o §8 da missao proibiu com todas as letras: editar o titulo no PC e
//! a data no celular tem que resultar nas duas coisas.
//!
//! Entao o que viaja e a MUDANCA DE CAMPO. Duas mudancas em campos diferentes
//! nao se tocam; duas mudancas no mesmo campo sao um conflito de verdade, e ai
//! sim ha uma decisao a tomar.
//!
//! # Por que o id da operacao e a chave de idempotencia
//!
//! O §53 exige que um retry nao duplique nada. O id nasce no dispositivo que
//! originou a mudanca, antes de qualquer envio, e viaja com ela: reenviar a
//! mesma operacao dez vezes aplica uma vez. E o mesmo id que o §78 pede para as
//! acoes do Hermes.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DeviceId, Hlc};

/// O tipo de uma entidade sincronizavel.
///
/// Texto, e nao enum fechado, de proposito: um cliente antigo precisa conseguir
/// **guardar e reenviar** uma operacao sobre um tipo que ele ainda nao conhece,
/// sem descartar (§27 e §74 — versoes N e N-1 convivem). Enum fechado
/// transformaria "tipo desconhecido" em erro de desserializacao, e a operacao
/// morreria no cliente velho.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityKind(pub String);

impl EntityKind {
    pub fn new(nome: impl Into<String>) -> Self {
        Self(nome.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Uma entidade, endereçada globalmente.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityRef {
    pub kind: EntityKind,
    /// O mesmo id em todos os dispositivos. O M/OS ja usa UUID v7 em todo lugar,
    /// que e ordenavel por tempo e nao colide entre maquinas — nao houve nada a
    /// mudar aqui, e e por isso que a sincronizacao nao precisou de um mapa de
    /// "id local para id remoto".
    pub id: Uuid,
}

impl EntityRef {
    pub fn new(kind: impl Into<String>, id: Uuid) -> Self {
        Self {
            kind: EntityKind::new(kind),
            id,
        }
    }
}

/// O que uma operacao faz.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum OpBody {
    /// Cria a entidade com os campos dados. Reaplicar e inofensivo: a criacao e
    /// identificada pelo id da entidade, que ja veio decidido.
    Create {
        fields: serde_json::Map<String, serde_json::Value>,
    },
    /// Muda campos. So os campos presentes; ausencia significa "nao mexi".
    Update {
        fields: serde_json::Map<String, serde_json::Value>,
    },
    /// Apaga logicamente.
    ///
    /// **Nunca exclusao fisica**, e por dois motivos independentes. O primeiro e
    /// de sincronizacao: um dispositivo que estava offline precisa saber que
    /// algo foi apagado, e uma linha ausente e indistinguivel de uma linha que
    /// nunca chegou. O segundo ja era regra do M/OS antes desta missao —
    /// arquivar antes de excluir, e todo Undo e restauracao de estado.
    Delete,
    /// Desfaz o apagamento logico.
    Restore,
}

/// Uma mudanca, pronta para viajar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Op {
    /// Chave de idempotencia. Nasce na origem, sobrevive a todo retry.
    pub id: Uuid,
    pub entity: EntityRef,
    pub body: OpBody,
    /// Quando, na ordem total do HLC. Contem o dispositivo de origem, que e o
    /// que a Timeline (§25) mostra e o que a auditoria pergunta.
    pub at: Hlc,
}

impl Op {
    pub fn new(id: Uuid, entity: EntityRef, body: OpBody, at: Hlc) -> Self {
        Self {
            id,
            entity,
            body,
            at,
        }
    }

    pub fn device(&self) -> DeviceId {
        self.at.device
    }

    /// Os nomes dos campos que esta operacao toca. Vazio para `Delete` e
    /// `Restore`, que agem sobre a entidade inteira.
    pub fn campos(&self) -> Vec<&str> {
        match &self.body {
            OpBody::Create { fields } | OpBody::Update { fields } => {
                fields.keys().map(String::as_str).collect()
            }
            OpBody::Delete | OpBody::Restore => Vec::new(),
        }
    }
}
