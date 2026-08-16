"""Gera os icones dos Apps irmaos a partir da folha `Marcas v0.3`.

O M/OS tem gerador proprio (`generate-icons.py`): a barra e poligono desenhado
a mao, com tres inclinacoes distintas, e nao depende de fonte nenhuma. Os
outros tres usam glifos do Material Symbols, e e isso que este script monta.

A folha e explicita quanto a um ponto: a FONTE NAO ENTRA NO EXECUTAVEL. Cada
glifo entra como SVG ja congelado nos eixos certos, versionado em
`packages/design-system/marks/`. Buscar da rede em tempo de build faria o icone
depender do Google estar no ar, e mudaria de forma sem ninguem ter pedido.

Uso:
    python scripts/generate-family-icons.py

Requer Pillow (para o .ico) e `resvg` no PATH (`cargo install resvg`).
"""

from __future__ import annotations

import re
import subprocess
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
MARKS = ROOT / "packages/design-system/marks"

FIELD = "#E7C24E"
INK_DARK = "#0A0C0E"

# A receita da folha, linha por linha: lado, raio (18%), fracao do glifo e a
# variante de eixo. FILL vira 1 de 32 para baixo — abaixo disso o traco vazado
# fecha e vira mancha. O peso sobe junto, pelo mesmo motivo.
RECIPE = {
    1024: (184, 0.64, "wght300"),
    512: (92, 0.64, "wght300"),
    256: (46, 0.64, "wght300"),
    128: (23, 0.65, "wght300"),
    64: (11, 0.66, "default"),
    48: (9, 0.67, "wght500"),
    32: (6, 0.69, "wght500fill1"),
    24: (4, 0.71, "wght600fill1"),
    16: (3, 0.81, "wght700fill1"),
    # Fora da folha: 180 e o tamanho do icone de tela inicial do iOS. Fica entre
    # 128 e 256, que compartilham o mesmo peso, entao so o raio e a fracao sao
    # derivados — 18% de 180 da 32, e a fracao interpola para 0.65.
    180: (32, 0.65, "wght300"),
}

GLYPHS = {
    "cronocad": "timer",
    "m-finance": "money",
    "coded-atlas": "screenshot_monitor",
}

PATH_ELEMENT = re.compile(r"<path\b[^>]*/>")


def glyph_paths(glyph: str, variant: str) -> str:
    """Extrai so os `<path>` do arquivo do Material Symbols.

    O SVG baixado traz `width`/`height`/`viewBox` proprios. Reaproveitar o
    elemento inteiro arrastaria essas medidas para dentro do ladrilho.
    """
    source = MARKS / f"{glyph}-{variant}.svg"
    if not source.is_file():
        raise SystemExit(f"Glifo ausente: {source}")
    paths = PATH_ELEMENT.findall(source.read_text(encoding="utf-8"))
    if not paths:
        raise SystemExit(f"Nenhum path em {source}")
    return "".join(paths)


def compose(
    glyph: str, size: int, ink: str = INK_DARK, maskable: bool = False
) -> str:
    radius, ratio, variant = RECIPE[size]
    if maskable:
        # O sistema operacional recorta o icone maskable na forma que quiser —
        # circulo, quadrado arredondado, gota. Entao o campo sangra ate a borda
        # (o raio viria do recorte, e desenhar o nosso deixaria um canto morto)
        # e o desenho encolhe para a zona segura, um circulo de 80% do lado.
        # 0.55 e o teto: a DIAGONAL da caixa do glifo e o que precisa caber, e
        # com 0.64 ela daria 0.90 — os cantos sairiam fora.
        radius = 0
        ratio = 0.55
    box = size * ratio
    offset = (size - box) / 2
    # O `viewBox` do Material Symbols vai de -960 a 0 no eixo Y. Um `<svg>`
    # aninhado resolve o deslocamento sozinho, e evita compor a matriz a mao.
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" '
        f'viewBox="0 0 {size} {size}">'
        f'<rect width="{size}" height="{size}" rx="{radius}" fill="{FIELD}"/>'
        f'<svg x="{offset:g}" y="{offset:g}" width="{box:g}" height="{box:g}" '
        f'viewBox="0 -960 960 960"><g fill="{ink}">{glyph_paths(glyph, variant)}</g></svg>'
        "</svg>"
    )


