# Sync automático e a faixa da Home — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** o M/OS do desktop passa a sincronizar sozinho — na abertura, ao voltar ao primeiro plano, depois de mutação e a cada 15 min — e a Home ganha uma faixa que só aparece quando tem notícia ou quando algo está errado.

**Architecture:** o laço mora numa tarefa `tokio` criada no `setup` do Tauri, que espera o app terminar de abrir antes da primeira rodada. Ela acorda por `Notify` (primeiro plano, mutação com *debounce*) e por intervalo de segurança. A tela não decide nada: uma função pura, `syncFaixa.ts`, resolve qual dos seis estados desenhar a partir do `SyncStatus`, e os componentes só desenham.

**Tech Stack:** Rust + Tauri v2 + tokio (`rt`, `sync`, `macros` já no `Cargo.toml`), React + TypeScript, vitest para as funções puras.

**Spec:** `docs/superpowers/specs/2026-08-28-sync-automatico-e-settings-design.md`

## Global Constraints

- **`TMP`/`TEMP` antes de todo `cargo`.** O sandbox nega ao linker mingw o acesso a `C:\WINDOWS\TEMP`. Em **cada** chamada de Bash, na mesma invocação:
  ```bash
  export TMP="C:/WINDOWS/TEMP/claude/C--Dev-pessoal-m-os/46c15ce2-16c4-4550-9306-5f16e86dd029/scratchpad/rstmp"; export TEMP="$TMP"; mkdir -p "$TMP"
  ```
- **`cargo test -p mos-desktop` NÃO roda nesta máquina.** Compila e o binário morre com `0xC0000139` (STATUS_ENTRYPOINT_NOT_FOUND), por ambiente e não por código. Para o crate desktop, a prova local é `cargo test -p mos-desktop --lib --no-run`; a execução fica para o CI. **Escreva a lógica testável em `crates/*`**, onde `cargo test` roda normal.
- **Nunca rodar `Stop-Process` em `mos-desktop`.** Mata a sessão real do dono do produto.
- **Toda mensagem de interface em português**, no tom do resto do app: diz o que aconteceu, não o que a função retornou.
- **Comentário explica POR QUÊ, não o quê.** É a convenção deste repositório em todos os arquivos que este plano toca.
- **`EntityKind` é texto, nunca enum fechado** (`SYNC.md` §9). Um tipo desconhecido conta e aparece; nunca quebra a desserialização.
- Commits em português, no formato `tipo(escopo): frase minúscula`, direto na `master` (preferência registrada do dono).

---

## Estrutura de arquivos

| Arquivo | Responsabilidade | Ação |
| --- | --- | --- |
| `crates/mos-sync/src/engine.rs` | `Rodada` ganha `recebidas_por_tipo`; o motor conta ao puxar | Modificar |
| `crates/mos-storage-sqlite/src/sync_projecao.rs` | funde o mapa entre as passadas de `sincronizar_agora` | Modificar |
| `apps/desktop/src-tauri/src/sync.rs` | o daemon, o estado em memória, os comandos novos | Modificar |
| `apps/desktop/src-tauri/src/lib.rs` | `UserSettings.sync_ultimo_resumo_em`; criar o daemon no `setup`; acordar em `reveal_window` | Modificar |
| `apps/desktop/src/syncFaixa.ts` | **função pura**: `SyncStatus` → qual estado e que frase | Criar |
| `apps/desktop/src/syncFaixa.test.ts` | os testes dela | Criar |
| `apps/desktop/src/SyncFaixa.tsx` | o componente que desenha o resultado | Criar |
| `apps/desktop/src/types.ts` | `SyncStatus` e `SyncRound` estendidos | Modificar |
| `apps/desktop/src/api.ts` | `syncDismissSummary`, `syncAppReady` | Modificar |
| `apps/desktop/src/App.tsx` | montar a faixa na `HomePage`; chamar `syncAppReady` no boot | Modificar |
| `apps/desktop/src/App.css` | o estilo da faixa | Modificar |
| `docs/SYNC.md` | §8 e §14 deixam de descrever um plano | Modificar |

---

## Task 0: Ligar a sincronização neste PC

Sem endereço, sem segredo e sem túnel, não há o que automatizar — e a única faixa
que dá para fotografar é a de "desligado". **Este passo não tem código e não tem
commit**, mas bloqueia a verificação de todos os outros.

**Files:** nenhum no repositório.

- [ ] **Step 1: Confirmar que de fato está desligado**

```bash
cat "$APPDATA/com.codedbym.mos/settings.json"
```

Esperado: um JSON **sem** a chave `syncEndpoint`.

```powershell
Get-ScheduledTask | Where-Object { $_.TaskName -match 'M-OS' } | Select-Object TaskName, State
```

Esperado: só `M-OS Hermes Tunnel`. Se `M-OS Sync Tunnel` já existir, pule aos passos 4 e 5.

- [ ] **Step 2: Achar uma chave SSH sem passphrase**

A tarefa roda no logon, sem ninguém olhando — uma chave com passphrase trava para sempre. O `sync-tunnel.ps1` procura, nesta ordem, `hermes_work`, `id_ed25519`, `hermes_home`, e pula qualquer uma cujo cabeçalho contenha `ENCRYPTED`.

```powershell
foreach ($n in @('hermes_work','id_ed25519','hermes_home')) {
  $p = Join-Path $env:USERPROFILE ".ssh\$n"
  if (Test-Path $p) {
    $enc = ((Get-Content $p -TotalCount 3) -join "`n") -match 'ENCRYPTED'
    Write-Output "$n : $(if ($enc) { 'com passphrase - nao serve' } else { 'SERVE' })"
  } else { Write-Output "$n : nao existe" }
}
```

Se nenhuma servir, **pare e pergunte ao dono do produto** — gerar chave nova e autorizá-la na VPS é decisão dele, não sua.

- [ ] **Step 3: Instalar a tarefa do túnel**

```powershell
& "C:\Dev\pessoal\m-os\scripts\install-sync-tunnel.ps1"
```

Depois, provar que a porta local responde:

```powershell
Test-NetConnection -ComputerName 127.0.0.1 -Port 9120 -InformationLevel Quiet
```

Esperado: `True`. `False` significa que o túnel não subiu — leia `deploy/README.md` §5 antes de seguir.

- [ ] **Step 4: Pôr endereço e segredo pela tela**

Abra o M/OS, vá em Settings → Sincronização, ponha `http://127.0.0.1:9120` no endereço e o segredo do hub no campo de segredo, e clique em Salvar.

O segredo **não sai daqui pelo terminal**: ele vai para o Credential Manager e nunca volta para a tela. Se não souber qual é, ele está na VPS em `/etc/mos-sync.env` (`deploy/README.md` §3).

- [ ] **Step 5: Uma rodada manual, e anotar o número**

Clique em "Sincronizar agora". Anote quantas subiram — é o tamanho da fila que este PC acumulou desde que a emissão foi ligada, e é o número que a faixa de "pendente" mostraria.

**Não commite nada nesta task.**

---

## Task 1: A rodada conta o que chegou, por tipo

**Files:**
- Modify: `crates/mos-sync/src/engine.rs` (struct `Rodada` em `:60`, laço do *pull* em `:141-175`)
- Modify: `crates/mos-storage-sqlite/src/sync_projecao.rs` (`sincronizar_agora`, o acúmulo entre passadas em `:765-769`)
- Test: `crates/mos-storage-sqlite/tests/sync_two_devices.rs` — **é aqui que mora o único dublê de `Transport` do repositório** (`HubLocal`, `:43`), junto de `Dispositivo` (`:99`) com `mudar` e `sincronizar`, e do `task()` (`:201`). O `crates/mos-sync/src/tests.rs` **não** tem transporte falso; não tente escrever o teste lá.

