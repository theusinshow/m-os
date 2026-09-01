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
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration as StdDuration,
};

use mos_core::CoreError;
use mos_storage_sqlite::{LeituraDeUso, SqliteStorage};
use mos_usage::{cota, varrer, Fonte};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use time::OffsetDateTime;

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
    /// A tira esta na lingueta.
    ///
    /// Espelho em memoria do `faixa_recolhida` do `settings.json`, e nao a
    /// fonte da verdade dele. Existe por causa do [`vigiar_o_cursor`]: aquele
    /// laco precisa deste booleano dezenas de vezes por segundo, e ler o
    /// `settings.json` do disco nessa cadencia seria I/O continuo para receber
    /// sempre a mesma resposta. Quem escreve aqui e quem ja estava lendo o
    /// arquivo de qualquer jeito — [`usage_faixa`] e [`faixa_recolher`].
    recolhida: AtomicBool,
    /// O retangulo que a tira PINTA, em pixels logicos relativos a janela.
    ///
    /// Medido pelo React e enviado por [`faixa_zona`]. A primeira versao
    /// calculava isto no Rust a partir de `LARGURA_LINGUETA`, um espelho do
    /// `App.css` — e espelho de CSS envelhece calado. Com mais de um anel a
    /// conta ficaria pior ainda: a altura pintada passaria a depender de quantas
    /// fontes responderam.
    ///
    /// `None` ate a primeira medida chegar, e ai vale o calculo de reserva.
    zona: Mutex<Option<(f64, f64, f64, f64)>>,
    /// A ultima cota de cada fonte externa, na ordem do `settings.json`.
    externas: Mutex<Vec<Option<CotaObservada>>>,
    /// A ultima cota que o servidor respondeu, e quando.
    ///
    /// Em memoria e nao no banco: ela vale minutos, e um numero de cota
    /// sobrevivendo a um reinicio seria um numero velho apresentado como novo.
    cota: Mutex<Option<CotaObservada>>,
}

/// Um provedor de IA que nao e o Claude Code.
///
/// # O que isto desfaz
///
/// A ADR-059 deixou os outros provedores de fora com um argumento correto:
/// "tres aneis com dois deles inventados seria o erro que esta ADR recusa, em
/// triplicado". O motivo era a INVENCAO. A saida de um comando que o dono
/// escolheu e apontou nao e invencao nossa — e um numero com origem, que e
/// exatamente o que faltava.
///
/// # O comando roda SEM shell
///
/// `programa` e `argumentos` sao separados de proposito: uma linha de comando
/// unica passada a um shell transformaria um caminho com espaco em dois
/// argumentos, e transformaria um `&` num segundo comando. Aqui nao ha shell
/// para interpretar nada.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FonteExterna {
    /// O que aparece embaixo do anel.
    pub nome: String,
    pub programa: String,
    #[serde(default)]
    pub argumentos: Vec<String>,
}

/// Quantos aneis a tira desenha, no maximo.
///
/// A janela da tira NAO pode crescer: `set_size` e ignorado numa janela
/// `resizable: false`, e ligar `resizable: true` mata a entrada dela no Windows
/// — as duas coisas medidas na tela e registradas na ADR-059. Entao ela nasce do
/// tamanho de tres aneis e nunca muda, e a ADR-061 e o que torna isso barato:
/// o espaco que sobra e transparente e nao engole clique, porque a zona de
/// clique segue o que esta PINTADO.
///
/// Tres, e nao um numero maior, porque a partir dai a tira deixa de ser uma
/// tira e vira uma coluna: quatro aneis passam de 380 pixels de altura.
pub const MAX_ANEIS: usize = 3;

/// Quanto tempo um comando externo tem para responder.
///
/// Cinco segundos, e depois ele e morto. Um comando pendurado seguraria a
/// passada de todos os outros, e o preco de perder uma leitura e um tique — o
/// de travar o laco e a faixa inteira parada.
const COTA_EXTERNA_TIMEOUT: StdDuration = StdDuration::from_secs(5);

/// Uma resposta do servidor, com a hora em que ela chegou.
#[derive(Debug, Clone, Copy)]
struct CotaObservada {
    cota: cota::Cota,
    em: OffsetDateTime,
    /// Isto veio do `MOS_FAIXA_DEMO`, e nao do servidor.
    demo: bool,
}

/// Uma janela de cota do servidor, pronta para a tela.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JanelaDaFaixa {
    /// Quanto da janela ja foi, contra o teto DE VERDADE.
    pub percentual: u16,
    /// Quando ela zera, em RFC 3339. Vem do servidor, e nao de uma conta.
    pub reseta_em: Option<String>,
    /// Este numero e o ultimo que deu certo, e a renovacao esta falhando.
    ///
    /// A tela mostra assim mesmo, marcado. Ver [`COTA_VALIDADE`].
    pub obsoleta: bool,
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
    /// A cota REAL da janela de cinco horas, quando o servidor responde.
    ///
    /// Presente, ela e a regua: o anel passa a medir contra o teto do plano em
    /// vez do pico observado, e `reseta_em` passa a ser a hora que o servidor
    /// deu em vez do fim da janela calculado aqui. Ausente, tudo volta a ser
    /// como a ADR-059 deixou — e e por isso que `peso` e `pico` continuam
    /// viajando junto.
    pub cota_sessao: Option<JanelaDaFaixa>,
    /// A janela de sete dias. So existe com cota: o transcript nunca teve como
    /// saber onde a semana comeca nem quanto ela aguenta.
    pub cota_semana: Option<JanelaDaFaixa>,
    /// Esta fonte tem transcript lido daqui, e nao so um numero de cota.
    ///
    /// Falso nas fontes externas, e e o que impede o painel de desenhar a barra
    /// HOJE para elas: `peso` e `pico` viriam zerados, e uma barra vazia
    /// rotulada "HOJE" diria "nao consumiu nada hoje" — que e uma frase
    /// diferente de "esta fonte nao me conta o historico dela".
    pub tem_historico: bool,
}

