# M/OS — possíveis novas features

**Data:** 24/09/2026  
**Status:** propostas para discussão. Nenhuma feature deste documento foi implementada nesta etapa.  
**Pedido:** conversar sobre possibilidades e documentá-las, sem alterar o aplicativo.

Este documento reúne hipóteses de produto baseadas na [auditoria](MOS-AUDIT.md), no [plano de continuidade](MOS-OVERHAUL-PLAN.md) e na missão de reduzir trabalho de organização. Não acrescenta automaticamente requisitos ao roadmap. A seleção e a ordem abaixo são recomendações do assistente, ainda não escolhas confirmadas pelo usuário.

## 1. O que faria uma feature valer a pena

Uma feature deve remover uma obrigação concreta: lembrar de algo, classificar uma entrada, reconstruir contexto, procurar uma decisão ou navegar entre módulos para agir.

O critério principal não é quantas entidades ou ferramentas o Hermes passa a conhecer. É quanto trabalho deixa de depender do usuário, sem tornar o resultado difícil de entender ou corrigir.

Não existem dados de adoção examinados que comprovem o impacto das propostas. Os benefícios abaixo são hipóteses a validar em cenários reais.

## 2. Visão geral

| ID | Possibilidade | O que mudaria no uso | Natureza | Interesse inicial |
| --- | --- | --- | --- | --- |
| NF01 | Inbox assistida | Capturar e receber uma ação pronta para aceitar ou corrigir | Evolução de Capture, ingestão e Hermes | Muito alto |
| NF02 | Resumo vivo do projeto | Abrir Project e entender situação, bloqueios e próximo passo | Nova experiência sobre relações existentes | Muito alto |
| NF03 | Busca que responde com fontes | Fazer uma pergunta vaga e encontrar resposta verificável | Evolução de Search e contexto Hermes | Muito alto |
| NF04 | Retomar de onde parei | Voltar ao trabalho com contexto recuperado | Nova experiência de continuidade | Alto |
| NF05 | Radar de compromissos | Ver o que devo e o que estou esperando de outras pessoas | Evolução de Waiting For e insights | Alto |
| NF06 | Preparação de reunião | Receber uma pauta curta baseada nas pendências anteriores | Nova composição de contexto | Alto |
| NF07 | Memória de decisões | Encontrar o que foi decidido, por quê e o que mudou | Evolução de insights e relações | Alto |
| NF08 | O que mudou desde a última visita | Retomar após ausência sem ler toda a atividade acumulada | Evolução de Rescue e Timeline | Alto |
| NF09 | Captura offline no celular | Guardar uma ideia mesmo sem conseguir acessar o servidor | Nova capacidade da PWA atual | Muito alto para uso móvel |
| NF10 | O que cabe no tempo que tenho | Encontrar uma ação viável para uma janela curta | Evolução do Piloto e das estimativas | Médio; precisa de bons dados |

“Nova experiência” não significa necessariamente nova página. O objetivo é aproveitar Home, Project, Task, Meeting, Search e Hermes como pontos de entrada.

## 3. NF01 — Inbox assistida

### A experiência

Você captura:

> “Cobrar Victor sexta sobre as bases da Caixa 02.”

O M/OS guarda o texto imediatamente. Depois apresenta uma interpretação pronta: possível associação ao projeto, pessoa citada e intenção de cobrar na sexta. Você pode aceitar, corrigir ou deixar como Capture.

O sistema deve distinguir “cobrar alguém na sexta” de “a pessoa prometeu entregar na sexta”. O segundo fato não pode ser inventado a partir do primeiro.

### O que poderia oferecer

- Transformar em Task, Reminder ou referência com os campos inferidos.
- Sugerir Project com base em contexto existente.
- Identificar possíveis esperas e datas.
- Revisar várias propostas em um lote pequeno.
- Executar metadata reversível conforme política validada, deixando recibo e correção acessíveis.

### Por que interessa

Ataca diretamente a dificuldade de voltar à Inbox para classificar tudo. O gesto principal passa a ser capturar; a organização chega depois.

### Base existente e novidade

Capture, ingestão, interpretação de voz e ações Hermes já existem. A novidade seria uma triagem consistente entre entradas, com decisões compactas e execução idempotente.

### Limites e validação

