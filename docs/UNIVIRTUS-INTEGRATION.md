# UNIVIRTUS — a investigação da integração com o M/Academic

Investigação de 2026-08-22, sobre a conta autenticada de `univirtus.uninter.com`.
Nada foi alterado no portal: todas as chamadas desta investigação foram GET.
Protótipo: `scripts/univirtus-investigation/probe-browser.js`.

---

## 1. Resultado geral: **Good**

O Univirtus não precisa ser raspado. O AVA é um SPA Backbone que consome uma
**API REST JSON própria**, e essa API entrega disciplina, semestre, prazo, nota,
peso e material como dados — não como HTML. É o primeiro item da prioridade do
§6 do pedido, e não o último.

Não é **Excellent** por três motivos concretos, todos medidos:

- **não existe endpoint consolidado.** O `#/ava/calendario` do próprio portal
  abre vazio, e não dispara chamada nenhuma. Prazo se monta disciplina por
  disciplina, com duas chamadas cada;
- **a sessão não se renova sozinha.** O que autentica é um segredo de sessão
  (`X-time`) emitido no login, e não há refresh;
- **professor não existe no dado.** `salaVirtual.nomeProfessor` vem `null`.

---

## 2. Autenticação

### O fluxo

```
POST  /ava/web/            (RU + senha, formulário próprio)
  ↓
escolha de ambiente        #/ava · #/ava/provas · #/ava/financeiro · #/ava/servico
  ↓
#/ava                      SPA autenticado
```

Não há MFA. Há um `recaptcha__pt_br.js` carregado na página de login, mas ele
não foi exigido no login normal. Não há redirect para IdP externo: o login é do
próprio domínio.

### O que autentica uma chamada de API

Duas coisas, e só elas:

| Peça | Onde vive | Observação |
| --- | --- | --- |
| cookie de sessão | `HttpOnly` | invisível ao JS; `document.cookie` só mostra `OptanonConsent` |
| header `X-time` | `sessionStorage`, em `user.time` | 18 dígitos, formato de ticks .NET |

**Medido:** com o cookie da sessão viva, a mesma URL responde

```
sem header nenhum ........................ 401
só X-Requested-With ...................... 401
só X-time ................................ 200   <- é ele
X-time + X-Requested-With ................ 200
```

E o `X-time` **não é um relógio, é um segredo**: ticks calculados na hora são
recusados com 401. Ele é emitido no login e vale enquanto a sessão valer.

`AppConfig.js` também envia `MacAddress` e `PC` — ambos **vazios** na web — e a
linha que setaria `Authorization` está comentada no código. Não há JWT, não há
Bearer, não há CSRF em GET.

> Consequência para o provider: a sessão precisa nascer de um login real de
> navegador. Não há endpoint de troca de credencial por token. Ver §9.

### Expiração

Não foi possível medir a duração sem esperar o vencimento. O que se sabe: a
sessão expirada devolve **401** nas chamadas de API, e `auth.js` tem
`handleHeaders(status)` que redireciona para o login. Não há mecanismo de
refresh no código do cliente.

---

## 3. As APIs encontradas

Base: `https://univirtus.uninter.com`. Todas GET, todas JSON.

