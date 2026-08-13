# M/OS — Behavioral Wireframes v0.1a

## 1. Status

**Status:** aprovado para implementação da fundação v0.1a

**Data:** 2026-08-13

**Natureza:** wireframes comportamentais; não são especificação visual pixel-perfect

## 2. Constraint principal

O fluxo deve preservar um pensamento com uma decisão, permitir encontrá-lo depois e nunca exigir classificação no momento da captura.

```text
capturar -> confirmar commit -> continuar
                         -> Inbox quando for conveniente
                         -> Search quando for necessário
```

Projects, Tasks, Kanban, Workspaces, cloud e Hermes não aparecem nem como telas vazias.

## 3. Shell permanente

```text
┌──────────────┬──────────────────────────────────────────────────────┐
│ M/OS         │ [Search Ctrl+K]                    [Quick Capture]   │
│              ├──────────────────────────────────────────────────────┤
│ Home         │                                                      │
│ Inbox     12 │                   active surface                     │
│              │                                                      │
│              │                                                      │
│              │                                                      │
│              │                                                      │
│ Settings     │                                                      │
└──────────────┴──────────────────────────────────────────────────────┘
```

- sidebar estável com `208px` no viewport de referência;
- Home e Inbox são destinos; Search e Quick Capture são comandos;
- Archive, Trash, Backup e Restore ficam em Settings ou em ações contextuais;
- o contador da Inbox some quando for zero;
- seleção, scroll e query são preservados ao voltar para uma superfície.

## 4. Home

### 4.1 Estado com conteúdo

```text
Home

What's on your mind?                                      [Capturar]
─────────────────────────────────────────────────────────────────────

Recentes                                             [Abrir Inbox 12]
Revisar o modelo de backup                            agora
Ideia para simplificar a navegação                    18 min
Pesquisar biblioteca de SQLite                       ontem
```

Comportamento:

1. foco inicial no campo quando a janela principal é aberta por intenção de captura;
2. `Enter` salva; `Shift+Enter` cria nova linha;
3. sucesso aparece somente depois do commit: `Salvo na Inbox`;
4. o campo é limpo somente após sucesso;
5. erro preserva o texto e informa que nada foi salvo;
6. Recentes mostra no máximo oito Captures ativas, sem cards.

### 4.2 Estado vazio

```text
What's on your mind?                                      [Capturar]

Suas capturas recentes aparecerão aqui.
```

Nenhum onboarding, ilustração ou métricas.

## 5. Quick Capture

```text
┌─────────────────────────────────────────────────────────────────┐
│ What's on your mind?                              [Capturar]    │
│ Pronto para salvar localmente                                   │
└─────────────────────────────────────────────────────────────────┘
```

Estados:

| Estado | Campo | Ação | Feedback |
|---|---|---|---|
| inicial | focado, vazio | desabilitada | `Pronto para salvar localmente` |
| digitando | preserva texto | habilitada | feedback anterior discreto |
| salvando | preserva texto | desabilitada | `Salvando localmente...` |
| sucesso | limpa depois do commit | desabilitada | `Salvo na Inbox` e fecha em seguida |
| erro | preserva texto | `Tentar novamente` | motivo seguro + `Nada foi salvo` |

Teclado:

- `Enter`: confirmar;
- `Shift+Enter`: nova linha;
- `Esc`: esconder sem apagar o texto não salvo durante a sessão;
- atalho global configurável abre e foca o campo;
- reabrir antes de salvar recupera o draft da sessão.

## 6. Inbox

### 6.1 List-detail

```text
Inbox · 12
──────────────────────────────┬──────────────────────────────────────
> Revisar o modelo de backup  │ Revisar o modelo de backup
  agora · Quick Capture       │
                              │ Capturado agora
  Ideia para navegação        │ Origem: Quick Capture
  18 min · Home               │
                              │ [Marcar processada] [Arquivar] [...]
  Pesquisar SQLite            │
  ontem · Quick Capture       │
──────────────────────────────┴──────────────────────────────────────
```

- a lista contém somente Captures `processing_state=inbox` e `lifecycle_state=active`;
- primeira linha é o conteúdo, segunda linha é origem e tempo;
- detalhe não repete metadata interna nem IDs;
- setas movem seleção; `Enter` move foco ao detalhe; `Esc` volta à lista;
- `Delete` não apaga diretamente: abre menu com Archive e Trash;
- ao processar, arquivar ou enviar à Trash, selecionar o próximo item sem deslocamento abrupto.

### 6.2 Ações reversíveis

```text
Capture arquivada.                                             [Desfazer]
```

