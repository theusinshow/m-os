//! Sinais de uso: onde o M/OS ainda pede trabalho manual (F9 do
//! MOS-OVERHAUL-PLAN, ADR-082).
//!
//! **Não é placar de produtividade** e não sai do aparelho. Cada número responde
//! uma pergunta do plano — a Inbox está parando? a organização automática
//! ajuda? as reuniões andam sozinhas? o sync atrapalha? o dia virou
//! administração? — e vem de tabelas que já existem. Nada de conteúdo:
//! contagens, idades e desfechos.

use serde::{Deserialize, Serialize};

/// O retrato de uma janela de dias.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SinaisDeUso {
    pub janela_dias: i64,
    pub inbox: SinaisDaInbox,
    pub propostas: SinaisDePropostas,
    pub reunioes: SinaisDeReunioes,
    pub adiamentos: SinaisDeAdiamento,
    pub sync: SinaisDeSync,
    pub dia: SinaisDoDia,
}

/// A Inbox está parando?
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SinaisDaInbox {
    pub pendentes: usize,
    pub idade_mediana_horas: Option<i64>,
    pub mais_de_7_dias: usize,
}

/// A organização pelo agente ajuda? Desfecho das propostas do Hermes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SinaisDePropostas {
    pub executadas: usize,
    pub canceladas: usize,
    pub recusadas: usize,
    pub falharam: usize,
    /// Reservadas sem desfecho gravado (ADR-074): o app caiu no meio.
    pub incertas: usize,
}

/// As reuniões andam sozinhas?
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SinaisDeReunioes {
    pub prontas: usize,
    /// Paradas esperando alguém: transcritor ausente, falha, consentimento.
    pub pedem_atencao: usize,
    /// Do job criado até pronto. Aproxima "parar → pronta".
    pub ate_pronta_mediana_minutos: Option<i64>,
    pub tentativas_extras: usize,
}

/// Onde a pessoa empurra em vez de decidir.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SinaisDeAdiamento {
    pub lembretes_adiados_na_janela: usize,
    /// Lembretes abertos adiados 3 vezes ou mais: o aviso virou ruído.
    pub lembretes_adiados_3x: usize,
    /// Tasks ativas adiadas 3 vezes ou mais.
    pub tasks_adiadas_3x: usize,
}

/// O sync atrapalha?
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SinaisDeSync {
    pub fila: usize,
    pub mais_antiga_horas: Option<i64>,
}

/// O dia virou administração?
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SinaisDoDia {
    pub dias_iniciados: usize,
    pub dias_encerrados: usize,
}

/// A mediana de uma lista, sem ordenar a original.
pub fn mediana(valores: &[i64]) -> Option<i64> {
    if valores.is_empty() {
        return None;
    }
    let mut ordenados = valores.to_vec();
    ordenados.sort_unstable();
    let meio = ordenados.len() / 2;
    Some(if ordenados.len() % 2 == 0 {
        (ordenados[meio - 1] + ordenados[meio]) / 2
    } else {
        ordenados[meio]
    })
}

impl SinaisDeUso {
    /// Frases curtas para a tela, só do que pede atenção. Vazio quando nada
    /// chama atenção — e isso também é uma resposta.
    pub fn leituras(&self) -> Vec<String> {
        let mut frases = Vec::new();
        if self.inbox.mais_de_7_dias > 0 {
            frases.push(format!(
                "{} {} na Inbox há mais de uma semana.",
                self.inbox.mais_de_7_dias,
                if self.inbox.mais_de_7_dias == 1 {
                    "captura"
                } else {
                    "capturas"
                }
            ));
        }
        let decididas = self.propostas.executadas + self.propostas.canceladas;
        if decididas >= 5 && self.propostas.canceladas * 2 > decididas {
            frases.push("Mais da metade das propostas do Hermes foi cancelada.".to_owned());
        }
        if self.propostas.incertas > 0 {
            frases.push(format!(
                "{} {} do Hermes sem desfecho confirmado.",
                self.propostas.incertas,
                if self.propostas.incertas == 1 {
                    "ação"
                } else {
                    "ações"
                }
            ));
        }
        if self.reunioes.pedem_atencao > 0 {
            frases.push(format!(
                "{} {} parada esperando você.",
                self.reunioes.pedem_atencao,
                if self.reunioes.pedem_atencao == 1 {
                    "reunião"
                } else {
                    "reuniões"
                }
            ));
        }
        if self.adiamentos.lembretes_adiados_3x > 0 {
            frases.push(format!(
                "{} {} adiado 3 vezes ou mais.",
                self.adiamentos.lembretes_adiados_3x,
                if self.adiamentos.lembretes_adiados_3x == 1 {
                    "lembrete"
                } else {
                    "lembretes"
                }
            ));
        }
        if self.adiamentos.tasks_adiadas_3x > 0 {
            frases.push(format!(
                "{} {} adiada 3 vezes ou mais.",
                self.adiamentos.tasks_adiadas_3x,
                if self.adiamentos.tasks_adiadas_3x == 1 {
                    "Task"
                } else {
                    "Tasks"
                }
            ));
        }
        if self.sync.mais_antiga_horas.is_some_and(|horas| horas >= 24) {
            frases.push("Há mudança esperando mais de um dia para sincronizar.".to_owned());
        }
        frases
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mediana_par_impar_e_vazia() {
        assert_eq!(mediana(&[]), None);
        assert_eq!(mediana(&[5, 1, 3]), Some(3));
        assert_eq!(mediana(&[4, 1, 3, 2]), Some(2));
    }

    #[test]
    fn sem_nada_chamando_atencao_nao_ha_leitura() {
        assert!(SinaisDeUso::default().leituras().is_empty());
    }

    #[test]
    fn leituras_so_do_que_pede_atencao() {
        let sinais = SinaisDeUso {
            inbox: SinaisDaInbox {
                pendentes: 9,
                idade_mediana_horas: Some(30),
                mais_de_7_dias: 2,
            },
            propostas: SinaisDePropostas {
                executadas: 2,
                canceladas: 4,
                incertas: 1,
                ..Default::default()
            },
            sync: SinaisDeSync {
                fila: 3,
                mais_antiga_horas: Some(30),
            },
            ..Default::default()
        };
        let frases = sinais.leituras();
        assert_eq!(frases.len(), 4, "{frases:?}");
        assert!(frases[0].contains("2 capturas"));
    }
}
