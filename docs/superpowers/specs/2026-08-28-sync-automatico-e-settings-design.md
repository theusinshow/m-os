# O sync deixa de ser um clique, e o Settings ganha um mapa — Design

**Status:** aprovado, aguardando plano

**Data:** 2026-08-28

**Baseline:** M/OS `v0.3.1` no commit `8b46735`, que fechou a sessão em que o
sync ganhou servidor, transporte e uma superfície de bolso.

**Origem:** pedido do dono do produto, no PC do trabalho. O fluxo real é
**PC de casa → PC do trabalho → celular**, e hoje o elo do meio é manual: o
desktop só sincroniza quando alguém clica.

---

## 1. A assimetria que criou o problema

A sessão anterior entregou o hub, o transporte HTTP e o `mos-web`. Ela entregou
também uma diferença de comportamento que ninguém escolheu:

| Aparelho | Quando sincroniza |
| --- | --- |
| `mos-web` (celular) | sozinho — no fundo e a cada escrita |
| desktop | **só no clique** |

O `SyncSettings` registrou a decisão por escrito, e ela estava certa na época:

> O botao de sincronizar existe porque hoje a rodada e MANUAL, e dizer isso na
> tela e mais honesto que um automatico que ninguem pediu.

Agora alguém pediu. E o desenho automático já estava especificado desde antes —
o `SYNC.md` §8 lista os gatilhos (abertura, primeiro plano, reconexão de rede,
pós-mutação com *debounce*, oportunidade de fundo, sinal de push, refresh
manual) e proíbe *polling* agressivo (§51). Nada aqui é invenção; é a
implementação do que o documento já dizia.

### 1.1 O que a máquina do trabalho mostrou

Antes de desenhar, o estado real deste PC foi conferido:

- `%APPDATA%\com.codedbym.mos\settings.json` **não tem chave de sync** — sem
  endereço de hub;
- não existe tarefa agendada `M-OS Sync Tunnel`; só a do Hermes está rodando.

Ou seja: **neste PC o sync nunca foi ligado.** Isso muda o desenho de um jeito
concreto — "desligado" não é um erro nem um caso de borda, é o estado em que a
feature nasce. Uma faixa que só sabe desenhar sucesso e falha estaria errada no
primeiro dia.

O PC de casa está do outro lado do mesmo problema: o commit `5263209` registrou
**370 operações na fila** esperando a primeira rodada.

---

## 2. As decisões, e o que foi recusado

| Decisão | O que foi recusado, e por quê |
| --- | --- |
| Sincroniza sozinho **na abertura e no fundo** | Um botão a clicar toda manhã ainda é sincronizar à mão, uma vez por dia. E não resolve a metade que importa: o que você faz às 15h precisa ter saído quando você levanta da mesa |
| A faixa aparece **só quando tem o que dizer** | Faixa fixa dizendo "tudo certo" todo dia vira móvel — some da percepção justamente antes do dia em que ela teria algo a dizer |
| "Primeira vez do dia" = **primeira abertura do dia civil** | Mesma régua da Daily Session. Não se inventa um segundo conceito de dia dentro do mesmo app |
| O laço mora **no backend**, com um sinal vindo da tela | No frontend, sync viraria refém de a webview estar viva e da página certa estar montada. O `SYNC.md` trata sync como motor, não como tela |

---

## 3. O motor

### 3.1 Onde ele mora

Uma tarefa de fundo criada no `setup` do Tauri, ao lado dos comandos que já
existem em `apps/desktop/src-tauri/src/sync.rs`. Ela acorda por quatro sinais:

| Sinal | Quando | De onde vem |
| --- | --- | --- |
| abertura | uma vez, **depois** que o app terminou de abrir | o mesmo portão que o `abertura.ts` já espera |
| primeiro plano | a janela voltou | `window-revealed`, que **já existe** e hoje só move o foco do input |
| pós-mutação | a fila cresceu, com *debounce* de ~10s | um `Notify` disparado por quem enfileira |
| rede de segurança | a cada 15 min, se nada mais acordou | intervalo longo. Não é *polling*: é o que garante que uma reconexão silenciosa não fique esperando para sempre |

### 3.2 A primeira rodada espera o app abrir

Não é otimização; é a diferença entre abrir e parecer travado.

`sincronizar_agora` segura o mutex do storage durante a rodada inteira, **de
propósito** — o commit `5263209` explica que soltar no meio faria uma mutação
local emitir um instante que o motor já passou, quebrando a ordem total de que a
reconciliação depende.

