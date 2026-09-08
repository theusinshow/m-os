//! O que merece te interromper — e o que faz isso acontecer.
//!
//! # A fronteira que este arquivo NAO cruza
//!
//! Ele **le** lembretes e **nao escreve** nenhum. Marcar um Reminder como
//! entregue e uma escrita de dominio, ela sincronizaria para o PC, e o desktop
//! tem o proprio agendador olhando os mesmos lembretes: dois aparelhos
//! disputando o mesmo estado produziria o lembrete que some do PC porque o
//! celular achou que ja tinha dado conta.
//!
//! O que ja foi avisado mora no `push.db`, que e local e nao sincroniza — ver
//! `assinaturas.rs`. O aviso e uma decisao DESTE aparelho sobre a propria tela;
//! o lembrete continua sendo do M/OS.
//!
//! A fronteira e sobre ENTREGA, e nao sobre escrita em geral: o `api.rs` cria,
//! conclui e cancela lembretes, porque essas tres sao a pessoa decidindo uma vez
//! num aparelho so — e as tres levam a estado que nenhum agendador disputa. Ver
//! a secao de lembretes la.
//!
//! # Por que a decisao e uma funcao pura
//!
//! [`o_que_avisar`] recebe lembretes e um instante, e devolve avisos. Sem rede,
//! sem banco, sem relogio. E o que permite testar "venceu ha um minuto avisa" e
//! "vence daqui a uma hora nao avisa" sem VPS, sem iPhone e sem esperar.

use std::sync::Arc;
use std::time::Duration;

use mos_core::{AttentionService, Reminder};
use serde::Serialize;
use time::OffsetDateTime;

use crate::assinaturas::Assinaturas;
use crate::push::{self, Entrega, Vapid};

/// De quanto em quanto tempo os lembretes sao conferidos.
///
/// Um minuto, o mesmo passo do sync. Um lembrete que chega com ate um minuto de
/// atraso ninguem percebe; um laco mais apertado gastaria bateria da VPS para
/// ganhar segundos que nao mudam nada.
const INTERVALO: Duration = Duration::from_secs(60);

/// Por quanto tempo a memoria do que ja foi avisado e mantida.
const MEMORIA_MS: i64 = 30 * 24 * 60 * 60 * 1000;

/// Uma notificacao, do jeito que o service worker vai ler.
///
/// Os campos sao poucos de proposito: o que cabe numa tela de bloqueio e um
/// titulo, uma linha e para onde ir ao tocar. Tudo isso viaja **cifrado** — ver
/// `push.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Aviso {
    pub titulo: String,
    pub corpo: String,
    /// Notificacoes com a mesma `tag` se SUBSTITUEM em vez de empilhar. Com o id
    /// do lembrete aqui, um lembrete que avisa de novo troca o cartao antigo em
    /// vez de virar o segundo de uma pilha.
    pub tag: String,
    /// Para onde o toque leva.
    pub url: String,
    /// Quantos lembretes cobram acao NESTE instante — o numero do badge do
    /// icone na tela de inicio.
    ///
    /// `None` significa "nao sei", e o service worker entao nao encosta no
    /// badge. E o caso do aviso de sync disparado de um lugar que nao tem a
    /// lista de lembretes na mao: zerar o badge ali seria inventar um numero.
    pub badge: Option<u32>,
}

/// Quantos lembretes cobram acao agora.
///
/// # Por que estes tres estados, e nao "vencido"
///
/// E a MESMA regra do `pedeAtencao` do front (`api.ts`), e ela tem que ser a
/// mesma: o badge do icone e o badge da barra de baixo mostram o mesmo numero, e
/// duas contagens diferentes para a mesma pergunta fariam uma das duas mentir.
///
/// `Scheduled` fica de fora de proposito — um badge que sobe com coisa que ainda
/// nao e hora e um badge que se aprende a ignorar.
pub fn quantos_cobram(lembretes: &[Reminder]) -> u32 {
    lembretes
        .iter()
        .filter(|lembrete| {
            matches!(
                lembrete.status,
                mos_core::ReminderStatus::Due
                    | mos_core::ReminderStatus::Delivered
                    | mos_core::ReminderStatus::Missed
            )
        })
        .count() as u32
}

