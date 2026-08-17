# M-Finance embutido no M/OS (Feature A) — Design

**Status:** aprovado para plano de implementação

**Data:** 2026-08-17

**Baseline:** M/OS `v0.2.11`; M-Finance vendorado em `apps/m-finance`, deploy em produção `https://m-finance-silk.vercel.app`

**Origem:** conversa sobre integrar M-Finance ao M/OS "do jeito que o CronoCAD foi". Pesquisa mostrou que o ADR-032 já decidiu explicitamente **não** portar a UI do M-Finance para dentro do M/OS (stack Next.js/Postgres, precisa de acesso mobile). Esta feature não contradiz o ADR-032: não porta código nem migra dados — só muda onde a mesma URL é exibida.

## 1. Objetivo

Hoje "M Finance" no App Registry abre `https://m-finance-silk.vercel.app` no navegador padrão do Windows via `ShellExecuteW`, o que tira o usuário do M/OS. Esta feature faz o M-Finance abrir **dentro** da janela do M/OS, como uma página nativa a mais no rail — sem portar código, sem tocar no Postgres, sem mudar o M-Finance.

## 2. Escopo

**Dentro:**
- novo destino de rail `finance` no grupo `TRABALHO`;
- `apps/desktop/src/FinancePage.tsx` — página que renderiza um `<iframe>` para a URL de produção do M-Finance;
- ajuste de CSP em `apps/desktop/src-tauri/tauri.conf.json` para permitir esse `frame-src`;
- documentação da decisão (este spec + nota no App Registry).

**Fora:**
- qualquer mudança em `apps/m-finance` (código, headers, auth);
- token-passing / SSO entre M/OS e M-Finance (login duplicado é aceito, ver §4);
- deep-linking para uma tela específica do M-Finance;
- estados de erro/offline customizados (usa o comportamento nativo do iframe/navegador);
- Feature B (Hermes agindo no M-Finance) — spec separada, ainda não iniciada.

## 3. Ponto de entrada

- `Page` type em `App.tsx` ganha `"finance"`.
- Grupo de rail `TRABALHO` (mesmo grupo de Projects, Calendar, Tempo) ganha o item `{ page: "finance", label: "Finance", icon: "finance" }`. Precisa de um ícone novo (`Icon name="finance"`) seguindo o padrão de stroke já existente — reaproveitar um ícone já existente e neutro (ex. o mesmo símbolo genérico usado para Apps) se não houver tempo de desenhar um novo, documentando a dívida.
- O App Registry mantém a entrada `m-finance` como está (metadata/URL), sem remover — quem clicar em "M Finance" na lista de Apps continua abrindo no navegador externo; o rail é o novo caminho "embutido", não uma substituição do App Registry.

## 4. Renderização

`FinancePage.tsx`, no mesmo nível de `TempoPage.tsx`:

```tsx
const FINANCE_URL = "https://m-finance-silk.vercel.app";

export function FinancePage() {
  return (
    <div className="page finance-page">
      <iframe
        className="finance-frame"
        src={FINANCE_URL}
        title="M Finance"
        allow="clipboard-write"
      />
    </div>
  );
}
```

- Sem `PageHeader`/`PaneHeader`: o M-Finance já tem navegação própria, duplicar cabeçalho é ruído (mesmo princípio já aplicado ao Tempo, mas aqui é ainda mais direto — nem o conteúdo é nativo).
- CSS (`App.css`): `.finance-page { padding: 0; }` e `.finance-frame { width: 100%; height: 100%; border: 0; }`, para o iframe ocupar toda a área de conteúdo do shell sem herdar o padding padrão de `.page`.
- Sessão: cookies da webview persistem entre aberturas (comportamento padrão do WebView2/WebKit); primeiro acesso pede login do Supabase, os seguintes não. Nenhum código de sessão/token é escrito pelo M/OS.

## 5. CSP

`apps/desktop/src-tauri/tauri.conf.json`, bloco `security.csp` (linhas ~52-55 hoje):

- prod: adicionar `frame-src https://m-finance-silk.vercel.app` a `default-src 'self' customprotocol: asset:; ...`.
- dev: mesma adição (o dev build usa a mesma URL de produção do M-Finance — não há M-Finance local rodando nesta feature).

Nenhuma permissão Tauri nova (`webview:*`) é necessária — iframe é HTML padrão, não usa a API de multiwebview nativa do Tauri (que não está configurada no repo).

## 6. Estados de borda

- Offline / M-Finance fora do ar: o iframe mostra o que o motor de renderização mostrar nativamente (página de erro do navegador embutido). Não fabricamos um estado de erro M/OS para isso nesta feature.
- Navegação dentro do iframe (voltar/avançar, deep links do M-Finance): comportamento padrão do iframe, não interceptado pelo M/OS.

## 7. QA / gate de conclusão

- `npm run build` (`apps/desktop`), `npm test -- --run`, `git diff --check`;
- inspeção visual real no cliente Tauri: abrir "Finance" no rail, confirmar que a página do M-Finance carrega dentro da janela (Dark, 1280×800 e 1440×900 no mínimo);
- confirmar login funcionando dentro do iframe (Supabase);
- confirmar que a navegação para outras páginas do M/OS e volta para Finance não perde a sessão da webview;
- confirmar CSP: sem erro de bloqueio no console ao carregar o iframe;
- nenhuma regra de negócio, API, banco ou contrato de domínio do M/OS ou do M-Finance alterada.

## 8. Fora de escopo / decisões futuras

- Ícone dedicado para "Finance" no rail pode ficar como dívida documentada se não houver tempo neste lote.
- SSO/token-passing entre M/OS e M-Finance fica para quando (e se) a Feature B (Action API) tornar isso necessário — não é pré-requisito desta feature.
- Este spec não reabre nem contradiz o ADR-032 (M-Finance continua Next.js/Postgres/Vercel, rodando exatamente como hoje).
