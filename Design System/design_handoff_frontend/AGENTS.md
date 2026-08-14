# AGENTS.md — M/OS

Instruções permanentes para qualquer agente de código trabalhando neste repositório.
Leia `handoff/mos-design-system.md` antes de escrever ou alterar qualquer interface. Ele é a fonte de verdade de design; este arquivo é a fonte de verdade de processo.

---

## 1. Regra zero

Design é parte estrutural deste produto, não acabamento. Uma feature funcionando fora da especificação visual **não está pronta**.

Se a especificação e o seu instinto de "boa prática de UI" conflitarem, siga a especificação. Se ela não cobrir o caso, **pergunte antes de inventar** — e proponha a regra nova em uma frase, para entrar no documento.

---

## 2. Tokens são obrigatórios

Importe `handoff/mos-tokens.css` uma vez, no entrypoint da aplicação.

- Proibido literal de cor (`#`, `rgb(`, `hsl(`) em qualquer arquivo de componente.
- Proibido tamanho, padding, radius ou duração fora da escala de tokens.
- Precisa de um valor que não existe? Não crie no componente: proponha o token.
- Dark é o padrão. Light é ativado por `data-theme="light"` no `<html>` e é **paridade completa** — nunca `filter: invert`, nunca derivar um modo do outro.

Antes de abrir PR, rode uma busca por `#[0-9a-fA-F]{3,6}` nos arquivos de UI. O resultado esperado é zero, fora de `mos-tokens.css`.

---

## 3. Como construir um componente

1. Leia a seção correspondente em `mos-design-system.md` inteira.
2. Implemente os **cinco estados**: repouso, hover, focus, ativo, bloqueado.
3. Garanta operação por teclado e foco visível.
4. Verifique nos dois modos.
5. Rode o checklist da seção 11 da especificação.

Componentes devem ser stateless e sem estilo próprio de layout. Espaçamento entre componentes é responsabilidade de quem os compõe, usando `gap` — nunca margens dentro do componente.

Use `display: flex` / `grid` com `gap` para qualquer grupo de irmãos. Nunca espaçamento por whitespace inline ou margem por elemento.

---

## 4. As cinco violações mais prováveis

Estas são as coisas que agentes de código erram neste projeto. Confira explicitamente antes de entregar:

1. **Mono vazando para conteúdo.** Fonte monoespaçada é só dado de sistema: timestamp, caminho, id, atalho, contagem, tipo. Nunca título, nome, texto do usuário, rótulo de botão ou mensagem.
2. **Card genérico.** Não existe card. Existe Panel em três variantes, e a variante padrão é a **nua** (sem borda e sem fundo). Sombra só em overlay flutuante.
3. **Accent virando decoração.** O âmbar é sinal: focus, seleção, a barra, progresso, primário, autoria de Hermes. Não é hover, não é borda de container, não é ícone inativo.
4. **Estado só por cor.** Warning e erro sempre com ícone e frase. Warning não tem cor.
5. **Motion longo.** Nada recorrente acima de 200ms. Zero bounce, zero skeleton pulsante, zero spinner abaixo de 300ms.

---

## 5. A barra `/`

É o elemento proprietário da marca e aparece em quatro papéis funcionais: caminho de contexto, limiar de campo, prefixo de comando, marcador de autoria/seleção (a versão vertical de 2px).

Sempre em `--font-system` e `--signal-ink`. Nunca dentro de conteúdo do usuário, nunca como divisor decorativo, nunca em outra cor.

---

## 6. Capture e Command são o produto

Antes de otimizar qualquer outra tela, essas duas precisam estar impecáveis.

- Capture não tem caixa: barra `/`, texto em 21px, linha de base.
- `Enter` salva **imediatamente**, com ou sem interpretação pronta. Nunca aguarde IA, rede ou validação para persistir o texto cru.
- A interpretação (Hermes) chega depois e é sempre editável e reversível.
- Quick Capture é overlay de 640px a 34% da altura, **nunca tela cheia**, entrada em 160ms e saída em 90ms.
- `Ctrl+Shift+Space` abre o Quick Capture global; `⌘K`/`Ctrl+K` abre Command; `Esc` cancela sem salvar; `⌘Z` desfaz.

Latência percebida é requisito de design, não de performance: se o overlay não aparece em ~160ms na máquina do usuário, o produto falhou mesmo que esteja bonito.

---

## 7. Escopo e disciplina

- Não adicione bibliotecas de UI (nem Material, nem Chakra, nem shadcn como skin). O design system é este; componentes são escritos à mão sobre os tokens.
- Não adicione telas, seções ou features que não foram pedidas.
- Não "melhore" visualmente o que não foi solicitado. Mudança de estilo fora do escopo do pedido é regressão.
- Ícones: SVG próprio, stroke 1.5 em 24px / 1.25 em 20px / 1 em 16px, terminais retos, um desenho por tamanho. Não instale pacote de ícones.
- Sem emoji na interface.

---

## 8. Ordem de trabalho

1. Tokens.
2. Capture e Command.
3. Row e Panel.
4. Navigation e overlays.
5. Controls e feedback.

Não pule etapas: cada uma é a referência visual da seguinte.