**Interfaces:**
- Produces: `mos_sync::Rodada.recebidas_por_tipo: BTreeMap<String, usize>` — **quantas ENTIDADES de cada tipo mudaram nesta rodada**, não quantas operações. As tasks 3 e 4 dependem deste nome e deste tipo.

**A decisão que o código precisa registrar:** `rodada.recebidas` conta **operações** (`lote.ops.len()`); o mapa conta **entidades**. Os dois números não batem de propósito — três edições da mesma Task são 3 operações e 1 task, e a faixa que dissesse "3 tasks chegaram" estaria mentindo para quem só mexeu numa. A faixa usa o mapa; a linha do Settings continua usando `recebidas`.

- [ ] **Step 1: Escrever o teste que falha**

No fim de `crates/mos-storage-sqlite/tests/sync_two_devices.rs`, usando o
harness que já está lá. Precisa de mais dois `EntityRef` além do `task()` que
existe em `:201` — acrescente-os ao lado dele:

```rust
fn capture() -> EntityRef {
    EntityRef::new("capture", Uuid::from_u128(4243))
}

fn tipo_do_futuro() -> EntityRef {
    EntityRef::new("tipo_do_futuro", Uuid::from_u128(4244))
}
```

E os testes:

```rust
#[test]
fn a_rodada_conta_entidades_por_tipo_e_nao_operacoes() {
    // Duas mudancas na MESMA task, e uma capture. A faixa da Home que dissesse
    // "2 tasks" para quem so mexeu numa estaria mentindo com numero — e e por
    // isso que o mapa conta ENTIDADE, enquanto `recebidas` conta operacao.
    let hub = HubLocal::default();
    let mut pc = Dispositivo::novo("PC", "windows");
    let mut iphone = Dispositivo::novo("iPhone", "ios");

    pc.mudar(&task(), &[("title", json!("Revisar memorial"))]);
    pc.mudar(&task(), &[("state", json!("doing"))]);
    pc.mudar(&capture(), &[("content", json!("ideia na rua"))]);
    pc.sincronizar(&hub);

    let rodada = iphone.sincronizar(&hub);

    assert_eq!(rodada.recebidas, 3, "`recebidas` conta OPERACOES");
    assert_eq!(
        rodada.recebidas_por_tipo.get("task"),
        Some(&1),
        "duas mudancas na mesma task sao UMA task"
    );
    assert_eq!(rodada.recebidas_por_tipo.get("capture"), Some(&1));
}

#[test]
fn um_tipo_desconhecido_conta_em_vez_de_sumir() {
    // `EntityKind` e texto e nao enum fechado (SYNC.md §9), justamente para um
    // cliente antigo guardar e reenviar um tipo que ele nao conhece. Sumir com
    // ele faria a faixa dizer que nada chegou quando algo chegou.
    let hub = HubLocal::default();
    let mut pc = Dispositivo::novo("PC", "windows");
    let mut iphone = Dispositivo::novo("iPhone", "ios");

    pc.mudar(&tipo_do_futuro(), &[("qualquer", json!(1))]);
    pc.sincronizar(&hub);

    let rodada = iphone.sincronizar(&hub);

    assert_eq!(rodada.recebidas_por_tipo.get("tipo_do_futuro"), Some(&1));
}
```

`Dispositivo::sincronizar` já devolve `mos_sync::Rodada` (`:154`), então não há
encanamento novo — só a asserção sobre o campo que ainda não existe.

- [ ] **Step 2: Rodar e ver falhar**

```bash
export TMP="C:/WINDOWS/TEMP/claude/C--Dev-pessoal-m-os/46c15ce2-16c4-4550-9306-5f16e86dd029/scratchpad/rstmp"; export TEMP="$TMP"; mkdir -p "$TMP"
cd /c/Dev/pessoal/m-os && cargo test -p mos-storage-sqlite --test sync_two_devices 2>&1 | tail -20
```

Esperado: erro de compilação — `no field 'recebidas_por_tipo' on type 'Rodada'`.

- [ ] **Step 3: O campo, e quem o preenche**

Em `crates/mos-sync/src/engine.rs`, na struct `Rodada` (depois de `pub tem_mais: bool`):

```rust
    /// Quantas ENTIDADES de cada tipo mudaram nesta rodada.
    ///
    /// Nao e `recebidas` reparticionado: aquele conta OPERACOES, e tres edicoes
    /// da mesma Task sao tres operacoes e uma task. A faixa da Home usa este
    /// mapa justamente porque "3 tasks chegaram" precisa significar tres tasks.
    ///
    /// `String` e nao enum: `EntityKind` e texto (§9), e um tipo que este
    /// cliente ainda nao conhece precisa CONTAR em vez de sumir.
    #[serde(default)]
    pub recebidas_por_tipo: BTreeMap<String, usize>,
```

No laço do *pull*, logo depois de `for ((kind, id), ops) in por_entidade {`, conte a entidade **antes** de qualquer coisa que possa falhar — o `break` do erro está mais abaixo, e o que já chegou permanece chegado:

```rust
                for ((kind, id), ops) in por_entidade {
                    *rodada
                        .recebidas_por_tipo
                        .entry(kind.clone())
                        .or_insert(0) += 1;
                    let base = projecao.estado_de(&ops[0]);
```

`BTreeMap` já está importado no topo do arquivo (o `por_entidade` usa).

- [ ] **Step 4: Rodar e ver passar**

```bash
export TMP="C:/WINDOWS/TEMP/claude/C--Dev-pessoal-m-os/46c15ce2-16c4-4550-9306-5f16e86dd029/scratchpad/rstmp"; export TEMP="$TMP"; mkdir -p "$TMP"
cd /c/Dev/pessoal/m-os && cargo test -p mos-storage-sqlite --test sync_two_devices 2>&1 | tail -20
```

Esperado: todos passam, incluindo os dois novos.

- [ ] **Step 5: Fundir o mapa entre as passadas**

`sincronizar_agora` roda várias passadas até esvaziar a fila. Sem isto, o mapa da última passada apaga o das anteriores — e o erro só apareceria com fila grande, que é exatamente o caso deste PC.

Em `crates/mos-storage-sqlite/src/sync_projecao.rs`, ao lado de `rodada.recebidas += passada.recebidas;`:

```rust
            rodada.recebidas += passada.recebidas;
            // Funde, e nao substitui: com a fila grande sao varias passadas, e
            // atribuir deixaria a faixa contando so a ultima.
            for (tipo, quantas) in passada.recebidas_por_tipo {
                *rodada.recebidas_por_tipo.entry(tipo).or_insert(0) += quantas;
            }
```

- [ ] **Step 6: O teste da fusão, no laço de verdade**

Em `crates/mos-sync-http/tests/task_de_verdade.rs`, ao lado do teste que já prova o laço com limite 3 e dez tarefas, acrescente a asserção do mapa. Ache-o por:

```bash
grep -n "limite\|10\|dez" crates/mos-sync-http/tests/task_de_verdade.rs | head
```

E some ao final dele:

```rust
    // O mapa sobrevive as varias passadas. Sem a fusao, aqui daria o tamanho do
    // ultimo lote em vez do total — e o defeito so apareceria com fila grande.
    assert_eq!(rodada.recebidas_por_tipo.get("task"), Some(&10));
```

- [ ] **Step 7: Rodar tudo que toca sync**

```bash
export TMP="C:/WINDOWS/TEMP/claude/C--Dev-pessoal-m-os/46c15ce2-16c4-4550-9306-5f16e86dd029/scratchpad/rstmp"; export TEMP="$TMP"; mkdir -p "$TMP"
cd /c/Dev/pessoal/m-os && cargo test -p mos-sync -p mos-storage-sqlite -p mos-sync-http 2>&1 | tail -25
```

Esperado: tudo verde.

- [ ] **Step 8: Commit**

