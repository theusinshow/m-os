# M/OS — Universal Drop Zone

**Estado:** implementado
**Data:** 2026-08-19
**Decisão:** ADR-046
**Subordinado a:** `VISION.md` §4 e §12, `CORE.md` §3 e §17, `UX-PRINCIPLES.md`
§4, §21 e §22, `ARCHITECTURE.md` §9 e §11

---

## 1. O que é

Uma entrada universal. Qualquer conteúdo solto sobre a janela do M/OS entra, sem
que a pessoa precise decidir antes o que aquilo é.

> **Solta primeiro. Organiza depois.**

Não é um componente de drag-and-drop: é o **pipeline de ingestão** do M/OS, com
uma superfície visual em cima. As outras portas previstas — Voice Inbox, Meeting
Agent, Share Sheet do iOS, clipboard, Hermes, extensão de navegador — entram como
uma variante nova de `IngestionSource`, e não como um segundo caminho.

---

## 2. O pipeline

```text
DROP
 │
 ├─ 1. abre        ingest_begin    Capture + linha de ingestão   ← COMMIT
 ├─ 2. recebe      ingest_chunk    bytes crus, em fatias de 4 MB
 ├─ 3. preserva    ingest_finish   hash, rename atômico          ← COMMIT
 ├─ 4. entende                     tipo, duplicata, contexto
 ├─ 5. cria                        Resource + relações           ← COMMIT
 └─ 6. lê                          texto do conteúdo, em thread própria
```

Três commits, e a ordem entre eles é a promessa:

| depois de | o que já está garantido |
|---|---|
| passo 1 | a Inbox sabe dizer o nome do que a pessoa soltou |
| passo 3 | o original está no disco, inteiro, endereçado pelo hash |
| passo 5 | existe entidade, relação e índice |

Falhar no passo 6 não custa nada do que veio antes. Falhar no 4 ou no 5 deixa uma
Capture na Inbox e o arquivo no disco. Falhar no 2 apaga o pedaço recebido — um
arquivo truncado não é o original — e a Capture continua na Inbox.

---

## 3. Onde cada coisa mora

```text
apps/desktop/src/DropZone.tsx        superfície, lote, painel, recibo
apps/desktop/src/dropIngest.ts       decisões puras do drop (testadas)
        │ invoke
apps/desktop/src-tauri/src/ingest.rs comandos; costura e nada mais
        ├──────────────┬──────────────────┐
        ▼              ▼                  ▼
   mos-core        mos-ingest      mos-storage-sqlite
   ingestion.rs    FileStore       ingestion_repository.rs
   as decisões     + extract       um passo, uma transação
```

**`mos-core::ingestion`** não toca disco, banco nem rede. Ele responde: que nome
isto vai ter, que tipo isto é, onde isto deve ser guardado, a que isto pertence e
com que confiança.

**`mos-ingest`** é o lado físico: recebe bytes, calcula o SHA-256 enquanto
recebe, faz `sync_all` e move para o destino. Ele **não depende de
`mos-storage-sqlite`**, e a ausência é a decisão — "o disco nunca escreve no
banco" deixa de ser regra a lembrar e vira impossibilidade de compilação (mesmo
truque estrutural da ADR-024 e do `mos-audio`). Existe separado do `src-tauri`
porque `cargo test -p mos-desktop` não roda nesta máquina (`SETUP-MAQUINA.md` §4),
e o que precisa de teste tem que morar onde o teste roda.

**`ingest.rs`** é casca: sem decisão própria, sem regra de domínio.

---

## 4. Por que os bytes atravessam a ponte

A janela `main` tem `dragDropEnabled: false` no `tauri.conf.json`. Isso faz o
WebView2 entregar o arrastar ao HTML em vez de o Tauri interceptá-lo — é o que
mantém o arrastar de widget da Home e o do Kanban funcionando, e ligar a captura
nativa quebraria os dois (limitação do WebView2, não escolha nossa).

