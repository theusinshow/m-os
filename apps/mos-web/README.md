# mos-web — a superfície de bolso

## O que ele é

Uma **porta** para o M/OS, e não um segundo M/OS. Capturar, ver a inbox, mexer
em tasks e — mais adiante — falar com o Hermes. O trabalho de verdade continua
no desktop, com o CAD aberto.

Essa fronteira está escrita aqui porque é ela que costuma escapar: o desktop
expõe 280 comandos, e crescer até lá seria uma decisão, não uma deriva.

Ele é **mais um dispositivo da malha**: banco próprio, `mos-core` próprio,
identidade própria, e sincroniza pelo mesmo hub que o desktop usa. Não há
caminho de dados novo.

### Por que dispositivo, e não proxy

A tentação óbvia seria repassar as chamadas para o desktop, ou ler o banco do
hub direto. As duas quebram o desenho: a primeira faz o celular parar de
funcionar quando o PC está desligado — que é justamente quando se tem uma ideia
na rua —, e a segunda transformaria o hub em autoridade, coisa que o `SYNC.md`
recusa em toda linha.

## Estado

| | |
|---|---|
| Servidor, banco, identidade | pronto |
| Sync contra o hub (fundo + a cada escrita) | pronto |
| API: capturar, inbox, tasks | pronto |
| Porta (passkey) | **escrita, não compilada** — ver abaixo |
| PWA em React | falta |
| Hermes com camada de ação | falta |

Dois testes de ponta a ponta, com hub de verdade, `mos-web` de verdade e um
segundo M/OS de verdade: a captura feita no bolso chega no PC, e a Task criada
no PC aparece no bolso. Conferido que os dois caem quando o sync é desligado.

## As features, e por que elas existem

```bash
cargo run -p mos-web                          # sem porta — só localhost
cargo run -p mos-web --features passkey       # a porta de verdade
```

`passkey` é **opcional** porque o `webauthn-rs` depende de OpenSSL, que não
existe na máquina de desenvolvimento. Como dependência obrigatória, ela impediria
de compilar todo o resto — servidor, sync, superfície — por causa da porta. Com
feature, o resto anda aqui e a porta é conferida onde OpenSSL existe: no CI e na
VPS, que é onde ela vai rodar.

O binário **recusa a subir** sem porta compilada se o bind não for localhost.
Uma porta de desenvolvimento que chega em produção é como a maioria dos
vazamentos começa.

### Para compilar a porta aqui

Uma das duas:

1. `winget install ShiningLight.OpenSSL` (ou Strawberry Perl, para o build
   `vendored`).
2. Deixar o CI conferir: `cargo test -p mos-web --features passkey` em Linux.

## Configuração

| variável | padrão | o que é |
|---|---|---|
| `MOS_WEB_BIND` | `127.0.0.1` | onde escuta |
| `MOS_WEB_PORT` | `9130` | porta |
| `MOS_WEB_DB` | `mos-web.db` | o banco deste dispositivo |
| `MOS_WEB_BACKUPS` | `backups` | onde ficam os snapshots |
| `MOS_WEB_HUB` | — | URL do hub. Vazio: funciona sozinho, e **avisa** |
| `MOS_WEB_TOKEN` | — | o segredo do hub |
| `MOS_WEB_INVITE` | — | convite para registrar um aparelho (com `passkey`) |

## O que falta decidir antes do celular

**Passkey exige origem estável**: uma passkey criada em `https://mos.exemplo`
não funciona em outro endereço, e é isso que torna phishing impossível. No
celular, isso quer dizer um **domínio de verdade com TLS** — não um IP, não um
túnel SSH.

## O Hermes: o que a medição mostrou

A camada de ação **não** é código novo a escrever — é código existente a
libertar. O executor mora em `apps/desktop/src-tauri/src/jarvis.rs`, 2.388
linhas, e apenas **54 delas tocam o Tauri**. O miolo é domínio puro.

O acoplamento tem forma conhecida, e foi medido:

| o que ele pede do hospedeiro | quantas vezes | resolvido? |
|---|---|---|
| os oito serviços (`app.state::<AppState>()`) | por toda parte | **sim** — `mos_core::Servicos` |
| `app.emit(...)` para avisar a janela | 7 | falta |
| `attention::poke` (acorda o agendador) | 2 | falta |
| `surface::now_local` (o fuso do usuário) | 1 | falta |
| `daily::hoje/iniciar/encerrar/resolver_objetivo` | 6 | falta — `daily.rs` também é do desktop |
| `finance::execute_create_bill` | 1 | **já é portável** — não usa `AppHandle` |

### O que já está feito

`mos_core::Servicos` reúne os oito serviços num tipo só, e `AppState::servicos()`
o produz. Era o maior dos acoplamentos, e o único espalhado por todo o arquivo.

### O que falta, e a forma que ele tem

Um trait `Ambiente` com três métodos — `agora_local`, `avisar`,
`cutucar_lembretes` —, implementado pelo desktop com `AppHandle` e pelo `mos-web`
com fuso configurado e avisos silenciosos. Mais portar `daily.rs` (~450 linhas),
que hoje é a única razão de `run_action` ainda precisar de uma janela.

### Por que parou aqui

Mover 500 linhas de código que funciona — agendador de lembretes e sessão do dia
— sem poder abrir o desktop e clicar é como se quebra algo em silêncio. A
fronteira acima está medida e o caminho é mecânico; o que falta é executá-lo com
o app na mão para conferir.
