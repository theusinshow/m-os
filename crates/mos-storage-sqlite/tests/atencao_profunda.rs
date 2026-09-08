//! O Attention System inteiro, contra o banco de verdade.
//!
//! Os testes de `attention.rs` provam as regras; estes provam que elas
//! atravessam a persistência. A diferença importa: uma máquina de estados
//! correta que perde o campo ao gravar é um lembrete que muda de comportamento
//! sozinho depois do primeiro restart.
//!
//! Cada teste aqui é um dos cenários do pedido, com o número da seção ao lado.

use std::sync::Arc;

use mos_core::{
    AttentionService, Clock, CreateReminder, DueReason, FixedClock, Priority, ReminderStatus,
};
use mos_storage_sqlite::SqliteStorage;
use time::{Duration, OffsetDateTime};

fn montar() -> (AttentionService, FixedClock, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap(),
    );
    // Longe do epoch e no meio da tarde: dentro do epoch, "ontem" não existe, e
    // de madrugada as horas de silêncio engoliriam metade dos casos.
    let clock =
        FixedClock::at(OffsetDateTime::UNIX_EPOCH + Duration::days(20_000) + Duration::hours(15));
    let service = AttentionService::new(storage, Arc::new(clock.clone()));
    (service, clock, directory)
}

/// O silêncio padrão vai de meia-noite às oito, e quase todo teste aqui roda de
/// tarde. Quando o teste precisa atravessar isso, ele desliga explicitamente.
fn sem_silencio(service: &AttentionService) {
    let mut ajustes = service.settings().unwrap();
    ajustes.quiet.enabled = false;
    service.save_settings(ajustes).unwrap();
}

/// Os testes raciocinam em UTC; o default do banco e o horario de Brasilia.
/// Deixar o default valendo faria "as tres da manha" do teste ser meia-noite do
/// silencio, e o teste passaria ou falharia por um motivo que ele nao investiga.
fn fuso_utc(service: &AttentionService) {
    let mut ajustes = service.settings().unwrap();
    ajustes.local_offset_minutes = 0;
    service.save_settings(ajustes).unwrap();
}

// ---------------------------------------------------------------- §4, §8, §62

/// **O cenário do pedido, do começo ao fim.**
///
/// "Me lembra hoje às 20:30 de enviar as bases para o Victor e não me deixa
/// esquecer." Vence, notifica, é ignorado — e continua pendente. Depois insiste.
/// Depois para de insistir e continua pendente.
#[test]
fn um_lembrete_persistente_ignorado_continua_pendente() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);

    let criado = service
        .create(
            CreateReminder::at(
                "Enviar bases para o Victor",
                clock.now() + Duration::hours(2),
            )
            .persisting()
            .with_priority(Priority::High),
        )
        .unwrap();

    // Antes da hora: nada acontece.
    assert!(service.sweep().unwrap().is_empty());

    // A hora chega.
    clock.advance(Duration::hours(2));
    let saiu = service.sweep().unwrap();
    assert_eq!(saiu.len(), 1);
    assert_eq!(saiu[0].reason, DueReason::DueNow);

    // A pessoa ignora. O lembrete continua existindo e continua cobrando.
    let depois = service.reminder(criado.id).unwrap();
    assert_eq!(depois.status, ReminderStatus::Due);
    assert!(depois.status.needs_attention());
    assert!(depois.retry_at.is_some(), "vai insistir");

    // Trinta minutos depois ele insiste.
    clock.advance(Duration::minutes(31));
    let insistiu = service.sweep().unwrap();
    assert_eq!(insistiu.len(), 1);
    assert_eq!(insistiu[0].reason, DueReason::Retry);
    assert_eq!(insistiu[0].reminder.escalation_step, 1);

    // Mais uma, mais outra — e então para de tocar.
    clock.advance(Duration::hours(2));
    assert_eq!(service.sweep().unwrap().len(), 1);
    clock.advance(Duration::hours(3));
    assert_eq!(service.sweep().unwrap().len(), 1);
    clock.advance(Duration::hours(5));
    assert!(
        service.sweep().unwrap().is_empty(),
        "tres degraus e chega de tocar"
    );

    // O que importa: ele NÃO sumiu.
    let final_ = service.reminder(criado.id).unwrap();
    assert!(!final_.status.is_terminal());
    let atencao = service.attention_list().unwrap();
    assert!(atencao.iter().any(|(item, _)| item.id == criado.id));
}

