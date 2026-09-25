# M/OS — auditoria de produto e inteligência

Data: 2026-09-24. Escopo: arquitetura, integração e loop operacional do checkout
v0.6.0. Esta entrega contém auditoria e correções executadas. **Não equivale à
conclusão de toda a visão Zero-Organization**, nem demonstra adoção diária.

## Conclusão de produto

O produto já tem boa parte da infraestrutura pedida: captura antes de
classificação, planejamento sugerido, atenção contextual, Rescue Mode, jobs de
reunião, ações Hermes e sync. O maior problema observado neste corte é a
continuidade entre essas peças: uma intenção desaparecia depois de encerrar o
dia; adiar não retirava a Task de Agora; falhas de escrita não eram mostradas;
algumas operações podiam deixar estado parcial ou emitir alterações fantasma.
Corrigir esse caminho reduz mais trabalho administrativo que acrescentar uma
nova Home, um segundo agente ou um gráfico de relações.

Pulse já tem um equivalente implementado: `mos-core::piloto`. Seus módulos
`atencao`, `proximo`, `planejador`, `resgate`, `avisos` e `autopilot` compõem o
Panorama usado pela Home, pelos avisos e pelo preâmbulo do Hermes. A decisão é
evoluir esse motor, preservando razões legíveis e separando sugestão de escrita.
Não foi criado um segundo motor chamado Pulse. Ver ADR-073 em `DECISIONS.md`.

## Arquitetura encontrada

| Camada | Implementação real | Fronteira relevante |
| --- | --- | --- |
| Domínio | `crates/mos-core`: entidades, regras, ports, serviços, Piloto, contratos de ação e contexto | Sem dependência de Tauri, SQLite ou plataforma |
| Persistência | `crates/mos-storage-sqlite`: SQLite, migrations até 0042, FTS, transações e emissão | Mutação e outbox pertencem ao mesmo commit |
| Sync | `mos-sync`, `mos-sync-http`, `mos-sync-server`; projeção e cobertura no storage | Operações por campo, relógio lógico, conflitos, tombstones, retry e cursor |
| Desktop | React + Tauri em `apps/desktop` | Captura global, áudio, arquivos e integrações locais |
| Bolso | Axum + React PWA em `apps/mos-web`, com SQLite próprio no servidor | HTTP até a réplica; não é banco offline no iPhone |
| Hermes | `mos-hermes` transporta gateway; `jarvis.rs` monta contexto e executa ações | Executor ainda acoplado ao shell desktop; modelo efetivo depende do gateway |
| Ingestão | `mos-ingest`, `mos-audio`, `mos-transcribe`; jobs no storage | Arquivo original e áudio locais; processamento posterior à captura |
| Observabilidade | saúde do sync, outbox/conflitos, jobs, histórico de ações, usage, Guardian | Sem evidência nesta auditoria de métricas de adoção completas |

A documentação é cumulativa: README ainda descrevia v0.2 sem sync; PLATFORMS
mistura contrato futuro com fases anteriores; o contrato multi-device não prova
que cada superfície já expõe cada serviço. Notas de estado foram acrescentadas
sem apagar o histórico das decisões. Não há `apps/ios`, APNs ou bridge nativa
iOS neste checkout. A versão do pacote da UI web ainda difere da do workspace.

### Dados e relações

Tasks possuem estado, prazo (`due_at`), dia pretendido (`scheduled_for`), início,
prioridade, estimativa, dependência, `waiting_for`, follow-up, checklist e
subtasks. Waiting For é atributo da Task/lembrete, não uma entidade Person.
Resources ligam-se a Tasks, Projects e Workspaces. Captures preservam origem;
insights de reunião preservam evidência e podem originar ações. Há relações
úteis; não há grafo genérico de entidades nem Person de primeira classe.

O Calendar compõe fatos e compromissos de outros módulos. Não há CRUD genérico
de Calendar Event com integração Google/Outlook. Timeline é memória composta,
não event sourcing de todas as mudanças do produto.

O registro executável de sync é `sync_cobertura.rs`: 30 tabelas classificadas
como sincronizáveis. Reuniões, segmentos, análises, insights, índices de
transcrição, áudio, Voice Notes e ingestões permanecem locais. Tasks extraídas
sincronizam. Os vínculos são entidades de relação, com IDs derivados e remoção
lógica. Horas concluídas viajam; cronômetro ativo e preferências de aparelho não.

