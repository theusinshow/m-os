//! A camada de DECISAO sobre os compromissos academicos.
//!
//! O M/Academic ate aqui respondia "o que existe na faculdade". Este modulo e o
//! que o faz responder **"o que exige minha atencao agora"** — e a diferenca
//! entre as duas perguntas e inteiramente a decisao de quem estuda.
//!
//! # Os dois vocabularios
//!
//! | | quem escreve | o que responde |
//! | --- | --- | --- |
//! | `status` | o Univirtus (ou a pessoa, a mao) | o que o PORTAL registra |
//! | `decision` | so a pessoa | o que EU resolvi |
//!
//! Eles discordam de proposito, e o caso comum e justamente esse: a pessoa
//! entrega o trabalho as 23h e o portal so atualiza no dia seguinte. Ate la o
//! M/OS teria de continuar cobrando algo que ja foi feito — a nao ser que ela
//! possa dizer "ja entreguei", e que essa frase sobreviva ao proximo sync.
//!
//! # Por que a atencao e derivada, e nunca gravada
//!
//! "Precisa de atencao" muda sozinho com a passagem do tempo: o que era "esta
//! semana" vira "hoje" sem ninguem tocar em nada. Uma coluna `needs_attention`
//! estaria errada toda madrugada. E a mesma decisao do `status` de semestre na
//! 0031.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::academic::{Compromisso, Horizonte};
use crate::error::{CoreError, ErrorCode};

/// O que a pessoa resolveu sobre um compromisso.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Ainda nao resolvi nada. E o estado que pede atencao.
    #[default]
    None,
    /// Considero resolvido, o portal concordando ou nao.
    Done,
    /// Decidi nao fazer. Sai da atencao e fica no historico, reversivel.
    Skipped,
}

impl Decision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Done => "done",
            Self::Skipped => "skipped",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value.trim() {
            "none" | "" => Ok(Self::None),
            "done" => Ok(Self::Done),
            "skipped" => Ok(Self::Skipped),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Decisao academica desconhecida no banco local.",
                false,
            )),
        }
    }

    /// A decisao tira o item da fila do que pede acao?
    pub fn is_settled(self) -> bool {
        matches!(self, Self::Done | Self::Skipped)
    }
}

/// Onde um compromisso cai na tela.
///
/// Uma faixa so por item: um compromisso que aparecesse em "precisa de atencao"
/// **e** em "esta semana" faria a pessoa decidir duas vezes sobre a mesma coisa,
/// e contar duas vezes o que falta.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Faixa {
    /// Pede decisao agora: vencido, hoje, amanha, ou prova nos proximos dias.
    Atencao,
    /// Tem data nos proximos sete dias e ja esta encaminhado.
    Semana,
    /// Mais adiante.
    Depois,
    /// Resolvido, descartado, ou de um semestre que ja fechou.
    Historico,
}

/// Quantos dias antes uma prova comeca a pedir atencao.
///
/// Tres, e nao sete: prova daqui a uma semana ainda e "esta semana", e colocar
/// tudo em atencao esvazia o sentido da palavra. Tres dias e o ponto em que
/// deixar para depois passa a custar.
pub const DIAS_DE_PROVA_EM_ATENCAO: i64 = 3;

