# Meeting Agent V2 — "Você participa da reunião. O M/OS cuida do resto." — Design

**Status:** aprovado pelo proprietário por delegação explícita ("autonomia completa")

**Data:** 2026-09-16

**Baseline:** M/OS `v0.6.0`, commit `197a41e`. Domínio em `crates/mos-core/src/meeting.rs`,
contrato em `meeting_analysis.rs`, repositório em `mos-storage-sqlite/src/meeting_repository.rs`,
casca em `apps/desktop/src-tauri/src/meeting.rs`, tela em `apps/desktop/src/MeetingsPage.tsx`.

**Revisa:** `MEETING-AGENT.md` §9.2 (recuperação exige decisão), §17.2 (sem atalho global),
§22 (superfícies), D-C (prazo só como Reminder — já superada pela ADR-066), e a ADR-047
(a detecção passa a acompanhar também o FIM).

---

## 0. A auditoria, em uma tabela

O que a V1 entrega é sólido — captura, recuperação, transcrição local, análise com evidência.
O que a torna cansativa não é falta de feature. É isto:

| Achado | Onde | Consequência |
|---|---|---|
| Transcrever e analisar só por botão | `MeetingActions` | três cliques e duas esperas por reunião; "às vezes não inicia" = ninguém clicou, ou clicou e o transcritor não estava pronto |
| Nenhuma fila, nenhum retry | `meeting_transcribe` sobe uma thread e esquece | Hermes fora do ar → `failed` e fim; queda do app no meio → reunião presa em `transcribing` para sempre (§9.3 foi documentado e não implementado) |
| **`paused` não cabe no CHECK da 0020** | `meetings_status_known` | Pausar grava um estado que o banco recusa: o botão falha. Ninguém tinha apertado (memória de 19/08) |
| **`capturing_meetings` ignora `paused`** | repositório | uma queda com a reunião pausada nunca é reconciliada |
| WAV temporário vaza em falha | `transcribe_channels` só apaga no sucesso | 115 MB por hora por canal ficam em `%TEMP%` |
| Nada percebe o fim da reunião | — | gravação esquecida roda por horas |
| Apagar é só definitivo | `meeting_delete` | sem lixeira, sem desfazer; a pasta pode ficar órfã se a remoção falhar |
| Prazo é texto e a Task nasce sem `due_at` | `AcceptDialog` | a ADR-066 trouxe `due_at`; a reunião não usa |
| Um item por vez | `AcceptDialog` | revisar seis ações são seis diálogos |

---

## 1. Modelo mental: Reunião → Processando → Pronta

O enum de dez estados **fica**. Ele é a verdade técnica, e está testado. O que muda é que a pessoa
não o vê mais: uma função pura `fase_da_reuniao(meeting, job, contexto)` traduz para a fase visível.

| Fase | Quando |
|---|---|
| `recording` | `recording`, `paused` |
| `finalizing` | `stopping` |
| `processing` | `recorded`/`transcribing`/`analyzing`, ou `transcribed` com análise na fila |
| `ready` | `ready` |
| `partially_ready` | `transcribed` sem análise possível (sem consentimento, ou análise esgotou as tentativas) |
| `recovered` | `interrupted` (só enquanto não começou a processar sozinho) |
| `needs_attention` | falha que não se resolve sozinha: transcritor não configurado, áudio vazio |
| `failed_recoverable` | `failed(transcription)` com tentativas esgotadas — o áudio está seguro |

Ortogonal a isso, `pendencias` conta itens propostos acionáveis: uma reunião `ready` com três ações
por revisar aparece como "pronta · 3 ações", e é isso que a Home e a Atenção leem.

---

## 2. Processamento automático e persistente

### 2.1 A tabela de jobs

`meeting_jobs`, uma linha por reunião (a reunião tem **um** pipeline, não uma fila de pedidos):

```text
meeting_id PK · stage (transcription|analysis) · status (queued|running|waiting_retry|
needs_attention|done|cancelled) · attempt_count · progress (0..1) · started_at · finished_at ·
last_error_code · last_error_message · next_retry_at · created_at · updated_at
```

