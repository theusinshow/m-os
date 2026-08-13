# M/OS - Technical Foundation v0.1a

## 1. Status

**Status:** implementada e pronta para dogfooding local

**Data:** 2026-08-13

Este documento descreve apenas a fundacao executavel do marco `v0.1a`. Nao
antecipa Tasks, Projects, Kanban, cloud ou iOS.

## 2. Estrutura

```text
apps/desktop/                  shell Tauri e renderer React
  src/                         superficies e contrato IPC tipado
  src-tauri/                   commands, tray, shortcut e lifecycle Windows
crates/mos-core/               dominio e casos de uso de Capture
crates/mos-storage-sqlite/     SQLite, FTS, migrations e backup/restore
spikes/desktop-shell/          spike descartavel anterior ao produto
```

Dependencias apontam para dentro:

```text
React -> Tauri commands -> mos-core ports <- mos-storage-sqlite
```

O Core nao conhece Tauri, React, SQLite ou cloud.

## 3. Fluxos entregues

### Capture

1. Home ou Quick Capture envia apenas `content` e `source`.
2. Core valida e cria UUIDv7 e timestamp UTC.
3. SQLite insere Capture e projecao FTS na mesma transacao.
4. A UI mostra sucesso somente depois do commit.
5. A primeira alteracao do dia dispara snapshot local em background.

### Inbox e lifecycle

- `processing_state`: `inbox | processed`;
- `lifecycle_state`: `active | archived | trashed`;
- restaurar Archive ou Trash preserva `processing_state`;
- nenhuma exclusao definitiva existe na v0.1a;
- a interface oferece Undo para as mutacoes da Inbox.

### Search

- FTS5 com tokenizer Unicode e remocao de diacriticos;
- prefix match por token;
- Archive e opt-in; Trash e sempre omitida;
- resultado abre a Capture original em um visualizador;
- projecao derivada pode ser reconstruida a partir de `captures`.

### Backup e restore

- snapshot consistente pela SQLite Backup API;
- pacote ZIP versionado com extensao `.mos-backup`;
- manifest com schema, contagem, tamanho e SHA-256 do banco;
- validacao completa antes de alterar o dataset;
- safety backup obrigatorio antes do restore;
- snapshot diario com sete retencoes.

Backups permanecem em texto claro e a UI informa essa caracteristica.

## 4. Armazenamento local

O runtime usa o diretorio de dados resolvido pelo Tauri para o identificador
`com.codedbym.mos`:

```text
m-os.db
m-os.db-wal
m-os.db-shm
settings.json
backups/
```

Baseline SQLite:

- `journal_mode=WAL`;
- `synchronous=FULL`;
- `foreign_keys=ON`;
- `trusted_schema=OFF`;
- `busy_timeout=1000ms`;
- `quick_check` antes de migrations e depois de restore.

## 5. Contratos de falha

Erros IPC sao estruturados com `code`, `message` e `retryable`. O renderer
preserva o draft quando uma Capture falha. Os testes exercitam:

- validacao de conteudo vazio;
- UUIDv7;
- commit atomico de Capture e FTS;
- rollback quando FTS falha;
- lock concorrente sem falso sucesso;
- banco cheio sem registro parcial;
- rebuild do indice;
- lifecycle preservado no restore;
- backup/restore round-trip com safety backup;
- rejeicao de backup adulterado antes do restore.

## 6. Shell Windows

- processo single instance;
- fechar janela esconde no tray;
- tray abre janela principal, Quick Capture ou encerra;
- Quick Capture fica sempre no topo e fora da taskbar;
- atalho global padrao `Ctrl+Shift+Space`;
- alteracao do atalho e persistida em `settings.json`;
- `Ctrl+K` abre Search;
- CSP nao permite carregamento remoto de scripts.

## 7. Comandos

```powershell
# testes Rust
cargo test --workspace

# typecheck e bundle do renderer
cd apps\desktop
npm run build

# aplicativo em desenvolvimento
npm run tauri dev
```

## 8. Validacao executada

- workspace Rust: 11 testes aprovados;
- TypeScript e Vite: build aprovado;
- shell aberta como aplicativo Windows real;
- Home, Inbox vazia, Search e Quick Capture inspecionadas por screenshot;
- controles e landmarks inspecionados pela arvore UI Automation;
- Quick Capture corrigida para nao exibir scrollbar interna;
- contrastes medidos nos temas claro e escuro.

## 9. Limites conscientes

- `v0.1a` e single-user e local-only;
- nao ha sincronizacao, autenticacao ou telemetria;
- nao ha edicao nem exclusao definitiva de Capture;
- export JSON, hardening de upgrades e packaging final pertencem a `v0.1c`;
- iOS continua plataforma futura, sem codigo ou contrato prematuro neste marco.

O proximo gate e dogfooding diario desta fundacao. `v0.1b` so deve iniciar
depois de validar confianca de captura, reencontro e operacao da Inbox.
