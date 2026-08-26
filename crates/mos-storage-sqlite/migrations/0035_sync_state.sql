-- O estado reconciliado de cada entidade, guardado ao lado do dominio.
--
-- # Por que uma tabela sombra, e nao um relogio por coluna
--
-- A reconciliacao e POR CAMPO, e decidir um campo exige saber o instante que
-- colocou o valor atual ali. As tabelas de dominio nao guardam isso: `tasks`
-- tem `title`, nao "o HLC que escreveu o title". Sem esse instante, o valor
-- local nao tem com que competir e perderia de qualquer operacao que chegasse
-- — inclusive de uma mais velha.
--
-- A alternativa seria uma coluna de relogio por coluna de dominio, ou uma
-- tabela com uma linha por campo. As duas espalham a mecanica de sync por
-- dentro do dominio, e o desenho inteiro do M/OS existe para manter as duas
-- coisas separadas. Aqui o estado reconciliado mora num lugar so, em JSON, e o
-- dominio continua sendo dominio.
--
-- # Quem escreve aqui
--
-- As DUAS direcoes, e essa e a parte que importa. A mudanca local passa por
-- `emitir()`, que aplica a operacao sobre este estado na mesma transacao; a
-- mudanca remota passa pela `Projecao` do motor. Se so a remota escrevesse,
-- uma edicao local ficaria invisivel para a reconciliacao e perderia de uma
-- operacao antiga que chegasse depois.
--
-- # O que e a fonte da verdade
--
-- Para RECONCILIAR, esta tabela. Para LER, o dominio — que e o que as telas,
-- as buscas e os relatorios consultam. A `Projecao` materializa uma na outra, e
-- e por isso que as duas nunca divergem sem alguem ter errado no caminho.

CREATE TABLE sync_state (
    entity_kind TEXT NOT NULL,
    entity_id   TEXT NOT NULL,
    -- `EstadoDaEntidade` serializado: campos com seus instantes, o apagamento
    -- logico e se a entidade ja foi vista existir.
    estado      TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    PRIMARY KEY (entity_kind, entity_id)
) STRICT;

PRAGMA user_version = 35;
