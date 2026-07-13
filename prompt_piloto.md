# Projeto HoraCAD — Rastreador de horas para desenhistas e projetistas

Leia este documento completamente antes de criar ou modificar qualquer arquivo.

Você atuará como arquiteto de software, desenvolvedor desktop full-stack e responsável técnico pelo início deste projeto.

## 1. Contexto do problema

Sou desenhista cadista e trabalho diariamente em diferentes desenhos, obras e projetos usando principalmente softwares como AutoCAD.

Atualmente tenho dois problemas:

1. Esqueço de registrar o horário em que comecei e terminei cada projeto.
2. Quando preciso cobrar pelo serviço, não sei exatamente quantas horas trabalhei em cada projeto.

Aplicativos tradicionais de controle de horas dependem de iniciar e parar o cronômetro manualmente. Isso não resolve completamente o problema, porque justamente esqueço de iniciar o cronômetro.

O software deve resolver isso por meio de:

* cronômetro por projeto;
* lembretes automáticos;
* detecção de programas CAD abertos;
* detecção de inatividade;
* reconstrução aproximada de períodos esquecidos;
* relatórios de horas e valores para cobrança.

## 2. Nome do projeto

Nome provisório:

**HoraCAD**

O nome deve estar centralizado em constantes e configurações para poder ser alterado futuramente sem precisar procurar textos espalhados pelo código.

## 3. Objetivo do MVP

Criar um aplicativo desktop para Windows que funcione localmente, permaneça disponível na bandeja do sistema e permita:

* cadastrar clientes;
* cadastrar projetos;
* definir valor por hora;
* iniciar, pausar, continuar e encerrar um cronômetro;
* manter apenas um cronômetro ativo por vez;
* detectar quando programas monitorados forem abertos;
* lembrar o usuário de escolher um projeto;
* detectar períodos de inatividade;
* permitir correção manual;
* consultar histórico;
* calcular horas e valores;
* gerar um relatório simples;
* continuar funcionando sem internet;
* preservar os dados entre reinicializações.

O MVP deve ser local-first. Não implementar autenticação, servidor externo ou sincronização em nuvem neste momento.

## 4. Plataforma e stack

Utilize:

* Tauri 2;
* React;
* TypeScript;
* Vite;
* Rust no backend do Tauri;
* SQLite local;
* plugin SQL oficial do Tauri;
* Tailwind CSS;
* React Router;
* Zustand ou solução simples equivalente para estado da interface;
* Zod para validações quando fizer sentido;
* biblioteca leve de ícones, preferencialmente Lucide;
* Vitest para testes unitários do frontend;
* testes Rust para regras implementadas no backend.

Use npm como gerenciador de pacotes, salvo se o repositório já possuir outro gerenciador definido.

Não use Electron.

Não adicione dependências sem necessidade. Antes de adicionar uma biblioteca, avalie se o recurso pode ser implementado com as APIs já existentes no projeto.

## 5. Prioridades técnicas

A ordem de prioridade é:

1. confiabilidade dos registros;
2. recuperação após fechamento inesperado;
3. simplicidade de uso;
4. funcionamento offline;
5. clareza dos relatórios;
6. baixo consumo de recursos;
7. aparência visual;
8. funcionalidades adicionais.

Não sacrifique confiabilidade para criar animações ou elementos visuais desnecessários.

## 6. Princípios do produto

O aplicativo deve exigir o mínimo possível de esforço do usuário.

A pergunta central de cada decisão deve ser:

> Isso reduz a possibilidade de o usuário esquecer de registrar seu trabalho?

O sistema não deve ser uma ferramenta de vigilância.

Não implementar:

* captura de tela;
* registro de teclas;
* leitura do conteúdo dos desenhos;
* envio de dados para terceiros;
* monitoramento de texto digitado;
* gravação detalhada da atividade do usuário;
* telemetria sem consentimento;
* leitura de arquivos CAD no MVP.

O aplicativo pode registrar:

* horário em que um processo monitorado foi detectado;
* horário em que o processo deixou de existir;
* tempo de inatividade do computador;
* projeto selecionado;
* sessões de trabalho;
* correções realizadas manualmente.

## 7. Arquitetura geral

Organize o projeto com separação clara entre:

* apresentação;
* estado da aplicação;
* regras de domínio;
* persistência;
* integração com o sistema operacional;
* comandos Tauri;
* geração de relatórios.

Estrutura inicial sugerida:

