//! Os cenarios que o §54 da missao exige, escritos como teste e nao como
//! promessa. Cada um nomeia o que aconteceria de errado sem a regra.

use serde_json::json;
use uuid::Uuid;

use crate::*;

fn pc() -> DeviceId {
    DeviceId(Uuid::from_u128(1))
}

fn iphone() -> DeviceId {
    DeviceId(Uuid::from_u128(2))
}

fn tarefa() -> EntityRef {
    EntityRef::new("task", Uuid::from_u128(100))
}

fn campos(pares: &[(&str, serde_json::Value)]) -> serde_json::Map<String, serde_json::Value> {
    pares
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect()
}

fn update(device: DeviceId, wall: i64, pares: &[(&str, serde_json::Value)]) -> Op {
    Op::new(
        Uuid::now_v7(),
        tarefa(),
        OpBody::Update {
            fields: campos(pares),
        },
        Hlc::new(wall, 0, device),
    )
}

// ---------------------------------------------------------------- o relogio

#[test]
fn o_relogio_nunca_anda_para_tras() {
    // Horario de verao, NTP e maquina virtual que hiberna fazem o relogio de
    // parede voltar de verdade. Se o HLC voltasse junto, um evento novo se
    // ordenaria antes de um velho e a reconciliacao passaria a mentir.
    let mut relogio = HlcClock::new(pc());
    let primeiro = relogio.tick(1_000);
    let segundo = relogio.tick(900); // o relogio do sistema voltou
    assert!(segundo > primeiro);
    assert_eq!(segundo.wall_ms, 1_000);
    assert_eq!(segundo.counter, 1);
}

#[test]
fn observar_o_futuro_puxa_o_relogio_junto() {
    // Sem isto, um celular atrasado geraria eventos que se ordenam ANTES de
    // coisas que ele acabou de receber do PC e ja mostrou na tela.
    let mut relogio = HlcClock::new(iphone());
    let remoto = Hlc::new(5_000, 3, pc());
    let emitido = relogio.observar(remoto, 1_000);
    assert!(emitido > remoto);
    assert_eq!(emitido.device, iphone());
}

#[test]
fn a_ordem_e_total_e_igual_nos_dois_lados() {
    // Mesmo milissegundo, mesmo contador, dispositivos diferentes: os dois
    // precisam chegar a MESMA ordem sem se falarem.
    let a = Hlc::new(1_000, 0, pc());
    let b = Hlc::new(1_000, 0, iphone());
    assert_ne!(a.cmp(&b), std::cmp::Ordering::Equal);
    assert_eq!(a.cmp(&b), a.cmp(&b));
}

// ------------------------------------------------- cenario 1: fila offline

#[test]
fn cenario_1_dez_captures_offline_chegam_todas() {
    // iPhone offline cria dez, reconecta. As dez aparecem — nenhuma se perde e
    // nenhuma vira a outra por colisao de id.
    let mut relogio = HlcClock::new(iphone());
    let ops: Vec<Op> = (0..10)
        .map(|i| {
            Op::new(
                Uuid::now_v7(),
                EntityRef::new("capture", Uuid::from_u128(200 + i)),
                OpBody::Create {
                    fields: campos(&[("content", json!(format!("nota {i}")))]),
                },
                relogio.tick(1_000 + i as i64),
            )
        })
        .collect();

    let mut vistas = std::collections::BTreeSet::new();
    for op in &ops {
        let r = aplicar(EstadoDaEntidade::default(), std::slice::from_ref(op));
        assert!(r.estado.visivel());
        vistas.insert(op.entity.id);
    }
    assert_eq!(vistas.len(), 10);
}

// ------------------------------------------ cenario 2: campos diferentes

