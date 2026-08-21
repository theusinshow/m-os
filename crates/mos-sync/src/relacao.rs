//! As relações do Knowledge Graph, como entidade de primeira classe.
//!
//! # O problema que este arquivo resolve
//!
//! `resource_projects` e `resource_workspaces` são junções: duas colunas, sem
//! id próprio. Sincronizá-las como campo de uma das pontas não funciona —
//! **merge por campo não serve para conjunto**. Se a lista de Projects de um
//! Resource fosse um campo, ligar a um Project no celular apagaria a ligação
//! feita no PC, porque o campo inteiro seria substituído pelo mais recente.
//!
//! O §24 pede relação como entidade de primeira classe, e é isso que ela vira
//! aqui. Duas decisões sustentam o desenho.
//!
//! # 1. O id é DERIVADO do par, e não sorteado
//!
//! Se cada dispositivo sorteasse um id ao ligar, dois dispositivos ligando o
//! mesmo Resource ao mesmo Project criariam **duas relações para o mesmo
//! vínculo** — e desfazer uma deixaria a outra de pé.
//!
//! UUID v5 é determinístico por nome: os dois lados calculam o mesmo id sem se
//! falarem. Ligar duas vezes vira a mesma entidade, e a idempotência sai de
//! graça.
//!
//! # 2. Desligar é CAMPO, e não `OpBody::Delete`
//!
//! `Delete` no motor tem semântica de "apagar ganha de editar", que está certa
//! para uma Task e **errada para um interruptor**: desvincular às 10:00 e
//! revincular às 10:05 tem que terminar vinculado.
//!
//! Então a relação nunca é apagada — ela tem um campo `linked`, e o merge por
//! campo decide pelo instante. Último gesto vence, que é o que um interruptor
//! deve fazer.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{EntityRef, OpBody};

/// O espaço de nomes das relações do M/OS.
///
/// Constante e arbitrário, como todo namespace de UUID v5 — o que importa é
/// que seja o MESMO em todos os dispositivos e nunca mude. Mudá-lo faria todas
/// as relações existentes ganharem ids novos, e as antigas ficariam órfãs.
const NAMESPACE: Uuid = Uuid::from_bytes([
    0x6d, 0x6f, 0x73, 0x72, 0x65, 0x6c, 0x61, 0x63, 0x61, 0x6f, 0x6b, 0x67, 0x72, 0x61, 0x70, 0x68,
]);

/// O tipo de um vínculo.
///
/// Texto, e não enum fechado, pelo mesmo motivo de `EntityKind`: um cliente
/// antigo precisa guardar e reenviar uma relação de um tipo que ele ainda não
/// conhece, em vez de descartá-la.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RelationKind(pub String);

impl RelationKind {
    pub fn new(nome: impl Into<String>) -> Self {
        Self(nome.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Um vínculo entre duas entidades.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relacao {
    pub kind: RelationKind,
    pub from: Uuid,
    pub to: Uuid,
}

impl Relacao {
    pub fn nova(kind: impl Into<String>, from: Uuid, to: Uuid) -> Self {
        Self {
            kind: RelationKind::new(kind),
            from,
            to,
        }
    }

    /// O id desta relação, igual em qualquer dispositivo.
    ///
    /// A ordem de `from` e `to` **faz parte da identidade**: `A→B` e `B→A` são
    /// vínculos diferentes, e colapsá-los tornaria impossível expressar uma
    /// relação com direção. Quem quiser um vínculo simétrico normaliza a ordem
    /// antes de chamar.
    pub fn id(&self) -> Uuid {
        let nome = format!("{}:{}:{}", self.kind.as_str(), self.from, self.to);
        Uuid::new_v5(&NAMESPACE, nome.as_bytes())
    }

    pub fn entidade(&self) -> EntityRef {
        EntityRef::new("relation", self.id())
    }

    /// A operação de ligar ou desligar.
    ///
    /// Sempre `Update`, e nunca `Delete`. Os campos de identificação viajam
    /// junto porque um dispositivo que recebe esta operação sem nunca ter visto
    /// a relação precisa saber **o que** foi ligado — o id sozinho é um hash,
    /// e não diz nada.
    pub fn alternar(&self, linked: bool) -> OpBody {
        OpBody::Update {
            fields: [
                ("kind".to_owned(), serde_json::json!(self.kind.as_str())),
                ("from".to_owned(), serde_json::json!(self.from.to_string())),
                ("to".to_owned(), serde_json::json!(self.to.to_string())),
                ("linked".to_owned(), serde_json::json!(linked)),
            ]
            .into_iter()
            .collect(),
        }
    }
}
