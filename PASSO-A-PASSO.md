# M/OS — Passo a passo resumido para iniciar no ORCA

## 1. Extraia a pasta

Sugestão:

```text
C:\Dev\pessoal\m-os
```

Estrutura:

```text
m-os/
├── docs/
│   ├── VISION.md
│   ├── PRODUCT.md
│   ├── CORE.md
│   ├── UX-PRINCIPLES.md
│   ├── ROADMAP.md
│   └── IDEAS.md
├── AGENTS.md
└── PASSO-A-PASSO.md
```

## 2. Inicialize o Git

No PowerShell:

```powershell
cd C:\Dev\pessoal\m-os
git init
git add .
git commit -m "docs: initial M/OS product vision"
```

## 3. Abra no ORCA

Adicione a pasta `m-os` como repositório.

Crie um worktree para a primeira fase, por exemplo:

```text
architecture-foundation
```

## 4. Abra um agente Codex

No agente Codex do ORCA, selecione o modelo que você pretende usar para arquitetura. Se sua instalação disponibilizar GPT-5.6 Sol, selecione-o pelo seletor de modelo do Codex/ORCA e confirme o modelo ativo antes de continuar.

Use reasoning alto para essa etapa arquitetural, caso essa opção esteja disponível.

## 5. Não implemente ainda

A primeira missão é arquitetura e revisão do produto.

O agente deve:

1. ler `AGENTS.md` e todos os documentos de `/docs`;
2. reconstruir a visão do produto;
3. apontar ambiguidades, contradições e lacunas;
4. separar requisitos, decisões de produto, hipóteses e ideias futuras;
5. avaliar o Core conceitual;
6. tratar `UX-PRINCIPLES.md` como constraint;
7. levantar riscos de overengineering;
8. comparar alternativas técnicas;
9. propor arquitetura desktop/mobile/local/cloud;
10. documentar as decisões antes de escrever código de produto.

## 6. Prompt inicial sugerido

```text
Você é o arquiteto principal do projeto M/OS.

Antes de fazer qualquer alteração, leia integralmente:

- AGENTS.md
- docs/VISION.md
- docs/PRODUCT.md
- docs/CORE.md
- docs/UX-PRINCIPLES.md
- docs/ROADMAP.md
- docs/IDEAS.md

Esses documentos são resultado de uma longa fase de descoberta de produto e representam a intenção do projeto.

Sua primeira tarefa NÃO é implementar o M/OS.

Quero que você trate este repositório como um produto novo entrando na fase de arquitetura.

Faça uma análise crítica e profunda do material existente.

Primeiro:

1. Reconstrua mentalmente o produto a partir dos documentos.
2. Identifique contradições, ambiguidades, sobreposições ou conceitos ainda mal definidos.
3. Diferencie claramente:
   - requisitos fundamentais;
   - decisões de produto;
   - hipóteses;
   - ideias futuras;
   - decisões técnicas ainda abertas.
4. Avalie se o CORE conceitual é suficiente para sustentar a visão de longo prazo.
5. Avalie com atenção especial os UX Principles. Eles devem ser tratados como constraints do produto, não como sugestões.
6. Identifique riscos de overengineering e de construção prematura.
7. Liste as decisões arquiteturais que precisam ser tomadas antes de qualquer implementação.

Depois dessa análise, proponha a arquitetura técnica mais adequada para o M/OS considerando especialmente:

- desktop-first;
- experiência de programa real no Windows;
- mobile companion;
- captura extremamente rápida;
- possibilidade de funcionamento offline;
- sincronização entre dispositivos;
- Universal Capture;
- busca global;
- Workspaces;
- Projects;
- Tasks;
- Resources;
- App Registry;
- integrações futuras;
- GitHub;
- Hermes;
- voz;
- softwares independentes como M-Finance e ChronoCAD;
- segurança e privacidade;
- possibilidade de evolução sem criar um monólito impossível de manter.

Não escolha tecnologias por popularidade. Compare alternativas e justifique trade-offs.

Não implemente código de produto ainda.

Você pode criar documentação de arquitetura dentro de /docs quando sua análise estiver madura.

Quero que o resultado desta fase seja uma fundação técnica profissional sobre a qual possamos decidir conscientemente como construir o M/OS.

Antes de criar arquivos, me apresente:
- sua leitura do produto;
- os principais riscos;
- as questões arquiteturais;
- sua estratégia proposta para esta fase.

Só depois avance para documentação.
```

## 7. Faça uma revisão independente

Depois da proposta arquitetural, crie outro worktree/agente, por exemplo:

```text
architecture-review
```

Prompt resumido:

```text
Leia toda a documentação do produto e a arquitetura proposta.
Tente destruir essa arquitetura.

Procure:
- decisões prematuras;
- overengineering;
- violações dos UX Principles;
- problemas de desktop/mobile;
- riscos de offline/sync;
- acoplamento desnecessário;
- problemas futuros nas integrações;
- decisões que dificultem manutenção.

Não implemente.
Produza um parecer crítico e priorizado.
```

Depois, entregue o parecer ao arquiteto principal para revisão.

## 8. Só depois comece a implementação

Fluxo recomendado:

```text
Documentação de produto
        ↓
Análise arquitetural
        ↓
Revisão independente
        ↓
Arquitetura aprovada
        ↓
Design foundations
        ↓
Technical foundation
        ↓
M/OS v0.1
```

O objetivo é evitar que a visão ampla do M/OS vire um projeto gigantesco antes de existir uma fundação sólida.