`last_error_message` é a frase para a pessoa e **nunca** contém fala (mesma regra de §16.3).

### 2.2 A política é pura

`mos-core/src/meeting_pipeline.rs`:

- `classificar_falha(codigo) -> Transitoria | Configuracao | Permanente`;
- `proxima_tentativa(stage, attempts, classe, now) -> Retry(at) | PedirAtencao`;
- escada: **30 s, 2 min, 10 min, 30 min, 2 h**; transcrição desiste depois de 4, análise
  depois de 6 (o Hermes cair por uma tarde é normal, whisper falhar quatro vezes não é);
- `Configuracao` (transcritor ausente) não queima tentativa: fica `needs_attention` e volta sozinha
  quando `ready()` passar — conferido a cada volta do laço;
- um job `running` num processo recém-nascido é órfão: volta a `queued`, e a reunião volta ao
  repouso (`transcribing → recorded`, `analyzing → transcribed`). Isto é a §9.3, finalmente.

### 2.3 O worker

Um laço (`meeting::run_pipeline`) acorda a cada 5 s **ou** por `Notify` (parar a gravação,
retry manual, consentimento dado). Um job por vez — o whisper usa todos os núcleos, e duas
transcrições simultâneas não terminam antes de uma. Parar a gravação enfileira transcrição;
transcrição concluída enfileira análise se houver consentimento; análise concluída marca `done`.
A tela não participa de nada disso.

**Transitória não vira `failed`.** A reunião volta ao repouso e o job espera. Só esgotar vira
`failed(stage)` — e mesmo aí o retry manual existe.

### 2.4 Parcial nunca inutiliza

Transcrição pronta e Hermes fora = `partially_ready`: resumo ausente, transcrição completa,
itens escritos nas notas (§6) já presentes, e a frase "Organização inteligente pendente — tenta
de novo sozinho". Nenhum botão obrigatório.

### 2.5 Progresso verdadeiro

A fração continua medida, nunca inventada (`processamento.ts`). Os passos:

```text
✓ Gravação salva          (stopping → recorded)
✓ Áudio preparado         (WAV exportado; medido)
● Transcrevendo  48%      (fração do whisper, dois canais = 0..0,5 e 0,5..1)
○ Organizando             (janelas: "parte 2 de 3"; sem %)
○ Pronta
```

A porcentagem global pondera **transcrição 70 %, análise 30 %** — o peso vem do tempo medido
(21 min de whisper contra ~1–2 min de Hermes por reunião de uma hora), e é declarado no código.

### 2.6 Recuperação sem pedir decisão

`interrupted` com áudio **é processada sozinha**. A §9.2 exigia decisão porque processar era
caro e manual; processar não apaga nada, e a política de retenção continua valendo. `Descartar`
continua disponível. `interrupted` sem nenhum frame vira `needs_attention` ("nada foi gravado"),
e nunca é apagada sozinha.

`stop_reason = crash_recovery` registra a origem.

---

## 3. Recording Guardian

### 3.1 O problema

A pessoa fecha o Teams e esquece o M/OS gravando. O Guardian percebe que a reunião
**provavelmente** acabou e pergunta — ou, se autorizado, encerra com contagem regressiva.

### 3.2 Os sinais, e o que cada um custa em privacidade

Nenhum sinal lê conteúdo. Todos já estão dentro das fronteiras das ADR-037 e ADR-047.

| Sinal | Fonte | Peso |
|---|---|---|
| app associado deixou de usar o microfone | `ConsentStore` (`LastUsedTimeStop != 0`) | forte |
| processo do app associado encerrado | `sysinfo`, nome de executável | forte (só Win32; pacote da Store = desconhecido) |
| áudio remoto ausente | nível RMS do loopback, **nunca o conteúdo** | médio |
| voz local ausente | nível RMS do microfone | médio |
| tela bloqueada | `LogonUI.exe` rodando | fraco, acumulativo |
| sem teclado/mouse | `GetLastInputInfo` (já usado pelo monitor) | fraco |
| duração anormal | relógio | só amplifica |
| fim do evento do Calendar | **não existe** — `Event` não existe (§0.3); a entrada fica prevista e sempre vazia | — |

