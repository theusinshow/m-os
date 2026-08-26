//! O outro lado do `Transport`.
//!
//! # Por que ele e tao pequeno
//!
//! O `SYNC.md` §11 diz que o `HubLocal` do teste `sync_two_devices.rs` vale
//! como **especificacao executavel do servidor**: guardar em ordem, devolver a
//! partir de um cursor, aceitar reenvio sem duplicar. Este crate e essa
//! especificacao com um disco embaixo e uma porta na frente — nada mais.
//!
//! A tentacao de por regra aqui e real e deve ser recusada. O M/OS e
//! local-first: cada dispositivo tem o banco inteiro e reconcilia sozinho, e o
//! servidor existe para os dois se alcancarem quando nao estao na mesma rede.
//! Um servidor que decide qualquer coisa vira a autoridade, e ai o app offline
//! passa a ser uma versao degradada em vez da versao normal.
//!
//! # O que ele NAO faz, e a razao de cada um
//!
//! - **Nao reconcilia.** A ordem total sai do HLC, que viaja dentro da
//!   operacao. Dois dispositivos que nunca falaram com o servidor chegam a
//!   mesma conclusao — e e isso que faz a sincronizacao ser uma conveniencia, e
//!   nao um requisito.
//! - **Nao conhece entidade.** `EntityKind` e texto justamente para o servidor
//!   poder guardar e devolver um tipo que ele nunca viu.
//! - **Nao filtra o que o dispositivo mandou.** O `HubLocal` tambem nao filtra,
//!   e a diferenca importa: aplicar a propria operacao de volta e inofensivo
//!   (a reconciliacao e deterministica), enquanto filtrar exigiria o servidor
//!   saber quem e quem e abriria a porta para ele decidir coisas.

pub mod http;
pub mod hub;

pub use http::{rotas, Estado};
pub use hub::{Hub, HubError};