```bash
cd /c/Dev/pessoal/m-os && git add crates/mos-sync/src/engine.rs crates/mos-storage-sqlite/src/sync_projecao.rs crates/mos-storage-sqlite/tests/sync_two_devices.rs crates/mos-sync-http/tests/task_de_verdade.rs && git commit -m "feat(sync): a rodada conta o que chegou por tipo, e conta entidade

A faixa da Home precisa dizer \"3 tasks e 1 capture\", e \`recebidas\` nao serve
para isso: ele conta OPERACOES, e tres edicoes da mesma Task sao tres operacoes
e uma task. Dizer \"3 tasks\" para quem mexeu numa so seria mentira com numero.

O mapa e fundido entre as passadas, e nao atribuido. Com a fila grande sao
varias passadas, e o defeito de atribuir so apareceria justamente la — o teste
do laco com limite 3 e dez tarefas e quem fixa isso.

Chave String e nao enum: EntityKind e texto (SYNC.md §9), e um tipo que este
cliente ainda nao conhece precisa contar em vez de sumir.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: A regra da faixa, como função pura

A tela não decide nada. Não há teste de DOM neste repositório (decisão registrada
no `vitest.config.ts`), então o que decide vira função pura — no molde do
`homeLayout.ts` e do `daily.ts`.

**Esta task não depende da Task 1** e pode ser feita em paralelo: ela consome
tipos de TypeScript que esta própria task define.

**Files:**
- Create: `apps/desktop/src/syncFaixa.ts`
- Create: `apps/desktop/src/syncFaixa.test.ts`
- Modify: `apps/desktop/src/types.ts` (`SyncStatus` em `:1437`, `SyncRound` em `:1446`)

**Interfaces:**
- Produces: `estadoDaFaixa(status: SyncStatus | null): FaixaDeSync | null` e o tipo `FaixaDeSync`. A Task 4 monta o componente em cima disto.

- [ ] **Step 1: Os tipos**

Em `apps/desktop/src/types.ts`, substitua a `SyncStatus` inteira (o bloco em `:1437`) por:

```ts
export type SyncStatus = {
  /** Onde o hub esta. Vazio significa nao configurado — e o estado normal. */
  endpoint: string;
  hasToken: boolean;
  /** Mudancas locais esperando para subir. */
  pending: number;
  enabled: boolean;
  /** Uma rodada esta em curso agora. */
  running: boolean;
  /** Quando a ultima rodada terminou, em RFC3339. Nulo: nunca rodou. */
  lastSyncAt: string | null;
  /** Por que a ultima rodada parou. Nulo: terminou inteira. */
  lastError: string | null;
  /**
   * O resumo da primeira rodada do dia, enquanto nao for lido.
   *
   * Quem decide que e "a primeira do dia" e o backend, e nao a tela: como
   * estado do React, sair da Home e voltar traria a faixa de novo no mesmo dia.
   */
  daySummary: SyncDaySummary | null;
};

export type SyncDaySummary = {
  /**
   * Quantas ENTIDADES de cada tipo mudaram. Nao e `received` reparticionado —
   * aquele conta operacoes. Chave livre: um tipo que esta versao ainda nao
   * conhece aparece pelo id em vez de sumir.
   */
  byKind: Record<string, number>;
  /** Quando essa rodada terminou, em RFC3339. */
  at: string;
};
```

E acrescente a `SyncRound` (dentro da struct, depois de `pending`):

```ts
  /** Quantas entidades de cada tipo chegaram. Ver `SyncDaySummary.byKind`. */
  receivedByKind: Record<string, number>;
```

- [ ] **Step 2: Escrever os testes que falham**

Crie `apps/desktop/src/syncFaixa.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { estadoDaFaixa, frasePorTipo } from "./syncFaixa";
import type { SyncStatus } from "./types";

function status(over: Partial<SyncStatus> = {}): SyncStatus {
  return {
    endpoint: "http://127.0.0.1:9120",
    hasToken: true,
    pending: 0,
    enabled: true,
    running: false,
    lastSyncAt: "2026-08-28T09:00:00Z",
    lastError: null,
    daySummary: null,
    ...over,
  };
}

describe("quando a faixa NÃO aparece", () => {
  it("não aparece sem status nenhum, porque ainda não se sabe de nada", () => {
    expect(estadoDaFaixa(null)).toBeNull();
  });

  it("não aparece com o sync desligado: isso não é um problema, é uma feature desligada", () => {
    expect(estadoDaFaixa(status({ endpoint: "", hasToken: false }))).toBeNull();
  });

  it("não aparece com endereço mas sem segredo — ainda é desligado", () => {
    expect(estadoDaFaixa(status({ hasToken: false }))).toBeNull();
  });

  it("não aparece quando está tudo em dia", () => {
    expect(estadoDaFaixa(status())).toBeNull();
  });

  it("não aparece numa rodada silenciosa: piscar a cada 15 min é o ruído que queremos evitar", () => {
    expect(estadoDaFaixa(status({ running: true }))).toBeNull();
  });

  it("não aparece com resumo de zero recebidas — não ter notícia não é notícia", () => {
    expect(estadoDaFaixa(status({ daySummary: { byKind: {}, at: "2026-08-28T09:00:00Z" } }))).toBeNull();
  });
});

describe("quando a faixa aparece", () => {
  it("conta o que chegou, na primeira abertura do dia", () => {
    const faixa = estadoDaFaixa(status({
      daySummary: { byKind: { task: 3, capture: 1 }, at: "2026-08-28T09:00:00Z" },
    }));
    expect(faixa?.tipo).toBe("chegou");
    expect(faixa?.dispensavel).toBe(true);
  });

  it("mostra a fila que não esvaziou", () => {
    const faixa = estadoDaFaixa(status({ pending: 47 }));
    expect(faixa?.tipo).toBe("pendente");
    expect(faixa?.titulo).toBe("47 MUDANÇAS ESPERANDO");
  });

  it("mostra o erro com o motivo cru, sem traduzir para 'algo deu errado'", () => {
    const faixa = estadoDaFaixa(status({ lastError: "connection refused" }));
    expect(faixa?.tipo).toBe("erro");
    expect(faixa?.corpo).toContain("connection refused");
  });

  it("o erro ganha da fila: a fila é consequência, o erro é a causa", () => {
    const faixa = estadoDaFaixa(status({ pending: 47, lastError: "connection refused" }));
    expect(faixa?.tipo).toBe("erro");
  });

  it("a notícia ganha do erro velho, porque a rodada que trouxe coisa funcionou", () => {
    const faixa = estadoDaFaixa(status({
      lastError: null,
      daySummary: { byKind: { task: 2 }, at: "2026-08-28T09:00:00Z" },
    }));
    expect(faixa?.tipo).toBe("chegou");
  });

  it("erro e pendente NÃO se dispensam: some quando a causa some", () => {
    expect(estadoDaFaixa(status({ lastError: "x" }))?.dispensavel).toBe(false);
    expect(estadoDaFaixa(status({ pending: 5 }))?.dispensavel).toBe(false);
  });

  it("gira quando uma rodada corre por cima de uma faixa que já estava lá", () => {
    const faixa = estadoDaFaixa(status({ pending: 47, running: true }));
    expect(faixa?.girando).toBe(true);
  });
});

