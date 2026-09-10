#!/usr/bin/env bash
#
# Sobe o M/OS de bolso na VPS, do zero, numa passada.
#
# Rodar como root:
#
#     ssh -t hermes@167.233.43.1 'sudo bash /tmp/mos/bootstrap-vps.sh'
#
# Ele espera encontrar os dois binários em /tmp/mos/ — o `mos-web` e o
# `mos-sync-server`, baixados dos artifacts do GitHub Actions.
#
# # Ele é idempotente, e isso não é detalhe
#
# Rodar duas vezes não quebra nada e, sobretudo, **não regenera segredo
# nenhum**. Um script de deploy que sorteia chave nova a cada execução é um
# script que, na segunda vez, mata todas as passkeys e todas as assinaturas de
# push — e o sintoma disso é tudo parecer certo e nada funcionar.
#
# Cada passo diz por quê. Um runbook que só lista comandos vira um runbook que
# ninguém sabe consertar quando um passo falha.

set -euo pipefail

DOMINIO="${MOS_DOMINIO:-167-233-43-1.sslip.io}"
ORIGEM="${MOS_ORIGEM:-/tmp/mos}"
CONTATO="${MOS_CONTATO:-mailto:matheusmendes077@gmail.com}"

if [ "$(id -u)" -ne 0 ]; then
  echo "Este script precisa de root: sudo bash $0" >&2
  exit 1
fi

dizer() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

# --------------------------------------------------------------- 1. binários

dizer "Binários"
for binario in mos-web mos-sync-server; do
  if [ ! -f "$ORIGEM/$binario" ]; then
    echo "Falta $ORIGEM/$binario. Baixe o artifact do Actions primeiro." >&2
    exit 1
  fi
  install -m 755 "$ORIGEM/$binario" "/usr/local/bin/$binario"
  echo "instalado /usr/local/bin/$binario"
done

# ------------------------------------------------------------- 2. usuários

# Usuário próprio por serviço, sem shell e sem home: nenhum dos dois é
# necessário, e um serviço que roda como root transforma qualquer defeito dele
# num defeito da máquina inteira.
dizer "Usuários e diretórios"
for servico in mos-sync mos-web; do
  id -u "$servico" >/dev/null 2>&1 ||
    useradd --system --no-create-home --shell /usr/sbin/nologin "$servico"
  mkdir -p "/var/lib/$servico"
  chown "$servico:$servico" "/var/lib/$servico"
  chmod 700 "/var/lib/$servico"
  echo "ok $servico"
done

# --------------------------------------------------------------- 3. segredos

# Gerados aqui e SÓ se ainda não existirem. Ver a nota sobre idempotência no
# topo: regenerar é o defeito mais caro que este script poderia ter.
dizer "Segredos"

if [ -f /etc/mos-sync.env ]; then
  TOKEN="$(grep '^MOS_SYNC_TOKEN=' /etc/mos-sync.env | cut -d= -f2-)"
  echo "token do hub: preservado"
else
  TOKEN="$(openssl rand -base64 48 | tr -d '\n')"
  echo "token do hub: gerado"
fi

if [ -f /etc/mos-web.env ]; then
  CONVITE="$(grep '^MOS_WEB_INVITE=' /etc/mos-web.env | cut -d= -f2-)"
  VAPID="$(grep '^MOS_WEB_VAPID_PRIVADA=' /etc/mos-web.env | cut -d= -f2-)"
  echo "convite e chave VAPID: preservados"
else
  CONVITE="$(openssl rand -base64 24 | tr -d '\n')"
  # A chave sai do próprio binário — P-256, o que o Web Push exige.
  VAPID="$(/usr/local/bin/mos-web --gerar-vapid | grep '^MOS_WEB_VAPID_PRIVADA=' | cut -d= -f2-)"
  echo "convite e chave VAPID: gerados"
fi

if [ -f /etc/mos-proxy.env ]; then
  SENHA="$(grep '^SENHA=' /etc/mos-proxy.env | cut -d= -f2-)"
  echo "senha do proxy: preservada"