## Checklist dos oito eixos

| Eixo | Decisão e resultado |
| --- | --- |
| Core | Carry-over de sessão encerrada, respeito ao planejamento futuro, ranking e contexto estruturado no domínio puro. `DailyService` expõe início e encerramento planejados. |
| Database | Início, carry-over, planejamento e emissão atômicos. Encerramento inclui movimento e pausa das Tasks no mesmo commit. Sem nova migration nem alteração de dados reais. |
| Sync | Emissão apenas para reordenação efetiva; testes de IDs alheios e duas réplicas. Troca de foco emite ambas as pontas. Protocolo e geração mantidos. |
| Desktop | Comandos delegam aos serviços; Home mostra falha, oferece proposta e ação direta. Encerramento distingue erro de leitura e escrita. Edição prévia de `App.css` preservada. |
| iOS | PWA recebe o mesmo motor e início atômico; falhas HTTP preservam a última leitura e são visíveis. Itens de atenção abrem destinos disponíveis. Encerramento assistido ainda não tem superfície/rota no bolso; serviço portável pronto. Não há build nativo a validar. |
| Notifications | Mantidos política e dedupe existentes; nenhum novo canal, polling de IA ou aviso acrescentado. |
| Hermes | Tipos filtrados antes do teto; transcrições participam da descoberta; Task anexada inclui contexto de execução limitado. Nenhuma autoridade nova de escrita concedida. |
| Tests | Regressões de domínio, transação, emissão, foco e duas réplicas; suites Rust/TypeScript, builds e bancada visual com falhas simuladas. |

## Inventário de features

Legenda: **I** = implementação encontrada; **P** = cobertura parcial do pedido;
**N** = não encontrada como feature independente. “Hermes” indica capacidade
observada no contrato/executor, não teste contra gateway de produção. “Atrito” é
avaliação de fluxo por código/bancada, não observação de frequência real de uso.
Nenhuma feature pode ser declarada abandonada só por estar escondida: não foram
consultados dados pessoais de uso nem entrevistas. Manutenção e uso efetivo não
são inferíveis pela mera existência de código/testes.