A consequência é que **o M/OS não recebe um caminho de disco**: ele recebe um
`File` do navegador. Os bytes são lidos em fatias de 4 MB e enviados no **corpo
bruto** da chamada IPC, e não como JSON — um `Vec<u8>` serializado em JSON custa
de três a quatro vezes o tamanho do arquivo, e um PDF de 40 MB viraria mais de
120 MB de string atravessando a ponte.

O id da ingestão viaja no header `x-mos-ingestion`, porque o corpo já está
ocupado sendo o arquivo.

---

## 5. Entradas suportadas

| entrada | vira | conteúdo lido |
|---|---|---|
| PDF | Capture + Resource `file` | texto e número de páginas (`lopdf`) |
| imagem (PNG/JPEG/GIF/BMP/WebP) | Capture + Resource `file` | dimensões, do cabeçalho |
| TXT, MD, CSV, JSON, XML, YAML, código | Capture + Resource `file` | o texto |
| DOCX, XLSX, ZIP, formato desconhecido | Capture + Resource `file` | nada, e isso não é erro |
| URL | Capture + Resource `site` | domínio como título |
| texto solto | Capture, e só | o próprio texto |
| vários arquivos | um item independente por arquivo | — |

O que decide o tipo é a extensão primeiro e o MIME depois — o navegador declara
`application/octet-stream` com frequência, e um palpite ruim não pode vencer um
dado que existe.

**Texto solto não vira Resource.** Ele já está preservado na Capture, já está na
Inbox, já é pesquisável e já pode virar Task, Note ou Resource pelo caminho que
existe. Criar um Resource automaticamente seria decidir por inferência o que a
frase significa.

---

## 6. Armazenamento

```text
%APPDATA%/com.codedbym.mos/
├── m-os.db                    metadata, entidades, relações, índices
└── drops/
    ├── .recebendo/            staging; nada aqui conta como preservado
    └── ab/cd/<sha256>.pdf     o original
```

Endereçado pelo conteúdo: dois drops do mesmo arquivo apontam para o mesmo lugar,
e **nenhum nome vindo do usuário participa do caminho**. Os dois primeiros pares
de dígitos viram pastas para não produzir um diretório com dezenas de milhares de
entradas.

O banco guarda o caminho **relativo**. Um caminho absoluto persistido quebraria no
dia em que o perfil mudasse de lugar.

---

## 7. Falhas

| onde falha | o que acontece | o que a pessoa vê |
|---|---|---|
| tamanho acima de 512 MB | recusa antes de escrever byte | erro na linha do item |
| leitura no renderer | `ingest_abort`, staging apagado | erro na linha; Capture na Inbox |
| disco cheio ao mover | ingestão `failed` | erro na linha; Capture na Inbox |
| tipo desconhecido | segue normalmente | "Guardado" |
| duplicata | relaciona o contexto ao que existia | "Já estava aqui" |
| extração de PDF quebra | `extraction_state = failed` | o arquivo continua lá e buscável pelo nome |
| PDF sem camada de texto | `extraction_state = empty` | "Sem texto para indexar — guardado do mesmo jeito" |
| app fecha no meio | abertura seguinte marca `interrupted` e limpa o staging | Capture na Inbox |
| app fecha após preservar | abertura seguinte **termina** a ingestão | o Resource aparece |

O único caminho que perde bytes é o que falha **antes** do passo 3 — e nele nunca
existiu arquivo para perder.

---

## 8. Contexto e confiança

O contexto da tela é capturado no instante do drop e gravado na linha de ingestão
**mesmo quando não gera relação nenhuma**: sem isso, descobrir depois que uma
relação deveria ter existido não teria como ser respondido.

| sinal | confiança | ação |
|---|---:|---|
| Project aberto | 0.95 | relaciona |
| Task aberta (o Project dela) | 0.90 | relaciona |
| lente de Workspace ativa | 0.80 | relaciona |
| nome do arquivo cita um Project | 0.60 | sugere, com um clique |
| nada | 0 | não inventa |

Os limiares são constantes nomeadas em `mos-core::ingestion`
(`CONFIDENCE_LINK`, `CONFIDENCE_SUGGEST`) para poderem ser calibrados sem caçar
número solto no código. O raciocínio de cada linha está na ADR-046.

