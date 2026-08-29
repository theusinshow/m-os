//! A faixa de uso: o laco que le os transcripts e os comandos que ela chama.
//!
//! **O renderer nunca e dono do tempo**, pelo mesmo motivo do `attention.rs`: a
//! varredura tem de sobreviver a reload do front e a janela fechada. Quem
//! acorda de trinta em trinta segundos e este laco.
//!
//! # A primeira carga e diferente das outras
//!
//! Sao 507 MB de transcript nesta maquina. A primeira passada le tudo, e leva o
//! tempo que levar; as seguintes leem so o que cresceu, e custam milissegundos.
//!
//! Enquanto a primeira nao termina, [`Faixa::calibrando`] e `true` — e a faixa
//! desenha o trilho e o numero absoluto, SEM porcentagem. Um anel preenchido
//! contra um pico que ainda nao foi observado seria o numero inventado que o
//! `Ring.tsx` proibe por escrito.

use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration as StdDuration,
};

use mos_core::CoreError;
use mos_storage_sqlite::{LeituraDeUso, SqliteStorage};
use mos_usage::{varrer, Fonte};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// De quanto em quanto tempo o laco volta a olhar o disco.
///
/// Trinta segundos porque o dado nao muda mais rapido que isso de forma util —
/// um request demora mais que trinta segundos para acontecer e ser gravado — e
/// porque a passada incremental so custa um `stat` por arquivo quando nada
/// mudou.
const INTERVALO: StdDuration = StdDuration::from_secs(30);

/// O estado que sobrevive entre passadas.
#[derive(Default)]
pub struct Uso {
    calibrando: AtomicBool,
}

/// O que a faixa desenha, para UMA fonte.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnelDaFaixa {
    pub nome: String,
    /// Milesimos de token-equivalente-de-input consumidos na janela corrente.
    pub peso: u64,
    /// A maior janela ja observada. Zero significa que nao ha regua.
    pub pico: u64,
    pub peso_hoje: u64,
    pub pico_dia: u64,
    /// Requisicoes da JANELA corrente.
    pub requisicoes: u64,
    /// Requisicoes do DIA. Separadas de proposito: contar o dia com o numero da
    /// sessao era o rotulo mentindo sobre o que media.
    pub requisicoes_hoje: u64,
    /// Quando a janela corrente fecha, em RFC 3339. `None` quando nao ha janela
    /// aberta — e ai nao ha o que resetar, e a faixa nao inventa um prazo.
    pub reseta_em: Option<String>,
    /// Quantas janelas o banco conhece.
    ///
    /// A faixa precisa disto para nao mentir no primeiro dia: com UMA janela
    /// conhecida, o pico E a sessao corrente, e a proporcao daria 100% por
    /// falta de comparacao, e nao por consumo alto.
    pub janelas_conhecidas: u64,
}

/// A faixa inteira.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Faixa {
    pub aneis: Vec<AnelDaFaixa>,
    /// A primeira carga ainda esta correndo: ha peso, mas nao ha regua.
    pub calibrando: bool,
    /// A faixa esta na lingueta.
    ///
    /// Viaja junto do dado, e nao num comando proprio, porque a tira ja pede
    /// isto na montagem e ja escuta o evento `usage` — um segundo caminho para
    /// o mesmo estado e um segundo jeito de ele ficar dessincronizado.
    pub recolhida: bool,
}

fn montar(
    leitura: LeituraDeUso,
    nome: &str,
    calibrando: bool,
    recolhida: bool,
) -> Result<Faixa, CoreError> {
    let (peso, requisicoes, reseta_em) = match &leitura.sessao {
        Some(sessao) => (
            sessao.peso,
            sessao.requisicoes,
            Some(
                sessao
                    .fim
                    .format(&time::format_description::well_known::Rfc3339)
                    .map_err(|causa| {
                        CoreError::new(
                            mos_core::ErrorCode::DataIntegrity,
                            format!("Falha ao formatar o fim da janela: {causa}"),
                            false,
                        )
                    })?,
            ),
        ),
        // Sem janela aberta o consumo da sessao e zero, e isso e um FATO — nao
        // um dado faltando. O anel mostra o trilho, que e o que zero significa.
        None => (0, 0, None),
    };

    Ok(Faixa {
        aneis: vec![AnelDaFaixa {
            nome: nome.to_string(),
            peso,
            pico: leitura.pico_sessao,
            peso_hoje: leitura.peso_hoje,
            pico_dia: leitura.pico_dia,
            requisicoes,
            requisicoes_hoje: leitura.requisicoes_hoje,
            reseta_em,
            janelas_conhecidas: leitura.janelas_conhecidas,
        }],
        calibrando,
        recolhida,
    })
}

