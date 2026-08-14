# M/OS — Technical Foundation v0.2 App Registry

## 1. Objetivo

Este corte introduz o App Registry como fundacao de Core, nao como uma nova experiencia visual pesada.

O objetivo e permitir que o M/OS conheca ferramentas e softwares usados pelo usuario, preserve dados localmente e exponha funcoes estaveis para a interface futura.

## 2. Escopo deste corte

Deve existir:

- entidade local de App;
- persistencia SQLite;
- busca local por App;
- comandos Tauri para criar, editar, listar, arquivar e registrar abertura;
- comando Tauri para abrir App com politica explicita;
- seletor de arquivo ou pasta para reduzir cadastro manual de `path`;
- presets manuais de Apps sugeridos para dogfooding sem cadastro completo imediato;
- tipos e API TypeScript para consumo futuro;
- inclusao em export JSON e backup por banco local.

Nao deve existir ainda:

- tela pesada de Workspaces;
- dashboard visual do App Registry;
- integracoes profundas com apps externos;
- automacao;
- Hermes;
- GitHub;
- sync/cloud;
- plugin system.

## 3. Modelo inicial

Um App representa uma ferramenta acessivel pelo ecossistema M/OS.

Campos iniciais:

- nome;
- descricao;
- origem opcional, tratada como metadata inerte;
- alvo de abertura opcional;
- tipo do alvo: `url` ou `path`;
- lifecycle;
- datas de criacao, atualizacao e ultima abertura registrada.

App e Project continuam sendo conceitos diferentes.

## 4. Decisao arquitetural

O App Registry entra como modulo proprio de Core e repository proprio no storage.

Isso evita tratar Apps como Projects, Tasks ou Resources antes da hora, preservando o vocabulario definido em `CORE.md`.

## 5. Decisao de seguranca

Este corte permite abrir um App, mas somente atraves de comando nativo controlado pelo backend Tauri.

Politica inicial:

- `url`: somente `http://` e `https://`;
- `path`: somente alvo local existente;
- Apps arquivados nao sao abertos;
- Search abre o detalhe do App, nao executa o alvo diretamente.

A abertura e considerada aceita quando o sistema operacional aceita a acao. So depois disso o M/OS registra `last_opened_at`.

Presets sugeridos nunca sao inseridos automaticamente. O usuario precisa acionar a criacao, e o sistema cria apenas Apps ainda inexistentes pelo nome.

O incremento posterior documentado em `TECHNICAL-FOUNDATION-V0.2-APP-CATALOG.md` separa `source_url` do alvo operacional e torna o cadastro dos Apps conhecidos idempotente no backend.