#[test]
fn cenario_2_campos_diferentes_convivem() {
    // O caso que o §8 nomeia: titulo no PC, data no celular. As DUAS edicoes
    // sobrevivem. Com "ultima gravacao vence" por entidade, uma sumiria.
    let ops = vec![
        update(pc(), 2_000, &[("title", json!("Revisar memorial"))]),
        update(iphone(), 2_100, &[("due_at", json!("2026-08-22T09:00:00Z"))]),
    ];
    let r = aplicar(EstadoDaEntidade::default(), &ops);

    assert_eq!(r.estado.campo("title").unwrap(), &json!("Revisar memorial"));
    assert_eq!(
        r.estado.campo("due_at").unwrap(),
        &json!("2026-08-22T09:00:00Z")
    );
    assert!(r.conflitos.is_empty(), "campos diferentes nao sao conflito");
}

// ------------------------------------------- cenario 3: o mesmo campo

#[test]
fn cenario_3_mesmo_campo_gera_conflito_sem_perda() {
    // Um vence, mas o outro NAO e jogado fora: fica no conflito, com os dois
    // lados, para a interface poder mostrar. E o que separa "resolver" de
    // "escolher um e apagar o outro".
    let ops = vec![
        update(pc(), 3_000, &[("title", json!("Memorial - revisao"))]),
        update(iphone(), 3_500, &[("title", json!("Revisar o memorial"))]),
    ];
    let r = aplicar(EstadoDaEntidade::default(), &ops);

    assert_eq!(
        r.estado.campo("title").unwrap(),
        &json!("Revisar o memorial")
    );
    assert_eq!(r.conflitos.len(), 1);
    let conflito = &r.conflitos[0];
    assert_eq!(conflito.campo, "title");
    assert_eq!(conflito.perdedor.valor, json!("Memorial - revisao"));
    assert_eq!(conflito.perdedor.at.device, pc());
}

#[test]
fn escrever_o_mesmo_valor_nao_e_conflito() {
    // Dois dispositivos marcando a mesma Task como concluida concordam. Chamar
    // isso de conflito encheria a tela de avisos sobre nada.
    let ops = vec![
        update(pc(), 4_000, &[("state", json!("done"))]),
        update(iphone(), 4_100, &[("state", json!("done"))]),
    ];
    let r = aplicar(EstadoDaEntidade::default(), &ops);
    assert!(r.conflitos.is_empty());
}

// --------------------------------- cenario 4: ordem de chegada e retomada

#[test]
fn cenario_4_a_ordem_de_chegada_nao_altera_o_resultado() {
    // App fechado no meio do sync, lote reenviado ao contrario, tres
    // dispositivos ao mesmo tempo: a rede entrega fora de ordem e o resultado
    // precisa ser o mesmo. Sem isso, dois aparelhos com os MESMOS dados
    // mostrariam telas diferentes.
    let ops = vec![
        update(pc(), 5_000, &[("title", json!("A"))]),
        update(iphone(), 5_100, &[("due_at", json!("amanha"))]),
        update(pc(), 5_200, &[("title", json!("B"))]),
    ];
    let direto = aplicar(EstadoDaEntidade::default(), &ops);

    let mut invertido: Vec<Op> = ops.clone();
    invertido.reverse();
    let ao_contrario = aplicar(EstadoDaEntidade::default(), &invertido);

    assert_eq!(direto.estado, ao_contrario.estado);
    assert_eq!(direto.estado.campo("title").unwrap(), &json!("B"));
}

// ------------------------------------------ cenario 5: retry e duplicacao

#[test]
fn cenario_5_reaplicar_nao_duplica_nem_muda_nada() {
    // O §53: um retry nao pode duplicar Task, Reminder, Capture nem Resource.
    let op = update(pc(), 6_000, &[("title", json!("unica"))]);
    let uma = aplicar(EstadoDaEntidade::default(), std::slice::from_ref(&op));
    let dez = aplicar(EstadoDaEntidade::default(), &vec![op.clone(); 10]);

    assert_eq!(uma.estado, dez.estado);
    assert!(dez.conflitos.is_empty(), "a mesma operacao nao briga consigo");
}

// ------------------------------------------------------- apagar e restaurar

