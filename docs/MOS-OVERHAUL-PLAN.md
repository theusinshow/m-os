# M/OS — plano completo de continuidade do overhaul

**Data:** 24/09/2026  
**Status:** em execução — ver §22 para o que já foi entregue.  
**Base:** [auditoria realizada](MOS-AUDIT.md), código do workspace v0.6.0 e missão Product & AI Overhaul.  
**Resultado esperado:** conseguir capturar, reencontrar e executar com pouco trabalho de organização, inclusive depois de alguns dias sem usar o sistema.

Este documento descreve o que planejo fazer, em que ordem, por qual motivo e como vou verificar cada entrega. Não é uma declaração de que todas essas capacidades já existem. Também não substitui o inventário e as evidências da auditoria.

## 1. Direção do trabalho

O M/OS deve ajudar antes de pedir organização. A sequência pretendida é:

**Capturar → guardar → compreender → relacionar → apresentar uma decisão simples → executar → permitir correção.**

Vou melhorar o sistema existente em ciclos pequenos. A prioridade será remover falhas de confiança e trabalho manual dos fluxos principais. Novas entidades, telas e tecnologias só entram quando resolverem uma limitação demonstrada.

Princípios que guiarão as decisões:

- Captura não depende de classificação, projeto, IA ou disponibilidade do gateway.
- O usuário não precisa iniciar o dia para ver uma próxima ação útil.
- Prazo e dia planejado continuam sendo conceitos diferentes.
- Automação precisa ter proveniência, estado legível e correção possível.
- A mesma regra de negócio atende desktop e celular.
- Falha de leitura não pode parecer ausência de dados; falha de escrita precisa aparecer.
- Notificação só interrompe quando existe motivo e ação útil.
- A interface mantém a identidade atual; mudanças visuais precisam reduzir atrito.
- Recursos existentes não serão removidos por suposição de desuso.

As referências principais são [VISION](VISION.md), [PRODUCT](PRODUCT.md), [UX-PRINCIPLES](UX-PRINCIPLES.md), [PLATFORMS](PLATFORMS.md), [FEATURE-DEVELOPMENT](FEATURE-DEVELOPMENT.md), [SYNC](SYNC.md) e [PILOTO](PILOTO.md). A ordem respeita o [ROADMAP](ROADMAP.md): confiança e recuperação precedem automação ampla.

## 2. Ponto de partida: o que já foi feito

Estas entregas pertencem ao ciclo anterior e não serão contadas como trabalho futuro:

| Área | Mudança já implementada |
| --- | --- |
| Arquitetura | Inventário do produto, limites por plataforma e decisão de evoluir o Piloto como Pulse, sem outro motor de prioridades |
| Planejamento | Carry-over de intenções de dia encerrado, teto de quatro objetivos e respeito a Tasks adiadas |
| Integridade | Início planejado e encerramento com reagendamento em transações que incluem a outbox |
| Foco | Validação do próximo alvo antes de parar a Task anterior; troca atômica na réplica local |
| Sync | Correção de emissões fantasma ao reordenar checklist ou objetivos com IDs de outra entidade |
| Hermes | Filtro de tipo antes do limite de candidatos; descoberta pelo índice de transcrições; contexto de execução de Task mais completo |
| Interface | Erros visíveis na Home, retry, texto do dia mais direto e retenção da última leitura na PWA quando HTTP falha |
| Celular | Itens de atenção conectados a lembretes, acadêmico, Inbox e sync nos destinos disponíveis |
| Documentação | `MOS-AUDIT.md`, ADR-073 e atualização do estado real em documentos principais |

Validação daquele ciclo: **1.494 testes Rust e 502 testes TypeScript aprovados**, dez testes dependentes de recursos reais ignorados, builds das interfaces aprovados e inspeção visual em bancada. Esses números são uma referência histórica, não resultado de testes novos deste plano.

As alterações estão no código. O aplicativo instalado não foi atualizado. Não houve validação ao vivo de microfone, push no iPhone, gateway Hermes, transcritor ou integrações externas.

## 3. Escopo e compromissos

Divido a continuidade em três níveis para não transformar possibilidades em promessas de implementação.

| Nível | O que significa | Itens |
| --- | --- | --- |
| Execução prioritária | Problemas observados que planejo corrigir primeiro | Undo, idempotência, edição por campo, continuidade do dia no bolso, contexto estruturado, observabilidade e testes críticos |
| Entrega progressiva | Capacidade desejada cujo desenho será fechado com um experimento técnico | Captura offline na PWA, sync de conteúdo de reuniões/arquivos, triagem automática, isolamento da rede no sync |
| Condicional | Só entra após demonstrar ganho sobre o que já existe | Embeddings, Person de primeira classe, Calendar Event genérico, cliente iOS nativo, novos conectores |

Não há cronograma em dias fechado. Esforços abaixo são relativos: **P** é uma mudança localizada; **M** atravessa algumas camadas; **G** exige protocolo, persistência ou várias superfícies. Não são estimativas de prazo contratual.

## 4. Sequência de execução