| Feature / evidência principal | Estado e valor | Integração / Hermes | Atrito, lacuna e decisão |
| --- | --- | --- | --- |
| Home operacional (`HomePiloto.tsx`, `piloto`) | I: Agora, Hoje, Próximo e Atenção | Mesmo motor desktop/bolso; Hermes recebe atenção | Reduz escolha; corrigidos adiamento e feedback. Manter |
| Pulse / priorização (`piloto/atencao.rs`) | I como Piloto; razões, severidade, ação | Tasks, lembretes, academia, reuniões, Inbox e sync | Não duplicar nem acrescentar dashboard. Fundir conceito |
| Quick Capture (`capture`, shell desktop) | I: texto sem classificação obrigatória | Capture é persistida antes de organização; Hermes cria | Atalho/tray dependem da plataforma. Manter; tempo real não medido |
| Inbox / Archive / Trash (`capture`, App) | I: captura e recuperação | Conversão em Task e Resource; Hermes converte Capture | Organização em lote ainda pede revisão. Melhorar triagem existente |
| Universal Drop Zone (`DropZone.tsx`, `mos-ingest`) | I: texto, URL, arquivo | Capture primeiro, processamento depois | Arquivos locais não viram anexos móveis automaticamente. Manter |
| Voice Inbox (`Voice.tsx`, `voice.rs`) | I no desktop | Transcrição, interpretação, propostas e undo | Depende de áudio/transcritor; sem paridade móvel. Terminar cobertura |
| Tasks / Kanban (`work`, `work_repository`) | I: trabalho executável | Projetos, Captures, objetivos, acadêmico e Hermes | Boa base; foco local agora atômico. Manter |
| Checklist / subtasks / referências (`TaskDetail`) | I: execução dentro da Task | Relações e checklist sincronizados; Hermes lê os textos anexados e tem ações de checklist | Sem entidade extra por passo simples. Manter |
| Waiting For (`waiting_for`, `follow_up_at`) | I como atributo | Atenção reaparece com follow-up; contexto Hermes ampliado | Não resolve identidade de pessoas. Manter modelo leve |
| Projects / Workspaces (`work`, relações) | I: contexto e agrupamento | Tasks, Resources e Apps; Hermes cria Project e associa Task | Algumas associações manuais. Manter; não exigir Project na captura |
| Calendar / Timeline (`calendar`, `TempoTimeline.tsx`) | I como projeção temporal; P frente ao pedido | Dia, reuniões, lembretes, atividades acadêmicas e trabalho | Não é calendário externo nem log universal. Terminar consulta contextual |
| Calendar Event genérico | N | Sem contrato/tool independente | Não inventar CRUD só para preencher catálogo |
| Reminders (`attention`, scheduler) | I: agendamento, snooze, resolução e histórico | Targets, Hermes, sync e Web Push | Falhas/canais variam por aparelho. Manter dedupe e ações úteis |
| Start My Day (`planejador`, `DailyService`) | I: proposta de até quatro objetivos | Desktop e PWA usam serviço comum | Um clique, edição opcional. Corrigidos carry-over e atomicidade |
| End My Day (`EncerrarDia.tsx`, `daily_repository`) | I desktop; P no bolso | Placar, resoluções e movimento; Hermes tem DayEnd | Sem relatório obrigatório. Encerramento planejado agora atômico |
| Rescue Mode (`piloto/resgate`) | I: ausência e pendências contextualizadas | Home, sinais de Inbox/Tasks/sync/reunião | Hipótese de retenção ainda sem dados. Manter, medir resolução e dispensa |
| Weekly Review / obsolescência (`daily`, `stale`) | I: histórico e envelhecimento | Sessões, objetivos e contexto do trabalho | Pode virar ritual adicional; não colocar no caminho da captura |
| Meeting Recorder (`mos-audio`, `meeting.rs`) | I desktop: gravação e recuperação | Origina pipeline persistente | Depende de dispositivo real; não testado ao vivo nesta entrega |
| Meeting Agent V2 (`meeting_pipeline`, `meeting_jobs`) | I: transcrever → analisar → revisar | Evidências, decisões, propostas de Tasks e esperas | Falhas/retry têm estado; ainda exige disponibilidade dos serviços |
| Recording Guardian (`meeting_guardian`) | I: sinais, cooldown, sugestão/corte | Eventos locais, retenção e recuperação | Auto-stop é opcional; não mudar sem calibração real |
| Apagar / reprocessar reuniões (`meeting_repository`) | I: ciclo de vida e jobs | Protege originais e permite revisão | Dados locais; não confundir com sync de reunião entre aparelhos |
| Search unificada (`search`, FTS nos repositórios) | I lexical; P para busca vaga | Captures, trabalho, referências, reuniões e outros tipos | Sem embeddings/typo tolerance geral. Ranking Hermes corrigido |
| Hermes conversa (`mos-hermes`, `hermes.rs`) | I: gateway, streaming e persistência | Contexto explícito, histórico e protocolos de ação/query | Modelo/custo/latência reais não medidos; depende de gateway |
| Hermes actions (`action.rs`, `jarvis.rs`) | I, catálogo limitado | Capture, Task, checklist, Project, Resource, Reminder, tempo, dia, Finance | Não é controle irrestrito do produto. Executor precisa sair do shell para paridade |
| Context Engine (`agent.rs`, `jarvis.rs`) | I lexical + estrutura; P semântico | Entidade atual, candidatos, contexto anexado, dia e atenção | 12 candidatos; Task anexada limitada; conteúdo completo não é enviado automaticamente |
| Proactive Hermes (`piloto`, reuniões, voz) | P: automações especializadas existentes | Sugestões/eventos nos fluxos, política de avisos | Não há motor geral de confiança calibrada para toda escrita |
| Undo (`UndoStep`, auditoria de ações) | I por ação; P global | Conversão, checklist, plano, dia e insights | Há inversas incompletas; TaskStart não restaura todo o foco anterior. Corrigir antes de autoexecução ampla |
| Knowledge Graph / Person | P relações específicas; N grafo/Person genéricos | Task–Project–Resource e proveniência; pessoa como texto | Não criar visualização sem resolver consultas e identidade |
| M/Academic (`academic`, provider) | I: semestre, disciplinas, exames, entregas, estudo | Tasks, calendário, Resources e provedor | Importar provedor em duas instalações pode duplicar; refs do provedor são locais |
| Tempo / CronoCAD (`tracking`) | I: sessões, horas, projetos/clientes, migração | Horas sincronizam; Hermes inicia/registra | Relatórios úteis; timer ativo é local, por desenho |
| Biblioteca / arquivos (`resource`, ingestão) | I: notas, URLs e referências | Busca e relações; Hermes cria Resource | Metadata não transporta arquivo original. Manter e explicitar disponibilidade |
| Command / Functions / radial (`command`, App) | I: ações e navegação globais | Reusa fluxos; não precisa outra palette | Descoberta deve ser contextual; múltiplas entradas podem confundir |
| Apps / GitHub registry (`app_repository`) | I: catálogo e abertura controlada | Workspaces e origem GitHub | Não é integração geral de issues/PRs. Manter escopo do launcher |
| M/Finance (`FinancePage`, `finance.rs`) | I ponte externa/visualização | Ação especializada de conta | Integração depende do serviço externo; não validada ao vivo |
| Usage / faixa / Argos (`mos-usage`, UI) | I: consumo e presença visual | Dados locais, provedores e affordances de desktop | Não comprovam adoção do M/OS. Manter fora do caminho principal |
| Notificações desktop / PWA (`attention`, `push`) | I: políticas e transporte | Histórico, dedupe e quiet hours; ações | Sem prova de entrega física em iPhone nesta auditoria |
| Sync (`sync_cobertura`, `sync_projecao`) | I: réplica, outbox, conflito e retry | Desktop ↔ servidor web por protocolo comum | Corrigidas operações fantasma; não cobre todos os arquivos/módulos |
| Backup / restore / export / updater | I: recuperação e distribuição | Safety backup e validação de arquivo | Testes locais existentes; não foi instalado release no app em uso |
| iOS nativo / Share Sheet / APNs | N no checkout | PWA é a manifestação existente | Não anunciar como implementação concluída |

