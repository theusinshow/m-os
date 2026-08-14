# Spec — estado vazio honesto nos painéis com escopo

Data: 2026-08-14
Escopo: `App.tsx`, painéis PROJECTS e APPS da Home. Nada em core, storage ou API.

---

## 0. O bug

A Home filtra Projects e Apps pelo Workspace ativo (`App.tsx:186-187`):

```js
const scopedProjects = currentWorkspace ? workspaceProjects : projects.filter(...);
const scopedApps = currentWorkspace ? workspaceApps : apps.filter(...);
```

Quando o resultado é vazio, os painéis mostram (`App.tsx:216` e `App.tsx:219`):

- *"Projects criados aparecerão aqui."*
- *"Apps cadastrados aparecerão aqui."*

As duas frases afirmam que **não existe nada cadastrado**. Mas o vazio tem duas causas
distintas, e a mensagem não as separa:

1. não existe nada cadastrado no sistema;
2. existe, mas nada está vinculado ao Workspace ativo.

No banco de referência a segunda causa é a real: **5 registros em `apps`, 0 em
`app_workspaces`**, com o Workspace "Testes" ativo. A Home afirma que não há apps
cadastrados enquanto cinco existem. O usuário perde acesso a dado próprio e não recebe
nenhuma pista de por quê.

O painel de Projects tem o mesmo defeito. Hoje não se manifesta porque
`project_workspaces` tem 2 vínculos, mas a condição é idêntica e falha do mesmo modo
assim que um Workspace ficar sem Projects.

---

## 1. Decisão

Separar os três estados e, no caso ambíguo, oferecer o caminho para resolver.

| Condição | Conteúdo |
|---|---|
| `total === 0` | Texto atual, que passa a ser verdadeiro só neste caso |
| `total > 0` e escopo vazio e há Workspace ativo | Contagem, nome do Workspace, e ação de vincular |
| escopo não vazio | Renderiza a lista, sem mudança |

O dado necessário já está no componente. `HomePage` recebe `apps` e `projects`
completos por prop e calcula `workspaceApps` e `workspaceProjects` no escopo — hoje só
consome o segundo. **Nenhuma chamada de API nova, nenhuma query nova, nenhuma migration.**

---

## 2. A ação de vincular

A UI de vínculo já existe: `WorkspacesPage` (`App.tsx:439`) tem checkboxes que ligam
Projects e Apps ao Workspace, sob `data-function-section="workspace.link_project"` e
`workspace.link_app`.

Não se constrói tela nova. O estado vazio chama `openWorkspace(currentWorkspace)`, já
disponível como prop de `HomePage` (`App.tsx:157`). `WorkspacesPage` aceita
`initialWorkspaceId`, então abre direto no Workspace certo.

---

## 3. Componente

A lógica de três estados é idêntica nos dois painéis. Fica num componente próprio em vez
de duplicada:

```tsx
function ScopedEmptyState({ total, workspace, noun, onLink }: {
  total: number;
  workspace: Workspace | undefined;
  noun: "app" | "project";
  onLink: () => void;
})
```

Sem anotação de tipo de retorno, seguindo a convenção do arquivo: nenhum dos cinco
arquivos de `src/` usa `JSX.Element`, e o React aqui é 19, onde o namespace global `JSX`
não é mais exposto por padrão — anotar quebraria o `tsc`.

`App.tsx` tem 117KB e concentra praticamente toda a UI. Este spec não propõe quebrá-lo —
seria refatoração não pedida. Mas também não acrescenta a mesma condicional duas vezes:
um componente com uma responsabilidade única é mais barato de manter e de ler.

---

## 4. Texto

Plural é obrigatório: com um único registro a frase quebra.

**Apps**
- `total === 0` → `Apps cadastrados aparecerão aqui.`
- `total === 1` → `1 app cadastrado, nenhum em {workspace}.`
- `total > 1` → `{total} apps cadastrados, nenhum em {workspace}.`

**Projects**
- `total === 0` → `Projects criados aparecerão aqui.`
- `total === 1` → `1 Project criado, nenhum em {workspace}.`
- `total > 1` → `{total} Projects criados, nenhum em {workspace}.`

Rótulo da ação: `Vincular`.

`Project` e `App` mantêm inicial maiúscula por serem nomes de conceito do produto, como
já ocorre no resto da interface.

---

## 5. Contagem

`total` conta apenas registros ativos — `lifecycleState === "active"` — coerente com o
que os painéis já filtram. Item arquivado não entra na contagem, senão a Home passaria a
prometer apps que o usuário arquivou de propósito.

---

## 6. Verificação

O banco de referência reproduz o bug sem preparação: Workspace "Testes" ativo,
`app_workspaces` vazio, 5 apps cadastrados.

1. **Antes** — Home com "Testes" ativo diz *"Apps cadastrados aparecerão aqui."*, o que é
   falso.
2. **Depois** — deve dizer *"5 apps cadastrados, nenhum em Testes."*
3. Acionar `Vincular` abre Workspaces já em "Testes", com os checkboxes visíveis.
4. Vincular um app e voltar à Home: o app aparece no painel.
5. Acionar "Todos" (sem Workspace ativo): os 5 apps aparecem, como já acontece hoje.
6. O painel de Projects continua listando os 2 vinculados, sem regressão.

Não há infraestrutura de teste de front no projeto — `package.json` define apenas
`"build": "tsc && vite build"`. Este spec não promete teste automatizado. A verificação é
`npm run build` mais os seis passos acima.

---

## 7. Fora de escopo

- Rever a decisão de escopar a Home por Workspace. O filtro é intencional; o defeito está
  na comunicação do vazio, não na regra.
- Vincular app ou project a Workspace direto da Home, sem passar pela tela de Workspaces.
- Os demais widgets do desenho v0.6 e do Ideas.
- Quebrar `App.tsx` em arquivos menores.