| Ordem | Frente | Prioridade | Esforço | Risco principal | Dependência |
| --- | --- | --- | --- | --- | --- |
| F0 | Checkpoint e reprodução das lacunas | P1 | P | Partir de estado diferente do auditado | Nenhuma |
| F1 | Integridade, idempotência e Undo | P1; P0 se a reprodução demonstrar perda/duplicação crítica | M–G | Alterar estado antigo ou repetir efeito | F0 |
| F2 | Captura, dia e retomada no celular | P1 | M | UI divergir do domínio | F1 para escritas novas |
| F3 | Sync, foco entre réplicas e captura offline | P1 | G | Conflito, migração ou retry incorreto | F1; F2 para a experiência mobile |
| F4 | Serviços portáveis do Hermes e contexto | P2, com paridade mobile P1 | G | Aumentar autoridade sem controle | F1 |
| F5 | Search contextual e relações | P2 | M–G | Melhorar aparência sem melhorar recall | F4 para ferramentas Hermes; corpus pode começar em F0 |
| F6 | Inbox inteligente e captura sem administração | P2 | M–G | Classificação errada ou criação duplicada | F1, F4 e F5 |
| F7 | Reuniões confiáveis e memória acessível | P1/P2 | G | Perder origem ou duplicar extração | F1, F4; F3 para distribuição de conteúdo |
| F8 | Pulse proativo e notificações | P2 | M | Excesso de avisos ou sugestões repetidas | F2, F6 e F7 |
| F9 | Métricas, UX, desempenho e limpeza | P2/P4 | M | Medir vaidade ou remover valor real | Instrumentação desde F0; avaliação após uso |
| F10 | Integrações, pacote e validação real | P1 para distribuir; P3 para expansões | M–G | Confundir build com produto validado | Marcos anteriores aprovados |

A ordem é de dependências, não uma exigência de esperar todas as frentes para entregar valor. Por exemplo, o encerramento do dia na PWA pode ser entregue antes de Search semântica ou transporte de arquivos. Falha crítica comprovada interrompe a sequência e passa à frente.

## 5. F0 — consolidar o checkpoint

### O que planejo fazer

1. Conferir o diff atual, preservar alterações anteriores do usuário e registrar quais arquivos pertencem ao overhaul.
2. Confirmar que cada correção da auditoria possui reprodução ou regressão coerente com o problema.
3. Reproduzir as pendências de Undo, resolução repetida de proposta, edição concorrente de objetivos e foco em duas réplicas.
4. Montar uma massa sintética integrada: dois projetos, Captures, Tasks, esperas, lembretes, reuniões e compromissos acadêmicos.
5. Registrar os fluxos atuais e suas decisões obrigatórias: capturar, planejar, encerrar, revisar reunião e reencontrar informação.
6. Criar uma lista de perguntas de busca e contexto com respostas esperadas, antes de alterar o ranking.

### Entrega e aceite

Um checkpoint revisável, com baseline, reproduções e backlog atualizado. Hipóteses que não se confirmarem serão reclassificadas; não serão tratadas como bugs comprovados. Os bancos de teste continuarão temporários.

## 6. F1 — confiança para permitir automação

### F1.1 — desfazer uma ação de verdade

**Problema:** o Undo de iniciar Task restaura apenas parte do estado. Uma inversa incompleta não é suficiente para deixar a IA agir com autonomia.

Planejo:

- Representar o efeito completo do início: Task acionada, estado/início anteriores e foco interrompido.
- Gravar os dados necessários à inversa junto do efeito local, quando compartilharem a mesma transação.
- Executar a reversão como um gesto atômico.
- Verificar se houve mudança posterior antes de restaurar valores antigos.
- Quando houver conflito, explicar o que não pode ser desfeito automaticamente, preservando o trabalho mais recente.
- Revisar as demais variantes de `UndoStep`, especialmente lotes, conversão de Capture e insights de reunião.
- Tornar explícito o que é reversível, compensável ou sem inversa.

**Aceite:** iniciar B enquanto A está ativa e desfazer restaura o estado esperado; editar B depois impede uma reversão destrutiva; repetir Undo não duplica nem corrompe nada; o efeito converge nas réplicas cobertas.

### F1.2 — não executar a mesma proposta duas vezes

**Problema:** executar o efeito e depois gravar o resultado da proposta deixa uma janela de falha. Uma resposta perdida pode provocar repetição.

Planejo:

- Identificar cada proposta/operação por chave estável, independente do texto mostrado.
- Vincular a execução ao registro persistido da proposta, verificando estado, conteúdo e permissões.
- Registrar tentativa, resultado e entidades afetadas; recuperar o resultado de uma operação já aplicada.
- Garantir atomicidade de efeito e registro para operações locais no mesmo banco.
- Para efeitos externos, usar chave idempotente quando o serviço suportar; caso contrário, registrar estado incerto e reconciliar antes de repetir.
- Não declarar garantia universal de “exatamente uma vez” sobre serviços que não oferecem esse contrato.

**Aceite:** clique duplo, timeout após commit, reinício e reenvio criam um único efeito local. Falha externa ambígua não vira sucesso nem nova tentativa cega.

### F1.3 — sincronizar a intenção da edição

**Problema:** `save_objective` ainda emite todos os campos, mesmo quando a intenção era editar apenas um deles.

Planejo:

- Separar as mutações por intenção: título/descrição, status, vínculo, prioridade e ordem.
- Ler e validar o estado necessário dentro da transação.
- Emitir apenas campos efetivamente alterados.
- Auditar outros repositórios que ainda gravem snapshots completos em gestos parciais.
- Manter exclusão, desvinculação, arquivamento e conclusão como operações distintas.

**Aceite:** editar o título no desktop e concluir o objetivo no outro dispositivo preserva ambas as decisões depois do sync; uma mutação recusada não deixa operação na outbox.

