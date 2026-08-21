# PLATFORMS — O M/OS é multi-device por definição

> A partir de 2026-08-21, **não existe mais "feature do desktop que talvez seja
> portada depois"**. Existe feature do M/OS, com manifestações apropriadas em
> cada plataforma. Este documento é o contrato dessa afirmação.

---

## 1. O que a auditoria encontrou

Antes de decidir qualquer coisa, o repositório foi medido. O resultado mudou o
plano, e vale registrar porque ele é a razão de esta migração **não** ser uma
reescrita:

**O cérebro do M/OS já era portátil, e ninguém tinha reparado.**

```
crates/mos-core/Cargo.toml  →  serde, serde_json, thiserror, time, uuid
```

Cinco dependências, nenhuma de plataforma. Sem Windows, sem Tauri, sem SQLite,
sem rede. O domínio inteiro — Captures, Tasks, Projects, Resources, Calendar,
Reminders, Meetings, Conversations, Tracking — mais 12 *ports* (traits de
repositório) vivem num crate que compila em qualquer lugar que o Rust compile.

A arquitetura já era hexagonal:

```
mos-core            domínio + ports          portável
mos-storage-sqlite  adapter (rusqlite)       portável (bundled-full compila o SQLite junto)
mos-hermes          adapter (reqwest/rustls) portável
mos-audio           WASAPI / windows-sys     WINDOWS APENAS
apps/desktop        shell Tauri + React      Windows hoje
```

O acoplamento indevido que a missão mandou procurar **não está no domínio**.
Ele está concentrado em três lugares, e todos no shell:

| Onde | O que prende ao Windows |
| --- | --- |
| `crates/mos-audio` | WASAPI via `windows-sys` — captura de mic e loopback |
| `src-tauri/src/monitor.rs` | Registro do Windows, processos, `ConsentStore` |
| `src-tauri/src/microfone.rs` | `ConsentStore` do Windows |

E o front-end é fino: `api.ts` são 184 `invoke` sobre 844 linhas. A lógica de
negócio está em Rust, não em React. **Não há `taskService.ts` para duplicar** —
o §61 da missão já estava satisfeito por construção.

**Consequência:** a estratégia não é "extrair o core". O core já está extraído.
É *plugar um segundo shell nele*.

---

## 2. A decisão de plataforma

**Tauri 2 para iOS.** Ver `DECISIONS.md`, ADR-052.

Verificado na documentação oficial (v2.tauri.app, agosto de 2026):

- Alvos Rust para iOS existem e são suportados: `aarch64-apple-ios`,
  `x86_64-apple-ios`, `aarch64-apple-ios-sim`.
- Plugins oficiais com iOS: biometria, deep link, haptics, câmera/barcode,
  notificações locais, stronghold (armazenamento cifrado).
- **Sem plugin oficial para push remoto (APNs) nem para Share Sheet.** Os dois
  são requisitos da missão (§15, §20) e vão precisar de plugin próprio ou
  comunidade. Está escrito aqui para não virar surpresa na Fase 7.

### O bloqueio que não dá para contornar

> **Compilar para iOS exige macOS com Xcode.** É restrição da Apple, não do
> Tauri. A máquina de desenvolvimento atual é Windows 11.

Nenhuma quantidade de arquitetura resolve isso. Toda a Fase 3 em diante
(build iOS, simulador, TestFlight, App Store) está **bloqueada por hardware**,
e não por decisão técnica. O que se pode fazer no Windows é tudo que este
documento e a Fase 1 cobrem: domínio, sync, contratos, capabilities, migrations
e testes — que é justamente a parte que, se ficar errada, custa caro depois.

---

## 3. A arquitetura alvo

```
                          M/OS
                            │
        ┌───────────────────┴───────────────────┐
        │              PORTÁVEL                 │
        │  mos-core     domínio + ports         │
        │  mos-sync     relógio, ops, merge     │
        │  mos-storage-sqlite  adapter          │
        │  mos-hermes   adapter do agente       │
        │  packages/design-system  tokens       │
        └───────────────────┬───────────────────┘
                            │
              ┌─────────────┴─────────────┐
              │                           │
        apps/desktop                 apps/ios
        Tauri + React                Tauri + React
        WASAPI, registro,            Keychain, haptics,
        atalho global, tray          share, push, câmera
              │                           │
              └────────── sync ───────────┘
```

Regras que sustentam o desenho:

1. **Nada de plataforma entra em `mos-core` ou `mos-sync`.** Se um `#[cfg(windows)]`
   aparecer nesses dois crates, o desenho quebrou.
2. **Capacidade específica fica atrás de porta.** Ver §5 abaixo.
3. **UI pode divergir; domínio não.** `TaskDesktopView` e `TaskMobileView` são
   legítimos; dois `task_service` não são.

---

## 4. Matriz de features

Estado real em 2026-08-21. `—` significa que não existe; **não** significa
"pronto e não testado".

