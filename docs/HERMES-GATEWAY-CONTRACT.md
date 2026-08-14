# Hermes Gateway — contrato factual

Investigação read-only concluída em 2026-08-13. Este documento registra **o que foi
verificado no código**, não o que foi suposto. Cada afirmação cita a evidência.

O M/OS não cria um agente. Ele é mais uma superfície do Hermes que já existe.

---

## 1. O que é o Hermes desta instalação

`NousResearch/Hermes-Agent`, checkout completo em
`%LOCALAPPDATA%\hermes\hermes-agent` (`package.json` → `repository.url`).

O componente que interessa ao M/OS é o **dashboard web** (`hermes dashboard`), um
servidor FastAPI que serve tanto a UI React quanto a API. Ele já é o backend do app
Electron `apps/desktop` — que é, funcionalmente, o mesmo papel que o M/OS quer ocupar.

Consequência: **existe implementação de referência**. O M/OS não precisa inventar
transporte, autenticação nem protocolo; precisa reimplementar em Rust/TypeScript o que
`apps/desktop/electron/` já faz em Node.

---

## 2. Rede

| Fato | Evidência |
|---|---|
| Porta padrão do dashboard: **9119** | `hermes_cli/subcommands/dashboard.py:26` |
| Bind é localhost-only por design | `docker-compose.yml:75`, `docs/security/network-egress-isolation.md:90` |
| Acesso remoto é **por túnel SSH**, recomendado pelo próprio projeto | `docker-compose.yml:75`: *"Localhost-only. For remote access, tunnel via `ssh -L 9119:localhost:9119`"* |
| O túnel já existe nesta máquina, num atalho de Desktop | `Hermes Dashboard.lnk` → `ssh -L 9119:localhost:9119 hermes@<vps>` |

Isto confirma a preferência arquitetural registrada: **não é preciso expor porta nova**.
O caminho `M/OS → 127.0.0.1:9119 → túnel SSH → Hermes VPS` é o modo suportado pelo
upstream, não uma gambiarra.

Estado do túnel no momento da investigação: **fechado** (`curl 127.0.0.1:9119/api/status`
→ `connection refused`). Isso é exatamente o estado `Offline` que o M/OS precisa tratar
com dignidade.

---

## 3. Autenticação

> **Verificado ao vivo em 2026-08-13, com o túnel aberto.** Esta instalação está em
> **modo gated com provider `basic`** — não em modo token. A suposição inicial de que
> loopback implicaria modo token estava **errada**, e é por isso que este gate existiu.

### 3.0 O que esta instalação usa (verificado)

`GET /api/status` responde:

```json
{"version":"0.16.0","release_date":"2026.6.5","gateway_running":true,
 "gateway_platforms":{"whatsapp":{"state":"connected"}},
 "active_sessions":0,"auth_required":true,"auth_providers":["basic"]}
```

`GET /api/auth/providers` responde:

```json
{"providers":[{"name":"basic","display_name":"Username & Password","supports_password":true}]}
```

Sem credencial, `GET /api/auth/me` e `GET /api/ws` retornam **401**.

**A versão bate com o checkout local** (0.16.0, release 2026.6.5) — o risco de drift de
contrato que a investigação apontou está descartado para esta instalação.

O WhatsApp está `connected` no mesmo gateway. Confirma na prática que reusar este caminho
é reusar o mesmo Hermes, com as mesmas skills.

### 3.1 Fluxo que o M/OS deve implementar

```
1. GET  /api/status              → auth_required: true, auth_providers: ["basic"]
2. POST /auth/password-login     → {provider:"basic", username, password}
                                 → 200 {"ok":true,"next":"/"} + cookies de sessão
3. POST /api/auth/ws-ticket      → autenticado por cookie
                                 → {"ticket":"…","ttl_seconds":30}
4. ws://127.0.0.1:9119/api/ws?ticket=<ticket>
```

**O ticket vive 30 segundos e é de uso único** (`routes.py:593`). A própria docstring
declara que pedir um ticket por WebSocket, em sequência rápida, é o padrão esperado.
Consequência de projeto: **cada reconexão cunha um ticket novo**, imediatamente antes de
abrir o socket. Guardar ticket é bug.