A Task ainda não tem relação própria com Resource; o que se relaciona é o Project
dela. O `task_id` fica gravado na ingestão, e é dele que a relação sai no dia em
que ela existir.

---

## 9. Search

Nenhum mecanismo novo. `search_resources` passou a fazer duas passadas:

1. `resource_search` — título, URL e motivo;
2. `ingestion_search` — nome do arquivo e texto extraído.

Nessa ordem, e não num `UNION`: os dois índices têm escalas de `bm25` que não se
comparam, e a ordem certa é decisão de produto — quem acerta pelo nome vem antes
de quem acerta na página 143 de um memorial.

O texto extraído mora numa **coluna** (`ingestions.extracted_text`) e o FTS é
externo a ela. Um índice que guardasse o único exemplar do texto deixaria de ser
reconstruível, e a ADR-009 exige que ele seja derivado. O teto é de 256 mil
caracteres: o texto existe para reencontrar o arquivo, não para substituí-lo.

---

## 10. Segurança

- **nada recebido é executado.** `open_ingested_file` recusa 22 extensões que o
  shell do Windows trataria como programa (`exe`, `ps1`, `lnk`, `hta`, …). Elas
  continuam guardadas, exportáveis e pesquisáveis; o que não existe é o botão que
  as dispara. "Mostrar na pasta" continua disponível — o arquivo é da pessoa;
  quem decide executar é ela, no Explorer, e não o M/OS;
- **nome nunca vira caminho.** `sanitize_file_name` derruba separadores, `..`,
  caracteres de controle e nomes reservados do Windows (`CON`, `LPT1`). O nome
  sobrevive como rótulo; o caminho vem do hash;
- **nada escapa da área de drops.** `FileStore::resolve` recusa caminho absoluto,
  qualquer componente que não seja um nome simples e qualquer coisa fora de
  `drops/`. A guarda existe mesmo com o caminho sendo derivado do hash: se um dia
  ele passar a vir de outro lugar, a fuga falha ali e não no filesystem;
- **o teto de tamanho é conferido pedaço a pedaço**, e não apenas contra o
  tamanho declarado — o declarado é dado do usuário;
- **nenhuma rede.** O pipeline inteiro é local. URL guarda o endereço; não baixa
  a página.

---

## 11. Extensão

Uma porta nova precisa de:

1. uma variante em `IngestionSource`;
2. um comando que chame `begin_ingestion` com a Capture correspondente;
3. quando houver bytes, `FileStore::receive` → `commit`.

O resto — detecção, deduplicação, contexto, criação, relação, índice, desfazer,
reconciliação na abertura — já vale para ela.

| porta futura | o que ela reusa |
|---|---|
| Voice Inbox | `DropText` com a transcrição; a Capture já é a origem |
| Meeting Agent | `DropFile` para anexos da reunião |
| Share Sheet do iOS | `DropUrl` e `DropFile` pelo mesmo contrato |
| Hermes | `understand_resource` lê `extracted_text`; `suggest_relations` escreve pelo `RelationPlan` |
| OCR | a fila é `extraction_state = 'empty'` |

O núcleo da ingestão **não conhece nenhum provedor de IA**, e essa fronteira é o
que faz a Drop Zone funcionar com o Hermes desligado.

---

## 12. Limites desta versão

- **sem OCR.** Imagem e PDF escaneado registram que não têm texto;
- **sem leitura de DOCX, XLSX e ZIP.** Eles são guardados e reencontráveis pelo
  nome; ler o conteúdo exigiria descompactar e interpretar XML;
- **sem metadata remota de URL.** Nada de título, descrição ou favicon: seria um
  crawler, e o §12 do briefing pede que não seja;
- **o original não entra no `.mos-backup`.** O backup carrega o banco, como já
  fazia com o áudio das reuniões. `drops/` fica ao lado do banco, no diretório de
  dados, e uma cópia do perfil leva os dois;
- **relação Resource–Task não existe**, só Resource–Project e
  Resource–Workspace;
- **sem miniatura de imagem** na Library.
