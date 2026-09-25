//! O perfil do Hermes como piloto do M/OS (ADR-087).
//!
//! Duas queixas de uso real motivaram isto: o Hermes perguntava "o que é
//! M-Finance?" — ninguém tinha contado a ele o que o M/OS tem — e respondia
//! pedido claro com "quer que eu crie a task?", porque o contrato mandava toda
//! ação virar proposta para confirmar.
//!
//! Aqui moram as três peças que resolvem isso, puras e testáveis:
//!
//! 1. **o manual** — os módulos do M/OS, o que cada um é, e o que está ligado
//!    NESTE aparelho agora;
//! 2. **a autonomia** — até onde o Hermes age sem pedir licença;
//! 3. **a conduta** — pedido claro se faz; só se pergunta o que falta.
//!
//! A autonomia só é segura porque a execução já é: reserva que não executa duas
//! vezes, desfazer completo e recibo de cada ação (ADR-074). O que não tem
//! volta ou mexe fora do M/OS continua pedindo confirmação em qualquer nível.

use serde::{Deserialize, Serialize};

use crate::ActionKind;

/// Até onde o Hermes age sozinho.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NivelDeAutonomia {
    /// Toda ação vira proposta para confirmar. Era o único modo até a ADR-087.
    Perguntar,
    /// Executa sozinho o que tem desfazer; pergunta o resto. O padrão.
    #[default]
    Reversivel,
    /// Executa tudo que é do M/OS; só o que sai dele (M-Finance) pede licença.
    Tudo,
}

impl NivelDeAutonomia {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Perguntar => "perguntar",
            Self::Reversivel => "reversivel",
            Self::Tudo => "tudo",
        }
    }

    /// Valor desconhecido vira o padrão: uma preferência gravada por versão
    /// mais nova não pode travar o chat.
    pub fn parse(valor: &str) -> Self {
        match valor {
            "perguntar" => Self::Perguntar,
            "tudo" => Self::Tudo,
            _ => Self::Reversivel,
        }
    }
}

/// Se a ação tem caminho de volta pelo recibo. Espelha o `UndoStep` que o
/// executor devolve para cada tipo (`agente_exec`); o teste abaixo trava a
/// lista para ela não divergir em silêncio.
fn tem_desfazer(kind: ActionKind) -> bool {
    use ActionKind as K;
    matches!(
        kind,
        K::CaptureCreate
            | K::CaptureToTask
            | K::TaskCreate
            | K::TaskSetState
            | K::TaskStart
            | K::TaskSetProject
            | K::TaskAddChecklist
            | K::TaskCheckItem
            | K::TaskSetPlan
            | K::ProjectCreate
            | K::ResourceCreate
            | K::ReminderCreate
            | K::TimeStop
            | K::TimeRecord
            | K::DayAddObjective
            | K::DaySetObjective
            | K::DaySetMain
    )
}

/// Se o M/OS executa esta ação sem mostrar o cartão de confirmação.
pub fn executa_sozinho(nivel: NivelDeAutonomia, kind: ActionKind) -> bool {
    match nivel {
        NivelDeAutonomia::Perguntar => false,
        NivelDeAutonomia::Reversivel => tem_desfazer(kind),
        // Efeito externo é outra conta: uma conta a pagar criada por engano não
        // se desfaz daqui.
        NivelDeAutonomia::Tudo => kind != ActionKind::MFinanceCreateBill,
    }
}

/// O que está disponível NESTE aparelho agora. O manual diz a verdade sobre o
/// que dá para fazer, e não uma lista de desejos.
#[derive(Clone, Copy, Debug, Default)]
pub struct Disponibilidade {
    pub m_finance: bool,
    pub academico: bool,
    pub reunioes: bool,
    pub sync: bool,
}