else
  # Sem `/`, `+` e `=`: a senha vai ser digitada num teclado de iPhone, e esses
  # tres sao os que mais custam a achar ali.
  SENHA="$(openssl rand -base64 18 | tr -d '\n/+=')"
  echo "senha do proxy: gerada"
fi
printf 'USUARIO=%s\nSENHA=%s\n' "${MOS_USUARIO:-matheus}" "$SENHA" >/etc/mos-proxy.env
chmod 600 /etc/mos-proxy.env && chown root:root /etc/mos-proxy.env

# Modo 600 e dono root: o systemd lê como root ANTES de baixar privilégio, e o
# usuário do serviço nunca precisa poder ler o arquivo.
cat >/etc/mos-sync.env <<ENV
MOS_SYNC_TOKEN=$TOKEN
MOS_SYNC_DB=/var/lib/mos-sync/hub.db
MOS_SYNC_PORT=9120
MOS_SYNC_BIND=127.0.0.1
ENV
chmod 600 /etc/mos-sync.env && chown root:root /etc/mos-sync.env

cat >/etc/mos-web.env <<ENV
MOS_WEB_BIND=127.0.0.1
MOS_WEB_PORT=9130
MOS_WEB_DB=/var/lib/mos-web/mos-web.db
MOS_WEB_BACKUPS=/var/lib/mos-web/backups
MOS_WEB_PUSH_DB=/var/lib/mos-web/push.db
MOS_WEB_HUB=http://127.0.0.1:9120
MOS_WEB_TOKEN=$TOKEN
MOS_WEB_INVITE=$CONVITE
MOS_WEB_VAPID_PRIVADA=$VAPID
MOS_WEB_VAPID_CONTATO=$CONTATO
MOS_WEB_ORIGEM=https://$DOMINIO
ENV
chmod 600 /etc/mos-web.env && chown root:root /etc/mos-web.env

# ---------------------------------------------------------------- 4. unidades

dizer "Unidades systemd"
for unidade in mos-sync mos-web; do
  if [ ! -f "$ORIGEM/$unidade.service" ]; then
    echo "Falta $ORIGEM/$unidade.service" >&2
    exit 1
  fi
  install -m 644 "$ORIGEM/$unidade.service" "/etc/systemd/system/$unidade.service"
done
systemctl daemon-reload
systemctl enable --now mos-sync
systemctl restart mos-sync
systemctl enable --now mos-web
systemctl restart mos-web

# ------------------------------------------------------------------ 5. Caddy

# TLS não é opcional aqui, e por duas razões independentes: passkey exige origem
# HTTPS estável, e o cookie de sessão é `Secure` — em HTTP ele nem é enviado, e
# o login pareceria não funcionar sem dizer por quê. Some-se uma terceira: o
# Web Push do iOS só existe sobre HTTPS.
dizer "Caddy e TLS"
if ! command -v caddy >/dev/null 2>&1; then
  apt-get install -y debian-keyring debian-archive-keyring apt-transport-https curl gnupg >/dev/null
  curl -fsSL https://dl.cloudsmith.io/public/caddy/stable/gpg.key |
    gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
  curl -fsSL https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt |
    tee /etc/apt/sources.list.d/caddy-stable.list >/dev/null
  apt-get update -qq
  apt-get install -y caddy >/dev/null
  echo "caddy instalado"
else
  echo "caddy já estava instalado"
fi