```text
horacad/
├── src/
│   ├── app/
│   │   ├── App.tsx
│   │   ├── router.tsx
│   │   └── providers.tsx
│   ├── components/
│   │   ├── layout/
│   │   ├── timer/
│   │   ├── projects/
│   │   ├── history/
│   │   ├── reports/
│   │   └── ui/
│   ├── features/
│   │   ├── clients/
│   │   ├── projects/
│   │   ├── timer/
│   │   ├── activity/
│   │   ├── reports/
│   │   └── settings/
│   ├── hooks/
│   ├── lib/
│   ├── services/
│   ├── stores/
│   ├── types/
│   └── styles/
├── src-tauri/
│   ├── src/
│   │   ├── commands/
│   │   ├── database/
│   │   ├── domain/
│   │   ├── monitoring/
│   │   ├── notifications/
│   │   ├── tray/
│   │   ├── state/
│   │   ├── lib.rs
│   │   └── main.rs
│   ├── migrations/
│   └── capabilities/
├── docs/
│   ├── PRODUCT.md
│   ├── ARCHITECTURE.md
│   ├── DATABASE.md
│   ├── UX-FLOWS.md
│   └── ROADMAP.md
├── CLAUDE.md
├── README.md
└── package.json
```

Adapte a estrutura quando houver uma justificativa técnica clara. Evite criar abstrações sem uso real.

## 8. Modelo de dados

Utilize IDs UUID em formato texto ou outra solução consistente e segura para dados locais.

### clients

```text
id
name
company_name
email
phone
notes
created_at
updated_at
archived_at
```

### projects

```text
id
client_id
name
code
description
hourly_rate_cents
status
color
created_at
updated_at
archived_at
```

Status possíveis:

```text
active
paused
completed
archived
```

Armazene valores monetários em centavos usando números inteiros.

### time_entries

```text
id
project_id
started_at
ended_at
duration_seconds
idle_seconds
description
activity_type
billable
hourly_rate_snapshot_cents
source
created_at
updated_at
deleted_at
```

Valores possíveis para `source`:

```text
timer
manual
reconstructed
```

Valores iniciais possíveis para `activity_type`:

```text
drawing
detailing
revision
meeting
study
other
```

O campo `hourly_rate_snapshot_cents` deve preservar o valor da hora utilizado no momento da sessão. Alterar o valor atual do projeto não pode modificar automaticamente sessões anteriores.

### active_timer

Deve existir no máximo um registro ativo.

```text
id
project_id
started_at
last_resumed_at
accumulated_seconds
status
description
activity_type
created_at
updated_at
```

Status:

```text
running
paused
```

A persistência desse estado é obrigatória para permitir recuperação após fechamento inesperado.

### monitored_apps

```text
id
display_name
process_name
enabled
remind_on_open
remind_on_close
created_at
updated_at
```

Cadastre inicialmente, quando aplicável:

```text
acad.exe
revit.exe
sketchup.exe
eberick.exe
qibuilder.exe
```

Não assuma que todos estão instalados.

### activity_events

Essa tabela será usada para reconstrução aproximada do dia.

```text
id
event_type
process_name
detected_at
metadata_json
processed
created_at
```

Eventos possíveis:

```text
app_opened
app_closed
idle_started
idle_ended
timer_started
timer_paused
timer_resumed
timer_stopped
```

### settings

Pode ser uma tabela chave-valor tipada ou uma tabela com campos explícitos.

Configurações iniciais:

```text
idle_detection_enabled
idle_threshold_minutes
process_monitoring_enabled
process_check_interval_seconds
remind_when_monitored_app_opens
remind_when_monitored_app_closes
rounding_enabled
rounding_interval_minutes
rounding_mode
start_with_windows
minimize_to_tray
close_to_tray
currency
locale
```

Valores iniciais recomendados:

```text
idle_threshold_minutes = 10
process_check_interval_seconds = 5
rounding_enabled = false
currency = BRL
locale = pt-BR
minimize_to_tray = true
close_to_tray = true
```

Crie migrations versionadas. Não crie tabelas diretamente e informalmente durante a inicialização.

## 9. Regras centrais do cronômetro

Implemente as regras de domínio separadas da interface.

### Iniciar

Ao iniciar um projeto:

* verificar se já existe cronômetro ativo;
* quando não existir, criar o estado ativo;
* quando existir outro cronômetro, não iniciar silenciosamente;
* apresentar opção para encerrar ou pausar o cronômetro anterior;
* salvar imediatamente no banco;
* registrar evento `timer_started`.

### Pausar

