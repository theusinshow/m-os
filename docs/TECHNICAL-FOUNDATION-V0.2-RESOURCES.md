# M/OS — Technical Foundation v0.2 Resources

**Status:** implementado; dogfood controlado aprovado, sinal de uso real ainda aberto

## 1. Objetivo

Este corte inicia a Fase Memory sem antecipar a interface da Library.

O objetivo é permitir que o Core preserve um link porque ele merece ser lembrado e também preserve por que ele foi salvo.

## 2. Escopo inicial

Deve existir:

- entidade `Resource` no Core;
- primeiro tipo concreto `link`;
- título, URL e nota contextual;
- proveniência opcional de Capture;
- lifecycle `active | archived | trashed`;
- persistência SQLite e projeção FTS reconstruível;
- criação, edição, consulta, listagem, busca, Archive, Trash e Restore;
- conversão Capture → Resource na mesma transação;
- comandos Tauri, tipos e API TypeScript;
- inclusão em backup do banco e export JSON;
- abertura controlada de links ativos pelo backend nativo;
- recuperação de Resources arquivados ou enviados à Lixeira.

Não deve existir ainda:

- modos alternativos de Library como grid ou galeria visual;
- tags, categorias ou taxonomia;
- download de imagens, favicon ou Open Graph;
- scraping e metadata automática;
- anexos ou cópia de arquivos para dentro do M/OS;
- relações com Project, Workspace, Task ou App;
- deduplicação automática;
- semantic search;
- importação de favoritos;
- compartilhamento mobile.

## 3. Modelo link-first

O conceito `Resource` continua amplo, mas este corte implementa apenas um caso real e verificável:

```text
Resource
- kind = link
- title
- url
- note
- source_capture_id?
- lifecycle_state
- created_at
- updated_at
```

Não serão criados tipos vazios para imagem, arquivo, artigo, biblioteca ou inspiração. Esses significados podem emergir posteriormente sem transformar possibilidades de `IDEAS.md` em requisitos atuais.

## 4. Invariantes

1. URL de Link aceita somente `http://` ou `https://`.
2. Título vazio assume a própria URL, mantendo a captura de link de baixa fricção.
3. Nota é contexto pessoal e pode ficar vazia.
4. Processar uma Capture cria Resource, preserva a Capture e marca seu processamento atomicamente.
5. A interface inicial poderá limitar uma derivação Resource por Capture; o banco reforça essa limitação até nova decisão.
6. Archive, Trash e Restore não removem proveniência.
7. Exclusão definitiva continua fora do corte.
8. Search indexa título, URL e nota, mas o índice não é fonte de verdade.
9. Library será uma projeção de Resources, não entidade nem container proprietário.

## 5. Busca e abertura

O backend expõe busca própria de Resources por título, URL e nota. O Command compõe esses resultados com os demais tipos e abre o detalhe correto na Library. Encontrar não executa: abrir a URL continua sendo uma ação explícita no detalhe e passa novamente pela validação `http://` ou `https://` no backend nativo.

## 6. Próximo passo e gate de dogfooding

Antes de qualquer expansão visual significativa da Library, esta fatia deve passar por um ciclo real de uso. O objetivo não é aprovar uma estética final, mas descobrir atritos no fluxo mínimo já implementado.

O ciclo deve exercitar:

1. salvar um link diretamente na Library;
2. capturar um link e convertê-lo pela Inbox;
3. reencontrar o link pelo título, URL e conteúdo de `Por quê?`;
4. abrir, editar, arquivar, enviar à Lixeira e restaurar;
5. observar se a nota contextual reduz a necessidade de lembrar por que o link importava;
6. registrar falhas, hesitações e passos redundantes encontrados no uso real.

Antes desse ciclo, a base técnica deve provar:

- upgrade de um dataset v4 populado até o schema atual, preservando Apps e reconstruindo Search;
- backup e Restore em round-trip preservando Resources;
- operação da lista e do detalhe da Library somente por teclado;
- preservação dos campos quando criação ou edição falhar.

Somente os atritos observados nesse ciclo podem orientar o próximo trabalho pesado de frontend. Modos visuais, metadata remota, taxonomia e relações de contexto continuam adiados até existir necessidade observada.

### Resultado do dogfood controlado

O roteiro isolado de 2026-08-13 aprovou criação direta, falha com preservação de campos, edição, busca por nota, Capture → Resource, proveniência, Archive, Trash, Restore, Undo, single instance e navegação da Library em `840×600` somente por teclado. O teste encontrou uma falha de reabertura de Resource arquivado pela busca; a navegação passou a usar uma chave explícita por intenção e o cenário foi repetido com sucesso.

As evidências e limitações estão em `DOGFOOD-V0.2-RESOURCES.md`. Esse ensaio reduz risco funcional, mas não substitui o ciclo de uso pessoal necessário para responder se `Por quê?` realmente reduz carga mental ao longo do tempo. Grid, metadata remota, tags, taxonomia e relações continuam fora de escopo até esse sinal existir.