describe("a frase do que chegou", () => {
  it("pluraliza, e usa a palavra que o M/OS usa na tela", () => {
    expect(frasePorTipo({ task: 3, capture: 1 })).toBe("3 tasks · 1 capture");
  });

  it("um de cada não vira plural", () => {
    expect(frasePorTipo({ task: 1 })).toBe("1 task");
  });

  it("um tipo que esta versão não conhece aparece pelo id em vez de sumir", () => {
    expect(frasePorTipo({ tipo_do_futuro: 2 })).toBe("2 tipo_do_futuro");
  });

  it("ordena do maior para o menor, para a notícia grande vir primeiro", () => {
    expect(frasePorTipo({ capture: 1, task: 5 })).toBe("5 tasks · 1 capture");
  });
});
```

- [ ] **Step 3: Rodar e ver falhar**

```bash
cd /c/Dev/pessoal/m-os/apps/desktop && npx vitest run src/syncFaixa.test.ts 2>&1 | tail -20
```

Esperado: FAIL — `Failed to resolve import "./syncFaixa"`.

- [ ] **Step 4: Escrever a função**

Crie `apps/desktop/src/syncFaixa.ts`:

```ts
/**
 * A faixa de sincronizacao da Home: quando aparece, e o que diz.
 *
 * Vive fora do `App.tsx` para poder ser testada. Nao ha teste de DOM neste
 * repositorio, por decisao registrada no `vitest.config.ts`, e a consequencia
 * pratica e a de sempre: o que DECIDE alguma coisa tem de ser funcao pura, e o
 * componente so desenha o resultado.
 *
 * # Por que esta faixa e uma excecao, e por que ela pode ser
 *
 * O `App.tsx` registra o principio da Home: "tudo que mora na Home do M/OS e um
 * widget arrumavel, e uma excecao seria a unica coisa da tela que nao se pode
 * mover nem esconder."
 *
 * A faixa contradiz isso, e a contradicao precisa ficar escrita ao lado da
 * razao que ela contradiz — senao vira precedente, e o proximo card fixo aponta
 * para este.
 *
 * A defesa: aquele principio protege a Home de ter um MORADOR permanente que
 * nao se arruma. A faixa nao mora aqui. Ela so existe quando ha noticia ou
 * quando algo esta errado, e some quando e lida ou quando a causa some. Na
 * maioria dos dias ela nao ocupa espaco nenhum, entao nao compete com os
 * widgets pelo espaco que o principio protege.
 *
 * O widget arrumavel foi considerado e recusado: um widget se ESCONDE, e um
 * widget de sync escondido e um sync quebrado que ninguem descobre.
 */
import type { SyncStatus } from "./types";

export type TipoDaFaixa = "chegou" | "pendente" | "erro";

export type FaixaDeSync = {
  tipo: TipoDaFaixa;
  titulo: string;
  corpo: string;
  /** Uma rodada corre agora, POR CIMA desta faixa. */
  girando: boolean;
  /**
   * Se o botao de dispensar aparece.
   *
   * Falso para erro e pendente, e essa e a regra que amarra os seis estados: um
   * aviso que se pode calar sem consertar a causa e um aviso que se cala
   * sempre. Eles somem quando a causa some, e nao quando incomodam.
   */
  dispensavel: boolean;
};

/* Como cada tipo se chama na tela, e o plural dele.

   So os tipos que o M/OS mostra por nome. O que nao esta aqui aparece pelo
   proprio id — feio, e MUITO melhor que sumir: `EntityKind` e texto e nao enum
   fechado (SYNC.md §9), justamente para um cliente antigo guardar e reenviar um
   tipo que ele nao conhece. Sumir com ele faria a faixa dizer que nada chegou
   quando algo chegou. */
const NOMES: Record<string, [string, string]> = {
  task: ["task", "tasks"],
  capture: ["capture", "captures"],
  project: ["project", "projects"],
  resource: ["resource", "resources"],
  reminder: ["lembrete", "lembretes"],
  workspace: ["contexto", "contextos"],
  daily_session: ["dia", "dias"],
  daily_objective: ["objetivo do dia", "objetivos do dia"],
  daily_reflection: ["reflexão", "reflexões"],
  weekly_review: ["fecho de semana", "fechos de semana"],
  academic_semester: ["semestre", "semestres"],
  academic_subject: ["disciplina", "disciplinas"],
  academic_assignment: ["entrega", "entregas"],
  academic_exam: ["prova", "provas"],
  academic_study_session: ["sessão de estudo", "sessões de estudo"],
};

/**
 * "3 tasks · 1 capture".
 *
 * Ordena pelo NUMERO e nao pelo nome: a noticia grande vem primeiro, e uma
 * ordem alfabetica poria "academic_exam" na frente de vinte tasks.
 */
export function frasePorTipo(porTipo: Record<string, number>): string {
  return Object.entries(porTipo)
    .filter(([, quantas]) => quantas > 0)
    .sort(([aNome, a], [bNome, b]) => b - a || aNome.localeCompare(bNome))
    .map(([tipo, quantas]) => {
      const nomes = NOMES[tipo];
      if (!nomes) return `${quantas} ${tipo}`;
      return `${quantas} ${quantas === 1 ? nomes[0] : nomes[1]}`;
    })
    .join(" · ");
}

/**
 * Qual faixa desenhar, ou nenhuma.
 *
 * A ORDEM das perguntas e o desenho, e nao acaso:
 *
 * 1. desligado sai primeiro, e sai calado. Quem nao ligou o sync nao tem um
 *    problema, tem uma feature desligada — transformar isso em aviso diario na
 *    Home seria propaganda dentro do proprio produto;
 * 2. a NOTICIA ganha do erro, porque uma rodada que trouxe coisa funcionou, e o
 *    erro que sobrou e de antes dela;
 * 3. o erro ganha da fila, porque a fila e consequencia e o erro e a causa.
 *    Mostrar "47 esperando" sem dizer por que manda consertar as cegas.
 */
export function estadoDaFaixa(status: SyncStatus | null): FaixaDeSync | null {
  if (!status) return null;
  // Desligado: sem endereco ou sem segredo, nao ha o que sincronizar.
  if (!status.endpoint || !status.hasToken) return null;

  const girando = status.running;

  const resumo = status.daySummary;
  const chegou = resumo ? Object.values(resumo.byKind).reduce((a, b) => a + b, 0) : 0;
  if (resumo && chegou > 0) {
    return {
      tipo: "chegou",
      titulo: "CHEGOU ENQUANTO VOCÊ ESTAVA FORA",
      corpo: frasePorTipo(resumo.byKind),
      girando,
      dispensavel: true,
    };
  }

  if (status.lastError) {
    return {
      tipo: "erro",
      titulo: "A SINCRONIZAÇÃO PAROU",
      // O motivo CRU, e nao "algo deu errado". A causa quase sempre esta fora
      // do M/OS — tunel caido, hub fora — e so o texto de verdade diz onde ir.
      corpo: status.lastError,
      girando,
      dispensavel: false,
    };
  }

  if (status.pending > 0) {
    return {
      tipo: "pendente",
      titulo: `${status.pending} ${status.pending === 1 ? "MUDANÇA ESPERANDO" : "MUDANÇAS ESPERANDO"}`,
      corpo: "Ainda não subiram. Vou tentar sozinho; o botão adianta.",
      girando,
      dispensavel: false,
    };
  }

  // Em dia. A Home nao muda — o horario da ultima rodada vive no cabecalho.
  return null;
}
```

- [ ] **Step 5: Rodar e ver passar**

```bash
cd /c/Dev/pessoal/m-os/apps/desktop && npx vitest run src/syncFaixa.test.ts 2>&1 | tail -20
```

Esperado: `Test Files 1 passed`, 18 testes verdes.

- [ ] **Step 6: Provar que o TypeScript inteiro ainda compila**

```bash
cd /c/Dev/pessoal/m-os/apps/desktop && npx tsc --noEmit 2>&1 | head -20
```

Esperado: **erros** em `App.tsx`, porque `SyncStatus` ganhou campos que o backend ainda não manda. Isso é esperado e a Task 3 resolve. Anote quais são; se houver erro em outro arquivo que não `App.tsx`, conserte antes de commitar.

- [ ] **Step 7: Commit**

```bash
cd /c/Dev/pessoal/m-os && git add apps/desktop/src/syncFaixa.ts apps/desktop/src/syncFaixa.test.ts apps/desktop/src/types.ts && git commit -m "feat(sync): a regra da faixa, antes da faixa

