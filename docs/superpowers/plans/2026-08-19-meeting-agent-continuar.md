# Meeting Agent — o que falta

**Data:** 2026-08-19
**Estado:** integrado no `master` (`7442e10`) e publicado. Fases 1 a 5 e a interface entregues.
**Leia antes:** `docs/MEETING-AGENT.md` (o contrato) e `docs/TECHNICAL-SPIKE-MEETING-AUDIO.md` (as medições).

---

## Onde a coisa parou

A cadeia inteira existe e compila:

```
Start → grava 2 canais → Stop → recupera de queda → transcreve local
      → analisa com o Hermes → item com evidência → Task + Reminder → desfazer
```

470 testes no Rust, 83 no renderer, clippy zerado, `tsc` limpo, build passando.

**Mas o app nunca foi aberto.** Escrevi ~800 linhas de CSS e JSX sem ver uma só
renderizada, e nenhuma reunião real passou pela cadeia. É daí que sai tudo
abaixo.

---

## 1. O primeiro passo de amanhã, e ele é curto

Antes de qualquer código:

```powershell
cd apps\desktop
npm run tauri dev
```

E então, em Settings → REUNIÕES, apontar o transcritor. O binário e o modelo
**não estão no repositório** — eles ficaram no scratchpad desta sessão, que é
temporário:

```
…\scratchpad\whisper\Release\whisper-cli.exe
…\scratchpad\whisper\ggml-large-v3-turbo-q5_0.bin   (547 MB)
```

**Provavelmente já sumiram.** Baixar de novo:

- binário: `whisper-bin-x64.zip` de https://github.com/ggml-org/whisper.cpp/releases (v1.9.2, 7,8 MB)
- modelo: `ggml-large-v3-turbo-q5_0.bin` de https://huggingface.co/ggerganov/whisper.cpp