Com a fila deste PC (centenas de operações, nunca subidas), uma rodada disparada
junto com a abertura seguraria o banco por vários segundos enquanto a webview
faz IPC. O `abertura.ts` gastaria as 12 tentativas dele contra um banco ocupado,
e o sintoma seria **a tela de erro que a sessão passada acabou de consertar** —
com uma causa nova e uma mensagem que continuaria mentindo.

Portanto, e com precisão — porque "espera o app abrir" é vago o bastante para ser
implementado errado:

1. A tarefa é criada no `setup`, **depois** de `app.manage(AppState)`. Antes
   disso ela nem existe, e o portão do §2430 do `lib.rs` já recusaria qualquer
   comando dela.
2. `manage()` não basta. Ele diz que o estado existe, não que a webview terminou
   a rajada de IPC do boot. A tarefa espera um segundo sinal: um comando
   `sync_app_pronto`, que o frontend chama uma vez quando o `bootState` vira
   `ready`.
3. **Com teto.** Se esse sinal nunca chegar — janela que não abriu, boot que
   falhou noutro ponto —, a tarefa roda mesmo assim depois de 30s. Um sync que
   depende da tela para existir é um sync que morre em silêncio quando a tela
   morre, e o M/OS abre minimizado na bandeja por padrão de configuração.

### 3.3 Uma rodada por vez, e a manual não perde

Um mutex **de rodada**, separado do mutex do banco. Duas rodadas concorrentes
brigariam pelo mesmo `HlcClock`, que é exatamente o que o commit do botão
descreveu não poder acontecer.

Clicar "Sincronizar agora" durante uma rodada automática **espera e mostra o
resultado daquela rodada** — não enfileira uma segunda nem devolve erro. Do
ponto de vista de quem clicou, o clique funcionou; e funcionou mesmo.

### 3.4 O que a rodada passa a contar

`SyncRound` hoje tem `sent`, `received`, `conflicts`, `pending`, `error`. Isso
basta para uma linha de status e **não basta para a faixa**: "6 mudanças
chegaram" é vago o bastante para não se confiar.

`SyncRound` ganha `receivedByKind: { task: 3, capture: 1, ... }`.
`sync_projecao.rs` já conhece o tipo de cada operação que aplica — é somar num
mapa no caminho que já existe.

`EntityKind` é texto e não enum fechado (`SYNC.md` §9), então o mapa é
`HashMap<String, usize>` e um tipo desconhecido conta sem quebrar. Rótulo que a
tela não conhece aparece pelo id, e não some.

---

## 4. Os seis estados

O `SYNC.md` §10 lista cinco estados que a interface precisa representar:
sincronizado, sincronizando, offline, alterações pendentes, erro. Falta um, e é
o estado em que a feature nasce nesta máquina.

| Estado | O que aconteceu | O que a Home faz |
| --- | --- | --- |
| **desligado** | sem endereço ou sem segredo | **não aparece na Home.** Vive no Settings, dizendo como ligar |
| **sincronizando** | rodada em curso | **depende de haver faixa.** Se já havia uma (erro, pendente, ou o clique em "Tentar agora"), ela vira o símbolo girando. Se não havia, a Home não muda — só o cabeçalho |
| **em dia** | rodada limpa, fila vazia | **some da Home.** O horário fica no cabeçalho |
| **chegou coisa** | a rodada trouxe algo, e é a primeira do dia | **a faixa**: resumo por tipo. Some quando dispensada |
| **pendente** | a fila não esvaziou (sem rede, hub fora, túnel caído) | faixa discreta: o número e "Tentar agora" |
| **erro** | a rodada parou | faixa com o motivo cru e o botão |

A linha de *sincronizando* é a que mais fácil sairia errada: uma rodada de fundo
a cada 15 minutos que pisca uma faixa na Home seria pior que o problema que esta
spec resolve. **Rodada silenciosa não abre faixa** — ela só troca a cara de uma
faixa que já estava lá. O cabeçalho já tem a região `SINCRONIZANDO`, e é ela que
carrega o estado calmo.

**A regra que amarra os seis:** *erro* e *pendente* **não se dispensam** — somem
quando a causa some. O resumo do dia se dispensa, porque já foi lido. Um aviso
que se pode calar sem consertar a causa é um aviso que se cala sempre.

