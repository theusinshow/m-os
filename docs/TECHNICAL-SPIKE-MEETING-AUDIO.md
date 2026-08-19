# M/OS — Meeting Audio Technical Spike

## 1. Status

**Status:** concluído — captura WASAPI aceita para a V1 do Meeting Agent

**Data:** 2026-08-18

**Escopo:** experimento descartável; nenhum código deste diretório é código de produto

**Implementação:** `spikes/meeting-audio`

**Subordinado a:** `MEETING-AGENT.md`, cujas decisões abertas D-1 a D-5 este documento fecha

## 2. Objetivo

Provar, antes de existir banco, página, Hermes ou qualquer interface, que o M/OS
consegue capturar simultaneamente o microfone e o áudio do Windows, em canais
separados, de forma durável e recuperável.

O `MEETING-AGENT.md` §25 condiciona todo o resto a isto:

> **Se o Gate A falhar, o trabalho para e este documento é reaberto.** Não haverá
> UI construída sobre uma captura que não funciona.

O spike não implementa Meeting, transcrição, análise, persistência ou interface.

## 3. Ambiente

| Item | Valor observado |
|---|---|
| Sistema | Windows 11 Pro x64, build 10.0.26200 |
| CPU | AMD Ryzen 9 9950X3D, 16 núcleos / 32 threads |
| RAM | 47,2 GB |
| Rust | 1.97.0, target `x86_64-pc-windows-msvc` |
| MSVC Build Tools | 17.14.37411.7 |
| `wasapi` | 0.24.0 |
| `windows-sys` | 0.61.2 |

### 3.1 O ambiente de áudio, que acabou sendo parte do achado

Esta máquina é um rig de streaming, e isso **mudou o desenho do experimento**:

```text
entrada (padrão)   Microfone (Voicemod)              ← virtual
entrada            Microfone (Logitech G733 Gaming Headset)
entrada            Microfone (Yeti GX)
entrada            Stream / Chat / Personal Mix (Elgato Virtual Audio)

saída (padrão)     System (Elgato Virtual Audio)     ← virtual
saída              Alto-falantes (Logitech G733 Gaming Headset)
saída              QN90D (NVIDIA High Definition Audio)
saída              Game / Music / Voice chat (Elgato Virtual Audio)
saída              Dummy Output (Voicemod)
```

Tanto a entrada quanto a **saída padrão são dispositivos virtuais**. Isso importa
porque um driver virtual mantém stream ativo o tempo todo — e, como a §5.3 mostra,
isso **esconde** o defeito que o experimento mais importante procurava. O primeiro
teste da D-2 deu um falso positivo por causa disso, e só a repetição contra um
endpoint ocioso de verdade produziu a resposta.

## 4. Matriz de evidências

| Capacidade | Procedimento | Resultado observado | Estado |
|---|---|---|---|
| Enumeração de dispositivos | `devices` | 6 entradas e 7 saídas listadas, com o padrão marcado | Passou |
| Captura de microfone | `record --secs 30` | 16000/1/i16, 2.998 pacotes, deriva de 2 ms | Passou |
| Captura de áudio do sistema | idem, canal `system` | loopback abriu no dispositivo de saída; 3.001 pacotes | Passou |
| Os dois simultaneamente | `record --secs 30` | ambos gravaram; divergência entre canais de 10–22 ms | Passou |
| Loopback + exclusive mode | leitura do crate | recusado com `WasapiError::LoopbackWithExclusiveMode` | Confere com a doc da Microsoft |
| Modo por evento no loopback | `--timing events` | funciona; maior intervalo 11 ms; zero fallbacks | Passou (D-1) |
| Modo por polling | `--timing polling` | funciona; maior intervalo 16 ms; 2,4× a CPU | Passou, inferior |
| Loopback em endpoint ocioso, sem keep-alive | `--no-keepalive --system-device "QN90D…"` | **0 frames, 0 pacotes em 25 s** | **Falha esperada — é o defeito** |
| Loopback em endpoint ocioso, com keep-alive | `--system-device "QN90D…"` | 24.978 ms, 2.498 pacotes | Passou (D-2) |
| `autoconvert` junto com loopback | `record` padrão | 16000/1/i16 aceito nos dois canais | Passou (D-3) |
| Formato nativo | `--no-autoconvert` | 48000/2/f32; 23,0 MB em 30 s contra 0,96 MB | Passou, 24× o disco |
| Gravação incremental | 45 s, chunks de 10 s | 5 chunks por canal, fechados e sincronizados | Passou |
| Recuperação após queda | `Stop-Process -Force` aos 45 s, depois `inspect` | **44 s recuperados nos dois canais, 0 bytes soltos** | Passou (Gate B) |
| Gravação longa | `record --secs 900` | 15 min, deriva de 1 ms, 0 descontinuidades, 0 erros | Passou |

