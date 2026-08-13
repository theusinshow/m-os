# M/OS — Independent Architecture Review

## 1. Contexto

**Data da revisão:** 2026-08-13

A revisão foi executada por um agente independente, em modo read-only, depois da primeira proposta de:

- `ARCHITECTURE.md`;
- `CORE-FOUNDATION.md`;
- `DECISIONS.md`;
- `V0.1-SCOPE.md`.

O revisor também leu integralmente os documentos originais do produto.

Objetivo da revisão:

- tentar invalidar a arquitetura;
- identificar decisões prematuras;
- encontrar violações dos UX Principles;
- avaliar riscos de desktop, iOS, local/cloud, persistência, sync e manutenção;
- impedir início prematuro de código de produto.

## 2. Veredito original

> Não aprovar ainda o início do código de produto.

O revisor considerou a direção geral sólida, mas identificou dois contratos fundamentais incompletos:

1. estado de Capture misturava processamento e retenção;
2. a promessa de durabilidade depois de `Saved` não possuía fault model testável.

O spike descartável de Tauri foi considerado aceitável somente depois de esclarecer lifecycle do processo e critérios de avaliação.

## 3. Achados e disposições

| Severidade | Achado | Disposição do arquiteto | Estado |
|---|---|---|---|
| Bloqueador | Capture misturava `inbox/processed` com `archived/trashed` | separar `processing_state` e `lifecycle_state`, com transições e Restore definidos | Tratado na proposta |
| Bloqueador | `Saved` não definia durabilidade | WAL + `synchronous=FULL`, fault model, erros que não podem confirmar sucesso e fault injection | Tratado na proposta |
| Alta | proveniência duplicaria resultados de Search | agrupar origem e derivado, mantendo Capture subordinada e pesquisável | Tratado na proposta |
| Alta | v0.1 grande demais para primeiro uso | dividir em v0.1a, v0.1b e v0.1c; dogfooding começa em Capture/Inbox/Search/backup | Tratado na proposta |
| Alta | disponibilidade de Quick Capture ambígua | close-to-tray, `Quit`, conflito de shortcut e ausência de startup automático explicitados | Tratado na proposta |
| Alta | sync parcialmente antecipado | remover revisão distribuída e adiar change journal, tombstones, cursor e conflito para ADR única | Tratado na proposta |
| Alta | Tauri demonstrava viabilidade, não superioridade | scorecard ponderado, gates obrigatórios e fallback WinUI 3 | Tratado na proposta |
| Alta | iOS online poderia produzir Capture volátil | exigir ack remoto ou handoff durável em App Group/shared container | Tratado como constraint futura |
| Alta | segurança aceita sem threat model | ADR-013 voltou para `Proposed`; threat model e dependências foram explicitados | Pendente de validação na máquina |
| Média | lifecycle de Task incompleto | remover `Planned`, definir estados, `completed_at`, Archive/Trash e relação com Project | Tratado na proposta |
| Média | backup e export vagos | definir snapshots, retenção, `.mos-backup`, checksums, restore total e export JSON separado | Tratado na proposta |
| Média | gate omitia design foundations | adicionar aprovação de fluxos, navegação, estados e acessibilidade antes da UI de produto | Tratado na proposta |

## 4. Decisões alteradas pela revisão

### Capture

Antes:

```text
state = inbox | processed | archived | trashed
```

Depois do tratamento, antes do spike:

```text
processing_state = inbox | processed
lifecycle_state  = active | archived | trashed
```

### Durabilidade

Antes:

```text
WAL a avaliar
```

Depois:

```text
journal_mode = WAL
synchronous = FULL
feedback Saved somente depois de COMMIT
fault model explícito
```

### Primeira entrega utilizável

Antes:

```text
Capture + Inbox + Projects + Tasks + Kanban + Search + hardening
```

Depois:

```text
v0.1a: Capture + Inbox + Search de Captures + backup/restore
v0.1b: Tasks + Projects + proveniência + Search agrupada
v0.1c: Kanban + hardening + release v0.1
```

### Stack desktop

Antes:

```text
Tauri recomendado por comparação qualitativa
```

Depois:

```text
Tauri permanece Proposed naquele checkpoint
scorecard mínimo 75/100
gates obrigatórios de lifecycle, acessibilidade, durabilidade e packaging
WinUI 3 como fallback explícito
```

O spike posterior aprovou Tauri com score 81/100 e todos os gates obrigatórios em `TECHNICAL-SPIKE-DESKTOP-SHELL.md`.

## 5. Pendências depois do tratamento

O tratamento documental não equivale a aprovação automática.

Continuam pendentes:

1. revisor independente verificar se os bloqueadores foram realmente encerrados;
2. usuário aprovar o corte v0.1a/v0.1b/v0.1c;
3. design foundations serem elaboradas e revisadas;
4. spike comparar Tauri contra o scorecard;
5. confirmar baseline de segurança na máquina Windows alvo;
6. executar testes reais de durabilidade, backup e restore;
7. passar ADRs técnicas relevantes de `Proposed` para `Accepted`.

## 6. Gate atual

- documentação de produto: existente;
- proposta arquitetural: existente e revisada;
- revisão independente inicial: concluída;
- tratamento dos achados: documentado;
- reverificação independente: concluída, sem bloqueador remanescente;
- escopo v0.1a/v0.1b/v0.1c: aprovado pelo usuário em 2026-08-13;
- arquitetura técnica: aprovada depois do spike técnico;
- design foundations: concluídas e aprovadas;
- spike técnico: concluído com score 81/100 e todos os gates obrigatórios aprovados;
- código de produto: autorizado somente dentro da fundação e do corte v0.1a aprovado.

## 7. Reverificação

O mesmo revisor realizou uma segunda leitura read-only depois do primeiro tratamento.

Veredito:

- nenhum bloqueador original permaneceu;
- nenhum novo achado crítico ou alto foi encontrado;
- documentação considerada adequada para avançar ao spike e às design foundations;
- naquele checkpoint, código de produto permanecia não autorizado; os gates posteriores foram concluídos em 2026-08-13.

A reverificação pediu cinco ajustes localizados:

| Ajuste | Tratamento |
|---|---|
| destino de Task reaberta | reabre em `backlog` e limpa `completed_at` |
| migrations apareciam em dois marcos | v0.1a cria suporte; v0.1c testa upgrades e recovery entre versões |
| múltiplos derivados ambíguos em Search | resultado agrupado com destinos separados; UI v0.1b limita uma Task derivada por Capture |
| consistência FTS opcional | entidade e FTS passam a ser atualizadas na mesma transação na v0.1 |
| scorecard sem método | escala 0–5, gate mínimo 4, fórmula e evidências obrigatórias adicionadas |

Depois desses ajustes, não há pendência documental crítica conhecida. Os gates posteriores foram cumpridos e registrados em `DESIGN-FOUNDATIONS.md`, `TECHNICAL-SPIKE-DESKTOP-SHELL.md` e `DECISIONS.md`.

Uma auditoria final read-only confirmou que não restou contradição crítica ou alta conhecida nos ajustes da segunda rodada.

## 8. Validação do ambiente alvo

O ambiente foi identificado como Windows 11 Pro x64.

A consulta automatizada do status BitLocker não pôde ser concluída por falta de acesso administrativo. Portanto:

- não se afirma que BitLocker está habilitado;
- não se afirma que BitLocker está desabilitado;
- ADR-013 permanece `Proposed`;
- a proteção do volume deve ser verificada administrativamente antes de utilizar dados pessoais reais ou o risco precisa ser aceito explicitamente.