O original permanece disponível. Falha de IA deixa o item pendente, sem bloquear captura. Ambiguidade importante vira uma pergunta curta. Repetir análise não pode duplicar Task ou Reminder. Antes de autoexecução ampla, Undo e registro de efeitos precisam estar completos.

**Sinal de valor:** menos decisões por item, sem crescimento de correções ou desfazimentos.

## 4. NF02 — resumo vivo do projeto

### A experiência

Ao abrir “Caixa 02”, você encontra:

- O que avançou desde a última revisão.
- O que está aberto ou bloqueado.
- O que depende de alguém.
- Decisões recentes e suas fontes.
- Arquivos e referências relevantes.
- Um próximo passo justificável.

Hermes também poderia responder:

> “Me atualiza sobre a Caixa 02.”

### Por que interessa

Hoje as informações úteis podem estar distribuídas entre Task, reunião, Capture e Resource. A proposta reduz o trabalho de remontar o projeto mentalmente.

### Base existente e novidade

Projetos e relações já existem. A novidade é a composição de uma visão operacional com fontes e atualização explícita. Não depende de criar um grafo visual nem um cadastro adicional.

### Limites e validação

O resumo deve separar fatos, ausência de informação e inferências. “Não encontrei bloqueios registrados” é diferente de “não há bloqueios”. Conteúdo indisponível em outro dispositivo precisa ser identificado. Resumo derivado não substitui a fonte.

**Sinal de valor:** responder “como está esse projeto?” sem abrir várias telas, mantendo acesso às evidências.

## 5. NF03 — busca que responde com fontes

### A experiência

Você escreve:

> “Aquele negócio das bases que o Victor pediu.”

O sistema apresenta candidatos relacionados, explica o acerto e, quando houver conteúdo suficiente, responde com referência à Task, Capture ou trecho de reunião.

Outra consulta:

> “Por que mudamos a solução de drenagem?”

A resposta aponta para a decisão registrada e permite abrir sua origem.

### Por que interessa

É a capacidade que permite capturar sem precisar memorizar onde cada coisa foi guardada.

### Base existente e novidade

Search lexical, índices e descoberta Hermes já existem. A evolução combina texto, relações, datas e contexto da consulta. Embeddings são uma opção técnica a avaliar, não a própria feature nem uma dependência obrigatória.

### Limites e validação

Uma resposta sem evidência suficiente deve reconhecer a lacuna. Filtros de conteúdo apagado/arquivado precisam ser respeitados. Consultas exatas não podem piorar enquanto se melhora a busca vaga.

**Sinal de valor:** resultado útil nas primeiras posições de um corpus conhecido e menos procura manual para abrir a fonte correta.

## 6. NF04 — retomar de onde parei

### A experiência

Depois de uma interrupção, você abre uma Task ou Project e vê:

> “Você estava revisando a armadura. O próximo passo registrado era conferir a ligação com o pilar. Estes são os documentos relacionados.”

O cartão poderia combinar progresso do checklist, referências usadas dentro do M/OS, última decisão e uma nota opcional de pausa.

### Por que interessa

Começar a trabalhar novamente exige recuperar contexto. Essa proposta reduz esse custo, especialmente depois de alternar entre trabalho, faculdade e reuniões.

### Base existente e novidade

Task ativa, checklist, histórico e referências são a base. A novidade é uma memória curta de continuidade, apresentada na retomada.

### Limites e validação

O M/OS não sabe o estado interno de um arquivo CAD só porque o aplicativo estava aberto. A primeira versão deve usar fatos registrados no M/OS e notas explícitas, sem monitorar tela ou clipboard indiscriminadamente. Uma intenção antiga deve ser identificada como antiga e permitir correção.

**Sinal de valor:** menos tempo para recuperar o próximo passo e menos necessidade de procurar arquivos relacionados.

## 7. NF05 — radar de compromissos

### A experiência

Você pergunta:

> “O que estou esperando do Victor? E o que eu fiquei de mandar para ele?”

O sistema organiza as duas direções, com Project, origem, prazo conhecido e follow-up. Uma promessa extraída de reunião pode aparecer como sugestão antes de virar compromisso registrado.

### Por que interessa

Evita que pendências interpessoais fiquem espalhadas entre Tasks, lembretes e memória de reunião.

### Base existente e novidade