## Correções executadas e por quê

| Problema reproduzível | Correção | Evidência de regressão |
| --- | --- | --- |
| Reordenar com ID de checklist de outra Task não alterava o banco local, mas emitia operação | Só emitir quando UPDATE ativo afetar linha | `piloto_planejamento::reordenacao_de_outra_task_nao_emite_mudanca_fantasma`, duas réplicas |
| Mesmo defeito em objetivo de outra sessão | Checar número de linhas antes da emissão | `daily_session::reordenar_objetivo_alheio_nao_emite_operacao` |
| Encerrar ontem fazia intenções livres sumirem da próxima proposta | Ler objetivos anteriores e carregar pendentes/carried-over, deduplicados | Testes de `piloto/planejador` |
| Carry-over podia trazer uma lista excessiva ou ressuscitar Task indisponível | Teto de quatro e mesma elegibilidade de Agora | Testes do planejador |
| Adiar para amanhã continuava recomendando agora | Excluir planejamento futuro salvo início explícito; atenção ao prazo permanece | Testes de `piloto/proximo` |
| Repetir planejamento contava outro adiamento | Escrita idempotente para o mesmo dia | `piloto_planejamento::planejar_emite_so_o_dia_e_adiar_conta` |
| Iniciar Task inválida parava a anterior | Validar alvo e trocar foco numa transação com emissão dos dois campos | Teste de troca de foco e duas réplicas |
| Início automático gravava dia, carry-over e planejamento em passos separados | `start_planned`, flag transitória e transação no storage | Falha na segunda Task desfaz dia, carry-over, primeira Task e outbox |
| Encerramento movia Tasks antes de validar resoluções | `end_planned` e `end_day_planned`, movimento/pausa/fecho no mesmo commit | Falha após movimento faz rollback; duplicata/retry não incrementam contagem |
| Proposta de encerramento ignorava objetivos da sessão antiga | Usar sessão/objetivos antigos quando hoje não começou | Teste com sessão stale |
| Busca Hermes truncava antes de filtrar tipo | Ranking puro aplica filtro antes de 12 resultados | Teste com 20 Tasks e reunião solicitada |
| Reunião só era descoberta pelo título | Consultar também índice de transcrição; um voto por termo/entidade | Ranking deduplica título/transcrição; suites do índice existentes |
| Task anexada omitia instruções de execução | Contexto inclui checklist, subtasks, dependência, espera, follow-up e referências | Teste de TaskDetail real contra SQLite |
| Home engolia falhas de começar/concluir/adiar | Feedback visível e liberação para retry; controles ocupados consistentes | Bancada desktop/PWA com falha simulada |
| HTTP com falha apagava listas da UI e fingia Inbox vazia | Preservar última leitura e mostrar aviso com retry | Bancada PWA, refresh 503 com panorama retido |
| Atenção mobile só abria Tasks | Conectar lembretes, acadêmico, Inbox e sync a destinos disponíveis | Navegação da Inbox conferida em viewport 390×844 |
| Cópia do dia enfatizava uma obrigação não cumprida | “Uma proposta para hoje”, “Hoje importa”, “Começar meu dia” | Mesmo gesto; sem novo formulário nem redesign |