### 4.1 Testes automatizados

`cargo test` — 12 testes, todos passando, e nenhum deles toca o Windows:

- rotação de chunk no limite exato;
- escrita menor que o chunk não rotaciona;
- **escrita desalinhada é recusada** — meio frame no arquivo desalinharia tudo
  que viesse depois, sem sintoma até alguém ouvir ruído branco;
- **chunk truncado conta até o último frame inteiro** e relata o resto;
- diretório inexistente devolve vazio, não erro;
- duração derivada de frames;
- deriva zero quando frames batem com o relógio;
- deriva positiva significa áudio faltando;
- canal sem frames reprova;
- canais divergentes reprovam;
- intervalo grande entre pacotes reprova;
- o caso bom não produz falha.

O veredito é **calculado com limites fixados antes do teste**. Um limite decidido
depois de olhar o resultado é uma desculpa, não um critério.

## 5. Resultados, por decisão aberta

### 5.1 D-1 — o modo por evento dispara no loopback? **Sim.**

A documentação da Microsoft diz que clientes de loopback por evento são suportados
desde o Windows 10 1703, mas há relato de campo consistente de que o evento nunca
dispara. **No 26200 ele dispara.**

| Modo | Maior intervalo, mic | Maior intervalo, system | CPU |
|---|---:|---:|---:|
| `events` | 11 ms | 11 ms | 0,26 % |
| `polling` | 16 ms | 16 ms | 0,62 % |

Zero fallbacks registrados em todas as corridas. **Decisão: `EventsShared`**, com o
polling mantido como degradação declarada — a captura troca de modo depois de três
timeouts seguidos e **grava que trocou**, porque degradar em silêncio seria
prometer no relatório um modo que não foi usado.

### 5.2 D-3 — `autoconvert` funciona junto com loopback? **Sim.**

`AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM` foi aceito nos dois canais, inclusive no
loopback. Pedimos 16 kHz mono i16 e foi o que recebemos.

| | Formato efetivo | 30 s em disco |
|---|---|---:|
| com `autoconvert` | `16000/1/i16` | 0,96 MB |
| sem | `48000/2/f32` | 23,04 MB |

**24× o disco** para informação que nenhum consumidor lê. A §5.6 do
`MEETING-AGENT.md` fica confirmada com número medido, e o fallback de resample
próprio deixa de ser necessário nesta máquina — mas continua no desenho, porque
"funciona aqui" não é "funciona".

### 5.3 D-2 — o keep-alive de silêncio é necessário? **Sim, e é obrigatório.**

Este é o achado que justifica o spike inteiro.

**Primeira medição, contra a saída padrão (Elgato Virtual Audio):**

| keep-alive | gravado | pacotes |
|---|---:|---:|
| ligado | 30.008 ms | 3.001 |
| desligado | 29.998 ms | 3.000 |

Nenhuma diferença. A leitura fácil seria "o keep-alive é dispensável no Windows 11"
— e ela estaria **errada**. A saída padrão é um driver virtual da Elgato, que
mantém stream ativo permanentemente e por isso alimenta o loopback sozinho.