/// A faixa inteira.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Faixa {
    pub aneis: Vec<AnelDaFaixa>,
    /// A primeira carga ainda esta correndo: ha peso, mas nao ha regua.
    pub calibrando: bool,
    /// A faixa esta desenhando numero INVENTADO, pedido pelo `MOS_FAIXA_DEMO`.
    ///
    /// Viaja ate a tela porque a tela tem de dizer isso. Um modo de
    /// demonstracao indistinguivel do real e exatamente o numero inventado que
    /// o `Ring.tsx` proibe — e aqui ele seria pior, porque teria a aparencia de
    /// cota conferida.
    pub demonstracao: bool,
    /// A faixa esta na lingueta.
    ///
    /// Viaja junto do dado, e nao num comando proprio, porque a tira ja pede
    /// isto na montagem e ja escuta o evento `usage` — um segundo caminho para
    /// o mesmo estado e um segundo jeito de ele ficar dessincronizado.
    pub recolhida: bool,
}

/// A cota observada vira payload, ou some por idade.
///
/// `agora` entra como parametro em vez de ser lido aqui para que o teste possa
/// envelhecer uma leitura sem esperar cinco minutos.
fn janela(
    limite: Option<cota::Limite>,
    observada_em: OffsetDateTime,
    agora: OffsetDateTime,
) -> Result<Option<JanelaDaFaixa>, CoreError> {
    let Some(limite) = limite else {
        return Ok(None);
    };
    let idade = agora - observada_em;
    if idade > COTA_VALIDADE {
        return Ok(None);
    }
    // Negativa e o caso da demonstracao, que se grava no futuro. `max(0)` para
    // que ela nao vire "velha" por aritmetica.
    let idade = idade.max(time::Duration::ZERO);
    let reseta_em = match limite.reseta_em {
        Some(quando) => Some(
            quando
                .format(&time::format_description::well_known::Rfc3339)
                .map_err(|causa| {
                    CoreError::new(
                        mos_core::ErrorCode::DataIntegrity,
                        format!("Falha ao formatar o reset da cota: {causa}"),
                        false,
                    )
                })?,
        ),
        None => None,
    };
    Ok(Some(JanelaDaFaixa {
        percentual: limite.percentual,
        reseta_em,
        // Uma leitura mais velha que o intervalo normal so pode ser uma que
        // falhou em renovar. Um respiro de meio intervalo evita marcar como
        // velho o dado que esta so a caminho.
        obsoleta: idade > COTA_INTERVALO * 3 / 2,
    }))
}

fn montar(
    leitura: LeituraDeUso,
    nome: &str,
    calibrando: bool,
    recolhida: bool,
    observada: Option<CotaObservada>,
    externas: &[(String, Option<CotaObservada>)],
    agora: OffsetDateTime,
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

    let demonstracao = observada.is_some_and(|observada| observada.demo);

    // Uma fonte externa so entra na faixa quando ela RESPONDEU. Um anel
    // permanente marcado "SEM RÉGUA" para um comando que nunca funcionou seria
    // ocupar a borda da tela com a lembranca de um erro de configuracao.
    let mut aneis = Vec::with_capacity(1 + externas.len());
    aneis.push(AnelDaFaixa {
        nome: nome.to_string(),
        peso,
        pico: leitura.pico_sessao,
        peso_hoje: leitura.peso_hoje,
        pico_dia: leitura.pico_dia,
        requisicoes,
        requisicoes_hoje: leitura.requisicoes_hoje,
        cota_sessao: match observada {
            Some(observada) => janela(observada.cota.sessao, observada.em, agora)?,
            None => None,
        },
        cota_semana: match observada {
            Some(observada) => janela(observada.cota.semana, observada.em, agora)?,
            None => None,
        },
        tem_historico: true,
        reseta_em,
        janelas_conhecidas: leitura.janelas_conhecidas,
    });

    for (nome, observada) in externas {
        if aneis.len() >= MAX_ANEIS {
            break;
        }
        let Some(observada) = observada else { continue };
        let sessao = janela(observada.cota.sessao, observada.em, agora)?;
        let semana = janela(observada.cota.semana, observada.em, agora)?;
        if sessao.is_none() && semana.is_none() {
            continue;
        }
        aneis.push(AnelDaFaixa {
            nome: nome.clone(),
            // Zerados, e nao ausentes, porque `tem_historico: false` e o campo
            // que responde por eles. Um `Option` em cada um espalharia a mesma
            // pergunta por seis lugares.
            peso: 0,
            pico: 0,
            peso_hoje: 0,
            pico_dia: 0,
            requisicoes: 0,
            requisicoes_hoje: 0,
            cota_sessao: sessao,
            cota_semana: semana,
            tem_historico: false,
            reseta_em: None,
            janelas_conhecidas: 0,
        });
    }

    Ok(Faixa {
        demonstracao,
        aneis,
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
            demonstracao: false,
        });
    };
    let calibrando = app
        .try_state::<Uso>()
        .map(|uso| uso.calibrando.load(Ordering::Relaxed))
        .unwrap_or(false);
    let estado = crate::services(&app)?;
    let recolhida = crate::load_settings(&estado.settings_path).faixa_recolhida;
    if let Some(uso) = app.try_state::<Uso>() {
        uso.recolhida.store(recolhida, Ordering::Relaxed);
    }
    let storage = estado.storage.clone();
    let leitura = storage.usage_leitura(crate::surface::now_local(&app))?;
    let observada = app
        .try_state::<Uso>()
        .and_then(|uso| uso.cota.lock().ok().and_then(|guarda| *guarda));

    // Os nomes vem do `settings.json` e as leituras do estado, casados pela
    // POSICAO. Guardar o nome junto da leitura duplicaria a fonte da verdade do
    // que se chama cada fonte, e um rename no arquivo deixaria a faixa com o
    // nome velho ate reiniciar.
    let configuradas = crate::load_settings(&estado.settings_path).faixa_fontes;
    let leituras = app
        .try_state::<Uso>()
        .and_then(|uso| uso.externas.lock().ok().map(|guarda| guarda.clone()))
        .unwrap_or_default();
    let externas: Vec<(String, Option<CotaObservada>)> = configuradas
        .into_iter()
        .enumerate()
        .map(|(indice, fonte)| (fonte.nome, leituras.get(indice).copied().flatten()))
        .collect();

    montar(
        leitura,
        &fonte.nome,
        calibrando,
        recolhida,
        observada,
        &externas,
        OffsetDateTime::now_utc(),
    )
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

