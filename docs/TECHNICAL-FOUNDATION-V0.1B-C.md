# M/OS - Technical Foundation v0.1b-c

## 1. Status

**Status:** implementada e validada localmente para dogfooding

**Data:** 2026-08-13

Este documento registra o corte funcional que completa o fluxo local da v0.1.
Nao adiciona cloud, iOS, Workspaces, Resources ou integracoes ao escopo atual.

## 2. Fluxo entregue

```text
Capture -> Inbox -> Task -> Project -> Kanban -> Search
```

- uma Capture pode originar no maximo uma Task nesta versao;
- a conversao e atomica: cria a Task e marca a Capture como processada;
- a Capture original permanece imutavel e acessivel pela proveniencia;
- Tasks diretas nao simulam uma Capture de origem;
- Project e opcional para Task;
- Project arquivado nao arquiva suas Tasks implicitamente.

## 3. Modelo local

O schema `2` adiciona `projects` e `tasks` ao SQLite. IDs continuam UUIDv7,
timestamps continuam UTC e lifecycle permanece separado do estado de trabalho.

Task usa tres estados estaveis:

```text
backlog | doing | done
```

Entrar em `done` define `completed_at`. Reabrir limpa esse timestamp. A relacao
`tasks.source_capture_id` e unica e preserva a proveniencia sem duplicar conteudo.

## 4. Busca

Captures, Tasks e Projects possuem projecoes FTS5 reconstruiveis. A busca
unificada devolve um enum tipado e agrupa Capture, Task derivada e Project
relacionado no mesmo contexto, evitando resultados duplicados para uma unica
linha de pensamento.

Archive continua opt-in e Trash permanece fora dos resultados. Abrir uma Task
derivada permite navegar de volta para a Capture original.

## 5. Migration e recovery

Antes do upgrade de schema `1` para `2`, o adapter cria um snapshot consistente
`pre-migration-v1-*.db` pela SQLite Backup API. A migration e transacional e a
abertura verifica integridade e pragmas antes de liberar o repositorio.

Restore aceita backup de schema anterior, valida o pacote antes de substituir o
dataset, cria safety backup do estado atual, migra e reconstroi as projecoes de
busca. Falha em qualquer etapa nao deve produzir sucesso parcial.

## 6. Exportacao

O export JSON e legivel, versionado e inclui Captures, Projects e Tasks. Ele e
uma saida de propriedade de dados, nao um formato de restore e nao promete
importacao. Backup e export permanecem em texto claro e a interface explicita
que podem conter dados pessoais.

## 7. Design system

O renderer usa `Design System/design_handoff_frontend/mos-tokens.css` como fonte unica para cor,
tipografia, espacamento, geometria e motion. Schibsted Grotesk e JetBrains Mono
sao empacotadas localmente. Os componentes usam SVGs proprios e nao dependem de
biblioteca de icones ou UI kit.

Extensoes operacionais adicionadas aos tokens foram limitadas a dimensoes de
layout, papeis de z-index, backdrop, linha, marker e focus ring. Dark e padrao;
light e forced colors mantem paridade funcional.

Superficies implementadas:

- rail global de 52 px;
- Capture sem caixa e Quick Capture de 640 px;
- Inbox e Projects em lista/detalhe;
- Kanban Backlog, Doing e Done;
- drawer de Task sem backdrop;
- Command Search de 720 px;
- Settings, estados vazios, falha, confirmacao e Undo.

## 8. Empacotamento

Localmente:

```powershell
cd apps\desktop
npm ci
npm run tauri build
```

O workflow `Package Windows` executa o mesmo build em `windows-latest` e publica
um instalador NSIS por usuario (`installMode: currentUser`) e o executavel
portatil como artifacts. Ele roda manualmente ou em tags `v*`.

## 9. Validacao automatizada

- dominio: validacao de nomes, estados e UUIDs;
- persistencia: atomicidade, proveniencia, timestamps e busca agrupada;
- migration: preservacao de Capture e snapshot pre-migration;
- manutencao: backup/restore, adulteracao, rebuild e export JSON;
- renderer: TypeScript e bundle Vite;
- workspace: formatacao, Clippy e testes no CI.
- release local: executavel e NSIS por usuario gerados com sucesso.

## 10. Limites conscientes

- single-user, Windows 11 x64 e local-only;
- sem sincronizacao, conta, telemetria ou criptografia propria;
- sem ordenacao persistente dentro das colunas do Kanban;
- sem exclusao definitiva;
- sem importacao do JSON exportado;
- iOS e cloud permanecem decisoes futuras, sem contratos prematuros nesta base.