**"desligado" não vira faixa** por escolha: quem não ligou o sync não tem um
problema, tem uma feature desligada. Transformar isso em aviso diário na Home
seria propaganda dentro do próprio produto.

---

## 5. A faixa

### 5.1 A exceção, e por que ela é uma

O `App.tsx` registra o princípio da Home ao apresentar o widget do dia:

> tudo que mora na Home do M/OS e um widget arrumavel, e uma excecao seria a
> unica coisa da tela que nao se pode mover nem esconder.

A faixa **é** uma exceção, e ela precisa ser defendida em vez de escondida. A
defesa: aquele princípio protege a Home de ter um morador permanente que não se
pode arrumar. A faixa não é moradora — ela **só existe quando tem notícia, e
some quando é lida**. Não compete por espaço com os widgets porque, na maioria
dos dias, não ocupa espaço nenhum.

Isso vai escrito no código, ao lado da razão que ele contradiz. Uma exceção sem
justificativa vira precedente, e o próximo a querer um card fixo na Home vai
apontar para esta.

**A recusa que acompanha:** o widget arrumável foi considerado e descartado.
Um widget se esconde — e um widget de sync escondido é um sync quebrado que
ninguém descobre. O estado que precisa te alcançar não pode morar num lugar que
se pode calar.

### 5.2 Onde ela fica

Entre o `ContextPath` e o `CaptureComposer`, dentro da `HomePage`. É a primeira
coisa que se lê ao chegar, no único dia em que ela existe. Empurrar o compositor
para baixo nesse dia é aceitável: é o dia em que a notícia vale mais que a
captura.

### 5.3 O que ela diz

```
┌──────────────────────────────────────────────────────────────┐
│ CHEGOU ENQUANTO VOCÊ ESTAVA FORA                             │
│ 3 tasks · 1 capture · 2 objetivos do dia                     │
│ há 2 minutos                                    [Dispensar]  │
└──────────────────────────────────────────────────────────────┘
```

```
┌──────────────────────────────────────────────────────────────┐
│ 47 MUDANÇAS ESPERANDO                                        │
│ Não alcancei o hub — o túnel não está de pé.  [Tentar agora] │
└──────────────────────────────────────────────────────────────┘
```

O segundo é o que **este PC** mostraria hoje, e por isso ele importa tanto
quanto o primeiro.

A faixa **não nomeia o aparelho de origem**. A operação carrega o dispositivo que
a emitiu, mas dizer "do celular" exigiria um catálogo de nomes de aparelho que
não existe, e "de `a3f2-…`" é pior que não dizer nada.

### 5.4 Quem lembra que já foi lido

O backend, não a tela. Como estado do React, sair da Home e voltar traria a
faixa de novo no mesmo dia.

`settings.json` guarda `sync.ultimoResumoEm: "2026-08-28"`. Quando uma rodada
termina e essa data é diferente de hoje, o resumo fica marcado como não lido;
dispensar marca lido. **É isto que implementa "primeira abertura do dia civil"** —
não há um segundo relógio, e a decisão mora onde a rodada acontece.

### 5.5 O que dá para testar

Não há teste de DOM neste repositório, por decisão registrada no
`vitest.config.ts`. A consequência prática é a mesma de sempre: o que decide
alguma coisa vira função pura.

`syncFaixa.ts`, no molde de `homeLayout.ts` e `daily.ts`: recebe `SyncStatus`, o
último `SyncRound` e se o resumo do dia está por ler, e devolve qual dos seis
estados desenhar e a frase. Os componentes só desenham o resultado.

Casos que o teste fixa: os seis estados; *erro* e *pendente* ignorando o pedido
de dispensa; um `EntityKind` desconhecido aparecendo em vez de sumir; e resumo
com zero recebidos **não** virando faixa.

---

## 6. O Settings

### 6.1 O reagrupamento

De cinco seções para sete, com nomes que descrevem o que está dentro:

| Nova seção | O que entra | Por quê |
| --- | --- | --- |
| **Sincronização** | `SyncSettings` + os controles do automático | Sai de dentro de "Conexão e aparência". Virou feature de verdade, e é a primeira porque é a que se visita |
| **Conexões** | Hermes, Univirtus, ponte do M-Finance | O que fala com fora, sem o tema no meio |
| **Aparência e entrada** | Tema, Captura rápida, Atalho da voz, Atalhos | O tema encontra a casa dele |
| **Início e atualizações** | `StartupSettings`, Atualizações | Abrir com o Windows e atualizar são o mesmo assunto: o ciclo de vida do app |
| **Reuniões** | `MeetingSettings` | Mantém, mas com `settings-section-title` como as outras — hoje é a única com `micro-label`, e por isso parece menor do que é |
| **Dados** | Portabilidade, Archive e Trash, Integridade, Diagnóstico | Como está |
| **Avançado** | Functions, CronoCAD | Como está |