Waiting For e follow-up já existem. A evolução seria agrupar compromissos por pessoa/contexto e relacionar o que eu devo ao que espero receber.

### Limites e validação

Não precisa começar com uma entidade Person nem exigir cadastro de contatos. Nomes iguais ou aliases precisam de tratamento explícito. Sugerir um follow-up não significa enviar mensagem: envio externo não faz parte desta proposta inicial.

**Sinal de valor:** encontrar as pendências de uma conversa antes de falar com a pessoa, com compromissos corretamente atribuídos.

## 8. NF06 — preparação de reunião

### A experiência

Ao abrir uma reunião vinculada ao projeto, o M/OS oferece:

- O que ficou pendente da última conversa.
- Perguntas que ainda precisam de resposta.
- Decisões anteriores relevantes.
- Documentos que provavelmente serão consultados.
- Uma pauta curta editável.

### Por que interessa

O valor da reunião começa antes da gravação. Preparar contexto pode reduzir perguntas repetidas e decisões tomadas sem lembrar do histórico.

### Base existente e novidade

O Meeting Agent já processa reuniões encerradas. A novidade é usar essa memória para preparar a próxima reunião.

### Limites e validação

Se o M/OS não souber de uma reunião futura, o usuário pode abrir a preparação a partir do projeto. A feature não depende inicialmente de Google/Outlook Calendar. Pauta sugerida não é compromisso confirmado. Fontes antigas ou incompletas precisam ser sinalizadas.

**Sinal de valor:** uma pauta útil com poucas correções, sem pesquisar manualmente a reunião anterior.

## 9. NF07 — memória de decisões

### A experiência

Você pergunta:

> “Qual solução escolhemos? Por quê? Essa decisão ainda vale?”

O M/OS reúne a decisão, sua justificativa registrada, data, origem e relação com tarefas. Se houver substituição explícita por uma decisão posterior, mostra essa sequência.

### Por que interessa

Uma Task diz o que fazer. A decisão explica por que fazer. Preservar essa relação evita repetir discussão ou executar com base em uma escolha superada.

### Base existente e novidade

Insights e evidências de reunião já fornecem parte da estrutura. A evolução seria recuperar decisões entre fontes e representar revisão/substituição quando houver informação suficiente.

### Limites e validação

Uma fala em reunião não é automaticamente uma decisão. Texto mais recente não invalida sozinho uma decisão anterior. Registrar revisão exige evidência ou ação explícita. Decisões também podem ser capturadas em texto, sem reunião.

**Sinal de valor:** explicar uma escolha com origem verificável e distinguir discussão, decisão e substituição.

## 10. NF08 — o que mudou desde a última visita

### A experiência

Depois de alguns dias fora, você encontra uma síntese curta:

> “Há dois prazos próximos, uma reunião pronta para revisar e uma dependência ainda aberta. Estas são as três ações mais úteis para retomar.”

Dentro de um Project, a mesma capacidade poderia mostrar mudanças desde a última abertura daquele contexto.

### Por que interessa

O usuário não precisa ler uma Timeline inteira nem fazer uma revisão administrativa para voltar ao fluxo.

### Base existente e novidade

Rescue Mode e Timeline já existem. A evolução seria distinguir mudanças relevantes de simples atividade e resumir a diferença entre duas visitas.

### Limites e validação

É necessário definir se “última visita” é por aparelho, por projeto ou pessoal. O sistema não deve considerar um item entendido só porque abriu a Home. Um feed de todas as alterações seria ruído; a síntese precisa de seleção e fontes.

**Sinal de valor:** retomar com poucas decisões, sem perder mudanças importantes nem receber cobrança pelo período ausente.

## 11. NF09 — captura offline no celular

### A experiência

Sem internet, você abre a captura, escreve e continua o que estava fazendo. O M/OS informa que a entrada está guardada no aparelho e será enviada quando a conexão e a sessão permitirem.

### Por que interessa

Uma memória externa precisa estar disponível no momento da ideia. Depender de acesso imediato ao servidor pode quebrar o hábito de capturar.

### Base existente e novidade

A PWA atual usa a réplica SQLite do servidor. A novidade seria uma fila durável de Captures no navegador, com reenvio idempotente. Isso não equivale a replicar todo o domínio no iPhone.

### Limites e validação

