# -*- coding: utf-8 -*-
"""
Gera os icones do M/OS respeitando a regra optica do `Symbol.tsx`.

O `tauri icon` produz todos os tamanhos reduzindo UMA arte grande. Isso contraria
a regra que o proprio simbolo do sistema fixa:

    "Escalar um unico SVG entre tamanhos e proibido pelo handoff, e o motivo e
     optico: a mesma inclinacao geometrica le mais fina conforme o desenho
     encolhe, entao o angulo abre para compensar."

Sao tres desenhos, e cada tamanho usa o seu. Este script desenha cada arquivo no
seu tamanho final, com supersampling, em vez de reduzir a versao grande.

Rodar:  python scripts/gerar-icones.py
"""
import io
import os
import struct

from PIL import Image, ImageDraw

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICONES = os.path.join(RAIZ, "apps", "desktop", "src-tauri", "icons")

# Os tres poligonos do `Symbol.tsx`, no mesmo viewBox de 64.
BARRAS = {
    "large": [(38, 8), (53, 8), (26, 56), (11, 56)],   # 22 graus — 128 para cima
    "medium": [(40, 10), (54, 10), (24, 54), (10, 54)],  # 18 graus — 48 a 127
    "small": [(42, 12), (56, 12), (22, 52), (8, 52)],   # 14 graus — abaixo de 48
}

SODIO = (231, 194, 78, 255)     # --signal-fill
TINTA = (10, 12, 14, 255)       # --on-signal
RAIO = 0.21                     # --rail-symbol-radius sobre --rail-symbol

# Desenha 16x maior e reduz: a arte nasce no tamanho certo, so o pixel e suavizado.
SUPER = 16


def barra_para(tamanho):
    if tamanho >= 128:
        return BARRAS["large"]
    if tamanho >= 48:
        return BARRAS["medium"]
    return BARRAS["small"]


def desenhar(tamanho):
    lado = tamanho * SUPER
    imagem = Image.new("RGBA", (lado, lado), (0, 0, 0, 0))
    pincel = ImageDraw.Draw(imagem)
    pincel.rounded_rectangle([0, 0, lado - 1, lado - 1], radius=int(lado * RAIO), fill=SODIO)
    escala = lado / 64.0
    pincel.polygon([(x * escala, y * escala) for x, y in barra_para(tamanho)], fill=TINTA)
    return imagem.resize((tamanho, tamanho), Image.LANCZOS)


def escrever_ico(caminho, tamanhos):
    """Monta o .ico a mao: o `save` do Pillow reamostra a partir de uma imagem so,
    que e exatamente o que este script existe para evitar."""
    imagens = []
    for tamanho in tamanhos:
        buffer = io.BytesIO()
        desenhar(tamanho).save(buffer, format="PNG")
        imagens.append((tamanho, buffer.getvalue()))

    cabecalho = struct.pack("<HHH", 0, 1, len(imagens))
    deslocamento = 6 + 16 * len(imagens)
    entradas, corpos = b"", b""
    for tamanho, dados in imagens:
        largura = 0 if tamanho >= 256 else tamanho
        entradas += struct.pack("<BBBBHHII", largura, largura, 0, 0, 1, 32, len(dados), deslocamento)
        corpos += dados
        deslocamento += len(dados)

    with open(caminho, "wb") as arquivo:
        arquivo.write(cabecalho + entradas + corpos)
    return [t for t, _ in imagens]


PNGS = {
    "32x32.png": 32,
    "128x128.png": 128,
    "128x128@2x.png": 256,
    "icon.png": 512,
    "Square30x30Logo.png": 30,
    "Square44x44Logo.png": 44,
    "Square71x71Logo.png": 71,
    "Square89x89Logo.png": 89,
    "Square107x107Logo.png": 107,
    "Square142x142Logo.png": 142,
    "Square150x150Logo.png": 150,
    "Square284x284Logo.png": 284,
    "Square310x310Logo.png": 310,
    "StoreLogo.png": 50,
}

if __name__ == "__main__":
    for nome, tamanho in sorted(PNGS.items(), key=lambda item: item[1]):
        desenhar(tamanho).save(os.path.join(ICONES, nome))
        print("%-24s %4dpx  %s" % (nome, tamanho, "small" if tamanho < 48 else "medium" if tamanho < 128 else "large"))

    tamanhos = escrever_ico(os.path.join(ICONES, "icon.ico"), [16, 24, 32, 48, 64, 128, 256])
    print("%-24s %s" % ("icon.ico", tamanhos))
