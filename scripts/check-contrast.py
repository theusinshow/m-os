"""Guarda o contraste dos tokens de cor do M/OS.

Existe porque o problema que ele checa ja aconteceu: o `--text-system` rodou
meses a 3,15:1 rotulando cada painel do aplicativo, e `--text-placeholder` a
2,31:1 fazia campo preenchido parecer campo vazio. Nada acusou — contraste ruim
nao quebra build, nao falha teste e nao aparece em revisao de codigo. So aparece
como "nao consigo enxergar direito", meses depois, e quem diz isso nao tem como
saber que o numero e 2,31.

O alvo e o WCAG 2.1 AA: 4,5:1 para texto, 3:1 para o que esta desabilitado.

O fundo de referencia e o CARD e nao a pagina, de proposito: no dark ele e o
mais CLARO dos dois, e portanto o caso pior para texto claro. Passar nele
garante os dois.

Roda sozinho: `python scripts/check-contrast.py`. Sai com codigo 1 se algo
reprova, para o CI poder barrar.
"""

import pathlib
import re
import sys

TOKENS = pathlib.Path(__file__).resolve().parent.parent / "packages" / "design-system" / "tokens.css"

# Fundo de referencia por tema, e o alvo de cada token.
CHECKS = [
    (
        "dark",
        "#171B1F",
        {
            "--text": 4.5,
            "--text-secondary": 4.5,
            "--text-system": 4.5,
            "--text-placeholder": 4.5,
            "--text-disabled": 3.0,
            "--signal-ink": 4.5,
        },
    ),
    (
        "light",
        "#FFFFFF",
        {
            "--text": 4.5,
            "--text-secondary": 4.5,
            "--text-system": 4.5,
            "--text-placeholder": 4.5,
            "--text-disabled": 3.0,
            "--signal-ink": 4.5,
        },
    ),
]


def relative_luminance(value: str) -> float:
    value = value.lstrip("#")
    channels = [int(value[index : index + 2], 16) / 255 for index in (0, 2, 4)]
    channels = [
        channel / 12.92 if channel <= 0.04045 else ((channel + 0.055) / 1.055) ** 2.4
        for channel in channels
    ]
    return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2]


def contrast(foreground: str, background: str) -> float:
    first, second = relative_luminance(foreground), relative_luminance(background)
    lighter, darker = max(first, second), min(first, second)
    return (lighter + 0.05) / (darker + 0.05)


def blocks(css: str) -> dict[str, str]:
    """O bloco `:root` e o `[data-theme='light']`, cada um com os seus valores."""
    dark, _, light = css.partition("[data-theme='light']")
    # O bloco de alto contraste do Windows troca cores por `Canvas`/`GrayText`,
    # que nao sao hex e nao devem ser medidos: quem manda ali e o sistema.
    light = light.partition("@media (forced-colors")[0]
    return {"dark": dark, "light": light}


def value_of(block: str, token: str) -> str | None:
    found = re.search(rf"{re.escape(token)}:\s*(#[0-9A-Fa-f]{{6}})", block)
    return found.group(1) if found else None


def main() -> int:
    css = TOKENS.read_text(encoding="utf-8")
    parts = blocks(css)
    failures = 0

    for theme, background, targets in CHECKS:
        print(f"=== {theme} · sobre {background}")
        for token, target in targets.items():
            value = value_of(parts[theme], token)
            if value is None:
                print(f"  {token:20} NAO ENCONTRADO")
                failures += 1
                continue
            measured = contrast(value, background)
            passed = measured >= target
            failures += 0 if passed else 1
            mark = "ok" if passed else "REPROVA"
            print(f"  {token:20} {value}  {measured:5.2f}:1  alvo {target:.1f}  {mark}")
        print()

    if failures:
        print(f"{failures} token(s) abaixo do alvo de contraste.")
        return 1
    print("Contraste dos tokens em dia.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
