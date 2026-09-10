#!/usr/bin/env python3
"""Tira o Basic Auth da frente do mos-web. A porta passa a ser a passkey.

Ordem importa, e ela e o que torna isto seguro: o `mos-web` so sobe publicado se
tiver porta — ou a INTERNA (passkey compilada + `MOS_WEB_INVITE`) ou a externa
declarada por `MOS_WEB_PORTA_EXTERNA=1`. Tirar a declaracao da externa ANTES de
saber que a interna existe derrubaria o servico. Entao aqui a ordem e:

    1. conferir que a porta interna responde  (/api/porta/estado)
    2. tirar `MOS_WEB_PORTA_EXTERNA` e reiniciar o mos-web
    3. so entao tirar o `basic_auth` do Caddy

Cada passo confere o anterior, e QUALQUER falha reverte tudo. Ver
`voltar-senha.py` para o caminho de volta feito a mao.
"""
import pathlib, re, shutil, subprocess, sys, time

CADDYFILE = pathlib.Path("/etc/caddy/Caddyfile")
WEB_ENV = pathlib.Path("/etc/mos-web.env")
CARIMBO = time.strftime("%Y%m%d-%H%M%S")


def curl(url, *extra):
    return subprocess.run(
        ["curl", "-s", "-o", "/dev/null", "-w", "%{http_code}", *extra, url],
        capture_output=True, text=True,
    ).stdout.strip()


def json_local(caminho):
    return subprocess.run(
        ["curl", "-sf", "-m", "8", "http://127.0.0.1:9130" + caminho],
        capture_output=True, text=True,
    ).stdout


# --- 1. a porta interna existe? ---------------------------------------------
estado = json_local("/api/porta/estado")
print("porta interna:", estado or "(sem resposta)")
for exigido in ('"passkey":true', '"porta":true', '"registrado":true'):
    if exigido not in estado:
        sys.exit(
            f"FALTA {exigido} — nao ha porta interna de verdade.\n"
            "Registre a passkey do aparelho ANTES de tirar a senha, ou voce fica de fora."
        )

# --- 2. o binario passa a exigir a porta dele -------------------------------
env_texto = WEB_ENV.read_text()
env_backup = WEB_ENV.with_suffix(".bak-" + CARIMBO)
shutil.copy2(WEB_ENV, env_backup)
novo_env = re.sub(r"^MOS_WEB_PORTA_EXTERNA=.*\n?", "", env_texto, flags=re.M)
WEB_ENV.write_text(novo_env)
WEB_ENV.chmod(0o600)
print("MOS_WEB_PORTA_EXTERNA removido; backup em", env_backup)

subprocess.run(["systemctl", "restart", "mos-web"], check=True)
time.sleep(5)
if not json_local("/api/porta/estado"):
    shutil.copy2(env_backup, WEB_ENV)
    subprocess.run(["systemctl", "restart", "mos-web"], check=False)
    sys.exit(
        "o mos-web NAO subiu sem a declaracao de porta externa — revertido.\n"
        "Veja: sudo journalctl -u mos-web -n 30 --no-pager"
    )
print("mos-web subiu com a porta interna")

# --- 3. e a senha sai da frente ---------------------------------------------
texto = CADDYFILE.read_text()
bloco = re.search(r"\n[ \t]*basic_auth \{\n(?:[^\n]*\n)*?[ \t]*\}\n", texto)
if not bloco:
    print("o Caddyfile ja nao tem `basic_auth` — nada a fazer nele")
else:
    caddy_backup = CADDYFILE.with_suffix(".bak-" + CARIMBO)
    shutil.copy2(CADDYFILE, caddy_backup)
    CADDYFILE.write_text(texto[: bloco.start()] + "\n" + texto[bloco.end():])
    conferencia = subprocess.run(
        ["caddy", "validate", "--config", str(CADDYFILE), "--adapter", "caddyfile"],
        capture_output=True, text=True,
    )
    if conferencia.returncode != 0:
        shutil.copy2(caddy_backup, CADDYFILE)
        sys.exit("caddy validate recusou — revertido.\n" + conferencia.stderr[-1500:])
    subprocess.run(["systemctl", "reload", "caddy"], check=True)
    print("basic_auth removido; backup em", caddy_backup)

# --- a prova ----------------------------------------------------------------
time.sleep(2)
base = "https://167-233-43-1.sslip.io"
casca = curl(base + "/")
dado = curl(base + "/api/tasks")
icone = curl(base + "/icone-180.png")
print(f"\ncasca  (/)            {casca}   esperado 200")
print(f"dado   (/api/tasks)   {dado}   esperado 401")
print(f"icone  (/icone-180)   {icone}   esperado 200")

if dado != "401":
    print("\n!! O DADO NAO ESTA FECHADO. Rode agora:")
    print("   sudo python3 /home/hermes/deploy/voltar-senha.py")
    sys.exit(1)
if casca != "200":
    print("\n!! A casca nao carrega — sem ela nao ha tela de entrar.")
    sys.exit(1)
print("\nA porta agora e a passkey.")