/// Um lembrete comum não insiste. Insistir é decisão da pessoa, e não do
/// sistema — senão tudo insiste e nada é levado a sério.
#[test]
fn um_lembrete_comum_toca_uma_vez_so() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);
    service
        .create(CreateReminder::at(
            "Alongar",
            clock.now() + Duration::minutes(10),
        ))
        .unwrap();

    clock.advance(Duration::minutes(11));
    assert_eq!(service.sweep().unwrap().len(), 1);
    clock.advance(Duration::hours(6));
    assert!(service.sweep().unwrap().is_empty());
}

// -------------------------------------------------------------------- §21

/// **Missed rescue.** O que venceu com o app fechado volta como PERDIDO, e não
/// como se tivesse acabado de vencer.
#[test]
fn o_que_venceu_com_o_app_fechado_volta_como_perdido() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);
    for titulo in ["Ligar para a Ana", "Pagar o boleto", "Revisar a prancha"] {
        service
            .create(CreateReminder::at(titulo, clock.now() + Duration::hours(1)))
            .unwrap();
    }

    // A máquina ficou desligada um dia inteiro.
    clock.advance(Duration::days(1));
    let saiu = service.sweep().unwrap();
    assert_eq!(saiu.len(), 3);
    assert!(saiu
        .iter()
        .all(|item| item.reason == DueReason::MissedWhileAway));
    assert!(saiu
        .iter()
        .all(|item| item.reminder.status == ReminderStatus::Missed));

    // Idempotente: a segunda varredura não repete nada.
    assert!(service.sweep().unwrap().is_empty());
}

// ---------------------------------------------------------------- §16, §53

/// **Reminder Stack.** Quatro alertas, um lembrete. Cada alerta toca, e só
/// depois do último o lembrete fica cobrando de verdade.
#[test]
fn uma_entrega_com_quatro_alertas_continua_sendo_um_lembrete() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);
    let prazo = clock.now() + Duration::days(2);

    let entrega = service
        .create(CreateReminder {
            leads: vec![24 * 60, 4 * 60, 60],
            ..CreateReminder::at("Entregar atividade da faculdade", prazo)
        })
        .unwrap();

    assert_eq!(service.triggers(entrega.id).unwrap().len(), 4);
    assert_eq!(
        service.open().unwrap().len(),
        1,
        "um lembrete, e nao quatro"
    );
    // O primeiro vencimento é o alerta mais adiantado.
    assert_eq!(
        service.reminder(entrega.id).unwrap().next_due_at,
        Some(prazo - Duration::days(1))
    );

    // Um dia antes: toca, e volta a esperar — ainda faltam três alertas.
    clock.advance(Duration::days(1) + Duration::minutes(1));
    assert_eq!(service.sweep().unwrap().len(), 1);
    let depois = service.reminder(entrega.id).unwrap();
    assert_eq!(
        depois.status,
        ReminderStatus::Scheduled,
        "aviso antecipado nao deixa a entrega vencida"
    );
    assert_eq!(depois.next_due_at, Some(prazo - Duration::hours(4)));

    // Os outros três.
    clock.advance(Duration::hours(20) + Duration::minutes(1));
    assert_eq!(service.sweep().unwrap().len(), 1);
    clock.advance(Duration::hours(3) + Duration::minutes(1));
    assert_eq!(service.sweep().unwrap().len(), 1);
    clock.advance(Duration::hours(1) + Duration::minutes(1));
    assert_eq!(service.sweep().unwrap().len(), 1);

    // Agora sim: o prazo passou e não há mais alerta pela frente.
    let vencido = service.reminder(entrega.id).unwrap();
    assert!(vencido.status.needs_attention());
    assert!(service
        .triggers(entrega.id)
        .unwrap()
        .iter()
        .all(|item| item.status == mos_core::StackTriggerStatus::Fired));
}

/// Concluir mata a pilha: nenhum dos alertas restantes chega a tocar.
#[test]
fn concluir_apaga_os_alertas_que_faltavam() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);
    let prazo = clock.now() + Duration::days(2);
    let entrega = service
        .create(CreateReminder {
            leads: vec![24 * 60, 60],
            ..CreateReminder::at("Entregar", prazo)
        })
        .unwrap();

    service.complete(entrega.id).unwrap();
    clock.advance(Duration::days(3));
    assert!(service.sweep().unwrap().is_empty());
    assert!(service
        .triggers(entrega.id)
        .unwrap()
        .iter()
        .all(|item| item.status == mos_core::StackTriggerStatus::Cancelled));
}