**Segunda medição, contra um endpoint físico ocioso (NVIDIA HDMI):**

| keep-alive | gravado | pacotes |
|---|---:|---:|
| ligado | 24.978 ms | 2.498 |
| **desligado** | **0 ms** | **0** |

O buraco do loopback é **real e total**. Sem um stream de renderização ativo, o
canal SYSTEM não produz um único frame.

Numa reunião isso significa: enquanto ninguém do outro lado fala, o canal remoto
não existe. Não é um arquivo menor — é a linha do tempo do SYSTEM deixando de
corresponder à do MIC, e a evidência `14:04` passando a apontar para o lugar
errado pelo resto da reunião.

**Decisão: o keep-alive entra, e não é opcional.** Ele escreve zeros no endpoint
de saída enquanto a gravação existir. Não produz som e custa um stream
compartilhado ocioso.

**E um segundo achado, sobre como NÃO medir isso:** a hipótese original era que o
silêncio chegaria como pacote marcado `AUDCLNT_BUFFERFLAGS_SILENT`. Não chega —
o keep-alive escreve zeros de verdade, e o loopback os entrega como áudio comum.
O contador `silentPackets` ficou em zero em **todas** as corridas. O sinal honesto
da D-2 é a ausência de **frames**, e o veredito foi corrigido para usá-lo. Um
critério baseado na flag acusaria falha justamente na configuração que funciona.

### 5.4 D-5 — qual a deriva real?

| Corrida | Canal | Gravado | Relógio do device | Deriva |
|---|---|---:|---:|---:|
| 30 s, events | mic | 29.978 ms | 29.980 ms | 2 ms |
| 30 s, events | system | 29.988 ms | 29.989 ms | 1 ms |
| 30 s, sem autoconvert | mic | 29.990 ms | 29.990 ms | 0 ms |

Divergência entre canais: **10 a 22 ms**. O orçamento da §19 do `MEETING-AGENT.md`
é de 200 ms em 60 minutos, e a corrida longa da §5.6 o confirma na escala certa.

**Uma armadilha de medição, registrada porque custou uma correção:** a primeira
versão do spike reportou **639.071 frames perdidos** em 20 s. Era falso. Com
`autoconvert` ligado, `BufferInfo.index` conta em frames do **dispositivo**
(48 kHz) enquanto lemos frames convertidos (16 kHz); a razão de 3:1 explica o
número exatamente. O contador foi corrigido para escalar pela razão, num
acumulador `f64` — com `u64`, o arredondamento por pacote somaria milhares de
frames fantasmas ao longo de uma hora.

Depois da correção o contador cai para ~31 frames em 30 s (≈2 ms) com
`autoconvert`, e **exatamente 0** sem ele. Isso é buffer do resampler, não áudio
perdido — e a prova é a deriva de 1 ms contra o relógio do dispositivo, que é o
número autoritativo. Foi mantido no relatório como `[NOTA]`, nunca como falha.

O mesmo cuidado valeu para a descontinuidade: o WASAPI marca
`DATA_DISCONTINUITY` no primeiro pacote depois do `Start`, **sempre**. Contá-la
faria toda gravação nascer com uma falha que não aconteceu.

### 5.5 Gate B — recuperação após queda

Procedimento: `Start-Process` do gravador, `Stop-Process -Force` aos 45 s — sem
Stop, sem flush final, sem chance de limpeza — e depois `inspect`.

```text
  mic      5 chunks · 705249 frames · 00:00:44 · 0 bytes soltos
  system   5 chunks · 705249 frames · 00:00:44 · 0 bytes soltos

  Recuperavel: 00:00:44
```

**44 de ~45 segundos recuperados, 97,8 %.** A perda é o conteúdo do `BufWriter` de
64 kB, que a 32 kB/s vale no máximo dois segundos.

Dois detalhes que valem mais que o número:

