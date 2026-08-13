# M/OS — Technical Foundation v0.2 Workspaces

## 1. Objetivo

Este corte introduz Workspaces como fundacao de contexto, nao como dashboard pesado.

Workspace ajuda o usuario a reunir Projects e Apps relacionados a um modo de trabalho sem transformar esses itens em silos.

## 2. Escopo deste corte

Deve existir:

- entidade local de Workspace;
- persistencia SQLite;
- busca local por Workspace;
- comandos Tauri para criar, editar, listar e arquivar;
- relacao muitos-para-muitos entre Workspace e Project;
- relacao muitos-para-muitos entre Workspace e App;
- tela minima para criar Workspace e vincular Projects/Apps existentes;
- inclusao em export JSON e backup por banco local.

Nao deve existir ainda:

- dashboard visual pesado de Workspace;
- permissao por Workspace;
- automacao;
- Hermes;
- integracoes profundas entre Apps;
- sync/cloud;
- plugin system.

## 3. Modelo inicial

Um Workspace representa um contexto amplo de atividade.

Campos iniciais:

- nome;
- descricao;
- lifecycle;
- datas de criacao e atualizacao.

Workspace nao substitui Project, App, Task ou Resource.

## 4. Decisao arquitetural

Workspaces entram no modulo de Core relacionado a trabalho e usam tabelas proprias no SQLite.

As relacoes com Projects e Apps sao muitos-para-muitos. Isso preserva a regra de produto: um Workspace reduz ruido, mas nao prende informacao.

## 5. Decisao de UX

A UI inicial permite:

- criar e editar Workspace;
- arquivar Workspace;
- marcar Projects pertencentes ao Workspace;
- marcar Apps pertencentes ao Workspace;
- abrir Project ou App a partir do detalhe do Workspace.

Search continua atravessando o sistema inteiro.
