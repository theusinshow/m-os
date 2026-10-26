#!/usr/bin/env python3
"""A senha HTTP na frente do mos-web, agora no Traefik do Dokploy.

    sudo python3 senha-traefik.py tirar   # a porta passa a ser so a passkey
    sudo python3 senha-traefik.py voltar  # a porta de emergencia volta

# Por que isto existe

Desde 22/10/2026 quem atende a 443 e o Traefik do Dokploy, e o Caddy esta
parado — `tirar-senha.py` e `voltar-senha.py` mexem num arquivo que ninguem mais
le. A migracao copiou o `basic_auth` do Caddy como o middleware `mos-auth`, e com
ele o defeito que o Caddy ja tinha: a PWA instalada no iPhone NAO guarda Basic
Auth, entao pede a senha a cada abertura, e o iOS busca o icone sem credencial e
desenha um quadrado preto.

A passkey ja esta compilada, configurada e com aparelho registrado
(`/api/porta/estado`), e a sessao dela dura 30 dias. A senha HTTP so empilha uma
segunda porta na frente da que vale.

# O que ele NAO toca

A definicao do `mos-auth` fica no arquivo: `voltar` so pendura ela de novo no
roteador. O `mos-sync` e o `hermes` nao mudam. O Traefik relê o diretorio
sozinho (`providers.file.watch`), entao nao ha restart.
"""
import pathlib, shutil, subprocess, sys, time

ROTAS = pathlib.Path("/etc/dokploy/traefik/dynamic/mos.yml")
BASE = "https://167-233-43-1.sslip.io"
INTERNO = "http://172.17.0.1:9130"
CARIMBO = time.strftime("%Y%m%d-%H%M%S")

# O roteador do mos-web, com e sem a senha. Texto exato, e nao YAML parseado:
# reescrever o arquivo inteiro por um parser perderia o formato que o Dokploy
# escreveu, e uma troca de linha que nao acha o alvo falha alto em vez de
# adivinhar.
SEM = """      service: mos-web
      priority: 100
"""
COM = """      service: mos-web
      middlewares: [mos-auth]
      priority: 100
"""


def status(url):
    return subprocess.run(
        ["curl", "-s", "-o", "/dev/null", "-m", "8", "-w", "%{http_code}", url],
        capture_output=True, text=True,
    ).stdout.strip()


def trocar(de, para):
    texto = ROTAS.read_text()
    if texto.count(de) != 1:
        sys.exit(f"nao achei o roteador do mos-web do jeito esperado em {ROTAS} — nada mudou")
    copia = ROTAS.with_name(ROTAS.name + ".bak-" + CARIMBO)
    shutil.copy2(ROTAS, copia)
    ROTAS.write_text(texto.replace(de, para))
    print("backup em", copia)
    time.sleep(3)  # o watch do Traefik
    return copia


def provar(esperado_raiz):
    raiz, dado, icone = status(BASE + "/"), status(BASE + "/api/tasks"), status(BASE + "/icone-180.png")
    print(f"casca  (/)            {raiz}   esperado {esperado_raiz}")
    print(f"dado   (/api/tasks)   {dado}   esperado 401")
    print(f"icone  (/icone-180)   {icone}   esperado 200")
    return raiz, dado, icone


if len(sys.argv) != 2 or sys.argv[1] not in ("tirar", "voltar"):
    sys.exit(__doc__)

if sys.argv[1] == "tirar":
    estado = subprocess.run(
        ["curl", "-sf", "-m", "8", INTERNO + "/api/porta/estado"],
        capture_output=True, text=True,
    ).stdout
    print("porta interna:", estado or "(sem resposta)")
    for exigido in ('"passkey":true', '"porta":true', '"registrado":true'):
        if exigido not in estado:
            sys.exit(f"FALTA {exigido} — sem porta interna, tirar a senha abriria o M/OS inteiro. Nada mudou.")

    if SEM in ROTAS.read_text() and COM not in ROTAS.read_text():
        print("o roteador ja esta sem a senha")
    else:
        copia = trocar(COM, SEM)
        raiz, dado, _ = provar("200")
        # O que importa e o DADO fechado. Se a API respondeu qualquer coisa que
        # nao 401, a porta interna nao esta guardando — volta na hora.
        if dado != "401":
            shutil.copy2(copia, ROTAS)
            sys.exit("!! /api respondeu sem sessao — a senha foi reposta. Nada ficou aberto.")
        if raiz != "200":
            print("!! a casca nao carregou ainda — confira em alguns segundos")
    print("\nA porta agora e a passkey. Se o Face ID nao abrir: sudo python3 senha-traefik.py voltar")
else:
    if COM in ROTAS.read_text():
        print("o roteador ja tem a senha")
    else:
        trocar(SEM, COM)
    provar("401")
    print("\nA senha esta de volta na frente do mos-web.")