// ------------------------------------------------------------- §19, §20, §65

/// **Recorrência fixa.** Concluir não encerra a série: ela avança sozinha.
#[test]
fn uma_serie_fixa_avanca_ao_ser_concluida() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);
    let regra = mos_core::Recurrence {
        rule: mos_core::RecurrenceRule::Daily,
        anchor: mos_core::RecurrenceAnchor::Fixed,
        hour: 8,
        minute: 0,
        offset_minutes: 0,
    };
    let diario = service
        .create(CreateReminder {
            recurrence: Some(regra),
            ..CreateReminder::at("Tomar o remedio", clock.now() + Duration::hours(1))
        })
        .unwrap();

    clock.advance(Duration::hours(2));
    service.sweep().unwrap();
    let depois = service.complete(diario.id).unwrap();

    assert_eq!(depois.status, ReminderStatus::Scheduled, "a serie continua");
    assert!(depois.completed_at.is_none());
    assert!(depois.next_due_at.unwrap() > clock.now());
    // E o histórico registrou a ocorrência.
    let historico = service.history(diario.id, 10).unwrap();
    assert!(historico
        .iter()
        .any(|evento| evento.kind == mos_core::ReminderEventKind::RecurrenceGenerated));
}

/// **Recorrência por conclusão — o cenário §65.** "Limpar o computador a cada
/// 30 dias depois que eu fizer."
#[test]
fn uma_serie_por_conclusao_conta_a_partir_do_dia_em_que_se_fez() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);
    let limpeza = service
        .create(CreateReminder {
            recurrence: Some(mos_core::Recurrence {
                rule: mos_core::RecurrenceRule::EveryDays { days: 30 },
                anchor: mos_core::RecurrenceAnchor::Completion,
                hour: 9,
                minute: 0,
                offset_minutes: 0,
            }),
            ..CreateReminder::at("Limpar o computador", clock.now() + Duration::hours(1))
        })
        .unwrap();

    // Concluída com uma semana de atraso: a próxima conta dali, e não do prazo.
    clock.advance(Duration::days(8));
    let depois = service.complete(limpeza.id).unwrap();
    let proxima = depois.next_due_at.unwrap();
    assert!(proxima > clock.now() + Duration::days(29));
    assert!(proxima < clock.now() + Duration::days(31));
}

// -------------------------------------------------------------------- §33

/// **Quiet hours.** A notificação espera; o lembrete continua vencido e
/// visível. É a separação inteira entre Reminder e Notification.
#[test]
fn o_silencio_segura_a_notificacao_e_nao_o_lembrete() {
    let (service, clock, _guard) = montar();
    fuso_utc(&service);
    // Silêncio da meia-noite às oito, que é o padrão. Empurra o relógio para as
    // três da manhã do dia seguinte.
    clock.advance(Duration::hours(12));

    let lembrete = service
        .create(CreateReminder::at(
            "Conferir o backup",
            clock.now() + Duration::hours(1),
        ))
        .unwrap();
    clock.advance(Duration::hours(2)); // ~04:00 UTC

    let saiu = service.sweep().unwrap();
    assert!(saiu.is_empty(), "nada e entregue de madrugada");

    // Mas o lembrete venceu de verdade, e aparece na tela.
    let estado = service.reminder(lembrete.id).unwrap();
    assert!(estado.status.needs_attention());
    assert!(
        estado.retry_at.is_some(),
        "vai sair quando o silencio acabar"
    );

    // Às oito, sai.
    clock.advance(Duration::hours(5));
    let agora_sim = service.sweep().unwrap();
    assert_eq!(agora_sim.len(), 1);
}

// -------------------------------------------------------------------- §32

/// **Someday.** Existe, aparece na lista, e não interrompe ninguém.
#[test]
fn um_lembrete_sem_data_nunca_dispara() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);
    let solto = service
        .create(CreateReminder::someday("Comprar cabo HDMI"))
        .unwrap();

    assert!(solto.next_due_at.is_none());
    clock.advance(Duration::days(365));
    assert!(service.sweep().unwrap().is_empty());
    assert!(service
        .open()
        .unwrap()
        .iter()
        .any(|item| item.id == solto.id));
    assert!(service.next_wake().unwrap().is_none());
}

// -------------------------------------------------------------- §10, §11, §31

