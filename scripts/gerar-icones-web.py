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

FUNDO = (10, 12, 14, 255)      # #0A0C0E — o mesmo `background_color` do manifest
SODIO = (231, 194, 78, 255)    # #E7C24E

# O traco do `icone.svg`, no viewBox de 64: M18 44 V20 l14 13 l14 -13 v24.
CAMINHO = [(18, 44), (18, 20), (32, 33), (46, 20), (46, 44)]
TRACO = 5

SUPER = 16
FILTRO = Image.BOX

# 180 e o que o iOS pede no `apple-touch-icon`; 192 e o que a notificacao usa
# como icone e badge; 512 e o que o `manifest` pede para o resto.
TAMANHOS = (180, 192, 512)


def desenhar(tamanho):
    lado = tamanho * SUPER
    escala = lado / 64.0
    imagem = Image.new("RGBA", (lado, lado), FUNDO)
    pincel = ImageDraw.Draw(imagem)
    pincel.line(
        [(x * escala, y * escala) for x, y in CAMINHO],
        fill=SODIO,
        width=int(TRACO * escala),
        # `curve` arredonda as junções. No SVG elas são `miter`, mas a diferença
        # entre as duas some abaixo de um pixel no tamanho final — e uma junção
        # sem tratamento nenhum deixa um entalhe visível no vértice do M.
        joint="curve",
    )
    return imagem.resize((tamanho, tamanho), FILTRO)


def main():
    for tamanho in TAMANHOS:
        caminho = os.path.join(DESTINO, "icone-%d.png" % tamanho)
        desenhar(tamanho).convert("RGB").save(caminho, "PNG", optimize=True)
        print("gerado", os.path.relpath(caminho, RAIZ))


if __name__ == "__main__":
    main()