| Finalidade | Endpoint | Estabilidade |
| --- | --- | --- |
| sessão viva? | `/ava/sistema/Escola/0/Usuario` | alta — é o ping mais barato |
| curso e situação | `/ava/sistema/UsuarioCurso/0/GetCursosAproveitamento?idUsuario=0` | alta |
| **histórico + semestre + nota final** | `/ava/integracao/UsuarioIntegracaoSistemaAcademico/0/GetDisciplinasAproveitamento?sidCdAluno={sCdAluno}` | alta — vem do sistema acadêmico, não do AVA |
| disciplinas inscritas | `/ava/sistema/UsuarioHistoricoCursoOferta/false/Usuario/` | alta |
| detalhe da disciplina | `/ava/ava/SalaVirtual/{idSalaVirtual}/Get` | alta |
| **avaliações + notas + pesos** | `/ava/bqs/AvaliacaoUsuario/1/paginacao/true?numRegistros=100&filtro=&ordenacao=&idSalaVirtual={sv}&idSalaVirtualOferta={svo}&ajustarDatasMatriculaCurso=false` | alta |
| **trabalhos + entrega** | `/ava/interacao/TrabalhoEtapa/{svo}/GetEtapasByOfertaInscrito/false?master=true&idSalaVirtualOfertaAproveitamento={svo}` | alta |
| roteiro (aulas) | `/ava/ava/SalaVirtualEstrutura/{sv}/TipoOferta/1?idSalaVirtualOferta={svo}&idSalaVirtualOfertaAproveitamento=&idSalaVirtualOfertaPai=` | média |
| atividades da aula | `/ava/ava/salaVirtualAtividade/0/EstruturaOferta/{svo}/?id={idEstrutura}&editar=false&idSalaVirtualOfertaPai=&idSalaVirtualOfertaAproveitamento=` | média |
| material complementar | `/ava/ava/SalaVirtualEstrutura/{sv}/TipoOferta/2/?idSalaVirtualOferta={svo}&idSalaVirtualOfertaPai=null&idSalaVirtualOfertaAproveitamento=null` | média |
| **arquivos de uma atividade** | `/ava/atv/AtividadeItemAprendizagem/{idAtividade}/Atividade?complementar={bool}` | média |
| download do arquivo | `/ava/repositorio/SistemaRepositorioPublico?id={sistemaRepositorio.url}` | ver §6 |
| avisos | `/ava/sistema/AvisoDestinatario/1/paginacao/true?numRegistros=10` | baixa — não aprofundado |

Autenticação, em todas: `Cookie: [REDACTED]` + `X-time: [REDACTED]`.

Todo corpo de resposta usa o mesmo envelope:

```json
{ "<entidadeNoPlural>": [ ... ], "id": 0, "sid": null,
  "totalRegistros": 0, "exception": null, "mensagens": [] }
```

`totalRegistros` **mente**: veio `0` em respostas com 17 itens. Conte o array.

### O que NÃO existe

- **calendário consolidado.** `#/ava/calendario` renderiza vazio e não chama nada;
- **avaliações de todas as disciplinas numa chamada.** Sem `idSalaVirtual` o
  endpoint devolve 404. É um loop por disciplina, obrigatoriamente;
- **frequência.** `SalaVirtualOfertaFrequencia` → 404;
- **pendências globais.** `AvaliacaoUsuarioExpiracao/0/Pendentes` sem oferta → 404;
- **professor.** `nomeProfessor` é `null`.

---

## 4. As estruturas

### Disciplina inscrita (`usuarioHistoricoCursoOfertas[]`)

```json
{ "id": 51125425, "idEscola": 9, "idCurso": 5359,
  "idSalaVirtual": 60236, "idSalaVirtualOferta": 1161461, "idUsuario": "[REDACTED]",
  "nomeCurso": "BACHARELADO EM ENGENHARIA CIVIL - DISTÂNCIA (5986)",
  "nomeSalaVirtual": "Projeto Arquitetônico",
  "codigoOferta": 905706, "totalFilhas": 6,
  "idSalaVirtualOfertaPai": null, "usuarioInscrito": true,
  "dataCriacao": "2026-07-20T11:37:00", "dataModificacao": "2026-08-22T13:03:57" }
```

`codigoOferta === 0` marca **sala de apoio**, e não disciplina — "Dúvidas sobre
Estágio" e "Pesquisa e extensão" caem aí. É o filtro que evita criar duas
Subjects fantasma.

### Histórico acadêmico (`aproveitamento[]`) — a fonte do semestre

```json
{ "cdOfertaDisciplina": 905706, "nomeDisciplina": "Projeto Arquitetônico",
  "nomeModulo": "Módulo B2/2026", "nomeModuloPOrdenacao": "2026B2",
  "nomeCicloTipo": "REGULAR", "cd_grade": 14271,
  "tipoSituacaoAluno": "EM CURSO",
  "aproveitamentoMD": null, "aproveitamentoRF": null, "aproveitamentoTF": null }
```

`nomeModuloPOrdenacao` é o **identificador de semestre**, e ordena
lexicograficamente: `2025B2 < 2025C1 < 2025C2 < 2026A1 < 2026A2 < 2026B1 < 2026B2`.
O corrente é o maior — não há campo "ativo", e ainda bem: seria a linha que
mente em janeiro (é a mesma decisão do ADR-058, §3).

Situações observadas: `EM CURSO`, `EM EXAME`, `APR.MÉDIA`, `APR.EXAME`, `CONCLUIDA`.
Notas: `MD` (média), `RF` (resultado final), `TF`, `ME`, `N1`…`N7`.

