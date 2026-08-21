# Weekly Review — o fecho da semana, e ele não é um placar — Design

**Status:** aprovado para plano de implementação

**Data:** 2026-08-21

**Baseline:** M/OS `v0.3.0` no commit `e0db543`, que entregou a Daily Session. Domínio em `crates/mos-core/src/daily.rs`, persistência em `crates/mos-storage-sqlite/src/daily_repository.rs` e migration `0028`, interface em `apps/desktop/src/DailySession.tsx` e `DailyFlows.tsx`.

**Origem:** o §29 do pedido da Daily Session mandou *preparar* o modelo para a revisão semanal sem construí-la, e `docs/DAILY-SESSION.md` §11 registrou a tela como pendência explícita. Quatro decisões de desenho foram tomadas pelo proprietário em conversa, e estão marcadas onde aparecem.

**Não revisa nenhuma ADR.** Ele consome o que a ADR-054 criou.

---

## 1. Objetivo

Fechar a semana em menos de dois minutos, e sair sabendo **o que dominou, o que voltou toda vez e o que você largou** — sem que a tela emita julgamento sobre a semana.

---

## 2. Duas coisas chamadas "Weekly Review", e esta é uma delas

`IDEAS.md` #56 descreve um retrato de estado: Projects ativos, Tasks concluídas, Tasks paradas, Inbox, próximas prioridades. O §29 da Daily Session descreve outra coisa: uma retrospectiva sobre as sessões diárias — planejado contra concluído, carry-overs repetidos, Projects dominantes, dias travados.

**Este desenho é o segundo.** Ele lê Daily Sessions e nada mais. O #56 continua existindo como ideia, e o dia em que ele for construído não tem por que reaproveitar esta tela: ele responde "onde eu estou", e esta responde "como foi".

---

## 3. A restrição que manda no conteúdo

`ATTENTION-SYSTEM.md` §19 é explícito sobre digests semanais: **sem gamificação** — nenhuma sequência, nenhuma medalha, nenhuma comparação com ontem — e *"digest não é resumo de produtividade"*. O `VISION.md` §14 diz a mesma coisa por outro lado: o M/OS existe para reduzir carga mental, não para criar uma nova.

A coisa mais óbvia de mostrar numa semana é `12 de 20 objetivos`. Ela é proibida aqui, e a proibição não é cerimônia: um número que some sete dias de decisões numa fração ensina a inflar o denominador nas segundas e a evitar objetivos difíceis nas quintas. O placar mede o planejamento, não o trabalho.

**Decisão do proprietário: narrativa, sem placar.** Número aparece só quando **ele é o assunto** — *"carregado 4 vezes"* informa uma decisão que precisa ser tomada; *"12 de 20"* não informa nada que a pessoa já não saiba, e julga.

A única contagem que sobrevive é `5 dias com sessão`, e ela é um fato sobre o uso do sistema, não sobre o trabalho — a mesma distinção que o §19 faz entre entrega e produtividade.

---

## 4. Escopo

**Dentro:**

- `crates/mos-core/src/weekly.rs` — `Week`, `WeeklyReview`, `WeekSummary` e `compose_week`, tudo puro;
- migration `0029_weekly_review.sql` — uma tabela, sem tocar em nenhuma existente;
- leitura e escrita no `daily_repository.rs`, com emissão de sync;
- os métodos da semana no `DailyService`, e não um serviço novo (§11);
- terceira aba na gaveta da sessão, com navegação `‹ ›` entre semanas;
- a linha discreta na Home, reusando `.daily-stale`;
- `apps/desktop/src/weekly.ts` — apresentação pura, com teste;
- correção do N+1 de reflexões no `history()` (§9).

**Fora, e cada um por um motivo:**

- **ações do Hermes** — fechar a semana não é gesto que se peça por voz, e cinco linhas a mais no catálogo custam token em toda mensagem do chat;
- **entrada no Command palette** — a linha na Home e a aba resolvem a descoberta; um comando para uma tela que aparece sozinha é caminho redundante;
- **notificações** — mesma decisão do §8 do `DAILY-SESSION.md`: a infraestrutura existe, a decisão de o M/OS cobrar a pessoa não foi tomada;
- **humor da semana** — os sete dias já responderam isso (§7);
- **qualquer escrita sobre objetivos** — o fecho da semana é registro, e não decisão sobre o que vem (§8);
- **`IDEAS.md` #56** — ver §2.

---

