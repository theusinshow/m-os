# Hermes — propostas de evolução como agente do M/OS

**Data:** 24/09/2026  
**Status:** discussão de produto; sem implementação nesta etapa.  
**Escopo:** melhorias do Hermes e possíveis especialistas internos, mantendo uma experiência simples para o usuário.

Referências: [auditoria](MOS-AUDIT.md), [plano de continuidade](MOS-OVERHAUL-PLAN.md), [Action Layer](HERMES-ACTION-LAYER.md) e [ideias de features](MOS-FEATURE-CANDIDATES.md).

## 1. A direção que proponho

O Hermes deve conseguir receber uma intenção, buscar o contexto necessário, realizar etapas autorizadas, verificar o resultado e manter continuidade quando algo interromper o trabalho.

Exemplo da experiência desejada:

> “Prepare a entrega da Caixa 02 e me mostre o que ainda falta.”

Ele poderia consultar o projeto, encontrar tarefas abertas e decisões relevantes, verificar quais referências estão disponíveis, preparar um checklist e um rascunho de entrega. O resultado mostraria o que foi verificado, o que foi produzido e o que ainda depende de uma pessoa ou arquivo.

“Preparar” não implica enviar mensagens, publicar arquivos ou confirmar uma entrega externa. O escopo da missão e a política de cada ferramenta determinam o que ele pode executar.

## 2. O ponto de partida real

O Hermes já possui contexto da interface, busca de candidatos, consulta limitada, contratos de ação, prévia, execução por serviços e Undo para algumas operações. Há automações específicas em reuniões e voz.

As lacunas relevantes para as propostas são:

- Contexto de projeto/reunião ainda fragmentado.
- Executor de ações acoplado ao adapter desktop.
- Inversas incompletas para certos efeitos.
- Falta de idempotência persistente da execução inteira em determinados caminhos.
- Ausência de uma camada geral verificada de trabalho com múltiplas etapas e retomada.
- Paridade limitada do Hermes na PWA.

O [contrato do gateway](HERMES-GATEWAY-CONTRACT.md) registra uma investigação histórica. Capacidades, modelo, sessões e suporte a delegação precisam ser reconferidos antes de escolher uma implementação. Estas propostas não afirmam que o gateway atual já oferece tudo isso.

## 3. H01 — missões com várias etapas e retomada

### Experiência

> “Revise as pendências desse projeto e prepare o que preciso resolver antes de sexta.”

O Hermes transforma o pedido em um trabalho delimitado: consultar fontes, identificar pendências, preparar sugestões, aplicar o que estiver autorizado e verificar efeitos. Mostra uma visão curta do andamento e permite interromper ou corrigir o rumo.

### O que seria novo

- Registro persistente do pedido e do escopo.
- Etapas com entrada, resultado e estado conhecido.
- Retomada após fechamento, timeout ou queda do gateway.
- Distinção entre aguardando dado, aguardando decisão, falha recuperável e concluído.
- Limites de tentativas, tempo e consumo para evitar ciclos improdutivos.

### Condição de confiança

Uma etapa concluída não pode ser repetida por simples reconexão. Antes de retomar, o Hermes verifica se as fontes e os alvos mudaram. Efeitos externos ambíguos exigem reconciliação, não retry cego.

**Prova de valor:** uma missão interrompida retoma sem duplicar tarefas, perder artefatos ou declarar conclusão antes de verificar os efeitos.

## 4. H02 — compreensão do trabalho em andamento

### Experiência

Dentro de uma Task, você diz:

> “Cria um lembrete para revisar isso amanhã.”

O Hermes usa a Task selecionada e sua relação com Project como referência. Se a intenção estiver clara, não exige que você repita título, projeto e contexto. Se houver dois alvos plausíveis, faz uma pergunta específica.

### Evolução proposta

- Contexto da interface atualizado por turno, sem carregar indefinidamente uma seleção antiga.
- Memória curta da missão: objetivo, entidades envolvidas, decisões já tomadas e próximos passos.
- Recuperação de dependências, esperas, checklist e documentos relacionados conforme a pergunta.
- Explicação breve das inferências que alterem o resultado.

**Limite:** contexto da tela não autoriza acesso irrestrito ao computador nem permite inventar o estado interno de um arquivo CAD.

**Prova de valor:** referências como “isso”, “esse projeto” e “o que ficou aberto” são resolvidas corretamente, inclusive depois de trocar de tela ou interromper a conversa.

## 5. H03 — investigação com fontes e lacunas explícitas

### Experiência

> “Descubra por que essa entrega ainda está travada.”

O Hermes consulta tarefas, bloqueios, esperas, decisões e reuniões disponíveis. Apresenta fatos sustentados por fontes, possíveis explicações e informação faltante.

### Evolução proposta