### Avaliação (`avaliacaoUsuarios[]`) — nota, peso e prazo juntos

```json
{ "id": 152558335, "idAvaliacao": 2713956,
  "status": "Finalizada", "idAvaliacaoUsuarioStatus": 3, "acao": "Gabarito",
  "dataInicio": "2026-07-13T00:00:00", "dataFim": "2026-08-24T23:59:00",
  "nota": 100, "notaMedia": 15, "tentativa": 1, "tentativaTotal": 3,
  "protocolo": "[REDACTED]",
  "avaliacao": { "id": 2713956, "nome": "APOL Objetiva 1 (Regular)",
                 "nomeAvaliacaoTipo": "APOL Objetiva", "nomeClassificacao": "APOL",
                 "peso": 100, "pesoMedia": 15, "totalQuestoes": 10,
                 "totalTentativas": 3, "periodo": "Regular" },
  "salas": [ { "idSalaVirtual": 60236, "idSalaVirtualOferta": 1161461,
               "codigoOferta": 905706, "nomeSalaVirtual": "Projeto Arquitetônico" } ] }
```

Leitura correta dos campos de nota — e ela **não é óbvia**:

| Campo | O que é | Vai para |
| --- | --- | --- |
| `nota` | o que a pessoa tirou | `score` |
| `avaliacao.peso` | o **teto** da avaliação (100) | `max_score` |
| `avaliacao.pesoMedia` | quanto ela vale na média (15) | `weight` |
| `notaMedia` | a contribuição já ponderada (15) | derivado — não guardar |

`avaliacao.peso` chamar-se "peso" e significar **teto** é a maior armadilha do
contrato. Quem mapear `peso → weight` faz a média inteira errada.

**Status:** `idAvaliacaoUsuarioStatus 3` = Finalizada (tem `id` próprio e `nota`);
`4` = "Aguardando início" — e aí **`id` vem `0`**. Ver §5.

### Trabalho (`trabalhoEtapas[]`)

```json
{ "id": 394147, "idTrabalho": 352876, "idTrabalhoTipo": 1,
  "nome": "Atividade Prática Presencial", "nomeTrabalhoTipo": "Trabalho",
  "dataInicio": "2026-07-13T00:00:00", "dataFim": "2026-08-24T23:59:59",
  "dataEntrega": null, "notaEtapa": null, "notaTrabalho": null,
  "podeEntregar": true, "tentativa": 1, "ordemTrabalho": 1 }
```

`dataEntrega === null` é o sinal de não entregue. `podeEntregar` é a janela
aberta, e não a pendência: há trabalho com `podeEntregar: false` e prazo vencido.

### Roteiro de estudo (`salaVirtualEstruturas[]`)

```json
{ "id": 2758782, "idSalaVirtual": 60236, "idSalaVirtualOferta": 1161461,
  "nome": "Noções de Projeto Arquitetônico", "estrutura": "Aula 1",
  "nomeSalaVirtualEstruturaCompleto": "Aula 1 - Noções de Projeto Arquitetônico",
  "nomeSalaVirtualEstruturaRotulo": "Aula", "ordem": 1, "totalAtividades": 7 }
```

E cada aula tem atividades (`salaVirtualAtividades[]`) com `idAtividade`,
`nomeAtividade`, `nomeTipoAtividade` ("Videoaula", "Leitura"), `pendencia` e
`porcentagemAcessado`.

### Material (`sistemaRepositorio`)

```json
{ "id": "60634399", "nome": "ORIENTACOES ATIVIDADE PRATICA.pdf",
  "extensao": "pdf", "externo": false, "exibirWeb": true,
  "url": "[TOKEN CIFRADO — REDACTED]" }
```

---

## 5. Os identificadores estáveis

| Campo do M/Academic | Vem de | Por que este |
| --- | --- | --- |
| `external_course_id` | `curso.id` (5359) | `idCurso5e` (5986) é o código público, e serve de rótulo |
| `external_semester_id` | `nomeModuloPOrdenacao` ("2026B2") | é o único campo que nomeia período e ainda ordena |
| `external_subject_id` | **`codigoOferta`** (905706) | é a chave que aparece nos DOIS lados — no AVA e no histórico |
| `external_exam_id` | **`idAvaliacao`** (2713956) | ver a armadilha abaixo |
| `external_assignment_id` | `idTrabalho` + `id` da etapa | um trabalho tem N etapas, cada uma com prazo próprio |
| `external_material_id` | `sistemaRepositorio.id` (60634399) | numérico, estável, independe da URL |
| `external_lesson_id` | `salaVirtualEstrutura.id` (2758782) | por oferta — muda entre semestres |