**O app associado** é o que tinha o microfone aberto quando a gravação começou (o alvo da oferta,
ou o aberto há mais tempo, excluindo o próprio M/OS). Se nenhum havia, o Guardian adota o primeiro
que abrir nos primeiros 5 minutos. Um Meet no Chrome é `chrome.exe`: o sinal é "o Chrome
fechou o microfone", o que é exatamente o fato certo **sem ler a aba** (§53 do pedido).

### 3.3 Atividade não é silêncio

Um segundo "ativo" por canal é nível acima de `piso × 3 + 2‰`, com o piso estimado por EMA que
cai rápido e sobe devagar. O microfone desta casa fala a −44 dBFS (≈6‰): limiar fixo apagaria
a pessoa. O loopback em silêncio é zero digital (keep-alive), então qualquer som conta.

O leitor não amostra o instantâneo — ele lê o **máximo desde a última leitura** (`window_max_milli`,
um atômico novo em `mos-audio` trocado por zero a cada leitura). Amostrar 1 Hz cairia nas pausas.

### 3.4 A heurística

Pura, em `mos-core/src/meeting_guardian.rs`. Entrada: `Observacao` por segundo. Estado:
`EstadoDoGuardian` (serializável). Saída: `Veredito`.

```text
confiança = 0
app associado sem microfone há ≥ 90 s ........................ +0,45
  … e processo encerrado ...................................... +0,20
remoto sem atividade há ≥ 90 s ............................... +0,15   (≥ 5 min: +0,25)
local sem atividade há ≥ 90 s ................................ +0,10   (≥ 5 min: +0,20)
tela bloqueada ou sem input há ≥ 5 min ....................... +0,10
duração ≥ limite longo (2 h) e sem atividade há ≥ 10 min ..... +0,05
```

**Vetos**, que zeram tudo:

1. atividade remota ou local nos últimos 45 s **enquanto o app associado ainda tem o microfone**;
2. o app associado readquiriu o microfone depois da suspeita (falso positivo desfeito sozinho);
3. gravação pausada.

Com o app já sem microfone há ≥ 10 min, o som remoto **não** veta: é música, vídeo, outra coisa.

**Sem app associado** (reunião presencial, gravação manual sem chamada): só silêncio dos dois
canais conta. ≥ 20 min → perguntar; ≥ 45 min + bloqueado/ausente ≥ 30 min → confiança alta
(`auto_inactivity`). Silêncio sozinho nunca passa de "perguntar" antes disso.

| Confiança | Veredito |
|---|---|
| < 0,55 | nada (ou `Suspeita` interna, sem UI) |
| ≥ 0,55 | `Perguntar` — "Parece que sua reunião terminou." |
| ≥ 0,80 **e** auto-stop ligado **e** sinal de app (ou regra de inatividade longa) | `ContagemRegressiva` de 20 s |

### 3.5 Cooldown, e por que ele escala

"Continuar gravando" silencia por **15 min**, depois 30, depois 60. Um sinal **novo e forte**
fura o cooldown: o app associado liberar o microfone **depois** do clique (ele o tinha readquirido),
ou o processo fechar depois do clique. Silêncio não fura.

Uma pergunta ignorada (nem encerrar, nem continuar) não se repete: ela fica visível na barra e no
tray até a situação mudar. Popup repetido é o que faz desligar a feature.

### 3.6 Reunião longa

Aos 2 h (e a cada hora depois), **com** sinal de inatividade (≥ 10 min sem atividade nos dois
canais, ou app sem microfone): "Esta reunião está sendo gravada há 2h37. Ela ainda está
acontecendo?" Sem inatividade, nada — reunião longa e ativa não é interrompida.

### 3.7 Bloqueio de tela

