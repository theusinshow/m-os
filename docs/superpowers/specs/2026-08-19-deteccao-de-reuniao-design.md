# A detecção de reunião — o microfone como sinal — Design

**Status:** aprovado para plano de implementação

**Data:** 2026-08-19

**Baseline:** M/OS `v0.3.0` no commit `ba94772`. Observação de processos em `apps/desktop/src-tauri/src/monitor.rs` (ADR-037), janela do lembrete em `tauri.conf.json` e `Reminder.tsx`, card de gravação entregue em `ba94772`.

**Origem:** três capturas do Notion trazidas pelo proprietário, e a pergunta dele — *"como o Notion faz?"* — que mudou a decisão. Ver §2.

**Revisa:** ADR-037, por meio da ADR-046.

## 1. Objetivo

Oferecer a gravação no momento em que uma reunião começa, sem que a pessoa precise lembrar de abrir o M/OS.

## 2. Por que microfone, e não título de janela

A escolha inicial do proprietário foi **título de janela**, tomada a partir de uma tabela que eu apresentei — e na qual eu classifiquei "microfone em uso" como a opção mais cara. **Estava errado**, e a pergunta dele desfez o erro.

A conta oficial do Notion é explícita sobre o mecanismo deles:

> "On desktop, Notion can detect that a meeting app is active and show this prompt to start AI Meeting Notes. **It doesn't read your browser content** or listen to audio unless you actively start notes."

O que eles observam é **qual processo está com o microfone aberto**. Comparado com título de janela:

| | título de janela | microfone em uso |
|---|---|---|
| o que expõe | **conteúdo** — nome de documento, de aba, de página | qual processo abriu o microfone |
| cobertura | exige lista: Meet, Zoom, Teams, e o que se esquecer | qualquer app, inclusive os não previstos |
| falso positivo | aba aberta sem reunião dispara | só dispara com microfone ativo |
| fronteira da ADR-037 | move para conteúdo | move para capacidade |

O microfone ganha nos dois eixos: **pega mais e conta menos**. E há uma terceira razão, que é a melhor: ele detecta o **fato certo**. Uma aba do Meet aberta não é uma reunião; um microfone aberto é.

## 3. Onde o Windows guarda isso

No registro, somente leitura, sem hook e sem injeção. Verificado nesta máquina:

```
HKCU\...\CapabilityAccessManager\ConsentStore\microphone\
    <PackageFamilyName>              ← apps da Store (WhatsApp, Copilot)
    NonPackaged\<caminho do exe>     ← apps Win32, com "\" trocado por "#"
```

Cada entrada tem `LastUsedTimeStart` e `LastUsedTimeStop`, em FILETIME. **`LastUsedTimeStop == 0` significa em uso agora.**

Os **dois** caminhos precisam ser lidos: o Chrome é `NonPackaged`, mas Teams e WhatsApp podem ser empacotados. Ler só um deixaria buracos que se pareceriam com "não funciona às vezes".

O crate `winreg` já está na árvore de dependências, por caminho transitivo.

## 4. As três exclusões, que são o que impede o ridículo

**O próprio M/OS sai da conta.** `mos-desktop.exe` está nessa lista — confirmado nesta máquina —, e quando ele grava, ele abre o microfone. Um detector ingênuo se veria gravando e ofereceria gravar de novo.

**Não oferece se já há gravação em curso**, mesmo que o microfone aberto seja de outro app: quem já está gravando não precisa da oferta.

**Não oferece para processo silenciado**, reusando o `suppress` que o `monitor.rs` já tem, com o mesmo "não neste app" que o lembrete usa.

## 5. O atraso de 20 segundos

O microfone precisa estar aberto por **20 segundos contínuos** antes de a oferta aparecer.

Não é conservadorismo: um microfone que abre por dois segundos é teste de som, atalho de push-to-talk, notificação do sistema. Reunião mantém aberto. Sem o atraso o popup vira ruído — e popup ruidoso é desligado no primeiro dia, o que custa a feature inteira em troca de nada.

O contador zera quando o microfone fecha. Uma reunião que cai e volta oferece de novo, e isso é desejado: pode ser outra reunião.