/// O que a faixa desenha agora.
///
/// Sem fonte — maquina sem Claude Code — devolve zero aneis, e a faixa nao
/// monta. Ela nao aparece vazia esperando um dado que nunca vira.
#[tauri::command]
pub fn usage_faixa<R: Runtime>(app: AppHandle<R>) -> Result<Faixa, CoreError> {
    let Some(fonte) = Fonte::claude_code() else {
        return Ok(Faixa {
            aneis: Vec::new(),
            calibrando: false,
            recolhida: false,
        });
    };
    let calibrando = app
        .try_state::<Uso>()
        .map(|uso| uso.calibrando.load(Ordering::Relaxed))
        .unwrap_or(false);
    let estado = crate::services(&app)?;
    let recolhida = crate::load_settings(&estado.settings_path).faixa_recolhida;
    let storage = estado.storage.clone();
    let leitura = storage.usage_leitura(crate::surface::now_local(&app))?;
    montar(leitura, &fonte.nome, calibrando, recolhida)
}

/// Abre e fecha o painel.
///
/// # Por que uma SEGUNDA janela, e nao a mesma crescendo
///
/// A primeira versao redimensionava a propria faixa no hover. Ela nunca abriu,
/// e o motivo so apareceu com o app na tela: `set_size` e ignorado numa janela
/// `resizable: false`, e ligar `resizable: true` faz a janela sem decoracao
/// **parar de receber qualquer evento de mouse** no Windows — hover e clique,
/// os dois. Foi medido nos dois sentidos: com `false` o hover volta a chegar ao
/// renderer, com `true` some.
///
/// Entao nada e redimensionado. Sao duas janelas de tamanho fixo, e `show` e
/// `hide` funcionam em qualquer uma. De quebra cada janela passa a ter
/// exatamente o tamanho do que ela pinta — e pixel transparente sobrando e
/// clique roubado do desktop, porque o Tauri nao faz click-through por regiao.
#[tauri::command]
pub fn faixa_painel_alternar<R: Runtime>(app: AppHandle<R>) -> Result<bool, CoreError> {
    let Some(painel) = app.get_webview_window(JANELA_PAINEL) else {
        return Ok(false);
    };
    // A VISIBILIDADE e a fonte da verdade, e nao um booleano guardado no
    // renderer: o painel se fecha sozinho pelo proprio botao, e um estado
    // paralelo na tira ficaria invertido no clique seguinte.
    if painel.is_visible().unwrap_or(false) {
        let _ = painel.hide();
        return Ok(false);
    }
    if let Some(tira) = app.get_webview_window(JANELA_FAIXA) {
        encostar_a_esquerda(&painel, &tira);
    }
    let _ = painel.show();
    let _ = painel.set_always_on_top(true);
    Ok(true)
}

#[tauri::command]
pub fn faixa_painel_fechar<R: Runtime>(app: AppHandle<R>) -> Result<(), CoreError> {
    if let Some(painel) = app.get_webview_window(JANELA_PAINEL) {
        let _ = painel.hide();
    }
    Ok(())
}

/// Traz o M/OS para a frente.
#[tauri::command]
pub fn faixa_abrir_app<R: Runtime>(app: AppHandle<R>) -> Result<(), CoreError> {
    if let Some(principal) = app.get_webview_window("main") {
        let _ = principal.unminimize();
        let _ = principal.show();
        let _ = principal.set_focus();
    }
    Ok(())
}

pub const JANELA_FAIXA: &str = "faixa";
pub const JANELA_PAINEL: &str = "faixa-painel";

