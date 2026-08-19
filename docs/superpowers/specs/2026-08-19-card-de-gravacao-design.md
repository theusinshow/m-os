# O card de gravação — anotações, onda e pausa — Design

**Status:** aprovado para plano de implementação

**Data:** 2026-08-19

**Baseline:** M/OS `v0.3.0` no commit `f6fe650`. Meeting Agent integrado em `bafbfb5`, com a máquina de dez estados em `crates/mos-core/src/meeting.rs`, a análise em `meeting_analysis.rs`, a captura em `crates/mos-audio/`, e a barra de gravação em `apps/desktop/src/RecordingBar.tsx`.

**Origem:** três capturas de tela do Notion trazidas pelo proprietário — o card de "Anotações IA" com abas, onda sonora ao vivo, Pausar e Parar. Da referência foi adotada a **estrutura** e recusado o que não tem função aqui (§8).

**Revisa:** o Non-Goal *"Realtime: transcrição, ações ou assistência ao vivo"* continua de pé — ver §4. O comentário do `RecordingBar.tsx` que proíbe waveform é revisado por §5.

## 1. Objetivo

Dar à reunião em curso um lugar de trabalho — onde se anota enquanto se ouve — em vez de apenas um indicador de que o microfone está aberto.

## 2. Escopo

**Dentro:** o card na página Reuniões, com abas Anotações e Transcrição; a coluna `notes` e o autosave; as notas subindo ao Hermes; a onda ao vivo e o evento que a alimenta; o estado `Paused` e as suas transições; e o encolhimento da barra da topbar.

**Fora, e decidido:**

- **detecção de reunião e widget de sobreposição** — é a frente A, e ela exige decidir a fronteira da ADR-037 antes de qualquer código; um Meet no Chrome é `chrome.exe`, e distingui-lo exige ler título de janela, que é atravessar a fronteira que aquela ADR desenhou;
- **transcrição ao vivo** — §4;
- **seletor de formato e o ícone de ajustes** das imagens — §8;
- **notas com formatação** — texto puro nesta versão; o M/OS não tem editor rico em lugar nenhum e introduzir um aqui seria a maior peça da spec pela menor razão.

## 3. O card

Vive no topo do painel de detalhe da página Reuniões, e só enquanto a reunião está em `recording` ou `paused`.

```
┌────────────────────────────────────────────────────────────┐
│  Reunião de 19/08 16:16                        ✎           │
│                                                            │
│  [ Anotações ] [ Transcrição ]   ▍▍▎▍▊▍▎  Pausar   Parar    │
│  ─────────────────────────────────────────────────────────  │
│                                                            │
│  o que você digitar aqui sobe junto com a transcrição       │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

A aba **Transcrição**, durante a gravação, não fica vazia nem promete o que não vai cumprir: ela diz que o texto chega ao parar, e diz por quê (§4). Depois de `transcribed` ela mostra a transcrição com os segmentos, que é o que a página já faz hoje.

## 4. Por que a transcrição não é ao vivo

O `MEETING-AGENT.md` §3 lista *"Realtime: transcrição, ações ou assistência ao vivo"* como Non-Goal, com a razão: *"conflita com pós-reunião confiável, e a regra de decisão do brief manda escolher confiabilidade"*.

A premissa mudou em parte — o áudio já é gravado em pedaços de 10 s e a máquina agora tem uma RTX 3060 que transcreveu 22 s instantaneamente. **Barato deixou de ser o obstáculo.** O obstáculo que resta é de qualidade, e ele não mudou: transcrever pedaços independentes corta palavras na emenda e perde o contexto que o modelo usa para desambiguar. Uma transcrição feita de uma vez, ao final, é melhor — e a reunião inteira é o caso de uso, não o instante.

Decisão do proprietário, tomada com esse trade-off na mesa: **fica como está.** A aba explica em vez de esconder.

## 5. A onda sonora

O `RecordingBar.tsx` diz hoje, em comentário: *"Sem waveform, sem medidor grande, sem cockpit. Um nível discreto é permitido porque responde a uma pergunta real — 'está me ouvindo?' — e qualquer coisa maior seria o showcase que o desenho proíbe."*

**Esta spec reverte isso, e só para o card.** O argumento que sustenta a reversão é o mesmo que sustentava a proibição: a pergunta que a forma responde. Na topbar, onde a barra acompanha você por telas que não são sobre a reunião, oito degraus bastam. No card — que só existe enquanto a reunião está aberta na sua frente, e onde você está anotando de cabeça baixa — a onda responde uma pergunta diferente e real: *"o outro lado parou de falar, ou eu perdi o áudio?"*. Movimento contínuo distingue silêncio de queda; oito degraus parados, não.

**E o medidor sai da topbar** (§9), então não há duas leituras do mesmo dado na mesma tela: a onda não convive com os oito degraus, ela os substitui e muda de lugar.

**O que isso custa, e é honesto dizer:** fora da página Reuniões você deixa de ter a resposta fina para *"está me ouvindo agora?"*. O que **não** se perde é a resposta grossa: a barra já acende borda de perigo quando os dois canais somem (`data-warning`), então "perdi o áudio" continua sendo dito em qualquer tela. Some o medidor contínuo, não o alarme.

**O dado.** O `meeting-tick` de 1 Hz continua carregando estado, duração e saúde dos canais. Nasce um evento novo, `meeting-level`, a **15 Hz — um a cada 66 ms** —, carregando **dois números**: mic e sistema, RMS em milésimos, como o tick já faz. Emitir o tick inteiro a 15 Hz mandaria quinze vezes por segundo um objeto que muda uma vez por segundo.

Trinta barras a 15 Hz mostram dois segundos de história, rolando. Com `prefers-reduced-motion` a onda **não some**: ela para de rolar e vira um medidor discreto de oito degraus, no mesmo lugar. A informação é necessária; só o movimento é opcional.

## 6. As anotações

Coluna `notes TEXT NOT NULL DEFAULT ''` na tabela `meetings`, migration 0022. Autosave com debounce; sem botão de salvar, porque um botão de salvar numa nota de reunião é uma chance de perder o que se escreveu.

### 6.1 Como elas chegam ao Hermes, e por que não viram itens sozinhas

As notas sobem **como contexto**, num bloco próprio antes das janelas de transcrição. Elas informam o resumo e a interpretação. **Elas não geram itens.**

Isso não é economia: é a regra de evidência. O prompt hoje exige que *"todo item precisa de pelo menos um `segment`"*, e avisa que *"um id que não está acima faz a evidência ser descartada, e o item perde o direito de virar Task num clique"*. Uma nota que você digitou não tem segmento — ela não foi dita, foi escrita. Deixar o modelo criar itens a partir dela obrigaria a uma de duas coisas, e as duas são piores:

1. **inventar evidência** — o item citaria um segmento que não o sustenta, e a âncora que dá confiança ao "aceitar num clique" deixaria de significar algo;
2. **permitir item sem evidência** — e aí a validação que hoje recusa lixo do modelo teria um buraco por onde tudo passa.

E há a saída simples: o que você digitou **já é seu texto**. Se é uma Task, o caminho é o Quick Capture ou o botão de Task, não uma volta pelo modelo para ele devolver o que você mesmo escreveu.

**O que muda no prompt:** um bloco `NOTAS DE QUEM GRAVOU` antes da transcrição, com a instrução explícita de que ele é contexto e não fonte de item — e que itens continuam exigindo `segment`. A validação em `parse_analysis` não muda, e é isso que garante que a instrução seja cumprida mesmo se o modelo a ignorar.

## 7. Pausar

Estado novo, `Paused`, entre `Recording` e `Stopping`. Duas transições: `Pause` e `Resume`.

```
  Recording ──Pause──►  Paused
  Paused    ──Resume─►  Recording
  Paused    ──Stop───►  Stopping
