"""Gera os icones do M/OS a partir dos tres desenhos do simbolo.

O handoff proibe escalar um unico SVG entre tamanhos: a mesma inclinacao
geometrica le mais fina conforme o desenho encolhe, entao o angulo abre para
compensar. Sao tres poligonos distintos, e cada tamanho usa o seu.

Por isso `tauri icon` nao serve — ele deriva tudo de uma fonte so.

Uso:
    python scripts/generate-icons.py

DEPOIS DE RODAR, LIMPE O RESOURCE DO BUILD:

    rm -rf target/release/build/mos-desktop-*
    rm -f  target/release/mos-desktop.exe

O `tauri-build` gera um `resource.rc` que aponta para `icons/icon.ico`, mas nao
declara `rerun-if-changed` nos arquivos de icone. Trocar o icone sozinho nao
invalida nada: o Cargo reaproveita o `.res` ja compilado e o executavel sai com
o icone antigo. O build passa, o instalador e produzido, nenhum aviso aparece —
so olhando os bytes do binario da para notar.

Para conferir que o icone entrou de verdade:

    python -c "ico=open('apps/desktop/src-tauri/icons/icon.ico','rb').read(); \
    s=ico.find(b'\\x89PNG\\r\\n\\x1a\\n'); \
    print(open('target/release/mos-desktop.exe','rb').read().find(ico[s:s+48]))"

Offset >= 0 significa que o icone correto esta embutido. -1 significa que o
resource ficou em cache.

Requer Pillow. Nao gera .icns (o alvo de release e Windows/NSIS); se o macOS
entrar, o .icns precisa de uma ferramenta que escreva aquele container.
"""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw

# viewBox 0 0 64 64 nos tres. Centroide em (32,32).
BARS = {
    # 1024 · 512 · 256 · 128 — 22 graus
    "large": [(38, 8), (53, 8), (26, 56), (11, 56)],
    # 64 · 48 — 18 graus
    "medium": [(40, 10), (54, 10), (24, 54), (10, 54)],
    # 32 · 24 · 16 — 14 graus
    "small": [(42, 12), (56, 12), (22, 52), (8, 52)],
}

FIELD = (0xE7, 0xC2, 0x4E)  # --signal-fill
INK = (0x0A, 0x0C, 0x0E)  # --on-signal (dark)

# Radius do quadrado fixado pelo handoff. Fora destes, 18.75% do lado — a
# proporcao que os pontos declarados descrevem.
RADIUS = {1024: 205, 64: 11, 32: 6, 16: 3}

# Supersampling: desenhamos 4x maior e reduzimos. A geometria continua sendo a
# do tamanho alvo; so as bordas ficam limpas.
SUPERSAMPLE = 4


def bar_for(size: int) -> list[tuple[int, int]]:
    if size >= 128:
        return BARS["large"]
    if size >= 48:
        return BARS["medium"]
    return BARS["small"]


def radius_for(size: int) -> int:
    if size in RADIUS:
        return RADIUS[size]
    return max(1, round(size * 0.1875))


def render(size: int) -> Image.Image:
    canvas = size * SUPERSAMPLE
    image = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)

    draw.rounded_rectangle(
        (0, 0, canvas - 1, canvas - 1),
        radius=radius_for(size) * SUPERSAMPLE,
        fill=FIELD,
    )

    scale = canvas / 64
    draw.polygon([(x * scale, y * scale) for x, y in bar_for(size)], fill=INK)

    return image.resize((size, size), Image.LANCZOS)


def main() -> None:
    icons = Path(__file__).resolve().parent.parent / "apps/desktop/src-tauri/icons"
    if not icons.is_dir():
        raise SystemExit(f"Pasta de icones nao encontrada: {icons}")

    targets = {
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

    for name, size in targets.items():
        render(size).save(icons / name)
        print(f"{name:<24} {size:>4}px  {'22' if size >= 128 else '18' if size >= 48 else '14'} graus")

    # O .ico carrega varios tamanhos, e cada um precisa do seu proprio desenho:
    # deixar o Pillow derivar as variantes de uma fonte so reintroduziria
    # exatamente o escalonamento que o handoff proibe.
    #
    # A base TEM que ser o maior desenho. O Pillow descarta de `sizes` tudo que
    # for maior que a imagem base, entao salvar a partir do frame de 16px
    # produzia um .ico com um unico icone de 16 — e o Windows ampliava aquilo
    # para 32, 48 e 256. Foi exatamente esse o bug da primeira versao.
    ico_sizes = [256, 128, 64, 48, 32, 24, 16]
    frames = [render(size) for size in ico_sizes]
    frames[0].save(
        icons / "icon.ico",
        format="ICO",
        sizes=[(size, size) for size in ico_sizes],
        append_images=frames[1:],
    )

    written = sorted(size for size, _ in Image.open(icons / "icon.ico").info["sizes"])
    print(f"{'icon.ico':<24} {', '.join(str(size) for size in written)}")
    missing = sorted(set(ico_sizes) - set(written))
    if missing:
        raise SystemExit(
            f"icon.ico saiu incompleto — faltam {missing}. O Windows ampliaria o "
            "tamanho mais proximo, e o icone ficaria borrado."
        )


if __name__ == "__main__":
    main()
