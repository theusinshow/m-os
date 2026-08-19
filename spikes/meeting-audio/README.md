# M/OS Meeting Agent — spike de áudio (Fase 1)

Experimento descartável. **Não é código de produto.**

Ele existe para responder, com número medido, as perguntas D-1 a D-5 de
`docs/MEETING-AGENT.md` §24, e para provar o **Gate A** antes de qualquer linha
de interface do Meeting Agent ser escrita. O resultado está registrado em
`docs/TECHNICAL-SPIKE-MEETING-AUDIO.md`.

## O que ele faz

```
START ──┬── microfone            (dispositivo de captura, WASAPI shared)
        ├── áudio do sistema     (dispositivo de SAÍDA + AUDCLNT_STREAMFLAGS_LOOPBACK)
        └── keep-alive           (stream de render escrevendo silêncio)
              ↓
        chunks de PCM cru, 10 s, um arquivo por chunk, por canal
              ↓
STOP ──► report.json com o veredito calculado
```

Os dois canais **nunca** são misturados: `mic/` é o usuário local e `system/` são
os participantes remotos, e é essa separação que sustenta a distinção entre
"o que EU prometi" e "o que outros disseram".

## Comandos

```powershell
cargo build --release
cargo test                 # 12 testes; a matemática roda sem tocar o Windows

.\target\release\meeting-audio-spike.exe devices
.\target\release\meeting-audio-spike.exe record --secs 900 --out .\sessao
.\target\release\meeting-audio-spike.exe inspect .\sessao
```

### Os experimentos

| Comando | Pergunta |
|---|---|
| `record --timing polling` | D-1: o modo por evento é mesmo melhor? |
| `record --no-keepalive --system-only --system-device "<saída ociosa>"` | D-2: o loopback para no silêncio? |
| `record --no-autoconvert` | D-3: quanto custa o formato nativo? |
| matar o processo e depois `inspect` | Gate B: quanto sobra de uma queda? |

`--system-device` importa: numa máquina com áudio virtual (Elgato, Voicemod,
Steam), a saída padrão tem um driver que roda continuamente e **esconde** o
buraco do loopback. O teste da D-2 só é válido contra um endpoint ocioso de
verdade.

## Como ouvir o que foi gravado

Os chunks são PCM cru — sem cabeçalho, de propósito (`MEETING-AGENT.md` §8.2).
O `ffmpeg` monta um WAV a partir deles:

```powershell
cmd /c "copy /b .\sessao\mic\*.pcm .\mic.pcm"
ffmpeg -f s16le -ar 16000 -ac 1 -i .\mic.pcm .\mic.wav
```

## O que ele deliberadamente não faz

Sem interface, sem banco, sem transcrição, sem Hermes, sem estado de Meeting.
Um spike que crescesse para esses lados deixaria de responder rápido a pergunta
que ele existe para responder — e o `AGENTS.md` é explícito sobre não converter
experimento em escopo.