Ao pausar:

* calcular o tempo desde `last_resumed_at`;
* somar ao tempo acumulado;
* alterar status para `paused`;
* persistir imediatamente;
* registrar evento `timer_paused`.

### Continuar

Ao continuar:

* definir um novo `last_resumed_at`;
* alterar status para `running`;
* persistir imediatamente;
* registrar evento `timer_resumed`.

### Encerrar

Ao encerrar:

* calcular a duração final;
* criar um `time_entry`;
* preservar o valor/hora atual como snapshot;
* remover o estado de `active_timer`;
* registrar evento `timer_stopped`;
* atualizar a interface.

### Recuperação

Quando o aplicativo abrir e encontrar um cronômetro em execução:

* não descartar o registro;
* calcular o período transcorrido;
* mostrar uma tela ou modal de recuperação;
* permitir manter, editar ou descartar o período;
* nunca decidir silenciosamente por apagar tempo.

### Mudança do relógio do sistema

Evite depender apenas de contadores mantidos no frontend.

Persista timestamps e calcule durações no backend. Considere possíveis alterações do relógio do sistema e documente a estratégia adotada.

## 10. Monitoramento de processos

Crie um serviço Rust responsável por verificar periodicamente os processos em execução no Windows.

Requisitos:

* manter uma lista configurável de executáveis;
* detectar transição de fechado para aberto;
* detectar transição de aberto para fechado;
* evitar emitir o mesmo evento repetidamente;
* registrar os eventos no banco;
* comunicar eventos relevantes ao frontend;
* continuar funcionando quando a janela principal estiver fechada e o aplicativo estiver na bandeja;
* não bloquear a thread principal;
* permitir desligar completamente o monitoramento nas configurações.

Comportamento ao detectar um programa CAD aberto sem cronômetro ativo:

```text
AutoCAD foi aberto.

Em qual projeto você vai trabalhar?

[Selecionar projeto]
[Ignorar agora]
[Não lembrar novamente hoje]
```

O lembrete deve usar notificação nativa quando possível. Ao clicar, deve abrir o aplicativo na tela apropriada.

Não vincule automaticamente um projeto sem confirmação no MVP.

Ao detectar o fechamento do programa relacionado enquanto houver cronômetro ativo:

```text
AutoCAD foi fechado.

Deseja encerrar o registro atual?

[Encerrar]
[Manter ativo]
[Pausar]
```

Não encerre silenciosamente.

## 11. Detecção de inatividade

Implemente uma camada específica para Windows usando APIs adequadas do sistema operacional.

Requisitos:

* consultar há quanto tempo não houve entrada de teclado ou mouse;
* não capturar quais teclas foram pressionadas;
* não capturar coordenadas ou conteúdo;
* iniciar evento de inatividade ao ultrapassar o limite configurado;
* registrar quando a atividade voltar;
* não descontar tempo automaticamente no comportamento padrão;
* apresentar decisão ao usuário.

Exemplo:

```text
Você ficou 18 minutos sem atividade.

O que deseja fazer com esse período?

[Manter como trabalhado]
[Descontar 18 minutos]
[Editar período]
```

A decisão deve atualizar corretamente o registro de tempo.

Documente claramente a diferença entre:

* duração bruta;
* tempo inativo;
* duração líquida;
* duração faturável;
* duração arredondada.

## 12. Arredondamento

O banco deve preservar sempre o tempo real.

O arredondamento deve ser aplicado apenas na visualização ou no cálculo de cobrança.

Opções:

```text
desativado
5 minutos
10 minutos
15 minutos
30 minutos
```

Modos:

```text
nearest
up
down
```

Exemplo:

```text
Tempo real: 1h07
Intervalo: 15 minutos
Modo para cima: 1h15
```

Nunca substitua a duração original pelo valor arredondado.

## 13. Telas do MVP

### Dashboard

Deve mostrar:

* projeto atual;
* cronômetro;
* status;
* descrição opcional;
* tipo de atividade;
* botões iniciar, pausar, continuar e encerrar;
* valor acumulado estimado;
* total trabalhado hoje;
* projetos recentes;
* resumo da semana;
* alertas pendentes.

O cronômetro deve ser o elemento visual principal.

### Projetos

Deve permitir:

* listar;
* pesquisar;
* criar;
* editar;
* arquivar;
* concluir;
* visualizar horas acumuladas;
* visualizar valor estimado;
* acessar o histórico do projeto.

### Clientes

Pode estar integrado à tela de projetos no primeiro MVP, mas os dados devem continuar separados no banco.