> **A armadilha do `id: 0`.** A avaliação ainda não iniciada vem com `id: 0` —
> porque o `id` é o da *tentativa do usuário*, e ainda não há tentativa. Cinco
> das sete avaliações desta disciplina vêm com `id: 0` ao mesmo tempo. Chavear
> por `id` criaria cinco registros colidindo em zero, e depois um sexto quando a
> prova fosse feita. **A chave é `idAvaliacao`**, que existe desde sempre.

`idSalaVirtual` e `idSalaVirtualOferta` são necessários para *consultar*, mas não
servem de chave de sincronização: `idSalaVirtualOferta` é a instância do
semestre, e a mesma disciplina refeita ganha outro. `codigoOferta` é o que o
histórico usa para falar da mesma disciplina.

---

## 6. Materiais: o que é permanente e o que não é

Três camadas, e elas se comportam diferente:

| Camada | Permanente? |
| --- | --- |
| `sistemaRepositorio.id` | **sim** — numérico, é o que guardar |
| `sistemaRepositorio.url` | token cifrado, exige sessão; não é endereço público |
| bytes servidos pelo CDN | **não** — CloudFront com `Policy`/`Signature`/`Key-Pair-Id` e `AWS:EpochTime` de poucas horas |

Ou seja: **guardar a URL do CDN é guardar um link morto.** O que o M/OS guarda é
o `id` e o nome; o download se resolve na hora, com sessão viva, por
`/ava/repositorio/SistemaRepositorioPublico?id={url}`.

**Onde o material realmente está** — medido nas 8 aulas de uma disciplina:

```
Aula 1..6  ->  0 arquivos   (videoaula e leitura: conteúdo embutido em atividadeEtiquetas)
Aula 7     ->  3 arquivos   [60634399, 51303791, 51303804]
Aula 8     ->  4 arquivos   [41669449, 25412356, 25601395, 51172555]
complementar (TipoOferta/2) -> 5 arquivos + o PLANO DE ENSINO
```

Quem varrer só o roteiro perde o Plano de Ensino, que é o documento mais útil da
disciplina inteira. `complementar=true` no `AtividadeItemAprendizagem` é o que o
traz.

---

## 7. O mapeamento para o M/Academic

```
Univirtus                              M/Academic (ADR-058)

nomeModuloPOrdenacao "2026B2"      →   Semester.name
curso.nome                         →   Semester.institution
usuarioHistoricoCursoOferta        →   Subject          (filtrar codigoOferta !== 0)
  codigoOferta                     →     external_id
  nomeSalaVirtual                  →     name
  (não existe)                     →     teacher — fica vazio
avaliacaoUsuario                   →   Exam
  avaliacao.nome                   →     name
  dataFim                          →     scheduled_at
  nota                             →     score
  avaliacao.peso                   →     max_score        <- não é o peso!
  avaliacao.pesoMedia              →     weight
trabalhoEtapa                      →   Assignment
  dataFim                          →     due_at
  dataEntrega                      →     submitted_at
  notaEtapa                        →     score
sistemaRepositorio                 →   Resource          (por junção, §7 do ACADEMIC.md)
salaVirtualEstrutura ("Aula N")    →   — sem lugar hoje
```

Duas observações que o ADR-058 já antecipa:

- **a nota continua morando na avaliação.** O Univirtus faz igual: não há tabela
  de notas do lado dele também, e `aproveitamentoMD` do histórico é derivado.
  Os dois modelos concordam, e isso é sorte boa;
- **`Aula N` não tem entidade no M/Academic.** Não inventar uma agora: o
  `StudySession` de hoje mede tempo, e não conteúdo. É o gancho do Study Agent
  que o §9 do `ACADEMIC.md` deixa fora de escopo.

E uma que ele **não** antecipa: a média do Univirtus e a de
`mos_core::academic::desempenho` vão discordar, porque a instituição tem regra de
exame e recuperação que o M/OS não modela. O `aproveitamentoMD` deve entrar como
**dado do provider**, exibido como "média oficial", e nunca sobrescrever a média
calculada — senão o M/OS passa a ter duas fontes para o mesmo fato, que é
exatamente o que o ADR-058 proibiu.