- toast não bloqueia;
- Undo restaura `processing_state` anterior e `lifecycle_state=active`;
- apenas uma ação recente fica disponível para Undo na fundação inicial;
- fechar a aplicação não promete preservar Undo.

### 6.3 Estado vazio

```text
Inbox

Nada aguardando decisão.
```

Não há CTA porque capturar já está permanentemente acessível.

## 7. Search

### 7.1 Command surface

```text
┌─────────────────────────────────────────────────────────────────┐
│ Buscar no M/OS...                                               │
├─────────────────────────────────────────────────────────────────┤
│ Capture  Revisar o modelo de backup                  agora      │
│ Capture  Backup local antes de migrations            ontem      │
└─────────────────────────────────────────────────────────────────┘
```

- `Ctrl+K` abre de qualquer superfície;
- busca inicia após texto não vazio, sem botão obrigatório;
- debounce máximo de `80ms`, cancelável;
- resultados são lista única; `Capture` é label discreto, não badge colorida;
- `Enter` abre a Capture e fecha Search;
- `Esc` fecha e devolve foco ao elemento anterior;
- itens arquivados são omitidos por padrão; toggle `Incluir arquivados` aparece somente com query ativa;
- Trash nunca aparece na busca normal.

### 7.2 Estados

```text
vazio inicial:   Digite para buscar.
sem resultado:   Nenhuma captura encontrada.
erro de índice:  A busca local precisa ser reconstruída. [Reconstruir]
rebuild:         Reconstruindo busca local...
```

Search não inventa sugestões, categorias ou semantic search.

## 8. Archive e Trash

Ambos vivem em Settings > Data, como listas operacionais simples.

### Archive

- mostra Captures arquivadas;
- `Restaurar` retorna a `active` preservando `processing_state`;
- conteúdo continua pesquisável somente quando `Incluir arquivados` estiver ativo.

### Trash

- mostra Captures em `trashed`;
- `Restaurar` retorna a `active` preservando `processing_state`;
- exclusão definitiva fica fora da v0.1a inicial até política de retenção ser aprovada.

## 9. Backup e Restore

### Backup manual

```text
Backup local

Cria uma cópia consistente do seu dataset em texto potencialmente sensível.
[Criar backup...]
```

Resultado:

- seletor nativo escolhe destino `.mos-backup`;
- sucesso mostra caminho e horário;
- cancelamento não é erro;
- falha não altera o dataset.

### Restore

```text
Restaurar backup

O dataset atual será substituído depois da criação de um safety backup.
[Escolher backup...]
```

Fluxo:

1. selecionar arquivo;
2. validar manifest, schema e checksum sem alterar o banco;
3. apresentar origem, data e tamanho;
4. confirmar substituição;
5. criar safety backup;
6. restaurar e executar integrity check;
7. reiniciar queries e mostrar resultado.

Restore é a única ação de v0.1a que exige confirmação modal, pois substitui o dataset inteiro.

## 10. Estados globais

### Inicialização

```text
Abrindo dados locais...
```

Só aparece quando migration ou integrity check ultrapassar `150ms`.

### Banco indisponível

```text
M/OS não abriu o banco local com segurança.
Nenhuma alteração foi feita.
[Abrir pasta de recuperação] [Tentar novamente]
```

Writes ficam bloqueados. A aplicação não cria um banco vazio sobre um arquivo problemático.

### Atalho em conflito

Settings mostra o atalho atual, erro específico e permite registrar outro. A falha não impede captura pela janela principal.

## 11. Revisão contra UX Principles

| Constraint | Aplicação no wireframe |
|---|---|
| Capture before classification | todo campo salva apenas conteúdo e origem |
| Friction proportional to intent | Capture usa uma ação; Restore exige confirmação |
| Progressive disclosure | listas mostram trecho, origem e tempo; detalhe contém ações |
| Information before containers | conteúdo em listas e divisores, sem dashboard de cards |
| One screen, one intention | Home captura; Inbox decide; Search encontra |
| Undo before confirmation overload | Archive, Trash e processar executam e oferecem Undo |
| Immediate feedback | feedback distingue commit, erro e ausência de persistência |
| Keyboard first | fluxo principal completo por teclado |
| Preserve context | seleção, query e scroll são preservados |
| Quiet UI | sem métricas, ilustrações, gradientes ou decoração sem função |

## 12. Gate encerrado

Estes wireframes encerram o gate comportamental da v0.1a. Ajustes visuais podem ocorrer durante implementação, mas não podem mudar intenções, estados ou invariantes sem atualizar este documento.