```

**A linha do tempo cuida de si.** O `durationMs` da Meeting é, por decisão registrada no tipo, *"medida em frames gravados, nunca por diferença de relógio"*, e o alinhamento entre os canais vem do keep-alive de silêncio — a Fase 1 mediu 2.498 pacotes com ele contra **zero** sem ele. Pausado, os dois canais param de escrever juntos; o tempo pausado simplesmente não vira frame, e portanto não vira duração. Não há matemática nova, e não há vão para reconstruir.

**O que o keep-alive exige:** ele para junto. Deixá-lo escrevendo silêncio durante a pausa faria o canal SYSTEM acumular frames que o MIC não tem, e aí sim a linha do tempo torceria — que é exatamente a falha de 4710 ms que o spike mediu, chegando pelo outro lado.

**A honestidade visual não é detalhe.** Pausado, o ponto vermelho **para de pulsar** e a barra passa a dizer `PAUSADO`. Um ponto pulsando com o microfone fechado é a mentira exata que a §17.2 existe para impedir — e ela é pior que não ter indicação nenhuma, porque ensina a confiar num sinal falso.

## 8. O que a referência deu, e o que foi recusado

**Adotado:** a estrutura de card com abas, a onda como sinal de vida do áudio, e o par Pausar/Parar.

**Recusado: o seletor de "Formato" e o ícone de ajustes.** No Notion eles existem porque a saída dele é um documento com estilos a escolher. Aqui o formato da saída é o contrato de análise — `summary`, `topics`, `items` com evidência —, e ele não é escolha de tela. Trazer os controles seria copiar a forma sem a função, que é o §16 do `UX-PRINCIPLES` na definição.

**Recusado: o rótulo "IA".** O botão do Notion diz "Iniciar Anotações IA". Aqui o que a pessoa inicia é uma **gravação**; a análise vem depois, por botão separado, e com consentimento próprio. Chamar a gravação de "IA" prometeria na hora errada.

## 9. A barra da topbar encolhe

Ponto, relógio e `PARAR` — mais `PAUSADO` quando for o caso. Os oito degraus de nível e o estado dos canais mudam de casa para o card, onde há espaço para eles significarem algo. O `data-warning` que acende a borda quando os dois canais somem **fica**, porque é alarme e não medida (§5).

`PARAR` **continua na barra**, e isso é deliberado: parar uma gravação não pode exigir navegar até Reuniões. A §17.2 promete indicação em qualquer tela; parar de qualquer tela é a consequência prática dessa promessa.

## 10. Verificação

**Nó:** a máquina de estados ganha testes para `Pause`, `Resume` e `Stop` a partir de `Paused`, e para as transições que devem ser **recusadas** — `Pause` sobre uma reunião já parada, `Resume` sobre uma que nunca pausou. A janela da onda (trinta amostras, descarte da mais velha) é função pura e é testada como tal.

**Rust:** a coluna `notes` no repositório, com o caso que a 0021 ensinou a olhar — leitura de reunião antiga, gravada antes da coluna existir, devolvendo string vazia e não erro. E o teste de que o keep-alive para junto com a pausa.

**Gate visual, pela skill `ver-o-app`:** card em 1280 e 840, nos dois temas, nos três estados — gravando, pausado e depois de parar. Teclado: as abas são navegáveis, o campo de notas recebe foco, e `Parar` é alcançável sem mouse. E `prefers-reduced-motion` conferido de fato, com a onda caindo para os oito degraus.

**O que este desenho não consegue verificar:** se anotar durante a reunião é algo que você realmente faz. A resposta não vem de teste — vem de uma reunião de verdade, e ela ainda não aconteceu nem uma vez.