/// Decide o que avisar, sem tocar em nada.
///
/// Devolve pares `(chave, aviso)`: a chave e o que o `push.db` usa para nao
/// repetir, e ela inclui o vencimento — um lembrete que se repete vence de novo
/// e precisa avisar de novo.
pub fn o_que_avisar(lembretes: &[Reminder], agora: OffsetDateTime) -> Vec<(String, Aviso)> {
    // A contagem e sobre a lista INTEIRA, e nao sobre os que estao sendo
    // avisados agora: com tres vencidos e so um por avisar, o badge diria "1" e
    // o icone estaria mentindo sobre o tamanho do problema.
    let cobrando = quantos_cobram(lembretes);
    lembretes
        .iter()
        .filter(|lembrete| lembrete.lifecycle_state == mos_core::LifecycleState::Active)
        .filter(|lembrete| !lembrete.status.is_terminal())
        .filter_map(|lembrete| {
            let vencimento = lembrete.next_due_at?;
            lembrete.overdue_by(agora)?;

            // O degrau da insistencia, DERIVADO — nunca escrito.
            //
            // A fronteira do topo deste arquivo continua valendo: este aparelho
            // le lembretes e nao escreve nenhum. `alert_slot` e como ele chega
            // ao mesmo degrau que o agendador do PC alcancaria, a partir do
            // vencimento e do relogio, sem tocar em `escalation_step`.
            //
            // Um lembrete comum tem degrau zero para sempre, e continua sendo
            // avisado uma vez so. Um persistente ganha ate tres, nos mesmos
            // 30 min / 1 h / 2 h que o PC usa — e e por virem da mesma funcao
            // que os dois nunca discordam.
            let degrau = mos_core::alert_slot(lembrete, agora);

            Some((
                format!(
                    "lembrete:{}:{}:{degrau}",
                    lembrete.id,
                    vencimento.unix_timestamp()
                ),
                Aviso {
                    titulo: lembrete.title.clone(),
                    corpo: corpo_do_aviso(lembrete, degrau, agora),
                    tag: format!("lembrete-{}", lembrete.id),
                    url: String::from("/"),
                    badge: Some(cobrando),
                },
            ))
        })
        .collect()
}

/// A segunda linha da notificacao.
///
/// O corpo do lembrete costuma ser vazio, e uma notificacao com segunda linha em
/// branco parece defeito. O que entra no lugar depende do que a pessoa precisa
/// saber para decidir se levanta agora:
///
/// - um follow-up pergunta pela PESSOA, porque "concluir" nao e a resposta que
///   ele quer;
/// - uma insistencia diz que isto CONTINUA pendente, e nao que venceu agora —
///   repetir "venceu agora" tres horas depois seria mentir sobre o atraso;
/// - o resto diz ha quanto tempo passou.
fn corpo_do_aviso(lembrete: &Reminder, degrau: u32, agora: OffsetDateTime) -> String {
    if lembrete.kind == mos_core::ReminderKind::FollowUp {
        let quem = lembrete.waiting_for.trim();
        return if quem.is_empty() {
            String::from("Ja respondeu?")
        } else {
            format!("{quem} respondeu?")
        };
    }
    if !lembrete.body.trim().is_empty() {
        return lembrete.body.clone();
    }
    if degrau > 0 {
        return format!(
            "Continua pendente — {}.",
            atraso_em_palavras(lembrete, agora)
        );
    }
    match lembrete.overdue_by(agora) {
        Some(atraso) if atraso.whole_minutes() >= 5 => {
            format!("Venceu {}.", atraso_em_palavras(lembrete, agora))
        }
        _ => String::from("Venceu agora."),
    }
}