---

## 8. A estratégia de sincronização

### Primeira sincronização

```
1. checkSession        GET /ava/sistema/Escola/0/Usuario        -> 200?
2. contexto            GetCursosAproveitamento                  -> idCurso, sCdAluno
3. histórico           GetDisciplinasAproveitamento             -> semestres + situação + nota final
4. ofertas             UsuarioHistoricoCursoOferta              -> idSalaVirtual / idSalaVirtualOferta
5. junta por codigoOferta; semestre corrente = max(nomeModuloPOrdenacao)
6. por disciplina:     AvaliacaoUsuario + TrabalhoEtapa         -> 2 chamadas
7. sob demanda:        estrutura + atividades + itens           -> materiais
```

Custo medido no semestre corrente: **6 chamadas** para disciplina, prazo e nota
(2 disciplinas). Material é caro — 1 chamada por atividade — e por isso deve ser
sob demanda, e não parte do sync.

### Incremental

Não há `If-Modified-Since`, não há ETag, não há cursor. O que existe:

- `dataModificacao` em `usuarioHistoricoCursoOferta` e em `avaliacao`;
- `dataModificacao` do próprio `avaliacaoUsuario`.

A comparação prática é **hash do payload normalizado por `external_id`**:

```
NEW        external_id não existe no M/OS
UPDATED    existe, hash mudou      -> atualizar campos do provider
UNCHANGED  existe, hash igual      -> nada
AUSENTE    estava, não veio agora  -> marcar unavailable_since, NUNCA apagar
```

Ausência não é exclusão: uma avaliação some da lista quando a janela fecha, e o
histórico da pessoa não pode evaporar por causa disso.

### Não duplicar

Todo objeto sincronizado carrega:

```
provider            = "univirtus"
external_id         = ver §5   (idAvaliacao, NÃO id)
external_updated_at = dataModificacao, quando existir
last_synced_at      = agora
```

Chave única: `(provider, external_id, kind)`.

### Sessão expirada

401 em qualquer chamada → parar o sync inteiro, marcar a conexão como expirada e
**não** marcar nada como ausente. Um sync que roda com sessão morta veria zero
disciplinas e concluiria que a pessoa trancou o curso.

---

## 9. Arquitetura recomendada

```
Univirtus (REST JSON + sessão de navegador)
      ↓
UnivirtusProvider          fala HTTP, conhece os endpoints, devolve o cru
      ↓
Normalizer                 aplica §5 e §7: escolhe as chaves, corrige peso↔teto
      ↓
Academic Sync Engine       NEW/UPDATED/UNCHANGED/AUSENTE, por external_id
      ↓
M/Academic                 Subject, Exam, Assignment, Resource — e a Task do M/OS
```

O `AcademicProvider` do §21 do pedido cabe quase intacto. Dois ajustes que a
investigação impõe:

```ts
interface AcademicProvider {
  checkSession(): Promise<boolean>
  getAcademicContext(): Promise<AcademicContext>
  getSubjects(): Promise<ExternalSubject[]>
  getExams(subject: ExternalSubjectRef): Promise<ExternalExam[]>              // POR disciplina
  getAssignments(subject: ExternalSubjectRef): Promise<ExternalAssignment[]>  // POR disciplina
  getMaterials(subject: ExternalSubjectRef): Promise<ExternalMaterial[]>      // caro, sob demanda
}
```

`authenticate()` **não** entra na interface. Não existe endpoint de credencial →
token; a sessão nasce de um login de navegador. O desenho honesto é uma
`UnivirtusSession` que o M/OS obtém uma vez (WebView de login) e guarda no cofre
local — cookie + `X-time` —, e que o provider só consome. Enfiar
`authenticate(user, pass)` na interface prometeria uma capacidade que o portal
não oferece.

E `getGrades()` some: a nota **não tem endpoint próprio**. Ela vem dentro da
avaliação, e essa é a mesma decisão do ADR-058. Manter `getGrades()` na interface
recriaria a terceira fonte que o ADR eliminou.

---

## 10. O que não tocar

Read-only não é só "não fazer POST". No Univirtus há **GET que altera estado**:

| Chamada | O que faz |
| --- | --- |
| a ação `Iniciar` de uma avaliação | **inicia tentativa** — consome uma das 3, e a prova passa a correr |
| `/ava/sistema/AvisoDestinatarioPopup/{id}/Lido/true` | marca aviso como lido |
| `GetAtividadeItemAprendizagemAcessado` | registra acesso, move `porcentagemAcessado` |

