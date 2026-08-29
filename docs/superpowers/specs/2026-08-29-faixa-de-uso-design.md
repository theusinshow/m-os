# A faixa de uso — quanto da cota de IA já foi, sem inventar o teto — Design

**Status:** implementado · ver `DECISIONS.md`, ADR-059, para o que a tela mudou no desenho (o hover virou clique, e a faixa virou duas janelas)

**Data:** 2026-08-29

**Baseline:** M/OS `v0.3.2` no commit `fc83b29`, que reabriu o portão do CI.

**Origem:** pedido do dono do produto, a partir de um mockup de menubar do macOS
— uma faixa preta colada na borda da tela com um anel por provedor de IA
(Claude, OpenAI, um terceiro), o percentual embaixo de cada um, e um popover com
duas leituras: "sessão atual, reseta em 51 min, 73% usado" e "todos os modelos,
reseta quinta 12:00, 7% usado".

---

## 1. O que o mockup pede, e o que a máquina pode entregar

O mockup promete três coisas. Duas a máquina entrega com precisão; a terceira
não existe em lugar nenhum, e é onde este desenho gasta a maior parte do seu
esforço.

| O que o mockup mostra | De onde sairia |
| --- | --- |
| quanto foi consumido | `usage` de cada request nos transcripts locais |
| quando a janela reseta | derivado do primeiro evento da janela de 5h |
| **de quanto é o teto** | **não existe** |

Os transcripts em `~/.claude/projects/**/*.jsonl` foram inspecionados nesta
máquina antes de qualquer linha de desenho. Cada linha de assistente carrega:

```
timestamp, message.model, requestId,
usage: { input_tokens, cache_creation_input_tokens,
         cache_read_input_tokens, output_tokens,
         output_tokens_details.thinking_tokens }
```

Não carrega teto de cota, não carrega hora de reset, não carrega quanto do plano
Max já foi queimado. O Claude Code simplesmente não grava isso.

Isso colide de frente com a regra que o `Ring.tsx` fixou por escrito:

> Um anel bonito preenchido com número inventado é pior que a ausência: ele
> ensina a confiar numa medida que não existe.

Então o "73%" do mockup não pode ser copiado. Ele precisa de um denominador que
a máquina realmente conheça.

### 1.1 O denominador: o próprio pico

A decisão é medir **contra o maior consumo já observado numa janela de 5h**, e
dizer isso no rótulo. É exatamente a doutrina que o `WeekRings` já aplica na
Home:

> A proporção é contra o melhor dia da semana, e não contra uma meta. O M/OS não
> tem meta diária de tasks, e inventar uma faria o anel medir uma régua que
> ninguém definiu.

O anel passa a responder uma pergunta que tem resposta — *"esta sessão está
pesada comparada às minhas sessões pesadas?"* — em vez de uma que não tem —
*"quanto falta para a Anthropic me barrar?"*. Perde-se a previsão do bloqueio;
ganha-se um número que não mente.

Teto declarado nas Settings foi considerado e recusado para o v1: dependeria de
um número que a Anthropic não publica, e "calibrar no chute" é a mesma invenção
com um passo a mais.

### 1.2 O achado que muda o cálculo: 36% das linhas são repetidas

O maior transcript da máquina foi contado:

```
linhas com "usage":  3277
requestId únicos:    2108
```

Um terço das linhas com `usage` repete um `requestId` já visto. Somar linha a
linha inflaria o consumo em cerca de 55%.

Isso promove a deduplicação de detalhe de implementação a **requisito de
corretude**, e é o que decide o formato da tabela na §3: a chave primária é o
`requestId`, e não uma linha de log.

### 1.3 A grandeza medida

Somar os quatro campos de token crus não funciona. No último request desta
sessão os números foram `cache_read: 73243` contra `output: 496` — o cache lido
engoliria o resto e o anel viraria um medidor de tamanho de contexto.

A soma é ponderada pelos **multiplicadores de preço publicados**, normalizados
ao token de input:

