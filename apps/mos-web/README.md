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
| Porta (passkey) | **montada** — cerimônia atrás da feature, guardião sempre |
| PWA em React | pronto |
| Notificação (Web Push) | pronto |
| Hermes com camada de ação | falta |

Testes de ponta a ponta, com hub de verdade, `mos-web` de verdade e um segundo
M/OS de verdade: a captura feita no bolso chega no PC e a Task criada no PC
aparece no bolso. Conferido que todos caem quando o sync é desligado.

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
| `MOS_WEB_VAPID_PRIVADA` | — | a chave do push. Vazio: sobe igual, e **não notifica** |
| `MOS_WEB_VAPID_CONTATO` | `mailto:mos@localhost` | `mailto:`/`https:`. A Apple recusa sem |
| `MOS_WEB_PUSH_DB` | `push.db` | as assinaturas de push, **fora** do banco de domínio |

A chave nasce uma vez:

```bash
mos-web --gerar-vapid
```

**Trocá-la mata todas as assinaturas** — o aparelho assinou com a pública
antiga, e o serviço de push recusa o que a nova assinar. O sintoma é o pior
possível: tudo parece funcionar e nada chega.

## O travamento que estava aqui desde sempre

Duas escritas seguidas — capturar duas coisas, ou criar um lembrete e concluí-lo
— **travavam o servidor para sempre**. Calado: sem log, sem erro, sem 500. O app
parava de responder e continuava parado depois de fechar e abrir, porque o
processo é que estava preso.

A causa é um abraço mortal no `mos-storage-sqlite`, que tem dois cadeados e os
entrega em ordem contrária conforme o caminho:

| caminho | pega primeiro | depois |
|---|---|---|
| qualquer escrita (`emitir`) | a conexão | o relógio lógico |
| uma rodada de sync (`sincronizar_agora`) | o relógio lógico | a conexão |

E o encontro não é raro: **toda escrita dispara uma rodada em segundo plano**,
então basta a segunda escrita cair dentro da rodada da primeira. Nunca tinha
aparecido porque nenhum teste escrevia duas vezes seguidas — foi a aba de
lembretes, que cria e conclui em sequência, que o tornou reprodutível.

O conserto daqui **desfaz o encontro em vez de reordenar os cadeados**: uma
escrita e uma rodada nunca acontecem juntas (`estado::Estado::vez`), e toda
escrita passa por `api::escrever` — uma rota nova escrita à mão sem a vez faria o
defeito voltar sem nada na tela dizendo isso. Leitura não pega a vez: ela só toca
a conexão, nunca o relógio, e fazer a inbox esperar por uma rodada de rede seria
pagar em tela travada por um risco que ela não corre.

`tests/de_bolso.rs::escrever_em_rajada_nao_trava` guarda o conserto.

**A ordem contrária continua lá dentro**, e ela alcança o desktop também — ele
emite escritas concorrentes de comandos diferentes. Arrumar a ordem no
`mos-storage-sqlite` é um conserto de outra caixa, e este aqui não o dispensa.

## A porta

Duas metades, e a divisão não é arrumação:

| | onde | compila aqui? |
|---|---|---|
| Sessão, cookie, guardião das rotas | `porta.rs` | **sim** |
| Cerimônia WebAuthn (o Face ID) | `auth.rs`, feature `passkey` | não (OpenSSL) |

Enquanto a porta inteira morava atrás da feature, a pergunta que mais importa —
*uma requisição sem sessão é recusada?* — só tinha resposta no CI. E o CI
respondeu "verde" por semanas para um `auth.rs` que estava escrito, compilando, e
**não montado em rota nenhuma**: `cargo check` não distingue rota montada de rota
esquecida numa gaveta. Só uma requisição distingue, e `tests/a_porta.rs` faz sete
delas — no Windows, em menos de um segundo.

O guardião decide pelo **caminho**: tudo sob `/api` exige sessão, menos
`/api/porta/`. Um sub-router protegido seria uma decisão que se perde — alguém
acrescenta uma rota no lugar errado e ela nasce pública. Aqui rota nova nasce
protegida por omissão.

A página continua livre, porque ela é a tela de entrar. Ela não expõe dado
nenhum: o dado está atrás da API.

## A notificação

Web Push são duas RFCs — a 8292 (assinatura VAPID) e a 8291 (payload cifrado) —
e as duas estão em `push.rs`, escritas à mão. O crate `web-push` foi medido e
recusado: ele traz `http ^0.2` para uma árvore que usa `http` 1.x no `axum`, e
puxa OpenSSL pelo `ece` — que aqui vive preso à feature `passkey` justamente
para o resto compilar nesta máquina.

O que torna isso seguro é que **a RFC 8291 publica vetores de teste completos**.
`cargo test -p mos-web` cifra a mensagem da RFC com as chaves da RFC e confere o
corpo byte a byte, no Windows, sem VPS e sem iPhone. E o `tests/notificacao.rs`
faz o papel do aparelho: gera o próprio par de chaves, assina, e **decifra** o
que o servidor mandou. Um push que não chega nunca diz por quê — esses dois
testes são o que diz.

O conteúdo viaja cifrado ponta a ponta: o serviço de push da Apple encaminha o
pacote sem poder ler uma palavra dele.

### O que avisa

| | |
|---|---|
| Lembrete que venceu | um laço de 60s lê `attention.waiting()` e avisa uma vez cada |
| Coisa nova vinda do PC | quando uma rodada de sync desce algo, diz **quantos** |

O `mos-web` **lê** lembretes e **não escreve** nenhum. Marcar um Reminder como entregue
sincronizaria para o PC, e o desktop tem o próprio agendador olhando os mesmos
lembretes — dois aparelhos disputando o mesmo estado produziriam o lembrete que
some do PC porque o celular achou que já tinha dado conta. O que já foi avisado
mora no `push.db`, que é local e não sincroniza.

### O que o iPhone exige, e que nada no código resolve

**Web Push no iOS só funciona para uma PWA instalada na Tela de Início.** No
Safari comum não chega nada, e o `Notification.requestPermission()` nem existe.
A aba **Avisos** detecta isso e mostra o passo, porque a alternativa é um botão
que falha calado.

Também exige **HTTPS num domínio estável** — a mesma exigência da passkey, e
pela mesma razão.

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
