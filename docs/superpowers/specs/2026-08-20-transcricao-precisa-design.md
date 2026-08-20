# Transcrição precisa — o ganho, os freios e o colapso

**Data:** 2026-08-20
**Estado:** desenho aprovado, à espera do plano
**Origem:** o dono relatou "a transcrição não tá muito precisa, tem como pegar de fato o
que foi dito?" depois da primeira reunião real gravada pelo M/OS.

---

## 1. O que a medição estabeleceu

Nada aqui é palpite. Uma reunião real de **6 minutos** (`01a01f95-125a-75e2-95c4-07031596446e`,
dois canais, voz nos dois) foi transcrita **onze vezes**, variando um fator por vez.

O corpus está preservado fora do app, porque a retenção da reunião é `delete_after_processing`.

### 1.1 O problema não estava onde eu apostei

| descoberta | evidência |
|---|---|
| O canal do sistema **já estava bom** | 721 palavras coerentes, jargão de engenharia certo |
| O canal do mic estava inutilizável | 96 de 123 segmentos eram muleta; `Tchau.` 24× seguidas; `Coi coi coi coi...` |
| A causa é o **nível do sinal** | mic a −44 dBFS; canal do sistema a −22 dBFS |
| Não é erro de reconhecimento, é **laço em silêncio** | as 24 repetições nascem depois dos 325 s, quando a ligação já tinha acabado |

Eu havia apostado no modelo como alavanca principal. A medição desmentiu: trocar o
`large-v3-turbo-q5_0` pelo `large-v3` sozinho melhorou pouco (23 frases reais contra 17), e
**não** resolveu o laço.

### 1.2 O VAD, e a armadilha dentro dele

| rodada | frases reais (4+ palavras) no mic |
|---|---|
| hoje (turbo, sem freios) | 17 |
| `large-v3` sozinho | 23 |
| `large-v3` + VAD no padrão (0.5) | **10** |
| `large-v3` + `-sns` + VAD 0.25 **sem ganho** | **9** |
| `large-v3` + `-sns` + VAD 0.25 **com ganho** | 23 |

**VAD no limiar padrão come fala.** Ele apagou a troca inteira sobre a armadura da laje —
*"você vai ter que colocar as barras na armadura de lágrima, né?"* virou três `Uhum.`

E VAD frouxo **sem ganho** é igualmente surdo. **O ganho é o que torna o VAD viável**; sozinho,
nenhum dos dois presta.

