# Subir o hub de sincronização na VPS

O que este documento entrega: o M/OS do PC sincronizando através da VPS, pelo
mesmo túnel SSH que o Hermes já usa. **Não** entrega o iPhone — ver a última
seção.

Cada passo diz *por quê*, porque um runbook que só lista comandos vira um
runbook que ninguém sabe consertar quando um passo falha.

---

## 0. O que roda onde

```
PC (Windows)                          VPS (167.233.43.1)
┌──────────────┐   túnel SSH   ┌─────────────────────────┐
│ M/OS         │──────────────▶│ mos-sync-server         │
│ 127.0.0.1    │  porta 9120   │ 127.0.0.1:9120          │
│ :9120        │               │ /var/lib/mos-sync/*.db  │
└──────────────┘               └─────────────────────────┘
```

O hub escuta **somente em localhost** na VPS. É a mesma decisão do dashboard do
Hermes, pela mesma razão: um banco pessoal não precisa de porta pública, e uma
porta que não existe não é atacada.

---

## 1. O binário

Não compile na VPS. Ela só precisa **executar** o hub, e uma toolchain Rust num
servidor exposto é superfície a mais sem contrapartida.

O workflow `Sync server` do GitHub Actions produz um executável estático para
Linux x86_64 (`musl`, não `gnu` — o binário `gnu` amarra o glibc do runner e
uma VPS mais antiga recusa a executá-lo com um erro que não explica nada).

```bash
# Dispare pelo Actions (workflow_dispatch) e baixe o artifact, ou:
gh run download --name mos-sync-server-linux-x86_64
```

Na VPS:

```bash
sudo install -m 755 mos-sync-server /usr/local/bin/mos-sync-server
```

---

## 2. O usuário e o diretório

Usuário próprio, sem shell e sem home: o hub não precisa de nenhum dos dois, e
um serviço que roda como `root` transforma qualquer defeito dele num defeito da
máquina inteira.

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin mos-sync
sudo mkdir -p /var/lib/mos-sync
sudo chown mos-sync:mos-sync /var/lib/mos-sync
sudo chmod 700 /var/lib/mos-sync
```

---

## 3. O segredo

Gere na VPS e **não** o digite em lugar nenhum antes de precisar:

```bash
openssl rand -base64 48 | tr -d '\n' | sudo tee /dev/null   # veja o valor
```

Grave o arquivo de ambiente. Modo `600` e dono `root`: o `systemd` lê como root
antes de baixar privilégio, e o usuário do serviço nunca precisa ler o arquivo.

```bash
sudo tee /etc/mos-sync.env >/dev/null <<'ENV'
MOS_SYNC_TOKEN=COLE_O_SEGREDO_AQUI
MOS_SYNC_DB=/var/lib/mos-sync/hub.db
MOS_SYNC_PORT=9120
MOS_SYNC_BIND=127.0.0.1
ENV
sudo chmod 600 /etc/mos-sync.env
sudo chown root:root /etc/mos-sync.env
```

O binário **recusa a subir** sem `MOS_SYNC_TOKEN` de 32+ caracteres. Isso é
proposital: um hub que sobe com token vazio sobe inseguro, e "eu troco depois" é
como toda porta aberta começa.

---

## 4. O serviço

```bash
sudo cp deploy/mos-sync.service /etc/systemd/system/mos-sync.service
sudo systemctl daemon-reload
sudo systemctl enable --now mos-sync
systemctl status mos-sync --no-pager
```

Conferência, na própria VPS:

```bash
curl -s http://127.0.0.1:9120/health
# {"ok":true,"contrato":1,"minimo":1}
```

Se isto não responder, nada adiante vai funcionar — resolva aqui, e não no PC.

---

## 5. O túnel, no PC

```powershell
# Uma vez, para testar:
ssh -N -L 9120:127.0.0.1:9120 hermes@167.233.43.1

# Permanente, como o do Hermes (tarefa agendada no logon):
scripts\sync-tunnel.ps1
```

Conferência no PC:

```powershell
curl.exe -s http://127.0.0.1:9120/health
```

---

## 6. O M/OS

Settings → **SINCRONIZAÇÃO**:

- **Endereço do hub:** `http://127.0.0.1:9120`
- **Segredo:** o mesmo do `/etc/mos-sync.env`