### Histórico

Deve permitir:

* visualizar sessões por dia;
* filtrar por período;
* filtrar por projeto;
* filtrar por cliente;
* editar início e fim;
* editar descrição;
* alterar tipo de atividade;
* definir como faturável ou não faturável;
* adicionar uma sessão manual;
* excluir com confirmação;
* restaurar ou usar soft delete quando adequado.

### Relatórios

Deve mostrar:

* cliente;
* projeto;
* período;
* horas reais;
* horas inativas;
* horas faturáveis;
* valor/hora;
* valor total;
* separação por tipo de atividade;
* lista detalhada das sessões.

Comece com:

* visualização na interface;
* impressão pelo sistema;
* exportação CSV.

A exportação PDF pode entrar após a estrutura principal estar estável.

### Configurações

Deve permitir configurar:

* programas monitorados;
* limite de inatividade;
* intervalo de verificação;
* notificações;
* arredondamento;
* comportamento ao fechar;
* inicialização com Windows;
* moeda;
* aparência.

## 14. Reconstrução do dia

Criar uma tela ou seção chamada provisoriamente de:

**Linha do tempo detectada**

Exemplo:

```text
08:12 — AutoCAD aberto
08:17 — Cronômetro iniciado no projeto 083-22
11:46 — AutoCAD fechado
13:34 — AutoCAD aberto
16:22 — AutoCAD fechado
```

Quando existirem períodos com programa CAD aberto, mas sem sessão registrada, permitir:

```text
Transformar período em registro
```

O formulário deve vir preenchido com:

* horário inicial;
* horário final;
* duração;
* programa detectado;
* projeto a selecionar;
* descrição opcional.

A sessão criada deve ter:

```text
source = reconstructed
```

Esse recurso não precisa tomar decisões automáticas no MVP.

## 15. Bandeja do sistema

O aplicativo deve continuar ativo na bandeja quando a janela for fechada, conforme configuração.

Menu inicial da bandeja:

```text
Abrir HoraCAD
Iniciar trabalho
Pausar cronômetro
Continuar cronômetro
Encerrar cronômetro
Projeto atual: nome do projeto
Sair completamente
```

Itens devem ser habilitados ou desabilitados conforme o estado atual.

Diferenciar:

* fechar janela;
* minimizar;
* sair completamente.

Ao sair completamente com cronômetro ativo, mostrar confirmação e opções seguras.

## 16. Notificações

Criar um serviço centralizado de notificações.

Tipos iniciais:

* programa monitorado aberto;
* programa monitorado fechado;
* cronômetro ainda ativo;
* retorno após inatividade;
* recuperação de sessão;
* lembrete sem projeto selecionado.

Evite notificações repetitivas.

Implemente cooldown e controle de eventos já apresentados.

## 17. Aparência visual

Direção visual:

* aplicação profissional;
* técnica;
* minimalista;
* moderna;
* apropriada para uso diário;
* boa legibilidade;
* hierarquia visual clara;
* sem aparência genérica de dashboard feito por IA.

Evitar:

* excesso de cards;
* gradientes decorativos;
* glassmorphism;
* bordas arredondadas exageradas;
* sombras fortes;
* emojis;
* textos genéricos;
* animações sem função;
* números gigantes sem contexto;
* várias cores competindo entre si.

Utilize:

* fundo neutro;
* superfícies discretas;
* bordas sutis;
* tipografia limpa;
* espaçamento consistente;
* uma cor de destaque;
* números tabulares para tempo e dinheiro;
* estados claros de execução, pausa e encerramento.

O aplicativo deve funcionar inicialmente em modo escuro, mas prepare tokens para modo claro futuro.

Crie tokens CSS para:

* cores;
* espaçamento;
* raios;
* sombras;
* tipografia;
* estados;
* largura da navegação.

Não espalhe valores visuais arbitrários em vários componentes.

## 18. Localização

A interface inicial deve estar em português do Brasil.

Utilize:

```text
Data: DD/MM/YYYY
Hora: formato de 24 horas
Moeda: R$
Duração: 2h 35min
```

Centralize textos importantes para facilitar internacionalização futura.

Não é necessário instalar uma biblioteca completa de internacionalização no início, salvo se houver benefício claro.

## 19. Segurança do Tauri

Mantenha permissões mínimas.

Requisitos:

* definir capabilities explicitamente;
* não liberar APIs amplas sem necessidade;
* validar todas as entradas no backend;
* evitar comandos genéricos capazes de executar qualquer comando do sistema;
* não expor SQL arbitrário ao frontend;
* criar comandos específicos;
* nunca montar consultas SQL com concatenação insegura;
* documentar as permissões concedidas.

Não utilizar o plugin shell para executar comandos arbitrários como solução principal do monitoramento.

## 20. Padrões de código

### TypeScript

* habilitar modo estrito;
* evitar `any`;
* criar tipos de domínio;
* validar dados externos;
* componentes pequenos e focados;
* separar componentes visuais das regras de negócio;
* não realizar SQL diretamente dentro de componentes React.

### Rust

* tratar erros explicitamente;
* não usar `unwrap()` em caminhos normais de produção;
* retornar erros compreensíveis para o frontend;
* separar comandos Tauri dos serviços;
* utilizar structs tipadas;
* documentar código específico do Windows;
* garantir que loops de monitoramento possam ser encerrados corretamente.

### Banco

* utilizar migrations;
* criar índices para consultas frequentes;
* manter timestamps consistentes;
* preservar histórico;
* evitar exclusões irreversíveis sem confirmação;
* testar regras de apenas um cronômetro ativo.

## 21. Estado e comunicação

O banco deve ser a fonte persistente da verdade.

O frontend pode manter estado derivado para renderização, mas não deve ser a única fonte do cronômetro.

Defina eventos Tauri tipados para:

```text
timer-state-changed
monitored-app-opened
monitored-app-closed
idle-started
idle-ended
database-updated
```

Evite sincronização por polling excessivo no frontend.

## 22. Testes mínimos

Criar testes para:

* cálculo de duração;
* pausa e retomada;
* encerramento;
* recuperação de cronômetro;
* arredondamento;
* cálculo monetário;
* desconto de inatividade;
* impossibilidade de dois cronômetros ativos;
* alteração de valor/hora sem modificar registros antigos;
* validação de horários inválidos;
* sessão atravessando meia-noite;
* filtros por período.

Criar também um checklist manual para:

* abrir e fechar o AutoCAD;
* receber notificação;
* iniciar projeto;
* minimizar para bandeja;
* fechar janela;
* reabrir;
* reiniciar o aplicativo durante um cronômetro;
* voltar após inatividade;
* editar uma sessão;
* gerar relatório.

## 23. Documentação obrigatória

Antes ou durante a implementação, criar:

### CLAUDE.md

Deve conter:

* visão do projeto;
* stack;
* comandos;
* arquitetura;
* convenções;
* regras críticas;
* processo de migrations;
* padrões de testes;
* decisões que não podem ser quebradas.

### PRODUCT.md

Deve conter:

* problema;
* público;
* proposta;
* casos de uso;
* escopo do MVP;
* não objetivos;
* critérios de sucesso.

### ARCHITECTURE.md

Deve conter:

* componentes;
* responsabilidades;
* comunicação frontend/backend;
* persistência;
* monitoramento;
* recuperação;
* decisões técnicas.

### DATABASE.md

Deve conter:

* tabelas;
* campos;
* relacionamentos;
* índices;
* migrations;
* regras de integridade.

### UX-FLOWS.md

Deve documentar:

* início normal;
* troca de projeto;
* esquecimento de cronômetro;
* inatividade;
* fechamento de software CAD;
* recuperação após falha;
* correção manual;
* geração de relatório.

### ROADMAP.md

Organize as fases, dependências, critérios de conclusão e itens futuros.

## 24. Roadmap de implementação

### Fase 0 — Fundação documental

* analisar o ambiente;
* confirmar ferramentas disponíveis;
* criar documentação;
* registrar decisões;
* criar backlog;
* definir critérios de aceite.

### Fase 1 — Scaffold e interface base

* criar Tauri + React + TypeScript;
* configurar Tailwind;
* configurar lint e testes;
* criar layout;
* criar rotas;
* criar tokens;
* criar telas vazias funcionais;
* configurar bandeja inicial;
* garantir que a aplicação compile.

### Fase 2 — Banco e cadastros

* configurar SQLite;
* criar migrations;
* criar clientes;
* criar projetos;
* implementar CRUD;
* testar persistência.

### Fase 3 — Motor do cronômetro

* implementar regras;
* persistir estado ativo;
* criar recuperação;
* conectar dashboard;
* testar todas as transições.

### Fase 4 — Monitoramento do Windows

* monitorar processos;
* registrar eventos;
* emitir notificações;
* adicionar programas configuráveis;
* testar abertura e fechamento.