Bloqueio é sinal acumulativo, não gatilho. No desbloqueio, se a gravação continuou e o Guardian
tem `fim_provavel`, a pergunta vem com a oferta de corte: "Encerrar e ignorar o que veio depois de
15:42?".

### 3.8 `fim_provavel`

O instante em que a atividade da reunião provavelmente acabou: o momento em que o app associado
liberou o microfone, ou a última atividade antes disso — o que vier depois — mais 15 s de margem.
Sem app: a última atividade de qualquer canal.

### 3.9 Superfícies

- **Barra da topbar** — "Parece que terminou · [Encerrar] [Continuar]"; na contagem, "Encerrando
  em 18 s · [Continuar gravando] [Encerrar agora]".
- **Janela de sobreposição** — a mesma `reuniao-detectada` (a oferta nunca coexiste com uma
  gravação: `decidir_oferta` recusa quando grava). Não rouba foco.
- **Tray** — o relógio vira "● Reunião · 34:12 · terminou?".

### 3.10 `stop_reason`

`manual | auto_meeting_ended | auto_inactivity | crash_recovery | device_failure | app_exit`.
Gravado na reunião; aparece só no painel técnico.

### 3.11 Métricas

`meeting_guardian_events(meeting_id, at, kind, confidence, excess_ms)`:
`suggested | continued | stopped_from_prompt | countdown_started | countdown_cancelled |
auto_stopped | long_prompted | trim_suggested | trim_applied | trim_reverted`. Local, sem conteúdo.

---

## 4. Corte (trim)

### 4.1 Não destrutivo

`meetings.trim_start_ms`, `trim_end_ms`, `trim_origin (manual|auto|suggested)`. Os chunks ficam.
A transcrição exporta só a faixa (`export_channel_range_normalized`), e os segmentos continuam em
tempo **absoluto** — a evidência `14:04` não muda de significado.

### 4.2 Três caminhos

1. **Automático, antes de processar.** Parada com `fim_provavel` e excesso ≥ 10 min **e** sinal de
   app (confiança alta): o fim é cortado logicamente, a retenção sobe para no mínimo 24 h (para o
   desfazer ter áudio), e a página diz "Ignoramos 31 min depois do fim da reunião · [Incluir]".
2. **Sugerido, depois de processar.** Sem sinal de app: se a última fala termina ≥ 10 min antes do
   fim da gravação, "Detectamos 31 minutos sem conversa no fim. Ignorar?". Aplicar só remove
   segmentos (quase sempre nenhum) e não retranscreve.
3. **Manual.** "Ajustar início e fim" — duas alças sobre a duração, sem editor de áudio.

### 4.3 Reprocessar só se necessário

- faixa nova ⊆ faixa antiga, com transcrição: remove segmentos fora; se algum item perdeu toda a
  evidência, reenfileira análise;
- faixa nova maior e áudio existe: reenfileira transcrição;
- faixa maior e áudio apagado: recusado, com a frase dizendo por quê.

---

## 5. Apagar

### 5.1 Lixeira

`lifecycle_state = 'trashed'` já existe no CHECK. Apagar → lixeira, com recibo e **Desfazer**
(ADR-035). Lixeira fica 30 dias e depois vira exclusão definitiva, na abertura. A política está
escrita no próprio diálogo.

- **Gravando:** "Esta reunião ainda está sendo gravada. [Encerrar e apagar] [Cancelar]".
- **Processando:** o job vira `cancelled` antes; o worker confere o lifecycle antes de gravar
  resultado, então um whisper que termina depois não ressuscita nada.
- **Tasks e Reminders criados ficam.** O diálogo diz isso.

### 5.2 Definitiva

Banco (reunião, segmentos, análise, itens, evidência, bookmarks, jobs, eventos do Guardian,
índices de busca) numa transação; depois o disco. Antes do banco, um **tombstone**
(`meetings/<id>/.apagar`) é escrito. Na abertura, diretório com tombstone e sem linha é removido;
diretório **sem** tombstone e sem linha continua sendo só relatado (§9.2 — nunca apagar o que
ninguém mandou apagar). Pastas `mos-meeting-*` em `%TEMP%` são derivadas e são limpas.