fn atraso_em_palavras(lembrete: &Reminder, agora: OffsetDateTime) -> String {
    let Some(atraso) = lembrete.overdue_by(agora) else {
        return String::from("agora");
    };
    if atraso.whole_hours() < 1 {
        format!("ha {} min", atraso.whole_minutes())
    } else if atraso.whole_days() < 1 {
        format!("ha {} h", atraso.whole_hours())
    } else {
        format!("ha {} d", atraso.whole_days())
    }
}

/// O aviso de que o PC mandou coisa.
///
/// Ele diz **quantos**, e nao o que: o motor de sync devolve contagem, e
/// inventar um titulo a partir de um numero seria a notificacao mentindo sobre
/// o que ela sabe.
pub fn chegou_do_pc(recebidas: usize, cobrando: Option<u32>) -> Aviso {
    Aviso {
        titulo: String::from("M/OS"),
        corpo: if recebidas == 1 {
            String::from("1 item novo veio do computador.")
        } else {
            format!("{recebidas} itens novos vieram do computador.")
        },
        // Tag fixa: dois lotes seguidos trocam o mesmo cartao em vez de encher a
        // tela de bloqueio com uma pilha de "veio coisa".
        tag: String::from("sync"),
        url: String::from("/"),
        badge: cobrando,
    }
}

/// Quanto tempo esperar por um servico de push antes de desistir.
const TIMEOUT: Duration = Duration::from_secs(15);

/// Quem sabe mandar.
///
/// Note o que ele NAO guarda: o cliente HTTP. Um `reqwest::blocking::Client`
/// carrega um runtime proprio, e construi-lo ou solta-lo de dentro de um
/// contexto assincrono derruba o processo com "cannot drop a runtime in a
/// context where blocking is not allowed" — e o `Avisador` nasce dentro do
/// `#[tokio::main]`. O cliente entao nasce dentro do `spawn_blocking`, que e
/// exatamente o que o `mos-sync-http` ja fazia por este mesmo motivo.
pub struct Avisador {
    assinaturas: Arc<Assinaturas>,
    vapid: Arc<Vapid>,
}

impl Avisador {
    pub fn novo(assinaturas: Arc<Assinaturas>, vapid: Arc<Vapid>) -> Self {
        Self { assinaturas, vapid }
    }

    /// Manda um aviso para todos os aparelhos assinados.
    ///
    /// Bloqueante — chame de dentro de `spawn_blocking`. Devolve quantos
    /// aceitaram.
    pub fn disparar(&self, aviso: &Aviso) -> usize {
        let cliente = match reqwest::blocking::Client::builder()
            .timeout(TIMEOUT)
            .build()
        {
            Ok(cliente) => cliente,
            Err(causa) => {
                eprintln!("[push] cliente HTTP: {causa}");
                return 0;
            }
        };
        let assinaturas = match self.assinaturas.todas() {
            Ok(assinaturas) => assinaturas,
            Err(causa) => {
                eprintln!("[push] nao consegui ler as assinaturas: {causa}");
                return 0;
            }
        };
        let texto = serde_json::to_vec(aviso).expect("um Aviso sempre vira JSON");
        let agora = OffsetDateTime::now_utc().unix_timestamp();
        let mut aceitos = 0;
        for assinatura in assinaturas {
            match push::enviar(&cliente, &self.vapid, &assinatura, &texto, agora) {
                Ok(Entrega::Aceita) => aceitos += 1,
                Ok(Entrega::Morta) => {
                    // O fabricante declarou o endpoint morto. Apagar aqui e o
                    // que impede uma ida a rede por minuto para sempre.
                    println!("[push] assinatura morta, removendo");
                    let _ = self.assinaturas.remover(&assinatura.endpoint);
                }
                Err(causa) => eprintln!("[push] falhou: {causa}"),
            }
        }
        aceitos
    }

