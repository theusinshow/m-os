#!/usr/bin/env bash
#
# Troca o convite por um que dê para digitar num teclado de celular.
#
#     ssh -t hermes@167.233.43.1 'sudo bash /tmp/mos/convite-novo.sh'
#
# # Por que o convite do bootstrap é ruim de digitar
#
# `openssl rand -base64 24` produz 32 caracteres com maiúscula, minúscula, `+`,
# `/` e `=`. No teclado do iPhone isso são três trocas de teclado e dois
# caracteres que ficam escondidos — e o erro de digitação aparece como "convite
# inválido", que é indistinguível do convite errado.
#
# Aqui: 16 caracteres em quatro grupos, de um alfabeto sem `0`, `o`, `1`, `l` e
# `i` — os cinco que se confundem lidos de um terminal e digitados de memória.
# São 32^16, ou 80 bits. Para um segredo que só serve para registrar um
# aparelho, e que se troca com este mesmo script, é folgado.
#
# # Trocar o convite é seguro
#
# Ele não autentica ninguém: ele autoriza um registro. Aparelhos já registrados
# continuam entrando com a passkey — o convite não é pedido de novo. O que o
# convite antigo perde é a capacidade de registrar um aparelho novo, que é
# exatamente o que se quer ao trocá-lo.

set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
  echo "Este script precisa de root: sudo bash $0" >&2
  exit 1
fi

if [ ! -f /etc/mos-web.env ]; then
  echo "Nao achei /etc/mos-web.env. Rode o bootstrap-vps.sh antes." >&2
  exit 1
fi

# `tr -dc` sobre bytes aleatórios, e não `$RANDOM`: o shell não tem gerador
# criptográfico, e um segredo sorteado com gerador previsível é um segredo
# adivinhável.
ALFABETO='abcdefghjkmnpqrstuvwxyz23456789'
grupo() { head -c 64 /dev/urandom | tr -dc "$ALFABETO" | head -c 4; }
CONVITE="$(grupo)-$(grupo)-$(grupo)-$(grupo)"

sed -i "s|^MOS_WEB_INVITE=.*|MOS_WEB_INVITE=$CONVITE|" /etc/mos-web.env
grep -q '^MOS_WEB_INVITE=' /etc/mos-web.env ||
  echo "MOS_WEB_INVITE=$CONVITE" >>/etc/mos-web.env

systemctl restart mos-web
sleep 2
printf 'mos-web ...... '
curl -sf http://127.0.0.1:9130/api/porta/estado >/dev/null && echo OK || echo FALHOU

cat <<FIM

  O convite novo:

      $CONVITE

  Digite no app, em Registrar. Os hifens fazem parte.

FIM