Os dois últimos são inofensivos mas **sujam a telemetria da pessoa**: um sync
noturno marcaria tudo como acessado, e `porcentagemAcessado` deixaria de
significar "eu estudei isto". O provider deve evitá-los.

O primeiro é intocável. Nenhum caminho do M/OS pode chegar nele.

---

## 11. Riscos

| Risco | Gravidade | Por quê |
| --- | --- | --- |
| sessão expira e não renova | **alta** | é o único ponto que quebra o sync inteiro, e depende de login humano |
| `X-time` mudar de nome ou virar Bearer | média | `Authorization` já está no código, comentado — o dia em que descomentarem, quebra |
| URL de CDN guardada | média | assinada e de vida curta; só quebra quem guardou em vez de resolver na hora |
| endpoints sem versão | média | não há `/v1/`; a rota é o contrato |
| `totalRegistros` mentir | baixa | já mente hoje; contar o array imuniza |
| N+1 por disciplina | baixa | 2 chamadas por disciplina; ~15 disciplinas é aceitável |
| `nomeModuloPOrdenacao` mudar de formato | baixa | quebraria a ordenação do semestre corrente |

Nenhum CAPTCHA foi exigido no fluxo normal. Não confundir com ausência: o
reCAPTCHA está carregado na página de login e pode aparecer sob suspeita.

---

## 12. Recomendação final

**Direct JSON endpoints**, com login por navegador uma vez.

DOM parsing é desnecessário — não há dado exclusivo do HTML. Browser automation
por clique é pior em tudo, e além disso passaria perto do botão "Iniciar" (§10).
O híbrido se limita ao que já é híbrido por natureza: **a sessão** vem de um
WebView de login, e **os dados** vêm de HTTP puro.

### As perguntas do §26, respondidas

| Pergunta | Resposta |
| --- | --- |
| como descobre minhas disciplinas? | `UsuarioHistoricoCursoOferta` cruzado com `GetDisciplinasAproveitamento` por `codigoOferta` |
| como acha as pendentes? | avaliação com `nota === null`; trabalho com `dataEntrega === null` |
| como sabe quando vencem? | `dataFim` nos dois — já em ISO, já com hora |
| como acha as próximas provas? | `avaliacaoUsuarios` com `idAvaliacaoUsuarioStatus 4` e `dataInicio` futura |
| como busca minhas notas? | `nota` / `avaliacao.peso` na avaliação; `aproveitamentoMD` / `RF` no histórico |
| como acha materiais? | `AtividadeItemAprendizagem` por atividade, e `complementar=true` para o Plano de Ensino |
| como mantém a sessão? | cookie `HttpOnly` + header `X-time`; **não** se mantém sozinha |
| como não duplica? | `(provider, external_id, kind)`, com `idAvaliacao` — nunca `id` |
| o que quebra quando mudarem? | a rota, que é o contrato; e o `X-time`, se virar Bearer |

---

# Production Implementation

> Esta seção descreve o que a investigação acima virou. Ela **acrescenta**, e não
> substitui: os achados de §1–§12 continuam sendo o registro do que o portal é.
>
> Implementado em 2026-08-22.

## 13. As camadas

```
Univirtus (REST JSON + sessão de navegador)
   |  apps/desktop/src-tauri/src/univirtus.rs      provider: sessão, HTTP, allowlist
   |  crates/mos-core/src/academic_univirtus.rs    normalizer: JSON -> tipos neutros
   |  crates/mos-core/src/academic_sync.rs         engine: reconciliação pura
   |  crates/mos-storage-sqlite/src/
   |      academic_provider_repository.rs          aplicação, num commit só
   v  M/Academic (as cinco tabelas da 0031)
```

A fronteira que importa: **nada acima do provider sabe o que é `/ava/bqs/`.** Os
tipos que atravessam são `ExternalSubject`, `ExternalAssessment`,
`ExternalAssignment` e `ExternalMaterial` — sem um campo chamado `idSalaVirtual`
nem `pesoMedia` em lugar nenhum. Trocar a UNINTER por outro AVA é escrever outro
`univirtus.rs`; as outras três camadas não mudam.

