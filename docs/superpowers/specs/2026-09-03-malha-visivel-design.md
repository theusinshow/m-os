# A malha visível — identidade estável, backfill por geração, e um endereço só

Data: 2026-09-03
Estado: aprovado, aguardando implementação
Escopo: três dos quatro buracos do sync. O quarto — por que o CronoCAD não
apareceu no PC2 — está fora deste spec até o diagnóstico daquela máquina chegar.

## O problema

O sync funciona e ninguém consegue ver isso funcionando. Em 02/09/2026 levou uma
manhã inteira para descobrir por que o PC do trabalho mostrava R$ 0,00, e a
descoberta veio de abrir o hub por um túnel SSH e contar operações à mão. Três
defeitos sustentam essa cegueira, e eles são independentes:

1. **A identidade de um aparelho pode se perder.** `este_dispositivo` acha o
   dispositivo por `SELECT id FROM devices WHERE is_this_device = 1`
   (`device_repository.rs:58`). Some a linha, nasce um id novo — e com ele um
   relógio novo e um cursor zerado. Foi o que aconteceu: o hub tem hoje **três**
   identidades, e duas empurraram exatamente o mesmo perfil de dados (8
   `academic_assignment`, 10 `academic_exam`, 5 `academic_subject`, 28
   `resource`, 1 `workspace`), com 141 e 101 operações. Uma delas é o PC2 antes,
   a outra é o PC2 depois.
2. **Ninguém sabe quem está na malha.** O hub tem uma tabela só — `sync_log`
   (`hub.rs:61`). Não há registro de aparelho, versão ou último contato. A
   pergunta "o outro PC está atrasado?" não tem onde ser respondida, nem no
   servidor nem na tela.
3. **O backfill é um booleano.** A marca é `sync_backfill_v1`
   (`sync_backfill.rs:31`), gravada uma vez. Quando a cobertura cresceu de 12
   para 26 tipos (v0.3.4), quem já tinha a marca **nunca re-emitiu** o que
   passou a ser sincronizável. A armadilha continua armada para a próxima vez
   que a cobertura crescer.

E há um quarto, que não é defeito de código e sim de topologia: **existem duas
maneiras de estar conectado** — o túnel SSH para os PCs e o acesso direto para o
bolso —, e a que costuma quebrar é a que ninguém vê.

## O objetivo

Abrir a tela de sincronização e responder, sem investigação: quem são os
aparelhos, em que versão cada um está, quando cada um falou pela última vez, e
se falta alguma coisa. E ter **um endereço só**, igual nos três aparelhos.

Explicitamente **não** é: mudar o modelo local-first. O desktop continua
funcionando com a VPS fora do ar — isso foi decidido, e este spec não encosta
nisso.

## 1. Identidade que não se perde

O `device_id` ganha uma âncora em `app_metadata`, na chave `sync_device_id`.

```
este_dispositivo(nome, plataforma, versao):
    id = SELECT id FROM devices WHERE is_this_device = 1
    se não achou:
        ancora = SELECT value FROM app_metadata WHERE key = 'sync_device_id'
        se ancora existe:
            recria a linha de devices com AQUELE id     # o cursor sobrevive
        senão:
            id = novo; grava a âncora
    UPDATE devices SET name, platform, app_version, updated_at
```

A âncora e a linha nascem na **mesma transação**: uma âncora gravada sem a linha
faria a próxima abertura ressuscitar um id que nunca existiu no hub.

Isto conserta o caso do PC2 daqui para a frente. Não desfaz o que já aconteceu —
a identidade antiga (`01a02490`) e a nova (`01a04658`) continuam as duas no log
do hub, e não há o que limpar: as entidades são as mesmas, e o LWW por campo
resolve qualquer disputa entre elas.

## 2. A malha, no hub e na tela

### No hub

Tabela nova, e ela guarda só o que descreve o aparelho:

```sql
CREATE TABLE IF NOT EXISTS aparelhos (
    id          TEXT PRIMARY KEY,   -- o mesmo DeviceId que assina as operações
    nome        TEXT NOT NULL,
    plataforma  TEXT NOT NULL,
    versao      TEXT NOT NULL,      -- versão do app, como "0.3.5"
    contrato    INTEGER NOT NULL,   -- a versão de contrato que ele fala
    visto_em    TEXT NOT NULL       -- RFC3339, hora do servidor
);
```

Duas rotas, com o mesmo `Authorization: Bearer` das outras:

- `POST /sync/aparelho` — corpo `{id, nome, plataforma, versao, contrato}`.
  Upsert, e a hora é a **do servidor**: relógio de cliente errado é comum, e um
  "visto há 3 dias" que na verdade foi agora mandaria a investigação para o lado
  errado.
- `GET /sync/aparelhos` — a lista, ordenada por `visto_em` decrescente.

**Por que isto não quebra a regra do `http.rs`.** O topo daquele arquivo diz que
uma terceira rota que o `Transport` não pede significa "alguém colocou regra no
servidor". Estas duas não carregam regra nenhuma: o hub grava o que o aparelho
diz de si e devolve a lista, sem decidir nada com isso — nenhuma operação é
recusada por versão, nenhum cliente é bloqueado. O comentário do arquivo será
atualizado para dizer isso, em vez de ser contrariado em silêncio.

### No cliente

A batida **não entra no trait `Transport`**. O trait espelha o que o motor
precisa (`push`, `pull`), e a identidade do aparelho não é assunto do motor. Ela
vira método próprio de `HttpTransport` (`mos-sync-http`), chamado pela camada do
app antes da rodada — desktop e `mos-web` chamam o mesmo método.