Guardar localmente e chegar ao servidor são estados diferentes. Quota, limpeza de armazenamento, fechamento da PWA, sessão expirada e troca de conta precisam ser tratados. A proposta não promete sincronização contínua em background quando a plataforma não a permitir.

**Sinal de valor:** captura preservada ao fechar/reabrir e entregue uma única vez após reconexão, com falha de armazenamento explicitamente informada.

## 12. NF10 — o que cabe no tempo que tenho

### A experiência

Você diz:

> “Tenho quinze minutos antes de sair.”

O M/OS sugere uma ou duas ações compatíveis com a janela, prioridades, dependências e estimativas conhecidas. Também pode explicar por que uma tarefa aparentemente pequena não cabe nesse intervalo.

### Por que interessa

Reduz o esforço de escolher uma tarefa em pequenos intervalos, sem transformar o dia inteiro em um calendário rígido.

### Base existente e novidade

O Piloto já recomenda próximos passos e Tasks podem ter estimativa. A evolução seria uma recomendação condicionada à disponibilidade declarada pelo usuário.

### Limites e validação

Estimativas são aproximadas. O sistema não deve inventar duração precisa nem decidir que você está cansado a partir de pouca atividade. A informação “tenho quinze minutos” é suficiente para a primeira experiência; não exige monitoramento comportamental novo.

**Sinal de valor:** recomendações executáveis dentro da janela, sem ignorar prioridades ou gerar mais planejamento manual.

## 13. Como as propostas se complementam

Um exemplo de jornada possível:

1. Você captura uma informação no telefone, mesmo offline — **NF09**.
2. A Inbox propõe como tratá-la — **NF01**.
3. O item passa a compor o contexto do projeto — **NF02**.
4. Um compromisso fica visível no radar — **NF05**.
5. Antes da próxima reunião, ele aparece na preparação — **NF06**.
6. A decisão tomada fica ligada à origem — **NF07**.
7. Depois você a reencontra por uma pergunta vaga — **NF03**.
8. Ao voltar ao trabalho ou ao sistema, recupera contexto — **NF04/NF08**.
9. Com pouco tempo disponível, escolhe um passo viável — **NF10**.

Não seria necessário construir todas de uma vez. O exemplo mostra como aproveitar as mesmas entidades e serviços em experiências diferentes.

## 14. Minha recomendação de prioridade

### Primeiro grupo: maior ligação com o problema relatado

**NF01, NF02 e NF03:** Inbox assistida, resumo vivo do projeto e busca com fontes. Juntas, ajudam a colocar informação, entender o contexto e recuperar depois.

Se a principal barreira for capturar longe do computador, **NF09** deve entrar nesse grupo. Essa troca depende do uso real, ainda não medido.

### Segundo grupo: continuidade do trabalho

**NF04, NF05 e NF08:** retomada, compromissos e mudanças desde a última visita. Atacam a dificuldade de voltar ao sistema e reconstruir pendências.

### Terceiro grupo: aproveitar melhor a memória de reuniões

**NF06 e NF07:** preparação e decisões. Podem ganhar prioridade se reuniões forem a principal origem de trabalho e esquecimento.

### Experimento menor após consolidar dados

**NF10:** recomendação por tempo disponível. É útil, mas depende de estimativas e estado de trabalho suficientemente confiáveis.

Essa é uma prioridade de valor potencial, não a ordem técnica completa. Undo, idempotência, persistência e tratamento de conflitos continuam sendo pré-requisitos para features que escrevem ou automatizam ações.

## 15. Oito eixos para discussão de viabilidade

Estas são hipóteses de enquadramento, não decisões finais de arquitetura. “iOS” representa a PWA existente; não implica cliente nativo novo.

### Core, Database, Sync e Hermes

