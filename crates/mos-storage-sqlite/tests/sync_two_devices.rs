//! Dois dispositivos de verdade, dois bancos de verdade, um laco de
//! sincronizacao inteiro.
//!
//! # Por que este teste existe
//!
//! O §80 da missao pede uma demonstracao: criar no PC, aparecer no iPhone,
//! editar no iPhone, aparecer no PC, criar offline, reconectar, nada some. O
//! iPhone nao existe ainda — compilar para iOS exige um Mac. Mas **o laco nao
//! depende de qual e a plataforma**: ele depende de dois bancos, dois relogios
//! e um transporte.
//!
//! Entao aqui os dois "dispositivos" sao dois arquivos SQLite com identidades
//! diferentes, e o que se prova e exatamente a parte que poderia estar errada:
//! a fila, a reconciliacao, o cursor e a ausencia de perda.
//!
//! # O `HubLocal` e a especificacao do servidor
//!
//! Nao ha servidor. O `HubLocal` abaixo e o menor que satisfaz o contrato — e
//! por isso ele vale como especificacao executavel: o dia em que um servidor
//! existir, ele precisa se comportar assim. Guardar em ordem, devolver a partir
//! de um cursor, aceitar reenvio sem duplicar.

use std::sync::Mutex;

use mos_sync::{
    aplicar, carregar_relogio, sincronizar, ClockRepository, ConflictRepository, Deposito,
    DeviceRepository, EntityRef, EstadoDaEntidade, HlcClock, Lote, Op, OpBody, OutboxRepository,
    Projecao, Resultado, Transport,
};
use mos_storage_sqlite::SqliteStorage;
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;

// ------------------------------------------------------------------ o hub

/// O outro lado, em memoria.
///
/// Guarda operacoes em ordem de chegada e devolve a partir de um indice. E o
/// minimo que o contrato exige, e nada alem disso — um hub esperto esconderia
/// defeito do motor.
#[derive(Default)]
struct HubLocal {
    log: Mutex<Vec<Op>>,
    /// Simula rede fora do ar.
    caido: Mutex<bool>,
}

impl HubLocal {
    fn derrubar(&self) {
        *self.caido.lock().unwrap() = true;
    }

    fn levantar(&self) {
        *self.caido.lock().unwrap() = false;
    }

    fn total(&self) -> usize {
        self.log.lock().unwrap().len()
    }
}

impl Transport for &HubLocal {
    fn push(&self, _contrato: u32, ops: &[Op]) -> Resultado<Vec<Uuid>> {
        if *self.caido.lock().unwrap() {
            return Err(mos_sync::SyncError::novo("Sem rede.", true));
        }
        let mut log = self.log.lock().unwrap();
        let mut aceitas = Vec::new();
        for op in ops {
            // Aceitar e diferente de guardar: uma operacao ja conhecida e
            // confirmada do mesmo jeito, e e isso que faz o retry ser seguro.
            if !log.iter().any(|existente| existente.id == op.id) {
                log.push(op.clone());
            }
            aceitas.push(op.id);
        }
        Ok(aceitas)
    }

    fn pull(&self, _contrato: u32, cursor: &str, limite: usize) -> Resultado<Lote> {
        if *self.caido.lock().unwrap() {
            return Err(mos_sync::SyncError::novo("Sem rede.", true));
        }
        let log = self.log.lock().unwrap();
        let de: usize = cursor.parse().unwrap_or(0);
        let ate = (de + limite).min(log.len());
        Ok(Lote {
            ops: log[de.min(log.len())..ate].to_vec(),
            proximo_cursor: ate.to_string(),
            tem_mais: ate < log.len(),
        })
    }
}

// -------------------------------------------------------------- dispositivo

/// Um dispositivo: banco, relogio e a projecao do que ele sabe.
struct Dispositivo {
    _dir: tempfile::TempDir,
    storage: SqliteStorage,
    relogio: HlcClock,
    /// O estado materializado. No M/OS de verdade isto sao as tabelas de
    /// dominio; aqui e um mapa, porque o que este teste prova e o LACO, e nao
    /// a traducao de operacao em Task.
    visao: Visao,
    hora: i64,
}

impl Dispositivo {
    fn novo(nome: &str, plataforma: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let arquivo = dir.path().join("mos.db");
        let backups = dir.path().join("backups");
        std::fs::create_dir_all(&backups).unwrap();
        let storage = SqliteStorage::open(&arquivo, &backups).unwrap();
        let device = storage.este_dispositivo(nome, plataforma, "0.3.0").unwrap();
        let relogio = carregar_relogio(&storage, device.id).unwrap();
        Self {
            _dir: dir,
            storage,
            relogio,
            visao: Visao::default(),
            hora: 1_000,
        }
    }