- **zero bytes soltos** — nenhum arquivo terminou no meio de um frame;
- **os dois canais recuperaram exatamente o mesmo número de frames** (705.249).
  A linha do tempo compartilhada sobrevive à queda, que é o que faz a evidência
  continuar apontando para o lugar certo depois de uma recuperação.

### 5.6 Corrida longa

`record --secs 900`, os dois canais, `events`, keep-alive ligado, com áudio real
tocando durante boa parte da corrida.

| | MIC (Voicemod) | SYSTEM (Elgato) |
|---|---:|---:|
| Tempo de parede | 900,3 s | 900,3 s |
| Gravado | 900.298 ms | 900.318 ms |
| Relógio do dispositivo | 900.299 ms | 900.319 ms |
| **Deriva** | **1 ms** | **1 ms** |
| Pacotes | 90.030 | 90.032 |
| Descontinuidades | **0** | **0** |
| Erros de leitura | **0** | **0** |
| Fallbacks de timing | **0** | **0** |
| Maior intervalo | 20 ms | 17 ms |
| Chunks | 91 | 91 |
| Bytes soltos | **0** | **0** |

Divergência entre canais: **−20 ms em 15 minutos**. CPU 0,29 % de um núcleo,
pico de memória 11 MB, 55,0 MB em disco para os dois canais (61 kB/s).

**Deriva de 1 ms em 15 minutos** extrapola para ~4 ms em uma hora, contra um
orçamento de 200 ms. A âncora de tempo compartilhada entre os dois canais está a
duas ordens de grandeza de folga.

### Os 31 frames, e por que eles não acumulam

O contador de frames perdidos marcou **31 em 30 segundos e os mesmos 31 em 900
segundos**. Um número que não cresce com o tempo não é uma taxa — é um
deslocamento único, do enchimento do resampler no início do stream. Se fosse
perda real de áudio ele teria virado ~930 frames em 15 minutos.

A prova independente é a deriva de 1 ms contra o relógio do dispositivo: se 31
frames tivessem sumido de verdade, ela seria de 2 ms, e se a perda fosse
proporcional, de 58 ms. **Nada foi perdido.** O contador fica no relatório como
`[NOTA]`, nunca como falha.

### Verificação do conteúdo, e não só da contagem

Contar frames prova que bytes chegaram; não prova que eles são o áudio certo.
Duas checagens fecham isso:

**Integridade.** Todos os 90 chunks fechados de cada canal têm exatamente
320.000 bytes — 10 s a 16 kHz mono i16. Nenhum chunk curto, nenhum buraco.

**Sinal.** Um tom senoidal de 440 Hz foi tocado por 15 s na saída padrão durante
a corrida. A energia por frequência no canal `system`, por Goertzel:

| chunk | janela | 220 Hz | 440 Hz | 880 Hz | 1300 Hz |
|---:|---|---:|---:|---:|---:|
| 22 | 220–230 s | 0,00007 | 0,00029 | 0,00001 | 0,00003 |
| **23** | **230–240 s** | 0,00005 | **0,00435** | 0,00005 | 0,00007 |
| **24** | **240–250 s** | 0,00000 | **0,01066** | 0,00000 | 0,00000 |
| 25 | 250–260 s | 0,00000 | 0,00000 | 0,00000 | 0,00000 |

O tom aparece **exatamente na janela em que foi tocado**, 40 a 100× acima das
frequências vizinhas e sem harmônicos espúrios. O loopback entrega o sinal, e não
apenas frames.

E uma observação que vale mais que o tom: ao longo da corrida, os vales de
silêncio do canal `mic` e do canal `system` **coincidem chunk a chunk** — o
chunk 004 está mudo nos dois, o 011 está no pico nos dois. É a linha do tempo
compartilhada visível no dado.

### Nota sobre o veredito desta corrida

O `report.json` desta corrida traz a linha antiga sobre `silentPackets`,
porque ela foi iniciada antes da correção descrita na §5.3. O número que importa
— frames — está correto; só a frase do veredito é da versão anterior.