/// O atalho que liga e desliga a faixa.
///
/// # Por que ele nao e so conveniencia
///
/// A ADR-060 registrou que "o gesto do tray e o UNICO caminho de esconder que
/// nao depende do clique na tira" — e naquela epoca a tira emudecia. A ADR-061
/// consertou o clique, e este atalho e o segundo caminho que nao depende dele:
/// dois caminhos independentes para o mesmo gesto, e nenhum deles precisa que o
/// outro funcione.
///
/// Fixo, e nao configuravel como os da Captura e da Voz. Aqueles competem por
/// teclas que o dono usa o dia inteiro; este liga e desliga uma tira de 96
/// pixels, e uma tela de Settings para ele custaria mais do que ele vale. Se
/// colidir com algo, o registro falha, o log diz, e os outros dois caminhos
/// continuam ali.
pub const ATALHO: &str = "CommandOrControl+Shift+U";

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
/// Altura para TRES aneis, e nao para um.
///
/// A janela nunca muda de tamanho — ver [`MAX_ANEIS`] —, entao ela nasce do
/// tamanho do maior caso. O que sobra e transparente e nao custa nada desde a
/// ADR-061: a zona de clique segue o que esta pintado, e nao a janela.
const ALTURA_FAIXA: f64 = 296.0;

/// A largura da lingueta, em pixels logicos.
///
/// Espelha `.faixa-lingueta` no `App.css`, e o espelho e proposital: quem
/// decide o que fica PINTADO na tela e o CSS, e quem decide o que RECEBE
/// clique e o Windows. Os dois numeros tem de concordar, e por isso cada
/// arquivo aponta para o outro.
const LARGURA_LINGUETA: f64 = 12.0;

/// De quanto em quanto tempo [`vigiar_o_cursor`] olha o ponteiro quando ele
/// esta longe da tira.
///
/// Cento e vinte milissegundos e o suficiente para o estado estar certo antes
/// de a mao chegar: ninguem atravessa 240 pixels e clica em menos que isso.
const VIGIA_LONGE: StdDuration = StdDuration::from_millis(120);

/// E quando ele esta perto.
///
/// Um quadro a 60Hz. Aqui a cadencia importa de verdade: entre o ponteiro
/// entrar no cartao e o vigia devolver o clique a janela ha esta janela de
/// tempo, e um clique dado dentro dela cai no desktop em vez de cair na tira.
const VIGIA_PERTO: StdDuration = StdDuration::from_millis(16);

/// De quanto em quanto tempo a cota e perguntada ao servidor.
///
/// Comecou em um minuto, que e o que o `agent-notch` usa. Rodando de verdade,
/// o servidor devolveu **429 Too Many Requests** — e nao por causa deste laco
/// sozinho: o proprio Claude Code consulta o mesmo endpoint enquanto trabalha,
/// e os dois somados passam do que ele aceita.
///
/// Dois minutos, entao. A janela medida e de CINCO HORAS: dois minutos de
/// atraso sao 0,7% dela, e nenhuma decisao muda por causa disso. Insistir em um
/// minuto compraria precisao que ninguem usa ao preco de um 429 — que custa a
/// leitura inteira, porque o recuo joga a proxima tentativa para longe.
const COTA_INTERVALO: StdDuration = StdDuration::from_secs(120);

/// O teto do recuo depois de uma falha.
///
/// O recuo dobra a cada erro e para aqui. Insistir de minuto em minuto contra um
/// servidor fora do ar seria barulho; desistir seria perder a cota quando ele
/// voltasse.
const COTA_RECUO_MAX: StdDuration = StdDuration::from_secs(300);

/// Por quanto tempo uma cota que falhou em renovar continua valendo.
///
/// # Por que um valor velho e melhor que nenhum
///
/// A doutrina do `Ring.tsx` proibe numero INVENTADO, e ela continua de pe: um
/// numero de cinco minutos atras nao e inventado, e velho. Apaga-lo por causa de
/// uma falha de rede "some a unica informacao que ainda valia" — a mesma frase
/// que o `atualizacao.rs` ja usa para nao apagar a data da ultima verificacao.
///
/// O que a tela deve e DIZER que ele e velho, e e o que o campo `obsoleta` faz.
/// Passados os cinco minutos ele some: numa janela de cinco horas, um dado mais
/// velho que isso ja pode estar longe.
const COTA_VALIDADE: time::Duration = time::Duration::minutes(5);

