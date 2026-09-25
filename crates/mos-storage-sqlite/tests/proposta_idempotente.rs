//! F1.2 do MOS-OVERHAUL-PLAN: a mesma proposta do Hermes nunca executa duas
//! vezes. Clique duplo, retry depois do desfecho e reinicio com a proposta
//! reservada — todos contra SQLite de verdade.

use std::sync::Arc;

use mos_core::{
    ConversationRepository, ConversationService, MessageStatus, NewConversation, NewMessage,
    PartBody, ProposalClaim, ProposalStatus,
};
use mos_storage_sqlite::SqliteStorage;

const RAW: &str = r#"{"action":"mos.task.create","args":{"title":"Revisar memorial"}}"#;

fn abrir(dir: &std::path::Path) -> Arc<SqliteStorage> {
    Arc::new(SqliteStorage::open(dir.join("mos.db"), dir.join("backups")).unwrap())
}

fn proposta(status: ProposalStatus) -> PartBody {
    let args = mos_core::parse_action(RAW).unwrap();
    PartBody::ActionProposal {
        raw: RAW.to_owned(),
        preview: mos_core::preview_of(&args),
        status,
        outcome: String::new(),
        audit: None,
    }
}

fn status_de(service: &ConversationService, id: &str) -> ProposalStatus {
    service
        .message(id)
        .unwrap()
        .parts
        .into_iter()
        .find_map(|part| match part.body {
            PartBody::ActionProposal { status, .. } => Some(status),
            _ => None,
        })
        .unwrap()
}

fn mensagem_com_proposta(storage: &Arc<SqliteStorage>) -> String {
    let conversation = storage
        .create_conversation(NewConversation::create())
        .unwrap();
    let answer = storage
        .append_message(NewMessage::pending_assistant(conversation.id))
        .unwrap();
    storage
        .finish_message(
            answer.id,
            MessageStatus::Complete,
            vec![proposta(ProposalStatus::Pending)],
        )
        .unwrap();
    answer.id.to_string()
}

#[test]
fn so_o_primeiro_clique_reserva_a_proposta() {
    let dir = tempfile::tempdir().unwrap();
    let storage = abrir(dir.path());
    let id = mensagem_com_proposta(&storage);
    let service = ConversationService::new(storage.clone());

    // Dois cliques ao mesmo tempo: exatamente um recebe a vez.
    let resultados: Vec<ProposalClaim> = std::thread::scope(|scope| {
        let a = scope.spawn(|| service.claim_proposal(&id, RAW, true).unwrap());
        let b = scope.spawn(|| service.claim_proposal(&id, RAW, true).unwrap());
        vec![a.join().unwrap(), b.join().unwrap()]
    });
    let reservas = resultados
        .iter()
        .filter(|claim| matches!(claim, ProposalClaim::Claimed))
        .count();
    assert_eq!(reservas, 1, "{resultados:?}");
    assert!(resultados
        .iter()
        .any(|claim| matches!(claim, ProposalClaim::InFlight)));
    assert_eq!(status_de(&service, &id), ProposalStatus::Executing);
}

#[test]
fn proposta_resolvida_devolve_o_desfecho_em_vez_de_repetir() {
    let dir = tempfile::tempdir().unwrap();
    let storage = abrir(dir.path());
    let id = mensagem_com_proposta(&storage);
    let service = ConversationService::new(storage.clone());
    assert!(matches!(
        service.claim_proposal(&id, RAW, true).unwrap(),
        ProposalClaim::Claimed
    ));
    service
        .attach_parts(
            &id,
            MessageStatus::Complete,
            vec![PartBody::ActionProposal {
                raw: RAW.to_owned(),
                preview: mos_core::preview_of(&mos_core::parse_action(RAW).unwrap()),
                status: ProposalStatus::Executed,
                outcome: "Task criada: Revisar memorial".into(),
                audit: None,
            }],
        )
        .unwrap();

    // Retry depois de timeout: o resultado é recuperado, não refeito.
    match service.claim_proposal(&id, RAW, true).unwrap() {
        ProposalClaim::Settled(PartBody::ActionProposal {
            status, outcome, ..
        }) => {
            assert_eq!(status, ProposalStatus::Executed);
            assert_eq!(outcome, "Task criada: Revisar memorial");
        }
        outro => panic!("esperava desfecho gravado, veio {outro:?}"),
    }
    // Cancelar depois de executada também não muda nada.
    assert!(matches!(
        service.claim_proposal(&id, RAW, false).unwrap(),
        ProposalClaim::Settled(_)
    ));
}

#[test]
fn reserva_orfa_de_uma_queda_fica_incerta_depois_de_reabrir() {
    let dir = tempfile::tempdir().unwrap();
    let id = {
        let storage = abrir(dir.path());
        let id = mensagem_com_proposta(&storage);
        let service = ConversationService::new(storage.clone());
        assert!(matches!(
            service.claim_proposal(&id, RAW, true).unwrap(),
            ProposalClaim::Claimed
        ));
        // O app cai aqui: efeito possivelmente aplicado, desfecho não gravado.
        id
    };
    let storage = abrir(dir.path());
    let service = ConversationService::new(storage.clone());
    service.settle_unfinished().unwrap();
    assert_eq!(status_de(&service, &id), ProposalStatus::Executing);
    assert!(matches!(
        service.claim_proposal(&id, RAW, true).unwrap(),
        ProposalClaim::InFlight
    ));
}