## 5. `Week` é a segunda-feira, e não o número ISO

```rust
pub struct Week(Day);          // sempre uma segunda-feira
Week::containing(&Day) -> Week
Week::previous(&self) -> Week
Week::next(&self) -> Week
Week::range(&self) -> (Day, Day)   // segunda, domingo
```

Número ISO tem duas armadilhas que a data da segunda simplesmente não tem: **semanas 53**, e o 1º de janeiro que pertence à semana 52 do ano anterior. Guardar `2026-W01` obrigaria a escolher uma convenção de virada de ano e a acertá-la em todo lugar que compara; guardar `2026-08-17` não obriga a nada.

`Week::containing` é a **única cópia** dessa regra. Nada de `date(day, 'weekday 0', '-6 days')` em SQL — ver §9.

A semana é **civil e fixa**, e não uma janela deslizante de sete dias: um "fecho" de janela deslizante não fecha nada, e a unicidade precisa de uma chave. É a mesma razão pela qual `Day` existe como campo em vez de ser decidido por cada leitor.

---

## 6. A narrativa é derivada, e a entidade é minúscula

**Nada do que a tela mostra é guardado.** A entidade tem um texto e uma data:

```rust
pub struct WeeklyReview {
    id: WeeklyReviewId,
    week: Week,
    summary: String,
    closed_at: OffsetDateTime,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}
```

Guardar a narrativa seria duplicar dado para exibir noutra superfície, que o `CORE-FOUNDATION.md` §2 princípio 6 proíbe — e ela envelheceria: reabrir um objetivo de terça mudaria a semana, e o resumo gravado continuaria dizendo o contrário.

O resto sai de uma função pura, no mesmo desenho do `calendar::compose` e do `daily::compose_context`:

```rust
pub struct WeekInput<'a> {
    pub week: Week,
    pub sessions: &'a [DailySession],
    pub objectives: &'a [DailyObjective],
    pub reflections: &'a [DailyReflection],
    /// Como achar o Project de um vínculo. Fechamento e não mapa pronto,
    /// porque só o comando do desktop conhece Tasks e Projects.
    pub project_of: &'a dyn Fn(&ObjectiveLink) -> Option<String>,
    pub carry_depth: &'a dyn Fn(DailyObjectiveId) -> usize,
}

pub struct WeekSummary {
    pub week: Week,
    pub days_with_session: usize,
    pub dominated: Vec<Dominant>,      // { label, main_days, days }
    pub recurring: Vec<Recurring>,     // { title, times_carried }
    pub dropped: Vec<String>,
    pub blocked_days: Vec<Day>,
    pub review: Option<WeeklyReview>,
    /// Nenhuma sessão na semana. A tela usa isto para NÃO oferecer o fecho.
    pub empty: bool,
}
```

### 6.1 "O que dominou" agrupa por Project **ou** por título

O agrupamento óbvio é por Project. Ele falha em silêncio no caso mais comum do início: uma semana inteira de objetivos em texto livre mostraria a seção vazia — e essa é justamente a única seção que responde *"onde foi meu tempo"*.

Então a chave de agrupamento é: **o Project quando o vínculo resolve, e o título normalizado quando não resolve.** Um objetivo ligado a uma Task resolve pelo Project da Task; ligado a um Project, por ele mesmo; livre, por si.

`main_days` conta os dias em que aquilo foi o **principal**, e `days` o total de aparições. Ordena por `main_days`, depois `days`. Ser principal três vezes numa semana é um fato mais forte que aparecer cinco vezes como secundário.

### 6.2 "O que voltou toda vez" segue a corrente, não o título

Entram os objetivos da semana com `carry_depth >= 2`. O corte em dois é o mesmo do `avisoDeCarregado` no front, e pela mesma razão: quase todo carry-over veio de ontem, e *"veio de ontem"* é ruído.

Uma corrente que atravessa a semana inteira aparece **uma vez**, com a profundidade do elo mais recente — cinco linhas iguais seriam a mesma informação repetida cinco vezes.

### 6.3 "Dias travados" vem dos humores que já foram respondidos

Os dias com `mood = blocked`. **Não é uma pergunta nova.** Perguntar o humor da semana no domingo seria pedir a mesma coisa uma oitava vez, com menos precisão do que as sete respostas que já existem.

---

## 7. O que o fecho guarda: um texto

**Decisão do proprietário.** Uma pergunta — *"Como foi a semana?"* — e um campo.

