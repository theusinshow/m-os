# Spec — Lab de ícones e logos

Data: 2026-10-27
Origem: pedido do dono — "um Lab bem completo para auditar todos os ícones e
logos do software, para deixar tudo alinhado e perfeito".

## 1. Decisões do dono

1. **Só uma bancada em HTML, para ver.** Nada dentro do app, nada no CI.
2. **Galeria com marcações.** Cada item mostra a regra medida, e nada bloqueia.
3. **A família inteira.** M/OS desktop, M/OS de bolso, CronoCAD, M-Finance e
   Coded Atlas. As marcas de terceiros (`AppIcon`, `MarcaDeIA`) aparecem na
   galeria, mas sem selo.
4. **Gerador mais HTML estático.** O que a bancada mostra sai do código e dos
   arquivos reais, e não de uma cópia feita à mão.

## 2. Peças

```
scripts/lab-icones/
  gerar.mjs      varre, mede e escreve a página
  imagens.mjs    lê PNG, ICO, ICNS e .rgba; escreve PNG (para embutir)
  medidas.mjs    as regras, em funções puras
  pagina.mjs     monta o HTML
  componentes.mjs  renderiza Icon, SmallIcon, MosSymbol, AppIcon e MarcaDeIA
  medidas.test.mjs imagens.test.mjs   node --test
```

- Roda com `npm run lab` em `apps/desktop`, onde o esbuild, o React e o
  `react-dom/server` já estão instalados. Nenhuma dependência nova.
- A saída é `spikes/lab-icones.html`, um arquivo só e fora do git, porque é
  gerada.
- O inventário é a varredura do `git ls-files` (svg, png, ico, icns, rgba,
  sem `node_modules`). Um arquivo que nenhum outro cita é marcado **órfão**.
  Os nomes que o bundler do Tauri usa sem citar, como `Square*Logo` e
  `StoreLogo`, não contam como órfãos.

## 3. As regras e a fonte de cada uma

O esperado vem do código, que é o que roda. O brief entra como segunda fonte.
Quando as duas discordam, a página abre com uma seção **Regras em conflito**.

| Regra | Fonte do esperado | Onde mede |
| --- | --- | --- |
| Barra por faixa (≥128 / 48–127 / <48) | os três polígonos do `Symbol.tsx` | SVG pelo `polygon`; bitmap pela reta ajustada nos centros da tinta, linha a linha |
| Raio do campo | M/OS: `RAIO` do `gerar-icones.py` (21%); irmãos: `RECIPE` do `generate-family-icons.py` (18%) | bitmap, pelo cruzamento de alfa na diagonal do canto; SVG, pelo `rx` |
| Cor | paleta e sódio do `tokens.css`, mais o antialias entre duas cores dela | SVG pelos hex de `fill` e `stroke`; bitmap pelo histograma dos pixels opacos |
| Desenho próprio por faixa | a mesma regra da barra | o tamanho N comparado com o maior reduzido para N; se forem quase iguais e de faixas diferentes, é desvio |
| Grade e traço | `Icon.tsx`: viewBox 20 (16 no `SmallIcon`), traço 1.25/1 do `tokens.css`, coordenada em .5 no contorno | o SVG renderizado |
| `filled` só em navegação | regra 3 do `Icon.tsx` | os usos de `filled` no `src`, e o arquivo de cada um |

**Selos:**
- **ok;**
- **desvio**, com o valor medido e o esperado;
- **exceção**, para um desvio que a fonte declara de propósito e com motivo, como
  o campo `#020A06` do m-finance ou o `home` desenhado em 48;
- **não medível**, com o motivo, como um quadro de ICNS antigo ou um ícone de
  irmão, que não tem barra.

**Tolerâncias:**
- **Ângulo:** ±2.5° a partir de 48px, ±5° abaixo disso.
- **Raio:** ±1.5 ponto percentual, ou 0.6px, o que for maior.
- **Cor:** 12 de distância RGB até a paleta ou até o segmento entre duas cores
  dela, com no máximo 0.5% dos pixels fora.

## 4. A página

1. **Placar:** quantos itens estão ok, com desvio, com exceção ou sem medida, e
   o filtro "só desvios".
2. **Regras em conflito:** o que o brief diz e o que o código faz, lado a lado.
3. **A família:** os cinco ícones de app em 16, 24, 32, 48, 64 e 256.
4. **Em contexto:**
   - a barra de tarefas do Windows, no claro e no escuro;
   - a bandeja;
   - a sidebar, com os ícones reais, nos dois temas;
   - a tela inicial do iPhone, com os apple-touch.
5. **Por família de arquivo:**
   - cada item em tamanho real e ampliado 8× com a grade de pixel;
   - os selos com o valor medido;
   - o caminho do arquivo;
   - os usos, ou a marca de órfão.

O tema claro/escuro da página usa os tokens reais (`tokens.css` embutido, com
`data-theme`).

## 5. Os oito eixos

```
core:          não se aplica — nada de domínio; é ferramenta de bancada
database:      não se aplica
sync:          não se aplica
desktop:       nenhuma mudança no app; o gerador lê os componentes dele
ios:           entra só como sujeito da auditoria (apple-touch do bolso)
notifications: não se aplica
hermes:        não se aplica
tests:         node --test nas medidas (ângulo, raio, cor, escalado, grade)
               e nos leitores de imagem (ida e volta do PNG, quadros do ICO)
```

## 6. Fora do escopo

- Corrigir os ícones. A bancada aponta, e a correção é um passo seguinte.
- Página dentro do app, bloqueio no CI.
- Julgar as marcas de terceiros pelas regras do brief.
