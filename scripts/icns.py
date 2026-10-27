# -*- coding: utf-8 -*-
"""Monta um .icns a mao, a partir de um PNG por tamanho.

Compartilhado pelos dois geradores (`gerar-icones.py` e
`generate-family-icons.py`). O `save` do Pillow em ICNS reduz UMA imagem grande
para todos os tamanhos — o escalonamento que a regra do simbolo proibe. Aqui
cada bloco recebe o PNG desenhado no proprio tamanho.

Era a falta deste arquivo que deixava o `icon.icns` do M/OS com o logo padrao do
Tauri e o do CronoCAD com a arte antiga: nenhum gerador escrevia .icns.
"""
import struct

# Tipo de bloco → lado do PNG. Os `@2x` (ic11–ic14) sao o tamanho de pixel, e
# por isso reaproveitam o desenho daquele lado.
BLOCOS = [
    ("icp4", 16),
    ("icp5", 32),
    ("icp6", 64),
    ("ic07", 128),
    ("ic08", 256),
    ("ic09", 512),
    ("ic10", 1024),
    ("ic11", 32),
    ("ic12", 64),
    ("ic13", 256),
    ("ic14", 512),
]

TAMANHOS = sorted({lado for _, lado in BLOCOS})


def escrever_icns(caminho, pngs):
    """`pngs`: {lado: bytes do PNG}. Precisa de todos os `TAMANHOS`."""
    faltam = [lado for lado in TAMANHOS if lado not in pngs]
    if faltam:
        raise SystemExit("icns sem os tamanhos %s" % faltam)
    corpo = b""
    for tipo, lado in BLOCOS:
        dados = pngs[lado]
        corpo += tipo.encode("ascii") + struct.pack(">I", len(dados) + 8) + dados
    with open(caminho, "wb") as arquivo:
        arquivo.write(b"icns" + struct.pack(">I", len(corpo) + 8) + corpo)