**Áreas previstas:** `mos-core` (`action`, `ports`, `service`), `mos-storage-sqlite`, adapter Hermes em `jarvis.rs` e suites de duas réplicas. Migrations serão aditivas se um registro persistente novo for necessário; o número será escolhido no momento da implementação.

## 7. F2 — ciclo diário completo com poucos gestos

### F2.1 — Start/End My Day no bolso

Planejo expor o encerramento assistido na API e PWA usando `DailyService` e o planejador existentes. A tela mostrará o que terminou, o que ficou aberto e a proposta de levar para amanhã. Reflexão e humor continuarão opcionais.

Também vou revisar:

- Dia ainda não iniciado, dia ativo, dia encerrado e sessão antiga aberta.
- Troca de data e fuso entre desktop e telefone.
- Proposta que ficou desatualizada enquanto a tela estava aberta.
- Task concluída em outro dispositivo antes da confirmação.
- Voltar à tela depois de timeout sem repetir efeitos.
- Possibilidade de ver e executar Agora sem passar por planejamento diário.

**Aceite:** começar pela proposta exige um gesto; encerrar não exige texto; a meta de interação é menos de 20 segundos em um cenário preparado, medida em uso real. Encerrar não altera prazo. Desktop e PWA chegam ao mesmo resultado de domínio.

### F2.2 — retomada depois de ausência

Planejo completar a manifestação do Rescue Mode no celular e revisar a desktop:

- Mostrar poucas pendências úteis e o motivo da seleção.
- Oferecer fazer hoje, fazer depois, concluir, revisar origem ou dispensar quando fizer sentido.
- Explicar aplicação parcial de um lote e permitir correção.
- Evitar cobrar início/fim do dia ou “organização atrasada”.
- Preservar os itens não selecionados, sem arquivar por simples envelhecimento.

**Aceite:** uma ausência simulada de vários dias não obriga a reorganizar toda a Inbox para voltar a trabalhar; erros do lote são identificáveis; itens dispensados não reaparecem imediatamente sem fato novo.

### F2.3 — qualidade dos fluxos principais

Vou percorrer Home, Capture, Task, Reminder e detalhes do dia com teclado e toque. Corrigirei loading, foco, submissão repetida, empty states, mensagens e caminhos sem saída que forem reproduzidos. Layout só muda quando isso melhora a execução.

## 8. F3 — sync e captura confiável em qualquer dispositivo

### F3.1 — rede lenta não pode impedir captura local

O portão de escrita atual protege a ordem dos locks, mas é mantido durante a rodada de sync. Primeiro vou medir captura e edição com transporte artificialmente lento, offline e com timeout.

Se o bloqueio se confirmar como relevante, planejo separar preparação do lote, transporte e aplicação, mantendo as garantias do relógio lógico, da outbox e da projeção. Não vou simplesmente remover o lock: isso poderia reintroduzir deadlock ou aplicação fora de ordem.

**Aceite:** transporte suspenso não bloqueia indefinidamente captura local; escritas ocorridas durante a rodada não somem; ack atrasado não confirma uma operação diferente; testes de falha e convergência passam.

### F3.2 — decidir o foco entre réplicas

Planejo definir e documentar o que significa “Task ativa” quando dois aparelhos começam trabalhos diferentes offline. A decisão deve distinguir foco pessoal e cronômetro local, preservando histórico.

Depois implementarei uma regra determinística compatível com o protocolo: mesma coleção de operações deve gerar o mesmo resultado, independente da ordem de chegada. Atomicidade local não será apresentada como exclusividade global.

**Aceite:** iniciar A e B offline, reconectar em ordens diferentes e repetir sync converge para a política documentada sem apagar histórico ou deixar um timer fictício.

### F3.3 — captura offline na PWA

A PWA atual depende do SQLite do servidor. Planejo um primeiro escopo limitado: **guardar Captures pendentes no aparelho**, sem tentar replicar todo o domínio no navegador.

- Persistir a captura e a chave da operação antes de confirmar que foi guardada localmente.
- Reenviar com idempotência quando houver conexão e sessão válida.
- Diferenciar “guardado neste aparelho”, “aguardando envio” e “recebido pelo M/OS”.
- Retomar ao reabrir o app, sem depender de execução em background garantida pelo navegador.
- Não perder texto se a sessão expirar; tratar corretamente troca de conta.
- Inspecionar quota, indisponibilidade de armazenamento e limpeza pelo navegador; não prometer a durabilidade de um SQLite nativo.
- Expandir para outros tipos de escrita somente depois de validar esse contrato.

**Aceite:** modo avião, captura, fechamento/reabertura e reconexão produzem um único item; erro de armazenamento não aparece como sucesso; sessão expirada preserva a entrada pendente sem enviá-la à conta errada.

### F3.4 — arquivos e reuniões entre dispositivos

Vou separar duas entregas: contexto estruturado sincronizável e transporte do conteúdo original. O primeiro pode trazer resumo, decisões e disponibilidade; o segundo exige contrato de blobs, integridade, retry e retenção.

Planejo definir identidade por conteúdo, estado de upload/download, referência ao dono do arquivo e recuperação de transferências parciais. Retenção automática não poderá remover a única cópia válida. Acesso a conteúdo ausente deve informar onde ele está, sem apresentar link quebrado como se estivesse disponível.

**Aceite:** abrir a origem de uma Task no outro aparelho mostra contexto ou estado de disponibilidade correto; retry de transferência não duplica arquivo; cancelamento e exclusão têm efeito definido. O transporte definitivo depende da avaliação da infraestrutura existente e de custos, não está escolhido neste plano.

