#!/usr/bin/env bash
#
# Tira o Basic Auth da frente e deixa a passkey ser a porta.
#
#     ssh -t hermes@167.233.43.1 'sudo bash /tmp/mos/abrir-para-passkey.sh'
#
# # Por que isto é um script separado, e não um passo do bootstrap
#
# Porque ele só pode rodar depois de existir um aparelho registrado — e o
# bootstrap roda antes de qualquer aparelho existir.
#
# A ordem importa e não é negociável:
#
#   1. o bootstrap sobe tudo com Basic Auth na frente;
#   2. você entra com usuário e senha, e registra a passkey com o convite;
#   3. este script tira o Basic Auth.
#
# Inverter 2 e 3 tranca você do lado de fora: sem Basic Auth e sem passkey
# registrada, não sobra nenhuma forma de entrar — e a única saída seria voltar
# aqui por SSH. Por isso ele **confere** antes, e recusa se não houver aparelho.

set -euo pipefail

DOMINIO="${MOS_DOMINIO:-167-233-43-1.sslip.io}"

if [ "$(id -u)" -ne 0 ]; then
  echo "Este script precisa de root: sudo bash $0" >&2
  exit 1
fi

echo "== Conferindo se dá para fechar a porta antiga"

ESTADO="$(curl -sf http://127.0.0.1:9130/api/porta/estado || true)"
if [ -z "$ESTADO" ]; then
  echo "O mos-web não respondeu em 127.0.0.1:9130. Ele está no ar?" >&2
  exit 1
fi
echo "  $ESTADO"

case "$ESTADO" in
  *'"passkey":true'*) ;;
  *)
    echo >&2
    echo "Este binário não tem a cerimônia WebAuthn compilada." >&2
    echo "Instale o mos-web construído com --features passkey antes." >&2
    exit 1
    ;;
esac

case "$ESTADO" in
  *'"registrado":true'*) ;;
  *)
    echo >&2
    echo "NENHUM aparelho registrado ainda." >&2
    echo >&2
    echo "Tirar o Basic Auth agora trancaria você do lado de fora: não sobraria" >&2
    echo "porta nenhuma. Entre no app com usuário e senha, registre a passkey" >&2
    echo "com o convite de /etc/mos-web.env, e rode isto de novo." >&2
    exit 1
    ;;
esac

echo "== Tirando o Basic Auth"

# A declaração some junto com a porta externa. Deixá-la seria dizer ao binário
# que há autenticação no proxy quando não há mais — e o guardião do `main.rs`
# passaria a acreditar numa porta que não existe.
sed -i '/^MOS_WEB_PORTA_EXTERNA=/d' /etc/mos-web.env

cat >/etc/caddy/Caddyfile <<CADDY
$DOMINIO {
	reverse_proxy 127.0.0.1:9130
}
CADDY

systemctl restart mos-web
systemctl reload caddy || systemctl restart caddy

sleep 3
printf 'mos-web ...... '
curl -sf http://127.0.0.1:9130/api/porta/estado >/dev/null && echo OK || echo FALHOU

# A senha continua no arquivo, e não é esquecimento: ela é o caminho de volta se
# a passkey se perder junto com o aparelho. Repô-la é reescrever o Caddyfile.
echo
echo "Pronto. A senha antiga continua em /etc/mos-proxy.env — é o caminho de"
echo "volta se o aparelho com a passkey se perder."
echo
echo "Abra https://$DOMINIO pelo ícone na tela de início. Agora é Face ID."