def render(glyph: str, size: int, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        "w", suffix=".svg", delete=False, encoding="utf-8"
    ) as handle:
        handle.write(compose(glyph, size))
        source = Path(handle.name)
    try:
        subprocess.run(
            ["resvg", str(source), str(destination), "-w", str(size), "-h", str(size)],
            check=True,
            capture_output=True,
        )
    finally:
        source.unlink()


def write_ico(glyph: str, destination: Path) -> None:
    """Cada tamanho do .ico recebe o SEU desenho.

    Deixar o Pillow derivar as variantes de uma fonte so reintroduziria o
    escalonamento que a receita existe para evitar: o glifo de 16 nao e o de
    256 reduzido, e sim outro peso, com FILL diferente.
    """
    sizes = [256, 128, 64, 48, 32, 24, 16]
    with tempfile.TemporaryDirectory() as workspace:
        frames = []
        for size in sizes:
            frame = Path(workspace) / f"{size}.png"
            render(glyph, size, frame)
            frames.append(Image.open(frame).convert("RGBA"))
        # A base TEM que ser o maior: o Pillow descarta de `sizes` tudo que for
        # maior que a imagem base.
        frames[0].save(
            destination,
            format="ICO",
            sizes=[(size, size) for size in sizes],
            append_images=frames[1:],
        )
    written = sorted(size for size, _ in Image.open(destination).info["sizes"])
    missing = sorted(set(sizes) - set(written))
    if missing:
        raise SystemExit(f"{destination.name} saiu incompleto — faltam {missing}.")


def main() -> None:
    # CronoCAD e Tauri: precisa do conjunto de PNG e do .ico.
    crono = ROOT / "apps/cronocad/src-tauri/icons"
    if crono.is_dir():
        for name, size in {
            "32x32.png": 32,
            "64x64.png": 64,
            "128x128.png": 128,
            "128x128@2x.png": 256,
            "icon.png": 512,
        }.items():
            render("timer", size, crono / name)
            print(f"cronocad/{name:<18} {size:>4}px  {RECIPE[size][2]}")
        write_ico("timer", crono / "icon.ico")
        print(f"cronocad/{'icon.ico':<18} 16, 24, 32, 48, 64, 128, 256")

    # Os dois web usam um SVG so, que o navegador escala. A aba renderiza entre
    # 16 e 32, entao o arquivo congela a linha de 32 da receita: FILL 1 e peso
    # 500. Congelar a linha de 256 daria um traco vazado que fecha e vira
    # mancha justamente no tamanho em que o icone e visto.
    for app, glyph in (("m-finance", "money"), ("coded-atlas", "screenshot_monitor")):
        icon = ROOT / f"apps/{app}/app/icon.svg"
        if icon.parent.is_dir():
            icon.write_text(compose(glyph, 32) + "\n", encoding="utf-8")
            print(f"{app}/app/icon.svg      32px  wght500fill1")

            # O icone de tela inicial do iOS sai como PNG estatico, e nao pelo
            # `ImageResponse`: o Satori nao desenha `path`, entao o `.tsx` so
            # conseguia aproximar a marca com borda de CSS. Um PNG gerado pela
            # mesma receita nao precisa aproximar nada.
            render(glyph, 180, icon.parent / "apple-icon.png")
            legacy = icon.parent / "apple-icon.tsx"
            if legacy.is_file():
                legacy.unlink()
                print(f"{app}/app/apple-icon.tsx removido — virou PNG")
            print(f"{app}/app/apple-icon.png 180px  wght300")

        # O manifest do PWA aponta para `/icon.svg` e `/maskable.svg`, servidos
        # de `public/` — arquivos distintos dos de `app/`. Era ali que a marca
        # anterior sobrevivia, e e justamente o que o sistema operacional usa ao
        # instalar o app na tela inicial.
        public = ROOT / f"apps/{app}/public"
        for name, masked in (("icon.svg", False), ("maskable.svg", True)):
            target = public / name
            if target.is_file():
                target.write_text(
                    compose(glyph, 512, maskable=masked) + "\n", encoding="utf-8"
                )
                print(f"{app}/public/{name:<13} 512px  {'maskable' if masked else 'wght300'}")


if __name__ == "__main__":
    main()
