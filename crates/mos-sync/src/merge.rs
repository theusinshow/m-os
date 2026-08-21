//! A reconciliacao: como duas historias viram uma.
//!
//! # A regra, em uma frase
//!
//! Campos diferentes **convivem**; o mesmo campo escrito duas vezes em paralelo
//! e um **conflito**, e conflito nunca some em silencio.
//!
//! # Por que "ultima gravacao vence" nao basta
//!
//! O §8 da missao proibe LWW cego, e com razao: com LWW por ENTIDADE, editar o
//! titulo no PC e a data no celular faz uma das duas edicoes desaparecer sem
//! aviso. Aqui o LWW e por CAMPO — e mesmo assim, quando ele decide, o valor
//! perdedor **nao e jogado fora**: vira um `Conflito`, com os dois lados, para
//! a interface poder mostrar.
//!
//! Isso e o que separa "resolver o conflito" de "escolher um e apagar o outro".
//!
//! # O apagamento ganha do resto
//!
//! Uma entidade apagada em qualquer dispositivo permanece apagada, mesmo que
//! outro a tenha editado depois — a nao ser que alguem a restaure
//! explicitamente, o que e um `Restore` com instante proprio. A razao e
//! assimetrica: restaurar o que foi apagado por engano custa um clique;
//! descobrir semanas depois que algo voltou sozinho custa confianca no sistema.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Hlc, Op, OpBody};

/// O estado reconciliado de uma entidade.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstadoDaEntidade {
    /// Valor corrente de cada campo, e o instante que o colocou ali. O instante
    /// por campo e o que permite a proxima operacao decidir sozinha se ela e
    /// mais nova, sem consultar ninguem.
    pub campos: BTreeMap<String, CampoResolvido>,
    /// Instante do apagamento logico, quando houver.
    pub deleted_at: Option<Hlc>,
    /// Verdadeiro depois que qualquer operacao de criacao foi vista. Uma
    /// entidade que so recebeu `Update` existe do mesmo jeito — a criacao pode
    /// estar num lote que ainda nao chegou, e recusar o update perderia dado.
    pub existe: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampoResolvido {
    pub valor: serde_json::Value,
    pub at: Hlc,
}

/// Duas escritas concorrentes no mesmo campo, com valores diferentes.
///
/// A que venceu esta no estado; a que perdeu esta aqui. Nada e descartado.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflito {
    pub campo: String,
    pub vencedor: CampoResolvido,
    pub perdedor: CampoResolvido,
}

/// O resultado de reconciliar um lote.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reconciliacao {
    pub estado: EstadoDaEntidade,
    pub conflitos: Vec<Conflito>,
}

/// Aplica operacoes sobre um estado, em qualquer ordem de chegada.
///
/// **A ordem de chegada nao importa** — e isso e o requisito, nao um detalhe. A
/// rede entrega fora de ordem, o retry reenvia o que ja passou, e um lote pode
/// vir de tres dispositivos ao mesmo tempo. Quem decide e o instante da
/// operacao, nao o momento em que ela caiu aqui. Duas maquinas que receberam o
/// mesmo conjunto chegam ao mesmo estado, tenham recebido na ordem que for.
///
/// Reaplicar uma operacao ja aplicada nao muda nada: ela perde do proprio valor
/// que colocou la, porque a comparacao e estritamente "maior que". E dai que sai
/// a idempotencia que o §53 pede.
pub fn aplicar(mut estado: EstadoDaEntidade, ops: &[Op]) -> Reconciliacao {
    let mut conflitos = Vec::new();

    // Ordenar antes de aplicar deixa o resultado independente da ordem de
    // chegada, e faz cada campo ser decidido uma vez so.
    let mut ordenadas: Vec<&Op> = ops.iter().collect();
    ordenadas.sort_by_key(|op| op.at);

    for op in ordenadas {
        match &op.body {
            OpBody::Create { fields } | OpBody::Update { fields } => {
                if matches!(op.body, OpBody::Create { .. }) {
                    estado.existe = true;
                }
                // Um `Update` tambem prova existencia: ver nota em `existe`.
                estado.existe = true;
                for (campo, valor) in fields {
                    let novo = CampoResolvido {
                        valor: valor.clone(),
                        at: op.at,
                    };
                    match estado.campos.get(campo) {
                        None => {
                            estado.campos.insert(campo.clone(), novo);
                        }
                        Some(atual) => {
                            if novo.at > atual.at {
                                // O novo vence. Se o valor era diferente e as
                                // duas escritas sao de dispositivos distintos,
                                // houve concorrencia de verdade — o perdedor
                                // fica registrado.
                                if atual.valor != novo.valor && atual.at.device != novo.at.device {
                                    conflitos.push(Conflito {
                                        campo: campo.clone(),
                                        vencedor: novo.clone(),
                                        perdedor: atual.clone(),
                                    });
                                }
                                estado.campos.insert(campo.clone(), novo);
                            } else if novo.valor != atual.valor
                                && novo.at.device != atual.at.device
                                && novo.at != atual.at
                            {
                                // Chegou mais velha que a corrente: perde, mas
                                // continua sendo uma escrita concorrente que
                                // alguem fez de verdade.
                                conflitos.push(Conflito {
                                    campo: campo.clone(),
                                    vencedor: atual.clone(),
                                    perdedor: novo,
                                });
                            }
                        }
                    }
                }
            }
            OpBody::Delete => {
                estado.existe = true;
                estado.deleted_at = Some(match estado.deleted_at {
                    Some(anterior) if anterior > op.at => anterior,
                    _ => op.at,
                });
            }
            OpBody::Restore => {
                // So restaura o que foi apagado ANTES: um `Restore` velho nao
                // desfaz um apagamento novo.
                if let Some(apagado) = estado.deleted_at {
                    if op.at > apagado {
                        estado.deleted_at = None;
                    }
                }
            }
        }
    }

    Reconciliacao { estado, conflitos }
}

impl EstadoDaEntidade {
    /// Se a entidade deve aparecer para o usuario.
    pub fn visivel(&self) -> bool {
        self.existe && self.deleted_at.is_none()
    }

    /// O valor corrente de um campo.
    pub fn campo(&self, nome: &str) -> Option<&serde_json::Value> {
        self.campos.get(nome).map(|resolvido| &resolvido.valor)
    }
}