/// A que distancia da zona pintada o vigia acelera.
///
/// A alternativa seria correr a 60Hz o dia inteiro para responder "o ponteiro
/// continua do outro lado da tela" — que e a resposta em quase todo tique.
const RAIO_DE_APROXIMACAO: f64 = 240.0;
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
// CSS.
//
// O preco disso — recolhida, os 84 pixels do cartao continuavam sendo janela
// transparente e continuavam engolindo clique do desktop — foi pago ate a
// ADR-061. Quem o eliminou foi [`vigiar_o_cursor`]: a janela so reivindica o
// clique onde ela PINTA, e a zona pintada encolhe junto com o cartao.

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
    let (Ok(onde), Ok(tira_tamanho), Ok(tamanho)) = (
        tira.outer_position(),
        tira.outer_size(),
        painel.outer_size(),
    ) else {
        return;
    };
    let x = onde.x - tamanho.width as i32;
    // Centrado na tira, e nao alinhado pelo topo dela. A seta do painel sai do
    // meio da borda direita, e alinhar topos faria a seta apontar para o vazio
    // acima do primeiro anel numa janela que agora cabe tres.
    let meio = onde.y + tira_tamanho.height as i32 / 2;
    let y = meio - tamanho.height as i32 / 2;
    let _ = painel.set_position(tauri::PhysicalPosition::new(x, y));
}

/// Pergunta a cota ao servidor da Anthropic, de minuto em minuto.
///
/// # A regua deixou de ser o pico
///
/// A ADR-059 mediu o transcript e concluiu, com razao, que ele nao traz teto de
/// cota nem hora de reset — e por isso o anel media contra o maior consumo ja
/// observado nesta maquina. O "Revisar quando" dela dizia: o dia em que o teto e
/// o reset existirem, a regua muda.
///
/// Eles existem. Nao no arquivo: no servidor. O `~/.claude/.credentials.json`
/// tem o token OAuth do proprio CLI, e com ele a resposta traz `session` e
/// `weekly_all` com percentual e `resets_at`. Isto e a ADR-062.
///
/// # O pico nao foi embora
///
/// Ele continua sendo calculado, gravado e enviado. Sem credencial, com o token
/// vencido, sem rede ou com a resposta em formato novo, a faixa volta inteira
/// para a regua da ADR-059 — que continua correta, so menos precisa. Uma regua
/// de reserva que nunca roda e uma regua que nao existe.
///
/// # O laco nao escreve nada
///
/// Nem no banco nem na credencial. A cota vale minutos e mora na memoria: um
/// numero de cota que sobrevivesse a um reinicio seria um numero velho
/// apresentado como novo. E renovar o token e trabalho do Claude Code, nao
/// nosso — este laco le o arquivo dele e nunca o toca.
async fn perguntar_a_cota<R: Runtime>(app: AppHandle<R>) {
    let cliente = match reqwest::Client::builder()
        // Curto de proposito: a resposta seguinte vem em um minuto, e uma
        // conexao pendurada por trinta segundos so atrasaria a proxima.
        .timeout(StdDuration::from_secs(10))
        .build()
    {
        Ok(cliente) => cliente,
        Err(causa) => {
            crate::diagnostico::escrever(
                crate::diagnostico::Nivel::Aviso,
                "cota",
                &format!("o cliente HTTP nao subiu, a faixa fica no pico: {causa}"),
            );
            return;
        }
    };

    // Com a demonstracao ligada o laco NAO fala com rede: ele publica o numero
    // pedido e para. Perguntar ao servidor e jogar a resposta fora seria gastar
    // pedido para nao usar, e mascarar um erro de rede que ninguem veria.
    if let Some(inventada) = cota::demo() {
        crate::diagnostico::escrever(
            crate::diagnostico::Nivel::Aviso,
            "cota",
            &format!(
                "{} esta ligado: a faixa desenha numero INVENTADO e nao fala com rede",
                cota::VAR_DEMO
            ),
        );
        if let Some(uso) = app.try_state::<Uso>() {
            if let Ok(mut guarda) = uso.cota.lock() {
                *guarda = Some(CotaObservada {
                    cota: inventada,
                    // No futuro, de proposito: assim ela nunca envelhece e nunca
                    // ganha o `~` de valor velho, que ali seria uma segunda
                    // mentira em cima da primeira.
                    em: OffsetDateTime::now_utc() + time::Duration::days(365),
                    demo: true,
                });
            }
        }
        emitir(&app);
        return;
    }

    // Zero enquanto vai bem. Cada falha dobra a espera, ate COTA_RECUO_MAX.
    let mut falhas: u32 = 0;
    // Para nao repetir a mesma linha de log de minuto em minuto.
    let mut ja_avisou = false;

    loop {
        let espera = match buscar(&cliente).await {
            Ok(cota) => {
                if let Some(uso) = app.try_state::<Uso>() {
                    if let Ok(mut guarda) = uso.cota.lock() {
                        *guarda = Some(CotaObservada {
                            cota,
                            em: OffsetDateTime::now_utc(),
                            demo: false,
                        });
                    }
                }
                if ja_avisou {
                    crate::diagnostico::escrever(
                        crate::diagnostico::Nivel::Aviso,
                        "cota",
                        "a cota voltou a responder",
                    );
                    ja_avisou = false;
                }
                falhas = 0;
                emitir(&app);
                COTA_INTERVALO
            }
            Err(motivo) => {
                // UMA linha por episodio, e nao uma por tentativa: um servidor
                // fora do ar por uma hora encheria o log com a mesma frase
                // sessenta vezes.
                if !ja_avisou {
                    crate::diagnostico::escrever(
                        crate::diagnostico::Nivel::Aviso,
                        "cota",
                        &format!("a cota nao respondeu, a faixa cai no pico: {motivo}"),
                    );
                    ja_avisou = true;
                }
                falhas = falhas.saturating_add(1);
                // A faixa precisa redesenhar mesmo na falha: e o que faz o
                // numero velho ganhar a marca de velho, e depois sumir.
                emitir(&app);
                (COTA_INTERVALO * 2u32.saturating_pow(falhas.min(8))).min(COTA_RECUO_MAX)
            }
        };
        tokio::time::sleep(espera).await;
    }
}

