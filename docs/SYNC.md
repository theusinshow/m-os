# SYNC — Como duas histórias viram uma

Implementação: `crates/mos-sync`. Tabelas: migration `0027_sync_foundation.sql`.

---

## 1. A premissa

Cada dispositivo tem um banco local completo e **autoridade sobre si mesmo**. O
servidor, quando existir, coordena e persiste — ele não transforma cliente em
terminal burro (§67). Toda operação do usuário grava local e atualiza a tela
antes de qualquer rede.

```
ação
 ↓
grava local          ← já pode aparecer na tela
 ↓
enfileira a operação
 ↓
sincroniza quando der ← e não quando o usuário espera
```

Se a rede nunca voltar, o M/OS continua sendo um sistema inteiro. Isso não é
tolerância a falha: é a ADR-002, que já dizia que o núcleo local é autoritativo.

---

## 2. O relógio

**Não usamos timestamp de parede para ordenar eventos.** Dois dispositivos não
concordam sobre que horas são — o relógio do celular anda, o do PC atrasa, NTP
corrige, horário de verão vira. Ordenar por parede significa que *um relógio
errado apaga o trabalho de quem estava certo*.

Usamos **HLC** (Hybrid Logical Clock): parede + contador + dispositivo.

- A parede mantém o número legível e próximo do tempo real — é o que faz a
  Timeline (§25) ter sentido.
- O contador resolve empate no mesmo milissegundo, e segura a ordem quando a
  parede volta.
- O dispositivo desempata o resto, **deterministicamente**: os dois lados
  chegam à mesma ordem sem se falarem.

Ao receber um evento do futuro, o relógio local sobe junto. Sem isso, um celular
atrasado geraria eventos que se ordenam antes de coisas que ele acabou de
receber e já mostrou na tela.

O último instante emitido é persistido (`sync_clock`). Reabrir o app com a
parede atrasada não pode gerar eventos no passado.

---

## 3. O que viaja

A **mudança de campo**, não a entidade inteira.

Se o celular mandasse a Task inteira e o PC mandasse a Task inteira, a única
reconciliação possível seria escolher uma — e a outra sumiria. É exatamente o
que o §8 proíbe.

```rust
Op {
  id:     Uuid,        // chave de idempotência, nasce na origem
  entity: EntityRef,   // { kind: "task", id }
  body:   Create | Update { fields } | Delete | Restore,
  at:     Hlc,         // instante + dispositivo de origem
}
```

`id` é o que faz o retry aplicar uma vez só (§53), e é o mesmo id que as ações
do Hermes usam (§78).

`Delete` é sempre lógico. Uma linha ausente é indistinguível de uma linha que
nunca chegou — o dispositivo que estava offline precisa *saber* que algo foi
apagado.

---

## 4. Reconciliação

**Campos diferentes convivem. O mesmo campo escrito duas vezes é conflito.**

| Situação | Resultado |
| --- | --- |
| PC muda `title`, iPhone muda `due_at` | ambos aplicam, sem conflito |
| Os dois mudam `title` com valores diferentes | mais recente vence, **perdedor guardado** |
| Os dois escrevem o mesmo valor | não é conflito |
| Apagado num, editado no outro | fica apagado |
| `Restore` mais velho que o `Delete` | não desfaz |

O ponto que importa: quando o LWW por campo decide, **o valor perdedor não é
descartado**. Ele vai para `sync_conflicts`, com os dois lados e os dois
dispositivos. É isso que separa *resolver o conflito* de *escolher um e apagar
o outro em silêncio*.

O apagamento ganhar da edição é assimetria deliberada: restaurar o que sumiu por
engano custa um clique; descobrir semanas depois que algo voltou sozinho custa a
confiança no sistema.

### A ordem de chegada não importa

A rede entrega fora de ordem, o retry reenvia o que já passou, um lote pode vir
de três dispositivos. `aplicar()` ordena pelo instante antes de aplicar — dois
aparelhos que receberam o mesmo conjunto chegam ao mesmo estado, na ordem que
for. Sem isso, duas máquinas com os mesmos dados mostrariam telas diferentes.