## 9. F4 — Hermes como camada operacional compartilhada

### F4.1 — retirar regras do shell desktop

Planejo separar três responsabilidades hoje próximas em `jarvis.rs`:

1. Leitura de contexto por contratos portáveis.
2. Planejamento/validação de ações no domínio.
3. Execução por serviços, com adapters para capacidades locais e remotas.

O contrato de ação existente será preservado onde possível. Tauri e HTTP deverão chamar o mesmo serviço. Operações dependentes de microfone, processo ou arquivo local continuarão em adapters de capacidade; não serão simuladas como disponíveis no servidor.

**Aceite:** a mesma ação sobre Task, Reminder ou dia produz validação e efeito equivalentes em desktop e API; ação sem capacidade disponível informa a limitação.

### F4.2 — Context Engine com evidências

Planejo montar contexto por intenção, usando uma combinação de:

- Entidade aberta e anexos escolhidos.
- Relações persistidas com Project, Task, Resource e origem.
- Esperas, dependências, prazos e agenda relevante.
- Decisões e trechos de reunião relacionados à consulta.
- Recência como critério complementar, sem esconder registros antigos relevantes.

Cada bloco terá origem identificável e limite de tamanho. Respostas sobre decisões devem permitir abrir a fonte. Quando o conteúdo estiver indisponível ou incompleto, o Hermes deve dizer isso.

Vou medir volume de contexto, tempo de recuperação, primeira resposta e chamadas por intenção. O orçamento inicial será definido com o baseline; não vou fixar modelo, preço ou limite arbitrário sem verificar o gateway configurado.

**Aceite:** “o que falta no projeto X?” reúne Tasks abertas, esperas e decisões disponíveis; “o que Victor ainda deve?” mostra registros correspondentes e ambiguidade de nomes quando existir; nenhum prompt recebe a base inteira como fallback.

### F4.3 — ampliar ferramentas por fluxo

| Intenção | Entrega planejada |
| --- | --- |
| “Marca isso como concluído” | Resolver referência contextual, verificar ambiguidade e executar com Undo consistente |
| “Me lembra amanhã de revisar o memorial” | Interpretar data/fuso, preservar alvo e criar sem duplicação por retry |
| “Essa reunião gerou alguma tarefa?” | Consultar insights, Tasks efetivadas e pendências de revisão com proveniência |
| “O que ficou pendente do projeto X?” | Consulta estruturada de trabalho, bloqueios e esperas |
| “Organiza essas capturas” | Prévia compacta ou aplicação permitida por política, com recibo de lote e inversa |
| “Me mostra tudo relacionado a esse projeto” | Consulta limitada e navegável de relações, sem despejar conteúdo sem relevância |

Não vou expor exclusão permanente ou qualquer ação externa só porque ela cabe no parser. A cobertura será explícita em catálogo, testes e UI. Conversa e execução Hermes na PWA entram depois desse serviço compartilhado, com autenticação e falhas de conexão verificadas.

## 10. F5 — Search superior e contexto relacionado

### Método de implementação

1. Criar corpus inicial com pelo menos 40 consultas distribuídas entre termos exatos, frases vagas, nomes, erros de digitação, datas e relações.
2. Registrar resultados esperados e casos em que “não encontrado” é a resposta correta.
3. Medir a busca atual: presença do resultado esperado no top 5, posição do primeiro resultado útil e latência local.
4. Melhorar recuperação lexical: normalização, expansão controlada de termos, combinação de acertos e ranking por entidade/relação.
5. Mostrar o motivo do resultado e o trecho correspondente quando houver evidência.
6. Usar o mesmo serviço de recuperação em Search e nas consultas Hermes, com apresentações e orçamentos diferentes.
7. Avaliar embeddings somente se persistirem falhas semânticas relevantes que os passos anteriores não resolvam.

Consultas que precisam estar no corpus:

- “aquele negócio das bases que o Victor pediu”;
- “reunião da caixa de drenagem”;
- “arquivo que guardei sobre Allplan”;
- “decisão da reunião de Jaboticatuba”;
- “o que aconteceu semana passada no projeto X?”.

**Aceite:** melhorar o top 5 no corpus sem regressão nas consultas exatas, preservar filtros de arquivamento/exclusão, abrir a entidade correta e mostrar evidência. A meta numérica final será definida após medir o baseline; não será escolhido um percentual depois do resultado para declarar sucesso.

### Relações e Timeline

Vou projetar o contexto relacionado a partir das tabelas existentes: origem da Capture, Project, referências, reunião/insight, Task derivada e histórico disponível. Se um fato não estiver registrado, a Timeline não deve inventá-lo.

O objetivo é abrir uma entidade e encontrar sua história útil. Um grafo visual, uma base genérica de triplas ou event sourcing de todo o sistema não são pré-requisitos.

## 11. F6 — Smart Inbox e Zero-Organization

### Fluxo pretendido

1. Usuário envia texto, URL, arquivo, screenshot ou voz por uma entrada já disponível.
2. O original é preservado imediatamente.
3. Processamento determina o que pode ser extraído com segurança.
4. O sistema relaciona candidatos a entidades existentes.
5. A política decide entre organizar metadata, sugerir uma ação ou pedir a única informação que falta.
6. O resultado aparece junto do item, com origem e possibilidade de corrigir.

### Trabalho planejado