| Feature | Core | Desktop | iOS | Sync | Hermes |
| --- | --- | --- | --- | --- | --- |
| Captures / Inbox | ✓ | ✓ | — | **emite** | ✓ |
| Tasks | ✓ | ✓ | — | **emite** | ✓ |
| Projects | ✓ | ✓ | — | **emite** | ✓ |
| Resources / Library | ✓ | ✓ | — | **emite** (metadado) | ✓ |
| Workspaces | ✓ | ✓ | — | fundação | — |
| Search (FTS local) | ✓ | ✓ | — | fundação | ✓ |
| Calendar | ✓ | ✓ | — | fundação | ✓ |
| Reminders / Attention | ✓ | ✓ | — | **emite** (intenção) | ✓ |
| CronoCAD (tempo) | ✓ | ✓ | — | fundação | parcial |
| Meetings (gravar) | ✓ | ✓ | — | fundação | ✓ |
| Meetings (transcrever) | ✓ | ✓ | — | fundação | — |
| Meetings (analisar) | ✓ | não provado | — | fundação | ✓ |
| Hermes (conversa) | ✓ | ✓ | — | fundação | ✓ |
| Hermes (ações) | ✓ | ✓ | — | fundação | ✓ |
| Universal Drop Zone | ✓ | ✓ | adaptado | fundação | ✓ |
| Voice Inbox | ✓ | ✓ | — | fundação | ✓ |
| Monitoramento de app | — | ✓ | impossível | n/a | — |
| Atalho global | — | ✓ | impossível | n/a | — |
| Share Sheet | — | n/a | — | n/a | — |
| Push remoto | — | — | — | — | — |
| Knowledge Graph (relações) | ✓ | ✓ | — | **emite** | — |
| Identidade de dispositivo | ✓ | ✓ | — | ✓ | — |

**"fundação"** quer dizer: as tabelas, o relógio, as operações, a reconciliação
e o motor de sincronização existem e estão testados — inclusive contra dois
bancos reais fazendo papel de dois dispositivos. O que falta nessas linhas é a
entidade *emitir* operações quando muda.

**"emite"** quer dizer que a entidade já registra cada mutação na fila de saída,
**dentro da mesma transação da mudança**. Cinco já emitem: Captures, Tasks,
Projects, Reminders e Resources.

Os dois parênteses são limites reais, não pendências disfarçadas. Reminder
emite **a intenção** — título, gatilho, prazo, status — e não a entrega: o
`deliveredCount` conta quantas vezes *este* aparelho tocou, e o iPhone tocar não
significa que o PC tocou. Resource emite **o metadado**; o arquivo em si é uma
camada separada, com upload, download, cache e checksum, e ela não existe.

As relações do Knowledge Graph também emitem, como entidade de primeira classe
com id derivado do par — ver `SYNC.md` §13.

Faltam sete: Calendar, Meetings, Conversations, Tracking, Workspaces, Apps e
Voice.

O que continua não existindo em nenhuma linha: transporte de rede e servidor.

**"impossível"** quer dizer restrição de plataforma, não pendência. Monitorar
processos e registrar atalho global não existem no iOS — a manifestação mobile
dessas features é outra coisa (Shortcuts, App Intents), e não uma porta delas.

---

## 5. Capabilities, e não `if platform`

O §35 proíbe `if iOS` espalhado. A regra:

```
Pergunte o que a plataforma PODE FAZER, nunca QUAL ELA É.
```

Errado:

```ts
if (platform === "ios") mostrarBotaoDeCompartilhar();
```

Certo:

```ts
if (capabilities.nativeShare) mostrarBotaoDeCompartilhar();
```

A diferença aparece no dia em que o macOS entrar: o primeiro esconde o botão
numa plataforma que tem share nativo; o segundo acerta sem ninguém tocar nele.

Capacidades reconhecidas hoje — ver `apps/desktop/src/platform.ts`:

| Capability | Windows | iOS (previsto) |
| --- | --- | --- |
| `globalShortcut` | ✓ | ✗ |
| `fileDrop` | ✓ | ✗ |
| `processMonitoring` | ✓ | ✗ |
| `systemAudioCapture` | ✓ | ✗ |
| `tray` | ✓ | ✗ |
| `nativeShare` | ✗ | ✓ |
| `haptics` | ✗ | ✓ |
| `biometrics` | ✗ | ✓ |
| `camera` | ✗ | ✓ |
| `pushRemote` | ✗ | a construir |
| `localNotifications` | ✓ | ✓ |
| `secureStorage` | ✓ | ✓ (Keychain) |
| `microphone` | ✓ | ✓ |

---

## 6. Serviços de plataforma

Recurso nativo fica atrás de interface, com uma implementação por plataforma:

```
NotificationService     SecureStorageService     FileService
BiometricService        ShareService             VoiceService
HapticService
```

O código de feature fala com a interface. Quem escolhe a implementação é o
shell, uma vez, na inicialização. É isso que permite adicionar uma terceira
plataforma sem varrer o repositório atrás de condicionais.

---

## 7. Checklist obrigatório de feature

Toda feature nova declara o que faz em cada eixo. Ausência é resposta válida —
**silêncio não é**.

```
[ ] Core          domínio e regras (mos-core)
[ ] Database      migration versionada, para frente
[ ] Sync          o que viaja, e como dois lados reconciliam
[ ] Desktop       manifestação e interação
[ ] iOS           manifestação e interação, ou "não se aplica, porque…"
[ ] Notifications local, remota, ou nenhuma
[ ] Hermes        o agente enxerga? age?
[ ] Tests         o que prova que funciona
```

Ver `FEATURE-DEVELOPMENT.md` para o processo completo.

---

## 8. O que ainda não existe

Honestidade sobre o estado, para ninguém construir em cima de promessa:

- **Não há servidor.** Não há auth, não há transporte, não há endpoint. A
  fundação de sync está pronta para receber um; ele não existe.
- **Não há app iOS.** Não há `apps/ios`, não há bundle id, não há signing.
- **Não há push.** Nem infraestrutura, nem certificado, nem plugin.
- **A fila de saída não é alimentada pelo domínio.** A fila funciona, o motor
  funciona e os dois estão testados contra dois bancos reais — mas quem escreve
  nela hoje é o teste. Ligar `mos-core` para emitir uma operação junto com cada
  mutação toca 12 repositórios, e vai de um em um.

O que existe e está testado está na matriz da §4, marcado como `✓` ou
`fundação`.
