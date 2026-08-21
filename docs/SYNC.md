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

Sincronizar em: abertura, voltar para o primeiro plano, reconectar rede, depois
de mutação (com *debounce*), oportunidade de background, sinal de push, refresh
manual.

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

---

## 11. O que falta

- **Transporte e servidor.** Não existem.
- **Auth.** Não existe.
- **Alimentar a fila.** As tabelas existem; nenhum repositório escreve nelas.
  Ligar isso é a Fase 2, e é onde `mos-core` vai emitir operações junto com cada
  mutação.
- **Arquivos binários.** Resources com PDF, imagem e áudio não sincronizam como
  blob dentro de JSON (§44). Metadado e binário são camadas separadas, com
  upload, download, cache e checksum próprios. Nada disso está feito.