O normalizador vive em `mos-core`, e não no desktop, porque ele é **puro**: dado
JSON, produz tipos. É o que permite testar `peso` ≠ `pesoMedia` e `id: 0` sem
sessão autenticada, sem rede e sem Tauri.

## 14. A sessão, na prática

Não há `authenticate(usuario, senha)`, e essa ausência é a decisão:

```
Settings -> UNIVIRTUS -> Conectar
   |
janela do app abre https://univirtus.uninter.com/ava/web/
   |
a pessoa entra na PÁGINA OFICIAL
   |
o M/OS espera a URL virar #/ava
   |
lê sessionStorage.user.time   (eval_with_callback)
lê o cookie HttpOnly          (WebviewWindow::cookies_for_url)
   |
valida com um GET real antes de guardar
   v
Credential Manager do Windows
```

**A senha nunca entra no M/OS.** Não há campo para ela, não há POST de login
reimplementado e não há nada a guardar. O cookie é `HttpOnly` — só o processo
dono do WebView o alcança, e é por isso que o login precisa ser uma janela do
app, e não o navegador do sistema.

O `X-time` **nunca é calculado**. `UnivirtusSession::new` valida o formato (18
dígitos), e nada mais: o valor só o servidor sabe validar, e a investigação
provou que ticks gerados na hora são recusados.

Cookie e `X-time` moram no Credential Manager (`m-os` / `univirtus-session`),
pelo mesmo caminho de `mos-hermes/src/auth.rs` e `finance.rs`.
`UnivirtusSession` tem `Display` manual que imprime `<redigida>` — um `{:?}` num
log é tudo que separa segredo guardado de segredo vazado.

## 15. A allowlist

`crate::univirtus::Rota` é um **enum fechado**. Não existe `request(path)` no
arquivo: para chamar algo novo é preciso escrever uma variante, e escrever a
variante é o momento em que alguém tem de decidir se aquilo é mesmo leitura.

Duas camadas, nesta ordem:

1. **o enum** — o que protege de verdade;
2. **`caminho_permitido`** — prefixos permitidos mais uma lista de fragmentos
   proibidos (`Iniciar`, `AvisoDestinatarioPopup`,
   `GetAtividadeItemAprendizagemAcessado`). Verificada antes de o socket abrir,
   com teste que falha se qualquer um deles passar.

A blacklist é a segunda camada de propósito: ela protege contra os três GET com
efeito colateral que a investigação encontrou, e contra nenhum dos que ela não
encontrou. O enum protege contra todos.

## 16. Os mapeamentos críticos

| Univirtus | M/Academic | Onde |
| --- | --- | --- |
| `avaliacao.peso` | **`max_score`** | `academic_univirtus::assessments` |
| `avaliacao.pesoMedia` | **`weight`** | idem |
| `idAvaliacao` (nunca `id`) | `external_id` da `Exam` | idem |
| `idTrabalho:id` da etapa | `external_id` do `Assignment` | `assignments` |
| `sistemaRepositorio.id` | `external_id` do material | `materials` |
| `aproveitamentoMD` | `academic_provider_subject_facts.official_grade` | `subjects` |
| `nomeModuloPOrdenacao` | `Semester` | `semesters` |
| `codigoOferta` | `external_id` da `Subject` | `subjects` |

Cada um tem teste nomeado que morre se for trocado:
`peso_e_teto_e_peso_media_e_peso`,
`tres_avaliacoes_com_id_zero_produzem_tres_identidades`,
`as_etapas_do_mesmo_trabalho_nao_colapsam` e
`o_material_e_identificado_pelo_id_do_repositorio_e_nunca_pela_url`.

## 17. A média oficial não invade a calculada

`aproveitamentoMD` vai para `academic_provider_subject_facts` — tabela própria,
**não** uma coluna de `academic_subjects`. O motivo é o ADR-058: a média do M/OS
é derivada em `mos_core::academic::desempenho`, e uma coluna com a média da
faculdade seria a terceira fonte que aquele ADR eliminou.

Na tela, as duas aparecem lado a lado no card da disciplina: a própria em
destaque, a oficial como `oficial 7,4`. Elas discordam de propósito — a UNINTER
conta exame e recuperação que o M/OS não modela.

## 18. A fronteira com a decisão pessoal

O portal é dono do **fato acadêmico**. A pessoa é dona da **organização**.