/// A largura da tira em repouso: o anel de 56, o respiro de 8 de cada lado e a
/// borda, mais o bastante para "SEM RÉGUA" caber numa linha so.
///
/// Medida, e nao arredondada para um numero bonito: cada pixel a mais e um pixel
/// de janela TRANSPARENTE que engole o clique do que estiver embaixo. Com 132
/// sobravam 44 pixels de buraco morto sobre o desktop, e com 260 de altura
/// sobravam cerca de 150.
const LARGURA_FAIXA: f64 = 96.0;
const ALTURA_FAIXA: f64 = 112.0;
// A lingueta e desenhada em CSS, e nao ha constante para ela aqui.
//
// # Por que recolher nao mexe na janela
//
// Foram tentados os dois caminhos, e os dois morreram na tela:
//
// * `set_size` e IGNORADO numa janela `resizable: false`, e ligar
//   `resizable: true` faz uma janela sem decoracao parar de receber qualquer
//   evento de mouse no Windows;
// * `set_position` funciona, e **mata a entrada da janela por uns quinze
//   segundos**. Medido com seis cliques alternados: OK, morto, morto, morto,
//   OK, morto. Um `hide` seguido de `show` depois do movimento nao recupera.
//
// Entao a janela da tira nasce onde vai morrer, e recolher e so uma classe no
// CSS. O preco esta escrito no `App.css`: recolhida, os 84 pixels do cartao
// continuam sendo janela transparente e continuam engolindo clique. Quem some
// de verdade e o item do tray, que esconde a janela inteira.

/// Cola a janela na borda direita, centrada na vertical.
///
/// Mesmo calculo que o `monitor.rs` faz para o lembrete: monitor corrente,
/// tamanho fisico, escala. A diferenca e a borda escolhida — o lembrete mora no
/// canto de baixo, e a faixa no meio da direita, que e onde o olho passa sem
/// procurar.
///
/// Chamada UMA vez, quando a faixa aparece. Recolher nao passa por aqui: mover
/// a janela mata a entrada dela — ver o comentario logo acima de
/// `LARGURA_FAIXA`.
///
/// O tamanho e PERGUNTADO a janela, e nao derivado das constantes. O
/// `tauri.conf.json` pede 96 de largura e o Windows entrega uma janela de 136:
/// a medida do config e a area de cliente, e sobra moldura invisivel. Posicionar
/// pela constante deixava 40 pixels da tira para fora da tela.
fn posicionar<R: Runtime>(janela: &tauri::WebviewWindow<R>) {
    let _ = janela.set_size(tauri::LogicalSize::new(LARGURA_FAIXA, ALTURA_FAIXA));
    let Ok(Some(monitor)) = janela.current_monitor() else {
        return;
    };
    let Ok(tamanho) = janela.outer_size() else {
        return;
    };
    let tela = monitor.size();
    let x = tela.width.saturating_sub(tamanho.width);
    let y = (tela.height.saturating_sub(tamanho.height)) / 2;
    let _ = janela.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
}

/// Encosta o painel na tira, alinhado pelo topo dela.
///
/// Em pixels FISICOS, lidos da tira que ja esta na tela: derivar a posicao do
/// painel de um calculo proprio faria os dois discordarem em qualquer monitor
/// com escala diferente de 1.
fn encostar_a_esquerda<R: Runtime>(
    painel: &tauri::WebviewWindow<R>,
    tira: &tauri::WebviewWindow<R>,
) {
    let (Ok(onde), Ok(tamanho)) = (tira.outer_position(), painel.outer_size()) else {
        return;
    };
    let x = onde.x - tamanho.width as i32;
    let _ = painel.set_position(tauri::PhysicalPosition::new(x, onde.y));
}

/// Mostra a faixa, se houver fonte para desenhar.
///
/// Chamada DEPOIS da primeira passada, e nunca no `setup`. Uma janela mostrada
/// antes de a webview dela ter navegado desenha — o `PrintWindow` prova — e nao
/// recebe evento de mouse nenhum: a tira aparecia certinha na tela e o clique
/// nela nao chegava ao renderer. Esperar o primeiro dado tambem evita mostrar um
/// anel vazio por um instante.
pub fn abrir<R: Runtime>(app: &AppHandle<R>) {
    if Fonte::claude_code().is_none() {
        return;
    }
    let Some(janela) = app.get_webview_window(JANELA_FAIXA) else {
        return;
    };
    if oculta(app) {
        let _ = janela.hide();
        return;
    }
    posicionar(&janela);
    let _ = janela.show();
    // Sem roubar o foco, pela mesma razao do lembrete: quem esta com as maos no
    // teclado nao pediu por uma janela nova.
    let _ = janela.set_always_on_top(true);
}