- Reusar ingestão, interpretação de voz e propostas existentes em vez de criar outra Inbox.
- Extrair candidatos a Task, Reminder, Project, prazo e Waiting For sem exigir cadastro prévio.
- Detectar itens já convertidos e propostas equivalentes.
- Oferecer “Fazer hoje”, “Depois”, “Aguardando”, “Salvar referência” e “Ignorar sugestão” conforme o contexto.
- Permitir revisar e aplicar lotes pequenos com resultado por item.
- Preservar a Capture quando o processamento falhar ou a sugestão for rejeitada.
- Registrar correções explícitas para melhorar defaults futuros, sem inferir preferência definitiva de uma única dispensa.
- Investigar OCR e compartilhamento apenas nas capacidades disponíveis; declarar formatos não suportados.

### Política de confiança

| Situação | Comportamento inicial |
| --- | --- |
| Projeção derivada ou metadata determinística sem efeito destrutivo | Executar silenciosamente, mantendo origem e possibilidade de recalcular |
| Associação reversível com evidência suficiente e política já validada | Permitir aplicação automática com recibo acessível e Undo |
| Criação de compromisso, prazo ambíguo, pessoa/projeto incertos | Mostrar proposta pronta ou uma pergunta curta |
| Exclusão permanente ou efeito externo sensível | Aplicar proteção específica da ação |

“Alta confiança” não será apenas um número declarado pelo modelo. Vou considerar evidência, unicidade da entidade, tipo de ação, reversibilidade, correções anteriores e qualidade medida no corpus. A automação começa restrita e só se expande depois de validada.

**Aceite:** captura continua funcionando sem IA; indisponibilidade do worker deixa estado recuperável; repetir análise não cria outra Task; corrigir associação não perde o original; Undo de lote identifica exatamente os efeitos aplicados.

## 12. F7 — reuniões que viram memória e ações

O pipeline V2 já existe. Planejo verificar e fechar os pontos de falha, em vez de substituí-lo.

### Matriz de cenários

| Cenário | Comportamento que vou exigir |
| --- | --- |
| Gravação normal | Parar inicia o processamento configurado e mostra progresso legível |
| App encerra ou falha durante gravação | Áudio recuperável e sessão identificada para retomada/revisão |
| Transcritor ausente | Estado acionável, preservando áudio e opção de tentar novamente |
| Análise falha ou gateway cai | Job recuperável, causa visível e retry sem duplicar extração |
| Reprocessamento | Nova versão não destrói original nem duplica Tasks já efetivadas |
| Reunião esquecida | Guardian respeita sinais, cooldown, ação de continuar e configuração de auto-stop |
| Trecho sugerido para corte | Recuperação do original continua possível conforme retenção |
| Exclusão | Trash/retention e impacto em derivados ficam claros |
| Outro dispositivo | Contexto acessível ou disponibilidade explicitamente informada |

Vou revisar extração de resumo, decisão, compromisso, Task, espera, pessoa citada e data. Cada ação proposta precisa apontar para evidência; uma menção não pode virar compromisso automaticamente.

**Aceite:** fixture de reunião conhecida gera resultados verificáveis; duas execuções não duplicam ações; fechar/reabrir preserva progresso; Undo não deixa vínculos órfãos. O teste final inclui áudio e gateway reais quando disponíveis, com resultados separados dos testes simulados.

## 13. F8 — Pulse proativo, com poucas interrupções

Pulse continuará sendo a evolução do Piloto compartilhado. Planejo conectá-lo aos eventos e estados já existentes:

- Capture recebida e processamento concluído.
- Reunião pronta, falha de job ou ações pendentes.
- Prazo próximo, Waiting For vencido e bloqueio persistente.
- Ausência, backlog acumulado e retomada.
- Sync com erro acionável ou conteúdo indisponível.

### O que muda para o usuário

A Home deve apresentar uma próxima ação, poucas exceções e a explicação do motivo. Quando o contexto mudar, a sugestão pode mudar; durante um gesto em andamento, não deve deslocar o alvo do toque.

Vou definir identidade estável do item de atenção, estado de dispensa/adiamento, validade e condição para reaparecer. A decisão sobre o que é compartilhado entre aparelhos será explícita: uma decisão do usuário pode precisar viajar; registro de entrega de notificação continua pertencendo ao canal/aparelho.

### Níveis de intervenção

| Nível | Uso |
| --- | --- |
| Ambient / silencioso | Atualização de contexto e informação disponível dentro da Home |
| Suggest / acionável | Poucas propostas com uma ação direta e explicação |
| Notify / interrupção | Situação importante, temporalmente relevante e com destino útil |

Planejo agrupamento, dedupe, cooldown e horários de silêncio nos canais existentes. Não haverá notificação apenas para pedir que o usuário volte a organizar o sistema.

**Aceite:** o mesmo fato não gera uma sequência de avisos equivalentes; dispensa é respeitada; novo fato relevante pode reativar atenção com motivo claro; abrir a notificação leva ao item certo; o sistema funciona sem consultar um LLM continuamente.

## 14. F9 — métricas, UX e desempenho

### Métricas locais que pretendo usar

| Pergunta | Medida |
| --- | --- |
| Capturar está rápido? | Tempo entre envio e persistência confirmada; p50/p95 por entrada |
| A organização automática ajuda? | Propostas aceitas, corrigidas, recusadas e desfeitas, por tipo |
| A Inbox está parando? | Quantidade e idade dos itens pendentes |
| Search encontra o esperado? | Top 5 no corpus, consultas sem resultado e abertura de fonte |
| Reuniões andam sozinhas? | Tempo parar→pronta, falhas, retries e intervenção manual |
| Pulse é útil? | Ações executadas, dispensas e repetição de atenção sem resolução |
| Sync interfere no trabalho? | Latência de escrita com rede lenta e idade da outbox |
| O dia virou administração? | Decisões/formulários exigidos e tempo de completar o fluxo |