Põe os dois num lugar estável (`C:\Dev\whisper\`, por exemplo) e aponta em
Settings. **Modelo multilíngue é obrigatório** — as variantes `.en` não servem
para português.

---

## 2. Gate F — a interface nunca foi vista

Nada disso foi exercitado, e é o `DESIGN-FOUNDATIONS.md` §16 inteiro:

- [ ] screenshot em `1280×800`, `1024×768` e `840×600`
- [ ] escalas do Windows a 100%, 125% e 150%
- [ ] tema claro, escuro e **High Contrast**
- [ ] navegação completa por teclado — em especial o diálogo de criar Task
- [ ] Narrator e árvore de UI Automation
- [ ] contraste programático dos pares novos
- [ ] estados: vazio, carregando, erro, gravando, sem transcrição
- [ ] `prefers-reduced-motion` (o ponto vermelho pulsa — ele para?)

O repositório tem uma skill para isto: `.claude/skills/ver-o-app/`, que veio do
master neste merge.

**Suspeitas minhas, para olhar primeiro:**

- a **barra de gravação** na topbar pode estar apertando o `page-meta` e o Argos;
- a **linha da lista** usa `grid-template-areas` com quatro campos; em 840px o
  título provavelmente estoura;
- o **diálogo** usa `.meeting-scrim` com `place-items: center` — em 600px de
  altura com o campo de data aberto, pode cortar o rodapé.

---

## 3. Gate G — nenhuma reunião real

O que precisa acontecer, na ordem:

- [ ] gravar 5 min com headset, falando sozinho → transcrever → conferir os
      timestamps contra o áudio
- [ ] gravar 15 min de reunião **de verdade** (Meet, Teams, o que for)
- [ ] conferir a separação VOCÊ / REMOTO na transcrição — é o que a V1 protege
      acima de tudo
- [ ] analisar com o Hermes e ver se os `my_action` são mesmo seus
- [ ] clicar numa evidência e conferir que ela cai na fala certa
- [ ] criar uma Task, ver o recibo, **desfazer** e conferir que a Task foi
      arquivada e o lembrete cancelado
- [ ] fechar a janela durante a gravação e conferir o tray (relógio + Parar)
- [ ] matar o processo pelo Gerenciador de Tarefas e conferir a recuperação

---

## 4. D-4 — a única pergunta técnica ainda aberta

**Desconexão de dispositivo no meio da gravação nunca foi exercitada.**

O código registra `Lost{at_ms}` quando a leitura falha, e a interface mostra
"Microfone caiu aos 32:10. O restante foi preservado." Mas ninguém arrancou um
headset para ver.

Teste: gravar, arrancar o headset aos ~2 min, continuar falando no mic da
webcam, parar. O esperado é que **o canal de sistema continue** e o mic apareça
como perdido. O que **não pode** acontecer é a barra continuar dizendo que grava.

Se a reconexão automática se mostrar viável, ela é um enhancement; a §20 já
autoriza falhar explicitamente na V1.

---

## 5. D-9 — 21 minutos por reunião de uma hora

Medido: `large-v3-turbo-q5_0` em CPU faz **5,6× tempo real** com 16 threads.
Uma reunião de 60 min gera 120 min de áudio (dois canais), ou seja **~21 min de
processamento**.

Aceitável? Só o uso responde. Se incomodar, o caminho está pronto e **não exige
instalar o CUDA Toolkit**: o release publica `whisper-cublas-12.4.0-bin-x64.zip`
(639 MB) que **empacota as DLLs do CUDA**. Trocar é trocar o caminho do binário
em Settings — não recompila nada. A RTX 5070 Ti deve derrubar isso para minutos.

---

## 6. Coisas menores, na ordem em que doem

- [ ] **A reunião não aparece na Search global.** O `SearchItem::Meeting` existe
      no domínio e o índice FTS é escrito, mas `search_all` do `WorkRepository`
      não junta reuniões ao resultado. Falta um `UNION` lá.
- [ ] **O widget da Home.** Não existe nenhum. "Compromissos de reuniões em
      aberto" tem query pronta (`meeting_open_commitments`) e seria o candidato
      natural — mas a ADR-034 manda widget só onde há dado, então ele só vale a
      pena depois de existirem reuniões de verdade.
- [ ] **Jarvis.** As seis "read tools" do brief não existem como ferramentas —
      a ADR-028 explica por quê (o gateway não registra ferramenta do lado do
      cliente). O caminho é injeção de contexto: falta o `@` da conversa do
      Hermes aceitar reunião como chip. Ver `MEETING-AGENT.md` §15.3.
- [ ] **O tempo faturável (D-D).** Você decidiu que a reunião *oferece* lançar o
      tempo como `ActivityType::Meeting`. Isso **não foi construído** — nem o
      comando nem o botão.
- [ ] **`Meeting` no Calendário.** O `CalendarKind::Meeting` existe no enum, mas
      `calendar::compose` não recebe reuniões. Uma linha de `ComposeInput`.
- [ ] **Limpeza de áudio.** `clean_expired_audio` roda na abertura e funciona,
      mas nunca foi vista apagando nada de verdade.

---

## 7. Duas coisas que eu deixaria quietas

**Não mexer na numeração de migration.** A de Meeting é a `0020` porque o master
chegou ao `0017` primeiro. Se um banco já rodou com ela, renumerar de novo
quebra bancos existentes de forma silenciosa.

**Não trocar o sidecar por `whisper-rs`** sem um motivo novo. A decisão está
justificada em `mos-transcribe/src/lib.rs`, e o custo dela é o `cargo build` de
todo mundo passar a compilar C++.

---

## 8. Onde as coisas estão

| O quê | Onde |
|---|---|
| O contrato e as decisões | `docs/MEETING-AGENT.md` |
| As medições do áudio | `docs/TECHNICAL-SPIKE-MEETING-AUDIO.md` |
| Por que Reuniões entrou no rail | `docs/DECISIONS.md`, ADR-044 |
| Domínio e máquina de estados | `crates/mos-core/src/meeting.rs` |
| O contrato `mos-meeting` | `crates/mos-core/src/meeting_analysis.rs` |
| Captura WASAPI | `crates/mos-audio/` |
| Transcrição | `crates/mos-transcribe/` |
| Comandos e laço | `apps/desktop/src-tauri/src/meeting.rs` |
| A interface | `apps/desktop/src/MeetingsPage.tsx`, `RecordingBar.tsx`, `MeetingSettings.tsx` |
| O spike descartável | `spikes/meeting-audio/` |

Testes que só rodam a mão:

```powershell
# a captura, contra hardware real
cargo test -p mos-audio --test hardware -- --ignored --nocapture

# a transcrição, contra um modelo real
$env:MOS_WHISPER_BIN="…\whisper-cli.exe"; $env:MOS_WHISPER_MODEL="…\ggml-…bin"
$env:MOS_WHISPER_WAV="…\mic-16k.wav"; $env:MOS_WHISPER_WAV2="…\system-16k.wav"
cargo test -p mos-transcribe --test real_model -- --ignored --nocapture

# a análise, contra o Hermes real (precisa do túnel aberto)
cargo test -p mos-desktop --lib gate_d -- --ignored --nocapture
```