/// A faixa esta desligada pelo tray?
///
/// So esta preferencia chega ate aqui. `faixa_recolhida` nao: recolher nao mexe
/// mais na janela, entao quem precisa dela e a tela, e ela viaja no payload de
/// [`Faixa`].
fn oculta<R: Runtime>(app: &AppHandle<R>) -> bool {
    // Sem `setup` terminado o padrao e a faixa aparecendo, que e o que um
    // `settings.json` sem o campo tambem diz.
    crate::services(app)
        .map(|estado| crate::load_settings(&estado.settings_path).faixa_oculta)
        .unwrap_or(false)
}

/// Recolhe a faixa na lingueta, ou a traz de volta.
///
/// So grava a preferencia e avisa a tela: a JANELA nao se mexe, porque move-la
/// mata a entrada dela. Quem esconde o cartao e o CSS.
///
/// Gravado no `settings.json` porque recolher e o gesto de quem nao quer ver
/// aquilo agora — e uma faixa que voltasse inteira a cada abertura obrigaria a
/// repetir o gesto todo dia.
#[tauri::command]
pub fn faixa_recolher<R: Runtime>(app: AppHandle<R>, recolhida: bool) -> Result<(), CoreError> {
    let estado = crate::services(&app)?;
    let mut settings = crate::load_settings(&estado.settings_path);
    settings.faixa_recolhida = recolhida;
    crate::save_settings(&estado.settings_path, &settings)?;

    // Recolher com o painel aberto deixaria um cartao de 440px flutuando ao lado
    // de uma lingueta de 12: o painel e do anel, e sem o anel na tela ele nao
    // tem dono.
    if recolhida {
        if let Some(painel) = app.get_webview_window(JANELA_PAINEL) {
            let _ = painel.hide();
        }
    }
    emitir(&app);
    Ok(())
}

/// Liga e desliga a faixa pelo item do tray.
///
/// Desligada, a janela some e o laco CONTINUA contando. Parar de ler seria
/// perder o consumo do periodo em que a faixa esteve escondida, e ao religa-la
/// o pico estaria errado — que e o unico numero que ela tem.
pub fn alternar_pela_bandeja<R: Runtime>(app: &AppHandle<R>) {
    let Ok(estado) = crate::services(app) else {
        return;
    };
    let mut settings = crate::load_settings(&estado.settings_path);
    settings.faixa_oculta = !settings.faixa_oculta;
    let oculta = settings.faixa_oculta;
    if crate::save_settings(&estado.settings_path, &settings).is_err() {
        return;
    }
    if oculta {
        for rotulo in [JANELA_FAIXA, JANELA_PAINEL] {
            if let Some(janela) = app.get_webview_window(rotulo) {
                let _ = janela.hide();
            }
        }
    } else {
        abrir(app);
    }
    marcar_na_bandeja(app, !oculta);
}

/// Poe a marca do item do tray de acordo com a preferencia gravada.
pub fn marcar_na_bandeja<R: Runtime>(app: &AppHandle<R>, marcado: bool) {
    if let Some(bandeja) = app.try_state::<crate::TrayHandles>() {
        for item in &bandeja.faixa {
            let _ = item.set_checked(marcado);
        }
    }
}

/// Manda a faixa redesenhar com o estado de agora.
fn emitir<R: Runtime>(app: &AppHandle<R>) {
    if let Ok(faixa) = usage_faixa(app.clone()) {
        let _ = app.emit("usage", faixa);
    }
}

/// Uma passada: le o que cresceu, grava, e avisa a faixa.
///
/// Devolve quantas requisicoes eram novas — zero e o caso comum e nao gera
/// evento, porque redesenhar a faixa com o mesmo numero e trabalho a toa.
fn passada(storage: &SqliteStorage, fonte: &Fonte) -> Result<u64, CoreError> {
    let conhecidos: HashMap<_, _> = storage.usage_ponteiros()?;
    let varredura = varrer(&fonte.raiz, &conhecidos);
    storage.usage_registrar(&varredura.eventos, &varredura.ponteiros)
}