Nao ha teste de DOM neste repositorio, entao o que decide vira funcao pura e o
componente so desenha — mesmo caminho do homeLayout.ts e do daily.ts.

A ordem das perguntas e o desenho: desligado sai calado, a noticia ganha do erro
(a rodada que trouxe coisa funcionou), e o erro ganha da fila (a fila e
consequencia). Erro e pendente nao se dispensam: um aviso que se cala sem
consertar a causa e um aviso que se cala sempre.

A excecao ao principio da Home fica escrita no arquivo, ao lado da razao que ela
contradiz. A faixa nao e moradora — so existe quando ha noticia, e some quando e
lida. O widget arrumavel foi recusado porque widget se esconde, e um widget de
sync escondido e um sync quebrado que ninguem descobre.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: O daemon

**Files:**
- Modify: `apps/desktop/src-tauri/src/sync.rs` (o arquivo inteiro cresce; hoje tem 152 linhas)
- Modify: `apps/desktop/src-tauri/src/lib.rs` (`UserSettings` em `:163`; `setup` em `:1940-2010`; `reveal_window` em `:1246`; lista de comandos em `:2249`)

**Interfaces:**
- Consumes: `mos_sync::Rodada.recebidas_por_tipo` (Task 1); os campos novos de `SyncStatus` em TS (Task 2).
- Produces: comandos `sync_app_pronto()` e `sync_dispensar_resumo()`; evento `sync-changed`; `SyncRuntime` gerenciado pelo Tauri.

**Lembre-se:** `cargo test -p mos-desktop` **não executa** nesta máquina. A prova
local é `--no-run`, e a verificação de verdade é a janela real (Step 9).

- [ ] **Step 1: O campo da data no `UserSettings`**

Em `apps/desktop/src-tauri/src/lib.rs`, dentro de `UserSettings`, logo depois de `sync_endpoint`:

```rust
    /// O dia em que o resumo da sincronizacao foi mostrado pela ultima vez.
    ///
    /// Data civil (`YYYY-MM-DD`), e nao instante: "primeira abertura do dia" e
    /// a mesma regua da Daily Session, e um segundo conceito de dia dentro do
    /// mesmo app seria uma divergencia esperando acontecer.
    ///
    /// Mora AQUI e nao no React porque, como estado da tela, sair da Home e
    /// voltar traria a faixa de novo no mesmo dia.
    #[serde(default)]
    sync_ultimo_resumo_em: String,
```

- [ ] **Step 2: O estado em memória, em `sync.rs`**

No topo de `apps/desktop/src-tauri/src/sync.rs`, depois dos `use` existentes:

```rust
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager};

/// De quanto em quanto tempo a rodada acontece sem ninguem pedir.
///
/// Quinze minutos e a REDE DE SEGURANCA, e nao o mecanismo: o que sincroniza de
/// verdade sao a abertura, o primeiro plano e a mutacao. Este intervalo existe
/// para o caso que nenhum dos tres cobre — a rede que voltou sozinha enquanto a
/// janela ficou aberta e parada. O `SYNC.md` §51 proibe polling agressivo, e
/// quinze minutos nao e polling: e uma tentativa por quarto de hora.
const REDE_DE_SEGURANCA: Duration = Duration::from_secs(15 * 60);

/// Quanto esperar depois de uma mutacao antes de sincronizar.
///
/// Sem isto, arrastar cinco tasks no Kanban dispararia cinco rodadas. O motor
/// segura o mutex do banco durante a rodada, entao rodada a mais nao e so
/// trafego — e a interface esperando.
const DEBOUNCE_DA_MUTACAO: Duration = Duration::from_secs(10);

/// Ate quando esperar a tela dizer que abriu, antes de rodar assim mesmo.
///
/// O sinal vem do renderer, e um sync que DEPENDE da tela e um sync que morre
/// em silencio quando a tela nao abre — e o M/OS pode abrir minimizado na
/// bandeja por configuracao. O teto existe para o automatico nunca ficar refem
/// de uma janela.
const TETO_DA_ABERTURA: Duration = Duration::from_secs(30);

/// O que a interface precisa saber e que nao cabe no banco.
pub struct SyncRuntime {
    /// Uma rodada por vez. Duas brigariam pelo mesmo `HlcClock`, e duas
    /// operacoes com o mesmo instante e o mesmo dispositivo quebram a ordem
    /// total — a unica coisa que a reconciliacao tem para desempatar.
    ///
    /// Assincrono de proposito: o clique manual ESPERA a rodada automatica e
    /// mostra o resultado dela, em vez de falhar ou enfileirar uma segunda.
    pub rodada: tokio::sync::Mutex<()>,
    /// Acorda o laco: primeiro plano, mutacao, clique.
    pub acordar: tokio::sync::Notify,
    /// A tela terminou de abrir.
    pub pronto: tokio::sync::Notify,
    /// Para a interface saber que gira agora.
    pub rodando: AtomicBool,
    pub ultima: Mutex<UltimaRodada>,
}

#[derive(Default)]
pub struct UltimaRodada {
    /// RFC3339 de quando a ultima rodada terminou.
    pub em: Option<String>,
    pub erro: Option<String>,
    /// O resumo por ler. `Some` so quando foi a primeira rodada do dia E ela
    /// trouxe alguma coisa.
    pub resumo: Option<Resumo>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resumo {
    pub by_kind: BTreeMap<String, usize>,
    pub at: String,
}

impl Default for SyncRuntime {
    fn default() -> Self {
        Self {
            rodada: tokio::sync::Mutex::new(()),
            acordar: tokio::sync::Notify::new(),
            pronto: tokio::sync::Notify::new(),
            rodando: AtomicBool::new(false),
            ultima: Mutex::new(UltimaRodada::default()),
        }
    }
}
```

- [ ] **Step 3: `SyncStatus` cresce, e uma rodada compartilhada**

Ainda em `sync.rs`, acrescente à struct `SyncStatus` (depois de `pub enabled: bool`):

```rust
    /// Uma rodada corre agora.
    pub running: bool,
    /// RFC3339 de quando a ultima terminou. `None`: nunca rodou.
    pub last_sync_at: Option<String>,
    /// Por que a ultima parou. `None`: terminou inteira.
    pub last_error: Option<String>,
    /// O resumo da primeira rodada do dia, enquanto nao for lido.
    pub day_summary: Option<Resumo>,
```

Preencha-os em `sync_status`, lendo o `SyncRuntime`:

```rust
#[tauri::command]
pub fn sync_status(state: State<'_, crate::AppState>, runtime: State<'_, SyncRuntime>) -> SyncStatus {
    use mos_sync::OutboxRepository;
    // ... o corpo que ja existe, montando endpoint/has_token/pending/enabled ...
    // e, no literal de retorno, os quatro novos:
    //   running: runtime.rodando.load(Ordering::Relaxed),
    //   last_sync_at: ultima.em.clone(),
    //   last_error: ultima.erro.clone(),
    //   day_summary: ultima.resumo.clone(),
}
```

Extraia o miolo do `sync_now` atual para uma função livre, porque o daemon e o
comando precisam da MESMA rodada — duplicar aqui seria duplicar a decisão de
quando parar:

```rust
/// Uma rodada, do jeito que o daemon e o botao fazem — o mesmo jeito.
///
/// Devolve `Ok(None)` quando nao ha nada configurado. Isso NAO e erro: o M/OS
/// funciona inteiro sem sincronizar, e o daemon chamaria isto a cada quinze
/// minutos numa maquina que nunca ligou o sync.
async fn rodar(
    app: &tauri::AppHandle,
    storage: Arc<SqliteStorage>,
    settings_path: std::path::PathBuf,
) -> Result<Option<SyncRound>, String> {
    let runtime = app.state::<SyncRuntime>();
    let endpoint = crate::load_settings(&settings_path).sync_endpoint;
    let Some(token) = token_guardado() else { return Ok(None) };
    if endpoint.is_empty() {
        return Ok(None);
    }

    // Uma por vez. Quem chegou depois ESPERA e ve o resultado desta.
    let _turno = runtime.rodada.lock().await;
    runtime.rodando.store(true, Ordering::Relaxed);
    let _ = app.emit("sync-changed", ());

    let resultado = tauri::async_runtime::spawn_blocking(move || {
        let transporte =
            mos_sync_http::HttpTransport::novo(endpoint, token).map_err(|erro| erro.mensagem)?;
        let agora = time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
        storage
            .sincronizar_agora(&transporte, agora as i64, LIMITE)
            .map_err(|erro| erro.message)
    })
    .await
    .map_err(|erro| format!("A rodada de sincronizacao nao terminou: {erro}"))?;

    runtime.rodando.store(false, Ordering::Relaxed);

    let rodada = resultado?;
    let agora = time::OffsetDateTime::now_utc();
    let em = agora
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    let hoje = format!("{}", agora.date());

    // O resumo so nasce na PRIMEIRA rodada do dia, e so se trouxe algo. Nao ter
    // noticia nao e noticia.
    {
        let mut settings = crate::load_settings(&settings_path);
        let primeira_do_dia = settings.sync_ultimo_resumo_em != hoje;
        let mut ultima = runtime.ultima.lock().map_err(|_| "Estado do sync ocupado.".to_string())?;
        ultima.em = Some(em.clone());
        ultima.erro = rodada.erro.clone();
        if primeira_do_dia && !rodada.recebidas_por_tipo.is_empty() {
            ultima.resumo = Some(Resumo {
                by_kind: rodada.recebidas_por_tipo.clone().into_iter().collect(),
                at: em.clone(),
            });
            // Marca ANTES de a tela ler: se o app fechar entre a rodada e a
            // leitura, o resumo se perde — e perder uma noticia e melhor que
            // mostrar a de anteontem como se fosse de hoje.
            settings.sync_ultimo_resumo_em = hoje;
            let _ = crate::save_settings(&settings_path, &settings);
        }
    }

    let _ = app.emit("sync-changed", ());
    Ok(Some(SyncRound {
        sent: rodada.enviadas,
        received: rodada.recebidas,
        conflicts: rodada.conflitos,
        pending: rodada.pendentes,
        received_by_kind: rodada.recebidas_por_tipo.into_iter().collect(),
        error: rodada.erro,
    }))
}
```

Acrescente `received_by_kind: BTreeMap<String, usize>` à struct `SyncRound`, e
reescreva `sync_now` para chamar `rodar` e traduzir `Ok(None)` na mensagem que já
existe hoje (`"Configure o endereco do hub antes de sincronizar."`).

- [ ] **Step 4: O laço**

Ainda em `sync.rs`:

```rust
/// O laco que faz o M/OS sincronizar sozinho.
///
/// # Por que a primeira rodada ESPERA
///
/// `sincronizar_agora` segura o mutex do storage a rodada inteira, de proposito
/// — soltar no meio faria uma mutacao local emitir um instante que o motor ja
/// passou. Com fila grande (este PC tinha centenas), rodar junto com a abertura
/// seguraria o banco enquanto a webview faz a rajada de IPC do boot: o
/// `abertura.ts` gastaria as 12 tentativas dele contra um banco ocupado, e o
/// sintoma seria a tela de erro que a sessao de 25/08 acabou de consertar — com
/// causa nova e a mesma mensagem mentirosa.
///
/// Por isso: espera o `pronto`, com teto. Ver `TETO_DA_ABERTURA`.
pub fn iniciar_daemon(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let (storage, settings_path) = {
            let state = app.state::<crate::AppState>();
            (Arc::clone(&state.storage), state.settings_path.clone())
        };

        {
            let runtime = app.state::<SyncRuntime>();
            let _ = tokio::time::timeout(TETO_DA_ABERTURA, runtime.pronto.notified()).await;
        }

        loop {
            if let Err(erro) = rodar(&app, Arc::clone(&storage), settings_path.clone()).await {
                // Ao `stderr`, e nao numa caixa: a rodada de fundo que falha nao
                // pode interromper quem esta trabalhando. A faixa mostra.
                eprintln!("[sync] a rodada de fundo parou: {erro}");
                let runtime = app.state::<SyncRuntime>();
                runtime.rodando.store(false, Ordering::Relaxed);
                if let Ok(mut ultima) = runtime.ultima.lock() {
                    ultima.erro = Some(erro);
                }
                let _ = app.emit("sync-changed", ());
            }

            // Dorme ate alguem pedir, ou ate a rede de seguranca.
            let runtime = app.state::<SyncRuntime>();
            let _ = tokio::time::timeout(REDE_DE_SEGURANCA, runtime.acordar.notified()).await;
        }
    });
}

/// Pede uma rodada. Ignorado se o daemon ainda nao existe.
///
/// `notify_one` e nao `notify_waiters`: o laco e um so, e uma chamada enquanto
/// ele ja roda fica GUARDADA — a proxima espera retorna na hora, em vez de a
/// mutacao que chegou no meio da rodada esperar quinze minutos.
pub fn acordar<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(runtime) = app.try_state::<SyncRuntime>() {
        runtime.acordar.notify_one();
    }
}

/// A tela terminou de abrir. Libera a primeira rodada.
#[tauri::command]
pub fn sync_app_pronto(runtime: State<'_, SyncRuntime>) {
    runtime.pronto.notify_waiters();
}

/// O resumo do dia foi lido.
#[tauri::command]
pub fn sync_dispensar_resumo(
    app: tauri::AppHandle,
    runtime: State<'_, SyncRuntime>,
) -> Result<(), String> {
    runtime
        .ultima
        .lock()
        .map_err(|_| "Estado do sync ocupado.".to_string())?
        .resumo = None;
    let _ = app.emit("sync-changed", ());
    Ok(())
}
```

- [ ] **Step 5: Ligar no `setup` e nos gatilhos**

Em `lib.rs`, no `setup`, **depois** do bloco `app.manage(AppState { ... })` — a ordem importa, porque o daemon lê `AppState` — e junto dos outros `app.manage`:

```rust
            app.manage(sync::SyncRuntime::default());
            // O sync automatico. Ele espera a tela dizer que abriu antes da
            // primeira rodada: ver `iniciar_daemon`.
            sync::iniciar_daemon(app.handle().clone());
```

Em `reveal_window`, logo depois de `let _ = window.emit("window-revealed", ());`:

```rust
        // Voltar ao primeiro plano e o gatilho que o fluxo casa > trabalho >
        // celular mais usa: sentar na mesa e trazer o M/OS para frente.
        sync::acordar(&window.app_handle().clone());
```

Para a mutação, **ouça o evento em vez de tocar os 25 lugares que o emitem** —
`data-changed` e `capture-changed` já cobrem toda escrita, e um `emit` novo em
cada um deles seria 25 chances de esquecer um. No `setup`, depois do
`iniciar_daemon`:

```rust
            // Debounce por Notify: o laco ja dorme, e acordar cedo demais so
            // adiantaria a rodada. `notify_one` guarda o pedido, entao a mutacao
            // que chega durante uma rodada nao se perde.
            {
                let handle = app.handle().clone();
                app.listen_any("data-changed", move |_| {
                    let handle = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(sync::DEBOUNCE_DA_MUTACAO).await;
                        sync::acordar(&handle);
                    });
                });
            }
```

Torne `DEBOUNCE_DA_MUTACAO` `pub` para isso. Acrescente `sync_app_pronto` e
`sync_dispensar_resumo` à lista `tauri::generate_handler![...]` (`:2249`).