    /// Uma passada: confere lembretes vencidos e avisa os que faltam.
    pub fn passada(&self, attention: &AttentionService, agora: OffsetDateTime) {
        // As horas de silencio DESTE aparelho. Nao sincronizam: silenciar o
        // celular a noite nao pode silenciar o PC do escritorio, e o contrario
        // tambem nao.
        //
        // Segura a NOTIFICACAO, e nao o lembrete: nada e marcado como avisado
        // enquanto o silencio dura, entao a mesma notificacao sai inteira quando
        // a janela terminar — com o atraso dito no corpo, que e o que o §33
        // pede.
        match attention.settings() {
            Ok(ajustes) => {
                // Pergunta pelo TOPO da escala: se nem `Urgent` pode sair, nada
                // pode. Com `allow_urgent` desligado — o default — isso vale
                // para a janela inteira.
                if ajustes
                    .quiet
                    .defer(agora, ajustes.offset(), mos_core::Priority::Urgent)
                    .is_some()
                {
                    return;
                }
            }
            Err(causa) => {
                // Sem ajustes legiveis, avisa. Perder um lembrete por causa de
                // uma configuracao ilegivel seria a falha que o sistema inteiro
                // existe para nao ter.
                eprintln!("[push] ajustes ilegiveis: {}", causa.message);
            }
        }

        // `open` e nao `waiting`: um lembrete que o PC ja marcou como `due` ou
        // `missed` chega aqui pelo sync nesse estado, e `waiting` so devolve o
        // que ainda espera. Com `waiting`, o celular ficava calado justamente
        // sobre o que ja estava atrasado.
        let lembretes = match attention.open() {
            Ok(lembretes) => lembretes,
            Err(causa) => {
                eprintln!("[push] nao consegui ler os lembretes: {}", causa.message);
                return;
            }
        };
        let agora_ms = (agora.unix_timestamp_nanos() / 1_000_000) as i64;
        for (chave, aviso) in o_que_avisar(&lembretes, agora) {
            // A marca vem ANTES do envio de proposito. Ao contrario, uma falha
            // depois de a notificacao ter saido faria ela sair de novo no
            // minuto seguinte — e notificacao repetida custa mais caro que
            // notificacao perdida.
            match self.assinaturas.avisar_uma_vez(&chave, agora_ms) {
                Ok(true) => {
                    self.disparar(&aviso);
                }
                Ok(false) => {}
                Err(causa) => eprintln!("[push] {causa}"),
            }
        }
        let _ = self.assinaturas.esquecer_antes_de(agora_ms - MEMORIA_MS);
    }
}

/// O laco dos lembretes.
pub fn iniciar(avisador: Arc<Avisador>, attention: Arc<AttentionService>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(INTERVALO).await;
            let avisador = Arc::clone(&avisador);
            let attention = Arc::clone(&attention);
            // `spawn_blocking` porque o envio e bloqueante, igual ao sync.
            let _ = tokio::task::spawn_blocking(move || {
                avisador.passada(&attention, OffsetDateTime::now_utc());
            })
            .await;
        }
    });
}

#[cfg(test)]
mod testes {
    use super::*;
    use mos_core::{
        DeliveryPolicy, LifecycleState, Priority, ReminderId, ReminderSource, ReminderStatus,
        Trigger,
    };

    fn lembrete(titulo: &str, corpo: &str, vence: OffsetDateTime) -> Reminder {
        let criado = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        Reminder {
            id: ReminderId::new(),
            title: String::from(titulo),
            body: String::from(corpo),
            target: None,
            trigger: Trigger::At { instant: vence },
            priority: Priority::Normal,
            status: ReminderStatus::Scheduled,
            policy: DeliveryPolicy::default(),
            source: ReminderSource::User,
            next_due_at: Some(vence),
            snooze_count: 0,
            delivered_count: 0,
            kind: mos_core::ReminderKind::Standard,
            waiting_for: String::new(),
            persistent: false,
            escalation_step: 0,
            last_triggered_at: None,
            retry_at: None,
            recurrence: None,
            created_at: criado,
            updated_at: criado,
            completed_at: None,
            lifecycle_state: LifecycleState::Active,
        }
    }

