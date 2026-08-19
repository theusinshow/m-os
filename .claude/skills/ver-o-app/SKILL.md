---
name: ver-o-app
description: Ver a janela do M/OS de verdade — subir o app, capturar a janela do Tauri (mesmo coberta ou minimizada), tirar foto da página inteira e dirigir a interface. Use ao conferir qualquer mudança visual do app desktop, ao pedir "abra o app", "tire um print", "mostre a tela", ou antes de afirmar que uma mudança de interface está pronta.
---

# Ver o app

O M/OS desktop é Tauri + WebView2. Três coisas nele quebram o instinto de quem
tenta olhar a tela pela primeira vez, e as três estão resolvidas aqui.

## O que NÃO funciona (e não adianta tentar de novo)

**O screenshot do `orca computer` fotografa a região da tela**, não a janela.
Qualquer coisa por cima do M/OS aparece no lugar dele — inclusive conteúdo
particular de outros aplicativos. `--restore-window` não corrige.

**A árvore de acessibilidade serve dado em cache.** Texto que muda no lugar e até
mudança de estrutura continuam vindo com o valor velho por várias leituras. Já
custou uma investigação inteira: o banco dizia `span 9`, a árvore insistia em `8`,
e o React estava certo desde o começo. O que força árvore nova é **salvar
qualquer arquivo do front** — o Vite recarrega o DOM e a leitura seguinte é fiel.

**A rolagem sintética não alcança o WebView.** `orca computer scroll` não move a
página. Para ver o que está abaixo da dobra, use o modo de página inteira abaixo.

## Subir o app

```bash
cd apps/desktop && npm run tauri dev
```

Rode em segundo plano; o build Rust leva de 20 s a alguns minutos. Espere pelo
processo, não por um tempo fixo:

```bash
until powershell -NoProfile -Command "if (Get-Process mos-desktop -ErrorAction SilentlyContinue) { exit 0 } else { exit 1 }"; do sleep 3; done
```

O front é servido pelo Vite em `http://localhost:1420/`, com HMR: editar `.tsx`
ou `.css` recarrega sem rebuild. Editar Rust dispara recompilação e reinício.

## A janela costuma estar MINIMIZADA

É a causa de quase todo "não consigo ler a janela". Minimizada, ela some do UIA
(sobra só uma auxiliar de 16×16 chamada `com.codedbym.mos-siw`) e reporta 160×28.

O script abaixo **restaura sozinho, sem roubar o foco** (`SW_SHOWNOACTIVATE`).
Não é preciso tratar isso à mão.

Cuidado ao enumerar janelas: o processo mantém **duas** com o título `M/OS` — uma
oculta de 436×261 e a de verdade. Pegar a primeira que aparecer traz a errada, e
mostrá-la deixa um retângulo estranho na tela do dono. O script pega a certa.

## Capturar

`capturar-janela.ps1` usa `PrintWindow` com `PW_RENDERFULLCONTENT`: pede à
própria janela que se desenhe. Funciona com ela coberta, atrás de outras ou fora
da tela — e a flag `2` é obrigatória, porque sem ela uma janela composta por GPU
devolve um retângulo preto.

Roda no **Windows PowerShell 5.1** (`powershell.exe`), não no 7: o
`System.Drawing` saiu do conjunto padrão do .NET moderno.

```bash
S=.claude/skills/ver-o-app
# a dobra visível
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(cygpath -w $S/capturar-janela.ps1)" \
  -Titulo "M/OS" -Processo "mos-desktop" -Saida "$(cygpath -w /tmp/tela.png)"

# a PÁGINA INTEIRA numa foto só
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(cygpath -w $S/capturar-janela.ps1)" \
  -Titulo "M/OS" -Processo "mos-desktop" -Altura 2400 -Saida "$(cygpath -w /tmp/home.png)"
```

`-Altura` estica a janela **fora da tela** (x = −8000), fotografa e devolve
tamanho e posição. Nada disso aparece para quem está usando o computador.

Depois **olhe a imagem** com a ferramenta Read. Ela é a única prova de aparência
que existe antes da tela do dono.

O script imprime `conteudo=NN%`, a fração de pixels não pretos. **Abaixo de ~5%
a captura falhou** — quase sempre porque a flag veio diferente de `2`, ou porque
o WebView2 suspendeu o desenho. Não trate uma foto preta como "a tela está
escura".

## Dirigir a interface

Com a janela restaurada, o UIA funciona:

```bash
orca computer get-app-state --app mos-desktop --no-screenshot     # a árvore
orca computer click --app mos-desktop --element-index 56 --no-screenshot
```

Sempre `--no-screenshot`: a foto do orca não presta, e pedi-la só desperdiça
tempo. Os índices vêm do `get-app-state` mais recente.

## Quando o app e a foto discordarem

**Suspeite da árvore primeiro** — foi ela que errou. A ordem de confiança é:

1. **o banco**, que é o oráculo de comportamento:
   ```bash
   python -c "import sqlite3,os; db=os.path.expandvars(r'%APPDATA%\com.codedbym.mos\m-os.db'); \
   con=sqlite3.connect(f'file:{db}?mode=ro',uri=True); print(con.execute('PRAGMA user_version').fetchone())"
   ```
2. **a foto** do `capturar-janela.ps1`;
3. **a árvore** do UIA, e só depois de um recarregamento do Vite.

Se faltar sinal, **faça o app falar**: renderize o estado num elemento da tela e
leia esse elemento. Vale mais que dez inspeções — foi o que finalmente resolveu a
investigação do `span`.

## Antes de dizer que está pronto

Para mudança visual, a bancada headless (`packages/design-system/tokens.css` +
`App.css` reais, em Playwright) pega o que teste nenhum pega, e cobre os dois
temas e as larguras de quebra sem depender da janela. Ela **não substitui** a
foto da janela real: a bancada reproduz a marcação à mão e pode divergir do que o
React gera. Use as duas — a bancada para varrer estados, a foto para confirmar.

E a palavra final sobre aparência continua sendo do dono do produto.
