# -*- coding: utf-8 -*-
"""
Gera os PNGs da superficie de bolso, a partir da MESMA geometria do `icone.svg`.

Rodar:  python scripts/gerar-icones-web.py

# Por que PNG, se ja existe o SVG

Porque o iOS **ignora SVG em `apple-touch-icon`**. Sem um PNG, o icone da tela
de inicio vira um print da propria pagina — e o app fica com cara de atalho de
navegador, que e exatamente o oposto do que "instalar no celular" deveria
parecer. O `manifest` aceita SVG e continua com ele; o iPhone e que nao.

# Por que quadrado cheio, e nao com cantos arredondados

O iOS arredonda o icone SOZINHO, por cima do que voce der. Um PNG que ja chega
arredondado, com cantos transparentes, ganha cantos PRETOS depois da mascara do
sistema — o desenho fica com uma moldura escura que ninguem desenhou.

# Por que nao reduzir uma arte grande

Mesma disciplina do `gerar-icones.py`: cada tamanho e desenhado no proprio
tamanho, com supersampling de 16x e reducao por BOX — que e a media exata da
area, e portanto o antialiasing correto para este caso. LANCZOS tem lobulos
negativos e produz halo claro em borda dura.
"""
import os

from PIL import Image, ImageDraw

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DESTINO = os.path.join(RAIZ, "apps", "mos-web", "ui", "public")

SODIO = (231, 194, 78, 255)    # #E7C24E — o campo
TINTA = (10, 12, 14, 255)      # #0A0C0E — a barra

# Os tres poligonos do brief, no viewBox de 64 — os MESMOS de
# `scripts/gerar-icones.py`. Aqui so o "large" e usado (180, 192 e 512 estao
# todos acima de 128), e os outros dois ficam para o dia em que este script
# gerar um favicon pequeno: apagar geometria correta para reescreve-la depois e
# o jeito de ela voltar diferente.
BARRAS = {
    "large": [(38, 8), (53, 8), (26, 56), (11, 56)],   # 22 graus
    "medium": [(40, 10), (54, 10), (24, 54), (10, 54)],  # 18 graus
    "small": [(42, 12), (56, 12), (22, 52), (8, 52)],   # 14 graus
}

SUPER = 16
FILTRO = Image.BOX

# 180 e o que o iOS pede no `apple-touch-icon`; 192 e o que a notificacao usa
# como icone e badge; 512 e o que o `manifest` pede para o resto.
TAMANHOS = (180, 192, 512)


def barra_para(tamanho):
    if tamanho >= 128:
        return BARRAS["large"]
    if tamanho >= 48:
        return BARRAS["medium"]
    return BARRAS["small"]


def desenhar(tamanho):
    lado = tamanho * SUPER
    escala = lado / 64.0
    # Quadrado CHEIO de sodio, sem cantos arredondados: o iOS arredonda sozinho
    # por cima. Um PNG que ja chega arredondado ganha cantos PRETOS depois da
    # mascara do sistema — moldura escura que ninguem desenhou.
    imagem = Image.new("RGBA", (lado, lado), SODIO)
    pincel = ImageDraw.Draw(imagem)
    pincel.polygon(
        [(x * escala, y * escala) for x, y in barra_para(tamanho)],
        fill=TINTA,
    )
    return imagem.resize((tamanho, tamanho), FILTRO)


def main():
    for tamanho in TAMANHOS:
        caminho = os.path.join(DESTINO, "icone-%d.png" % tamanho)
        desenhar(tamanho).convert("RGB").save(caminho, "PNG", optimize=True)
        print("gerado", os.path.relpath(caminho, RAIZ))


if __name__ == "__main__":
    main()