## 6. Medidas

| Métrica | Orçamento (`MEETING-AGENT.md` §19) | Medido |
|---|---|---|
| CPU, processo inteiro | < 2 % | 0,23–0,62 % de um núcleo; 0,29 % em 15 min |
| Memória, pico | < 60 MB | 10–11 MB |
| Disco, dois canais | ~64 kB/s | 64,0 kB/s (0,96 MB / 30 s / canal) |
| Deriva em 60 min | < 200 ms | **1 ms em 15 min** (~4 ms/h extrapolado) |
| Intervalo entre pacotes | — | 10–11 ms (events) |

O polling custa 2,4× a CPU do modo por evento para um intervalo pior. É a
justificativa quantitativa de `EventsShared` ser o padrão.

## 7. Decisão

**A captura WASAPI é aceita para a V1 do Meeting Agent.** Gate A e Gate B passam.

Ficam aceitos, e passam a valer como decisão de arquitetura:

- `wasapi` 0.24 como adapter, num crate `mos-audio` Windows-only sem acesso ao
  banco;
- dispositivo de **saída** aberto como captura para o loopback, shared mode;
- **keep-alive de silêncio obrigatório** enquanto houver gravação (D-2);
- `StreamMode::EventsShared` com `autoconvert`, pedindo 16 kHz mono i16 (D-1, D-3);
- polling como degradação **declarada e registrada**, nunca silenciosa;
- chunks de PCM cru de 10 s, com `sync_all` no fechamento;
- duração medida em frames em disco, nunca por diferença de relógio;
- `BufferInfo.index` escalado pela razão de taxas antes de virar "frame perdido";
- a primeira `DATA_DISCONTINUITY` depois do `Start` não conta como defeito.

**O código do spike não será promovido por cópia.** O `mos-audio` de produção
recria apenas os padrões aprovados, com erro próprio, testes próprios e a
fronteira estrutural que a §4.2 do `MEETING-AGENT.md` define.

## 8. Limitações e o que continua aberto

Não bloqueiam a Fase 2, mas bloqueiam o Gate G:

- **desconexão de dispositivo no meio da gravação não foi exercitada** (D-4). O
  spike não implementa reconexão; ele registra `Lost{at_ms}` quando a leitura
  falha. Arrancar o headset dez vezes é QA manual, e a política da §20 —
  reconectar uma vez e, falhando, falhar explicitamente — continua a ser provada;
- **troca de dispositivo padrão no meio** também não foi exercitada;
- disco cheio não foi injetado;
- a máquina de teste tem áudio virtual (Elgato, Voicemod). O comportamento numa
  máquina limpa foi inferido do teste no endpoint HDMI ocioso, não observado
  isoladamente;
- nenhuma reunião real foi gravada ainda: os testes usaram silêncio e áudio do
  sistema, não uma chamada;
- áudio protegido por DRM não é capturável por loopback, por construção do
  Windows. Não afeta reunião, mas afeta qualquer expectativa de "grava tudo".

## 9. Reproduzir

```powershell
cd C:\...\m-os\spikes\meeting-audio
cargo test
cargo build --release

.\target\release\meeting-audio-spike.exe devices

# o experimento que importa: escolha uma saída que NADA esteja usando
.\target\release\meeting-audio-spike.exe record --secs 25 --system-only `
    --system-device "QN90D (NVIDIA High Definition Audio)" --no-keepalive --out .\sem-keepalive
.\target\release\meeting-audio-spike.exe record --secs 25 --system-only `
    --system-device "QN90D (NVIDIA High Definition Audio)" --out .\com-keepalive
```

Numa máquina com áudio virtual, **não use a saída padrão para este teste**: o
driver virtual mantém o motor acordado e o resultado dá falso positivo.

O diretório `target` é ignorado pelo Git. As sessões gravadas contêm áudio e não
devem ser versionadas.