---

## 5. Idempotência

Reaplicar uma operação já aplicada não muda nada: ela perde do próprio valor que
colocou lá, porque a comparação é estritamente *maior que*. Um retry não pode
duplicar Task, Reminder, Capture nem Resource — e o teste
`cenario_5_reaplicar_nao_duplica_nem_muda_nada` prova isso com dez cópias.

---

## 5.1 O que atravessa, e o que fica

Desde 31/08/2026 a resposta nao e mais "os tipos que alguem lembrou de ligar".
**Toda tabela do esquema esta classificada**, em `sync_cobertura.rs`, e um teste
recusa tabela nova sem decisao. As duas listas sao `SINCRONIZAVEIS` e `LOCAIS`,
e a segunda exige o motivo escrito ao lado.

Isso existe por causa de um defeito real: o CronoCAD gravava horas desde sempre
e **nunca emitiu uma operacao**. Nao deu erro, nao apareceu em log, nao quebrou
teste — vinte e duas horas de trabalho existiam num PC e nao no outro. A causa
nao foi distracao: criar uma tabela e escrever nela sao dois atos completos por
si, e sincroniza-la era um terceiro que ninguem era forcado a considerar.

Nao ha lista certa. `usage_requisicao` fora do sync esta tao correto quanto
`time_entries` dentro. O que estava errado era nao ter escolhido.

Fica de fora, em resumo: a maquinaria do proprio sync; a telemetria de uso e de
atividade; o que descreve a maquina (apps monitorados, cronometro em curso,
deteccao de ociosidade); o que guarda ARQUIVO em disco, porque o hub carrega
campos e nao blobs (reunioes, notas de voz, ingestoes); a sessao do provedor
academico; e o arranjo de tela, porque os dois monitores sao diferentes.

## 5.2 O backfill

A operacao so nasce na mesma transacao da mudanca. Isso significa que **ligar a
sincronizacao nao move nada do que ja estava no banco** — um M/OS com historico
mandaria para o outro aparelho apenas o que fosse tocado dali em diante.

`sync_backfill.rs` faz a passagem unica. Ele nao e uma migration: precisa do
HLC, e o HLC so existe depois de `habilitar_sync` — antes disso o dispositivo
nem tem identidade. Roda logo depois dela, no desktop e no `mos-web`, e falhar
nao impede o app de abrir.

Le as colunas pelo proprio `Mapa` da projecao, e nao por uma lista propria: o
que o backfill manda e exatamente o que a emissao mandaria. A ordem e de
dependencia — `project_tracking` antes de `project` bateria na chave
estrangeira.

**Ele nao funde duplicata.** Se a mesma coisa foi criada nos dois PCs de forma
independente, sao duas entidades com ids diferentes, e as duas aparecem. O sync
nao tem como adivinhar que sao a mesma.

## 6. As tabelas

| Tabela | Para quê |
| --- | --- |
| `devices` | quem é cada instalação; `is_this_device` responde "quem sou eu" |
| `sync_outbox` | o que este dispositivo mudou e ainda não confirmou |
| `sync_conflicts` | escritas concorrentes, com o lado perdedor guardado |
| `sync_clock` | o HLC entre execuções, e o cursor do pull |

**A migration 0027 não altera nenhuma tabela existente.** Nem uma coluna nova em
`tasks`, `captures` ou `projects`. O desktop tem dados de verdade e não pode
regredir por uma feature que ainda não tem cliente do outro lado (§62, §75). Se
o desenho do sync mudar, essas quatro tabelas se apagam sem tocar no que existe.

---

## 7. Sync incremental

O custo cresce com **o que mudou**, nunca com o tamanho da base (§43).

`sync_clock.pull_cursor` guarda até onde este dispositivo já puxou. Vazio
significa "nunca puxou" — é o que dispara a sincronização inicial em vez da
incremental. Baixar o banco inteiro a cada abertura está proibido.

---

## 8. Gatilhos