/// O que a faixa precisa saber alem do proprio compromisso.
#[derive(Clone, Copy, Debug)]
pub struct ContextoDaFaixa {
    /// Ja no fuso de quem esta olhando.
    pub agora_local: OffsetDateTime,
    /// O semestre do item ja terminou?
    ///
    /// # Por que isto importa
    ///
    /// Uma atividade vencida ha 151 dias, de um semestre fechado, nao e
    /// urgencia: e arqueologia. Sem esta regra a primeira viewport do
    /// M/Academic viraria um deposito de tudo que nunca foi entregue desde
    /// 2025 — e o que vence amanha ficaria embaixo.
    pub semestre_encerrado: bool,
    /// Quando o semestre corrente comecou.
    ///
    /// # O atraso que nunca foi um atraso
    ///
    /// O Univirtus republica etapas de trabalho com prazos de ciclos
    /// anteriores: "Estatica dos Corpos" chega no semestre 2026B2 (julho) com
    /// quatro entregas vencidas em marco e maio. Elas nao sao coisas que a
    /// pessoa deixou de fazer neste semestre — sao restos do calendario antigo
    /// que vieram junto.
    ///
    /// Um prazo anterior ao inicio do proprio semestre e o sinal, e ele e
    /// preciso: nada que valha para este periodo vence antes de ele comecar.
    pub semestre_comecou_em: Option<OffsetDateTime>,
}

/// Em que faixa este compromisso cai.
pub fn faixa_de(item: &Compromisso, contexto: ContextoDaFaixa) -> Faixa {
    // Decidido e decidido. Nem o atraso reabre: quem disse "ja entreguei" nao
    // quer ser lembrado de novo as 00h01.
    if item.decision.is_settled() {
        return Faixa::Historico;
    }
    if contexto.semestre_encerrado {
        return Faixa::Historico;
    }

    if item.horizonte == Horizonte::Overdue {
        if let Some(inicio) = contexto.semestre_comecou_em {
            if item.at < inicio {
                return Faixa::Historico;
            }
        }
    }

    match item.horizonte {
        Horizonte::Overdue | Horizonte::Today | Horizonte::Tomorrow => Faixa::Atencao,
        Horizonte::ThisWeek => {
            // A prova dos proximos tres dias sobe: ela nao se resolve em dez
            // minutos, e descobrir na vespera e o problema que o M/Academic
            // existe para evitar.
            if item.kind == "exam" && dias_ate(item.at, contexto.agora_local) <= DIAS_DE_PROVA_EM_ATENCAO
            {
                Faixa::Atencao
            } else {
                Faixa::Semana
            }
        }
        Horizonte::Later => Faixa::Depois,
    }
}

fn dias_ate(quando: OffsetDateTime, agora: OffsetDateTime) -> i64 {
    (quando - agora).whole_days().max(0)
}

/// O compromisso ja tem plano de execucao?
///
/// Derivado, e nao guardado: um estado "planejado" ao lado de `planned_at`
/// abriria a possibilidade de eles discordarem, e nada diria qual manda.
pub fn esta_planejado(item: &Compromisso) -> bool {
    item.planned_at.is_some() || item.task_id.is_some()
}

/// O plano de execucao que a pessoa escolheu.
#[derive(Clone, Copy, Debug)]
pub struct Plano {
    pub quando: OffsetDateTime,
    /// Minutos reservados. Zero significa "sem duracao definida".
    pub minutos: i64,
}

impl Plano {
    pub fn novo(quando: OffsetDateTime, minutos: i64) -> Result<Self, CoreError> {
        if minutos < 0 {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "A duracao planejada nao pode ser negativa.",
                false,
            ));
        }
        // Oito horas: acima disso nao e um bloco de estudo, e um erro de
        // digitacao. Recusar aqui e melhor que desenhar uma barra de dois dias
        // no calendario.
        if minutos > 8 * 60 {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Um bloco de estudo acima de oito horas provavelmente e engano.",
                false,
            ));
        }
        Ok(Self { quando, minutos })
    }

    pub fn fim(&self) -> OffsetDateTime {
        self.quando + time::Duration::minutes(self.minutos.max(0))
    }
}