Falhas de `/auth/password-login` são deliberadamente genéricas, para o endpoint não virar
oráculo de enumeração (`routes.py:456`):

| Código | Significado |
|---|---|
| `404` | provider desconhecido ou sem suporte a senha |
| `401` | credencial inválida |
| `503` | backing store inacessível |
| `429` | tentativas demais deste IP |

Cada um merece mensagem distinta na UI. `429` em particular **não** deve disparar retry.

### 3.2 Cookies

`hermes_session_at` (acesso, ~15 min) e `hermes_session_rt` (refresh, 24h rotativo com
detecção de reuso), com variantes `__Host-` e `__Secure-` conforme o deploy. Em loopback
HTTP são os nomes crus.

O middleware **rotaciona o access token automaticamente** a partir do refresh na próxima
requisição autenticada. Portanto o cliente precisa de **cookie jar persistente entre
requisições** — não de chamadas soltas — e uma sessão cujo AT expirou continua viva.
Tratar AT ausente como "deslogado" forçaria relogin a cada 15 minutos sem necessidade
(`connection-config.cjs:249`).

### 3.3 Modo `token` (não é o caso desta instalação)

- REST: header `X-Hermes-Session-Token`
- WebSocket: query `?token=<token>`
- Descoberta do token: o HTML servido em `/` injeta
  `window.__HERMES_SESSION_TOKEN__ = "..."`, e o cliente o extrai
  (`electron/dashboard-token.cjs:32`, `resolveServedDashboardToken`)

O comentário do arquivo registra o motivo de o cliente reler o token do HTML em vez de
confiar no que ele mesmo passou: se os dois divergirem, *"HTTP readiness probes still
pass while /api/ws rejects the renderer's token"*. O M/OS deve seguir a mesma ordem.

### 3.4 Modo `oauth` (gateways hospedados)

- REST: cookie HttpOnly (`hermes_session_at` / `hermes_session_rt`, com variantes
  `__Host-` e `__Secure-`)
- WebSocket: ticket de uso único via `POST /api/auth/ws-ticket`, usado como `?ticket=`

Não é o cenário de loopback, mas o cliente do M/OS deve classificar o modo pelo
`/api/status` em vez de assumir — é uma linha de código e evita um bug de "funciona na
minha máquina".

---

## 4. Protocolo do chat

**JSON-RPC 2.0, newline-delimited, sobre WebSocket em `/api/ws`.**
Bidirecional e idêntico ao transporte stdio da TUI (`tui_gateway/ws.py:8-12`).

```
ws://127.0.0.1:9119/api/ws?token=<token>
```

`web_server.py:10821` autentica e delega a `tui_gateway.ws.handle_ws`, que reusa
`tui_gateway.server.dispatch` **verbatim** — o mesmo dispatcher que atende a TUI, o
cliente web e o iOS. É por isso que reusar este caminho não duplica skills nem
ferramentas: é literalmente o mesmo agente, com as mesmas capacidades.

O feature flag `_DASHBOARD_EMBEDDED_CHAT_ENABLED` está **hardcoded como `True`**
nesta versão (`web_server.py:198`). Se estivesse desligado, o socket fecharia com
código `4403`.

### Handshake

Ao aceitar a conexão, o servidor emite imediatamente (`ws.py:193`):

```json
{"jsonrpc":"2.0","method":"event","params":{"type":"gateway.ready","payload":{"skin":"..."}}}
```

Receber esse frame é o sinal de `Online`. Só HTTP `/api/status` respondendo **não** é
suficiente — o cliente Electron documenta exatamente esse falso positivo
(`connection-config.cjs:99`).

### Códigos de fechamento

| Código | Significado | Evidência |
|---|---|---|
| `4401` | falha de autenticação | `web_server.py:10828` |
| `4403` | chat embutido desligado, ou origem não permitida | `web_server.py:10824`, `10833` |
| `-32700` | erro de parse do JSON | `ws.py:243` |
| `-32603` | erro interno no dispatch | `ws.py:274` |

