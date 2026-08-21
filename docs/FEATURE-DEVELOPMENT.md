# FEATURE-DEVELOPMENT — Como uma feature nasce no M/OS

> **O M/OS é multi-device por definição.** Não existe "feature do desktop que
> talvez seja portada depois". Existe feature do M/OS, com manifestações
> apropriadas em cada plataforma — e "não se aplica aqui" é uma dessas
> manifestações, desde que dita em voz alta.

---

## 1. A ordem

Errado — e é assim que dívida de plataforma nasce:

```
feature no desktop
  ↓ meses
alguém lembra do celular
  ↓
reimplementa tudo
```

Certo:

```
Feature X
├── Domínio      o que é, e quais regras
├── Database     migration, sempre para frente
├── Sync         o que viaja, e como dois lados reconciliam
├── Desktop UI   manifestação com densidade e teclado
└── Mobile UI    manifestação com toque e contexto
```

O domínio primeiro não é formalidade. É o que impede a mesma regra de ser
escrita duas vezes, com dois comportamentos, em dois lugares.

---

## 2. O checklist

Toda feature declara os oito eixos antes de ser considerada pronta. **Ausência é
resposta válida; silêncio não é.**

```
[ ] Core           regra em mos-core, sem plataforma dentro
[ ] Database       migration versionada; nunca destrói dado antigo
[ ] Sync           o que viaja e como reconcilia — ou "local, não sincroniza, porque…"
[ ] Desktop        manifestação e interação
[ ] iOS            manifestação e interação — ou "não se aplica, porque…"
[ ] Notifications  local, remota, ou nenhuma
[ ] Hermes         o agente enxerga? age?
[ ] Tests          o que prova que funciona
```

Escreva a resposta, mesmo quando for "não". "iOS: não se aplica — monitorar
processos não existe na plataforma" é uma resposta boa. Deixar o eixo em branco
não é.

### Exemplo preenchido

```
feature: waiting-for

core:          Waiting, com prazo e responsável
database:      migration 00XX, tabela própria
sync:          sincroniza; merge por campo
desktop:       coluna na Home + linha na Timeline
ios:           card na Home + notificação ao vencer
notifications: local nos dois; remota quando houver servidor
hermes:        lê e cria
tests:         domínio + migration + reconciliação
```

---

## 3. As três regras duras

**1. Nada de plataforma em `mos-core` ou `mos-sync`.** Um `#[cfg(windows)]`
nesses dois crates significa que o desenho quebrou. Eles precisam compilar
idênticos no Windows e no iOS, e a única garantia real é não haver como
escrever código de plataforma ali dentro.

**2. Pergunte o que a plataforma pode fazer, nunca qual ela é.**

```ts
if (plataforma === "ios")            // proibido
if (capacidades().nativeShare)       // certo
```

Ver `apps/desktop/src/platform.ts`. A diferença aparece no dia em que o macOS
entrar: o primeiro esconde o botão numa plataforma que tem share nativo.

**3. Lógica de negócio não se duplica.** `TaskDesktopView` e `TaskMobileView`
são legítimos. Dois `taskService` não são. Se a mesma regra precisa existir nos
dois lados, ela está no lugar errado — sobe para o domínio.

---

## 4. Migrations

Sempre para frente, sempre versionada, nunca destrutiva.

- Uma migration nova nunca altera dado que uma versão antiga do app ainda lê de
  um jeito diferente. Desktop e iPhone **não atualizam juntos** — a App Store
  não publica quando o desktop publica.
- Banco criado por versão mais nova é recusado na abertura, com mensagem. Já é
  o comportamento hoje.
- Antes de subir de versão, o M/OS grava um snapshot pré-migration. Continua
  valendo.
- Prefira acrescentar tabela a alterar tabela. A migration 0027 acrescentou
  quatro e não tocou em nenhuma existente — se o desenho do sync mudar, elas se
  apagam sem risco.

---

## 5. Interface: pergunte a função, não o tamanho

Antes de portar uma tela, a pergunta certa é:

> **Qual é a função desta tela?**

E não "como eu encolho isto para 393px?".

Projects no desktop é `sidebar + lista + detalhe`, três painéis ao mesmo tempo,
porque a tela cabe e o mouse alcança. No iPhone a mesma função vira uma pilha de
navegação: `Projects → Project → Detalhe`. Mesmos dados, mesma regra, gesto
diferente.

| Desktop | iOS |
| --- | --- |
| hover, atalho, menu de contexto | toque, swipe, toque longo |
| múltiplos painéis | pilha de navegação |
| drag and drop | bottom sheet |
| command palette | busca contextual |
| densidade alta | densidade reduzida |
| — | haptics, share nativo |

Linguagem visual igual. Interação de acordo com a plataforma.

---

## 6. Antes de dizer que acabou

- Os testes que cobrem o que mudou rodaram, e você viu a saída.
- Se mexeu em interface, você **viu a tela** — não a árvore de acessibilidade,
  que serve dado em cache. Ver a skill `ver-o-app`.
- Se a feature sincroniza, existe teste de reconciliação para o caso em que os
  dois lados mexem na mesma coisa.
- O checklist da §2 está preenchido, inclusive nos "não se aplica".

---

## 7. Onde ler o resto

| Documento | Assunto |
| --- | --- |
| `PLATFORMS.md` | auditoria, matriz de features, capabilities |
| `DAILY-SESSION.md` | um checklist preenchido de ponta a ponta, com os "não se aplica" escritos |
| `SYNC.md` | relógio, operações, reconciliação, contrato |
| `DECISIONS.md` | ADR-052 (estratégia mobile), ADR-053 (sync) |
| `ARCHITECTURE.md` | a arquitetura do desktop |
| `UX-PRINCIPLES.md` | as regras de interface que não mudam com a plataforma |
