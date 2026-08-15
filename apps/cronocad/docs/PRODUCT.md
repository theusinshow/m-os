# PRODUCT.md — CronoCAD

## Problema

Desenhistas cadistas trabalham diariamente em varios desenhos, obras e projetos
(AutoCAD, Revit, etc.) e enfrentam dois problemas:

1. **Esquecem de registrar** quando comecaram e terminaram cada projeto.
2. Na hora de **cobrar**, nao sabem exatamente quantas horas trabalharam.

Cronometros tradicionais dependem de o usuario lembrar de iniciar/parar — o que
falha justamente porque o usuario esquece.

## Publico

Profissionais autonomos e pequenos escritorios de desenho/projeto tecnico que
usam softwares CAD no Windows e cobram por hora.

## Proposta

Aplicativo desktop **local-first** que reduz o esforco e o esquecimento:

- cronometro por projeto (um ativo por vez);
- deteccao de programas CAD abertos, com lembrete para escolher o projeto;
- deteccao de inatividade, com decisao do usuario sobre o periodo;
- reconstrucao aproximada do dia a partir de eventos detectados;
- relatorios de horas e valores para cobranca (com exportacao CSV);
- funcionamento offline e persistencia entre reinicializacoes.

## Casos de uso principais

1. **Inicio normal:** abrir o app, escolher projeto, iniciar cronometro.
2. **Esquecimento:** abrir o AutoCAD sem cronometro -> receber lembrete para
   selecionar o projeto.
3. **Troca de projeto:** ao iniciar outro projeto com um cronometro ativo, o
   app oferece encerrar/pausar o anterior (nunca troca silenciosamente).
4. **Inatividade:** apos X minutos sem uso, registrar o periodo e perguntar ao
   usuario se mantem, desconta ou edita.
5. **Fechamento do CAD:** ao fechar o AutoCAD com cronometro ativo, perguntar se
   encerra, mantem ou pausa.
6. **Recuperacao:** apos fechamento inesperado, ao reabrir, oferecer manter,
   editar ou descartar o periodo do cronometro que estava ativo.
7. **Correcao manual:** editar inicio/fim, descricao e tipo de atividade.
8. **Cobranca:** gerar relatorio por cliente/projeto/periodo e exportar CSV.

## Escopo do MVP

Cadastro de clientes e projetos; valor/hora; cronometro com pausa/retomada/
encerramento e recuperacao; deteccao de CAD e de inatividade com decisao do
usuario; historico com edicao/filtragem; relatorios com CSV; bandeja do
sistema; tudo offline e persistente. Interface em pt-BR, modo escuro.

## Nao objetivos (agora)

Login, contas, Supabase, sincronizacao, painel web, mobile, assinaturas, nota
fiscal, integracao com DWG, plugin de AutoCAD, IA, equipes, captura de tela,
financas completas, integracao com calendarios, leitura do nome do desenho.
(Ver lista completa na secao 25 do documento do produto.)

## Criterios de sucesso (MVP)

O MVP estara funcional quando os 17 criterios de aceite forem atendidos
(instalar/abrir no Windows; dados locais; criar cliente/projeto; valor/hora;
unico cronometro ativo; pausa/retomada/encerramento; sobreviver a
reinicializacao; lembretes de abertura/fechamento do AutoCAD; deteccao e
tratamento de inatividade; correcao manual; historico filtravel; calculo
correto de horas/valores; relatorio + CSV; funcionar na bandeja; sem dependencia
de internet). Detalhe em `docs/ROADMAP.md`.
