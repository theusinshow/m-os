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
}

/// Decide o que avisar, sem tocar em nada.
///
/// Devolve pares `(chave, aviso)`: a chave e o que o `push.db` usa para nao
/// repetir, e ela inclui o vencimento — um lembrete que se repete vence de novo
/// e precisa avisar de novo.
pub fn o_que_avisar(lembretes: &[Reminder], agora: OffsetDateTime) -> Vec<(String, Aviso)> {
    lembretes
        .iter()
        .filter_map(|lembrete| {
            let vencimento = lembrete.next_due_at?;
            lembrete.overdue_by(agora)?;
            Some((
                format!("lembrete:{}:{}", lembrete.id, vencimento.unix_timestamp()),
                Aviso {
                    titulo: lembrete.title.clone(),
                    // O corpo do lembrete costuma ser vazio, e uma notificacao
                    // com segunda linha em branco parece defeito. Nesse caso a
                    // segunda linha diz o que a pessoa quer saber de qualquer
                    // forma: que isto venceu.
                    corpo: if lembrete.body.trim().is_empty() {
                        String::from("Venceu agora.")
                    } else {
                        lembrete.body.clone()
                    },
                    tag: format!("lembrete-{}", lembrete.id),
                    url: String::from("/"),
                },
            ))
        })
        .collect()
}

/// O aviso de que o PC mandou coisa.
///
/// Ele diz **quantos**, e nao o que: o motor de sync devolve contagem, e
/// inventar um titulo a partir de um numero seria a notificacao mentindo sobre
/// o que ela sabe.
pub fn chegou_do_pc(recebidas: usize) -> Aviso {
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
        let lembretes = match attention.waiting() {
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
    #[test]
    fn corpo_vazio_vira_uma_frase_util() {
        let agora = instante(1_000_000);
        let avisos = o_que_avisar(&[lembrete("Titulo", "   ", instante(999_000))], agora);
        assert_eq!(avisos[0].1.corpo, "Venceu agora.");
    }

    #[test]
    fn o_aviso_do_sync_concorda_com_o_singular() {
        assert!(chegou_do_pc(1).corpo.contains("1 item novo"));
        assert!(chegou_do_pc(3).corpo.contains("3 itens novos"));
        assert_eq!(
            chegou_do_pc(1).tag,
            chegou_do_pc(9).tag,
            "mesma tag, um cartao so"
        );
    }
}