pub async fn run<R: Runtime>(app: AppHandle<R>) {
    let Some(fonte) = Fonte::claude_code() else {
        // Sem Claude Code nao ha o que ler, e o laco nem comeca. Um laco que
        // acorda de trinta em trinta segundos para nao achar nada e so um
        // consumo de bateria com nome bonito.
        return;
    };

    if let Some(uso) = app.try_state::<Uso>() {
        uso.calibrando.store(true, Ordering::Relaxed);
    }
    let mut mostrada = false;
    let mut marcou = false;

    loop {
        let storage = match crate::services(&app) {
            Ok(estado) => estado.storage.clone(),
            // O `setup` ainda nao terminou. Esperar a proxima volta e o certo:
            // a corrida de abertura ja mordeu o `attention_count` uma vez.
            Err(_) => {
                tokio::time::sleep(INTERVALO).await;
                continue;
            }
        };

        let fonte_da_vez = fonte.clone();
        // Em thread de bloqueio: a primeira carga le meio giga, e faze-lo no
        // executor async prenderia o runtime inteiro por minutos.
        let resultado =
            tauri::async_runtime::spawn_blocking(move || passada(&storage, &fonte_da_vez)).await;

        let novas = match resultado {
            Ok(Ok(novas)) => Some(novas),
            Ok(Err(causa)) => {
                crate::diagnostico::escrever(
                    crate::diagnostico::Nivel::Aviso,
                    "faixa",
                    &format!("a varredura de uso falhou: {causa}"),
                );
                None
            }
            Err(causa) => {
                crate::diagnostico::escrever(
                    crate::diagnostico::Nivel::Aviso,
                    "faixa",
                    &format!("a varredura de uso nao terminou: {causa}"),
                );
                None
            }
        };

        // A calibragem acaba na primeira passada COMPLETA, e nao na primeira
        // que devolve numero: se ela falhou, o pico continua sem base e a faixa
        // continua sem porcentagem.
        let primeira = if let Some(uso) = app.try_state::<Uso>() {
            novas.is_some() && uso.calibrando.swap(false, Ordering::Relaxed)
        } else {
            false
        };

        // A marca do tray so pode ser corrigida depois do `setup`: e la que o
        // caminho do `settings.json` passa a existir.
        if !marcou {
            marcar_na_bandeja(&app, !oculta(&app));
            marcou = true;
        }

        if !mostrada && novas.is_some() {
            abrir(&app);
            mostrada = true;
        }

        if primeira || novas.is_some_and(|quantas| quantas > 0) {
            emitir(&app);
        }

        tokio::time::sleep(INTERVALO).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_storage_sqlite::Sessao;
    use time::macros::datetime;

    fn leitura() -> LeituraDeUso {
        LeituraDeUso {
            sessao: Some(Sessao {
                inicio: datetime!(2026-08-29 03:00:00 UTC),
                fim: datetime!(2026-08-29 08:00:00 UTC),
                peso: 500_000,
                requisicoes: 12,
            }),
            pico_sessao: 1_000_000,
            peso_hoje: 700_000,
            requisicoes_hoje: 31,
            pico_dia: 2_000_000,
            janelas_conhecidas: 9,
        }
    }

    #[test]
    fn a_faixa_leva_o_pico_e_o_prazo() {
        let faixa = montar(leitura(), "Claude Code", false, false).unwrap();
        let anel = &faixa.aneis[0];
        assert_eq!(anel.peso, 500_000);
        assert_eq!(anel.pico, 1_000_000);
        assert_eq!(anel.reseta_em.as_deref(), Some("2026-08-29T08:00:00Z"));
        assert!(!faixa.calibrando);
    }

    #[test]
    fn sem_janela_aberta_o_prazo_nao_e_inventado() {
        let leitura = LeituraDeUso {
            sessao: None,
            ..leitura()
        };
        let anel = &montar(leitura, "Claude Code", false, false).unwrap().aneis[0];
        assert_eq!(anel.peso, 0);
        assert_eq!(anel.requisicoes, 0);
        assert_eq!(anel.reseta_em, None, "sem janela nao ha o que resetar");
        assert_eq!(anel.pico, 1_000_000, "mas o pico historico continua");
    }

    #[test]
    fn calibrando_atravessa_ate_a_faixa() {
        let faixa = montar(leitura(), "Claude Code", true, false).unwrap();
        assert!(faixa.calibrando);
    }
}