    /// Uma mudanca local: grava a projecao e enfileira a operacao.
    ///
    /// E o que o M/OS vai fazer em toda mutacao — a ordem importa: a tela
    /// atualiza primeiro, a fila depois, e nenhuma das duas espera rede.
    fn mudar(&mut self, entidade: &EntityRef, campos: &[(&str, serde_json::Value)]) -> Op {
        self.hora += 10;
        let at = self.relogio.tick(self.hora);
        let op = Op::new(
            Uuid::now_v7(),
            entidade.clone(),
            OpBody::Update {
                fields: campos
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), v.clone()))
                    .collect(),
            },
            at,
        );
        let base = self.visao.estado_de(&op);
        let reconciliado = aplicar(base, std::slice::from_ref(&op)).estado;
        self.visao.guardar(&op, &reconciliado).unwrap();
        self.storage.enfileirar(&op).unwrap();
        self.storage.guardar(self.relogio.ultimo()).unwrap();
        op
    }

    fn sincronizar(&mut self, hub: &HubLocal) -> mos_sync::Rodada {
        self.hora += 10;
        let deposito = Deposito {
            outbox: &self.storage,
            conflitos: &self.storage,
            relogio: &self.storage,
            dispositivos: &self.storage,
        };
        sincronizar(
            &deposito,
            &hub,
            &mut self.relogio,
            &mut self.visao,
            self.hora,
            100,
        )
    }

    fn ver(&self, entidade: &EntityRef, campo: &str) -> Option<serde_json::Value> {
        self.visao
            .0
            .get(&(entidade.kind.as_str().to_owned(), entidade.id))
            .and_then(|estado| estado.campo(campo).cloned())
    }
}

/// A projecao do teste: o menor adaptador que satisfaz a porta.
#[derive(Clone, Default, Debug, PartialEq)]
struct Visao(BTreeMap<(String, Uuid), EstadoDaEntidade>);

impl Visao {
    fn chave(op: &Op) -> (String, Uuid) {
        (op.entity.kind.as_str().to_owned(), op.entity.id)
    }
}

impl Projecao for Visao {
    fn estado_de(&self, op: &Op) -> EstadoDaEntidade {
        self.0.get(&Self::chave(op)).cloned().unwrap_or_default()
    }

    fn guardar(&mut self, op: &Op, estado: &EstadoDaEntidade) -> Resultado<()> {
        self.0.insert(Self::chave(op), estado.clone());
        Ok(())
    }
}

fn task() -> EntityRef {
    EntityRef::new("task", Uuid::from_u128(4242))
}

fn capture() -> EntityRef {
    EntityRef::new("capture", Uuid::from_u128(4243))
}

fn tipo_do_futuro() -> EntityRef {
    EntityRef::new("tipo_do_futuro", Uuid::from_u128(4244))
}

// ------------------------------------------------------------------ testes

#[test]
fn criar_no_pc_aparece_no_iphone() {
    // §80, passos 1 a 4.
    let hub = HubLocal::default();
    let mut pc = Dispositivo::novo("PC Principal", "windows");
    let mut iphone = Dispositivo::novo("iPhone 14 Pro", "ios");

    pc.mudar(&task(), &[("title", json!("Revisar memorial"))]);
    pc.sincronizar(&hub);
    iphone.sincronizar(&hub);

    assert_eq!(
        iphone.ver(&task(), "title"),
        Some(json!("Revisar memorial")),
        "a Task criada no PC precisa chegar ao iPhone"
    );
}

#[test]
fn editar_no_iphone_volta_para_o_pc() {
    // §80, passos 5 e 6.
    let hub = HubLocal::default();
    let mut pc = Dispositivo::novo("PC", "windows");
    let mut iphone = Dispositivo::novo("iPhone", "ios");

    pc.mudar(&task(), &[("title", json!("original"))]);
    pc.sincronizar(&hub);
    iphone.sincronizar(&hub);

    iphone.mudar(&task(), &[("title", json!("editado no celular"))]);
    iphone.sincronizar(&hub);
    pc.sincronizar(&hub);

    assert_eq!(pc.ver(&task(), "title"), Some(json!("editado no celular")));
}