- Consultas por relação, intervalo de tempo e entidade.
- Fontes navegáveis para afirmações relevantes.
- Separação entre fato registrado, hipótese e ausência de dados.
- Verificação de versões e datas antes de usar uma decisão antiga.
- Detecção de informações potencialmente contraditórias para revisão.

**Limite:** “não encontrei uma resposta registrada” não significa “a pessoa não respondeu”. Múltiplos agentes concordando também não substituem evidência.

**Prova de valor:** uma conclusão importante pode ser conferida na fonte, e o Hermes reconhece quando os registros não permitem concluir.

## 6. H04 — produzir materiais de trabalho

### Experiência

> “Monte a pauta da reunião com o Victor.”

ou:

> “Prepare um resumo de entrega com decisões, arquivos e pendências.”

O Hermes cria um resultado utilizável: pauta, checklist, relatório, comparação, nota de decisão ou rascunho de mensagem. O material pode ser salvo como Resource relacionado ao projeto e revisado pelo usuário.

### Evolução proposta

- Artefato associado à missão e às fontes utilizadas.
- Edição e versões com origem clara.
- Atualização de um material existente sem sobrescrever alterações humanas silenciosamente.
- Indicação de trechos incompletos e dados que precisam de confirmação.

**Limite:** preparar mensagem não é enviá-la. Alterar arquivos externos exige ferramenta e escopo próprios; não é consequência automática de salvar um Resource.

**Prova de valor:** o usuário consegue aproveitar o material com poucas correções e entender o que foi incluído ou omitido.

## 7. H05 — memória de preferências corrigível

### Experiência

Você diz:

> “Para esse cliente, sempre separe pendências técnicas das informações que preciso receber.”

O Hermes pode guardar essa preferência no escopo adequado e aplicá-la em novos materiais daquele cliente/projeto.

### Evolução proposta

- Distinguir preferências declaradas, fatos do projeto e hipóteses temporárias.
- Definir escopo: pessoal, projeto, cliente ou missão.
- Mostrar o que está sendo lembrado e permitir corrigir ou esquecer.
- Registrar origem, data e condição de validade.
- Não promover uma correção isolada a regra universal.

**Limite:** memória não pode transformar uma instrução antiga em autorização permanente para ações externas ou destrutivas. Não deve misturar contexto de projetos independentes sem motivo.

**Prova de valor:** diminui a repetição de instruções sem aplicar preferências fora do contexto em que fazem sentido.

## 8. H06 — acompanhamento de perguntas e condições

### Experiência

> “Quando houver informação suficiente para decidir qual base usar, me mostre.”

O Hermes registra uma pergunta delimitada. Quando uma nova reunião, Capture ou atualização relacionada chega, verifica se há uma resposta candidata e mostra a evidência.

### Evolução proposta

- Pergunta com entidade, condição e prazo/validade quando aplicável.
- Reavaliação por evento relevante, aproveitando os jobs existentes.
- Estados: aberta, possível resposta encontrada, resolvida ou dispensada.
- Controle para não repetir a mesma sugestão sem mudança.
- Preparação de próximos passos, como um rascunho de follow-up, sem envio implícito.

**Limite:** o escopo inicial usa dados que já entraram no M/OS. Isso não concede acesso automático a email, WhatsApp ou arquivos externos.

**Prova de valor:** uma informação nova resolve uma dúvida antiga sem obrigar o usuário a lembrar de pesquisar novamente e sem gerar notificações repetidas.

## 9. H07 — especialistas internos sob uma conversa única

### Experiência

Para um pedido complexo, o Hermes pode dividir o trabalho internamente, por exemplo:

| Papel possível | Responsabilidade | Limite |
| --- | --- | --- |
| Pesquisador | Encontrar fontes e contexto relacionado | Leitura delimitada; não altera entidades |
| Planejador | Propor sequência e dependências | Não executa o próprio plano diretamente |
| Revisor | Procurar omissões, conflitos e afirmações sem suporte | Relata evidências; não presume que concordância prova correção |
| Executor | Aplicar operações aprovadas pela política | Usa serviços, validações e registro de efeitos |

O usuário continua falando com um Hermes. Não precisa escolher um especialista, copiar contexto entre chats ou resolver versões concorrentes de uma mesma tarefa.

### Critério para adotar

Primeiro eu testaria esses papéis como etapas de um único agente. Delegação real para agentes separados só entraria se demonstrasse ganho de qualidade ou latência que compensasse custo e coordenação.

Não há motivo para executar quatro modelos para criar um lembrete. Também não é desejável ter vários agentes escrevendo simultaneamente na mesma entidade sem um coordenador de efeitos.

**Prova de valor:** comparação com execução por um agente, usando os mesmos casos e fontes; menos omissões ou menor tempo sem aumento indevido de custo e erros.

## 10. H08 — recibo verificável e autonomia proporcional à ação