Falha na batida **não interrompe a rodada**: quem não conseguiu se anunciar
ainda tem trabalho a sincronizar, e trocar dado por metadado seria péssimo
negócio. O erro vai para o log.

### Na tela

Em Settings → SINCRONIZAÇÃO, uma seção nova, "A MALHA":

```
DESKTOP-634TJR1   0.3.5   este aparelho
PC-TRABALHO       0.3.5   há 12 min
M/OS de bolso     0.3.5   há 1 h
```

Quando alguma versão difere da deste aparelho, a linha ganha o aviso — texto,
não bloqueio: *"em versão diferente: 0.3.3"*. É a frase que teria encerrado a
investigação de 02/09 em dois segundos.

## 3. O backfill passa a ter geração

`sync_backfill_v1` vira `sync_backfill_geracao`, com o valor sendo o número da
geração já passada. No código, uma constante:

```rust
/// A geração da cobertura. Sobe quando `sync_cobertura.rs` passa a incluir
/// tipos que antes não atravessavam — e é isso que faz o backfill rodar de
/// novo em quem já tinha passado por ele.
const GERACAO_ATUAL: u32 = 2;
```

Regra: se a marca gravada for menor que `GERACAO_ATUAL`, o backfill roda e grava
a geração nova. Igual ou maior, devolve zero como hoje.

A geração começa em **2** porque a v1 cobria 12 tipos e a cobertura atual cobre
26. A migração lê a marca antiga: quem tem `sync_backfill_v1` recebe
`sync_backfill_geracao = 1`, e portanto re-emite — que é exatamente o objetivo.

O preço, dito na cara: re-emitir tudo gera tráfego e operações repetidas no log
do hub. É o preço de o dado velho atravessar, e ele é pago uma vez por geração.

**Quem sobe a geração é quem muda a cobertura.** O teste que hoje recusa tabela
nova sem classificação (`sync_cobertura.rs`) ganha um irmão: um teste que falha
se a lista de tipos sincronizáveis mudou sem a geração subir. Sem isso, esta
mesma armadilha volta na próxima cobertura — e ela já voltou uma vez.

## 4. Um endereço só

O Caddy passa a separar por caminho:

```
167-233-43-1.sslip.io {
	@sync path /sync/*
	handle @sync {
		reverse_proxy 127.0.0.1:9120
	}
	handle {
		basic_auth {
			<usuario> <hash>
		}
		reverse_proxy 127.0.0.1:9130
	}
}
```

`/sync/*` fica **fora** do `basic_auth` por necessidade, e não por descuido: o
cliente manda `Authorization: Bearer`, e o `basic_auth` do Caddy recusaria antes
de o hub ver o token. A proteção do hub é o segredo de 64 caracteres que já
existe, comparado em tempo constante (`http.rs`, `tempo_constante`).

O que isto expõe, dito com todas as letras: o hub passa a ser alcançável da
internet. Quem tiver o segredo pode ler e escrever o log inteiro. Era assim
antes também para quem tivesse o segredo **e** uma chave SSH; agora basta o
segredo. A troca é deliberada: some a única peça que quebra calada.

Depois disso, nos dois PCs:

- o endereço em Settings vira `https://167-233-43-1.sslip.io`;
- a tarefa agendada **`M-OS Sync Tunnel`** é removida
  (`Unregister-ScheduledTask -TaskName "M-OS Sync Tunnel" -Confirm:$false`);
- `scripts/sync-tunnel.ps1` e `scripts/install-sync-tunnel.ps1` saem do
  repositório. Script que sobra é script que alguém roda de novo.

O `mos-web` na VPS **não muda**: ele fala com o hub por `127.0.0.1:9120`, na
mesma máquina, e passar pelo Caddy só acrescentaria um salto.

O `deploy/bootstrap-vps.sh` é idempotente e absorve o Caddyfile novo; rodá-lo de
novo é o caminho oficial de aplicar isto.

## Como isto é conferido

**Testes:**

- Apagar a linha de `devices` e chamar `este_dispositivo` devolve o **mesmo id**
  (âncora), e o `pull_cursor` sobrevive.
- Banco sem âncora e sem linha cria um id e grava a âncora — as duas coisas, ou
  nenhuma.
- Marca de geração menor faz o backfill re-emitir; marca igual devolve zero.
- A lista de tipos sincronizáveis mudou sem a geração subir → o teste falha.
- `POST /sync/aparelho` grava e `GET /sync/aparelhos` devolve; token errado dá
  401 nas duas.
- A batida falha e a rodada acontece assim mesmo.

**Na máquina, e não só em teste:**

- `curl https://167-233-43-1.sslip.io/sync/pull?contrato=1&cursor=&limite=1` sem
  token → **401** (e não o desafio de `basic_auth`); com o token → **200**.
- `curl https://167-233-43-1.sslip.io/` continua pedindo `basic_auth`.
- Com o túnel derrubado e a tarefa removida, o desktop sincroniza.
- A tela da malha mostra os três aparelhos, e o teste manual é desligar a rede
  de um deles e ver o "há N min" crescer.

## O que fica para depois

O item 1 da conversa — **por que o CronoCAD não apareceu no PC2** — não entra
aqui. A rodada já converge por conta própria (`sincronizar_agora` repete
passadas até `pendentes == 0 && !tem_mais`, com teto de 100 passadas), então a
hipótese de "trouxe só 100" está descartada. Restam duas, e elas pedem consertos
opostos: materialização que não converge (`sync_state` cheio, tabelas de domínio
vazias) ou rodada que para cedo por operação que o hub recusa (`sync_outbox` com
`attempts` alto e `last_error` preenchido). O diagnóstico do PC2 decide qual, e
o conserto ganha spec próprio.