No desktop, desde 28/08, a rodada é **automática**. Quatro gatilhos, em
`apps/desktop/src-tauri/src/sync.rs`:

| Gatilho | Quando | Como |
| --- | --- | --- |
| abertura | uma vez, **depois** que o app terminou de abrir | o renderer chama `sync_app_pronto` ao chegar em `ready`; teto de 30s |
| primeiro plano | a janela voltou | `reveal_window` acorda o laço |
| pós-mutação | a fila cresceu | o daemon **ouve** `data-changed`, com *debounce* de 10s |
| rede de segurança | a cada 15 min, se nada mais acordou | só existe para a rede que voltou sozinha |

**A primeira rodada espera o app abrir, e isso não é otimização.**
`sincronizar_agora` segura o mutex do relógio a rodada inteira, de propósito
(§11). Com fila grande, rodar junto com a abertura seguraria o banco durante a
rajada de IPC do boot, e a webview gastaria as 12 tentativas dela contra um
banco ocupado — parando na tela de erro que diz, mentindo, que os dados não
abriram com segurança.

**A mutação é ouvida, e não emitida.** `data-changed` já sai de 25 lugares;
acrescentar um aviso em cada um seriam 25 chances de esquecer um — e o esquecido
não daria erro, daria uma entidade que só sai deste aparelho no próximo quarto
de hora.

Ainda **não** existem: sinal de push do hub para o desktop, e reconexão de rede
como gatilho. Os dois estão cobertos, com atraso, pela rede de segurança.

**Nunca polling agressivo** (§51). No iPhone isso é bateria; em qualquer lugar é
requisição sem motivo. O sistema é orientado a evento.

O iOS suspende o app — o desenho tem que estar correto **assumindo** que ele
será suspenso no meio de qualquer coisa. É para isso que a fila é persistente e
as operações são idempotentes: retomar é reenviar o que não foi confirmado.

---

## 9. Contrato e versões

`CONTRACT_VERSION` versiona **o formato que viaja**, não o app. Desktop 0.9 e
iPhone 0.7 falam contrato 1 sem problema — e precisam, porque a App Store não
publica quando o desktop publica (§27, §73, §74).

Duas escolhas específicas existem para isso:

- `EntityKind` é **texto**, não enum fechado. Um cliente antigo precisa
  *guardar e reenviar* uma operação sobre um tipo que ele ainda não conhece. Com
  enum, "tipo desconhecido" viraria erro de desserialização e a operação morreria
  no cliente velho.
- `Platform::Outra(String)` pelo mesmo motivo, na lista de dispositivos.

---

## 10. Observabilidade

Eventos a registrar (§39), sem dado pessoal: `sync_started`, `sync_completed`,
`entity_pushed`, `entity_pulled`, `conflict_found`, `conflict_resolved`,
`retry`, `auth_expired`, `migration`.

Estados que a interface precisa saber representar (§40): sincronizado,
sincronizando, offline, alterações pendentes, erro. Não precisa estar sempre à
vista — precisa ser descobrível quando algo está errado.

**São seis, e não cinco.** Falta **desligado**: sem endereço ou sem segredo, e é
o estado em que a feature nasce em toda máquina nova. Ele é o único que **não
vira aviso**: quem não ligou o sync não tem um problema, tem uma feature
desligada, e um aviso diário na Home sobre isso seria propaganda dentro do
próprio produto. Ele vive só no Settings.

No desktop, a regra de qual estado desenhar é função pura em
`apps/desktop/src/syncFaixa.ts`, e a ordem das perguntas é o desenho: desligado
sai calado; a notícia ganha do erro (a rodada que trouxe coisa funcionou); o
erro ganha da fila (a fila é consequência, o erro é a causa). *Erro* e
*pendente* **não se dispensam** — um aviso que se pode calar sem consertar a
causa é um aviso que se cala sempre.

---

## 11. O motor

`sincronizar()` faz uma rodada: empurra a fila, puxa o que mudou desde o cursor,
reconcilia por entidade, grava conflitos e persiste o relógio.

Duas fronteiras mantêm o motor livre de plataforma:

