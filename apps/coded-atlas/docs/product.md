# Coded Atlas — Produto

## Objetivo

**Coded Atlas** é uma ferramenta interna da **Coded by M** que recebe a URL de um projeto
web e gera automaticamente um pacote visual de apresentação, transformando um site finalizado
em um **catálogo visual premium** pronto para páginas de case, apresentações comerciais,
laboratório e redes sociais.

Documentos relacionados:

- `design.md` → linguagem visual, fluxos e estados de interface.
- `architecture.md` → especificação técnica e contratos de implementação.

---

## O que a ferramenta gera

A partir de uma URL, o sistema captura:

- screenshots em desktop e mobile;
- screenshots full page (desktop e mobile);
- thumbnails para portfólio;
- dados estruturados do projeto (`catalog.json`);
- estrutura inicial para futuro case.

Em versões futuras: vídeos curtos de navegação, detecção de seções e rascunho de case.

---

## Nome do produto

**Coded Atlas.** "Atlas" comunica catálogo, mapeamento, coleção visual e organização de
projetos — alinhado à ideia de paisagem digital da marca. Não soa genérico e combina com a
direção premium, técnica e visual da Coded by M.

Outras opções consideradas (descartadas): CaseForge, Project Atlas, M Capture, Coded Showcase,
Portfolio Engine, Site Atlas, Visual Atlas.

---

## Relação com a Coded by M

Posicionamento da marca:

> Web Design Premium para empresas que querem transmitir mais valor, confiança e profissionalismo.

O Coded Atlas **não muda** esse posicionamento. Ele é uma **ferramenta interna/laboratorial**
criada para melhorar como a Coded by M documenta, apresenta e transforma seus projetos em cases.

- **Mensagem correta:** "Criamos ferramentas próprias para apresentar melhor os projetos que construímos."
- **Mensagem errada:** "Somos uma plataforma de geração automática de catálogos."

Função estratégica: prova de capacidade técnica, peça forte do Laboratório, ferramenta real de
produtividade, diferencial visual no portfólio e demonstração de método e detalhismo.

---

## Problema que resolve

Criar cases para portfólio é manual e repetitivo: abrir o site, tirar prints, ajustar tamanhos,
capturar mobile e desktop, gravar tela, organizar arquivos, montar mockups, criar thumbnails,
estruturar a página de case — e repetir isso a cada projeto. Esse processo consome tempo, gera
inconsistência visual e atrasa a publicação dos projetos.

---

## Oportunidade

Transformar uma tarefa operacional em um produto visual com valor de marca. Em vez de só gerar
prints, o sistema entrega um **pacote completo de apresentação**, com estética consistente e
pronto para uso no portfólio.

---

## Conceito central

**Coded Atlas transforma uma URL em um catálogo visual de projeto.**

```txt
URL do projeto
↓ Captura automática
↓ Screenshots desktop e mobile
↓ Organização dos assets
↓ Geração de preview
↓ Rascunho de case (futuro)
↓ Publicação no portfólio (futuro)
```

---

## Usuário principal

**Matheus / Coded by M.** A primeira versão é uma ferramenta interna para uso próprio.

Usuários futuros possíveis (não prioritários no início): designers, devs front-end, freelancers,
pequenos estúdios, equipes de marketing e criadores de portfólio.

---

## Escopo inicial

### Entrada

URL do projeto, nome, slug, categoria, cliente e descrição curta (opcional).

### Saída

Screenshot desktop, screenshot mobile, full page desktop, full page mobile, thumbnails,
`catalog.json` e a estrutura inicial para futuro case. Página de resultado com galeria dos assets.

---

## MVP — v0.1

### Objetivo

Validar a captura automática de sites a partir de uma URL. A primeira versão **não precisa** de
IA, banco complexo, autenticação ou publicação automática.

### Funcionalidades obrigatórias

1. Formulário de cadastro de projeto.
2. Validação básica de URL.
3. Captura desktop (1440x900).
4. Captura mobile (390x844).
5. Captura full page desktop e mobile.
6. Salvamento dos arquivos em pasta local.
7. Geração de `catalog.json`.
8. Página de resultado com galeria dos assets.
9. Acesso para baixar/abrir os arquivos gerados.

### Fora do MVP

Login, multiusuário, pagamento, dashboard avançado, IA generativa, publicação automática,
banco PostgreSQL, painel administrativo complexo, edição manual avançada de assets.

---

## Roadmap

### v0.1 — Captura essencial

Input de URL, captura desktop/mobile/full page, organização em pasta, galeria de resultado.

### v0.2 — Vídeo automático

Gravação desktop/mobile com scroll suave, duração configurável, exportação WebM (MP4 futuro),
thumbnail automática do vídeo.

### v0.3 — Detecção inteligente de seções

Detectar `header`/`main`/`section`/`footer`, identificar títulos, mapear blocos, capturar
seções individualmente e sugerir nomes.

### v0.4 — Gerador de case

Gerar `case-draft.mdx`, criar estrutura de case, sugerir título e resumo, organizar imagens por
bloco, preparar integração com `/cases/[slug]` no portfólio.

### v1.0 — Ferramenta de portfólio completa

Interface premium, histórico de projetos, reprocessamento de captures, exportação ZIP,
integração com storage e GitHub, publicação manual assistida, página pública no Laboratório.

### Pós-v1.0

Evoluções planejadas (lightbox, paleta de comando, operações em lote e opções de captura por
geração) estão detalhadas e priorizadas em `ROADMAP.md`. A publicação automática no GitHub
ficou fora de escopo; a publicação no portfólio segue manual assistida.

---

## Integração futura com o portfólio

```txt
Coded Atlas → gera assets → catalog.json → case-draft.mdx
→ Portfolio Coded by M lê os dados → case aparece em /cases/[slug]
```

Cada projeto gerado pode virar um fragmento na Paisagem Digital da Home, usando: nome, categoria,
thumbnail, slug, cor/acento, tipo de projeto e descrição curta.

---

## Critérios de qualidade (produto)

A ferramenta é julgada por: captura correta da página, qualidade das imagens, consistência
desktop/mobile, organização dos arquivos, clareza da interface, velocidade aceitável, baixa
chance de falha, facilidade de usar os assets no portfólio, aparência premium e capacidade de
evoluir para automações maiores.

---

## Decisão final

O Coded Atlas começa como ferramenta interna simples, mas com direção visual e técnica forte.
A prioridade inicial **não é vender o software** — é facilitar a criação de cases, fortalecer o
portfólio, provar capacidade técnica e criar uma peça diferenciada para o Laboratório, mostrando
que a Coded by M constrói não apenas sites, mas sistemas próprios para elevar a apresentação dos
projetos.