#[test]
fn o_apagamento_ganha_da_edicao_concorrente() {
    // Assimetria deliberada: restaurar o que sumiu por engano custa um clique;
    // descobrir semanas depois que algo voltou sozinho custa a confianca no
    // sistema inteiro.
    let ops = vec![
        Op::new(
            Uuid::now_v7(),
            tarefa(),
            OpBody::Delete,
            Hlc::new(7_000, 0, pc()),
        ),
        update(iphone(), 7_500, &[("title", json!("editado depois"))]),
    ];
    let r = aplicar(EstadoDaEntidade::default(), &ops);
    assert!(!r.estado.visivel());
    // O texto continua guardado: restaurar devolve a Task inteira, e nao uma
    // casca vazia.
    assert_eq!(r.estado.campo("title").unwrap(), &json!("editado depois"));
}

#[test]
fn restaurar_so_desfaz_apagamento_anterior() {
    let ops = vec![
        Op::new(
            Uuid::now_v7(),
            tarefa(),
            OpBody::Restore,
            Hlc::new(8_000, 0, iphone()),
        ),
        Op::new(
            Uuid::now_v7(),
            tarefa(),
            OpBody::Delete,
            Hlc::new(8_500, 0, pc()),
        ),
    ];
    let r = aplicar(EstadoDaEntidade::default(), &ops);
    assert!(!r.estado.visivel(), "um Restore velho nao desfaz um Delete novo");
}

// ----------------------------------------------------------- o contrato

#[test]
fn o_contrato_tolera_versoes_diferentes() {
    // A App Store nao publica quando o desktop publica. Exigir versao identica
    // dos dois lados quebraria o sistema em toda atualizacao.
    assert!(contrato_compativel(CONTRACT_VERSION));
    assert!(contrato_compativel(MIN_CONTRACT_VERSION));
    assert!(!contrato_compativel(CONTRACT_VERSION + 1));
}

#[test]
fn plataforma_desconhecida_nao_vira_erro() {
    // Um cliente futuro precisa aparecer na lista de dispositivos com o proprio
    // nome, e nao sumir dela.
    let lida = Platform::ler("visionos");
    assert_eq!(lida.as_str(), "visionos");
    assert_eq!(Platform::ler("ios"), Platform::Ios);
}

#[test]
fn tipo_de_entidade_desconhecido_atravessa_sem_quebrar() {
    // §27 e §74: um cliente antigo precisa guardar e reenviar uma operacao
    // sobre um tipo que ele ainda nao conhece. Enum fechado transformaria isso
    // em erro de desserializacao, e a operacao morreria no cliente velho.
    let cru = json!({
        "id": Uuid::from_u128(9).to_string(),
        "entity": { "kind": "invencao-futura", "id": Uuid::from_u128(10).to_string() },
        "body": { "kind": "update", "fields": { "x": 1 } },
        "at": { "wallMs": 1, "counter": 0, "device": Uuid::from_u128(1).to_string() }
    });
    let op: Op = serde_json::from_value(cru).expect("tipo desconhecido precisa atravessar");
    assert_eq!(op.entity.kind.as_str(), "invencao-futura");
}


// ------------------------------------------------ relacoes do Knowledge Graph

fn recurso() -> Uuid {
    Uuid::from_u128(900)
}

fn projeto() -> Uuid {
    Uuid::from_u128(901)
}

#[test]
fn dois_dispositivos_chegam_ao_mesmo_id_sem_se_falarem() {
    // A razao de o id ser DERIVADO e nao sorteado. Se cada lado sorteasse,
    // ligar o mesmo Resource ao mesmo Project nos dois criaria DUAS relacoes
    // para o mesmo vinculo, e desfazer uma deixaria a outra de pe.
    let no_pc = Relacao::nova("resourceProject", recurso(), projeto());
    let no_iphone = Relacao::nova("resourceProject", recurso(), projeto());
    assert_eq!(no_pc.id(), no_iphone.id());
}