O nome velho da primeira seção — "Conexão e aparência" — é o diagnóstico
inteiro: ele junta Hermes, Univirtus, sync, a ponte do M-Finance **e o tema
claro**, e o "e" no meio do título é a confissão de que nunca houve um critério.

### 6.2 A navegação

Índice fixo à esquerda da coluna de conteúdo, com as sete seções. Clique salta;
a seção visível fica marcada. **Não é o rail do app** — é navegação de página,
dentro da página, e o rail continua sendo o que troca de página.

### 6.3 O código, como meio e não como fim

A reorganização não foi pedida como refactor, mas passa por um: a página inteira
é hoje **uma linha de JSX** de dezenas de milhares de caracteres, dentro de um
`App.tsx` de 4017 linhas. Não se reagrupa o que não se consegue ler, e não se
revisa um diff que cabe numa linha só.

- `SettingsPage.tsx` sai do `App.tsx`, levando os `*Settings` que só ela usa;
- `settingsNav.ts` — o catálogo das seções, puro e testável, exatamente como
  `HOME_SECTIONS` mora no `homeLayout.ts`;
- **nada muda na tela por causa disso.** É reagrupamento, não redesenho.

---

## 7. O que não é código

### 7.1 Ligar este PC

Sem endereço, sem segredo e sem túnel, o automático não tem o que automatizar.
O passo usa o que a sessão passada já escreveu: `scripts/install-sync-tunnel.ps1`
para a tarefa agendada, e o `deploy/README.md` §5 para o resto.

A tarefa precisa de uma chave SSH **sem passphrase** — ela roda no logon, sem
ninguém olhando. O `sync-tunnel.ps1` procura `hermes_work`, `id_ed25519` e
`hermes_home`, nessa ordem, e pula qualquer uma marcada `ENCRYPTED`.

### 7.2 O `SYNC.md` está desatualizado

O §14 ainda afirma:

> **Transporte real e servidor.** O `Transport` está definido; nenhuma
> implementação de rede existe.

Isso deixou de ser verdade em 26/08. O §8 lista os gatilhos como plano; eles
passam a ser descrição. Os dois são corrigidos junto com esta mudança — um
documento que descreve um sistema que não existe mais é pior que nenhum, porque
é lido com confiança.

---

## 8. O que fica de fora, e por quê

- **Sinal de push do hub para o desktop.** O `SYNC.md` §8 lista, e o `mos-web` já
  tem Web Push. No desktop exigiria conexão persistente com o hub; o intervalo de
  15 min mais o gatilho de primeiro plano cobrem o fluxo real sem isso.
- **Reconexão de rede como gatilho.** Windows expõe o evento; ele foi deixado
  fora porque a rede de segurança de 15 min já o cobre com atraso aceitável, e
  cada gatilho novo é uma forma nova de a rodada rodar quando não devia.
- **Sincronizar Calendar, Meetings, Conversations, Tracking e Voice.** Cinco
  entidades ainda não emitem (`SYNC.md` §14). Automatizar a rodada não muda o que
  ela carrega, e a faixa vai contar corretamente o que existe.
- **Os arquivos dos Resources.** Só o metadado viaja (§44). Continua assim.

---

## 9. A ordem, e as duas metades

Isto é uma spec com dois assuntos, e eles não dependem um do outro. Se algum
precisar ser cortado, o corte é limpo nesta linha:

1. **Ligar este PC (§7.1).** Primeiro, e sem código. Sem isto nada do resto é
   verificável nesta máquina — o automático rodaria contra um endereço vazio, e
   a única faixa que eu conseguiria fotografar seria a de "desligado".
2. **O motor e a faixa (§3, §4, §5).** É o que foi pedido.
3. **O Settings (§6).** Independente. A seção de Sincronização nova assume os
   controles do passo 2, então vem depois — mas o reagrupamento e a navegação
   valem sozinhos, e não bloqueiam nada.
4. **Os documentos (§7.2).** Junto do passo 2, não depois: o `SYNC.md` §8 deixa
   de descrever um plano no exato commit em que o plano vira código.
