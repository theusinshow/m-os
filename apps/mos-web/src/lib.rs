//! O M/OS de bolso, como biblioteca.
//!
//! O binario e uma casca fina sobre isto. A separacao existe para o teste poder
//! subir o `mos-web` de VERDADE — mesmo estado, mesmas rotas — em vez de falar
//! com um processo por fora ou, pior, testar uma copia do caminho real.
//!
//! Ja aconteceu neste repositorio: um teste de sincronizacao montava o
//! `Deposito` e a `Projecao` a mao, passava por um caminho que o M/OS nao usa, e
//! por isso nao exercitava a retentativa de materializacao — ele dizia que
//! estava tudo bem enquanto uma entidade ficava invisivel.

#[cfg(feature = "passkey")]
pub mod auth;

pub mod api;
pub mod assinaturas;
pub mod avisos;
pub mod estado;
pub mod push;
pub mod sync;
