# Changelog

Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/);
versionamento [SemVer](https://semver.org/lang/pt-BR/).

## [Nao lancado]

### Corrigido — Falha ao iniciar sumia sem dizer o motivo
- O `run()` terminava em `.expect(...)`: qualquer erro ao subir (tipicamente uma
  migration que nao aplica) virava **abort do Windows, sem janela e sem
  mensagem**. Na pratica o app "sumia" e nao havia como o usuario saber o
  porque — um dia inteiro de trabalho ficou sem registro por causa disso.
- Agora um **dialogo nativo** mostra o motivo antes de encerrar. A regra
  critica 1 (confiabilidade dos registros) inclui o usuario **saber** quando o
  app nao esta gravando.
- Caso real que motivou a mudanca: a migration 0005 foi aplicada num banco real
  por um `tauri:dev` e teve o bloco de **comentarios** alterado antes do commit.
  O `sqlx` faz checksum do arquivo inteiro, comentarios inclusive, e passou a
  recusar o boot — embora o schema estivesse correto. Ver
  `docs/superpowers/specs/2026-07-17-falha-ao-iniciar.md`.

### Corrigido — App nao abria quando ja estava rodando na bandeja
- O app **fecha para a bandeja** e continua vivo, mas nao havia guarda de
  instancia unica. Abrir pelo icone com ele escondido subia um **segundo
  processo** sobre o mesmo SQLite: a janela nao aparecia e os dois processos
  disputavam o banco — o cenario que a regra critica de integridade dos
  registros existe para evitar.
- Agora, com `tauri-plugin-single-instance`, a tentativa de abrir de novo apenas
  **traz a janela existente de volta** (reusa `tray::show_main_window`).

### Adicionado — Adicionar tempo esquecido
- Registrar trabalho esquecido exigia o formulario de sessao manual, com
  **inicio e fim** em campos de data-hora. Mas quem esqueceu de ligar o
  cronometro nao lembra do horario — lembra da **duracao**. Ter que inventar um
  horario fazia o registro ser adiado, e adiado virava esquecido.
- Novo modal **"Adicionar tempo esquecido"** (`QuickTimeModal`): projeto, dia,
  nota e botoes de incremento (`+15min`, `+30min`, `+1h`, `+2h`, `-15min`,
  `Limpar`) que somam um total antes de salvar. Teto de 24h por lancamento;
  Salvar so habilita com total maior que zero.
- Aberto de tres lugares: o **painel do cronometro** parado ("Esqueceu de
  registrar?"), o cabecalho do **Historico**, e uma acao por linha da tabela,
  ancorada naquela sessao.
- O horario e derivado da duracao (`src/lib/quickTime.ts`): o bloco termina
  onde a sessao ancora comeca, ou agora (hoje), ou no fim da ultima sessao do
  dia, ou as 18:00 num dia vazio. O inicio e sempre `fim - duracao`.
- **A sessao original nunca e alterada.** O tempo somado a uma sessao curta
  nasce como um registro `manual` separado, e o historico continua distinguindo
  o que foi cronometrado do que foi estimado.

### Adicionado — Confirmacao ao encerrar o cronometro
- Encerrar grava a sessao em `time_entries` e e **irreversivel**, mas disparava
  com um unico clique. Agora abre uma confirmacao (`StopConfirmModal`) com o
  tempo ao vivo, o valor que sera gravado e a saida **"Pausar em vez disso"**
  como acao primaria — o erro mais provavel e querer pausar e encerrar por
  engano. Com o cronometro ja pausado, essa opcao nao aparece.
- O cronometro **continua correndo** enquanto o modal esta aberto: a decisao e
  adiada, o tempo nao se perde.

### Alterado — Hierarquia dos botoes Pausar/Encerrar
- O app promovia o **Encerrar** (irreversivel) sobre o **Pausar** em tres
  lugares: no painel do cronometro e nos avisos de "CAD fechado" e "sair do
  app", onde Encerrar era ate o botao primario. Agora **Pausar e a acao
  primaria** nos tres, e Encerrar e discreto.

### Alterado — Novo design system ("Design Language")
- Paleta redesenhada (`src/styles/tokens.css`, mesmos nomes de token): base
  escura com temperatura verde (`#000F08`, nunca preto puro), estrutura off-white
  quente (`#F5F2ED`) e um **sinal vermelho** raro (`#FB3640`) para acao/foco/
  estado ativo. Modo escuro e claro (off-white quente).
- Linguagem **angular**: raios = 0; foco `2px solid` no sinal com offset 3px.
- Tipografia **Panchang** (display) + **Satoshi** (corpo), **empacotadas
  localmente** (woff2 em `public/fonts`, `@font-face` proprio) para funcionar
  100% offline; CSP com `font-src 'self'`.
- Titulos, wordmark e numeros em Panchang; cartao do cronometro com acento de
  sinal; estado "em execucao" usa o sinal (gravando), "pausado" em ambar.
- Novo icone: monograma "C" no vermelho-sinal sobre a base escura.

### Alterado — Renomeacao para CronoCAD
- Produto renomeado de **HoraCAD** para **CronoCAD**: nome de exibicao
  (`APP.name`), `productName`, `identifier` (`com.cronocad.app`), crate/lib
  (`cronocad`/`cronocad_lib`), banco (`cronocad.sqlite`), textos (bandeja,
  notificacoes, PDF/fatura) e documentacao. O identificador novo usa um
  diretorio de dados proprio; a instalacao antiga do HoraCAD pode ser
  desinstalada. Icone sera atualizado no proximo redesign.

### Adicionado — Widget flutuante sobre o CAD
- Nova janela `reminder`: pequena, sem bordas, **sempre no topo** e fora da
  barra de tarefas. Ao abrir um programa monitorado **sem cronometro ativo**, o
  backend a posiciona no canto superior direito e a exibe **sobre o CAD**
  (`reminder::show`), no lugar da notificacao de abertura.
- `ReminderWidget`: pergunta em qual projeto iniciar (pre-seleciona o mais
  recente) com **Iniciar / Ignorar / Nao lembrar hoje**; `main.tsx` roteia por
  rotulo de janela (widget vs app principal). O lembrete de "programa aberto"
  saiu da janela principal (evita prompt duplicado). Capability estendida a
  janela `reminder`.

### Adicionado — Melhorias (cronometro esperto, cobranca, UX)
- **Cronometro esperto**: metas de horas por projeto (migration 0003 +
  `budget_minutes`), barra de progresso e alerta ao estourar; **inicio em 1
  clique** dos projetos recentes no Painel; pre-selecao do ultimo projeto usado
  no Painel e no lembrete de "programa aberto". Comando `list_project_totals`.
- **Cobranca**: dados do **emissor** nas Configuracoes (migration 0004);
  **fatura por cliente em PDF** (`export_invoice_pdf`, cabecalho do emissor +
  itens + total); atalhos de periodo (Hoje / Este mes / Mes passado / Tudo) e
  **ajuste percentual** (desconto/acrescimo) refletido no total, no PDF e na
  fatura.
- **UX**: **modo claro persistente** (localStorage); **onboarding** de primeira
  execucao no Painel quando ainda nao ha projetos.

### Adicionado — Polimentos pos-MVP
- **Confirmacao ao sair com cronometro ativo** pela bandeja: o menu "Sair
  completamente" traz a janela e abre um dialogo (pausar e sair / encerrar e
  sair / sair assim mesmo / cancelar) via evento `request-quit` + comando
  `quit_app`. Sair sem encerrar continua seguro (recuperacao).
- **Exportacao PDF** do relatorio (`export_report_pdf`, com `printpdf` +
  dialogo nativo) — botao "Exportar PDF" nos Relatorios, paginado.
- **Iniciar com o Windows** aplicado de fato via `tauri-plugin-autostart`:
  `update_settings` sincroniza o autostart do SO e o estado e alinhado no
  startup.

### Adicionado — Fases 7 e 8 (Reconstrucao do dia e estabilizacao)
- Tela **Linha do tempo detectada** (`/linha-do-tempo`): eventos do dia via
  `list_activity_events` + `repository::activity_events`; deteccao de lacunas
  com `pairAppSessions`/`findGaps` (puros e testados) e "Transformar em registro"
  criando sessao `source = reconstructed` (formulario pre-preenchido).
- `save_text_file`/create passam a aceitar a origem `reconstructed`.
- Estabilizacao: estilos de impressao (imprime so o conteudo), passe de
  acessibilidade (rotulos, foco visivel), remocao dos ultimos mocks, guia de uso
  `docs/USAGE.md`.
- Testes: **33 Rust** + **37 Vitest** (inclui `lib/timeline`).

### Adicionado — Fase 6 (Historico e relatorios)
- Sessoes manuais (`create_time_entry`), edicao (`update_time_entry`, recalcula
  a duracao), exclusao reversivel (`delete_time_entry` / soft delete) e
  restauracao (`restore_time_entry`). Validacao de horarios com suporte a
  travessia de meia-noite (timestamps absolutos).
- Historico com filtros por periodo, cliente e projeto; marca de nao faturavel;
  totais; visao de sessoes excluidas com restaurar.
- Relatorios com filtros, totais (horas reais/inativas/faturaveis e valor),
  separacao por tipo de atividade e lista detalhada. **Arredondamento aplicado
  apenas na visualizacao/cobranca** (o tempo real permanece intacto).
- Exportacao **CSV** via dialogo nativo (`tauri-plugin-dialog` + `save_text_file`)
  e **impressao** pelo sistema.
- Frontend: `EntryForm`, filtros, `lib/datetime` e `lib/csv`; store de sessoes
  com create/update/remove/restore.
- Testes: **32 Rust** (meia-noite, horario invalido, manual/edicao/soft delete/
  restore) + **33 Vitest** (CSV, datetime).

### Adicionado — Fase 5 (Inatividade)
- Deteccao de inatividade (`idle`) via `GetLastInputInfo` do Windows — le apenas
  o tempo desde a ultima entrada, nunca teclas/coordenadas/conteudo (secoes
  6 e 11). `classify` puro e testado; loop nao bloqueante, desligavel e
  encerrado de forma limpa no `RunEvent::Exit`.
- Eventos `idle_started`/`idle_ended` (em `activity_events` e Tauri). Ao voltar
  a atividade com cronometro ativo, o app abre o lembrete manter/descontar/
  editar — nunca descontando automaticamente.
- Migration **0002**: coluna `active_timer.idle_seconds`. Comando `discount_idle`
  acumula o tempo inativo, que e transferido para `time_entries.idle_seconds` ao
  encerrar (limitado a duracao bruta, garantindo net >= 0).
- Frontend: `discountIdle` no store, `IdlePrompt` (com minutos editaveis).
- Testes: **28 Rust** (classify de transicoes, desconto persistido) + 30 Vitest.

### Adicionado — Fase 4 (Monitoramento do Windows)
- Servico de monitoramento (`monitoring`) com `sysinfo` (le apenas nomes de
  processos — secao 6, sem shell): loop nao bloqueante e desligavel, encerrado
  de forma limpa ao sair (`MonitorShared::stop` no `RunEvent::Exit`).
- Deteccao de transicoes aberto/fechado (`diff_transitions`, pura e testada) sem
  repetir eventos; gravacao em `activity_events` e emissao de
  `monitored-app-opened`/`monitored-app-closed`.
- Notificacoes nativas com cooldown e supressao "nao lembrar hoje"
  (`suppress_app_reminder_today`).
- Repositorios e comandos de `settings` e `monitored_apps` (get/update + CRUD).
- Frontend: services/stores de configuracoes e apps; lembretes no app
  (`MonitorPrompts`): escolher projeto ao abrir sem cronometro; encerrar/manter/
  pausar ao fechar com cronometro. Tela de **Configuracoes** totalmente editavel
  (monitoramento, inatividade, arredondamento, comportamento) e CRUD de
  programas monitorados.
- Testes: **25 Rust** (diff de transicoes, cooldown) + 30 Vitest.

### Adicionado — Fase 3 (Motor do cronometro)
- Motor do cronometro no backend: `repository::timer` (start/pause/resume/stop/
  discard) usando `domain::timer`, e `timer_service` que emite
  `timer-state-changed` e atualiza a bandeja. Comandos Tauri correspondentes.
- Ao encerrar, cria-se um `time_entry` com **snapshot** do valor/hora, dentro de
  uma transacao que tambem remove o `active_timer`; eventos gravados em
  `activity_events`.
- **Recuperacao** de cronometro apos fechamento inesperado: modal no startup
  com manter/encerrar/descartar (nunca decide silenciosamente).
- Bandeja reflete o estado (habilita/desabilita itens e mostra o projeto atual)
  e aciona pausar/continuar/encerrar.
- Frontend: services (`timer`, `timeEntries`), stores (`timerStore`,
  `entriesStore`), `TimerPanel` interativo, `RecoveryModal`, e sincronizacao por
  evento (`useAppSync`) sem polling. Painel, Historico e Relatorios passam a
  usar **sessoes reais**; mocks de clientes/projetos/sessoes removidos.
- Testes: **22 Rust** (ciclo start/pause/resume/stop, conflito de segundo
  cronometro, snapshot congelado, descarte) e **30 Vitest** (inclui logica de
  recuperacao do store).

### Adicionado — Fase 2 (Banco e cadastros)
- Acesso ao banco no backend via o **mesmo** pool `sqlx` do plugin oficial
  (`preload` no `tauri.conf.json` + helper `database::pool`), sem expor SQL ao
  frontend (secao 19).
- Camada `repository` (sqlx) e `models` com validacao para clientes e projetos.
- Comandos Tauri de CRUD validados: `list/get/create/update/archive_client` e
  `list/get/create/update_project`, `set_project_status`.
- Frontend: servicos tipados (`services/clients`, `services/projects`), store de
  catalogo (Zustand) e telas de criacao/edicao — formulario de projeto com
  seletor de cliente e valor/hora, modal de gestao de clientes, pesquisa,
  concluir e arquivar. A tela de Projetos passa a usar **dados reais**.
- Primitivos de UI: `Modal`, `Field`, `Input`, `Textarea`, `Select`.
- Testes de persistencia/validacao em Rust (total: **17 testes Rust**), com
  banco SQLite em memoria aplicando a migration real, incluindo a garantia de
  cronometro unico no nivel do banco.

### Adicionado — Fundacao (Fases 0 e 1)
- Scaffold Tauri 2 + React 18 + TypeScript (estrito) + Vite 6.
- Tailwind CSS 3 com tokens visuais centralizados (`src/styles/tokens.css`),
  dark-first e preparado para modo claro.
- ESLint (flat config, `no-explicit-any`), Prettier e Vitest configurados.
- Layout principal com navegacao lateral e barra superior; alternador de tema.
- Rotas: Painel, Projetos, Historico, Relatorios, Configuracoes.
- Dashboard funcional com dados temporarios tipados: cartao do cronometro
  (elemento principal), resumos, sessoes recentes, linha do tempo e alertas.
- Telas base de Projetos, Historico, Relatorios e Configuracoes.
- Regras de dominio puras com testes espelhados (frontend Vitest e backend
  `cargo test`): calculo de duracao, pausa/retomada, desconto de inatividade,
  arredondamento e calculo monetario (27 testes TS + 9 testes Rust).
- Backend Rust: comando `app_info`, tipo de erro unificado, modulos de
  monitoramento e notificacoes como contratos (contornos para fases futuras).
- Bandeja do sistema com menu inicial e comportamento "fechar para a bandeja".
- Banco SQLite via `tauri-plugin-sql` com migration inicial versionada
  (`0001_initial_schema.sql`): clientes, projetos, sessoes, cronometro ativo
  (unico garantido no banco), apps monitorados, eventos e configuracoes.
- Capabilities Tauri com permissoes minimas; sem SQL arbitrario no frontend.
- Icones do app gerados; identificador `com.cronocad.app`.
- Documentacao: `CLAUDE.md`, `README.md`, `docs/PRODUCT.md`,
  `docs/ARCHITECTURE.md`, `docs/DATABASE.md`, `docs/UX-FLOWS.md`,
  `docs/ROADMAP.md`.

### Observacoes
- Motor do cronometro, monitoramento de processos e deteccao de inatividade
  ainda **nao** implementados (apenas contratos/telas). Ver `docs/ROADMAP.md`.