/// Roda os comandos das fontes externas, no mesmo compasso da cota.
///
/// # A fronteira de espectador, de novo
///
/// Vale aqui o que vale para a credencial do Claude Code: nos LEMOS o que a
/// ferramenta do outro publica, e nao mexemos nela. O comando e escolhido e
/// apontado pelo dono no proprio `settings.json` — o M/OS nao descobre binario
/// sozinho, nao adivinha argumento e nao passa por shell.
///
/// # Uma falha nao apaga as outras
///
/// Cada fonte tem a sua vaga na lista, e uma que falha zera SO a sua. A
/// alternativa — recomecar a lista a cada volta — faria um comando quebrado
/// derrubar da tela os anéis que estavam certos.
async fn perguntar_as_externas<R: Runtime>(app: AppHandle<R>) {
    loop {
        let fontes = match crate::services(&app) {
            Ok(estado) => crate::load_settings(&estado.settings_path).faixa_fontes,
            Err(_) => Vec::new(),
        };
        if fontes.is_empty() {
            // Sem fonte configurada o laco dorme e volta a olhar: o
            // `settings.json` e editado a mao, e exigir reiniciar o app depois
            // de adicionar uma linha seria uma pegadinha.
            tokio::time::sleep(COTA_RECUO_MAX).await;
            continue;
        }

        let mut leituras = Vec::with_capacity(fontes.len());
        for fonte in &fontes {
            leituras.push(match rodar(fonte).await {
                Ok(cota) => Some(CotaObservada {
                    cota,
                    em: OffsetDateTime::now_utc(),
                    demo: false,
                }),
                Err(motivo) => {
                    crate::diagnostico::escrever(
                        crate::diagnostico::Nivel::Aviso,
                        "cota",
                        &format!("a fonte \"{}\" nao respondeu: {motivo}", fonte.nome),
                    );
                    None
                }
            });
        }

        if let Some(uso) = app.try_state::<Uso>() {
            if let Ok(mut guarda) = uso.externas.lock() {
                *guarda = leituras;
            }
        }
        emitir(&app);
        tokio::time::sleep(COTA_INTERVALO).await;
    }
}

/// Um comando, com prazo. O erro e uma frase para o log.
async fn rodar(fonte: &FonteExterna) -> Result<cota::Cota, String> {
    let mut comando = tokio::process::Command::new(&fonte.programa);
    comando.args(&fonte.argumentos);
    #[cfg(windows)]
    {
        // Sem janela de console piscando na cara de quem esta trabalhando.
        comando.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    let saida = tokio::time::timeout(COTA_EXTERNA_TIMEOUT, comando.output())
        .await
        .map_err(|_| format!("passou de {}s", COTA_EXTERNA_TIMEOUT.as_secs()))?
        .map_err(|causa| format!("nao deu para executar: {causa}"))?;

    if !saida.status.success() {
        return Err(format!("terminou em {}", saida.status));
    }
    let texto = String::from_utf8_lossy(&saida.stdout);
    cota::ler_fonte_externa(&texto)
        .ok_or_else(|| "a saida nao tinha sessionUsedPercent nem weeklyUsedPercent".to_string())
}

/// Uma tentativa. O erro e uma frase para o log, e nunca carrega o token.
async fn buscar(cliente: &reqwest::Client) -> Result<cota::Cota, String> {
    let credencial = cota::credencial().ok_or("sem credencial do Claude Code")?;
    if credencial.vencida(OffsetDateTime::now_utc()) {
        // Nao renovamos: o token e do Claude Code. Quando ele renovar, a
        // proxima volta le o arquivo novo e a cota volta sozinha.
        return Err("o token do Claude Code venceu".to_string());
    }

    let mut pedido = cliente.get(cota::ENDERECO).bearer_auth(&credencial.token);
    for (nome, valor) in cota::CABECALHOS {
        pedido = pedido.header(nome, valor);
    }

    let resposta = pedido.send().await.map_err(|causa| {
        // `causa` traz a URL, nunca o cabecalho — o token nao vaza para o log.
        format!("a requisicao falhou: {causa}")
    })?;
    let status = resposta.status();
    if !status.is_success() {
        return Err(format!("o servidor respondeu {status}"));
    }
    let corpo = resposta
        .text()
        .await
        .map_err(|causa| format!("o corpo nao veio inteiro: {causa}"))?;

    cota::ler_resposta(&corpo).ok_or_else(|| {
        "a resposta nao trouxe nenhuma das duas janelas (o formato mudou?)".to_string()
    })
}

/// Um retangulo da TELA, em pixels fisicos.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Zona {
    x: f64,
    y: f64,
    largura: f64,
    altura: f64,
}

impl Zona {
    fn contem(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.largura && y >= self.y && y < self.y + self.altura
    }

    /// A menor distancia do ponto ate a borda. Zero de dentro.
    fn distancia(&self, x: f64, y: f64) -> f64 {
        let dx = (self.x - x).max(0.0).max(x - (self.x + self.largura));
        let dy = (self.y - y).max(0.0).max(y - (self.y + self.altura));
        dx.hypot(dy)
    }
}