/// Adiar conta fadiga; remarcar não. É a distinção do §11, e ela é o que faz o
/// sistema saber quando oferecer ajuda.
#[test]
fn adiar_conta_fadiga_e_remarcar_nao() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);
    let item = service
        .create(CreateReminder::at(
            "Revisar o memorial",
            clock.now() + Duration::hours(1),
        ))
        .unwrap();

    let mut atual = item;
    for _ in 0..5 {
        atual = service
            .snooze(atual.id, clock.now() + Duration::hours(3))
            .unwrap();
    }
    assert_eq!(atual.snooze_count, 5);
    assert!(
        atual.snooze_fatigue(),
        "cinco adiamentos pedem outra conversa"
    );

    let remarcado = service
        .reschedule(atual.id, clock.now() + Duration::days(2))
        .unwrap();
    assert_eq!(
        remarcado.snooze_count, 5,
        "remarcar nao adia: nao sobe nem zera a contagem"
    );

    // E o histórico distingue os dois gestos.
    let historico = service.history(atual.id, 20).unwrap();
    assert_eq!(
        historico
            .iter()
            .filter(|evento| evento.kind == mos_core::ReminderEventKind::Snoozed)
            .count(),
        5
    );
    assert!(historico
        .iter()
        .any(|evento| evento.kind == mos_core::ReminderEventKind::Rescheduled));
}

// -------------------------------------------------------------------- §7

/// **Needs Attention.** As regras são determinísticas e dizem o porquê.
#[test]
fn needs_attention_explica_por_que_cada_item_esta_la() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);

    let esquecido = service
        .create(
            CreateReminder::at("Enviar as bases", clock.now() + Duration::hours(1))
                .persisting()
                .with_priority(Priority::High),
        )
        .unwrap();
    service
        .create(CreateReminder::at(
            "Daqui a uma semana",
            clock.now() + Duration::days(7),
        ))
        .unwrap();

    clock.advance(Duration::days(1));
    service.sweep().unwrap();

    let lista = service.attention_list().unwrap();
    assert_eq!(lista.len(), 1, "o futuro nao cobra atencao");
    let (item, motivo) = &lista[0];
    assert_eq!(item.id, esquecido.id);
    assert!(motivo.reasons.contains(&mos_core::AttentionReason::Missed));
    assert!(motivo
        .reasons
        .contains(&mos_core::AttentionReason::Persistent));
    assert!(motivo.weight > 0);
}

// -------------------------------------------------------------------- §41

/// O histórico conta a vida do lembrete sem ruído.
#[test]
fn o_historico_registra_o_que_aconteceu() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);
    let item = service
        .create(CreateReminder::at(
            "Mandar o PDF",
            clock.now() + Duration::hours(1),
        ))
        .unwrap();

    // Um minuto depois da hora, e nao duas horas: acima da folga de cinco
    // minutos o vencimento vira PERDIDO, que e outro evento — e o teste quer os
    // dois caminhos distinguiveis.
    clock.advance(Duration::hours(1) + Duration::minutes(1));
    service.sweep().unwrap();
    service
        .snooze(item.id, clock.now() + Duration::hours(2))
        .unwrap();
    clock.advance(Duration::hours(2) + Duration::minutes(1));
    service.sweep().unwrap();
    service.complete(item.id).unwrap();

    let historico = service.history(item.id, 20).unwrap();
    let tipos: Vec<_> = historico.iter().map(|evento| evento.kind).collect();
    for esperado in [
        mos_core::ReminderEventKind::Created,
        mos_core::ReminderEventKind::Triggered,
        mos_core::ReminderEventKind::Snoozed,
        mos_core::ReminderEventKind::Completed,
    ] {
        assert!(
            tipos.contains(&esperado),
            "faltou {esperado:?} em {tipos:?}"
        );
    }
}

// -------------------------------------------------------------------- §18

/// **Follow-up.** Cobrar o Victor é o MESMO lembrete com outra pergunta, e não
/// uma Task nova chamada "Cobrar Victor".
#[test]
fn um_follow_up_e_um_lembrete_com_outra_pergunta() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);
    let cobranca = service
        .create(CreateReminder {
            waiting_for: Some("Victor".to_owned()),
            ..CreateReminder::at("Base estrutural", clock.now() + Duration::days(3))
        })
        .unwrap();

    assert_eq!(cobranca.kind, mos_core::ReminderKind::FollowUp);
    assert_eq!(cobranca.waiting_for, "Victor");
    // Continua sendo um lembrete comum para o agendador.
    clock.advance(Duration::days(3) + Duration::minutes(1));
    assert_eq!(service.sweep().unwrap().len(), 1);
}