    fn instante(segundos: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(segundos).unwrap()
    }

    #[test]
    fn o_que_venceu_avisa() {
        let agora = instante(1_000_000);
        let lembretes = vec![lembrete("Ligar pro dentista", "", instante(999_940))];
        let avisos = o_que_avisar(&lembretes, agora);
        assert_eq!(avisos.len(), 1);
        assert_eq!(avisos[0].1.titulo, "Ligar pro dentista");
    }

    #[test]
    fn o_que_ainda_nao_venceu_fica_quieto() {
        let agora = instante(1_000_000);
        let lembretes = vec![lembrete("Daqui a pouco", "", instante(1_003_600))];
        assert!(o_que_avisar(&lembretes, agora).is_empty());
    }

    /// Um lembrete sem vencimento (gatilho por contexto, e nao por hora) nao
    /// tem quando avisar — e chutar um horario seria interromper por engano.
    #[test]
    fn lembrete_sem_vencimento_nao_avisa() {
        let agora = instante(1_000_000);
        let mut sem_hora = lembrete("Quando eu abrir o CAD", "", instante(1));
        sem_hora.next_due_at = None;
        assert!(o_que_avisar(&[sem_hora], agora).is_empty());
    }

    /// A chave carrega o vencimento, e e isso que faz a proxima ocorrencia de um
    /// lembrete repetido avisar de novo em vez de ficar muda para sempre.
    #[test]
    fn a_chave_muda_quando_o_vencimento_muda() {
        let agora = instante(1_000_000);
        let mut um = lembrete("Remedio", "", instante(999_000));
        let chave_um = o_que_avisar(&[um.clone()], agora)[0].0.clone();
        um.next_due_at = Some(instante(999_500));
        let chave_dois = o_que_avisar(&[um], agora)[0].0.clone();
        assert_ne!(chave_um, chave_dois);
    }

    /// Um corpo vazio viraria uma segunda linha em branco na tela de bloqueio, e
    /// isso parece defeito do app.
    ///
    /// A frase que entra no lugar diz o TAMANHO do atraso quando ele existe:
    /// "venceu agora" numa notificacao de dezesseis minutos atras seria o
    /// aparelho mentindo sobre a unica informacao que muda o que se faz a
    /// seguir.
    #[test]
    fn corpo_vazio_vira_uma_frase_util() {
        let agora = instante(1_000_000);
        // Um minuto de atraso: dentro da margem em que "agora" e verdade.
        let recem = o_que_avisar(&[lembrete("Titulo", "   ", instante(999_940))], agora);
        assert_eq!(recem[0].1.corpo, "Venceu agora.");

        // Dezesseis minutos: a frase diz quanto.
        let atrasado = o_que_avisar(&[lembrete("Titulo", "   ", instante(999_000))], agora);
        assert_eq!(atrasado[0].1.corpo, "Venceu ha 16 min.");
    }

    /// Um follow-up pergunta pela PESSOA. "Concluir" nao e a resposta que ele
    /// quer, e a notificacao nao pode fingir que e.
    #[test]
    fn um_follow_up_pergunta_pela_pessoa() {
        let agora = instante(1_000_000);
        let mut cobranca = lembrete("Base estrutural", "", instante(999_000));
        cobranca.kind = mos_core::ReminderKind::FollowUp;
        cobranca.waiting_for = "Victor".into();
        let avisos = o_que_avisar(&[cobranca], agora);
        assert_eq!(avisos[0].1.corpo, "Victor respondeu?");
    }