### Experiência

Depois de agir, o Hermes informa algo como:

> “Criei duas Tasks, associei três referências ao projeto e deixei uma data para você esclarecer.”

O recibo abre as entidades e permite desfazer os efeitos reversíveis. Etapas que falharam ou não foram executadas ficam visíveis.

### Evolução proposta

- Registro dos efeitos realizados, sem depender da narrativa gerada pelo modelo.
- Verificação por leitura após a escrita quando necessária.
- Undo com proteção contra mudanças posteriores.
- Tratamento claro de aplicação parcial e resultado externo incerto.
- Política que considere intenção do usuário, risco, reversibilidade, ambiguidade e capacidade disponível.
- Diagnóstico de latência, tentativas e consumo, sem expor raciocínio interno privado nem armazenar conteúdo excessivo nos logs.

Consultas e ações locais claramente solicitadas não deveriam gerar confirmações repetidas por padrão. Operações fora do escopo ou com consequências diferentes exigem tratamento específico. “Confiança do modelo” sozinha não concede autoridade.

**Prova de valor:** o recibo corresponde ao banco/serviço, repetir a chamada não cria outro efeito e desfazer não apaga uma edição mais recente.

## 11. Jornada integrada possível

Pedido:

> “Prepare a reunião da Caixa 02 e destaque o que preciso cobrar do Victor.”

1. **H02:** identifica projeto e reunião ou pergunta apenas o alvo ambíguo.
2. **H01:** registra o trabalho e suas etapas.
3. **H03:** recupera pendências e fontes.
4. **H05:** aplica preferências pertinentes de organização da pauta.
5. **H07:** usa revisão especializada apenas se o caso justificar.
6. **H04:** prepara uma pauta editável com referências.
7. **H08:** mostra o que produziu e o que ficou incerto.
8. **H06:** pode manter uma pergunta aberta se isso tiver sido solicitado.

Essa jornada não envia cobranças, não agenda reunião externa e não marca pendências como resolvidas por conta própria.

## 12. O que priorizaria

| Ordem sugerida | Capacidade | Razão |
| --- | --- | --- |
| 1 | H08 + fundamentos de execução de H01 | Impedir duplicação e garantir que efeito, estado e Undo sejam confiáveis |
| 2 | H02 e H03 | Melhor contexto e melhores evidências beneficiam quase todos os pedidos |
| 3 | H04 e retomada completa de H01 | Passar de resposta para trabalho preparado e recuperável |
| 4 | H05 e H06 | Reduzir repetição de instruções e acompanhamento manual |
| 5 | H07, se o experimento justificar | Delegação é um meio; a experiência e a qualidade são o objetivo |

Não proponho começar pela troca de modelo ou por uma equipe permanente de agentes. Antes disso, é preciso medir recuperação de contexto, falhas de ferramentas, latência e tarefas concluídas corretamente.

## 13. Oito eixos de viabilidade

| Eixo | Direção proposta |
| --- | --- |
| Core | Contratos de contexto, missão, etapas, efeitos e política portáveis; adapters mantêm detalhes de plataforma |
| Database | Persistir o necessário para retomada, idempotência, preferências e proveniência; migrations aditivas quando justificadas |
| Sync | Resultados e decisões pessoais podem viajar; leases de execução, credenciais e caches exigem escopo específico; impedir execução duplicada entre dispositivos |
| Desktop | Contexto da tela e capacidades locais; andamento e recibo dentro da experiência Hermes existente |
| iOS/PWA | Consultar missões, esclarecer pendências, revisar materiais e usar as ações disponíveis pelo mesmo serviço; sem prometer microfone/processos do desktop |
| Notifications | Progresso ambiente por padrão; interrupção apenas por condição relevante, com dedupe e destino útil |
| Hermes | Uma interface central, ferramentas explícitas e orçamento por trabalho; especialistas opcionais subordinados ao mesmo escopo |
| Tests | Missão interrompida, retry, fonte ausente, ação parcial, edição posterior, duplicação entre réplicas, memória fora de escopo e qualidade com/sem delegação |

Essas respostas enquadram a discussão. Cada capacidade selecionada ainda precisa do checklist detalhado antes de implementação, conforme [FEATURE-DEVELOPMENT](FEATURE-DEVELOPMENT.md).

## 14. Perguntas que mudariam a prioridade

1. O Hermes hoje decepciona mais por não entender o contexto, não conseguir executar ou exigir que você acompanhe cada etapa?
2. O primeiro trabalho completo deveria ser organizar Captures, preparar reuniões ou acompanhar uma entrega de projeto?
3. Você quer principalmente materiais prontos para revisar ou também mudanças locais reversíveis aplicadas conforme seu pedido?

Nenhuma dessas preferências foi presumida como confirmada. Esta etapa produziu somente a documentação das propostas.