Vou priorizar agregados de dados já existentes. Novos eventos serão locais, sem conteúdo de Captures/transcrições, com retenção definida. Não planejo analytics remoto nem placar de produtividade.

### Revisão da experiência

Vou testar estados vazio, carregando, ocupado, sucesso, falha, offline, desatualizado e parcialmente aplicado. A revisão inclui foco, teclado, leitor de tela quando disponível, targets de toque e redução de movimento.

Para desempenho, medirei cold start, abertura da Home, busca, consumo de memória e bloqueios perceptíveis. O aviso de bundle grande é um indício a investigar; sozinho não justifica refatorar o frontend.

### Limpeza e Feature Graveyard

Vou procurar caminhos duplicados, componentes sem referência, experimentos distribuídos por engano e telas que não levam a uma ação útil. Cada candidato receberá decisão: manter, terminar, fundir, substituir ou remover, com evidência.

Não vou apagar widgets/layouts, dados ou módulos por supor que você não os usa. Uma remoção precisa preservar dados e oferecer um caminho equivalente quando a capacidade ainda tiver valor.

## 15. F10 — integrações e validação do produto real

### Integrações existentes

- **M/Academic:** identidade externa para impedir importação duplicada em duas instalações, reconciliação de atualizações, vínculos com Task e calendário.
- **Tempo/CronoCAD:** distinguir timer local de horas sincronizáveis, conferir proveniência e importação idempotente.
- **M/Finance:** verificar contrato da ponte e tratamento de resultado incerto; não executar movimentações reais como teste de rotina.
- **GitHub/App Registry:** documentar que catálogo e abertura de app não são integração de issues/PRs.
- **Gateway Hermes e transcritor:** verificar autenticação, reconexão, erro, cancelamento, timeout e recuperação.
- **Web Push e passkeys:** validar o caminho com as features necessárias e depois em dispositivo real; build padrão não prova entrega de notificação ou autenticação completa.

Novos conectores, como Google/Outlook Calendar, dependem de um fluxo observado que justifique o acesso e de credenciais apropriadas. Não fazem parte automática da correção do calendário interno.

### Distribuição

Planejo preparar build identificável, notas de versão, compatibilidade entre versões e recuperação antes de atualizar o ambiente em uso. Se houver migration, verificarei upgrade sobre cópia de banco e snapshot prévio. Voltar para binário antigo só será opção se ele entender o schema; do contrário, a recuperação deve usar backup compatível.

Nenhum teste destrutivo será feito no único banco real. A publicação/instalação será distinguida de compilação e de validação em bancada. Credenciais, custos ou ações externas irreversíveis serão tratados quando houver uma ação concreta, sem bloquear investigação e implementação local.

## 16. Oito eixos por frente

As tabelas abaixo registram os oito eixos exigidos em `FEATURE-DEVELOPMENT.md`. Elas serão refinadas em cada entrega; nenhum eixo poderá ficar implícito.

### Core, Database, Sync e Hermes

| Frente | Core | Database | Sync | Hermes |
| --- | --- | --- | --- | --- |
| F0 | Reproduzir regras existentes | Fixtures temporárias | Baseline com duas réplicas | Corpus e catálogo atual |
| F1 | Intenção, efeito, inversa e validação | Transação/registro de execução; migration aditiva se necessária | Campos alterados, retry e conflitos de Undo | Execução idempotente e inversas completas |
| F2 | Reusar planejador/DailyService | Persistência existente; só acrescentar se houver necessidade comprovada | Sessões/objetivos/decisões compartilhadas | Mesmas operações de dia |
| F3 | Contrato de foco e operação | Fila de captura no browser separada do banco de domínio; estado de transferência | Rede fora da seção crítica quando seguro; convergência e blobs definidos | Só afirmar disponibilidade confirmada |
| F4 | Ports e serviços de contexto/ação | Auditoria e proveniência limitadas | Política explícita para conversas/ações; credenciais locais | Serviço compartilhado entre adapters |
| F5 | Consulta e ranking | Índices derivados e reconstruíveis | Fontes viajam; índice local recomposto | Mesma recuperação com orçamento próprio |
| F6 | Política de triagem | Jobs/propostas/efeitos e original preservado | Entidades produzidas e decisões; não sincronizar lease de worker sem desenho | Classificação e ação governadas por risco |
| F7 | Pipeline/insights/Guardian portáveis onde aplicável | Jobs, evidência, revisão e retenção | Contexto estruturado e conteúdo conforme F3 | Recuperar decisões e aplicar propostas |
| F8 | Piloto único, identidade e validade da atenção | Estado mínimo de decisão/dedupe | Separar decisão compartilhada de entrega local | Proatividade por evento e política |
| F9 | Regras de contagem; UI fora do domínio | Agregados/eventos mínimos | Métricas locais por padrão | Qualidade/latência, sem registrar conteúdo sensível |
| F10 | Contratos dos provedores | Upgrade e identidade externa | Compatibilidade entre versões e importação | Falhas de integração visíveis |

### Desktop, iOS, Notifications e Tests

“iOS” abaixo significa a manifestação PWA existente. Não implica cliente nativo entregue.

