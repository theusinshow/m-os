# Coded Atlas — Design & UX

## Propósito

Este documento define a **experiência e a linguagem visual** do Coded Atlas: como a ferramenta
deve parecer, soar e se comportar. Complementa `product.md` (o quê/porquê) e `architecture.md`
(como construir).

Princípio que rege tudo:

> Nada deve parecer SaaS genérico. O Coded Atlas é uma ferramenta construída **pela** Coded by M,
> não uma plataforma de prateleira.

---

## Sensação desejada

Premium · técnico · limpo · escuro · preciso · cinematográfico · funcional · não genérico.

O visual deve parecer uma extensão do universo da Coded by M — especialmente do Laboratório, da
Paisagem Digital, do sistema de projetos e dos cases individuais.

---

## Direção visual

- fundo escuro;
- cards com bordas discretas;
- tipografia limpa;
- grids técnicos;
- detalhes triangulares sutis;
- microinterações elegantes;
- nada colorido demais;
- sensação de ferramenta interna premium / laboratório.

---

## Sistema visual (v1.0)

Tokens definidos em `app/globals.css` (`@theme`, OKLCH). Estratégia **Restrained**:
neutros frios tintados + **um acento cobre**. Texto sempre de alto contraste.

- **Acento — cobre/âmbar** (`--color-accent` ≈ `oklch(0.72 0.115 62)`). Usado só em
  estado ativo, links, foco, passo atual e ações de baixo peso. Conceito atlas/cartografia;
  foge do clichê verde/azul de ferramenta dev. **Não** decorar com ele.
- **Ação primária** continua em branco-gelo (`bg-zinc-100`), o maior contraste.
- **Status** reservado a estado: `--color-ok` (verde, sucesso), `--color-bad` (vermelho, erro),
  `--color-warn` (âmbar, atenção). Nunca como enfeite.
- **Superfícies frias**: `--color-base` (fundo), `--color-surface` / `--color-surface-2` (painéis),
  `--color-line` / `--color-line-soft` (bordas).
- **Texto**: primário `zinc-50/100`, secundário `zinc-300/400`. Evitar `zinc-600/700` como texto.
- **Foco** sempre visível em cobre (`:focus-visible`). **Navegação global** persistente (`AppNav`)
  no topo, com o item atual sublinhado em cobre.
- Utilitários da marca: `.bg-grid` (grid técnico) e `.tri` (marcador triangular).

---

## Fluxo do usuário

```txt
1. Usuário acessa o Coded Atlas
2. Preenche URL e dados do projeto
3. Clica em "Gerar Catálogo"
4. Sistema inicia a captura
5. Sistema mostra o status do processo em etapas
6. Sistema gera os arquivos
7. Usuário visualiza a galeria de assets
8. Usuário baixa os arquivos ou usa no portfólio
```

---

## Estados da interface

A interface deve ter estados claros e visíveis — nada de "carregando" genérico. Os estados
espelham os passos reais da captura (ver enum `CaptureStep` no `architecture.md`):

- vazio (antes de preencher);
- preenchendo;
- validando URL;
- abrindo o site;
- capturando desktop;
- capturando mobile;
- gerando full page;
- gerando thumbnails;
- finalizando;
- concluído;
- erro (com mensagem amigável, nunca stack trace).

---

## Telas

### Tela inicial (`/`)

Landing interna da ferramenta. Explica rápido o que é, para que serve e como usar.

Elementos: título grande, descrição curta, input de URL, botão principal, pequena explicação do
fluxo, visual técnico ao fundo. CTA para gerar catálogo.

### Tela de geração (`/generate`)

Página principal. Contém formulário, status de geração e preview do resultado.

Elementos: status em etapas, barra de progresso visual, logs simplificados, preview parcial e
(futuro) botão de cancelar.

### Tela de resultado (`/projects/[slug]`)

Visualização de um projeto gerado.

Elementos: hero com thumbnail, dados do projeto, galeria desktop, galeria mobile, vídeos (futuro),
botão de download e (futuro) botão "Gerar Case". Desktop e mobile devem aparecer em seções
visualmente separadas.

### Página do Laboratório (`/lab/coded-atlas`) — futura

Apresenta o produto como experimento do Laboratório dentro do portfólio. Seções sugeridas: hero
do experimento, problema, como funciona, capturas geradas, vídeo de navegação, estrutura técnica,
próximos passos, CTA para ver projetos.

---

## Componentes (perspectiva de UX/UI)

### `UrlInput`

Formulário de entrada. Campos: URL, nome do projeto, slug, categoria, cliente, descrição curta.
Validação de URL visível antes do envio.

### `GenerationStatus`

Mostra o andamento em etapas, reagindo aos eventos de progresso. Cada etapa acende conforme
acontece de verdade — não é animação fake.

### `GeneratedGallery`

Galeria dos assets: screenshot desktop, screenshot mobile, full page desktop, full page mobile,
thumbnails (e vídeos no futuro).

### `DeviceFrame`

Exibe screenshots dentro de molduras visuais. Variações: desktop frame, mobile frame, browser
frame simples e preview sem frame.

### `ProjectCatalogCard`

Card resumido para listar projetos gerados. Dados: thumbnail, nome, categoria, URL, data de
geração e ação para abrir.

### `AssetDownloads`

Acesso para baixar ou abrir os arquivos gerados de um projeto.

---

## Copy base

| Elemento | Texto |
|---|---|
| Título | Coded Atlas |
| Subtítulo | Transforme uma URL em um catálogo visual de projeto. |
| Descrição curta | Uma ferramenta interna da Coded by M para capturar sites, gerar screenshots e organizar assets visuais para cases de portfólio. |
| CTA principal | Gerar Catálogo |
| CTA secundário | Ver Projetos Gerados |
| Carregando | Construindo catálogo visual... |
| Conclusão | Catálogo gerado com sucesso. |
| Erro | Não foi possível capturar este projeto. Verifique a URL e tente novamente. |

As mensagens de erro exibidas ao usuário devem ser sempre amigáveis e em PT-BR. A tabela técnica
que liga cada código de erro à sua mensagem está no `architecture.md` (seção "Taxonomia de Erros").

---

## Princípio visual final

O Coded Atlas deve parecer uma ferramenta construída pela própria Coded by M: precisa, escura,
técnica, elegante, com movimento discreto, detalhes triangulares e sensação de laboratório
premium. Se em algum momento a interface começar a parecer um SaaS genérico, a direção está errada.
