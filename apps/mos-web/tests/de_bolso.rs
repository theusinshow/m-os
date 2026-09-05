//! A captura feita no bolso aparece no PC.
//!
//! E o teste que justifica o `mos-web` existir. Ele NAO usa mock de nada: sobe o
//! hub de verdade, o `mos-web` de verdade com banco proprio, e um segundo M/OS
//! tambem de verdade. A captura entra pela rota HTTP que o navegador vai chamar,
//! e a conferencia e feita pela leitura normal do outro aparelho.
//!
//! Se um dia isto passar com o `mos-web` sendo um proxy do desktop, o teste
//! esta errado — a promessa e que o celular funcione com o PC desligado.

use std::net::SocketAddr;
use std::path::Path;

use mos_core::{CaptureRepository, LifecycleState, WorkRepository};
use mos_storage_sqlite::SqliteStorage;
use mos_sync::DeviceRepository;
use mos_sync_server::{Estado as EstadoHub, Hub};

const TOKEN: &str = "segredo-de-teste-com-tamanho-suficiente";

async fn servir_hub() -> SocketAddr {
    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = ouvinte.local_addr().unwrap();
    let rotas = mos_sync_server::rotas(EstadoHub::novo(Hub::em_memoria().unwrap(), TOKEN));
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });
    endereco
}

/// Sobe o `mos-web` de verdade, com banco proprio, apontado para o hub.
async fn servir_web(pasta: &Path, hub: SocketAddr) -> SocketAddr {
    let backups = pasta.join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let estado = mos_web::estado::Estado::abrir(
        pasta.join("web.db").to_str().unwrap(),
        backups.to_str().unwrap(),
        Some(mos_web::estado::Hub {
            url: format!("http://{hub}"),
            token: TOKEN.to_owned(),
        }),
        // Sem push: estes testes conferem o laco de dados, e uma chave VAPID
        // aqui so acrescentaria uma ida a rede que nao tem para onde ir.
        None,
        // Sem porta: quem confere a porta e o `tests/a_porta.rs`, e exigir
        // sessao aqui obrigaria todo teste de dados a fazer login antes.
        None,
    )
    .unwrap();

    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = ouvinte.local_addr().unwrap();
    let rotas = mos_web::api::rotas().with_state(estado);
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });
    endereco
}

/// O outro aparelho: um M/OS comum.
fn outro_aparelho(pasta: &Path) -> SqliteStorage {
    let backups = pasta.join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let storage = SqliteStorage::open(pasta.join("mos.db"), &backups).unwrap();
    let device = storage.este_dispositivo("PC", "windows", "0.3.1").unwrap();
    storage.habilitar_sync(device.id).unwrap();
    storage
}

