# M/OS — Behavioral Wireframes v0.2 Library

## 1. Status

**Status:** definido para o primeiro corte visual da Fase Memory

**Data:** 2026-08-13

**Natureza:** wireframes comportamentais; não são especificação visual pixel-perfect

## 2. Constraint principal

Library é uma projeção de Resources. Ela deve permitir preservar e reencontrar um link junto do motivo pelo qual ele foi salvo, sem antecipar taxonomia, metadata remota ou uma experiência visual de inspiração.

```text
URL + contexto opcional
        ↓
Resource local confirmado
        ↓
Library e Command Search
```

O primeiro corte implementa somente `Resource(kind=link)`.

## 3. Navegação

Library entra como destino primário porque possui uma intenção estável e diferente de Apps ou Projects:

> reencontrar conhecimento e referências preservadas.

Ela usa o mesmo rail permanente das demais superfícies. Search continua sendo comando global e atravessa Resources sem exigir que o usuário abra a Library primeiro.

## 4. Library

### 4.1 Lista e detalhe

```text
LIBRARY
──────────────────────────────┬──────────────────────────────────────
> Motion                      │ RESOURCE · LINK
  motion.dev · agora          │
                              │ Motion
  SQLite FTS5                 │ https://motion.dev
  sqlite.org · ontem          │
                              │ Boa referência para animações de Hero.
                              │
                              │ [Abrir link] [Editar] [...]
──────────────────────────────┴──────────────────────────────────────
```

- a lista contém Resources ativos, ordenados por atualização recente;
- primeira linha mostra o título;
- segunda linha mostra o domínio e o tempo relativo;
- o detalhe preserva título, URL, nota pessoal e proveniência quando existir;
- o link abre pelo backend nativo somente depois de validar `http://` ou `https://`;
- Archive e Trash permanecem em ações secundárias;
- nenhuma categoria, tag, imagem ou favicon é inventada.

### 4.2 Criação direta

```text
NOVO RESOURCE

URL       [https://...                         ]
TÍTULO    [opcional; usa a URL quando vazio    ]
POR QUÊ?  [contexto pessoal opcional           ]

                              [Cancelar] [Salvar]
```

- URL é o único campo obrigatório;
- título vazio usa a URL como fallback;
- nota responde silenciosamente “por que salvei isso?”;
- erro preserva todos os campos;
- sucesso seleciona o Resource criado na Library.

### 4.3 Estado vazio

```text
Library

Guarde um link junto do motivo pelo qual ele merece ser lembrado.
[Salvar primeiro link]
```

Uma ação dominante é suficiente. Não entram ilustração, onboarding ou exemplos artificiais.

## 5. Inbox → Resource

O detalhe de uma Capture passa a oferecer dois destinos explícitos:

```text
[Criar Task] [Salvar como Resource] [Marcar como processada]
```

Ao escolher Resource:

- o formulário tenta usar o conteúdo como URL apenas quando ele já começa com `http://` ou `https://`;
- conteúdo não reconhecido permanece como nota ou título, sem interpretação remota;
- salvar cria Resource, preserva a Capture e marca seu processamento na mesma transação;
- falha mantém a Capture na Inbox e preserva os campos;
- sucesso abre o novo Resource na Library.

## 6. Command Search

Resources entram na lista unificada por título, URL e nota.

```text
RESOURCE   Motion
           motion.dev · Boa referência para animações de Hero
```

- selecionar o resultado abre o detalhe na Library;
- `Incluir arquivados` também se aplica a Resources;
- Trash nunca aparece;
- o resultado não abre a URL diretamente, preservando a diferença entre encontrar e executar.

## 7. Archive e Trash

Settings > Archive e Trash ganha duas listas operacionais:

- Resources arquivados;
- Lixeira de Resources.

Restore retorna o item a `active` sem remover proveniência. Exclusão definitiva continua fora do corte.

## 8. Estados e acessibilidade

- criação, edição e abertura informam progresso, sucesso ou erro no próprio contexto;
- formulários possuem labels visíveis;
- lista, detalhe e ações funcionam com teclado;
- foco retorna a um destino previsível ao cancelar edição;
- URL e nota longa quebram linha sem alterar a largura da superfície;
- forced colors e reduced motion reutilizam os contratos existentes;
- nenhum estado depende apenas de cor.

## 9. Fora deste corte

- grid visual e modos alternativos de Library;
- screenshots, imagens e arquivos;
- favicon, Open Graph e scraping;
- tags, categorias e taxonomia;
- relações com Project, Workspace, Task ou App;
- deduplicação automática;
- semantic search;
- importação de favoritos;
- Share mobile.

## 10. Gate

O corte visual está pronto quando:

1. criação direta e conversão da Inbox preservam dados em caso de erro;
2. Resource ativo pode ser encontrado, aberto, editado, arquivado e enviado à Lixeira;
3. Archive e Trash podem ser restaurados;
4. Command Search encontra título, URL e nota e abre o Resource correto;
5. a interface não introduz classificação obrigatória nem possibilidades futuras como controles vazios.
