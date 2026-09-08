//! Abrir o túnel SSH do Hermes a partir do M/OS.
//!
//! # Por que isto existe, se já há uma tarefa agendada
//!
//! A tarefa (`scripts/install-hermes-tunnel.ps1`) cobre o caso normal: o túnel
//! sobe no logon e se reergue sozinho. Ela não cobre o caso que dói — a máquina
//! em que ela nunca foi instalada, ou o dia em que ela morreu e o próximo logon
//! está a horas de distância. Nesses dois casos o Hermes aparece **Offline** no
//! M/OS, corretamente, e a única saída era abrir o PowerShell e lembrar o
//! comando.
//!
//! # O que ele NÃO faz
//!
//! Não reergue: o laço de reconexão continua sendo da tarefa agendada. Este
//! comando abre uma vez, e se a VPS cair depois disso o túnel morre e o botão
//! volta a aparecer. Duplicar o laço aqui daria dois donos para o mesmo socket
//! — e foi exatamente essa disputa que o `ExitOnForwardFailure` do script já
//! teve de aprender a evitar.
//!
//! Não guarda credencial. A chave é a mesma do `hermes-tunnel.ps1`, lida do
//! `~/.ssh` de quem está logado.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use serde::Serialize;

/// A porta do dashboard do Hermes, dos dois lados do túnel.
///
/// Ela é fixa porque o `docker-compose.yml` do Hermes-Agent a fixa, e o
/// `DEFAULT_BASE_URL` daqui aponta para ela.
const PORTA: u16 = 9119;

const ALVO: &str = "hermes@167.233.43.1";

/// Os nomes de chave que este projeto já usou, em ordem de preferência.
///
/// Uma lista e não um nome fixo porque as duas máquinas do dono nomeiam
/// diferente — e a versão antiga do script procurava um `id_ed25519` que não
/// existia numa delas, girando calada. Ver o cabeçalho de `hermes-tunnel.ps1`.
const CHAVES: &[&str] = &["hermes_work", "id_ed25519", "hermes_home"];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunelStatus {
    /// A porta local está atendendo. É o que decide se o botão aparece.
    pub aberto: bool,
    /// Há uma chave utilizável no `~/.ssh`. Sem ela o botão não tem o que fazer,
    /// e dizer isso ANTES do toque é melhor que falhar depois dele.
    pub tem_chave: bool,
}

/// A porta local está atendendo?
///
/// Conexão TCP, e não `netstat`: o que importa é se dá para FALAR com ela. Uma
/// porta em `LISTEN` que recusa conexão contaria como aberta e o app diria que
/// está tudo bem enquanto o Hermes segue inalcançável.
fn porta_aberta() -> bool {
    use std::net::{Ipv4Addr, SocketAddr, TcpStream};
    let endereco = SocketAddr::from((Ipv4Addr::LOCALHOST, PORTA));
    TcpStream::connect_timeout(&endereco, Duration::from_millis(400)).is_ok()
}

/// A primeira chave utilizável, ou nada.
///
/// Chave com passphrase é descartada: o `ssh` ficaria esperando alguém digitar,
/// e ninguém está olhando — o botão pareceria travado. Mesma regra do script.
fn chave() -> Option<PathBuf> {
    let casa = dirs_next()?;
    CHAVES.iter().find_map(|nome| {
        let caminho = casa.join(".ssh").join(nome);
        let cabecalho = std::fs::read_to_string(&caminho).ok()?;
        let comeco: String = cabecalho.lines().take(3).collect::<Vec<_>>().join("\n");
        (!comeco.contains("ENCRYPTED")).then_some(caminho)
    })
}

/// O diretório do usuário. `USERPROFILE` no Windows, `HOME` no resto.
fn dirs_next() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

#[tauri::command]
pub fn hermes_tunnel_status() -> TunelStatus {
    TunelStatus {
        aberto: porta_aberta(),
        tem_chave: chave().is_some(),
    }
}