### Métodos relevantes à Fase 1

Todos verificados em `tui_gateway/server.py`:

| Método | Linha | Papel no M/OS |
|---|---|---|
| `session.create` | 3986 | cria a **sessão própria** do M/OS |
| `prompt.submit` | 5660 | envia a mensagem do usuário |
| `session.interrupt` | 5377 | **cancelamento** |
| `session.close` | 5294 | encerra ao fechar o app |
| `session.status` | 5035 | estado da sessão |
| `session.history` | 5092 | reidratar a conversa |
| `session.list` | 4128 | listar sessões |

`session.create` aceita `title`, `cwd`, `model`, `provider`, `reasoning_effort`,
`profile` e `messages` (histórico semente) — todos opcionais. O M/OS mandará `title`
para a sessão ser reconhecível no dashboard, e nada além disso na Fase 1.

`prompt.submit` recebe `{session_id, text}` e responde erro `4009 "session busy"` se
houver turno em andamento. Isso define o estado de UI: enquanto rodando, o campo do
Command não aceita novo envio — ele oferece cancelar.

### Eventos de streaming

Chegam como `{"jsonrpc":"2.0","method":"event","params":{"type":<tipo>,...}}`.
Tipos emitidos (`grep _emit` em `tui_gateway/server.py`):

```
message.start · message.delta · message.complete
reasoning.delta · reasoning.available · thinking.delta
tool.start · tool.generating · tool.complete
status.update · session.info · error
approval.request · browser.progress
voice.status · voice.transcript
skin.changed · preview.restart.{progress,complete}
```

Streaming **existe e é por token**: o servidor desativa o algoritmo de Nagle
explicitamente para preservar a cadência (`ws.py:155-170`), com o comentário de que sem
isso *"a burst after the model's think-pause lands on the client in one tick"*. Ou seja,
a fidelidade da digitação é um requisito assumido pelo upstream — o M/OS deve respeitá-la
e não rebufferizar.

**`approval.request` é um evento de entrada, não de saída.** O agente pode pedir
aprovação no meio do turno e ficar esperando `approval.respond`. Um cliente que ignora
esse evento trava a conversa sem explicação. Ver seção 6.

---

## 5. O que o M/OS NÃO usa

O gateway expõe muito mais (`/api/files/*`, `/api/fs/*`, `/api/pty`, `/api/cron/*`,
`/api/env`, `/api/config`, `/api/model/set`, `/api/providers/oauth/*`, `/api/audio/*`).

Nada disso entra na Fase 1. Em particular, o M/OS **não** escreve configuração, **não**
troca modelo e **não** mexe em env — o Hermes continua dono de modelo, provider,
reasoning, skills e execução agentic, conforme decidido.

---

## 6. Estado da verificação

Verificado ao vivo em 2026-08-13, com o túnel aberto:

- [x] alcançabilidade em `127.0.0.1:9119`
- [x] `GET /api/status` — corpo real, `auth_required: true`
- [x] modo de auth efetivo: **gated, provider `basic` (senha)**
- [x] versão da VPS = versão do checkout local (0.16.0 / 2026.6.5) — **sem drift**
- [x] `GET /api/auth/providers` — provider id confirmado
- [x] `/api/auth/me` e `/api/ws` rejeitam com 401 sem credencial
- [x] WhatsApp `connected` no mesmo gateway — é o mesmo Hermes

Lido do código, não exercitado ao vivo (exige sessão autenticada, o que só o app fará):

- [ ] frame `gateway.ready` na aceitação do socket
- [ ] sequência `message.delta` de um turno real
- [ ] `approval.request` → `approval.respond`
- [ ] `4009 session busy` em envio concorrente

Estes quatro são exatamente os cenários de teste da Spec B §10. Serão observados durante
a implementação, não presumidos.

**Uma decisão de produto ficou aberta e foi resolvida:** `approval.request` é evento de
entrada — se um skill pedir aprovação num turno iniciado pelo M/OS, ou o M/OS renderiza e
responde, ou a sessão trava. Decidido: renderiza e responde, com `deny` por omissão.