    /// A insistencia so existe para quem pediu, e ela usa os mesmos intervalos
    /// do agendador do PC. Um lembrete comum avisa uma vez e pronto.
    #[test]
    fn so_o_persistente_ganha_um_segundo_aviso() {
        let venceu = instante(1_000_000);
        let comum = lembrete("Alongar", "", venceu);
        let mut insistente = lembrete("Enviar as bases", "", venceu);
        insistente.persistent = true;

        let chave = |lembrete: &Reminder, agora| {
            o_que_avisar(std::slice::from_ref(lembrete), agora)[0]
                .0
                .clone()
        };

        // Um minuto depois do vencimento: o primeiro aviso de cada um. Nao no
        // proprio instante — `overdue_by` e estrito, e no segundo exato do
        // vencimento ainda nao ha atraso nenhum.
        let primeiro = venceu + time::Duration::minutes(1);

        // Vinte e nove minutos: nenhum dos dois mudou de chave.
        let cedo = venceu + time::Duration::minutes(29);
        assert_eq!(chave(&comum, primeiro), chave(&comum, cedo));
        assert_eq!(chave(&insistente, primeiro), chave(&insistente, cedo));

        // Trinta e um: so o persistente ganha uma chave nova, e portanto so ele
        // volta a tocar.
        let depois = venceu + time::Duration::minutes(31);
        assert_eq!(chave(&comum, primeiro), chave(&comum, depois));
        assert_ne!(chave(&insistente, primeiro), chave(&insistente, depois));
    }

    /// O que ja foi resolvido nao avisa mais — mesmo que a linha ainda esteja na
    /// lista que veio do banco.
    #[test]
    fn nada_terminal_avisa() {
        let agora = instante(1_000_000);
        let mut feito = lembrete("Ja fiz", "", instante(999_000));
        feito.status = mos_core::ReminderStatus::Completed;
        feito.completed_at = Some(instante(999_500));
        assert!(o_que_avisar(&[feito], agora).is_empty());
    }

    #[test]
    fn o_aviso_do_sync_concorda_com_o_singular() {
        assert!(chegou_do_pc(1, None).corpo.contains("1 item novo"));
        assert!(chegou_do_pc(3, None).corpo.contains("3 itens novos"));
        assert_eq!(
            chegou_do_pc(1, None).tag,
            chegou_do_pc(9, None).tag,
            "mesma tag, um cartao so"
        );
    }

    /// Tres vencidos e um so por avisar: o badge tem que dizer TRES.
    ///
    /// A contagem e sobre a lista inteira, e nao sobre o que esta saindo agora
    /// — senao o icone diria "1" enquanto ha tres coisas cobrando, que e a
    /// forma mais silenciosa de o badge mentir.
    #[test]
    fn o_badge_conta_todos_os_que_cobram_e_nao_so_o_que_vai_sair() {
        let agora = instante(1_000_000);
        let mut vencidos = vec![
            lembrete("Um", "", instante(999_000)),
            lembrete("Dois", "", instante(999_100)),
            lembrete("Tres", "", instante(999_200)),
        ];
        for l in &mut vencidos {
            l.status = ReminderStatus::Due;
        }
        let avisos = o_que_avisar(&vencidos, agora);
        assert_eq!(avisos.len(), 3);
        for (_, aviso) in &avisos {
            assert_eq!(aviso.badge, Some(3));
        }
    }

    /// Agendado nao cobra nada: um badge que sobe com coisa que ainda nao e
    /// hora e um badge que se aprende a ignorar.
    #[test]
    fn agendado_nao_entra_no_badge() {
        let agendado = lembrete("Semana que vem", "", instante(2_000_000));
        assert_eq!(quantos_cobram(&[agendado]), 0);
    }

    /// `missed` e `delivered` cobram tanto quanto `due` — sao as mesmas tres
    /// situacoes que o `pedeAtencao` do front conta.
    #[test]
    fn perdido_e_entregue_tambem_cobram() {
        let mut perdido = lembrete("Perdi", "", instante(900_000));
        perdido.status = ReminderStatus::Missed;
        let mut entregue = lembrete("Chegou", "", instante(900_000));
        entregue.status = ReminderStatus::Delivered;
        assert_eq!(quantos_cobram(&[perdido, entregue]), 2);
    }
}