| Frente | Desktop | iOS/PWA | Notifications | Tests |
| --- | --- | --- | --- | --- |
| F0 | Baseline dos fluxos | Baseline de toque e conexão | Inventariar canais | Reproduções e corpus |
| F1 | Recibos e Undo claros | Mesmo contrato HTTP onde exposto | Nenhum canal novo; falha junto da ação | Rollback, retry, reinício, concorrência e inversa |
| F2 | Reduzir decisões obrigatórias | Encerramento e Rescue adequados ao toque | Sem cobrança por ritual não feito | Data/fuso, stale, proposta antiga e UI |
| F3 | Captura não espera rede | Captura pendente e disponibilidade de conteúdo | Alertar só problema acionável | Offline, sessão, duas réplicas, transferência parcial |
| F4 | Adapter Tauri fino | Adapter autenticado e conversa quando serviço estiver pronto | Resultado no contexto; push só se justificado | Contrato comum, capacidade ausente, cancelamento |
| F5 | Busca por teclado e contexto | Busca compacta por toque | Não se aplica: busca é solicitada pelo usuário | Corpus, ranking, filtros e fonte correta |
| F6 | Entradas existentes e revisão em lote | Captura e correção rápidas; share conforme capacidade | Ambient por padrão, sem aviso por cada Capture | Falha de worker, duplicata, correção e Undo |
| F7 | Áudio/Guardian e revisão | Ler/revisar contexto; captura nativa não prometida | Pronta/falha relevante com dedupe | Fixtures e áudio real em etapa separada |
| F8 | Home e canal local | Home e Web Push | Agrupamento, quiet hours, cooldown e destino | Mesmo fato, dispensa, novo fato, entrega real |
| F9 | Teclado/densidade/performance | Toque/responsividade/performance | Medir ruído sem novos avisos | Inspeção visual e benchmarks dos fluxos |
| F10 | Pacote e integração local | PWA instalada, login e rede reais | Push físico e limitações registradas | Upgrade, backup/restore e testes de integração |

## 17. Opções que dependem de evidência

| Opção | Quando faz sentido avançar | Alternativa inicial |
| --- | --- | --- |
| Embeddings / busca vetorial | Corpus mostra falhas semânticas importantes após melhorias lexicais/relacionais | FTS, expansão controlada e relações |
| Person de primeira classe | Nomes ambíguos e consultas transversais não são resolvidos com o modelo leve | Waiting For, nome citado e proveniência |
| Calendar Event genérico | Há compromissos relevantes que não cabem nos tipos atuais | Projeção dos módulos existentes |
| Calendário externo | Um fluxo real exige consultar/criar compromissos fora do M/OS | Agenda interna e captura de contexto |
| iOS nativo | Limitações demonstradas da PWA impedem captura, armazenamento ou capacidades essenciais | PWA com fila de captura e interfaces adequadas |
| Grafo visual | Usuário precisa explorar relações espacialmente para resolver uma tarefa | Contexto relacionado e consulta por Project |
| Captura passiva ampla | Existe fonte útil com escopo, consentimento e controle claros | Captura explícita por atalhos, voz e drop |
| Nova página de métricas | Um agregado local permite uma decisão que a UI atual não oferece | Diagnóstico/export compacto |

Essas opções são investigações condicionais. Não vou instalar uma infraestrutura vetorial, criar um cadastro obrigatório de pessoas ou reescrever o app para satisfazer uma lista abstrata de entidades.

## 18. Cenários integrados de aceite

| ID | Cenário | Resultado esperado |
| --- | --- | --- |
| A01 | Capturar texto com IA indisponível | Original salvo e processamento pendente legível |
| A02 | Capturar offline no telefone e reconectar | Um item recebido; pendência local só some após confirmação |
| A03 | Aceitar dia e ocorrer falha intermediária | Nenhum planejamento parcial nem outbox órfã |
| A04 | Encerrar e levar itens para amanhã | Prazo preservado, sessão consistente e retry sem novo adiamento |
| A05 | Adiar Task e reabrir Home | Sai de Agora até o dia planejado; prazo importante permanece visível |
| A06 | Começar B sobre A e desfazer | Inversa consistente; edição posterior não é sobrescrita |
| A07 | Repetir proposta Hermes após timeout | Um efeito local e recibo recuperável |
| A08 | Editar campos distintos offline | Convergência preserva as duas intenções |
| A09 | Buscar uma decisão por frase vaga | Fonte relevante no resultado e trecho verificável |
| A10 | Parar reunião, reiniciar e reprocessar | Áudio preservado, job retoma e Tasks não duplicam |
| A11 | Corrigir triagem automática | Conteúdo original e relação correta permanecem acessíveis |
| A12 | Voltar após vários dias ausente | Poucas ações viáveis, sem obrigação de limpar toda a Inbox |
| A13 | Dispensar um aviso nos contextos definidos | Não reaparece sem mudança relevante segundo a política |
| A14 | Usar versão anterior suportada em outra réplica | Protocolo compatível ou limitação explícita, sem corrupção silenciosa |
| A15 | Falhar API durante refresh | Última leitura preservada, identificada como desatualizada e com retry |
| A16 | Restaurar backup de validação | Dados e relações recuperados; versão/schema corretamente verificados |

Os cenários devem exercitar serviço, banco e adapter quando atravessarem camadas. Teste unitário sozinho não fecha um caso de sync, gravação ou integração externa.