#[test]
fn dez_capturas_offline_chegam_depois_de_reconectar() {
    // §80, passos 7 a 9. A fila e persistente: o que foi criado sem rede sai
    // quando a rede voltar, sem ninguem pedir de novo.
    let hub = HubLocal::default();
    let mut iphone = Dispositivo::novo("iPhone", "ios");
    let mut pc = Dispositivo::novo("PC", "windows");

    hub.derrubar();
    for i in 0..10 {
        let capture = EntityRef::new("capture", Uuid::from_u128(500 + i));
        iphone.mudar(&capture, &[("content", json!(format!("ideia {i}")))]);
    }
    let rodada = iphone.sincronizar(&hub);
    assert!(rodada.erro.is_some(), "sem rede a rodada precisa falhar");
    assert_eq!(
        iphone.storage.quantidade_pendente().unwrap(),
        10,
        "nada pode sair da fila antes de o outro lado confirmar"
    );

    hub.levantar();
    iphone.sincronizar(&hub);
    assert_eq!(iphone.storage.quantidade_pendente().unwrap(), 0);

    pc.sincronizar(&hub);
    for i in 0..10 {
        let capture = EntityRef::new("capture", Uuid::from_u128(500 + i));
        assert_eq!(
            pc.ver(&capture, "content"),
            Some(json!(format!("ideia {i}"))),
            "a capture {i} precisa ter chegado"
        );
    }
}

#[test]
fn campos_diferentes_nos_dois_lados_convivem() {
    // O caso do §8, agora contra banco de verdade e nao so no motor.
    let hub = HubLocal::default();
    let mut pc = Dispositivo::novo("PC", "windows");
    let mut iphone = Dispositivo::novo("iPhone", "ios");

    pc.mudar(&task(), &[("title", json!("base"))]);
    pc.sincronizar(&hub);
    iphone.sincronizar(&hub);

    // Os dois editam ao mesmo tempo, campos diferentes, sem se falarem.
    pc.mudar(&task(), &[("title", json!("Revisar o memorial"))]);
    iphone.mudar(&task(), &[("due_at", json!("2026-08-22T09:00:00Z"))]);

    pc.sincronizar(&hub);
    iphone.sincronizar(&hub);
    pc.sincronizar(&hub);

    assert_eq!(pc.ver(&task(), "title"), Some(json!("Revisar o memorial")));
    assert_eq!(
        pc.ver(&task(), "due_at"),
        Some(json!("2026-08-22T09:00:00Z")),
        "a edicao do celular nao pode ter sumido"
    );
    assert_eq!(
        pc.storage.abertos(10).unwrap().len(),
        0,
        "campos diferentes nao sao conflito"
    );
}

#[test]
fn o_mesmo_campo_nos_dois_lados_guarda_o_perdedor() {
    // A garantia central: um vence, o outro NAO some.
    let hub = HubLocal::default();
    let mut pc = Dispositivo::novo("PC", "windows");
    let mut iphone = Dispositivo::novo("iPhone", "ios");

    pc.mudar(&task(), &[("title", json!("base"))]);
    pc.sincronizar(&hub);
    iphone.sincronizar(&hub);

    pc.mudar(&task(), &[("title", json!("versao do PC"))]);
    iphone.mudar(&task(), &[("title", json!("versao do celular"))]);
    pc.sincronizar(&hub);
    iphone.sincronizar(&hub);

    let conflitos = iphone.storage.abertos(10).unwrap();
    assert_eq!(conflitos.len(), 1, "o mesmo campo dos dois lados e conflito");
    let guardado = &conflitos[0];
    assert_eq!(guardado.campo, "title");
    // O valor perdedor esta guardado, inteiro, com o dispositivo de origem.
    let valores = [
        guardado.vencedor.valor.clone(),
        guardado.perdedor.valor.clone(),
    ];
    assert!(valores.contains(&json!("versao do PC")));
    assert!(valores.contains(&json!("versao do celular")));
}

#[test]
fn sincronizar_de_novo_nao_duplica_nem_muda_nada() {
    // §53 e §80 passo 13: reiniciar e sincronizar de novo nao pode inventar
    // nada. O hub guarda uma operacao so, e a projecao nao muda.
    let hub = HubLocal::default();
    let mut pc = Dispositivo::novo("PC", "windows");

    pc.mudar(&task(), &[("title", json!("unica"))]);
    pc.sincronizar(&hub);
    let depois_da_primeira = pc.visao.clone();

    for _ in 0..5 {
        pc.sincronizar(&hub);
    }

    assert_eq!(hub.total(), 1, "o hub nao pode acumular copias");
    assert_eq!(pc.visao, depois_da_primeira, "o estado nao pode mudar");
    assert_eq!(pc.storage.quantidade_pendente().unwrap(), 0);
}