// -------------------------------------------------------------------- §59

/// **Falha de canal nunca resolve a intenção.** Este é o teste que guarda a
/// promessa inteira do sistema.
#[test]
fn falha_de_entrega_nao_encosta_no_lembrete() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);
    let item = service
        .create(CreateReminder::at(
            "Ligar para o cliente",
            clock.now() + Duration::hours(1),
        ))
        .unwrap();

    clock.advance(Duration::hours(2));
    service.sweep().unwrap();

    let entrega = service
        .queue_delivery(
            item.id,
            mos_core::Channel::Windows,
            "reminder-due",
            mos_core::VisualLevel::Normal,
        )
        .unwrap()
        .unwrap();
    service.mark_failed(&entrega, "o Windows recusou").unwrap();

    let depois = service.reminder(item.id).unwrap();
    assert!(!depois.status.is_terminal());
    assert!(depois.status.needs_attention());
}

/// **Uma cobrança é uma cobrança, mesmo saindo por dois canais.**
///
/// `delivered_count` responde "quantas vezes eu fui cobrado disto?", e a
/// resposta certa para um toque que saiu no app E no Windows ao mesmo tempo é
/// UMA. Contando por canal, o número dobrava — e a regra que chama de
/// `ignored` quem foi avisado duas vezes passava a acusar quem foi avisado uma.
#[test]
fn dois_canais_na_mesma_rodada_contam_uma_cobranca() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);
    let item = service
        .create(CreateReminder::at(
            "Ligar para o cliente",
            clock.now() + Duration::hours(1),
        ))
        .unwrap();

    clock.advance(Duration::hours(1) + Duration::minutes(1));
    service.sweep().unwrap();

    let no_app = service
        .queue_delivery(
            item.id,
            mos_core::Channel::InApp,
            "reminder-due",
            mos_core::VisualLevel::Normal,
        )
        .unwrap()
        .unwrap();
    service.record_delivered(&no_app).unwrap();

    let no_windows = service
        .queue_delivery(
            item.id,
            mos_core::Channel::Windows,
            "os-reminder-due",
            mos_core::VisualLevel::Normal,
        )
        .unwrap()
        .unwrap();
    service.record_channel_delivery(&no_windows).unwrap();

    let depois = service.reminder(item.id).unwrap();
    assert_eq!(depois.delivered_count, 1, "duas telas, uma cobranca");
    // E, com uma cobranca so, ele ainda nao e "ignorado".
    let (_, motivo) = service
        .attention_list()
        .unwrap()
        .into_iter()
        .find(|(lembrete, _)| lembrete.id == item.id)
        .expect("deveria estar em Needs Attention por estar atrasado");
    assert!(!motivo.reasons.contains(&mos_core::AttentionReason::Ignored));

    // As DUAS entregas ficaram registradas — o que nao viaja e a contagem, e
    // nao o fato.
    assert_eq!(
        service
            .history(item.id, 20)
            .unwrap()
            .iter()
            .filter(|evento| evento.kind == mos_core::ReminderEventKind::Delivered)
            .count(),
        2
    );
}

/// A deduplicação impede a cópia, e não a próxima cobrança legítima.
#[test]
fn a_deduplicacao_nao_silencia_o_proximo_aviso() {
    let (service, clock, _guard) = montar();
    sem_silencio(&service);
    fuso_utc(&service);
    let item = service
        .create(CreateReminder::at(
            "Conferir",
            clock.now() + Duration::hours(1),
        ))
        .unwrap();
    clock.advance(Duration::hours(2));
    service.sweep().unwrap();

    let primeira = service
        .queue_delivery(
            item.id,
            mos_core::Channel::InApp,
            "reminder-due",
            mos_core::VisualLevel::Normal,
        )
        .unwrap();
    assert!(primeira.is_some());
    let copia = service
        .queue_delivery(
            item.id,
            mos_core::Channel::InApp,
            "reminder-due",
            mos_core::VisualLevel::Normal,
        )
        .unwrap();
    assert!(copia.is_none(), "a copia e bloqueada");

    // Outro assunto sobre o mesmo lembrete continua passando.
    let outro = service
        .queue_delivery(
            item.id,
            mos_core::Channel::InApp,
            "reminder-missed",
            mos_core::VisualLevel::Normal,
        )
        .unwrap();
    assert!(outro.is_some());
}