## Hermes: cobertura e limites

O gateway não recebe o banco inteiro. O preâmbulo traz contexto da tela, dia,
atenção e candidatos. `mos-query` oferece busca limitada; `mos-action` produz
proposta parseada e validada. A execução usa serviços e registra efeito/undo.
Contexto anexado é uma escolha explícita diferente da descoberta automática.

A melhoria desta entrega filtra o tipo antes do limite e encontra reuniões pelo
texto transcrito, mas **não entrega a transcrição inteira como candidato**.
Para responder “o que ficou decidido?”, ainda é necessário contexto da reunião
ou resultado estruturado suficiente. Não foi implementado RAG vetorial.
Tasks anexadas incluem até 20 passos, 10 subtasks e 10 referências; não há
benchmark novo de tokens/custo/latência. O modelo efetivo e seus preços não são
fixados por esta auditoria e não foram inferidos da marca Hermes.

Catálogo real: criar Capture/converter em Task; criar Task, mudar estado,
começar, planejar dia, projeto, checklist e plano (prazo/prioridade/espera);
criar Project/Resource; criar/snooze/resolver Reminder; iniciar/registrar tempo;
iniciar/editar/encerrar dia; ação especializada Finance. Não há ferramenta
universal de exclusão, Person, Calendar Event ou controle completo de gravação.
As entidades disponíveis na busca não implicam ferramentas de escrita sobre
elas. O executor ainda depende de `AppState`/Tauri.

Não ampliar autoexecução por um número de confiança autodeclarado pelo modelo.
O caminho recomendado é usar os eventos e jobs existentes, com três saídas:
**silenciosa** para projeção/metadata reversível, **sugestão** para decisões
ambíguas, **aviso** só quando houver urgência e ação. Evidência, dedupe, política
de risco e inversa completa precedem a promoção de uma sugestão a automação.
Hoje esse desenho já existe parcialmente em Piloto e Meeting Agent; não há
implementação global nova de auto-triagem nesta entrega.

## Loop de uso e atrito

1. Capturar texto/voz/arquivo sem escolher projeto, categoria ou prazo.
2. Persistir imediatamente; processamento e proposta chegam depois.
3. Piloto apresenta uma próxima ação e poucas exceções com razão.
4. Aceitar o dia em um clique, começar/concluir pela Home ou abrir contexto.
5. Encerrar com proposta pronta e movimento opcional, preservando prazos.
6. Depois de ausência, retomar intenções viáveis e mostrar Rescue sem cobrança.

O caminho desktop “aceitar dia” permanece um clique; encerrar abre a proposta e
confirma, com reflexão opcional. Isso é contagem estrutural de controles, não
medição de tempo humano. A meta de menos de 20 segundos ainda requer uso real.
Foram removidas etapas internas frágeis, não módulos inteiros por preferência
estética. Não houve mudança de CSS ou identidade visual nesta implementação.

## Prioridades restantes: Product Debt e Technical Debt

Frequência abaixo significa condição de ocorrência, não estatística de uso.
Esforço: P = pequeno, M = médio, G = grande. São pendências, não entregas feitas.