### Fase 5 — Inatividade

* implementar detector;
* registrar eventos;
* criar fluxo de decisão;
* aplicar desconto quando autorizado.

### Fase 6 — Histórico e relatórios

* edição manual;
* filtros;
* totais;
* arredondamento;
* CSV;
* impressão.

### Fase 7 — Reconstrução do dia

* linha do tempo;
* identificação de lacunas;
* criação de sessões reconstruídas.

### Fase 8 — Estabilização

* testes;
* correções;
* acessibilidade;
* desempenho;
* build Windows;
* instalador;
* documentação de uso.

## 25. Fora do escopo atual

Não implementar agora:

* login;
* Supabase;
* sincronização;
* painel web;
* aplicativo mobile;
* cobrança de assinatura;
* emissão de nota fiscal;
* integração direta com arquivos DWG;
* plugin do AutoCAD;
* IA;
* gestão de equipes;
* capturas de tela;
* sistema completo de finanças;
* integração com calendários;
* leitura automática do nome do desenho.

Esses itens podem constar apenas no roadmap futuro.

## 26. Critérios de aceite do MVP

O MVP estará funcional quando:

1. O aplicativo instalar e abrir no Windows.
2. Os dados forem armazenados localmente.
3. For possível criar cliente e projeto.
4. For possível definir valor/hora.
5. Existir apenas um cronômetro ativo.
6. Pausa, retomada e encerramento funcionarem.
7. Um cronômetro sobreviver ao fechamento e reabertura.
8. O AutoCAD aberto puder gerar um lembrete.
9. O fechamento do AutoCAD puder gerar um lembrete.
10. A inatividade puder ser detectada.
11. O usuário puder decidir o tratamento da inatividade.
12. Registros puderem ser corrigidos manualmente.
13. O histórico puder ser filtrado.
14. O sistema calcular horas e valores corretamente.
15. Um relatório puder ser visualizado e exportado em CSV.
16. A aplicação funcionar na bandeja.
17. Nenhum dado depender de conexão com internet.

## 27. Forma de trabalhar

Não tente implementar todo o produto em uma única alteração extensa.

Trabalhe em fases pequenas e verificáveis.

Para cada fase:

1. analise os arquivos existentes;
2. declare internamente o escopo;
3. implemente;
4. execute formatação;
5. execute lint;
6. execute testes;
7. execute build;
8. corrija erros;
9. atualize a documentação;
10. apresente um resumo objetivo.

Não deixe arquivos de exemplo, mocks abandonados ou componentes sem função.

Não esconda erros com comentários, desativação de lint ou tipos genéricos.

Quando houver uma decisão não especificada:

* escolha a alternativa mais simples;
* preserve possibilidade de evolução;
* documente a decisão;
* não interrompa o trabalho por questões pequenas.

Só peça esclarecimento quando existir um bloqueio real que possa causar perda de dados, incompatibilidade estrutural ou mudança significativa de escopo.

## 28. Tarefa a executar agora

Nesta primeira execução:

1. Inspecione a pasta atual e o ambiente.
2. Verifique se já existe algum projeto.
3. Caso esteja vazia, crie o projeto Tauri 2 com React, TypeScript e Vite.
4. Configure a base do frontend.
5. Configure Tailwind.
6. Configure lint, formatação e testes.
7. Crie toda a documentação inicial.
8. Crie o `CLAUDE.md`.
9. Crie a estrutura de diretórios.
10. Implemente os tokens visuais.
11. Implemente o layout principal com navegação.
12. Crie as rotas:

    * Dashboard;
    * Projetos;
    * Histórico;
    * Relatórios;
    * Configurações.
13. Crie um dashboard inicial visualmente funcional usando dados temporários tipados.
14. Configure a bandeja inicial do Tauri.
15. Prepare a camada de banco e a pasta de migrations, mas não improvise tabelas sem migrations.
16. Execute os comandos de validação.
17. Corrija todos os erros encontrados.
18. Atualize o README com comandos para desenvolvimento e build.
19. Não implemente ainda o monitoramento completo de processos ou inatividade, salvo interfaces e contratos necessários para preparar a arquitetura.
20. Ao terminar, apresente:

    * arquivos criados;
    * decisões tomadas;
    * comandos executados;
    * resultado dos testes;
    * resultado do build;
    * limitações atuais;
    * próxima fase recomendada.

O resultado desta primeira execução deve ser uma fundação limpa, compilável, documentada e pronta para receber o banco e o motor de cronômetro.