Sem humor, pela razão do §6.3. Sem os três campos guiados de "o que funcionou / o que travou / o que muda": isso é o formulário de journaling que o pedido da Daily Session recusou por nome, e ele transforma dois minutos em dez.

Texto vazio **não impede o fecho**: fechar a semana é o gesto, e escrever é opcional. Isso difere do `NewDailyReflection::create`, que devolve `None` quando não há nada a guardar — lá a reflexão é um acessório do encerramento; aqui ela é o único campo, e a linha precisa existir para a semana constar como fechada.

---

## 8. O fecho é registro, e não decisão

Encerrar a semana **não toca em objetivo nenhum**. Não resolve pendentes, não larga carry-overs crônicos, não cria objetivos para a semana seguinte.

O proprietário escolheu "fecho da semana" contra "preparação da próxima", e a fronteira importa: o Start My Day já pergunta sobre os carry-overs todo dia, e uma segunda superfície decidindo o destino dos mesmos objetivos criaria dois lugares onde a mesma escolha é feita — com resultados possivelmente diferentes na mesma manhã.

---

## 9. Persistência

```sql
CREATE TABLE weekly_reviews (
    id          TEXT PRIMARY KEY NOT NULL,
    week_start  TEXT NOT NULL,          -- segunda-feira, AAAA-MM-DD
    summary     TEXT NOT NULL DEFAULT '',
    closed_at   TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    CONSTRAINT weekly_reviews_week_shape CHECK (
        week_start GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
    )
);
CREATE UNIQUE INDEX weekly_reviews_one_per_week ON weekly_reviews (week_start);
```

Uma tabela nova, **nenhuma alteração em tabela existente** — a mesma regra da 0027 e da 0028.

O `CHECK` de formato existe pelo motivo que a 0028 registrou: `2026-8-17` e `2026-08-17` são a mesma data e duas chaves diferentes, e o índice único não veria a duplicata.

**O CHECK não verifica que é segunda-feira.** SQLite conseguiria (`strftime('%w', week_start) = '1'`), e isso seria a regra da semana escrita num segundo lugar. Quem garante é `Week`, que é o único construtor.

### 9.1 Sync

Emite `weekly_review`, com `weekStart`, `summary` e `closedAt`. Merge por campo, como o resto. Editar o texto no PC e no celular no mesmo minuto é conflito legítimo, e o perdedor vai para `sync_conflicts` como qualquer outro.

### 9.2 A semana pendente é calculada no serviço

O gatilho da Home precisa de: *a semana mais recente, anterior à atual, que teve pelo menos uma sessão e não tem fecho.*

Daria para resolver em SQL com `date(day, 'weekday 0', '-6 days')`. **Não vamos.** Seria a regra da semana em dois lugares, e é assim que o `arrange_widgets` do Rust ficou para trás em silêncio — com os testes dele passando, que é o pior jeito de ficar para trás.

```rust
pub fn pending_week(&self, current: &Week) -> Result<Option<Week>, CoreError> {
    // 120 sessões ≈ quatro meses de uso diário. Além disso, uma semana não
    // fechada deixou de ser pendência e virou histórico.
    let sessions = self.repository.sessions(120)?;
    let fechadas: HashSet<Week> = self.repository.weekly_reviews(60)?...;
    sessions.iter()
        .map(|sessao| Week::containing(&sessao.day))
        .filter(|semana| *semana < *current && !fechadas.contains(semana))
        .max()
}
```

`sessions(limit)` já existe e já é barata — o teto de 365 linhas curtas foi dimensionado no commit anterior.

### 9.3 O N+1 que este desenho paga

`DailyService::history()` lê a reflexão de cada sessão numa consulta por dia. A semana precisa de sete de uma vez, e a correção serve as duas: `reflections_of(&[DailySessionId]) -> Vec<DailyReflection>`, no mesmo desenho do `objectives_of` que já existe.

É melhoria dirigida ao que este trabalho toca, e não refatoração oportunista.

---

## 10. Interface

### 10.1 A aba

A gaveta da sessão já tem `Sessão | Histórico`. Entra `Semana`, com `‹ ›` para andar — assim o histórico de semanas fica alcançável **sem uma segunda lista**, e a aba de Histórico continua sendo só dos dias.

A aba abre na **semana pendente**, quando há uma; senão, na semana corrente. Abrir sempre na corrente faria a linha da Home levar a uma tela que não é a que ela anunciou.