| P | Dívida / impacto | Frequência ou gatilho | Esforço / risco | Dependência e próximo critério de aceite |
| --- | --- | --- | --- | --- |
| P1 | Undo de TaskStart não restaura started_at/foco anterior completo | Desfazer início via Hermes | M / médio | Inversa atômica com guarda contra edição posterior; provar com duas Tasks |
| P1 | PWA sem encerramento/resgate completos e sem executor Hermes | Uso diário no bolso | M–G / médio | Consumir serviços portáveis; evitar duplicação em React/HTTP |
| P1 | Offline do telefone não é offline do desktop | PWA sem acesso ao servidor | G / alto | Protocolo/fila local e autenticação; definir conflitos antes de promessa de captura offline |
| P1 | Reunião e arquivos não acompanham Tasks ao outro aparelho | Abrir origem no bolso/outro PC | G / alto | Transporte de blobs, ciclo de vida/retention e estado de disponibilidade |
| P1 | Escritas aguardam portão durante rodada de sync | Hub lento/rede com timeout | M / alto | Medir latência; separar rede/aplicação sem reintroduzir inversão de locks (`sync_projecao.rs`) |
| P1 | Início simultâneo offline pode deixar duas Tasks com started_at entre réplicas | Dois dispositivos iniciam antes de sincronizar | M / alto | Invariante global requer política de convergência; atomicidade local não a resolve |
| P1 | Provedor acadêmico conectado em dois PCs pode duplicar importação | Segunda instalação importando | M / alto | Identidade externa compartilhada; refs atuais são locais |
| P2 | Search vaga perde sinonímia, erro de digitação e relações | Consulta sem termos literais | M / médio | Corpus de perguntas reais com resultado esperado; testar ranking híbrido antes de embeddings |
| P2 | Contexto de reunião/projeto ainda fragmentado | Perguntar decisão/pendência cruzada | M / médio | Consulta estruturada limitada por relação, data e evidência; orçamento de contexto |
| P2 | Triagem de Inbox ainda depende de decisão por item | Backlog de Captures | M / médio | Reusar propostas/jobs; aplicar lote reversível com chave idempotente e revisão de ambiguidades |
| P1 | Resolução de proposta Hermes não fornece idempotência persistente da execução inteira | Retry após resposta perdida | M / alto | Vincular operação à proposta e registrar execução/efeito atomicamente antes de ampliar automação |
| P1 | `save_objective` ainda emite snapshot completo de campos | Edições concorrentes em dispositivos | M / médio | Mutação por intenção/campos alterados; testar título em um aparelho e status em outro |
| P2 | Undo parcial/batches sem semântica uniforme | Mais autonomia de IA | G / alto | Inversas com versão, expiração e efeito conjunto; não sobrescrever edições posteriores |
| P2 | Métricas de adoção/atrito não consolidadas | Avaliar se o produto ajuda de fato | P–M / baixo | Contagens locais agregadas, sem conteúdo; comparar semanas, não criar score punitivo |
| P3 | Pessoa é texto livre e Calendar Event não é entidade | Consulta transversal pessoa/compromisso | G / médio | Casos reais demonstrando valor; não exigir cadastro na captura |
| P4 | Bundle desktop grande (aviso Vite >500 kB) | Startup/download | M / baixo | Medir cold start antes de alterar split/chunks |
| P4 | Documentação histórica e versões de pacotes divergentes | Planejar mudanças/onboarding | P / baixo | Consolidar estado atual mantendo ADRs históricas |

## Feature graveyard e redundâncias

| Candidato | Classificação | Fundamentação |
| --- | --- | --- |
| Novo Pulse separado do Piloto | **Fundir** | Já responde à mesma pergunta com o mesmo retrato; motor duplicado diverge |
| Proposta automática e assistente manual do dia | **Manter**, manual secundário | Mesma intenção, graus diferentes de controle; formulário não obrigatório |
| Widgets e arranjos da Home | **Manter/avaliar** | Valor de personalização não medido; não apagar layout do usuário. Piloto deve continuar visível |
| Bancadas de UI | **Manter** | Instrumento de inspeção isolada, não feature abandonada; usadas nesta validação |
| Spikes e specs históricas | **Manter como histórico** | Não confundir experimento/documento com feature distribuída |
| Hermes dentro do shell desktop | **Substituir a dependência**, preservar contratos | Extrair executor/context reader para adapter portátil quando conectar bolso |
| Grafo visual genérico novo | **Não criar por enquanto** | Relações e recuperação trazem valor antes de um canvas |
| Nova Command Palette concorrente | **Fundir** | Command/Functions e atalhos existentes já abrem ações |
| Telas de métricas novas | **Não criar por enquanto** | Reaproveitar contagens e diagnóstico local; provar decisão que a tela permitiria |
| Qualquer módulo alegadamente “não usado” | **Sem conclusão** | Não há dados de uso examinados que justifiquem remoção |

