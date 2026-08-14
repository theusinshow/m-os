# Spec B — ponte com o Hermes

Data: 2026-08-13
Mãe: `2026-08-13-mos-v03-design.md`
Contrato: `docs/HERMES-GATEWAY-CONTRACT.md`

---

## 1. Princípio

O M/OS **não** cria um agente. Ele é mais uma superfície do Hermes que já roda na VPS e
já é usado por WhatsApp e pelo dashboard.

| Hermes é dono de | M/OS é dono de |
|---|---|
| modelo, provider, reasoning | interface e UX |
| skills e tools | contexto local |
| execução agentic | sessão própria |
| histórico da conversa (`state.db` na VPS) | o `session_id` e mais nada |

Nenhuma skill é duplicada. O `/api/ws` delega a `tui_gateway.server.dispatch` verbatim —
o mesmo dispatcher que atende a TUI e o WhatsApp. Reusar esse caminho é o que garante
que as capacidades sejam idênticas sem nenhum trabalho de sincronização.

---

## 2. Camadas

```
Command (React)   →  modo Search | Hermes
      ↓ invoke / listen
hermes.ts         →  fronteira do renderer, espelha o padrão de api.ts
      ↓ comandos Tauri
HermesBridge      →  sessão, correlação JSON-RPC, máquina de estados
      ↓
Transport         →  WebSocket, auth, backoff
      ↓
127.0.0.1:9119    →  túnel SSH, iniciado externamente
      ↓
Hermes VPS
```

**Crate novo: `crates/mos-hermes`.** Justificativa não é estética: `CORE-FOUNDATION.md:33`
estabelece que *"Hermes não faz parte do Core e não é necessário para o Core funcionar"*.

O crate **não declara dependência de `mos-storage-sqlite`**. Com isso, "Hermes nunca
escreve no SQLite" deixa de ser uma regra que alguém precisa lembrar de respeitar e passa
a ser impossibilidade de compilação. Quando as ações do M/OS chegarem (Fase 3+), elas
passam pela camada de aplicação existente — nunca por aqui.

**Por que a ponte fica em Rust e não no React:**

1. o token nunca entra no WebView;
2. o app já tem `invoke`/`listen` como fronteira única — os deltas de streaming reusam o
   mesmo caminho que `listen("capture-changed")` já usa hoje;
3. não é preciso afrouxar a CSP para abrir WebSocket a partir do renderer.

Nenhuma chamada de rede em componente React. Nenhuma lógica de Hermes no domínio.

---

## 3. Conexão

**Preguiçosa.** Só ao entrar no modo Hermes pela primeira vez. Túnel morto não atrasa o
boot do M/OS, e o app continua inteiro sem Hermes — requisito explícito.

> **Revisão de 2026-08-13, pós-verificação ao vivo.** A versão anterior desta seção
> supunha modo `token` no loopback e tratava o modo gated como fora de escopo. A
> verificação mostrou o contrário: esta instalação roda **gated com provider `basic`**
> (`auth_required: true`, `auth_providers: ["basic"]`). Se a suposição tivesse
> sobrevivido até a implementação, a Fase 1 seria impossível.

Ordem de abertura, verificada:

1. `GET /api/status` → alcançável? `auth_required` e `auth_providers`.
2. `POST /auth/password-login` com `{provider:"basic", username, password}` → cookies de
   sessão.
3. `POST /api/auth/ws-ticket` → `{ticket, ttl_seconds: 30}`.
4. Abrir `ws://127.0.0.1:9119/api/ws?ticket=<ticket>` **imediatamente**.
5. **Esperar o frame `gateway.ready`.** Só então o estado é `Online`. HTTP 200 não basta —
   é o falso positivo que `connection-config.cjs:99` documenta.

**Ticket é efêmero: 30 segundos, uso único.** Um ticket por socket, cunhado logo antes de
conectar. Guardar ticket é bug, e reconexão sempre cunha outro.

**Cookie jar persistente** entre requisições, não chamadas soltas: o middleware rotaciona
o access token (~15 min) a partir do refresh (24h) na próxima requisição autenticada.
Sessão com AT expirado e RT vivo **continua conectável** — tratar AT ausente como
deslogado forçaria relogin a cada 15 minutos sem motivo.