# --- A porta é a passkey, e ela mora no BINÁRIO -------------------------------
#
# **Correção de 10/09/2026.** Este comentário dizia que o `auth.rs` tinha passkey
# "escrito e não montado em rota nenhuma", e que por isso quem autenticava era o
# proxy. Fazia meses que não era verdade, e o texto sobreviveu ao código:
# `api.rs::servidor` faz `merge` da cerimônia e aplica `porta::guarda` como
# camada; `porta.rs` protege todo `/api/*` fora de `/api/porta/`, de modo que
# uma rota nova nasce protegida por omissão.
#
# A consequência de acreditar no comentário: o M/OS de bolso ficou com DUAS
# portas em série — a senha do Caddy e a passkey — e a de fora, sendo a que se
# digita, virou a que incomoda. Trocá-la por uma fácil seria enfraquecer a única
# das duas que a maioria dos ataques encontra primeiro.
#
# Então a senha sai da frente. O que autentica é a passkey:
#
#   registrar  exige o CONVITE (`MOS_WEB_INVITE`), comparado em tempo constante
#   entrar     exige a chave privada, que não sai do Secure Enclave do aparelho
#
# O que fica público é a CASCA: o HTML, o JS e o CSS que precisam carregar para
# a tela de entrar existir. Nenhum dado atravessa — ele está atrás do guardião.
#
# `/etc/mos-proxy.env` continua sendo gerado, e não é sobra: é a porta de
# emergência. Se a passkey falhar num aparelho novo, `deploy/voltar-senha.py`
# repõe o Basic Auth em um comando, com o hash saindo dali.