#[test]
fn a_direcao_faz_parte_da_identidade() {
    // `A -> B` e `B -> A` sao vinculos diferentes. Colapsa-los tornaria
    // impossivel expressar uma relacao com direcao.
    let ida = Relacao::nova("resourceProject", recurso(), projeto());
    let volta = Relacao::nova("resourceProject", projeto(), recurso());
    assert_ne!(ida.id(), volta.id());
}

#[test]
fn o_tipo_faz_parte_da_identidade() {
    let a = Relacao::nova("resourceProject", recurso(), projeto());
    let b = Relacao::nova("resourceWorkspace", recurso(), projeto());
    assert_ne!(a.id(), b.id());
}

#[test]
fn desligar_e_religar_termina_ligado() {
    // A razao de `linked` ser CAMPO e nao `OpBody::Delete`. Com `Delete`, a
    // regra de "apagar ganha" faria desvincular as 10:00 vencer revincular as
    // 10:05 — e o vinculo nunca mais voltaria.
    let relacao = Relacao::nova("resourceProject", recurso(), projeto());
    let ops = vec![
        Op::new(
            Uuid::now_v7(),
            relacao.entidade(),
            relacao.alternar(true),
            Hlc::new(1_000, 0, pc()),
        ),
        Op::new(
            Uuid::now_v7(),
            relacao.entidade(),
            relacao.alternar(false),
            Hlc::new(2_000, 0, iphone()),
        ),
        Op::new(
            Uuid::now_v7(),
            relacao.entidade(),
            relacao.alternar(true),
            Hlc::new(3_000, 0, pc()),
        ),
    ];
    let r = aplicar(EstadoDaEntidade::default(), &ops);
    assert_eq!(r.estado.campo("linked").unwrap(), &json!(true));
    assert!(r.estado.visivel(), "a relacao nunca e apagada, so alternada");
}

#[test]
fn ligar_nos_dois_dispositivos_nao_duplica_nem_conflita() {
    // Dois dispositivos ligando o mesmo vinculo concordam. Chamar isso de
    // conflito encheria a tela de aviso sobre nada.
    let relacao = Relacao::nova("resourceProject", recurso(), projeto());
    let ops = vec![
        Op::new(
            Uuid::now_v7(),
            relacao.entidade(),
            relacao.alternar(true),
            Hlc::new(1_000, 0, pc()),
        ),
        Op::new(
            Uuid::now_v7(),
            relacao.entidade(),
            relacao.alternar(true),
            Hlc::new(1_500, 0, iphone()),
        ),
    ];
    let r = aplicar(EstadoDaEntidade::default(), &ops);
    assert_eq!(r.estado.campo("linked").unwrap(), &json!(true));
    assert!(r.conflitos.is_empty());
}

#[test]
fn a_relacao_viaja_dizendo_o_que_ligou() {
    // O id e um hash e nao diz nada. Um dispositivo que recebe esta operacao
    // sem nunca ter visto a relacao precisa saber O QUE foi ligado.
    let relacao = Relacao::nova("resourceProject", recurso(), projeto());
    match relacao.alternar(true) {
        OpBody::Update { fields } => {
            assert_eq!(fields["kind"], json!("resourceProject"));
            assert_eq!(fields["from"], json!(recurso().to_string()));
            assert_eq!(fields["to"], json!(projeto().to_string()));
            assert_eq!(fields["linked"], json!(true));
        }
        outro => panic!("alternar precisa ser Update, veio {outro:?}"),
    }
}

#[test]
fn o_namespace_e_estavel_entre_execucoes() {
    // Mudar o namespace faria todas as relacoes existentes ganharem ids novos,
    // e as antigas ficariam orfas. Este teste trava o valor.
    let relacao = Relacao::nova("resourceProject", Uuid::from_u128(1), Uuid::from_u128(2));
    assert_eq!(
        relacao.id().to_string(),
        "12fa7421-9883-5e03-806b-5e9c46ede391".to_string(),
        "o id de uma relacao nao pode mudar entre versoes do M/OS"
    );
}
