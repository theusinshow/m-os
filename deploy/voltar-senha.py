#!/usr/bin/env python3
"""Repoe o Basic Auth na frente do mos-web. A porta de emergencia.

Existe para UM caso: a passkey parou de abrir — aparelho novo, credencial
perdida, Face ID recusando — e voce precisa entrar AGORA. Ela nao desliga a
passkey; poe uma segunda porta na frente dela, que e como a VPS viveu ate
10/09/2026.

A senha e a que ja esta em `/etc/mos-proxy.env`, sorteada pelo bootstrap. Nao ha
o que digitar nem decorar: rode e leia.
"""
import pathlib, re, shutil, subprocess, sys, time

CADDYFILE = pathlib.Path("/etc/caddy/Caddyfile")
PROXY_ENV = pathlib.Path("/etc/mos-proxy.env")
WEB_ENV = pathlib.Path("/etc/mos-web.env")
CARIMBO = time.strftime("%Y%m%d-%H%M%S")

if not PROXY_ENV.is_file():
    sys.exit("/etc/mos-proxy.env nao existe — rode o bootstrap-vps.sh para gerar a senha")
env = PROXY_ENV.read_text()
usuario = (re.search(r"^USUARIO=(.*)$", env, re.M) or [None, "matheus"])[1].strip()
achado = re.search(r"^SENHA=(.*)$", env, re.M)
if not achado or not achado.group(1).strip():
    sys.exit("/etc/mos-proxy.env sem SENHA")
senha = achado.group(1).strip()

texto = CADDYFILE.read_text()
if "basic_auth" in texto:
    print("o Caddyfile ja tem `basic_auth` — nada a fazer")
else:
    # O `handle` SEM matcher e o que atende tudo que nao e `/sync/*`. E dentro
    # dele que a senha entra, para o hub continuar de fora: o cliente de sync
    # manda `Authorization: Bearer`, e o Basic Auth recusaria antes de o hub ver.
    alvo = re.search(r"^([ \t]*)handle \{$", texto, re.M)
    if not alvo:
        sys.exit("nao achei o `handle {` sem matcher no Caddyfile")
    recuo = alvo.group(1)
    hashed = subprocess.run(
        ["caddy", "hash-password", "--plaintext", senha],
        capture_output=True, text=True, check=True,
    ).stdout.strip()
    bloco = (
        f"{recuo}handle {{\n"
        f"{recuo}\tbasic_auth {{\n"
        f"{recuo}\t\t{usuario} {hashed}\n"
        f"{recuo}\t}}"
    )
    backup = CADDYFILE.with_suffix(".bak-" + CARIMBO)
    shutil.copy2(CADDYFILE, backup)
    CADDYFILE.write_text(texto[: alvo.start()] + bloco + texto[alvo.end():])
    conferencia = subprocess.run(
        ["caddy", "validate", "--config", str(CADDYFILE), "--adapter", "caddyfile"],
        capture_output=True, text=True,
    )
    if conferencia.returncode != 0:
        shutil.copy2(backup, CADDYFILE)
        sys.exit("caddy validate recusou — revertido.\n" + conferencia.stderr[-1500:])
    subprocess.run(["systemctl", "reload", "caddy"], check=True)
    print("basic_auth reposto; backup em", backup)

# E o binario volta a saber que ha porta na frente. Sem isto ele nao muda de
# comportamento — mas o proximo restart dele passa a bater com a realidade.
web = WEB_ENV.read_text()
if "MOS_WEB_PORTA_EXTERNA=1" not in web:
    shutil.copy2(WEB_ENV, WEB_ENV.with_suffix(".bak-" + CARIMBO))
    WEB_ENV.write_text(web.rstrip("\n") + "\nMOS_WEB_PORTA_EXTERNA=1\n")
    WEB_ENV.chmod(0o600)
    print("MOS_WEB_PORTA_EXTERNA=1 reposto")

time.sleep(2)
codigo = subprocess.run(
    ["curl", "-s", "-o", "/dev/null", "-w", "%{http_code}", "https://167-233-43-1.sslip.io/"],
    capture_output=True, text=True,
).stdout.strip()
print(f"\nraiz sem senha: {codigo}   (esperado 401)")
print(f"\n  usuario: {usuario}\n  senha:   {senha}")