/// Onde a tira efetivamente PINTA, dado o retangulo de cliente dela.
///
/// Aberta, a janela inteira: `.faixa-shell` e `.faixa-tira` ocupam 100% dela
/// justamente para que nao sobre pixel transparente. Recolhida, so a lingueta —
/// e ela mora na DIREITA, porque o `.faixa-shell` e `row-reverse` para ficar
/// colada na borda da tela quando o cartao some.
///
/// Em pixels fisicos porque e com eles que a posicao do cursor chega.
fn zona_opaca(
    origem: (f64, f64),
    tamanho: (f64, f64),
    escala: f64,
    recolhida: bool,
    medida: Option<(f64, f64, f64, f64)>,
) -> Zona {
    let (x, y) = origem;
    let (largura, altura) = tamanho;

    // A medida da tela manda. Ela sabe quantos aneis couberam e onde a lingueta
    // parou; aqui so ha o retangulo da janela.
    if let Some((mx, my, ml, ma)) = medida {
        if ml > 0.0 && ma > 0.0 {
            return Zona {
                x: x + mx * escala,
                y: y + my * escala,
                largura: (ml * escala).min(largura),
                altura: (ma * escala).min(altura),
            };
        }
    }

    // Reserva, ate a primeira medida chegar. Erra para o lado SEGURO — reivindica
    // a janela inteira quando aberta — porque roubar um clique do desktop por um
    // quadro e menos grave que a tira nascer surda.
    if !recolhida {
        return Zona {
            x,
            y,
            largura,
            altura,
        };
    }
    let lingueta = (LARGURA_LINGUETA * escala).min(largura);
    Zona {
        x: x + largura - lingueta,
        y,
        largura: lingueta,
        altura,
    }
}

/// A tela diz onde ela pintou.
///
/// Em pixels LOGICOS relativos a janela, que e o que o `getBoundingClientRect`
/// devolve. A conversao para fisico e a soma da posicao da janela ficam do lado
/// do Rust, que e quem conhece a escala do monitor.
#[tauri::command]
pub fn faixa_zona<R: Runtime>(
    app: AppHandle<R>,
    x: f64,
    y: f64,
    largura: f64,
    altura: f64,
) -> Result<(), CoreError> {
    if let Some(uso) = app.try_state::<Uso>() {
        if let Ok(mut guarda) = uso.zona.lock() {
            *guarda = Some((x, y, largura, altura));
        }
    }
    Ok(())
}

