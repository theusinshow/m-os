# Sonda do Univirtus

Protótipo descartável da investigação de 2026-08-22. **Não é código de produção**
e não deve virar um — o relatório é o produto, e ele está em
`docs/UNIVIRTUS-INTEGRATION.md`.

## O que é

`probe-browser.js` valida, contra a conta real, que os endpoints do relatório
respondem e que o caminho disciplina → semestre → avaliação → trabalho → material
fecha de ponta a ponta.

## Como rodar

1. autentique no AVA pelo navegador, normalmente;
2. abra o console em qualquer página de `https://univirtus.uninter.com/ava/web/`;
3. cole o conteúdo de `probe-browser.js`.

## Por que no console, e não em Node

A API do Univirtus autentica por **cookie `HttpOnly` + header `X-time`**, e o
`X-time` é um segredo emitido no login (§2 do relatório). Rodando na aba, a sonda
reusa a sessão viva através do `$.ajax` da própria página — quem anexa o header é
o `AppConfig.js` do portal. **Nenhum segredo passa por este repositório**: não há
credencial, cookie nem token em lugar nenhum destes arquivos, e a sonda não
imprime valor de sessão.

Uma versão em Node exigiria extrair cookie e `X-time` para fora do navegador, que
é exatamente o que a investigação decidiu não fazer.

## Garantia de leitura

Todas as chamadas são GET, e nenhuma delas está na lista de "GET que alteram
estado" do §10 do relatório. Em particular a sonda **nunca** aciona `Iniciar`
numa avaliação, **nunca** marca aviso como lido e **nunca** chama
`GetAtividadeItemAprendizagemAcessado`.

## Saída esperada

```
[univirtus] sessao valida
[univirtus] 15 disciplinas encontradas
[univirtus] curso: BACHARELADO EM ENGENHARIA CIVIL - DISTÂNCIA (idCurso 5359)
[univirtus] semestres: 2025B2, 2025C1, 2025C2, 2026A1, 2026A2, 2026B1, 2026B2
[univirtus] 2 disciplinas no semestre corrente (2026B2)
[univirtus]   <disciplina>: 7 avaliacoes, 1 trabalhos
[univirtus]   <disciplina>: 0 avaliacoes, 4 trabalhos
[univirtus] 7 avaliacoes, 5 trabalhos, 10 sem nota/entrega
[univirtus] 8 aulas em <disciplina>
[univirtus]   Aula 1: 5 atividades
[univirtus]   0 materiais no roteiro (ids: )
[univirtus]   6 materiais complementares (ids: ...)
```

Contagens variam com a conta e com o momento do semestre. O que importa é que
nenhuma linha diga `sessao invalida` e que os materiais complementares apareçam —
é o caminho mais fácil de quebrar.