```
SEMANA DE 18 A 24 DE AGOSTO           ‹  ›

5 dias com sessão

O QUE DOMINOU
  063-26            principal em 3 dias
  Hermes            principal em 1 dia

O QUE VOLTOU TODA VEZ
  Atualizar documentação    carregado 4 vezes

O QUE VOCÊ LARGOU
  Revisar proposta antiga

DIAS TRAVADOS
  qua, qui

COMO FOI A SEMANA?
  [                                    ]

                        [ Encerrar a semana ]
```

Seção sem conteúdo **não desenha o rótulo**. Uma semana sem nada largado não deve mostrar "O QUE VOCÊ LARGOU" seguido de vazio — é a mesma regra do `resumoDoDia`, onde zero não vira linha.

Semana já fechada mostra o texto salvo e o botão vira `Salvar`. **Não existe "reabrir semana":** ela não tem estado a reabrir — é um registro, e editar o texto é a única mudança possível.

### 10.2 A linha na Home

Reusa `.daily-stale`, o mesmo componente e o mesmo estilo que hoje dizem *"você ainda não encerrou 20/08"*:

> A semana de 18–24 acabou.  `[Encerrar]`

Aparece **só quando há semana pendente**, e some assim que ela é fechada. Ela informa e oferece; não bloqueia nada, e não vira badge nem contador.

### 10.3 Semana sem sessão nenhuma não oferece fecho

`WeekSummary::empty` existe para isto. Não há o que revisar, e um botão ali ensinaria que o M/OS quer um registro por semana mesmo quando não houve semana — que é exatamente a carga de organização que o `VISION.md` §14 proíbe criar.

A `pending_week` já filtra por "teve pelo menos uma sessão", então a linha da Home nunca aponta para uma semana vazia. O `empty` cobre o caso de a pessoa navegar até lá com o `‹`.

---

## 11. Os métodos da semana ficam no `DailyService`

Não haverá `WeeklyService`. A semana lê sessões, objetivos e reflexões — os três repositórios que o `DailyService` já tem —, e `pending_week` depende de `sessions()`, que é dele. Um serviço novo que dependesse do mesmo repositório para responder outra pergunta seria fronteira sem substância, e mais uma coisa a fiar no `AppState`.

Entram cinco métodos: `week(&Week)`, `pending_week(&Week)`, `close_week(&Week, &str)`, `weekly_reviews(limit)` e o `reflections_of` do §9.3. Se o arquivo do serviço passar a incomodar, o corte certo é por entidade e não por tela — e aí o `DailyService` inteiro sai do `service.rs`, o que é outro trabalho.

---

## 12. Testes

**Domínio** — `Week::containing` numa segunda, num domingo e na virada de ano; `previous`/`next` atravessando mês e ano bissexto; `compose_week` com: agrupamento por Project, agrupamento por título quando não há vínculo, corrente longa aparecendo uma vez só, corte em `carry_depth >= 2`, dias travados vindos dos humores, semana vazia marcando `empty`.

**Banco** — unicidade por semana; upsert do texto preservando `closed_at`; `sessions_between` nas bordas — a comparação é entre datas civis (`day >= segunda AND day <= domingo`), e os dois extremos entram; emissão de `weekly_review`; nada emitido com sync desligado.

**Serviço** — `pending_week` ignorando a semana corrente, ignorando semana já fechada, ignorando semana sem sessão, e devolvendo a mais recente entre duas candidatas.

**Front puro** — o rótulo "18 a 24 de agosto" atravessando mês ("30 de setembro a 6 de outubro") e ano; seção vazia não virando rótulo; a aba escolhendo semana pendente contra corrente.

**Não haverá teste de componente**: não há DOM no runner (`vitest.config.ts`), e é por isso que `weekly.ts` existe.

---

## 13. Riscos

**O agrupamento por título normalizado pode colar coisas diferentes.** "Revisar memorial" numa segunda e "Revisar memorial do 063-26" numa quarta são dois títulos e uma intenção — ou duas. A normalização será conservadora (caixa e acento, nada de fuzzy), aceitando separar o que talvez fosse junto. Errar para o lado de separar mostra duas linhas verdadeiras; errar para o lado de juntar inventa uma dominância que não houve.

**A semana pendente pode aparecer atrasada.** Quem passa duas semanas sem abrir o M/OS vê a linha apontando para a semana retrasada. É o comportamento correto — ela é a mais recente **não fechada** —, mas o rótulo precisa dizer a data, e não "a semana passada".