---

## 6. Notas, marcadores e momentos

- **Marcar momento** (⭐): `meeting_bookmarks(id, meeting_id, at_ms, created_at)`. Sobe ao Hermes
  como "momentos que a pessoa marcou: 00:14:05" — sinal de relevância, nunca fato.
- **Marcadores nas notas**, lidos de forma determinística na parada (não precisam do Hermes):
  `!task texto`, `!decision texto`, `!question texto`, com `@pessoa` e `#projeto` na mesma linha.
  Viram itens com `origin = written`. **Escrito é fato de quem escreveu**: não precisa de evidência
  para entrar no lote, e a interface diz "escrito por você".
- `origin` distingue `spoken` (Hermes com evidência), `written` (marcador) e `manual` (criado da
  transcrição pela pessoa).

---

## 7. Análise V2

### 7.1 Contrato

O bloco `mos-meeting` ganha `"version": 2`, `title`, `project` (nome ou código, validado contra
a lista real) e três `kind` novos: `commitment` (compromisso de outra pessoa com você),
`dependency`, `reference`. `other_action` continua aceito e é lido como `commitment`. `topic`
continua fora de `items`.

`MeetingAnalysisV2` é a **projeção** dos itens por kind — `userActions = my_action`,
`externalCommitments = commitment|other_action`, etc. Não é segunda tabela.

### 7.2 O domínio valida

- `project` inexistente → descartado; existente → sugestão (§7.4).
- título > 80 caracteres, vazio, ou igual ao padrão → descartado.
- deduplicação: mesmo kind + texto normalizado (sem acento, caixa, pontuação, stopwords) dentro
  da análise vira um item com evidências somadas.
- decisão com linguagem hipotética ("talvez", "acho que", "poderia", "quem sabe", "vamos ver")
  cai para `open_question` com confiança baixa — brainstorm não vira decisão.
- ação sem evidência continua fora do lote (salvo `written`).
- responsável: `my_action` cuja evidência é toda do canal REMOTO e sem "eu" na fala perde para
  `medium`; o canal é sinal, não sentença.

### 7.3 Prazos naturais

`resolver_prazo(expressao, referencia_local) -> Option<PrazoResolvido { expressao_original, em,
confianca }>`, pt-BR, determinístico: `hoje`, `amanhã`, `depois de amanhã`, dias da semana (com
"que vem"/"próxima"), `fim de semana`, `semana que vem`, `fim do mês`, `dia 20`, `20/09`,
`até segunda`, `em N dias`, `daqui a N semanas`. Referência = início da reunião, no fuso local.
Hora padrão 18:00 (vence no fim do expediente). Persistido em `meeting_insights.due_expression`,
`due_at`, `due_confidence`. Expressão que não resolve fica só como texto.

### 7.4 Project e título automáticos

Pontuação determinística em `inferir_project`: código tipo `167-25` citado e existente (alta),
`#projeto` nas notas (alta), cronômetro ativo do CronoCAD num Project durante a gravação (alta),
nome citado ≥ 3 vezes e único (média), sugestão do Hermes validada (média). **Só alta associa
sozinha**; média aparece como "Parece ser do Project X · [Associar]".

Título: o do Hermes substitui o automático **só** enquanto o título ainda é o padrão do relógio.
Renomeado pela pessoa, nunca mais é tocado.

### 7.5 Vocabulário

Termos manuais (Settings) + nomes e códigos de Projects. Usos:

1. entram no prompt do Hermes como glossário;
2. **transcrição normalizada**: `meeting_segments.text_normalized` guarda a versão com termos
   corrigidos por semelhança forte (sem acento e caixa, distância ≤ 1 para termos ≥ 6 letras,
   ≤ 2 para ≥ 10, nunca para palavra comum do português). `text` cru nunca muda. Correção
   incerta é marcada e aparece como **[Criciúma?]**;
3. **nunca** vai ao whisper: `--prompt` com vocabulário produziu 82 repetições em 20/08.