- [ ] **Step 6: Provar que compila**

```bash
export TMP="C:/WINDOWS/TEMP/claude/C--Dev-pessoal-m-os/46c15ce2-16c4-4550-9306-5f16e86dd029/scratchpad/rstmp"; export TEMP="$TMP"; mkdir -p "$TMP"
cd /c/Dev/pessoal/m-os && cargo test -p mos-desktop --lib --no-run 2>&1 | tail -30
```

Esperado: compila. **Não tente executar** — o binário morre com `0xC0000139` nesta máquina, por ambiente. Diga no relatório que a execução ficou para o CI.

Se o `app.listen_any` não existir com esse nome no Tauri 2 desta versão, veja o
que o `Listener` expõe:

```bash
export TMP="C:/WINDOWS/TEMP/claude/C--Dev-pessoal-m-os/46c15ce2-16c4-4550-9306-5f16e86dd029/scratchpad/rstmp"; export TEMP="$TMP"; mkdir -p "$TMP"
cd /c/Dev/pessoal/m-os && cargo doc -p tauri --no-deps 2>&1 | tail -3
grep -rn "fn listen" ~/.cargo/registry/src/*/tauri-2*/src/lib.rs | head
```

- [ ] **Step 7: A ponte para o TypeScript**

Em `apps/desktop/src/api.ts`, junto dos outros de sync (`:1084-1102`):

```ts
  /** A tela abriu. Libera a primeira rodada automatica. */
  syncAppReady() {
    return invoke<void>("sync_app_pronto");
  },

  /** O resumo do dia foi lido. */
  syncDismissSummary() {
    return invoke<void>("sync_dispensar_resumo");
  },
```

- [ ] **Step 8: Commit**

```bash
cd /c/Dev/pessoal/m-os && git add apps/desktop/src-tauri/src/sync.rs apps/desktop/src-tauri/src/lib.rs apps/desktop/src/api.ts && git commit -m "feat(sync): o M/OS sincroniza sozinho, e a primeira rodada espera o app abrir

O fluxo e casa > trabalho > celular, e o elo do meio era manual: o celular ja
sincronizava sozinho e o desktop so no clique. Quatro gatilhos, e nenhum e
polling (SYNC.md §51): abertura, primeiro plano, mutacao com debounce de 10s, e
uma rede de seguranca de 15 min para a rede que volta sozinha.

# A primeira rodada espera, e isso nao e otimizacao

`sincronizar_agora` segura o mutex do storage a rodada inteira, de proposito.
Com a fila grande deste PC, rodar junto com a abertura seguraria o banco durante
a rajada de IPC do boot: o `abertura.ts` gastaria as 12 tentativas contra um
banco ocupado, e o sintoma seria a tela de erro que 4800e75 acabou de consertar
— com causa nova e a mesma mensagem mentirosa.

Entao o daemon espera um sinal do renderer. Com teto de 30s: um sync que depende
da tela morre em silencio quando a tela nao abre, e o M/OS abre minimizado por
configuracao.

# A mutacao e OUVIDA, e nao emitida

`data-changed` ja e emitido em 25 lugares. Tocar os 25 seriam 25 chances de
esquecer um, e o esquecido nao daria erro — daria uma entidade que so sai deste
aparelho no proximo quarto de hora. `listen_any` pega todos, agora e os que
vierem depois.

# Uma rodada por vez, e a manual nao perde

Mutex assincrono: o clique durante uma rodada de fundo ESPERA e mostra o
resultado dela, em vez de falhar ou emitir um segundo relogio. Duas operacoes
com o mesmo instante e o mesmo dispositivo quebram a ordem total.

`cargo test -p mos-desktop` compila; a execucao ficou para o CI (0xC0000139
nesta maquina, por ambiente).

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: A faixa na tela

**Files:**
- Create: `apps/desktop/src/SyncFaixa.tsx`
- Modify: `apps/desktop/src/App.tsx` (`HomePage` em `:510`; o `return` em `:758-762`; o boot que resolve `bootState`)
- Modify: `apps/desktop/src/App.css`

**Interfaces:**
- Consumes: `estadoDaFaixa`, `FaixaDeSync` (Task 2); `api.syncDismissSummary`, `api.syncAppReady`, evento `sync-changed` (Task 3).

- [ ] **Step 1: O componente**

Crie `apps/desktop/src/SyncFaixa.tsx`:

```tsx
/**
 * A faixa de sincronizacao. So desenha — quem DECIDE e o `syncFaixa.ts`.
 *
 * A justificativa da excecao ao principio da Home mora la, junto da regra.
 */
import { useCallback, useEffect, useState } from "react";
import { api, appError } from "./api";
import { Button } from "./Button";
import { MosSymbol } from "./Symbol";
import { estadoDaFaixa } from "./syncFaixa";
import type { SyncStatus } from "./types";

export function SyncFaixa({ status, onChanged }: { status: SyncStatus | null; onChanged: () => void }) {
  const [ocupado, setOcupado] = useState(false);
  const faixa = estadoDaFaixa(status);
  if (!faixa) return null;

  async function agora() {
    setOcupado(true);
    try { await api.syncNow(); } catch { /* o proprio status conta o erro */ }
    setOcupado(false);
    onChanged();
  }

  async function dispensar() {
    try { await api.syncDismissSummary(); } catch { /* nada a fazer */ }
    onChanged();
  }

  return <section className="sync-faixa" data-tipo={faixa.tipo} aria-live="polite">
    <div className="sync-faixa-texto">
      <span className="micro-label">{faixa.titulo}</span>
      <p>{faixa.corpo}</p>
    </div>
    <div className="sync-faixa-acoes">
      {faixa.girando || ocupado ? <MosSymbol size={16} spinning /> : null}
      {faixa.dispensavel
        ? <Button variant="ghost" size="sm" onClick={() => void dispensar()}>Dispensar</Button>
        : <Button variant="secondary" size="sm" disabled={faixa.girando || ocupado} onClick={() => void agora()}>Tentar agora</Button>}
    </div>
  </section>;
}
```

Confirme os nomes reais de `Button` e `MosSymbol` antes de escrever:

```bash
cd /c/Dev/pessoal/m-os/apps/desktop/src && grep -n "^export" Button.tsx Symbol.tsx
```

- [ ] **Step 2: O estado do sync no `App`, e o `syncAppReady`**

No componente raiz de `App.tsx`, junto do `useEffect` que registra os `listen` (`:3492`):

```tsx
  const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null);
  const refreshSync = useCallback(() => {
    void api.syncStatus().then(setSyncStatus).catch(() => undefined);
  }, []);
  useEffect(() => {
    refreshSync();
    // Sem polling: o backend avisa quando uma rodada termina.
    const parar = listen("sync-changed", refreshSync);
    return () => { void parar.then((f) => f()); };
  }, [refreshSync]);
```

E, onde o boot conclui com sucesso (`bootState` vira `"ready"`), avise o daemon
**uma vez**:

```tsx
  useEffect(() => {
    if (bootState !== "ready") return;
    // Libera a primeira rodada automatica. O daemon espera este sinal para nao
    // segurar o banco durante a rajada de IPC do boot — ver `iniciar_daemon`.
    void api.syncAppReady().catch(() => undefined);
  }, [bootState]);
```

Passe `syncStatus` e `refreshSync` para `HomePage` (some às props dela e ao tipo
inline da assinatura em `:510`).

- [ ] **Step 3: Montar na Home**

Em `HomePage`, no `return`, entre `<ContextPath ... />` e `<CaptureComposer ... />`:

```tsx
    <ContextPath segments={["M", "HOME"]} />
    {/* A faixa NAO e widget, e essa e a unica excecao ao principio de que tudo
        na Home se arruma. Ela pode ser: nao mora aqui — so existe quando ha
        noticia ou quando algo esta errado, e some quando e lida ou quando a
        causa some. A justificativa inteira, e o que foi recusado (o widget
        arrumavel), estao no `syncFaixa.ts`. */}
    <SyncFaixa status={syncStatus} onChanged={refreshSync} />
    <CaptureComposer ... />
