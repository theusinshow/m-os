# ROADMAP.md — CronoCAD

Fases pequenas e verificaveis. Cada fase: implementar -> formatar -> lint ->
testes -> build -> corrigir -> atualizar docs.

## Fase 0 — Fundacao documental  ✅ concluida
- Ambiente analisado; ferramentas confirmadas (Node, Rust+MSVC, WebView2).
- Documentacao criada (`CLAUDE.md`, `docs/*`, `README`, `CHANGELOG`).
- Backlog e criterios de aceite registrados (este arquivo + PRODUCT).

## Fase 1 — Scaffold e interface base  ✅ concluida
- Tauri 2 + React + TS + Vite; Tailwind; ESLint/Prettier/Vitest.
- Layout com navegacao; rotas (Painel, Projetos, Historico, Relatorios,
  Configuracoes); tokens visuais.
- Dashboard funcional com dados temporarios tipados.
- Bandeja inicial; fechar-para-bandeja.
- Camada de banco + pasta de migrations (esquema 0001).
- **Criterio de conclusao:** compila (frontend e backend), testes passam,
  build gera instalador. ✔

## Fase 2 — Banco e cadastros  ✅ concluida
- Acesso ao banco no Rust via pool do plugin (`preload` + `database::pool`),
  sem SQL exposto ao frontend.
- Camada `repository` (sqlx) + comandos Tauri validados para CRUD de clientes e
  projetos; camada de servicos/stores no frontend.
- Telas de criacao/edicao (projetos + gestao de clientes) substituindo os mocks
  da tela de Projetos; pesquisa, concluir e arquivar.
- **17 testes Rust** (dominio + persistencia/validacao, incluindo a garantia de
  cronometro unico no banco).
- **Conclusao:** criar cliente/projeto, definir valor/hora, dados persistem
  localmente. ✔

## Fase 3 — Motor do cronometro  ✅ concluida
- Regras de dominio (`domain::timer`) ligadas ao `active_timer` via
  `repository::timer` e `timer_service` (start/pause/resume/stop/discard).
- Unico cronometro ativo; snapshot de valor/hora ao encerrar (transacao que
  cria o `time_entry` e remove o `active_timer`); eventos em `activity_events`.
- Recuperacao apos fechamento inesperado: `RecoveryModal` (manter/encerrar/
  descartar) no startup, nunca decidindo silenciosamente.
- Bandeja reflete o estado (itens habilitados/desabilitados + projeto atual) e
  aciona pausar/continuar/encerrar; evento `timer-state-changed` re-sincroniza
  o frontend sem polling.
- Painel interativo; Historico e Relatorios passam a usar sessoes reais.
- **22 testes Rust** + **30 testes Vitest**.
- **Conclusao:** transicoes testadas; estado persiste a cada mudanca e o
  cronometro sobrevive a reinicializacao (recuperacao). ✔

## Fase 4 — Monitoramento do Windows  ✅ concluida
- Servico `monitoring` com `sysinfo` (so nomes de processos), nao bloqueante e
  desligavel; encerrado de forma limpa no `RunEvent::Exit`.
- Transicoes aberto/fechado sem repetir; eventos gravados em `activity_events` e
  emitidos como `monitored-app-opened`/`closed`.
- Notificacoes nativas com cooldown e "nao lembrar hoje"
  (`suppress_app_reminder_today`).
- Configuracoes editaveis e CRUD de programas monitorados na tela de
  Configuracoes; lembretes no app (escolher projeto ao abrir sem cronometro;
  encerrar/manter/pausar ao fechar com cronometro).
- **25 testes Rust** (inclui `diff_transitions` e cooldown) + 30 Vitest.
- **Conclusao:** abrir/fechar um programa monitorado gera lembrete; programas
  configuraveis. ✔

## Fase 5 — Inatividade  ✅ concluida
- `idle::run` com `GetLastInputInfo` (so o tempo ocioso, sem conteudo);
  `classify` puro e testado; loop nao bloqueante, desligavel e encerravel.
- Eventos `idle_started`/`idle_ended` em `activity_events` + Tauri; lembrete no
  app com manter/descontar/editar (nunca desconta sozinho).
- Migration 0002 (`active_timer.idle_seconds`); `discount_idle` acumula o
  inativo, que vai para a sessao ao encerrar (limitado a duracao bruta).
- **28 testes Rust** (classify + persistencia do desconto) + 30 Vitest.
- **Conclusao:** inatividade detectada e tratada por decisao do usuario. ✔

## Fase 6 — Historico e relatorios  ✅ concluida
- Sessao manual (`source = manual`), edicao (recalcula duracao), exclusao
  reversivel (soft delete) e restauracao; validacao de horarios e travessia de
  meia-noite (testada).
- Historico com filtros (periodo, cliente, projeto), faturavel/nao, totais e
  visao de excluidas.
- Relatorios com filtros, totais (reais/inativas/faturaveis/valor), separacao
  por tipo de atividade e lista detalhada; **arredondamento aplicado apenas na
  visualizacao** (tempo real preservado).
- Exportacao **CSV** (dialogo nativo + `tauri-plugin-dialog`) e **impressao**
  pelo sistema (`window.print`).
- **32 testes Rust** (meia-noite, horario invalido, manual/edicao/soft delete/
  restore) + **33 Vitest** (inclui CSV e datetime).
- **Conclusao:** correcao manual, filtros, calculo correto e relatorio + CSV. ✔

## Fase 7 — Reconstrucao do dia  ✅ concluida
- Tela "Linha do tempo detectada": eventos do dia a partir de `activity_events`
  (`list_activity_events`), com selecao de data.
- Identificacao de lacunas (programa aberto sem sessao) com `pairAppSessions` e
  `findGaps` (puros e testados); "Transformar em registro" pre-preenche o
  formulario e cria a sessao com `source = reconstructed`.
- **33 testes Rust** + **37 Vitest** (inclui a logica de linha do tempo).
- **Conclusao:** linha do tempo real e reconstrucao de lacunas. ✔

## Fase 8 — Estabilizacao  ✅ concluida
- Suite verde ponta a ponta (typecheck, lint, 37 Vitest, 33 Rust, clippy) e
  build/instalador Windows gerados a cada fase.
- Estilos de impressao (imprime so o conteudo, sem navegacao); rotulos
  acessiveis em botoes de icone, foco visivel e navegacao por teclado.
- Mocks removidos; documentacao de uso em `docs/USAGE.md` e checklist manual em
  `docs/UX-FLOWS.md`.

## Futuro (fora do escopo atual)
Login, sincronizacao/nuvem, painel web, mobile, assinaturas, nota fiscal,
integracao com DWG, plugin AutoCAD, IA, equipes, PDF, calendarios.