/// O prazo tem hora de verdade?
///
/// # Por que a pergunta existe
///
/// O Univirtus manda `2026-08-24T23:59:00` — hora real, e ela importa: "vence
/// 23h59" e diferente de "vence hoje". Mas um compromisso criado a mao sem hora
/// vira meia-noite, e mostrar "24 ago · 00:00" afirmaria uma precisao que
/// ninguem informou.
///
/// A regra e simples e honesta: **meia-noite em ponto significa "sem hora"**.
/// Ela nao inventa 23h59 no lugar — o §10 do pedido recusa isso — e nao esconde
/// a hora quando ela existe.
pub fn tem_hora_real(quando: OffsetDateTime) -> bool {
    !(quando.hour() == 0 && quando.minute() == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::academic::Compromisso;
    use time::macros::datetime;

    fn item(kind: &str, at: OffsetDateTime, horizonte: Horizonte, decision: Decision) -> Compromisso {
        Compromisso {
            kind: kind.to_owned(),
            id: "x".into(),
            title: "APOL 3".into(),
            subject_id: "s".into(),
            subject: "Estatica".into(),
            subject_accent: String::new(),
            at,
            horizonte,
            task_id: None,
            location: String::new(),
            decision,
            planned_at: None,
            planned_minutes: 0,
        }
    }

    fn contexto(encerrado: bool) -> ContextoDaFaixa {
        ContextoDaFaixa {
            agora_local: datetime!(2026-08-22 13:00 UTC),
            semestre_encerrado: encerrado,
            semestre_comecou_em: Some(datetime!(2026-07-01 00:00 UTC)),
        }
    }

    #[test]
    fn atrasado_e_sem_decisao_pede_atencao() {
        let i = item(
            "assignment",
            datetime!(2026-08-20 23:59 UTC),
            Horizonte::Overdue,
            Decision::None,
        );
        assert_eq!(faixa_de(&i, contexto(false)), Faixa::Atencao);
    }

    /// A decisao vale mais que o atraso: quem disse "ja entreguei" nao volta
    /// para a fila.
    #[test]
    fn atrasado_e_concluido_vai_para_o_historico() {
        let i = item(
            "assignment",
            datetime!(2026-08-20 23:59 UTC),
            Horizonte::Overdue,
            Decision::Done,
        );
        assert_eq!(faixa_de(&i, contexto(false)), Faixa::Historico);
    }

    #[test]
    fn atrasado_e_descartado_vai_para_o_historico() {
        let i = item(
            "assignment",
            datetime!(2026-08-20 23:59 UTC),
            Horizonte::Overdue,
            Decision::Skipped,
        );
        assert_eq!(faixa_de(&i, contexto(false)), Faixa::Historico);
    }

    /// A regra que impede a tela de virar deposito: 151 dias de atraso num
    /// semestre fechado nao e urgencia.
    #[test]
    fn atrasado_de_semestre_encerrado_nao_ocupa_a_atencao() {
        let i = item(
            "assignment",
            datetime!(2026-03-23 23:59 UTC),
            Horizonte::Overdue,
            Decision::None,
        );
        assert_eq!(faixa_de(&i, contexto(true)), Faixa::Historico);
    }

    /// O caso real: o portal republica etapas com prazo de marco num semestre
    /// que comecou em julho. Isso nao e atraso — e resto de calendario antigo.
    #[test]
    fn prazo_anterior_ao_inicio_do_semestre_e_resto_e_nao_atraso() {
        let i = item(
            "assignment",
            datetime!(2026-03-23 23:59 UTC),
            Horizonte::Overdue,
            Decision::None,
        );
        assert_eq!(faixa_de(&i, contexto(false)), Faixa::Historico);
    }

    /// E o atraso de verdade — dentro do semestre — continua pedindo atencao.
    #[test]
    fn atraso_dentro_do_semestre_continua_sendo_atraso() {
        let i = item(
            "assignment",
            datetime!(2026-08-20 23:59 UTC),
            Horizonte::Overdue,
            Decision::None,
        );
        assert_eq!(faixa_de(&i, contexto(false)), Faixa::Atencao);
    }

    #[test]
    fn o_que_vence_hoje_e_amanha_pede_atencao() {
        for horizonte in [Horizonte::Today, Horizonte::Tomorrow] {
            let i = item(
                "assignment",
                datetime!(2026-08-22 23:59 UTC),
                horizonte,
                Decision::None,
            );
            assert_eq!(faixa_de(&i, contexto(false)), Faixa::Atencao);
        }
    }

    #[test]
    fn trabalho_da_semana_fica_na_semana() {
        let i = item(
            "assignment",
            datetime!(2026-08-27 23:59 UTC),
            Horizonte::ThisWeek,
            Decision::None,
        );
        assert_eq!(faixa_de(&i, contexto(false)), Faixa::Semana);
    }

    /// Prova em tres dias sobe para atencao; prova em seis fica na semana.
    #[test]
    fn a_prova_proxima_sobe_e_a_distante_nao() {
        let perto = item(
            "exam",
            datetime!(2026-08-24 20:00 UTC),
            Horizonte::ThisWeek,
            Decision::None,
        );
        assert_eq!(faixa_de(&perto, contexto(false)), Faixa::Atencao);

        let longe = item(
            "exam",
            datetime!(2026-08-28 20:00 UTC),
            Horizonte::ThisWeek,
            Decision::None,
        );
        assert_eq!(faixa_de(&longe, contexto(false)), Faixa::Semana);
    }

    #[test]
    fn o_que_e_de_setembro_fica_em_depois() {
        let i = item(
            "exam",
            datetime!(2026-09-14 23:59 UTC),
            Horizonte::Later,
            Decision::None,
        );
        assert_eq!(faixa_de(&i, contexto(false)), Faixa::Depois);
    }

    // -----------------------------------------------------------------------
    // Hora real
    // -----------------------------------------------------------------------

    /// 23h59 e hora de verdade e tem de aparecer. Meia-noite e ausencia de
    /// hora, e mostrar "00:00" afirmaria uma precisao que ninguem informou.
    #[test]
    fn meia_noite_significa_sem_hora_e_o_resto_e_hora_de_verdade() {
        assert!(tem_hora_real(datetime!(2026-08-24 23:59 UTC)));
        assert!(tem_hora_real(datetime!(2026-08-24 08:00 UTC)));
        assert!(!tem_hora_real(datetime!(2026-08-24 00:00 UTC)));
    }

    // -----------------------------------------------------------------------
    // Plano
    // -----------------------------------------------------------------------

    #[test]
    fn o_plano_recusa_duracao_absurda_e_negativa() {
        let quando = datetime!(2026-08-26 19:30 UTC);
        assert!(Plano::novo(quando, 60).is_ok());
        assert!(Plano::novo(quando, 0).is_ok());
        assert!(Plano::novo(quando, -1).is_err());
        assert!(Plano::novo(quando, 9 * 60).is_err());
    }

    #[test]
    fn o_fim_do_bloco_soma_a_duracao() {
        let plano = Plano::novo(datetime!(2026-08-26 19:30 UTC), 60).unwrap();
        assert_eq!(plano.fim(), datetime!(2026-08-26 20:30 UTC));
    }

    #[test]
    fn planejado_e_derivado_da_data_ou_da_task() {
        let mut i = item(
            "assignment",
            datetime!(2026-08-27 23:59 UTC),
            Horizonte::ThisWeek,
            Decision::None,
        );
        assert!(!esta_planejado(&i));
        i.planned_at = Some(datetime!(2026-08-26 19:30 UTC));
        assert!(esta_planejado(&i));
        i.planned_at = None;
        i.task_id = Some("task-1".into());
        assert!(esta_planejado(&i));
    }

    #[test]
    fn a_decisao_do_banco_e_lida_e_a_desconhecida_e_recusada() {
        assert_eq!(Decision::parse("done").unwrap(), Decision::Done);
        assert_eq!(Decision::parse("").unwrap(), Decision::None);
        assert!(Decision::parse("planejado").is_err());
    }
}
