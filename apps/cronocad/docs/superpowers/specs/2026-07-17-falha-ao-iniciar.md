# Incidente: o app parou de abrir, sem dizer nada

Data: 2026-07-17
Status: resolvido

## Sintoma

O usuario ligou o computador e o CronoCAD nao abria. Clicar no icone nao
produzia nada: sem janela, sem erro, sem mensagem. Reinstalar nao resolveu.

Custo: **um dia inteiro de trabalho sem registro**. O usuario so recuperou as
horas depois, de memoria — a feature "adicionar tempo esquecido"
(`2026-07-16-tempo-esquecido-design.md`) nasceu deste incidente.

## Investigacao

O que fechou o caso foi **executar o exe pela linha de comando** e ler o stderr.
O erro estava na primeira linha, desde sempre:

```
thread 'main' panicked at src\lib.rs:156:
  PluginInitialization("sql", "migration 5 was previously applied but has been modified")
```

Exit code `-1073740791` (`0xC0000409`) — o abort do Rust com `panic = "abort"`.

Hipoteses descartadas antes (todas erradas, todas caras):
instalacao corrompida; antivirus; processo zumbi na bandeja bloqueando um
segundo processo.

## Causa raiz

Duas causas, uma dentro da outra.

### 1. Por que o boot falhava

Um `tauri:dev` aplicou a migration `0005_project_notes.sql` no **banco real** do
usuario em 14/07 16:57. Depois disso, o **bloco de comentarios** no topo do
arquivo mudou, antes do commit `b1f91a9`.

O `sqlx` calcula o checksum do **arquivo inteiro**, comentarios inclusive. Ele
via "a migration mudou" e recusava subir.

O schema estava **correto**: o DDL gravado no banco era identico, coluna por
coluna e indice por indice, ao que o arquivo atual produz. O app se recusou a
abrir por causa de um comentario SQL.

### 2. Por que custou um dia

`lib.rs` terminava em `.expect("erro ao iniciar a aplicacao CronoCAD")`. Sem
console, `expect` num app de janela nao mostra nada: o processo simplesmente
morre. O app tinha a resposta exata e nao a entregou a ninguem.

Esta e a causa que importa. A migration foi um acidente pontual; o silencio
transforma **qualquer** acidente futuro no mesmo dia perdido.

## Correcao

1. **Reparo do banco do usuario** (pontual): o checksum gravado da v5 foi
   atualizado para o do arquivo, apos verificar que o schema ja era equivalente.
   Sem tocar em schema nem em dados. Backup em
   `cronocad.sqlite.bak-antes-do-reparo`.
2. **Codigo** (duravel): `run()` deixa de usar `.expect` e passa a chamar
   `fail_visibly`, que mostra um **dialogo nativo** (`rfd`) com o motivo antes de
   encerrar.

Verificacao: com a migration alterada de proposito, o app passou a exibir
"CronoCAD — falha ao iniciar" com o motivo, em vez de morrer calado. Revertida a
migration, o app sobe normalmente com o banco real.

## Licoes

1. **Nunca editar uma migration ja aplicada — nem os comentarios.** A regra ja
   estava no `CLAUDE.md` ("Nunca edite uma migration ja aplicada em producao"),
   mas soava como "nao mude o DDL". O checksum do `sqlx` cobre o arquivo todo.
   Rodar `tauri:dev` **e** aplicar em producao: o dev e o app instalado usam o
   mesmo banco.
2. **Um app de registros nunca deve falhar em silencio.** A regra critica 1
   (confiabilidade dos registros) inclui o usuario **saber** quando o app nao
   esta gravando. `.expect` em codigo de janela e sempre um bug.
3. **Rodar o binario e ler o stderr vem antes de qualquer hipotese.** Todas as
   teorias levantadas antes disso estavam erradas, e o erro estava na primeira
   linha da saida.