fn sincronizar(storage: &SqliteStorage, hub: SocketAddr) -> mos_sync::Rodada {
    let transporte = mos_sync_http::HttpTransport::novo(format!("http://{hub}"), TOKEN).unwrap();
    let agora = (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64;
    storage.sincronizar_agora(&transporte, agora, 100).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_captura_do_bolso_chega_no_pc() {
    let hub = servir_hub().await;
    let pasta_web = tempfile::tempdir().unwrap();
    let pasta_pc = tempfile::tempdir().unwrap();
    let web = servir_web(pasta_web.path(), hub).await;

    // Pela rota que o navegador vai chamar.
    let resposta = reqwest::Client::new()
        .post(format!("http://{web}/api/capturar"))
        .json(&serde_json::json!({ "texto": "testar aquela biblioteca depois" }))
        .send()
        .await
        .unwrap();
    assert!(resposta.status().is_success(), "capturar falhou");

    // A resposta NAO espera o sync, entao aqui e preciso dar tempo ao empurrao
    // de segundo plano. Espera por uma CONDICAO, e nao por um numero fixo:
    // dormir "o bastante" e como um teste instavel comeca.
    let caminho_pc = pasta_pc.path().to_path_buf();
    let pc = tokio::task::spawn_blocking(move || {
        let pc = outro_aparelho(&caminho_pc);
        for _ in 0..50 {
            if sincronizar(&pc, hub).recebidas > 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        pc
    })
    .await
    .unwrap();

    let capturas = pc.by_lifecycle(LifecycleState::Active, 50).unwrap();
    assert_eq!(capturas.len(), 1, "a captura nao chegou no PC");
    assert_eq!(capturas[0].content, "testar aquela biblioteca depois");
}

/// E a volta: a Task criada no PC aparece na lista do bolso.
#[tokio::test(flavor = "multi_thread")]
async fn a_task_do_pc_aparece_no_bolso() {
    let hub = servir_hub().await;
    let pasta_web = tempfile::tempdir().unwrap();
    let pasta_pc = tempfile::tempdir().unwrap();
    let web = servir_web(pasta_web.path(), hub).await;

    let caminho_pc = pasta_pc.path().to_path_buf();
    tokio::task::spawn_blocking(move || {
        let pc = outro_aparelho(&caminho_pc);
        let nova = mos_core::NewTask::create("Comprar cimento", "", None).unwrap();
        pc.create_task(nova).unwrap();
        assert_eq!(sincronizar(&pc, hub).enviadas, 1);
    })
    .await
    .unwrap();

    // O laco de fundo do `mos-web` roda a cada minuto; o teste nao espera por
    // ele. Cada captura dispara o empurrao, que PUXA junto — o mesmo gatilho que
    // o uso real tem.
    let mut encontrada = false;
    for _ in 0..50 {
        let itens: serde_json::Value = reqwest::get(format!("http://{web}/api/tasks"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        if itens.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
            assert_eq!(itens[0]["title"], "Comprar cimento");
            encontrada = true;
            break;
        }
        let _ = reqwest::Client::new()
            .post(format!("http://{web}/api/capturar"))
            .json(&serde_json::json!({ "texto": "ping" }))
            .send()
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(encontrada, "a Task do PC nao apareceu no bolso");
}

/// O lembrete criado no bolso chega no PC, apontando para a Task certa.
///
/// # Por que este teste existe, e nao so o de captura
///
/// Porque o `avisos.rs` abre dizendo que este aparelho **le** lembretes e **nao
/// escreve** nenhum, e criar um e uma escrita. A fronteira que continua de pe e
/// outra: ele nao escreve estado de ENTREGA — isso o desktop tambem faz, e dois
/// agendadores na mesma coluna produzem o lembrete que some do PC. Criar e a
/// pessoa decidindo, uma vez, e precisa sincronizar como a Task ja sincroniza.
///
/// Um lembrete que fica so no celular seria pior que nenhum: ele apareceria na
/// lista, tocaria no bolso, e nunca existiria no PC — que e onde a pessoa
/// trabalha.
#[tokio::test(flavor = "multi_thread")]
async fn o_lembrete_do_bolso_chega_no_pc() {
    use mos_core::AttentionRepository;

    let hub = servir_hub().await;
    let pasta_web = tempfile::tempdir().unwrap();
    let pasta_pc = tempfile::tempdir().unwrap();
    let web = servir_web(pasta_web.path(), hub).await;
    let cliente = reqwest::Client::new();

    // A Task nasce no bolso, para o lembrete ter um alvo real. Um id inventado
    // passaria pela rota e provaria menos: o que se quer conferir e que o alvo
    // sobrevive a viagem.
    let task: serde_json::Value = cliente
        .post(format!("http://{web}/api/tasks"))
        .json(&serde_json::json!({ "titulo": "Levar o notebook" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // O instante chega RESOLVIDO, como a tela manda. Uma hora a frente para o
    // lembrete nascer `scheduled` e nao vencido.
    let quando = time::OffsetDateTime::now_utc() + time::Duration::hours(1);
    let resposta = cliente
        .post(format!("http://{web}/api/lembretes"))
        .json(&serde_json::json!({
            "titulo": "Levar o notebook",
            "quando": quando
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            "alvo_tipo": "task",
            "alvo_id": task_id,
        }))
        .send()
        .await
        .unwrap();
    assert!(resposta.status().is_success(), "criar lembrete falhou");

    // E ele aparece na lista do proprio bolso, que e o que a aba desenha.
    let lista: serde_json::Value = cliente
        .get(format!("http://{web}/api/lembretes"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(lista.as_array().map(Vec::len), Some(1));
    assert_eq!(lista[0]["target"]["type"], "task");
    assert_eq!(lista[0]["target"]["id"], task_id.as_str());

    // Espera por CONDICAO, e nao por um numero fixo de milissegundos.
    let caminho_pc = pasta_pc.path().to_path_buf();
    let pc = tokio::task::spawn_blocking(move || {
        let pc = outro_aparelho(&caminho_pc);
        for _ in 0..50 {
            sincronizar(&pc, hub);
            if !pc.open_reminders().unwrap().is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        pc
    })
    .await
    .unwrap();

    let lembretes = pc.open_reminders().unwrap();
    assert_eq!(lembretes.len(), 1, "o lembrete do bolso nao chegou no PC");
    assert_eq!(lembretes[0].title, "Levar o notebook");
    assert_eq!(
        lembretes[0].target,
        Some(mos_core::ReminderTarget::Task(
            mos_core::TaskId::parse(&task_id).unwrap()
        )),
        "o lembrete chegou solto: o alvo se perdeu na viagem"
    );
}

/// Concluir no bolso tira o lembrete da lista.
///
/// Concluir e cancelar levam a estado TERMINAL, e e por isso que estao na porta
/// enquanto `snooze` nao esta: depois delas nenhum agendador olha mais para o
/// lembrete, e nao ha o que os dois aparelhos disputem.
#[tokio::test(flavor = "multi_thread")]
async fn concluir_no_bolso_tira_da_lista() {
    let hub = servir_hub().await;
    let pasta = tempfile::tempdir().unwrap();
    let web = servir_web(pasta.path(), hub).await;
    let cliente = reqwest::Client::new();

    let quando = time::OffsetDateTime::now_utc() + time::Duration::hours(2);
    let criado: serde_json::Value = cliente
        .post(format!("http://{web}/api/lembretes"))
        .json(&serde_json::json!({
            "titulo": "Ligar para a marcenaria",
            "quando": quando
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = criado["id"].as_str().unwrap();

    let resposta = cliente
        .post(format!("http://{web}/api/lembretes/{id}/concluir"))
        .send()
        .await
        .unwrap();
    assert!(resposta.status().is_success(), "concluir falhou");

    let lista: serde_json::Value = cliente
        .get(format!("http://{web}/api/lembretes"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        lista.as_array().map(Vec::len),
        Some(0),
        "o lembrete concluido continua cobrando"
    );
}

/// Escrever duas vezes seguidas nao trava o servidor.
///
/// # O defeito que este teste guarda
///
/// O `SqliteStorage` tem dois cadeados — a conexao e o relogio logico — e os
/// dois caminhos os pegavam em ORDEM CONTRARIA: uma escrita fecha a conexao e
/// depois pede o relogio; uma rodada de sync fecha o relogio e depois pede a
/// conexao. Duas ordens contrarias sao um abraco mortal, e como TODA escrita
/// dispara uma rodada em segundo plano, bastava a segunda escrita cair dentro da
/// rodada da primeira.
///
/// Isso nao era um caso de borda: era capturar duas coisas seguidas. O servidor
/// travava **para sempre**, calado — sem log, sem erro, sem 500 —, e continuava
/// travado depois de fechar e abrir o app. Ver `estado::Estado::vez`.
///
/// Seis escritas seguidas, e nao duas, porque o encontro depende de a segunda
/// cair DENTRO da rodada da primeira: uma so tentativa poderia passar por sorte
/// de tempo, e um teste que passa por sorte nao guarda nada.
#[tokio::test(flavor = "multi_thread")]
async fn escrever_em_rajada_nao_trava() {
    let hub = servir_hub().await;
    let pasta = tempfile::tempdir().unwrap();
    let web = servir_web(pasta.path(), hub).await;
    let cliente = reqwest::Client::new();

    for numero in 0..6 {
        let resposta = cliente
            .post(format!("http://{web}/api/capturar"))
            .json(&serde_json::json!({ "texto": format!("ideia {numero}") }))
            .send()
            .await
            .unwrap();
        assert!(resposta.status().is_success(), "a captura {numero} falhou");
    }

    // E o banco tem as seis: travar nao e o unico jeito de perder escrita.
    let inbox: serde_json::Value = cliente
        .get(format!("http://{web}/api/inbox"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(inbox.as_array().map(Vec::len), Some(6));
}

/// A PWA sai de dentro do binario.
///
/// Sem isto o servidor sobe, a API responde e a pessoa ve uma pagina em branco —
/// o pior sintoma possivel, porque nada indica onde procurar. O teste falha na
/// compilacao se a pasta `static/` nao existir, e falha aqui se ela existir
/// vazia.
#[tokio::test(flavor = "multi_thread")]
async fn a_pagina_vem_embutida() {
    let hub = servir_hub().await;
    let pasta = tempfile::tempdir().unwrap();
    let backups = pasta.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let estado = mos_web::estado::Estado::abrir(
        pasta.path().join("web.db").to_str().unwrap(),
        backups.to_str().unwrap(),
        Some(mos_web::estado::Hub {
            url: format!("http://{hub}"),
            token: TOKEN.to_owned(),
        }),
        // Sem push: estes testes conferem o laco de dados, e uma chave VAPID
        // aqui so acrescentaria uma ida a rede que nao tem para onde ir.
        None,
        // Sem porta: quem confere a porta e o `tests/a_porta.rs`, e exigir
        // sessao aqui obrigaria todo teste de dados a fazer login antes.
        None,
    )
    .unwrap();

    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = ouvinte.local_addr().unwrap();
    let rotas = mos_web::api::rotas_com_pagina().with_state(estado);
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });

    let raiz = reqwest::get(format!("http://{endereco}/")).await.unwrap();
    assert!(raiz.status().is_success());
    let html = raiz.text().await.unwrap();
    assert!(html.contains("<div id=\"raiz\">"), "o index.html nao veio");
    assert!(
        html.contains("manifest.webmanifest"),
        "sem manifest nao ha 'Adicionar a Tela de Inicio'"
    );

    // Um caminho que nao existe devolve a pagina, e nao 404: a PWA e uma pagina
    // so, e um app instalado recarrega numa rota interna o tempo todo.
    let interna = reqwest::get(format!("http://{endereco}/tasks"))
        .await
        .unwrap();
    assert!(interna.status().is_success(), "rota interna deu 404");

    let manifest = reqwest::get(format!("http://{endereco}/manifest.webmanifest"))
        .await
        .unwrap();
    assert!(manifest.status().is_success());
    assert!(manifest.text().await.unwrap().contains("standalone"));

    // Mas um ARQUIVO CARIMBADO que nao existe tem que dar 404 de verdade.
    //
    // Ele so e pedido quando o navegador guardou um `index.html` velho, o de
    // antes do deploy. Devolver a pagina ali fazia o navegador pedir um `.js`,
    // receber HTML, recusar por causa do `Content-Type` e nao mostrar erro
    // nenhum: tela branca no app instalado, sem nada dizendo o que houve. Foi
    // exatamente o que aconteceu no iPhone, duas vezes.
    for carimbado in ["/assets/index-DEADBEEF.js", "/fontes/inexistente-v1.woff2"] {
        let resposta = reqwest::get(format!("http://{endereco}{carimbado}"))
            .await
            .unwrap();
        assert_eq!(
            resposta.status(),
            reqwest::StatusCode::NOT_FOUND,
            "{carimbado} devolveu a pagina em vez de 404 — a tela branca volta"
        );
    }
}

/// O panorama soma a semana no fuso de QUEM PERGUNTA.
///
/// O servidor roda em UTC e quem pergunta esta em UTC-3. Uma hora lancada as 22h
/// de domingo no fuso de quem le e domingo — mas ja e segunda em UTC, e cairia
/// na semana seguinte se o corte fosse do servidor. Este teste fixa o instante
/// exatamente nessa fresta.
#[tokio::test(flavor = "multi_thread")]
async fn o_panorama_corta_a_semana_no_fuso_do_aparelho() {
    use mos_core::{NewProject, NewTimeEntry, TimeTrackingRepository, WorkRepository};

    let hub = servir_hub().await;
    let pasta_web = tempfile::tempdir().unwrap();

    // O banco do bolso, semeado ANTES de subir o servidor: as horas ja existem
    // quando alguem pergunta, que e o caso real depois do sync.
    let backups = pasta_web.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    {
        let storage = SqliteStorage::open(pasta_web.path().join("web.db"), &backups).unwrap();
        let projeto = NewProject::create("Rancho Queimado", "", "").unwrap();
        let id = projeto.id;
        storage.create_project(projeto).unwrap();

        // Domingo, 22h em UTC-3 — que em UTC ja e segunda, 01h.
        let domingo_tarde = time::OffsetDateTime::parse(
            "2026-09-06T22:00:00-03:00",
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap();
        storage
            .create_time_entry(NewTimeEntry {
                project_id: id,
                started_at: domingo_tarde,
                ended_at: None,
                duration_seconds: 3_600,
                idle_seconds: 0,
                description: String::new(),
                activity_type: mos_core::ActivityType::Drawing,
                billable: true,
                hourly_rate_snapshot_cents: 3_000,
                source: mos_core::EntrySource::Manual,
            })
            .unwrap();
    }

    let web = servir_web(pasta_web.path(), hub).await;

    // Pergunta na segunda seguinte, 09h no fuso de quem le. A hora de domingo
    // pertence a semana ANTERIOR nesse fuso, entao a semana atual tem zero.
    let segunda = "2026-09-07T09:00:00-03:00";
    let panorama: serde_json::Value = reqwest::Client::new()
        .get(format!("http://{web}/api/panorama?agora={segunda}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        panorama["horas"]["semanaSegundos"], 0,
        "a hora de domingo entrou na semana da segunda: o corte usou o fuso errado"
    );

    // E perguntando no proprio domingo, ela conta.
    let domingo = "2026-09-06T23:00:00-03:00";
    let panorama: serde_json::Value = reqwest::Client::new()
        .get(format!("http://{web}/api/panorama?agora={domingo}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(panorama["horas"]["semanaSegundos"], 3_600);
    assert_eq!(panorama["horas"]["semanaValorCents"], 3_000);
    assert_eq!(
        panorama["horas"]["hojeSegundos"], 3_600,
        "a hora de hoje nao entrou no total do dia"
    );
}

/// A agenda devolve o que aconteceu na janela, e nada fora dela.
///
/// A composicao e do `mos_core::compose`, que ja tem teste proprio. O que ESTE
/// teste prova e o que so existe aqui: que o bolso alimenta a composicao com as
/// fontes certas, e que a janela vem do aparelho.
#[tokio::test(flavor = "multi_thread")]
async fn a_agenda_devolve_o_que_esta_na_janela() {
    use mos_core::{NewProject, NewTimeEntry, TimeTrackingRepository, WorkRepository};

    let hub = servir_hub().await;
    let pasta_web = tempfile::tempdir().unwrap();
    let backups = pasta_web.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    {
        let storage = SqliteStorage::open(pasta_web.path().join("web.db"), &backups).unwrap();
        let projeto = NewProject::create("Rancho Queimado", "", "").unwrap();
        let id = projeto.id;
        storage.create_project(projeto).unwrap();

        let quando = |texto: &str| {
            time::OffsetDateTime::parse(texto, &time::format_description::well_known::Rfc3339)
                .unwrap()
        };
        for (inicio, descricao) in [
            ("2026-09-10T13:00:00-03:00", "dentro da janela"),
            ("2026-09-20T13:00:00-03:00", "fora da janela"),
        ] {
            storage
                .create_time_entry(NewTimeEntry {
                    project_id: id,
                    started_at: quando(inicio),
                    ended_at: None,
                    duration_seconds: 3_600,
                    idle_seconds: 0,
                    description: descricao.to_owned(),
                    activity_type: mos_core::ActivityType::Drawing,
                    billable: true,
                    hourly_rate_snapshot_cents: 3_000,
                    source: mos_core::EntrySource::Manual,
                })
                .unwrap();
        }
    }

    let web = servir_web(pasta_web.path(), hub).await;
    let itens: Vec<serde_json::Value> = reqwest::Client::new()
        .get(format!(
            "http://{web}/api/agenda?desde={}&ate={}",
            urlencoding("2026-09-09T00:00:00-03:00"),
            urlencoding("2026-09-11T23:59:59-03:00")
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let titulos: Vec<String> = itens
        .iter()
        .map(|item| item["title"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(
        itens.len(),
        1,
        "a janela trouxe o que esta fora dela: {titulos:?}"
    );
    assert!(
        titulos[0].contains("Rancho Queimado"),
        "o item veio sem o nome do projeto: {titulos:?}"
    );

    // Fim antes do inicio e pedido malformado, e nao lista vazia: devolver vazio
    // faria uma janela invertida parecer um dia sem nada.
    let resposta = reqwest::Client::new()
        .get(format!(
            "http://{web}/api/agenda?desde={}&ate={}",
            urlencoding("2026-09-11T00:00:00-03:00"),
            urlencoding("2026-09-09T00:00:00-03:00")
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resposta.status(), 400);
}

/// `+` num RFC3339 vira espaco na query string se ninguem escapar.
fn urlencoding(texto: &str) -> String {
    texto.replace(':', "%3A").replace('+', "%2B")
}

/// Editar no bolso chega editado no PC.
///
/// Este teste é o que sustenta a promessa da auditoria: editar não é uma
/// operação do `mos-web`, é uma operação do M/OS que a tela do bolso alcança. Se
/// a edição não atravessasse, ela seria exatamente o sistema paralelo que o
/// desenho recusa.
///
/// Muda TÍTULO e HORA na mesma chamada, e confere os dois do outro lado: o
/// campo que não viaja é o que mais dá trabalho para descobrir depois.
#[tokio::test(flavor = "multi_thread")]
async fn editar_o_lembrete_no_bolso_chega_editado_no_pc() {
    use mos_core::AttentionRepository;

    let hub = servir_hub().await;
    let pasta_web = tempfile::tempdir().unwrap();
    let pasta_pc = tempfile::tempdir().unwrap();
    let web = servir_web(pasta_web.path(), hub).await;
    let cliente = reqwest::Client::new();

    let quando = time::OffsetDateTime::now_utc() + time::Duration::hours(1);
    let criado: serde_json::Value = cliente
        .post(format!("http://{web}/api/lembretes"))
        .json(&serde_json::json!({
            "titulo": "Ligar pro dentista",
            "nota": "confirmar o horário",
            "quando": quando
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = criado["id"].as_str().unwrap().to_owned();

    let remarcado = time::OffsetDateTime::now_utc() + time::Duration::hours(30);
    let resposta = cliente
        .patch(format!("http://{web}/api/lembretes/{id}"))
        .json(&serde_json::json!({
            "titulo": "Ligar pro dentista e remarcar",
            "quando": remarcado
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
        }))
        .send()
        .await
        .unwrap();
    assert!(resposta.status().is_success(), "editar falhou");

    // A nota NÃO foi mandada, e por isso não pode ter sumido: ausência é "não
    // mexi", e não "apague". É a regra que impede a tela que edita só a hora de
    // limpar o corpo do lembrete sem querer.
    let depois: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(depois["body"], "confirmar o horário");
    assert_eq!(depois["title"], "Ligar pro dentista e remarcar");

    let caminho_pc = pasta_pc.path().to_path_buf();
    let esperado = "Ligar pro dentista e remarcar";
    let pc = tokio::task::spawn_blocking(move || {
        let pc = outro_aparelho(&caminho_pc);
        for _ in 0..50 {
            sincronizar(&pc, hub);
            let chegou = pc
                .open_reminders()
                .unwrap()
                .iter()
                .any(|lembrete| lembrete.title == esperado);
            if chegou {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        pc
    })
    .await
    .unwrap();

    let lembretes = pc.open_reminders().unwrap();
    assert_eq!(lembretes.len(), 1, "o lembrete nao chegou no PC");
    assert_eq!(
        lembretes[0].title, esperado,
        "o titulo editado no bolso nao atravessou"
    );
    assert_eq!(
        lembretes[0].body, "confirmar o horário",
        "a nota que ninguem tocou foi apagada na viagem"
    );
    let no_pc = lembretes[0].next_due_at.unwrap();
    assert!(
        (no_pc - remarcado).abs() < time::Duration::seconds(2),
        "a hora remarcada nao atravessou: PC tem {no_pc}, bolso mandou {remarcado}"
    );
}

/// Adiar no bolso empurra a hora e conta a fadiga.
#[tokio::test(flavor = "multi_thread")]
async fn adiar_no_bolso_empurra_a_hora() {
    let hub = servir_hub().await;
    let pasta = tempfile::tempdir().unwrap();
    let web = servir_web(pasta.path(), hub).await;
    let cliente = reqwest::Client::new();

    let quando = time::OffsetDateTime::now_utc() + time::Duration::hours(1);
    let criado: serde_json::Value = cliente
        .post(format!("http://{web}/api/lembretes"))
        .json(&serde_json::json!({
            "titulo": "Trocar o filtro",
            "quando": quando
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = criado["id"].as_str().unwrap().to_owned();

    let ate = time::OffsetDateTime::now_utc() + time::Duration::hours(5);
    let adiado: serde_json::Value = cliente
        .post(format!("http://{web}/api/lembretes/{id}/adiar"))
        .json(&serde_json::json!({
            "ate": ate
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(adiado["status"], "snoozed");
    assert_eq!(adiado["snoozeCount"], 1, "adiar tem que contar");
}

/// Arquivar tira da lista aberta sem apagar a linha.
///
/// Arquivar e nao apagar: um toque errado no onibus nao deveria ser
/// irreversivel, e o "excluir" da tela do bolso e este.
#[tokio::test(flavor = "multi_thread")]
async fn arquivar_no_bolso_tira_da_lista_sem_apagar() {
    let hub = servir_hub().await;
    let pasta = tempfile::tempdir().unwrap();
    let web = servir_web(pasta.path(), hub).await;
    let cliente = reqwest::Client::new();

    let quando = time::OffsetDateTime::now_utc() + time::Duration::hours(1);
    let criado: serde_json::Value = cliente
        .post(format!("http://{web}/api/lembretes"))
        .json(&serde_json::json!({
            "titulo": "Isto foi engano",
            "quando": quando
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = criado["id"].as_str().unwrap().to_owned();

    cliente
        .post(format!("http://{web}/api/lembretes/{id}/arquivar"))
        .send()
        .await
        .unwrap();

    let lista: serde_json::Value = cliente
        .get(format!("http://{web}/api/lembretes"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        lista.as_array().map(Vec::len),
        Some(0),
        "o arquivado continua na lista aberta"
    );

    // Mas a linha continua la: buscar pelo id ainda responde.
    let ainda: serde_json::Value = cliente
        .get(format!("http://{web}/api/lembretes/{id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ainda["id"], id.as_str(), "arquivar apagou a linha");
    assert_eq!(ainda["lifecycleState"], "archived");
}