Nenhuma feature existente foi removida nesta entrega. Foram eliminadas
sequências duplicadas de fechamento, carry-over, planejamento e troca de foco
nos adapters, transferidas para operações atômicas compartilhadas.

## Observabilidade e métricas

Já existem saúde do sync, outbox, conflitos, falhas de notificação, estados de
jobs de reunião, eventos do Guardian, histórico Hermes e consumo por provedor.
Esta entrega melhora observabilidade na UI: erro de mutação deixa de ser
silencioso e falha de refresh não vira “zero itens”. Não foi ligado analytics
remoto nem inspecionado conteúdo real para inferir produtividade.

Métricas locais propostas: tempo captura→persistência (p50/p95), backlog por
idade, propostas aceitas/corrigidas/desfeitas, reunião parada→pronta, falha/retry,
atenções resolvidas/dispensadas e uso dos atalhos. Separar capacidade de
processamento de adoção. Começar por agregados das tabelas existentes; adicionar
evento somente quando a pergunta não puder ser respondida por elas.

## Validação e limites

- `cargo test --workspace -- --test-threads=2`: 1.494 aprovados, zero falhas, 10 ignorados; 50 suites incluindo doc-tests. Features padrão do workspace.
- Desktop: 34 arquivos / 422 testes TypeScript aprovados; build TypeScript + Vite aprovado.
- PWA: 11 arquivos / 80 testes TypeScript aprovados; build TypeScript + Vite aprovado.
- Regressões SQLite: 25 testes de Daily Session e 7 de planejamento/foco aprovados;
  suite completa de storage também executada no workspace.
- Bancada desktop: feedback visível após falha, retry habilitado, sem overflow
  horizontal no viewport inspecionado. Bancada PWA 390×844: falha 503, retry,
  retenção do panorama e navegação da atenção para Fazer. Dados sintéticos.
- `rustfmt` nos arquivos Rust modificados e `git diff --check`.

A primeira tentativa de suite com paralelismo padrão foi interrompida por
lentidão; isso não é evidência de deadlock. A execução final usa duas threads.
O controle Orca do app nativo estava indisponível, portanto não se declara
validação do executável instalado, microfone, tray ou atalho global. Não houve
teste ao vivo de gateway/modelo, transcritor, hub externo, push físico, Finance,
Univirtus ou autenticação passkey com feature OpenSSL. Não foi feito benchmark
de startup/memória nem teste humano de adoção ao longo de dias.

Os bancos de regressão são temporários. Nenhum banco real foi migrado, apagado
ou usado como massa de testes. O processo M/OS já aberto não foi substituído.
A edição de `apps/desktop/src/App.css` que precedia esta tarefa foi preservada.

## Critério de sucesso: resposta honesta neste corte

| Pergunta | Resultado |
| --- | --- |
| Capturar em segundos? | Fluxos existem sem classificação obrigatória; tempo/dispositivo real não medido |
| Organizar sozinho a maior parte? | Ainda não demonstrado: ingestão/reuniões/propostas ajudam, auto-triagem geral é parcial |
| Reencontrar qualquer informação? | Não: busca literal melhorada no Hermes, sem recall semântico universal |
| Mostrar ação relevante? | Piloto existente, agora respeita adiamento e continuidade; relevância depende de dados reais |
| Hermes entender/controlar tudo? | Não: cobertura explícita no catálogo; contexto Task e descoberta de reunião melhores |
| Reuniões virarem memória/ações? | Pipeline implementado; validação local não substitui áudio/serviços ao vivo |
| Sync invisível? | Convergência testada nos tipos cobertos; arquivo local e PWA offline seguem limitações |
| Retomar após ausência? | Rescue já existe; carry-over e teto corrigidos. Adoção ainda precisa ser observada |

A entrega melhora a confiabilidade do loop que já existe. As lacunas acima
continuam abertas e não devem ser apresentadas como automação concluída.