| Do portal (o sync escreve) | Da pessoa (o sync nunca toca) |
| --- | --- |
| título, prazo, peso, teto, nota, estado | `priority`, `task_id` |
| nome e código da disciplina | `accent`, `notes` |
| nome e datas do semestre | `location` da prova, `lifecycle_state` |

Os `UPDATE` do repositório listam colunas explicitamente por isso. Nenhum diz
`SET ... priority = ?`, e o teste
`a_republicacao_do_portal_nao_desfaz_a_decisao_da_pessoa` prova que marcar uma
atividade como urgente e criar a Task dela sobrevive a uma mudança de prazo.

`teacher` é o caso especial: o Univirtus manda vazio, e vazio **não apaga** — o
`UPDATE` usa `CASE WHEN ?5 = '' THEN teacher ELSE ?5 END`.

## 19. Sincronização

**Recorte: o semestre corrente.** Não há endpoint consolidado; avaliações e
trabalhos custam duas chamadas por disciplina. Varrer o histórico inteiro
gastaria cerca de 30 chamadas por rodada para reconfirmar notas de semestres
fechados. O histórico completo vem de graça na primeira chamada, e dá semestre,
situação e média oficial de tudo.

O recorte é declarado, e `reconcile_scoped` o respeita: o que não foi perguntado
**não é marcado como ausente**. Sem isso, cada sincronização marcaria como
sumidas todas as avaliações dos semestres passados, e a seguinte as
ressuscitaria — dois eventos falsos por rodada, para sempre.

**Idempotência** por `payload_hash` em `academic_external_refs`, chaveado por
`(provider, kind, external_id)` mais um índice único em
`(provider, kind, local_id)`. Rodar quatro vezes seguidas termina com o mesmo
banco; há teste.

**Ausência nunca apaga.** `unavailable_since` marca, e reaparecer limpa a marca.

**Tudo num commit.** Semestres, disciplinas, avaliações, trabalhos, materiais,
referências e estado entram juntos ou não entram. Em sequência, uma queda no
meio deixaria uma `Exam` gravada sem a referência que a liga ao `idAvaliacao`, e
a rodada seguinte criaria a segunda — a duplicata nasceria de uma falha de rede.

**Falha isolada.** Uma disciplina que não responde vira aviso no relatório; as
outras entram, e o resultado é `completed_with_warnings`.

**Sessão expirada** (401) **não aplica retrato nenhum**. Um retrato vazio,
aplicado, seria lido como "sumiu tudo". A conexão vira `expired`, os dados de
antes ficam intactos, e a tela passa a oferecer "Reconectar".

**Automática:** uma vez por abertura do app, e só quando já há sessão guardada.
Sem polling — dado acadêmico muda algumas vezes por semana.

## 20. O que ficou de fora, e por quê

| Fora | Motivo |
| --- | --- |
| baixar os arquivos | a URL é assinada e vence em horas; o material é `Resource` sem URL até o KNOW/OS resolver storage |
| entidade "Aula" | o roteiro é endereço de consulta; o `StudySession` de hoje mede tempo, e não conteúdo (§9 do `ACADEMIC.md`) |
| professor | `salaVirtual.nomeProfessor` vem `null` |
| frequência | o endpoint devolve 404 |
| avisos | `AvisoDestinatario` responde, mas marcar como lido é GET com efeito colateral; entrar aqui exigiria decidir isso antes |
| criar Task automática | o M/Academic já tem o gesto, e criar dezenas sem critério é o oposto do que o módulo pede |

## 21. Limitações reais

- **a sessão não se renova.** É do portal, e não da implementação: não há
  refresh. Reconectar é um clique, e os dados sobrevivem à expiração;
- **desconectar corta o fio.** `forget_provider` apaga as referências e preserva
  as entidades. Reconectar depois traz um conjunto novo **ao lado** do antigo —
  há teste que registra isso
  (`reconectar_depois_de_desconectar_recria_e_isso_esta_documentado`) em vez de
  fingir que não acontece. Quem quer só pausar sincroniza menos, e não
  desconecta;
- **as datas do semestre são inferidas** do rótulo `2026B2`. O M/OS precisa de
  intervalo porque `Semester::status_em` deriva o estado das datas; o intervalo
  respeita a ordem e contém o presente, mas não bate com o calendário oficial ao
  dia;
- **peso de trabalho é 0.** O Univirtus não publica peso de trabalho na média, e
  zero é o que o ADR-058 §4 já entende como "não conta quando há avaliação com
  peso".