```

- [ ] **Step 4: O estilo**

Em `App.css`, junto dos outros blocos da Home:

```css
/* A faixa de sync. Discreta de proposito: ela aparece quando ha noticia, e uma
   noticia que grita todo dia deixa de ser lida. A borda a esquerda carrega o
   estado, e nao o fundo inteiro. */
.sync-faixa {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-4);
  margin-bottom: var(--space-4);
  border: 1px solid var(--border-subtle);
  border-left: 3px solid var(--accent);
  border-radius: var(--radius-sm);
  background: var(--surface-raised);
}
.sync-faixa[data-tipo="erro"] { border-left-color: var(--danger); }
.sync-faixa[data-tipo="pendente"] { border-left-color: var(--warning); }
.sync-faixa-texto p { margin: var(--space-1) 0 0; color: var(--text-secondary); }
.sync-faixa-acoes { display: flex; align-items: center; gap: var(--space-2); flex-shrink: 0; }
```

Confirme os nomes reais dos tokens antes de escrever — invente nenhum:

```bash
cd /c/Dev/pessoal/m-os && grep -n "^  --" packages/design-system/tokens.css | head -40
```

- [ ] **Step 5: O estado calmo, no cabeçalho**

É a outra metade da decisão "a faixa some quando está em dia": some da Home
**porque** o cabeçalho guarda o horário. Sem isto, "em dia" não é um estado
visível em lugar nenhum, e a única forma de saber se o sync ainda funciona seria
abrir o Settings.

O cabeçalho já tem a região certa — `system-state`, em `App.tsx:3913`, que hoje
mostra `SINCRONIZANDO` quando o app está ocupado. Acrescente o horário ao lado
do `page-meta`, e **só quando o sync está ligado**:

```tsx
{syncStatus?.endpoint && syncStatus.hasToken && syncStatus.lastSyncAt
  ? <span className="page-meta" title="Última sincronização">SYNC {relativeTime(syncStatus.lastSyncAt)}</span>
  : null}
```

Discreto de propósito: é o estado calmo. Se ele chamasse atenção, seria a faixa
fixa que esta spec recusou.

- [ ] **Step 6: TypeScript e testes**

```bash
cd /c/Dev/pessoal/m-os/apps/desktop && npx tsc --noEmit 2>&1 | head -20 && npm test 2>&1 | tail -15
```

Esperado: `tsc` sem saída, e todos os testes verdes. Os erros de `App.tsx` que a Task 2 deixou pendentes desaparecem aqui.

- [ ] **Step 7: Ver na janela de verdade**

Este é o único passo que prova a feature. Use a skill `ver-o-app`.

Confira, nesta ordem:

1. o M/OS abre **sem** a tela de erro e sem demora perceptível — é a regressão que a espera da Task 3 existe para evitar;
2. em uma máquina com fila pendente, a faixa de **pendente** aparece com o número certo;
3. clicar "Tentar agora" esvazia a fila e a faixa some sozinha;
4. desligar o túnel (`Stop-ScheduledTask -TaskName "M-OS Sync Tunnel"`) e reabrir o M/OS: a faixa de **erro** aparece com o motivo cru, e **não tem** botão de dispensar;
5. religar o túnel e sincronizar: a faixa some.

Para o estado **chegou**, force a primeira-do-dia editando `syncUltimoResumoEm` no `settings.json` para uma data passada, com o M/OS fechado, e mexa em algo no celular antes de reabrir.

**Nunca use `Stop-Process` no `mos-desktop`** — feche pela janela.

- [ ] **Step 8: Commit**

```bash
cd /c/Dev/pessoal/m-os && git add apps/desktop/src/SyncFaixa.tsx apps/desktop/src/App.tsx apps/desktop/src/App.css && git commit -m "feat(home): a faixa que so aparece quando tem o que dizer

Com o sync automatico, uma faixa fixa dizendo 'tudo certo' viraria movel — some
da percepcao justamente antes do dia em que teria algo. Entao ela nao existe na
maioria dos dias, e o estado calmo continua no cabecalho.

A excecao ao principio da Home fica escrita ao lado do lugar onde ela e montada,
e a defesa inteira no syncFaixa.ts. Sem isso vira precedente, e o proximo card
fixo aponta para este.

Sem polling na tela: o backend emite `sync-changed` quando uma rodada termina.

Conferido na janela real: [PREENCHER com o que voce viu nos cinco passos].

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Os documentos param de mentir

O `SYNC.md` §14 ainda diz que nenhuma implementação de rede existe — deixou de
ser verdade em 26/08 — e o §8 lista os gatilhos como plano. Um documento que
descreve um sistema que não existe mais é pior que nenhum, porque é lido com
confiança.

**Files:**
- Modify: `docs/SYNC.md` (§8 em `:145-160`, §10 em `:176-188`, §14 em `:308-324`)

- [ ] **Step 1: §8 vira descrição**

Reescreva a abertura do §8 para dizer o que existe, com os números reais
(`DEBOUNCE_DA_MUTACAO` = 10s, `REDE_DE_SEGURANCA` = 15 min), e mantenha na lista
de "ainda não" apenas o sinal de push e a reconexão de rede — que a §8 da spec
deixa explicitamente de fora.

- [ ] **Step 2: §10 ganha o sexto estado**

A lista de estados diz cinco. Acrescente **desligado**, e a razão de ele não
virar faixa: quem não ligou o sync não tem um problema, tem uma feature
desligada.

- [ ] **Step 3: §14 perde o que já existe**

Apague o marcador "Transporte real e servidor" e o "Auth" (o token no Credential
Manager e o segredo do hub existem desde 26/08). O resto do §14 continua verdade:
faltam Calendar, Meetings, Conversations, Tracking, Workspaces (não — este passou
a emitir em `4788efa`), Apps e Voice; e faltam os binários dos Resources.

**Confira a lista contra o código antes de escrever** — não copie a da spec:

```bash
cd /c/Dev/pessoal/m-os && grep -n "sync_emit\|emitir" crates/mos-storage-sqlite/src/sync_projecao.rs | head -30
```

- [ ] **Step 4: Commit**

```bash
cd /c/Dev/pessoal/m-os && git add docs/SYNC.md && git commit -m "docs(sync): o §8 deixa de descrever um plano, e o §14 perde o que ja existe

O §14 ainda dizia 'nenhuma implementacao de rede existe' dois dias depois de ela
existir, e o §8 listava os gatilhos como futuro no commit em que eles viraram
codigo. Documento que descreve um sistema que nao existe mais e pior que
nenhum, porque e lido com confianca.

O §10 ganha o sexto estado: desligado. Ele faltava porque, ate ontem, ninguem
tinha aberto o M/OS numa maquina onde o sync nunca foi ligado.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

## Verificação final

```bash
export TMP="C:/WINDOWS/TEMP/claude/C--Dev-pessoal-m-os/46c15ce2-16c4-4550-9306-5f16e86dd029/scratchpad/rstmp"; export TEMP="$TMP"; mkdir -p "$TMP"
cd /c/Dev/pessoal/m-os && cargo test --workspace --exclude mos-desktop 2>&1 | tail -20
cd /c/Dev/pessoal/m-os && cargo test -p mos-desktop --lib --no-run 2>&1 | tail -5
cd /c/Dev/pessoal/m-os/apps/desktop && npm test 2>&1 | tail -10 && npx tsc --noEmit
```

Ao relatar: diga explicitamente que os testes do crate `mos-desktop` **compilam e
não foram executados** nesta máquina, e o que você viu na janela real. Não diga
"funciona" sobre nada que você não abriu.