---

## 8. Do item à Task

- **Revisão em lote**: alta vem marcada, média desmarcada com "revisar", baixa não tem caixa.
  "Criar 2 tarefas" cria tudo numa transação, com um recibo e um desfazer.
- **`Task.due_at`** recebe o prazo resolvido (editável na linha). Reminder é opcional.
- **Aguardando**: `commitment`/`other_action` aceito vira Task com `waiting_for = owner`, que é o
  Waiting For que o piloto já cobra.
- **Deduplicação**: Task ativa com o mesmo título normalizado no mesmo Project, criada nos últimos
  14 dias, faz o item ser **ligado** a ela em vez de criar outra.

---

## 9. Superfícies

- **Página da reunião**: título · data · duração · Project; progresso quando processando; **O que
  exige sua atenção** (lote); Decisões; Aguardando outras pessoas; Perguntas em aberto; Riscos;
  Tópicos; Notas; Transcrição. `•••`: Renomear · Ajustar início e fim · Preparar follow-up ·
  Perguntar ao Hermes · Arquivar · Apagar reunião.
- **Lista**: Em andamento · Precisa de atenção · Recentes, busca, filtro, menu de contexto.
- **Transcrição**: busca, filtros (Você/Remoto/Marcados/Com itens), copiar trecho, ouvir trecho,
  marcar momento, criar Task, marcar decisão.
- **Ouvir**: o renderer nunca vê path. `meeting_clip(id, start_ms, canal)` devolve um WAV curto
  (≤ 30 s) em base64; `Ambos | Você | Remoto`.
- **Follow-up**: texto montado sem IA a partir de resumo, decisões e ações — copiar, nunca enviar.
- **Perguntar ao Hermes**: abre uma conversa nova com a reunião anexada (ContextEntity::Meeting
  já existe).
- **Barra global**: `● Reunião · 34:12 · [⭐] [Pausar] [Encerrar]` + Guardian.
- **Tray**: Marcar momento, Pausar/Retomar, Encerrar.
- **Command**: Iniciar reunião, Abrir reunião atual, Marcar momento, Pausar, Retomar, Encerrar.
- **Atalhos**: `Ctrl+Alt+M` — inicia, ou marca momento se já grava; `Ctrl+Alt+Shift+M` — encerra.
  Registrados com a mesma checagem de colisão dos outros. No ABNT2 `AltGr+M` não produz caractere.
- **Home / Atenção**: o piloto ganha `MeetingReview` ("Reunião pronta · 3 ações · [Revisar]") e
  `MeetingNeedsAttention`. A Home já é o lugar do "precisa de você"; a Inbox de Captures continua
  sendo de Captures.
- **Notificação**: "Reunião pronta — 3 ações · 2 decisões" quando a janela não está em primeiro
  plano.
- **Settings → Reuniões**: Gravação (detectar, teste de áudio), Automação (processar sozinho),
  Proteção contra gravação esquecida (três caixas), Privacidade (retenção padrão), Vocabulário,
  Transcritor (recolhido em "Avançado").
- **Painel técnico**: só em `import.meta.env.DEV`.

---

## 10. O que fica de fora, e por quê

| Fora | Por quê |
|---|---|
| Sync das reuniões | `sync_cobertura.rs` as classifica como locais porque a linha aponta para áudio. Sincronizar o **derivado** (título, resumo, itens) é possível, mas exige geração 5 da cobertura, projeção e backfill — um projeto próprio. As Tasks criadas já sincronizam, e é isso que chega ao celular |
| Participantes | não há entidade Pessoa nem `Event`; `owner` continua texto |
| Fim do Calendar como sinal | `Event` não existe; a entrada do Guardian fica prevista e vazia |
| Transcrição progressiva durante a gravação | spec 19/08 §4: corta palavras na emenda. Mantido |
| Iniciar gravação sozinho | consentimento — a detecção oferece, a pessoa clica |
| iOS | o domínio (pipeline, guardian, prazo, dedupe) é puro e serve; loopback é Windows |