/// Abre o túnel e espera ele atender.
///
/// # Por que espera, em vez de responder na hora
///
/// Porque `spawn` devolve sucesso assim que o processo nasce — e um `ssh` que
/// vai falhar a autenticação também nasce. Responder ali diria "abri" para um
/// túnel que morre meio segundo depois, e o usuário veria o botão sumir e
/// voltar sem entender por quê.
///
/// Cinco segundos: a VPS responde em menos de um, e o teto existe para o caso
/// de ela estar fora — nesse caso a mensagem diz isso, em vez de a tela ficar
/// esperando para sempre.
#[tauri::command]
pub async fn hermes_tunnel_open() -> Result<TunelStatus, String> {
    if porta_aberta() {
        // Já há um túnel — o atalho de Desktop, ou a tarefa agendada. Subir um
        // segundo faria `ExitOnForwardFailure` derrubar este na hora.
        return Ok(hermes_tunnel_status());
    }

    let Some(chave) = chave() else {
        return Err(format!(
            "Nenhuma chave SSH sem passphrase em ~/.ssh (procurei: {}). \
             Sem ela o túnel não sobe.",
            CHAVES.join(", ")
        ));
    };

    let mut comando = Command::new("ssh");
    comando
        .arg("-i")
        .arg(&chave)
        .args(["-N", "-T"])
        .args(["-o", "BatchMode=yes"])
        // Sem isto, um túnel que não consegue reservar a porta continua vivo
        // como sessão inútil, e o app ficaria com um `ssh` pendurado sem
        // encaminhar nada.
        .args(["-o", "ExitOnForwardFailure=yes"])
        .args(["-o", "ServerAliveInterval=30"])
        .args(["-o", "ServerAliveCountMax=3"])
        .args(["-o", "StrictHostKeyChecking=accept-new"])
        .arg("-L")
        .arg(format!("{PORTA}:127.0.0.1:{PORTA}"))
        .arg(ALVO);

    #[cfg(windows)]
    {
        // Sem isto, cada abertura pisca uma janela preta de console por cima do
        // app. `CREATE_NO_WINDOW`.
        use std::os::windows::process::CommandExt;
        comando.creation_flags(0x0800_0000);
    }

    let mut filho = comando
        .spawn()
        .map_err(|causa| format!("Não consegui iniciar o ssh: {causa}"))?;

    for _ in 0..25 {
        // O processo morreu antes de encaminhar: autenticação recusada, host
        // inalcançável. Dizer isso é melhor que esperar os cinco segundos
        // inteiros para então dizer "não abriu".
        if let Ok(Some(saida)) = filho.try_wait() {
            return Err(format!(
                "O ssh encerrou sem abrir o túnel (código {}). \
                 Confira se a chave está autorizada na VPS.",
                saida.code().unwrap_or(-1)
            ));
        }
        if porta_aberta() {
            return Ok(hermes_tunnel_status());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    Err(String::from(
        "O ssh subiu mas a porta 9119 não respondeu em 5 s. \
         A VPS pode estar fora do ar.",
    ))
}

#[cfg(test)]
mod testes {
    use super::*;

    /// A lista de chaves é a MESMA do `hermes-tunnel.ps1`, e na mesma ordem.
    ///
    /// Duas listas divergentes produziriam o pior sintoma: o botão abrindo com
    /// uma chave e a tarefa agendada tentando outra, cada um funcionando numa
    /// máquina diferente.
    #[test]
    fn as_chaves_sao_as_mesmas_do_script() {
        assert_eq!(CHAVES, &["hermes_work", "id_ed25519", "hermes_home"]);
    }

    /// A porta é a do `DEFAULT_BASE_URL`. Se uma mudar sem a outra, o botão
    /// abriria um túnel para uma porta que o app não consulta.
    #[test]
    fn a_porta_e_a_do_endereco_padrao() {
        assert!(crate::hermes::DEFAULT_BASE_URL.ends_with(&PORTA.to_string()));
    }
}
