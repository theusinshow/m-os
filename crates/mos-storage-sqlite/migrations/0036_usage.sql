-- O consumo de IA lido dos transcripts locais do Claude Code.
--
-- # Por que a chave primaria e o `request_id`
--
-- Nao e escolha de estilo: e o unico jeito de o numero estar certo. O maior
-- transcript desta maquina foi contado antes do desenho e tem 3277 linhas com
-- `usage` para 2108 `requestId` unicos. Um terco das linhas repete um request
-- ja gravado — o Claude Code reescreve a mesma mensagem de assistente em
-- situacoes que nao dependem de nos, e somar linha a linha inflaria o consumo
-- em cerca de 55%.
--
-- Com a chave primaria aqui, `INSERT OR IGNORE` resolve a duplicacao por
-- construcao, e nao por disciplina de quem escreve o laco.
--
-- # O que isso compra alem da corretude
--
-- A varredura fica IDEMPOTENTE. Reprocessar um arquivo inteiro — porque o
-- offset se perdeu, porque o arquivo foi reescrito, porque o banco e novo —
-- chega exatamente ao mesmo resultado. Por isso `usage_fonte` pode ser tratada
-- como otimizacao pura: um bug de offset vira lentidao, nunca numero errado.
--
-- # Sem chave estrangeira para o dominio
--
-- Nada aqui aponta para `tasks`, `projects` ou `captures`. Consumo de IA e
-- observacao de uma ferramenta de TERCEIRO, e amarra-lo ao dominio faria uma
-- mudanca de formato do Claude Code virar problema de integridade do M/OS.

CREATE TABLE usage_requisicao (
    request_id    TEXT PRIMARY KEY,
    em            TEXT NOT NULL,
    modelo        TEXT NOT NULL,
    -- O inicio da janela de 5h em que este request caiu, ja arredondado para a
    -- hora cheia. Guardado e nao derivado na consulta porque ele e o eixo de
    -- todo agrupamento, e recalcula-lo em SQL a cada leitura custaria uma
    -- funcao de data por linha.
    janela_inicio TEXT NOT NULL,
    -- Milesimos de token-equivalente-de-input. Milesimos porque o cache lido
    -- pesa 0,1 e arredondar isso para zero apagaria a parcela mais frequente.
    peso          INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_usage_requisicao_janela ON usage_requisicao(janela_inicio);

-- Onde a leitura de cada arquivo parou. Otimizacao, nao corretude: ver acima.
--
-- Sao 507 MB em 18 projetos nesta maquina. Sem esta tabela, cada tique de 30s
-- releria meio giga de disco.
CREATE TABLE usage_fonte (
    caminho TEXT PRIMARY KEY,
    -- O byte seguinte a ultima linha COMPLETA lida. Uma linha sem `\n` no fim
    -- esta sendo escrita agora e nao avanca o offset.
    offset  INTEGER NOT NULL,
    tamanho INTEGER NOT NULL,
    mtime   INTEGER NOT NULL
) STRICT;

-- O agregado por janela de 5h.
--
-- Derivada de `usage_requisicao`, e existe por uma razao so: o denominador do
-- anel e o PICO historico, e ele precisa sair de um `SELECT MAX(peso)` em vez
-- de uma soma sobre centenas de milhares de linhas a cada 30 segundos.
CREATE TABLE usage_janela (
    inicio      TEXT PRIMARY KEY,
    fim         TEXT NOT NULL,
    peso        INTEGER NOT NULL,
    requisicoes INTEGER NOT NULL
) STRICT;

PRAGMA user_version = 36;
