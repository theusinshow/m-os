# M/OS — Dogfood controlado v0.2 Resources

**Data:** 2026-08-13

**Resultado:** aprovado com uma regressão encontrada, corrigida e repetida

**Escopo:** ensaio funcional isolado; não substitui observação de uso pessoal ao longo do tempo

## Ambiente seguro

O dogfood executou em paralelo ao app de desenvolvimento real sem compartilhar identidade, banco, WebView, porta ou artefatos de build:

- identidade: `com.codedbym.mos.dogfood`;
- dados: `%APPDATA%\com.codedbym.mos.dogfood`;
- WebView: `%LOCALAPPDATA%\com.codedbym.mos.dogfood`;
- Vite: `127.0.0.1:1421`;
- target Rust: `%TEMP%\m-os-dogfood-target`;
- configuração reproduzível: `apps/desktop/src-tauri/tauri.dogfood.conf.json`.

O processo real permaneceu em `target\debug\mos-desktop.exe` e o dogfood em `%TEMP%\m-os-dogfood-target\debug\mos-desktop.exe`. Abrir o executável dogfood uma segunda vez retornou código `0` e manteve exatamente duas instâncias persistentes — real e dogfood — confirmando o isolamento de single instance.

## Cenários exercitados

| Cenário | Observado | Resultado |
|---|---|---|
| Criação direta com `ftp://` | erro claro; URL, título e `Por quê?` permaneceram editáveis | aprovado |
| Correção para HTTPS | Resource criado e exibido na Library | aprovado |
| Busca por `Por quê?` | `cometa violeta` encontrou o Resource sem abrir o navegador | aprovado |
| Edição | título e nota atualizados; busca encontrou `revisado` | aprovado |
| Quick Capture → Inbox → Resource | URL pré-preenchida, título e nota salvos | aprovado |
| Proveniência | detalhe mostrou origem; Capture abriu como `Processada` | aprovado |
| Archive e busca | oculto da busca normal e encontrado com `Incluir arquivados` | aprovado após correção |
| Resource arquivado | detalhe exibido; `Abrir link` reportado como desabilitado pela UI Automation | aprovado |
| Restore, Trash e Restore | ciclos concluídos pelo detalhe e por Settings | aprovado |
| Undo | `Ctrl+Z` restaurou o Resource dentro da janela de 8 segundos | aprovado |
| Teclado da Library | `ArrowDown`, `Home`, `End`, `Enter` e `Esc` moveram seleção, abriram detalhe e devolveram foco | aprovado |
| Layout mínimo | em `840×600`, lista e detalhe ficaram em painéis exclusivos com retorno explícito | aprovado |

Fixtures reservados usados: `Dogfood direto editado`, `cometa violeta revisado`, `Dogfood via Inbox` e `origem âmbar — validar proveniência`.

## Atrito encontrado e decisão

**S1 — recuperação:** depois de arquivar o Resource atualmente roteado, abri-lo novamente pelo mesmo resultado de busca fechava o Command, mas a Library mantinha outro item no detalhe. A causa era a intenção de navegação repetir o mesmo ID no estado pai, enquanto a seleção local já havia mudado.

A correção adicionou uma chave monotônica à intenção de abrir Resource. A Library agora reage a cada abertura, inclusive quando o ID se repete. O cenário foi repetido: o Resource arquivado apareceu na lista com estado explícito, o detalhe abriu corretamente e a ação nativa permaneceu desabilitada.

## Evidências de acessibilidade e limites

A árvore UI Automation expôs landmarks, título da Library, grupo de Resources, detalhe, formulários, estado arquivado e ações. O provedor Orca não conseguia enviar teclado por sua guarda de foreground nessa sessão; o foco e as teclas foram aplicados pela UI Automation do Windows ao mesmo elemento e processo isolados. O padrão nativo `ExpandCollapse` foi usado para disclosures.

Não foram alterados tema High Contrast nem scaling global do Windows, pois isso afetaria a sessão do usuário. Esses cenários, junto do Narrator, permanecem no checklist manual de release. Tema claro/escuro e contrastes foram revisados programaticamente; `forced-colors` usa cores de sistema.

## Gate restante antes de frontend pesado

O fluxo mínimo está tecnicamente pronto para uso real. O próximo sinal não é outra camada visual: é salvar e reencontrar links reais durante alguns dias e registrar se `Por quê?` reduz a necessidade de reconstruir contexto. Só fricções observadas nesse uso podem justificar grid, metadata remota, tags, taxonomia ou relações adicionais.