- **`Transport`** — como falar com o outro lado. Existe como trait porque o
  transporte real ainda não existe, e o motor precisa ser exercitável inteiro
  sem rede. Quando um servidor existir, ele implementa isto e nada no motor
  muda.
- **`Projecao`** — como uma operação vira entidade. O motor sabe reconciliar,
  mas não sabe o que é uma Task. É essa fronteira que permite acrescentar um
  tipo novo sem tocar no motor.

Garantias, cada uma com teste:

| Garantia | Como |
| --- | --- |
| Nunca perde operação local | sai da fila quando o outro lado **confirma**, não quando é enviada |
| Nunca duplica | chave de idempotência nasce na origem; `push` é repetível |
| Custo proporcional ao que mudou | `pull` leva cursor |
| Ordem de chegada não importa | quem decide é o instante da operação |
| Falha parcial não perde o feito | o que já foi aplicado permanece |

### A prova

`crates/mos-storage-sqlite/tests/sync_two_devices.rs` — **dois bancos SQLite de
verdade fazendo papel de dois dispositivos**, com identidades e relógios
próprios, contra um hub em memória.

O iPhone não existe (compilar para iOS exige Mac). Mas o laço não depende de
qual é a plataforma: depende de dois bancos, dois relógios e um transporte. O
que se prova é exatamente a parte que poderia estar errada.

Nove testes, cobrindo os passos do §80 que não dependem de hardware: criar no PC
e aparecer no outro, editar no outro e voltar, dez capturas offline chegando
depois de reconectar, campos diferentes convivendo, mesmo campo guardando o
perdedor, sincronizar de novo sem duplicar, relógio sobrevivendo ao fechamento
do app, cursor fazendo o custo crescer só com o que mudou, e apagar num lado
apagando no outro.

O `HubLocal` do teste é o menor que satisfaz o contrato — e por isso vale como
**especificação executável do servidor**: guardar em ordem, devolver a partir de
um cursor, aceitar reenvio sem duplicar.

---

## 12. Emissão: a operação entra na mesma transação

Uma regra, e ela não é detalhe de implementação:

> **A operação é gravada na mesma transação da mudança.**

Sem isso existem dois modos de falhar em silêncio:

- Gravar a Capture e falhar ao enfileirar deixa uma Capture que **nunca sai
  deste dispositivo**, e ninguém fica sabendo.
- Enfileirar e falhar ao gravar manda para o outro lado uma mudança que **não
  aconteceu aqui**.

Uma transação torna os dois impossíveis. O relógio entra junto pelo mesmo
motivo: se a operação commita e o relógio não, reabrir o app reemitiria aquele
instante para outra operação — e duas operações com o mesmo instante e o mesmo
dispositivo quebram a ordem total.

`habilitar_sync` nunca chamado significa nenhuma emissão, e **nenhuma mutação
falha por causa disso**. É o que permite ligar por entidade sem parar o desktop.

Duas distinções que decidiram o desenho.

**A intenção viaja; a entrega fica.** Um lembrete emite título, gatilho, prazo e
status. Não emite `deliveredCount`, que conta quantas vezes *este* dispositivo
mostrou o aviso — o iPhone tocar não significa que o PC tocou, e sincronizar
esse número faria dois aparelhos disputarem um contador que nem descreve a mesma
coisa. `snoozeCount` viaja, porque adiar é ação da pessoa e não do aparelho.

**Arquivar é mudança de campo, não `OpBody::Delete`.**
O `Delete` é para a exclusão definitiva, a que o M/OS só aceita depois de
arquivar. Se arquivar virasse `Delete`, a regra de "apagar ganha" faria o
arquivamento vencer a restauração para sempre — e a Capture nunca mais voltaria.

---

## 13. Relações: o Knowledge Graph

`resource_projects`, `resource_workspaces` e `project_workspaces` são junções:
duas colunas, sem id próprio. Sincronizá-las como campo de uma das pontas não
funciona — **merge por campo não serve para conjunto**. Se a lista de Projects
de um Resource fosse um campo, ligar a um Project no celular apagaria a ligação
feita no PC, porque o campo inteiro seria substituído pelo mais recente.