**Credenciais.** O M/OS precisa de usuário e senha do dashboard, guardados no **Windows
Credential Manager** — decisão que `ARCHITECTURE.md:556` já estabelecia para tokens.
Nunca em `localStorage`, nunca em `.env`, nunca no WebView. Configurados em Settings; sem
credencial, o modo Hermes fica indisponível com mensagem explícita, não quebrado.

Erros de login têm tratamento distinto por código, e são propositalmente genéricos do
lado do servidor para não virarem oráculo de enumeração: `401` credencial inválida,
`404` provider ausente, `503` store inacessível, `429` tentativas demais. **`429` não
dispara retry** — retry aqui piora a situação que causou o bloqueio.

Base URL configurável em Settings, default `http://127.0.0.1:9119`. O modo é classificado
pelo `/api/status` a cada conexão, nunca assumido: se um dia esta instalação virar modo
`token` ou `oauth`, o cliente percebe em vez de falhar de lado.

---

## 4. Máquina de estados

```
Offline ──conectar──> Connecting ──gateway.ready──> Online
   ↑                       │                          │
   └──── falha/backoff ────┘                          │
   └──────────────── socket caiu ─────────────────────┘

Error: estado terminal por turno, não da conexão
```

| Estado | O que o usuário vê |
|---|---|
| `Offline` | "Hermes indisponível — o túnel SSH não está aberto." Diz a causa provável, porque ela é quase sempre essa. |
| `Connecting` | a barra girando (`barSpin`), o único spinner do sistema |
| `Online` | o modo Hermes aceita envio |
| `Error` | o que aconteceu, e o que fazer agora |

Estado aparece no slot de sistema da topbar, que a Spec A já constrói, e dentro do
Command em modo Hermes.

**Retry:** backoff exponencial com teto, e **sem retry automático em `Offline` frio** —
túnel fechado não é falha transitória, e reconectar em loop contra um socket recusado é
ruído. Reconexão automática só depois de ter estado `Online` uma vez, que é o caso do
túnel que caiu. Fora isso, reconecta quando o usuário pede.

**Códigos de fechamento** têm tratamento distinto. `4401` é falha de autenticação, e a
causa mais provável é ticket vencido (30s) ou sessão expirada: cunhar ticket novo e
tentar **uma vez**; se falhar de novo, refazer o login e tentar uma última vez; só então
`Error`. `4403` é recusa de política e **não** se resolve com repetição. Nenhum dos dois
é "erro de rede" e nenhum deve ser apresentado como tal.

**Drift de versão:** método JSON-RPC desconhecido é erro de contrato explícito, nomeando
o método. O contrato foi lido do checkout local de junho/2026 e a VPS pode divergir —
essa é a mitigação.

---

## 5. Sessão

Própria e **persistente**. Separada do WhatsApp e do dashboard.

- Primeira vez: `session.create` com `title: "M/OS"`. Nada de `cwd`, `model`,
  `provider` ou `reasoning_effort` — o Hermes decide, é dono disso.
- O `session_id` é guardado localmente. O histórico **não** é: ele vive no `state.db` da
  VPS e é de lá que vem.
- Ao reabrir o app: `session.resume` com o id guardado.
- Se o id não existir mais na VPS, `session.resume` aceita **título** como alvo, então
  `"M/OS"` é a rota de recuperação antes de criar sessão nova.
- `session.close` ao fechar o app.

`prompt.submit` responde `4009 "session busy"` quando há turno em andamento. Isso define
a UI diretamente: enquanto roda, o campo não aceita novo envio — oferece cancelar.

---

## 6. Streaming

Eventos chegam como `{"jsonrpc":"2.0","method":"event","params":{"type":…}}` e são
repassados ao renderer por `emit`/`listen`.

| Evento | Tratamento na Fase 1 |
|---|---|
| `message.start` / `message.delta` / `message.complete` | renderiza a resposta, token a token |
| `reasoning.delta`, `thinking.delta`, `reasoning.available` | acumulado, **escondido** por padrão, revelável sob ação discreta |
| `tool.start` / `tool.generating` / `tool.complete` | linha de estado em mono, atrás da barra de 2px em sódio que o design define como marcador de autoria do sistema |
| `approval.request` | §7 |
| `status.update`, `session.info` | alimentam o estado da topbar |
| `error` | mensagem legível, turno encerrado |
| `voice.*`, `skin.changed`, `preview.restart.*`, `browser.progress` | ignorados nesta fase |

