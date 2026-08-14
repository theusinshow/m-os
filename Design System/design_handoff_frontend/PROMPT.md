# Como usar isso no Claude Code

## 1. Colocar o pacote no repositório

Copie a pasta inteira para a raiz do repo do M/OS:

```
m-os/design_handoff_frontend/
```

## 2. Fixar as regras (uma vez)

O repo já tem `AGENTS.md`. Acrescente estas linhas nele (ou crie `CLAUDE.md` na raiz com o mesmo conteúdo) para que valham em toda sessão, sem precisar repetir:

```markdown
## Front-end

O design do front-end está fechado em `design_handoff_frontend/`.

- `design_handoff_frontend/README.md` é a especificação. Ele manda sobre qualquer palpite visual.
- `design_handoff_frontend/mos-tokens.css` é a única fonte de cor, spacing, radius, tipografia e duração. Nenhum desses valores pode aparecer hardcoded em componente.
- `design_handoff_frontend/design/*.dc.html` são referências em HTML. São para ler e recriar em React + TypeScript no app existente — nunca para copiar para dentro do código.
- Motion só existe se estiver na tabela de motion do README.
- Não introduzir biblioteca de UI, de estilo ou de animação.
- Nenhuma mudança de visual pode alterar assinatura em `api.ts`, o schema, ou o comportamento do kanban e do drag.
```

## 3. Primeiro prompt

Cole isso no Claude Code, na raiz do repo:

```
Leia design_handoff_frontend/README.md inteiro, mais mos-tokens.css e mos-design-system.md,
antes de escrever qualquer código.

O back-end e o fluxo já estão prontos. Sua tarefa é a camada visual do apps/desktop:
recriar o desenho do handoff em React + TypeScript com o CSS que já existe, sem
mudar comportamento, sem mudar api.ts e sem adicionar dependência.

Comece pela etapa 1 da "Ordem sugerida de implementação":
1. Importar mos-tokens.css como primeiro arquivo de estilo e empacotar as duas fontes localmente.
2. Varrer App.css e remover todo valor de cor, spacing, radius e duração hardcoded, trocando por token.
3. Me mostrar um diff resumido e a lista do que sobrou sem token antes de seguir.

Não avance para a etapa 2 sem eu aprovar.
```

Depois disso, um prompt por etapa da lista (shell → Home → Inbox → overlays → resto). Uma etapa por vez, com revisão no meio, funciona muito melhor que pedir as seis telas de uma vez.

## 4. Para conferir contra o desenho

Os protótipos abrem no navegador direto: `design_handoff_frontend/design/M-OS Redesign v0.7 - Telas.dc.html`. Clique nos ícones do rail para trocar de tela, `Ctrl+K` abre o Command, `Esc` fecha. Use-os como referência visual lado a lado com o app rodando.