O §24 pede relação como entidade de primeira classe. Duas decisões fazem isso
funcionar:

**1. O id é derivado do par, não sorteado.** UUID v5 sobre
`{tipo}:{de}:{para}`, num namespace fixo. Os dois dispositivos calculam o mesmo
id sem se falarem. Se cada um sorteasse, ligar o mesmo Resource ao mesmo Project
nos dois criaria **duas relações para o mesmo vínculo** — e desfazer uma
deixaria a outra de pé.

O tipo e a direção fazem parte da identidade: `A→B` não é `B→A`, e ligar as
mesmas duas pontas por dois motivos diferentes são dois vínculos.

**2. Desligar é campo, não `OpBody::Delete`.** O `Delete` tem semântica de
"apagar ganha de editar", que está certa para uma Task e **errada para um
interruptor**: desvincular às 10:00 e revincular às 10:05 tem que terminar
vinculado. A relação nunca é apagada — ela tem um campo `linked`, e o merge por
campo decide pelo instante. Último gesto vence.

Os campos de identificação (`kind`, `from`, `to`) viajam junto com `linked`,
porque um dispositivo que recebe a operação sem nunca ter visto a relação
precisa saber **o que** foi ligado: o id é um hash e não diz nada.

O namespace é constante e **nunca muda**: mudá-lo faria todas as relações
existentes ganharem ids novos, e as antigas ficariam órfãs. Há um teste que
trava o valor.

---

## 14. O que falta

- **Emitir operações nas outras entidades.** Já emitem: Captures, Tasks,
  Projects, Reminders, Resources e a Daily Session (`daily_session`,
  `daily_objective`, `daily_reflection` — ver `DAILY-SESSION.md`), o fecho da
  semana (`weekly_review`) e o M/Academic (`academic_semester`,
  `academic_subject`, `academic_assignment`, `academic_exam`,
  `academic_study_session` — ver `ACADEMIC.md`) e Workspaces. Faltam
  Calendar, Meetings, Conversations, Tracking, Apps e Voice. Nenhum deles
  bloqueia aresta do Knowledge Graph — Workspace bloqueava, e foi por isso que
  ele veio antes.
- **O arquivo dos Resources.** Só o metadado viaja. PDF, imagem e áudio são
  outra camada (§44), com upload, download, cache e checksum. Não existe.
- **Arquivos binários.** Resources com PDF, imagem e áudio não sincronizam como
  blob dentro de JSON (§44). Metadado e binário são camadas separadas, com
  upload, download, cache e checksum próprios. Nada disso está feito.

## Reparo e prova (V2, fase 1)

Duas coisas que a V1 não tinha, e cuja falta custou uma manhã de investigação em
02/09/2026.

**A fila de pendentes é uma tabela.** Uma entidade que chega e não vira linha
fica em `sync_pendentes`, e a **abertura do app** varre `sync_state` procurando
sombra sem linha, materializando o que achar. Antes, a fila era um `Vec` em
memória e o cursor avançava assim mesmo: a entidade ficava viva no banco de
sincronização, invisível na tela, e nada dizia isso. A varredura olha o banco e
não a fila — é por isso que ela conserta também os bancos que já estão nesse
estado, sem ninguém rodar diagnóstico.

**O manifesto prova a igualdade.** Cada aparelho manda, por família, contagem e
hash — calculados sobre `sync_state`, e **nunca** sobre as tabelas de domínio,
que carregam o `updated_at` de quem aplicou. O hub guarda o retrato de cada um,
trocado inteiro a cada batida, e a tela responde `alinhado`, `atrás` ou
`divergente`, com reparo oferecido sempre.

Aparelho sem manifesto não é acusado de nada: ele é um M/OS que ainda não
atualizou.

O que esta fase **não** faz: estado canônico no servidor, revisão-base por campo
e confirmação de aplicação. Isso é a fase 2, e o spec já está escrito.