#[test]
fn o_relogio_sobrevive_ao_fechamento_do_app() {
    // §80 passo 13. Reabrir com o relogio de parede atrasado nao pode gerar
    // eventos que se ordenam antes do que ja foi sincronizado.
    let dir = tempfile::tempdir().unwrap();
    let arquivo = dir.path().join("mos.db");
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();

    let ultimo = {
        let storage = SqliteStorage::open(&arquivo, &backups).unwrap();
        let device = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();
        let mut relogio = carregar_relogio(&storage, device.id).unwrap();
        let momento = relogio.tick(9_000_000);
        storage.guardar(momento).unwrap();
        momento
    };

    // O app fecha e reabre. O relogio de parede voltou para bem antes.
    let storage = SqliteStorage::open(&arquivo, &backups).unwrap();
    let device = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();
    let mut relogio = carregar_relogio(&storage, device.id).unwrap();
    let novo = relogio.tick(1_000);

    assert!(
        novo > ultimo,
        "o instante depois de reabrir precisa vir depois do ultimo guardado"
    );
}

#[test]
fn o_cursor_faz_o_custo_crescer_com_o_que_mudou() {
    // §43: ninguem baixa a base inteira para descobrir que nada mudou.
    let hub = HubLocal::default();
    let mut pc = Dispositivo::novo("PC", "windows");
    let mut iphone = Dispositivo::novo("iPhone", "ios");

    for i in 0..5 {
        pc.mudar(
            &EntityRef::new("task", Uuid::from_u128(700 + i)),
            &[("title", json!(format!("t{i}")))],
        );
    }
    pc.sincronizar(&hub);

    let primeira = iphone.sincronizar(&hub);
    assert_eq!(primeira.recebidas, 5, "a primeira rodada traz tudo");

    let segunda = iphone.sincronizar(&hub);
    assert_eq!(
        segunda.recebidas, 0,
        "sem mudanca nova, a rodada seguinte nao traz nada"
    );

    pc.mudar(&task(), &[("title", json!("nova"))]);
    pc.sincronizar(&hub);
    let terceira = iphone.sincronizar(&hub);
    assert_eq!(terceira.recebidas, 1, "so o que mudou desde o cursor");
}

#[test]
fn apagar_num_dispositivo_apaga_no_outro() {
    let hub = HubLocal::default();
    let mut pc = Dispositivo::novo("PC", "windows");
    let mut iphone = Dispositivo::novo("iPhone", "ios");

    pc.mudar(&task(), &[("title", json!("some"))]);
    pc.sincronizar(&hub);
    iphone.sincronizar(&hub);

    iphone.hora += 10;
    let at = iphone.relogio.tick(iphone.hora);
    let apagar = Op::new(Uuid::now_v7(), task(), OpBody::Delete, at);
    iphone.storage.enfileirar(&apagar).unwrap();
    iphone.sincronizar(&hub);
    pc.sincronizar(&hub);

    let chave = ("task".to_owned(), task().id);
    assert!(
        !pc.visao.0.get(&chave).unwrap().visivel(),
        "apagar no celular precisa apagar no PC"
    );
    assert_eq!(
        pc.ver(&task(), "title"),
        Some(json!("some")),
        "o conteudo continua guardado: restaurar devolve a Task inteira"
    );
}

/// A faixa da Home diz "3 tasks e 1 capture", e `recebidas` nao serve para isso.
///
/// Ele conta OPERACOES: duas edicoes da mesma Task sao duas operacoes e uma
/// task. Dizer "2 tasks" para quem mexeu numa so seria mentira com numero.
#[test]
fn a_rodada_conta_entidades_por_tipo_e_nao_operacoes() {
    let hub = HubLocal::default();
    let mut pc = Dispositivo::novo("PC", "windows");
    let mut iphone = Dispositivo::novo("iPhone", "ios");

    pc.mudar(&task(), &[("title", json!("Revisar memorial"))]);
    pc.mudar(&task(), &[("state", json!("doing"))]);
    pc.mudar(&capture(), &[("content", json!("ideia na rua"))]);
    // `EntityKind` e texto e nao enum fechado (SYNC.md §9): um tipo que este
    // cliente nao conhece precisa CONTAR em vez de sumir, senao a faixa diz que
    // nada chegou quando algo chegou.
    pc.mudar(&tipo_do_futuro(), &[("qualquer", json!(1))]);
    pc.sincronizar(&hub);

    let rodada = iphone.sincronizar(&hub);

    assert_eq!(rodada.recebidas, 4, "`recebidas` conta OPERACOES");
    assert_eq!(
        rodada.recebidas_por_tipo.get("task"),
        Some(&1),
        "duas mudancas na mesma task sao UMA task"
    );
    assert_eq!(rodada.recebidas_por_tipo.get("capture"), Some(&1));
    assert_eq!(rodada.recebidas_por_tipo.get("tipo_do_futuro"), Some(&1));
}