## 19. Estratégia de entrega e controle de risco

Cada ciclo seguirá esta sequência:

1. Reproduzir o problema e registrar a causa.
2. Definir comportamento e os oito eixos.
3. Implementar a menor mudança que resolva o fluxo completo daquele ciclo.
4. Criar regressão para risco real de dados, concorrência ou protocolo.
5. Rodar testes afetados; ampliar para suites/builds dos consumidores quando necessário.
6. Inspecionar visualmente quando mudar UI.
7. Atualizar auditoria, decisão arquitetural e estado deste plano.
8. Preparar uma entrega revisável com limitações explícitas.

Alteração de schema terá migration para frente, teste de upgrade e verificação de compatibilidade. Chaves de idempotência, blobs e novos tipos sincronizáveis precisarão de decisão documentada. Credenciais não entram em fixture, log ou relatório.

Não vou usar contagem de arquivos, linhas ou features como sinal de progresso. Cada entrega deve demonstrar um gesto mais simples, uma falha eliminada ou uma informação que ficou mais fácil de recuperar.

## 20. Marcos de conclusão

### Marco 1 — confiança

- [x] Undo completo das ações prioritárias (TaskStart; demais variantes classificadas em ADR-074).
- [x] Resolução de proposta protegida contra duplicação.
- [x] Edição concorrente de objetivos preserva campos independentes.
- [x] Falhas críticas reproduzidas cobertas por regressão.

### Marco 2 — continuidade diária entre dispositivos

- [x] Começar, encerrar e retomar o dia no desktop e na PWA.
- [ ] Captura mobile pendente funciona sob o contrato offline definido.
- [ ] Estado desatualizado, indisponível e sincronizado é compreensível.
- [x] Política de foco entre réplicas implementada e testada.

### Marco 3 — recuperar e agir pelo contexto

- [ ] Serviço compartilhado de contexto/ação Hermes.
- [ ] Consultas de Project, Waiting For e reunião com fontes.
- [ ] Busca medida contra corpus e melhorada sem regressão relevante.
- [ ] Ferramentas novas possuem política, idempotência e inversa quando aplicável.

### Marco 4 — organizar sem trabalho administrativo constante

- [ ] Triagem de Capture com fallback, revisão e correção.
- [ ] Reuniões processadas e reprocessadas sem duplicar ações.
- [ ] Pulse apresenta poucas atenções úteis e respeita dispensa.
- [ ] Métricas mostram onde ainda existe intervenção manual excessiva.

### Marco 5 — produto validado e distribuível

- [ ] Testes automáticos e builds dos consumidores aprovados.
- [ ] Fluxos nativos e mobile inspecionados em ambiente real disponível.
- [ ] Backup, upgrade e recuperação verificados.
- [ ] Integrações testadas ou limitações claramente registradas.
- [ ] Pacote, notas de versão e estado de instalação documentados.
- [ ] Período de uso real avaliado antes de afirmar melhora de hábito.

O último item não pode ser provado em uma sessão de programação. Uma janela inicial de sete dias de uso pode orientar a avaliação, mas não será tratada como estudo estatístico nem executada por automação recorrente sem solicitação.

## 21. Próximo passo concreto

O primeiro ciclo planejado é **F0 + F1**: reproduzir e corrigir Undo de TaskStart, repetição de propostas Hermes e escrita de objetivos por campo. São os problemas que precisam estar controlados antes de aumentar a autonomia da IA.

Em seguida, vou fechar **encerramento e retomada no celular**, usando o domínio já existente. Contexto, Search e triagem evoluem sobre essa base, enquanto o sync e o transporte de conteúdo recebem experimentos próprios.

O objetivo de conclusão permanece o da missão: abrir o M/OS deve ajudar a decidir e executar imediatamente. O usuário não deve precisar organizar tudo para começar a receber valor.

## 22. Registro de execução

| Frente | Estado | Entrega | Evidência |
| --- | --- | --- | --- |
| F0 | Feito | Baseline do checkpoint: suites `mos-core` + `mos-storage-sqlite` verdes antes de qualquer mudança; alterações do ciclo anterior preservadas e commitadas junto com F1 | `cargo test -p mos-core -p mos-storage-sqlite` |
| F1.1 | Feito | `UndoTaskStart`: inversa atômica com guarda contra edição posterior e sem religar foco quando outra Task corre | `piloto_planejamento::desfazer_*` (duas réplicas) |
| F1.2 | Feito | `ProposalStatus::Executing` + `claim_proposal`; desfecho recuperado em retry; reserva órfã fica incerta | `proposta_idempotente` |
| F1.3 | Feito | `edit_objective` por intenção; `save_reminder` emite diff | `titulo_num_aparelho_e_conclusao_no_outro_sobrevivem_juntos` |

| F2.1 | Feito | Encerrar o dia na PWA; retry depois de timeout devolve o dia encerrado | `de_bolso::encerrar_o_dia_no_bolso_move_para_amanha_sem_tocar_o_prazo` |
| F2.2 | Feito | Rescue Mode na PWA; `aplicar_resgate` no core com falha por item | `de_bolso::resgate_no_bolso_aplica_o_lote_e_aponta_a_falha`; bancada 390px |

| F3.2 | Feito | Foco = início mais recente, derivado na leitura; parar solta sobras | `piloto_planejamento::inicios_offline_convergem_para_o_mais_recente_em_qualquer_ordem` |

Decisões registradas em ADR-074 (F1), ADR-075 (F2) e ADR-076 (F3.2).