/// A tira so recebe clique onde ela pinta. Todo o resto atravessa.
///
/// # O problema que isto resolve
///
/// Duas coisas que a ADR-060 registrou como preco pago e como defeito aberto:
///
/// * **o pixel transparente engolia clique.** Recolhida, os 84 pixels que o
///   cartao ocupava continuavam sendo janela, e continuavam roubando o clique
///   do desktop embaixo. O Tauri nao faz click-through por regiao sozinho;
/// * **a tira emudecia.** Seis cliques alternados no mesmo processo deram
///   `OK, morto, morto, morto, OK, morto`, e as vezes ela nascia surda ate o app
///   reiniciar. Cinco tentativas de conserto morreram na tela — `resizable`,
///   `set_position`, `hide`/`show`, `focus`, mostrar depois da primeira passada.
///
/// # Por que este caminho e diferente dos cinco que falharam
///
/// Os cinco mexiam na JANELA e torciam para o Windows decidir certo sozinho
/// quem recebe o clique — decisao que, numa janela transparente e sem
/// decoracao, ele toma por conta propria e as vezes toma errado. Aqui a decisao
/// deixa de ser dele: `set_ignore_cursor_events` liga e desliga
/// `WS_EX_TRANSPARENT` na mao, e este laco a reafirma toda vez que o ponteiro
/// cruza a borda do que esta pintado.
///
/// E dessa reafirmacao vem a segunda propriedade, que e a que importa mais:
/// **a surdez deixa de ser permanente**. Se a entrada da janela se perder, o
/// proximo tique com o cursor sobre o cartao a devolve. Uma tira que emudecia
/// ate o app reiniciar passa a emudecer, no pior caso, por um quadro.
///
/// Nao e a causa raiz — ela continua desconhecida, e a ADR-060 continua com a
/// pergunta aberta. E o conserto que nao depende de descobri-la.
///
/// # Onde este laco NAO mexe
///
/// No painel. Ele e mostrado e escondido a cada uso, nunca falhou, e pinta a
/// janela inteira — nao ha pixel morto nele para devolver a ninguem. Uma
/// mudanca de cada vez.
async fn vigiar_o_cursor<R: Runtime>(app: AppHandle<R>) {
    // `None` obriga a primeira volta a aplicar. Vira `None` de novo sempre que a
    // janela some: ao voltar, o estado dela nao e mais o que ficou gravado aqui.
    let mut aplicado: Option<bool> = None;

    loop {
        let Some(janela) = app.get_webview_window(JANELA_FAIXA) else {
            tokio::time::sleep(VIGIA_LONGE).await;
            continue;
        };
        if !janela.is_visible().unwrap_or(false) {
            aplicado = None;
            tokio::time::sleep(VIGIA_LONGE).await;
            continue;
        }

        // A area de CLIENTE, e nao a externa: o Windows entrega uma janela sem
        // decoracao maior do que o `tauri.conf.json` pediu, e a moldura que
        // sobra nao aparece no CSS. Medir por fora deslocaria a zona inteira.
        let (Ok(origem), Ok(tamanho), Ok(escala), Ok(cursor)) = (
            janela.inner_position(),
            janela.inner_size(),
            janela.scale_factor(),
            app.cursor_position(),
        ) else {
            tokio::time::sleep(VIGIA_LONGE).await;
            continue;
        };

        let recolhida = app
            .try_state::<Uso>()
            .map(|uso| uso.recolhida.load(Ordering::Relaxed))
            .unwrap_or(false);
        let medida = app
            .try_state::<Uso>()
            .and_then(|uso| uso.zona.lock().ok().and_then(|guarda| *guarda));
        let zona = zona_opaca(
            (origem.x as f64, origem.y as f64),
            (tamanho.width as f64, tamanho.height as f64),
            escala,
            recolhida,
            medida,
        );

        let dentro = zona.contem(cursor.x, cursor.y);
        // So na TROCA: mexer no estilo estendido da janela a cada tique seria
        // uma chamada ao Windows sessenta vezes por segundo para nao mudar nada.
        if aplicado != Some(dentro) && janela.set_ignore_cursor_events(!dentro).is_ok() {
            aplicado = Some(dentro);
        }

        let perto = zona.distancia(cursor.x, cursor.y) <= RAIO_DE_APROXIMACAO;
        tokio::time::sleep(if perto { VIGIA_PERTO } else { VIGIA_LONGE }).await;
    }
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
    if let Some(uso) = app.try_state::<Uso>() {
        uso.recolhida.store(recolhida, Ordering::Relaxed);
    }

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

    // Laco proprio, e nao um passo deste: um acorda de trinta em trinta
    // segundos e o outro precisa de dezenas de vezes por segundo. Amarrados,
    // o rapido herdaria a cadencia do lento.
    tauri::async_runtime::spawn(vigiar_o_cursor(app.clone()));

    // E o mesmo motivo para a cota: ela pergunta de minuto em minuto, e recua
    // sozinha quando o servidor nao responde. Amarrada a varredura, herdaria a
    // cadencia dela e o recuo nao teria onde morar.
    tauri::async_runtime::spawn(perguntar_a_cota(app.clone()));
    tauri::async_runtime::spawn(perguntar_as_externas(app.clone()));

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

    /// A hora de referencia dos testes de `montar`. Fixa porque a idade da cota
    /// e o que decide se ela aparece, e uma hora que anda daria um teste que
    /// falha sozinho daqui a cinco minutos.
    const AGORA: OffsetDateTime = datetime!(2026-08-31 12:00:00 UTC);

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

    fn observada(minutos_atras: i64) -> CotaObservada {
        CotaObservada {
            cota: cota::Cota {
                sessao: Some(cota::Limite {
                    percentual: 23,
                    reseta_em: Some(datetime!(2026-08-31 15:50:00 UTC)),
                }),
                semana: Some(cota::Limite {
                    percentual: 3,
                    reseta_em: Some(datetime!(2026-09-06 17:00:00 UTC)),
                }),
            },
            em: AGORA - time::Duration::minutes(minutos_atras),
            demo: false,
        }
    }

    /// Com cota, o anel ganha o denominador de verdade — e o pico continua
    /// viajando junto, porque e ele que responde quando a cota some.
    #[test]
    fn a_cota_do_servidor_chega_a_faixa() {
        let faixa = montar(
            leitura(),
            "Claude Code",
            false,
            false,
            Some(observada(0)),
            &[],
            AGORA,
        )
        .unwrap();
        let anel = &faixa.aneis[0];
        let sessao = anel.cota_sessao.as_ref().expect("sessão");
        assert_eq!(sessao.percentual, 23);
        assert_eq!(sessao.reseta_em.as_deref(), Some("2026-08-31T15:50:00Z"));
        assert!(!sessao.obsoleta);
        assert_eq!(anel.cota_semana.as_ref().unwrap().percentual, 3);
        assert!(anel.pico > 0, "o pico continua sendo enviado");
    }

    /// Uma leitura que nao renovou continua na tela, MARCADA. Apaga-la seria
    /// trocar informacao velha por nenhuma.
    ///
    /// Quatro minutos porque o limiar e um intervalo e meio, e o intervalo virou
    /// dois minutos quando o servidor devolveu 429. Este teste ja falhou uma vez
    /// por causa disso — a amostra de dois minutos deixou de ser velha no mesmo
    /// commit em que a constante mudou —, e a licao esta no nome do numero: ele
    /// acompanha `COTA_INTERVALO`, nao um relogio de parede.
    #[test]
    fn a_cota_que_nao_renovou_fica_marcada_como_velha() {
        let faixa = montar(
            leitura(),
            "Claude Code",
            false,
            false,
            Some(observada(4)),
            &[],
            AGORA,
        )
        .unwrap();
        let sessao = faixa.aneis[0].cota_sessao.as_ref().unwrap();
        assert_eq!(sessao.percentual, 23);
        assert!(sessao.obsoleta);
    }

    /// Passados os cinco minutos ela some, e a regua volta a ser o pico.
    #[test]
    fn depois_de_cinco_minutos_a_cota_some() {
        let faixa = montar(
            leitura(),
            "Claude Code",
            false,
            false,
            Some(observada(6)),
            &[],
            AGORA,
        )
        .unwrap();
        assert!(faixa.aneis[0].cota_sessao.is_none());
        assert!(faixa.aneis[0].cota_semana.is_none());
    }

    /// Sem cota nenhuma, a faixa e exatamente a que a ADR-059 deixou.
    #[test]
    fn sem_cota_a_faixa_e_a_da_adr_059() {
        let faixa = montar(leitura(), "Claude Code", false, false, None, &[], AGORA).unwrap();
        assert!(faixa.aneis[0].cota_sessao.is_none());
        assert!(
            faixa.aneis[0].reseta_em.is_some(),
            "o prazo calculado continua"
        );
    }

    fn externa(nome: &str, sessao: u16, minutos_atras: i64) -> (String, Option<CotaObservada>) {
        (
            nome.to_string(),
            Some(CotaObservada {
                cota: cota::Cota {
                    sessao: Some(cota::Limite {
                        percentual: sessao,
                        reseta_em: None,
                    }),
                    semana: None,
                },
                em: AGORA - time::Duration::minutes(minutos_atras),
                demo: false,
            }),
        )
    }

    /// Uma fonte externa vira um anel ao lado do Claude Code.
    #[test]
    fn a_fonte_externa_ganha_o_proprio_anel() {
        let faixa = montar(
            leitura(),
            "Claude Code",
            false,
            false,
            Some(observada(0)),
            &[externa("Codex", 42, 0)],
            AGORA,
        )
        .unwrap();
        assert_eq!(faixa.aneis.len(), 2);
        assert_eq!(faixa.aneis[1].nome, "Codex");
        assert_eq!(faixa.aneis[1].cota_sessao.as_ref().unwrap().percentual, 42);
        assert!(
            !faixa.aneis[1].tem_historico,
            "ela nao conta o historico dela"
        );
        assert!(faixa.aneis[0].tem_historico, "e o Claude Code conta");
    }

    /// Uma fonte que nao respondeu NAO vira anel.
    ///
    /// Um anel permanente marcado "SEM RÉGUA" para um comando quebrado seria
    /// ocupar a borda da tela com a lembranca de um erro de configuracao.
    #[test]
    fn a_fonte_que_nao_respondeu_nao_aparece() {
        let faixa = montar(
            leitura(),
            "Claude Code",
            false,
            false,
            None,
            &[("Codex".to_string(), None)],
            AGORA,
        )
        .unwrap();
        assert_eq!(faixa.aneis.len(), 1);
    }

    /// E uma que respondeu ha muito tempo some junto, pela mesma regra de idade
    /// que vale para a cota do Claude Code.
    #[test]
    fn a_fonte_externa_tambem_envelhece() {
        let faixa = montar(
            leitura(),
            "Claude Code",
            false,
            false,
            None,
            &[externa("Codex", 42, 6)],
            AGORA,
        )
        .unwrap();
        assert_eq!(faixa.aneis.len(), 1, "seis minutos e velho demais");
    }

    /// A ordem e a do `settings.json`, e o Claude Code e sempre o primeiro.
    #[test]
    fn a_ordem_e_a_do_arquivo_com_o_claude_code_na_frente() {
        let faixa = montar(
            leitura(),
            "Claude Code",
            false,
            false,
            Some(observada(0)),
            &[externa("Codex", 1, 0), externa("Cursor", 2, 0)],
            AGORA,
        )
        .unwrap();
        let nomes: Vec<&str> = faixa.aneis.iter().map(|anel| anel.nome.as_str()).collect();
        assert_eq!(nomes, ["Claude Code", "Codex", "Cursor"]);
    }

    /// Aberta, a janela inteira e clicavel: nao sobra pixel morto nela.
    #[test]
    fn aberta_a_zona_e_a_janela_toda() {
        let zona = zona_opaca((1824.0, 484.0), (96.0, 112.0), 1.0, false, None);
        assert!(zona.contem(1824.0, 484.0));
        assert!(zona.contem(1919.0, 595.0));
        // A borda de fora NAO pertence: 1920 e o primeiro pixel de quem esta ao
        // lado, e reivindica-lo seria roubar de novo o clique que este laco
        // existe para devolver.
        assert!(!zona.contem(1920.0, 500.0));
        assert!(!zona.contem(1823.0, 500.0));
    }

    /// Recolhida, os 84 pixels do cartao voltam a ser do desktop.
    ///
    /// E o preco que a ADR-060 registrou como pago; este teste e o que diz que
    /// ele deixou de ser.
    #[test]
    fn recolhida_so_a_lingueta_da_direita_recebe_clique() {
        let zona = zona_opaca((1824.0, 484.0), (96.0, 112.0), 1.0, true, None);
        assert_eq!(zona.largura, 12.0);
        // A lingueta e a DIREITA: `row-reverse` a cola na borda da tela.
        assert!(zona.contem(1908.0, 500.0));
        assert!(zona.contem(1919.0, 500.0));
        // O buraco de 84 pixels onde o cartao estava.
        assert!(!zona.contem(1907.0, 500.0));
        assert!(!zona.contem(1824.0, 500.0));
        // A altura nao encolhe: a lingueta e uma coluna inteira.
        assert!(zona.contem(1910.0, 595.0));
    }

    /// A lingueta e 12 pixels LOGICOS, e o cursor chega em fisicos.
    #[test]
    fn a_lingueta_acompanha_a_escala_do_monitor() {
        let zona = zona_opaca((1800.0, 400.0), (120.0, 140.0), 1.25, true, None);
        assert_eq!(zona.largura, 15.0);
        assert_eq!(zona.x, 1905.0);
    }

    /// Numa janela mais estreita que a lingueta, a zona e a janela — e nunca um
    /// retangulo de largura negativa comecando fora dela.
    #[test]
    fn a_lingueta_nunca_passa_da_janela() {
        let zona = zona_opaca((1912.0, 400.0), (8.0, 140.0), 1.0, true, None);
        assert_eq!(zona.x, 1912.0);
        assert_eq!(zona.largura, 8.0);
    }

    /// A distancia e o que decide se o vigia corre a 60Hz ou a 8Hz.
    #[test]
    fn a_distancia_e_zero_dentro_e_cresce_para_fora() {
        let zona = zona_opaca((1824.0, 484.0), (96.0, 112.0), 1.0, false, None);
        assert_eq!(zona.distancia(1900.0, 500.0), 0.0);
        assert_eq!(zona.distancia(1804.0, 500.0), 20.0);
        assert_eq!(zona.distancia(1824.0, 464.0), 20.0);
        // Na diagonal, a hipotenusa: 3-4-5.
        assert_eq!(zona.distancia(1821.0, 480.0), 5.0);
        assert!(zona.distancia(1000.0, 500.0) > RAIO_DE_APROXIMACAO);
    }

    #[test]
    fn a_faixa_leva_o_pico_e_o_prazo() {
        let faixa = montar(leitura(), "Claude Code", false, false, None, &[], AGORA).unwrap();
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
        let anel = &montar(leitura, "Claude Code", false, false, None, &[], AGORA)
            .unwrap()
            .aneis[0];
        assert_eq!(anel.peso, 0);
        assert_eq!(anel.requisicoes, 0);
        assert_eq!(anel.reseta_em, None, "sem janela nao ha o que resetar");
        assert_eq!(anel.pico, 1_000_000, "mas o pico historico continua");
    }

    #[test]
    fn calibrando_atravessa_ate_a_faixa() {
        let faixa = montar(leitura(), "Claude Code", true, false, None, &[], AGORA).unwrap();
        assert!(faixa.calibrando);
    }
}