**Por que não parar no `large-v3` sozinho, que também deu 23?** Porque frase contada não é
frase limpa. O `large-v3` sozinho ainda repete (laço de 4), ainda inventa (*"Só porque eu sinto
muito"*) e erra o termo do ofício. A configuração com ganho e freios entrega as mesmas 23
frases **sem o lixo em volta** — e acerta a palavra que a §1.3 mostra.

### 1.3 O termo decisivo

Aos 330 s o dono fala de uma armadura. As configurações discordam na palavra, e o dono
confirmou que a certa é **laje**:

| configuração | o que entendeu |
|---|---|
| `large-v3` sozinho | armadura de **lágrima** |
| turbo + ganho + freios | armadura da **Live** |
| **`large-v3` + ganho + `-sns` + VAD 0.25** | armadura da **laje** ✓ |

É esta que vai para o código.

### 1.4 O vocabulário inicial reprovou

Testado, e o resultado foi destruição: `--prompt` com os termos do ofício produziu **82
repetições da mesma frase** e apagou o resto da reunião. Fica **fora** deste desenho. Não é um
botão que erra pouco — é um botão que, quando erra, apaga a reunião.

---

## 2. As decisões

### D-1 — O ganho mora no adapter de áudio, e é adaptativo

`crates/mos-audio/src/wav.rs` ganha `export_channel_normalized`, irmão do `export_channel`
existente. Ele mede o RMS do canal e **só aplica ganho se estiver abaixo de −32 dBFS**, mirando
**−25 dBFS**, com joelho suave (`tanh`) em vez de corte — e **teto de 20 dB** de ganho, para que
um canal quase mudo não vire um canal de chiado amplificado.

Os três números vêm da medição: o mic estava em −44 dBFS, o canal do sistema em −22 dBFS, e o
ganho que produziu a rodada aprovada levou o mic a −25,2 dBFS. O piso de −32 dBFS fica entre os
dois canais reais, com folga dos dois lados.

Adaptativo, e não ganho fixo, porque o canal do sistema chega em −22 dBFS: amplificá-lo não
ajudaria e clipar o que já está bom seria estragar o único canal que funciona.

Os dois chamadores (`meeting.rs` e `voice.rs`) passam a usar o novo — a nota de voz é o mesmo
microfone baixo. O `export_channel` cru **continua existindo e testado**, para quem um dia
precisar do áudio fiel.

**Os chunks no disco não mudam.** O ganho vive no WAV temporário que só o whisper vê. O áudio
guardado continua sendo o que o microfone captou.

### D-2 — Os freios são opinião do produto, e ficam no código

`crates/mos-transcribe/src/lib.rs`: a montagem da linha de comando, hoje imperativa dentro do
`transcribe`, vira **função pura `args(...)`** — é o que permite testá-la.

Acrescenta `-sns` e `--vad -vm <silero> -vt 0.25 -vp 250`. Os valores saem da medição da §1.2,
não de palpite.

Limiares finos (`-nth`, `-et`, `-lpt`) ficam no default **de propósito**: mexer em cinco botões
ao mesmo tempo é não medir nada. Se sobrar laço depois do colapso da D-3, aí sim.

### D-3 — O colapso de laço é regra de domínio

`crates/mos-core/src/meeting.rs`: `clean_segments` passa a colapsar sequências de **3 ou mais**
segmentos idênticos consecutivos em um só, guardando o início do primeiro e o fim do último.

**Três, e não dois**, porque "uhum, uhum" de verdade acontece numa ligação; 24 "Tchau"
seguidos, não.

Mora no domínio, ao lado do `is_speech` que já existe, pela razão que aquele registra: a regra
não pode depender de quem escreveu o adapter. E é a rede que pega o resto de laço que
**nenhuma** configuração do whisper matou — todas as onze rodadas deixaram de 3 a 10.

### D-4 — O caminho do VAD é configuração, e a falta dele degrada

`WhisperConfig` ganha `vadModel`. **Vazio significa VAD desligado, nunca erro**: uma máquina que
não baixou o Silero continua transcrevendo como antes. É a mesma disciplina do `binary` e do
`model`, e a razão da D-7 do MEETING-AGENT — a escolha de runtime é do usuário.

`MeetingSettings.tsx` ganha o campo. **Sem caixa de vocabulário** (§1.4).

### D-5 — O progresso passa a ser verdade

Hoje o provider reporta `0.0`, `0.9` e `1.0` por canal: uma barra construída sobre isso pularia
de nada para quase tudo.

O binário aceita `-pp` junto com `-np` e imprime `progress = NN%` no stderr — confirmado neste
build. O `transcribe` troca `output()` por `spawn()` e **lê o stderr enquanto o processo roda**,
guardando as últimas linhas não vazias para a mensagem de erro, que hoje sai daí.

Esta spec entrega **o número verdadeiro**. A barra na tela é da spec do redesenho de Reuniões.

### D-6 — O modelo do dono aponta para o `large-v3`

Mudança no `settings.json` da máquina, não no código. Custo medido, para 6 minutos: o mic sai
de 29 s para ~10 s (o VAD compensa a lentidão do modelo), o canal do sistema sai de 12 s para
~40 s.

---

## 2.5 A correção de rumo, escrita depois da verificação

Esta seção existe porque a verificação da Task 6 reprovou, e o que ela achou
derruba parte do que está escrito acima. Fica registrada em vez de silenciosamente
editada: quem ler a §1 precisa saber o quanto confiar nela.

### O que estava errado no meu próprio cálculo

A D-1 mandava mirar −25 dBFS, e o código mirava por regra de três (`alvo / rms`).
Mas o joelho `tanh` comprime os picos: mirando −25 dBFS, o áudio aterrissava em
**−26,5**. Corrigido — o ganho agora **mede o resultado e recalibra**, em até três
voltas, e o teto subiu de 20 para 24 dB porque o ponto medido exigia 10,7× e o teto
anterior cortava exatamente ele.

### O que estava errado nos critérios de aceitação

Muito pior, e é meu: **"23 frases e 'laje'" não é reproduzível.** Variando só o
ganho, com todo o resto idêntico:

| ganho | frases reais no mic | acertou "laje"? |
|---|---|---|
| 10,00 | 11 | não |
| 10,68 | 17 | não |
| 11,08 | 12 | não |
| 12,00 | 17 | não |
| 10,68, com truncamento em vez de arredondamento | **23** | **sim** |

As duas últimas linhas são **o mesmo ganho**, separadas por **1 LSB** de
arredondamento na amostra. O whisper é determinístico — a mesma entrada devolve a
mesma saída, verificado rodando duas vezes —, então isto não é sorteio: é
sensibilidade caótica à forma de onda num canal de baixa relação sinal-ruído.

**Consequência:** a rodada `r7` da §1.3 foi uma amostra de sorte, e não uma
propriedade da configuração. A escolha do `large-v3` sobre o turbo repousa sobre
evidência mais fraca do que a §1.3 sugere. Ela **não foi refeita** — fica registrada
como dívida, e o caminho para pagá-la é comparar distribuição sobre várias
perturbações, nunca amostra única.

### O que resistiu, e é o que sustenta o trabalho

| | antes | depois |
|---|---|---|
| Canal do sistema (a substância da reunião) | 62 frases, 721 palavras | **79 frases, 739 palavras** |
| Laço entre segmentos (24 "Tchau") | presente | **morto em toda configuração** |
| Repetição dentro de um segmento | 12× | **2×** |
| Canal do mic | 17 frases | 11–17 frases |

O canal que carrega o conteúdo melhorou, e o pior sintoma morreu. O canal do
microfone segue na mesma faixa — ele é seis minutos de muleta com pouca fala.

### A D-3 ganhou uma segunda metade

O colapso de segmentos não alcançava repetição **dentro** de um segmento, e ela
existia no áudio real. `colapsar_repeticao_interna` cobre isso, com o mesmo limiar
de três e pela mesma razão.

---

## 3. O que este desenho NÃO conserta

Precisa estar escrito, porque a próxima pessoa vai achar que consertou:

- *"Vamos olhar a **vida** de cima"* continua errado (era "laje");
- *"as **armas**/formas estão prontas"* segue instável entre rodadas;
- o remédio para isso seria vocabulário, e ele reprovou (§1.4).

O caminho real é gravar o microfone com nível decente **na origem**, e isso é outro projeto:
mexe na captura, não na transcrição.

---

## 4. Testes

Os três crates afetados são justamente os que **rodam nesta máquina** — o `mos-desktop` não
roda, por incompatibilidade de DLL registrada no `SETUP-MAQUINA.md` §4. Isso não é acaso: é o
critério que decidiu onde cada regra mora.

| crate | o que se verifica |
|---|---|
| `mos-audio` | canal baixo recebe ganho; canal já alto passa intocado; o joelho não clipa; os chunks não mudam |
| `mos-transcribe` | `args()` com e sem VAD; caminho vazio não gera flag; `-sns` sempre presente; parsing do `progress = NN%` |
| `mos-core` | 3 idênticos colapsam em 1; 2 sobrevivem; o intervalo do colapsado vai do início do primeiro ao fim do último |

**Verificação final:** reprocessar a mesma reunião pela bancada. O critério que se
sustenta é **ausência de laço** — entre segmentos e dentro deles — e o canal do
sistema não regredir. Contagem de frases do canal do mic e acerto de um termo
específico **não servem de critério**: a §2.5 mostra que ambos oscilam com 1 LSB de
diferença na entrada.

---

## 5. O que fica para depois

- **Barra de progresso na tela** — vai com o redesenho de Reuniões.
- **Argos como presença do Hermes** — spec própria; há conflito documentado a reconciliar
  (`argosPose.ts` diz que "a conexão não entra"), e a saída é cor carregar presença enquanto
  pose continua carregando fato.
- **Redesenho da tela de Reuniões** — navegação, palavras e estados de carregamento.
