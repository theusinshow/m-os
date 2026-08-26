# mos-web — a superfície de bolso

**Estado: esqueleto. Não compila nesta máquina, e o motivo está registrado.**

## O que ele é

Uma **porta** para o M/OS, e não um segundo M/OS. Capturar, ver a inbox, mexer
em tasks e falar com o Hermes — o trabalho de verdade continua no desktop, com o
CAD aberto.

Essa fronteira está escrita aqui porque é ela que costuma escapar: o desktop
expõe 280 comandos, e crescer até lá seria uma decisão, não uma deriva.

Ele é **mais um dispositivo da malha**: banco próprio, `mos-core` próprio, e
sincroniza pelo mesmo hub que o desktop usa. Não há caminho de dados novo.

## Por que está fora do workspace

O `webauthn-rs` — a biblioteca madura de passkey em Rust — depende de OpenSSL.
Esta máquina de desenvolvimento não tem, e as duas saídas tentadas falharam:

- OpenSSL do sistema: não existe;
- `vendored` (compila do zero): o perl do Git for Windows não traz os módulos
  que o build da OpenSSL exige (`Locale::Maketext::Simple`).

Existem alternativas puro-Rust (`passkey`, `passki`), mas são jovens, e
biblioteca jovem guardando a porta da casa é uma troca ruim.

Membro do workspace, este crate quebraria `cargo build --workspace` para todo
mundo por causa de algo que só vai rodar na VPS. Por isso `exclude`, com o mesmo
raciocínio que o `apps/cronocad/src-tauri` já usava.

## Para destravar

Uma das três:

1. **OpenSSL nesta máquina** — `winget install ShiningLight.OpenSSL`, ou um perl
   completo (Strawberry Perl). Volta para `members` e segue com passkey de
   verdade.
2. **Desenvolver contra o CI** — Linux compila e testa; a iteração vira um
   round-trip de alguns minutos.
3. **Trocar a porta** — segredo longo + sessão compila em qualquer lugar, e o
   passkey entra depois. Foi recusado uma vez: passkey era a escolha.

## O que já existe

- `src/auth.rs` — passkey (registro e login), sessão em cookie opaco com o token
  guardado em hash, convite obrigatório para qualquer registro. **Escrito e não
  compilado.**
- `Cargo.toml` — dependências decididas.

## O que falta

- A superfície: capturar, inbox, tasks.
- O laço de sync contra o hub (o `mos-sync-http` já existe e é reaproveitado).
- A PWA em React.
- A camada de ação do Hermes, em forma mobile (~1.400 linhas no desktop, e a
  interface é reescrita; o protocolo em `mos-hermes` vem de graça).
- Endereço público com TLS — **passkey exige origem estável**, e no celular isso
  quer dizer um domínio, não um IP.