**Não rebufferizar.** O servidor desativa o algoritmo de Nagle de propósito para
preservar a cadência de digitação (`ws.py:155-170`), e reagrupar os deltas do lado do
M/OS desfaria esse trabalho.

O único ponto de motion novo é o texto chegando. Nada pulsa, nada tem bolha, nada tem
orb, nenhum gradiente. O Hermes deve parecer capacidade nativa do M/OS — a resposta é
texto na tipografia do sistema, e a autoria é marcada pela barra, que é o mecanismo que o
design já definiu para "isto foi o sistema que produziu".

---

## 7. Aprovação

`approval.request` é evento **de entrada**: o agente para e espera. Cliente que o ignora
trava a conversa em silêncio.

O Command renderiza o pedido e responde com `approval.respond`
(`{session_id, choice, all}`). Isso não dá ao Hermes nenhum poder novo sobre o M/OS — é o
Hermes pedindo permissão para as ferramentas dele mesmo, as que ele já usa pelo WhatsApp.

`choice` tem default `"deny"` no servidor. O M/OS segue o mesmo default: **fechar o
Command sem escolher é negar**, nunca aprovar por omissão. Isso está alinhado ao princípio
de confirmação explícita para ação externa ou destrutiva.

Rede de segurança: `session.interrupt` já resolve todas as aprovações pendentes como
`deny` (`resolve_gateway_approval(..., "deny", resolve_all=True)`). Cancelar é, portanto,
saída segura de um approval travado — não só de um turno longo.

---

## 8. Superfície de UX

`Ctrl+K` abre o Command. Dois modos no mesmo campo, alternados por `Tab`, com o modo
**visível** — não folclore:

```
┌─────────────────────────────────────────┐
│ / O que você quer fazer?                │
│                                         │
│ Search                         Hermes   │
│ ─────────────────────────────────────── │
└─────────────────────────────────────────┘
```

O protótipo já previa `TAB HERMES` no rodapé do Command; o alternador visível é a mesma
decisão, tornada legível. A barra `/` continua sendo o limiar de entrada, em mono e
`--signal-ink`, porque é a mesma regra de identidade: todo campo onde algo entra no
sistema começa com a barra.

Proibido, e isto é requisito, não gosto: bolha de chat, sidebar de suporte, orb, gradiente
"AI", sparkle, qualquer coisa que faça o M/OS parecer ter um ChatGPT colado nele.

---

## 9. Fora de escopo nesta fase

Não implementar: `mos_create_task`, `mos_update_task`, `mos_create_project`,
`mos_create_capture`, `mos_move_task`, escrita direta no banco, automações, voz, contexto
automático da tela, gerência automática do túnel SSH.

O M/OS **não** usa `/api/files/*`, `/api/fs/*`, `/api/pty`, `/api/cron/*`, `/api/env`,
`/api/config`, `/api/model/set` nem `/api/audio/*`, embora o gateway os exponha.

Ordem de evolução acordada: Chat → `mos_search` e `mos_get_context` → `mos_create_capture`
e `mos_create_task` → demais ações → apps externos. **Leitura antes de escrita, sempre.**

---

## 10. Verificação

`mos-hermes` é testável sem rede: o `Transport` é um trait, e os testes rodam contra um
duplo que reproduz frames reais capturados do contrato — `gateway.ready`, uma sequência
de `message.delta`, um `approval.request`, um `4009 session busy`, um fechamento `4401`.

Cenários que precisam ser observados, não presumidos:

1. túnel fechado → `Offline` com a causa nomeada, e o M/OS inteiro segue funcionando;
2. túnel cai no meio de um turno → estado degrada sem perder o que já chegou;
3. `session.resume` com id morto → recuperação por título;
4. `approval.request` → render, resposta, e o turno continua;
5. cancelamento no meio do streaming → `session.interrupt`, aprovações pendentes negadas;
6. método desconhecido → erro de contrato nomeando o método.

7. `429` no login → nenhuma tentativa automática, mensagem dizendo o que houve.

**Verificação de infraestrutura concluída** (ver `HERMES-GATEWAY-CONTRACT.md` §6):
alcançabilidade, modo de auth, provider e versão confirmados ao vivo, sem drift entre a
VPS e o checkout local. O que resta observar são os quatro cenários de protocolo que
exigem sessão autenticada — `gateway.ready`, `message.delta`, `approval.request` e
`4009 session busy` — e eles serão exercitados durante a implementação.