`http` e não `https` está correto aqui: o transporte já é o SSH, que cifra tudo
entre o PC e a VPS. Um TLS por cima do túnel cifraria duas vezes o mesmo trecho.

Clique **Sincronizar agora**. A linha diz quantas subiram e quantas desceram; a
fila deve ir a zero.

---

## 7. O segundo aparelho

É o teste que importa, e ele não precisa de celular: instale o M/OS em outra
máquina (ou rode uma segunda instância com banco próprio), aponte para o mesmo
hub com o mesmo segredo, e sincronize. O que nasceu num aparelho tem que
aparecer no outro.

---

## O que isto NÃO resolve: o iPhone

O túnel SSH serve o desktop. O iPhone não mantém um, e para ele o hub vai
precisar de **endereço público com TLS** — um `mos-sync.seudominio` atrás de
Caddy ou nginx com Let's Encrypt, e `MOS_SYNC_BIND` continuando em `127.0.0.1`
com o proxy na frente.

Isso é uma decisão separada, com consequências próprias: um endereço público
existe para o mundo inteiro, e a partir daí o segredo compartilhado passa a ser
a única coisa entre a internet e o seu banco. Vale fazer no dia em que o app iOS
existir — antes disso, é superfície exposta sem cliente para usá-la.

---

# A superfície de bolso (`mos-web`)

O hub acima faz dois aparelhos se alcançarem. Isto é o aparelho que você carrega
no bolso: um M/OS pequeno, com banco próprio, que sincroniza pelo mesmo hub.

## 1. O binário

Workflow `M/OS web` no Actions. Ele constrói a PWA **antes** do Rust — o
`rust-embed` lê a pasta `static/` em tempo de compilação, e a ordem invertida
produz um binário que sobe, responde à API e serve uma **página em branco**.

```bash
gh run download --name mos-web-linux-x86_64
sudo install -m 755 mos-web /usr/local/bin/mos-web
```

## 2. O usuário

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin mos-web
sudo mkdir -p /var/lib/mos-web
sudo chown mos-web:mos-web /var/lib/mos-web
sudo chmod 700 /var/lib/mos-web
```

## 3. O ambiente

```bash
sudo tee /etc/mos-web.env >/dev/null <<'ENV'
MOS_WEB_BIND=127.0.0.1
MOS_WEB_PORT=9130
MOS_WEB_DB=/var/lib/mos-web/mos-web.db
MOS_WEB_BACKUPS=/var/lib/mos-web/backups
MOS_WEB_HUB=http://127.0.0.1:9120
MOS_WEB_TOKEN=O_MESMO_SEGREDO_DO_HUB
MOS_WEB_INVITE=UM_SEGUNDO_SEGREDO_SO_PARA_REGISTRAR_APARELHO
ENV
sudo chmod 600 /etc/mos-web.env
```

`MOS_WEB_HUB` aponta para `127.0.0.1:9120` porque o hub roda **na mesma VPS**.
Os dois conversam por localhost, e nada disso sai da máquina.

`MOS_WEB_INVITE` é o que passkey **não** resolve: ele autentica quem já é
conhecido, mas não decide quem passa a ser. Sem essa trava, a primeira pessoa
que achasse a URL viraria a dona da casa.

## 4. O serviço

```bash
sudo cp deploy/mos-web.service /etc/systemd/system/mos-web.service
sudo systemctl daemon-reload
sudo systemctl enable --now mos-web
curl -s http://127.0.0.1:9130/api/estado
```

## 5. O proxy com TLS — e por que ele não é opcional

**Passkey exige origem HTTPS estável**, e o cookie de sessão é `Secure`: em HTTP
ele nem é enviado, e o login pareceria não funcionar sem dizer por quê.

Com Caddy, é uma linha:

```
mos.seudominio.com {
    reverse_proxy 127.0.0.1:9130
}
```

O `MOS_WEB_BIND` continua em `127.0.0.1`: quem fala com a internet é o proxy.

**Um aviso que não cabe em nota de rodapé:** a partir daqui existe um endereço
público, e atrás dele está o seu cérebro inteiro. O que separa os dois é a
passkey e o convite. Vale conferir os dois antes de apontar o DNS.

## 6. No iPhone

Safari → o endereço → Compartilhar → **Adicionar à Tela de Início**. Ele abre em
tela cheia, sem barra do navegador, com o ícone do M/OS.