# O hub fica FORA do Basic Auth, e isso não é descuido.
#
# O cliente de sincronização manda `Authorization: Bearer`, e o Basic Auth do
# Caddy recusaria a chamada antes de o hub sequer ver o token. A proteção de
# `/sync/*` é o segredo de 64 caracteres, comparado em tempo constante dentro do
# próprio hub (`http.rs`, `tempo_constante`).
#
# O que isso expõe, dito com todas as letras: o hub passa a ser alcançável da
# internet, e quem tiver o segredo lê e escreve o log inteiro. Antes exigia o
# segredo E uma chave SSH, porque o hub só escutava em `127.0.0.1` e os PCs
# chegavam por túnel. A troca é deliberada: o túnel era a peça que quebrava
# calada, e um endereço só é o que faz os três aparelhos se conectarem igual.
# O Caddy não autentica mais nada, e por isso o arquivo ficou curto.
#
# Ele faz uma coisa só: termina o TLS e reparte por porta — `/sync/*` vai para o
# hub, o resto vai para o `mos-web`. Quem decide quem entra é o `mos-web`, com a
# passkey.
#
# Um efeito colateral que vale registrar, porque custou uma investigação: o
# iPhone busca o `apple-touch-icon` e o `manifest.webmanifest` ao "Adicionar à
# Tela de Início", e essa busca NÃO carrega credencial de Basic Auth. Enquanto
# havia senha aqui, ele levava 401 nos dois e não falhava visivelmente — desenhava
# um monograma com a primeira letra do título, um "M" cinza no lugar da marca. O
# sintoma não apontava para a causa em lugar nenhum. Sem Basic Auth o problema
# deixa de existir, e não por acaso: ele era filho da porta estar no lugar errado.
cat >/etc/caddy/Caddyfile <<CADDY
$DOMINIO {
	@sync path /sync/*
	handle @sync {
		reverse_proxy 127.0.0.1:9120
	}

	handle {
		reverse_proxy 127.0.0.1:9130
	}
}
CADDY
systemctl enable caddy >/dev/null 2>&1 || true
systemctl restart caddy

# --------------------------------------------------------------- 6. firewall

# A 80 não é decoração: é onde o desafio do Let's Encrypt acontece. Sem ela não
# há certificado, e sem certificado não há passkey nem push.
dizer "Firewall"
if command -v ufw >/dev/null 2>&1 && ufw status | grep -q "Status: active"; then
  ufw allow 80/tcp >/dev/null
  ufw allow 443/tcp >/dev/null
  echo "80 e 443 liberadas no ufw"
else
  echo "ufw inativo — nada a fazer aqui"
fi

# ------------------------------------------------------------ 7. conferência

dizer "Conferência"

# Dez segundos, e não três: o mos-web abre três bancos e roda migrations antes
# de escutar. O teto antigo dava "FALHOU" para um serviço que subia bem — e um
# runbook que grita falso ensina a ignorar o que ele diz.
for _ in $(seq 1 10); do
  curl -sf http://127.0.0.1:9130/api/porta/estado >/dev/null && break
  sleep 1
done

printf 'hub    ....... '
curl -sf http://127.0.0.1:9120/health >/dev/null && echo OK || echo FALHOU

# `/api/porta/estado` e nao `/api/estado`: com a porta montada, a segunda
# responde 401 para quem nao tem sessao — e o `curl -f` chamava isso de falha.
# A conferencia dizia "FALHOU" exatamente quando tudo estava certo.
PORTA="$(curl -sf http://127.0.0.1:9130/api/porta/estado || true)"
printf 'mos-web ...... '
[ -n "$PORTA" ] && echo OK || echo FALHOU

printf 'porta ........ '
case "$PORTA" in
  *'"passkey":true'*) echo "passkey compilada" ;;
  *) echo "SO o proxy (binario sem --features passkey)" ;;
esac

printf 'chave push ... '
# Esta rota exige sessao, entao a conferencia vai ao ambiente: e o que o
# servidor leu para decidir se notifica.
grep -q '^MOS_WEB_VAPID_PRIVADA=.\+' /etc/mos-web.env && echo OK ||
  echo "FALHOU — sem chave VAPID, o celular nao recebe nada"

# O icone da tela de inicio, pela porta DE FORA e sem credencial: e assim que o
# iPhone o busca. Um 401 aqui nao quebra nada visivelmente — ele so troca a
# marca por um "M" cinza, e ninguem liga uma coisa na outra meses depois.
printf 'icone PWA .... '
CODIGO="$(curl -s -o /dev/null -w '%{http_code}' "https://$DOMINIO/icone-180.png" || true)"
[ "$CODIGO" = "200" ] && echo OK ||
  echo "FALHOU ($CODIGO) — o iPhone vai desenhar um M no lugar da marca"

# A casca CARREGA — é ela que desenha a tela de entrar.
printf 'casca ........ '
CODIGO="$(curl -s -o /dev/null -w '%{http_code}' "https://$DOMINIO/" || true)"
[ "$CODIGO" = "200" ] && echo OK ||
  echo "FALHOU ($CODIGO) — sem a casca nao ha nem tela de entrar"

# E a metade que importa tanto quanto: o DADO continua fechado.
#
# Esta linha é a que prova que o guardião está de pé. Sem ela, um `mos-web`
# subindo sem sessão nenhuma — feature ausente, `MOS_WEB_INVITE` vazio — passaria
# por esta conferência com todos os OK e a API inteira aberta.
printf 'dado fechado . '
CODIGO="$(curl -s -o /dev/null -w '%{http_code}' "https://$DOMINIO/api/tasks" || true)"
[ "$CODIGO" = "401" ] && echo OK ||
  echo "FALHOU ($CODIGO) — /api/tasks devia exigir sessao. NAO deixe assim."

dizer "Pronto"
cat <<FIM

  Abra no iPhone:   https://$DOMINIO

  O certificado pode levar até um minuto na primeira vez. Se der erro de TLS,
  espere e recarregue — e confira com:

      journalctl -u caddy -n 30 --no-pager

  ------------------------------------------------------------------
  O CONVITE. É ele que registra um aparelho novo — depois disso, quem
  abre a porta é o Face ID, e não uma senha digitada:

      $CONVITE

  ------------------------------------------------------------------
  O segredo do hub, para o M/OS do PC (Settings -> SINCRONIZAÇÃO):

      $TOKEN

  ------------------------------------------------------------------
  A porta de emergência, se a passkey falhar num aparelho novo:

      sudo python3 deploy/voltar-senha.py

  Ela repõe o Basic Auth no Caddy com esta senha, já sorteada:

      usuário: ${MOS_USUARIO:-matheus}
      senha:   $SENHA
  ------------------------------------------------------------------

  Estão em /etc/mos-web.env e /etc/mos-proxy.env, modo 600. Não precisam ser
  anotados em lugar nenhum além do seu gerenciador de senhas — e rodar este
  script de novo NÃO os regenera.

FIM