| campo | peso |
| --- | --- |
| `input_tokens` | 1 |
| `cache_creation_input_tokens` | 1,25 |
| `cache_read_input_tokens` | 0,1 |
| `output_tokens` | 5 |

A unidade resultante é "tokens-equivalentes-de-input". Ela não é preço em reais
e não pretende ser: é uma razão publicada, o que a torna verificável, e é
proporcional ao que a cota consome. `thinking_tokens` não entra separado — já
está contado dentro de `output_tokens`.

---

## 2. `crates/mos-usage` — o leitor, puro

Crate nova, sem Tauri e sem SQLite. Fica isolada porque analisa o formato de
arquivo de uma ferramenta de terceiro, que pode mudar sem aviso: quando mudar,
o teste que quebra deve apontar para um crate de 400 linhas, e não para o
domínio pessoal do M/OS.

Três responsabilidades:

- **`parse_linha(&str) -> Option<Evento>`** — extrai `timestamp`, `model`,
  `requestId` e os quatro campos de `usage`. Linha sem `usage` — user,
  tool_result, sidechain, resumo — devolve `None`. Ausência não é erro: a
  maioria das linhas de um transcript não é um request.
- **`peso(&Evento) -> u64`** — a soma ponderada da §1.3, em milésimos para não
  perder o `0,1` do cache lido na aritmética inteira.
- **`janelas(eventos) -> Vec<Janela>`** — agrupa em blocos de 5h. A janela abre
  no primeiro evento **arredondado para a hora cheia**, fecha 5h depois, e um
  intervalo maior que 5h sem evento abre a próxima.

O `Evento` guarda o `requestId` porque a deduplicação da §1.2 acontece na
camada de persistência, não aqui: o leitor não tem memória entre arquivos.

### 2.1 Leitura incremental

`~/.claude/projects` tem **507 MB** nesta máquina, em 18 projetos. Reler tudo a
cada tique está fora de questão.

O leitor guarda, por arquivo, `offset`, `tamanho` e `mtime`:

- `mtime` e `tamanho` iguais aos guardados → nem abre o arquivo;
- `tamanho > offset` → lê só o delta a partir do `offset`;
- `tamanho < offset` → o arquivo foi reescrito ou truncado; relê do zero.

A leitura incremental é **otimização, não corretura**. A corretude vem da chave
primária do `requestId`: se todo o estado de offset for perdido, uma varredura
completa produz exatamente o mesmo resultado.

---

## 3. `0036_usage.sql` — três tabelas

Próxima migration livre é a 0036; o banco atual está em `user_version = 35`.

```sql
CREATE TABLE usage_requisicao (
  request_id     TEXT PRIMARY KEY,
  em             TEXT NOT NULL,
  modelo         TEXT NOT NULL,
  janela_inicio  TEXT NOT NULL,
  peso           INTEGER NOT NULL
);
CREATE INDEX idx_usage_requisicao_janela ON usage_requisicao(janela_inicio);

CREATE TABLE usage_fonte (
  caminho  TEXT PRIMARY KEY,
  offset   INTEGER NOT NULL,
  tamanho  INTEGER NOT NULL,
  mtime    INTEGER NOT NULL
);

CREATE TABLE usage_janela (
  inicio       TEXT PRIMARY KEY,
  fim          TEXT NOT NULL,
  peso         INTEGER NOT NULL,
  requisicoes  INTEGER NOT NULL
);
```

`usage_requisicao` é a fonte da verdade. A chave primária mata a duplicação de
36% por construção — `INSERT OR IGNORE` — e torna a varredura idempotente.

`usage_janela` é agregado derivado, e existe por uma razão só: o pico precisa
sair de um `SELECT MAX(peso) FROM usage_janela`, e não de uma soma sobre
duzentas mil linhas a cada 30 segundos.

Estimativa de volume: da ordem de 2×10⁵ requisições nos 507 MB, algo como 15 MB
de tabela. Cabe no banco local sem cerimônia.

---

## 4. `src-tauri/src/usage.rs` — o laço e os comandos

