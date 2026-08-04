//! Regras de dominio puras, independentes da interface e da persistencia.
//!
//! Espelham a logica testada no frontend, mas o backend e a fonte da verdade
//! para calculo de duracao (secao 9): timestamps sao persistidos e as duracoes
//! recalculadas aqui, robustas a alteracoes do relogio do sistema.

// As funcoes de dominio sao comprovadas por testes (`cargo test`) e serao
// ligadas aos comandos do cronometro na Fase 3. Ate la, nao ha chamador no
// caminho de producao — o `allow` documenta essa intencao sem mascarar erros.
#[allow(dead_code)]
pub mod timer;

pub mod billing;