| ID | Core | Database | Sync | Hermes |
| --- | --- | --- | --- | --- |
| NF01 | Política de proposta/aplicação | Originais, propostas, efeitos e jobs | Entidades/decisões; execução com dedupe | Classifica, explica e age conforme política |
| NF02 | Consulta de contexto do projeto | Preferir projeção de relações existentes | Fontes sincronizadas; disponibilidade explícita | Resume e abre evidências |
| NF03 | Recuperação e ranking compartilhados | Índices derivados | Fontes viajam; índice recomposto | Resposta limitada pelas fontes |
| NF04 | Modelo de continuidade a definir | Ponto de retomada e fontes, se necessário | Intenção pessoal pode viajar; estado local de apps não | Explica a retomada sem inventar progresso |
| NF05 | Consulta de compromissos nas duas direções | Esperas/insights existentes; identidade leve | Compromissos e correções | Resolve contexto e sugere follow-up |
| NF06 | Composição de pauta/contexto | Pauta derivada; edição persistida se houver | Fonte e pauta editada conforme contrato | Prepara perguntas com origem |
| NF07 | Decisão, evidência e substituição explícita | Reusar insights; extensão só quando necessária | Decisões/relações com compatibilidade definida | Explica escolhas e seus limites |
| NF08 | Seleção de mudanças relevantes | Cursor de visita com escopo definido | Decidir cursor pessoal versus por aparelho | Sintetiza fatos disponíveis |
| NF09 | Contrato de captura idempotente | Fila local do browser e recepção no servidor | Reenvio sem duplicação | Processa depois; não bloqueia a captura |
| NF10 | Seleção condicionada a janela | Estimativas existentes; sem novo diário obrigatório | Estimativas viajam; janela pode ser efêmera | Entende disponibilidade e explica opções |

### Desktop, iOS, Notifications e Tests

| ID | Desktop | iOS/PWA | Notifications | Tests |
| --- | --- | --- | --- | --- |
| NF01 | Inbox e revisão compacta | Aceitar/corrigir por toque | Ambient por padrão | Duplicação, ambiguidade, erro e Undo |
| NF02 | Contexto dentro de Project | Resumo compacto e fontes navegáveis | Nenhuma por padrão | Dados ausentes, fontes e atualização |
| NF03 | Search/Command/Hermes | Busca por toque | Nenhuma: consulta solicitada | Corpus, filtros e evidência |
| NF04 | Ação de continuar na Task/Project | Continuidade do conteúdo disponível | Nenhuma por padrão | Contexto antigo, ausência de fonte, correção |
| NF05 | Consulta por pessoa/projeto | Lista de pendências acionáveis | Só follow-up devido conforme política existente | Homônimos, autoria e duplicatas |
| NF06 | Preparação no Project/Meeting | Leitura e edição breve | Opcional para reunião conhecida, com dedupe | Fontes, falta de agenda e pauta editada |
| NF07 | Detalhe de Project/Meeting e Search | Leitura e consulta | Nenhuma por padrão | Discussão versus decisão; substituição |
| NF08 | Home ou Project na retomada | Síntese curta | Ambient; sem cobrança de retorno | Cursor, dois aparelhos e dispensa |
| NF09 | Receber captura pelo sync existente | Fila local e estados de envio | Estado na captura; sem spam de reconexão | Modo avião, reabertura, sessão e quota |
| NF10 | Pergunta ou filtro rápido | Seleção simples de disponibilidade | Nenhuma | Estimativa ausente, dependência e prioridade |

## 16. O que eu deixaria fora por enquanto

- Um Pulse separado: o Piloto já ocupa esse papel e deve ser evoluído.
- Outra Command Palette concorrente com os comandos existentes.
- Um grafo visual como condição para relacionar entidades.
- Cadastro obrigatório de pessoas, categorias ou projetos antes de capturar.
- Envio automático de cobranças por email ou mensageria como consequência implícita de um Waiting For.
- Monitoramento indiscriminado de tela, áudio ou clipboard para alimentar IA.
- Gamificação que transforme uso do sistema em uma obrigação adicional.
- Embeddings ou um novo banco como símbolo de “inteligência”, sem benefício medido.
- Novo app iOS antes de demonstrar quais limitações da PWA precisam ser resolvidas nativamente.

## 17. Perguntas para continuar a conversa

1. O maior incômodo hoje é colocar informação, reencontrar depois ou decidir o que fazer?
2. Você perde mais contexto entre projetos, depois de reuniões ou depois de alguns dias sem abrir o M/OS?
3. Para organização automática, o comportamento preferido seria sugestões prontas ou aplicação reversível com aviso discreto nos casos bem definidos?

Essas respostas podem mudar a prioridade. Nenhuma preferência foi presumida como confirmada, e nenhuma feature foi adicionada ao aplicativo nesta discussão.
