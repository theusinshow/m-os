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
import math
import os
import struct

from PIL import Image, ImageDraw

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DESTINO = os.path.join(RAIZ, "apps", "mos-web", "ui", "public")

# Os valores sao Display-P3, e nao sRGB: o PNG sai com o perfil P3 embutido
# (`perfil_display_p3`). Em sRGB, o #E7C24E do desktop parecia "ovo envelhecido"
# no iPhone, que gerencia cor e mostra o hex como ele e — ver ADR-090. Este e o
# mesmo tom, com a saturacao escolhida no proprio iPhone: p3(0.96 0.79 0.20).
SODIO = (245, 201, 51, 255)    # p3(0.96 0.79 0.20) — o campo
TINTA = (10, 12, 14, 255)      # #0A0C0E — a barra; no escuro P3 e sRGB coincidem

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


def perfil_display_p3():
    """Um perfil ICC v2 de Display P3, montado a mao.

    O Pillow so sabe criar sRGB, e o Windows nao traz um P3 no sistema. Um
    perfil de monitor por matriz e pequeno e fixo: tres primarias e o ponto
    branco (adaptados a D50, como o ICC exige) e a curva do sRGB, que e a mesma
    do P3. Os numeros sao os do perfil Display P3 da Apple.
    """

    def s15(valor):
        return struct.pack(">i", round(valor * 65536))

    def xyz(x, y, z):
        return b"XYZ " + b"\0" * 4 + s15(x) + s15(y) + s15(z)

    nome = b"Display P3 (M/OS)"
    desc = (
        b"desc" + b"\0" * 4 + struct.pack(">I", len(nome) + 1) + nome + b"\0"
        + b"\0" * 8 + b"\0" * 3 + b"\0" * 67
    )
    cprt = b"text" + b"\0" * 4 + b"Public domain\0"

    def srgb(v):
        return v / 12.92 if v <= 0.04045 else math.pow((v + 0.055) / 1.055, 2.4)

    pontos = 1024
    curv = b"curv" + b"\0" * 4 + struct.pack(">I", pontos) + b"".join(
        struct.pack(">H", round(srgb(i / (pontos - 1)) * 65535)) for i in range(pontos)
    )

    etiquetas = [
        (b"desc", desc),
        (b"cprt", cprt),
        (b"wtpt", xyz(0.964203, 1.0, 0.824905)),
        (b"rXYZ", xyz(0.515121, 0.241196, -0.001053)),
        (b"gXYZ", xyz(0.291977, 0.692245, 0.041885)),
        (b"bXYZ", xyz(0.157104, 0.066574, 0.784073)),
        (b"rTRC", curv),
        (b"gTRC", curv),
        (b"bTRC", curv),
    ]

    tabela = struct.pack(">I", len(etiquetas))
    corpo = b""
    deslocamento = 128 + 4 + 12 * len(etiquetas)
    for assinatura, dado in etiquetas:
        tabela += assinatura + struct.pack(">II", deslocamento + len(corpo), len(dado))
        corpo += dado + b"\0" * (-len(dado) % 4)

    tamanho = 128 + len(tabela) + len(corpo)
    cabecalho = (
        struct.pack(">I", tamanho) + b"\0" * 4 + struct.pack(">I", 0x02100000)
        + b"mntr" + b"RGB " + b"XYZ " + b"\0" * 12 + b"acsp" + b"\0" * 4
        + b"\0" * 4 + b"\0" * 8 + b"\0" * 8 + b"\0" * 4
        + s15(0.9642) + s15(1.0) + s15(0.8249) + b"\0" * 4 + b"\0" * 16 + b"\0" * 28
    )
    assert len(cabecalho) == 128
    return cabecalho + tabela + corpo


def main():
    perfil = perfil_display_p3()
    # O 180 sai tambem como `apple-touch-icon.png`: e o caminho que o iOS tenta
    # por conta propria quando nao le o `<link>` da pagina.
    saidas = [(t, "icone-%d.png" % t) for t in TAMANHOS] + [(180, "apple-touch-icon.png")]
    for tamanho, nome in saidas:
        caminho = os.path.join(DESTINO, nome)
        desenhar(tamanho).convert("RGB").save(
            caminho, "PNG", optimize=True, icc_profile=perfil
        )
        print("gerado", os.path.relpath(caminho, RAIZ))


if __name__ == "__main__":
    main()
