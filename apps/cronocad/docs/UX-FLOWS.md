# UX-FLOWS.md — CronoCAD

Fluxos de uso do MVP. Principio: **nunca decidir silenciosamente** sobre o tempo
do usuario; sempre oferecer decisao clara.

## 1. Inicio normal
1. Usuario abre o app (ou ele ja esta na bandeja).
2. Seleciona um projeto no painel.
3. Clica em **Iniciar**. -> cria `active_timer` (status `running`),
   persiste imediatamente, registra evento `timer_started`.
4. O cronometro passa a ser o elemento principal do painel.

## 2. Troca de projeto (com cronometro ativo)
1. Usuario tenta iniciar outro projeto.
2. Como ja existe cronometro ativo, o app **nao inicia silenciosamente**: exibe
   opcoes para **encerrar** ou **pausar** o cronometro anterior.
3. Apos a escolha, inicia o novo cronometro.

## 3. Esquecimento (CAD aberto sem cronometro)
1. Monitoramento detecta `acad.exe` aberto sem cronometro ativo.
2. Notificacao nativa:
   > AutoCAD foi aberto. Em qual projeto voce vai trabalhar?
   > [Selecionar projeto] [Ignorar agora] [Nao lembrar novamente hoje]
3. Ao clicar, abre o app na tela apropriada. **Nao** vincula projeto sem
   confirmacao.

## 4. Inatividade
1. Sem teclado/mouse por mais que o limite (padrao 10 min) -> evento
   `idle_started`; ao retornar, `idle_ended`.
2. O app pergunta:
   > Voce ficou X minutos sem atividade.
   > [Manter como trabalhado] [Descontar X minutos] [Editar periodo]
3. A decisao atualiza `idle_seconds`/duracao do registro. Padrao: **nao**
   descontar automaticamente.

## 5. Fechamento do software CAD
1. Monitoramento detecta fechamento do `acad.exe` com cronometro ativo.
2. Pergunta:
   > AutoCAD foi fechado. Deseja encerrar o registro atual?
   > [Encerrar] [Manter ativo] [Pausar]
3. **Nunca** encerra silenciosamente.

## 6. Recuperacao apos falha
1. Ao abrir, o app encontra um `active_timer` em execucao.
2. Calcula o periodo transcorrido e mostra modal de recuperacao:
   **manter**, **editar** ou **descartar** o periodo.
3. **Nunca** apaga tempo silenciosamente.

## 7. Correcao manual
1. No historico, o usuario edita inicio/fim, descricao, tipo de atividade e
   faturavel; ou adiciona sessao manual (`source = manual`).
2. Validacao de horarios (fim >= inicio; sessao pode atravessar meia-noite).
3. Exclusao com confirmacao; preferir soft delete.

## 8. Reconstrucao do dia
1. "Linha do tempo detectada" lista eventos (`app_opened`/`app_closed`, etc.).
2. Para periodos com CAD aberto sem sessao, oferece
   **Transformar periodo em registro**, com formulario pre-preenchido
   (inicio, fim, duracao, programa, projeto a selecionar, descricao).
3. A sessao criada recebe `source = reconstructed`.

## 9. Geracao de relatorio
1. Usuario filtra por cliente/projeto/periodo.
2. Ve horas reais, inativas, faturaveis, valor/hora e total, por tipo de
   atividade, com lista detalhada.
3. Exporta CSV / imprime pelo sistema. (PDF fica para depois.)

---

## Checklist manual de verificacao

- [ ] Abrir e fechar o AutoCAD (deteccao de abertura/fechamento).
- [ ] Receber notificacao de programa monitorado.
- [ ] Iniciar projeto e ver o cronometro correr.
- [ ] Minimizar para a bandeja.
- [ ] Fechar a janela (deve ir para a bandeja, nao encerrar).
- [ ] Reabrir pela bandeja.
- [ ] Reiniciar o aplicativo durante um cronometro (recuperacao).
- [ ] Voltar apos inatividade e escolher o tratamento.
- [ ] Editar uma sessao no historico.
- [ ] Gerar relatorio e exportar CSV.