**Quando mais de um processo está com o microfone aberto** — e isso é comum, porque o Discord fica aberto ao lado do Meet —, o alvo é o que está aberto **há mais tempo**. Não é sorteio: o que abriu primeiro é o que provavelmente é a reunião, e o que abriu depois costuma ser o acessório. O alvo importa por um motivo só, e é concreto: `Não neste app` precisa saber qual app silenciar. A oferta em si não muda — ela inicia uma gravação do sistema inteiro, e não daquele processo.

## 6. A janela

Janela **nova**, e não a do lembrete.

As duas têm a mesma configuração — `transparent`, `decorations: false`, `alwaysOnTop`, `skipTaskbar`, `focus: false`, e `shadow: false`, que foi a correção da borda branca em `ddec664`. Mas compartilhar uma janela faria lembrete e detecção disputarem o mesmo espaço no pior momento possível: **durante uma reunião**. Custa uma entrada no `tauri.conf.json` e um ramo no `switch` de `App()`.

**O que ela oferece:**

| ação | o que faz |
|---|---|
| `Gravar reunião` | inicia a gravação e some |
| `Agora não` | some, e volta a oferecer se o microfone fechar e abrir de novo |
| `Não neste app` | silencia aquele processo, pelo mesmo caminho do lembrete |

Ela **não** diz "IA". O que se inicia é uma **gravação**; a análise vem depois, por botão separado e com consentimento próprio. O botão do Notion diz "Iniciar Anotações IA" e promete na hora errada.

## 7. A ADR-046, e o que ela admite

A fronteira da ADR-037 vai de *"nomes de programa, e nada além disso"* para *"nomes de programa, e qual programa está com o microfone aberto"*. **Ligada de fábrica**, com toggle em Settings → REUNIÕES.

A ADR precisa dizer o que a feature **não** faz, porque é isso que mantém a fronteira estreita: **não lê título de janela, não lê conteúdo de aba, não escuta o áudio.** Saber que o Chrome abriu o microfone não diz com quem se fala nem sobre o quê.

**O custo, dito com todas as letras:** o M/OS passa a observar uma coisa nova sem que ninguém tenha pedido. A ADR-037 desenhou a fronteira justamente para que atravessá-la fosse difícil e visível; ligar de fábrica atravessa **com aviso, e não com pedido**. Foi decisão do proprietário, tomada com esse trade-off na mesa, e o argumento a favor é o do Notion: uma feature que exige ser descoberta para servir não serve a quem não a descobre.

O toggle é a mitigação, e ele precisa ser **fácil de achar** — não enterrado em Avançado.

## 8. Escopo

**Dentro:** a leitura dos dois caminhos do ConsentStore; a integração no laço do `monitor.rs`; o atraso, as exclusões e o silenciamento; a janela nova e as três ações; o toggle; a ADR-046.

**Fora, e decidido:**

- **título de janela e qualquer leitura de conteúdo** — §2 e §7;
- **iniciar a gravação sozinho** — continua sendo Non-Goal: `ADR-037` diz *"observação não vira hora sozinha"*, e o mesmo vale para gravação. A janela **oferece**;
- **detectar de qual app é a reunião para nomear a Meeting** — exigiria o título, que é o que a §2 recusa. O nome continua sendo `Reuniao de <data> <hora>`, editável;
- **macOS e Linux** — o ConsentStore é do Windows; `ADR-001` já limita a plataforma.

## 9. Verificação

**Nó:** a decisão de oferecer é função pura e é testada como tal — dado um conjunto de processos com microfone aberto, há quanto tempo cada um está aberto, quais estão silenciados, se o M/OS grava, e se o toggle está ligado, ela devolve *oferecer para X* ou *não oferecer*. Os casos: exclusão do próprio M/OS, gravação em curso, processo silenciado, microfone aberto há menos de 20 s, e o caminho feliz.

**Rust:** a leitura do registro é testada contra uma estrutura montada em memória, e não contra o registro real — o teste não pode depender de qual app está com o microfone aberto na máquina de quem roda.

**Na máquina, e este é o gate:** abrir um Meet no Chrome e confirmar que a janela aparece **depois** dos 20 segundos e não antes; confirmar que ela **não** aparece durante uma gravação do próprio M/OS; e conferir os três botões, incluindo que "Não neste app" realmente silencia.

**O que este desenho não consegue verificar:** se a oferta chega no momento certo do ponto de vista de quem está entrando na reunião — se 20 segundos é cedo demais, tarde demais, ou irritante. Isso só uma semana de uso responde, e a ADR deve dizer o que fazer se a resposta for "irritante".