No mesmo molde do `attention.rs`, que já é o dono do tempo no processo Rust —
o renderer nunca agenda nada.

Um laço acorda a cada 30s, percorre os `.jsonl`, insere as requisições novas e
recalcula as janelas tocadas. Comandos expostos:

- `usage_faixa() -> Faixa` — `{ fontes: [...], calibrando: bool }`, onde cada
  fonte traz `{ nome, peso_atual, pico, proporcao, reseta_em, requisicoes,
  peso_hoje, pico_dia }`.
- `faixa_expandir(bool)` — redimensiona a janela, ver §5.

### 4.1 A primeira carga

Os 507 MB são varridos **uma vez**, em background, fora do caminho do boot.
Enquanto ela não termina, `calibrando` é `true`.

E enquanto `calibrando` é `true` a faixa desenha **o trilho e o peso absoluto no
centro, sem percentual**. Um anel preenchido contra um pico que ainda não foi
observado seria precisamente o número inventado que a §1 recusa. É também o que
o `Ring.tsx` já manda fazer no zero: "mostra só o trilho e o número".

---

## 5. A faixa

Janela `faixa` nova no `tauri.conf.json`, com as mesmas chaves que `lembrete` já
usa: `transparent`, `alwaysOnTop`, `skipTaskbar`, `decorations: false`,
`focus: false`, `shadow: false`, `visible: false`.

Posicionada colada à borda direita e centrada na vertical, pelo mesmo
`current_monitor()` + `set_position` que o `monitor.rs` usa para o lembrete.

Ela **não rouba o foco**, pela mesma razão registrada lá: quem está com as mãos
no teclado não pediu por uma janela nova.

**Repouso:** 132px de largura. Um anel por fonte — hoje só Claude Code —
desenhado com o `Ring` de 56px que já existe, sódio para carga, percentual
embaixo.

**Hover:** o frontend chama `faixa_expandir(true)`, o backend redimensiona para
560px crescendo para a esquerda, e o popover abre com duas leituras:

- **sessão** — peso da janela de 5h corrente, proporção contra o pico, e quanto
  falta para o reset. O reset é hora exata, derivada do início da janela, e não
  estimativa;
- **hoje** — peso do dia contra o maior dia já observado.

Sair volta para 132px.

Redimensionar em vez de manter uma janela larga e transparente é deliberado:
uma janela larga cobriria o desktop e exigiria `ignore_cursor_events`, que
mataria o próprio hover que precisa ser detectado.

Clique no anel abre a janela principal do M/OS. Sem menu e sem configuração na
faixa.

---

## 6. Erros

- arquivo ilegível — pulado, registrado no `diagnostico.rs` que já existe;
- linha malformada — incrementa um contador, a varredura segue. Um transcript
  cortado no meio por um crash é normal, não excepcional;
- `~/.claude` ausente — nenhuma fonte, e sem fonte a faixa **não monta**. Ela
  não aparece vazia esperando dado que nunca virá.

---

## 7. Testes

- `mos-usage`: linha real com `usage`, linha sem, linha corrompida; peso
  conferido contra um evento de números conhecidos; agrupamento com intervalo
  maior que 5h; e o caso que a §1.2 tornou obrigatório — **o mesmo `requestId`
  duas vezes conta uma vez**.
- Leitura incremental em `tempdir`: crescer o arquivo lê só o delta; truncar
  relê do zero; `mtime` parado não reabre.
- `mos-storage-sqlite`: a 0036 aplica sobre um banco em `user_version = 35`;
  o upsert de janela é idempotente; varrer duas vezes o mesmo arquivo não muda
  o peso.
- Renderização da faixa na bancada headless com o CSS real, antes de declarar
  pronto.

---

## 8. Fora do v1

- OpenAI e outros provedores. A interface `Fonte` nasce plugável para eles, mas
  nenhum adaptador é escrito — o mockup mostra três anéis, e três anéis com dois
  deles inventados seria o erro da §1 em triplicado;
- teto declarado nas Settings;
- histórico navegável de consumo.