/// O manual do M/OS para o Hermes: o que cada módulo é, em uma linha, e se
/// está ligado aqui.
pub fn manual_do_mos(d: &Disponibilidade) -> String {
    let ligado = |sim: bool| {
        if sim {
            "ligado aqui"
        } else {
            "desligado neste aparelho"
        }
    };
    format!(
        "[O M/OS]\n\
         O M/OS é o sistema pessoal de trabalho do usuário (engenharia estrutural, obras, estudo). Módulos:\n\
         - Home/Piloto: o que fazer agora, o dia (começar/encerrar, objetivos) e o que precisa de atenção.\n\
         - Inbox: Captures — tudo que entra antes de ser organizado (texto, voz, link, arquivo).\n\
         - Tasks: o trabalho executável, com estado (Kanban), prazo, dia planejado, prioridade, checklist, espera por alguém e dependência.\n\
         - Projects e Workspaces: agrupam Tasks, Resources e reuniões.\n\
         - Library: Resources — links, notas e arquivos de referência.\n\
         - Lembretes: avisos com hora, recorrência e insistência.\n\
         - Tempo (antigo CronoCAD): cronômetro e horas lançadas por Project, com valor/hora.\n\
         - Reuniões: gravação, transcrição, resumo, decisões e itens que viram Task ({reunioes}).\n\
         - Academic: semestre, disciplinas, provas e entregas da faculdade ({academico}).\n\
         - M-Finance: o app de finanças do usuário, separado do M/OS; daqui só se cria conta a pagar ({finance}).\n\
         - Sync: o mesmo M/OS no outro PC e no celular ({sync}).\n\
         Quando o usuário citar um desses nomes, é o módulo do M/OS — não pergunte o que é.\n\
         [Fim do M/OS]\n\n",
        reunioes = ligado(d.reunioes),
        academico = ligado(d.academico),
        finance = ligado(d.m_finance),
        sync = ligado(d.sync),
    )
}

/// Como o Hermes se comporta, conforme a autonomia.
pub fn conduta(nivel: NivelDeAutonomia) -> String {
    let agir = match nivel {
        NivelDeAutonomia::Perguntar => {
            "Toda ação vira um cartão que o usuário confirma. Proponha direto — não pergunte \
             em texto \"quer que eu faça?\"; o cartão já é a pergunta."
        }
        NivelDeAutonomia::Reversivel => {
            "O usuário te deu autonomia: ações com desfazer (criar/mover/começar Task, \
             checklist, capture, lembrete, projeto, resource, objetivo do dia, lançar horas) \
             o M/OS EXECUTA assim que você as propõe, e mostra recibo com Desfazer. As outras \
             viram cartão para confirmar."
        }
        NivelDeAutonomia::Tudo => {
            "O usuário te deu autonomia total dentro do M/OS: toda ação que você propuser é \
             EXECUTADA na hora, com recibo. Só o que sai do M/OS (M-Finance) pede confirmação."
        }
    };
    format!(
        "[Como agir]\n\
         Pedido claro se FAZ: inclua a ação e diga em uma frase o que foi feito. Não \
         pergunte \"quer que eu crie?\", não explique o caminho na interface, não liste \
         opções quando só uma faz sentido.\n\
         Pergunte só quando faltar um dado que você não tem como deduzir, ou quando a \
         referência for ambígua (duas Tasks com o mesmo nome) — e aí pergunte UMA coisa.\n\
         {agir}\n\
         [Fim de como agir]\n\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perguntar_nunca_executa_e_tudo_nunca_executa_o_externo() {
        for kind in ActionKind::all() {
            assert!(!executa_sozinho(NivelDeAutonomia::Perguntar, kind));
        }
        assert!(!executa_sozinho(
            NivelDeAutonomia::Tudo,
            ActionKind::MFinanceCreateBill
        ));
        assert!(executa_sozinho(NivelDeAutonomia::Tudo, ActionKind::DayEnd));
    }

    #[test]
    fn reversivel_executa_so_o_que_tem_desfazer() {
        assert!(executa_sozinho(
            NivelDeAutonomia::Reversivel,
            ActionKind::TaskCreate
        ));
        assert!(executa_sozinho(
            NivelDeAutonomia::Reversivel,
            ActionKind::TaskStart
        ));
        // Sem caminho de volta no recibo: continuam pedindo confirmação.
        for kind in [
            ActionKind::ReminderSnooze,
            ActionKind::ReminderResolve,
            ActionKind::TimeStart,
            ActionKind::DayStart,
            ActionKind::DayEnd,
            ActionKind::TaskPlan,
            ActionKind::MFinanceCreateBill,
        ] {
            assert!(
                !executa_sozinho(NivelDeAutonomia::Reversivel, kind),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn preferencia_desconhecida_vira_o_padrao() {
        assert_eq!(NivelDeAutonomia::parse("xyz"), NivelDeAutonomia::Reversivel);
        for nivel in [
            NivelDeAutonomia::Perguntar,
            NivelDeAutonomia::Reversivel,
            NivelDeAutonomia::Tudo,
        ] {
            assert_eq!(NivelDeAutonomia::parse(nivel.as_str()), nivel);
        }
    }

    #[test]
    fn o_manual_diz_o_que_e_m_finance_e_se_esta_ligado() {
        let manual = manual_do_mos(&Disponibilidade {
            m_finance: false,
            ..Default::default()
        });
        assert!(manual.contains("M-Finance: o app de finanças"));
        assert!(manual.contains("desligado neste aparelho"));
    }
}
