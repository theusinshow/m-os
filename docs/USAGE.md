# USAGE.md — Guia de uso do CronoCAD

Guia rapido para o usuario final. O CronoCAD roda localmente no Windows e mantem
todos os dados no seu computador (nada depende de internet).

## Instalacao

1. Execute o instalador `CronoCAD_x.y.z_x64-setup.exe`.
2. Abra o **CronoCAD**. O aplicativo fica disponivel na **bandeja do sistema**.

O banco de dados e criado automaticamente na primeira execucao, no diretorio de
dados do aplicativo do seu usuario.

## Primeiros passos

1. **Cadastre um projeto** em *Projetos → Novo projeto* (informe o nome, um
   codigo opcional, o **valor/hora** e, se quiser, o cliente). Clientes sao
   gerenciados pelo botao *Clientes* na mesma tela.
2. No **Painel**, escolha o projeto, o tipo de atividade e clique em **Iniciar**.
3. Use **Pausar/Continuar** e **Encerrar** conforme o trabalho. Ao encerrar, a
   sessao vai para o **Historico** com o valor calculado.

Existe **no maximo um cronometro ativo** por vez. Para trocar de projeto,
encerre ou pause o atual primeiro.

## Deteccao de programas (AutoCAD, Revit, …)

Em *Configuracoes → Programas monitorados* voce controla quais executaveis sao
observados (ja vem AutoCAD, Revit, SketchUp, Eberick e QiBuilder; habilite os
que usa). Quando um programa monitorado abre **sem** cronometro ativo, o app
lembra de escolher um projeto; ao fechar **com** cronometro ativo, pergunta se
deseja encerrar, manter ou pausar. Voce pode dizer *Nao lembrar hoje*.

> O app le apenas **o nome** dos programas em execucao — nunca o conteudo dos
> desenhos, teclas digitadas ou telas.

## Inatividade

Se voce ficar sem usar teclado/mouse por mais que o limite configurado (padrao
10 min) com um cronometro rodando, ao voltar o app pergunta o que fazer com esse
periodo: **manter como trabalhado**, **descontar** ou **editar** os minutos. O
tempo real e sempre preservado; o desconto afeta so a duracao liquida e o valor.

## Recuperacao apos fechar inesperadamente

Se o app for fechado com um cronometro em andamento, na proxima abertura ele
oferece **manter em execucao**, **encerrar registro** ou **descartar** — nunca
apaga tempo sozinho.

## Historico

Em *Historico* voce filtra por periodo, cliente e projeto; edita inicio/fim,
descricao, atividade e faturavel; **adiciona sessoes manuais**; e **exclui**
(com confirmacao). Sessoes excluidas podem ser **restauradas** em *Mostrar
excluidas*.

## Linha do tempo detectada

Em *Linha do tempo* voce ve os eventos do dia (programas abertos/fechados,
cronometro, inatividade) e as **lacunas** — periodos com programa aberto sem
sessao registrada. Clique em *Transformar em registro* para criar uma sessao a
partir da lacuna (marcada como reconstruida).

## Relatorios

Em *Relatorios* voce filtra por periodo/cliente/projeto e ve horas reais,
inativas, faturaveis e o valor total, com separacao por tipo de atividade e a
lista detalhada. Exporte em **CSV** ou use **Imprimir** para gerar um PDF pelo
sistema. Se o **arredondamento** estiver ativo em Configuracoes, ele e aplicado
apenas na visualizacao/cobranca — o tempo real no banco nao muda.

## Bandeja e janela

- Fechar a janela **minimiza para a bandeja** (o app continua monitorando).
- Pelo menu da bandeja voce abre o app e controla pausar/continuar/encerrar.
- **Sair completamente** encerra o app. Um cronometro em andamento fica
  preservado e sera oferecido para recuperacao na proxima abertura.

## Configuracoes

Ajuste monitoramento (ligar/desligar, intervalo, avisos), inatividade (limite),
arredondamento (intervalo e modo), comportamento (bandeja, iniciar com o
Windows) e regiao (moeda, idioma). Clique em **Salvar**.

## Checklist manual

Um roteiro de verificacao manual esta em [UX-FLOWS.md](UX-FLOWS.md).
