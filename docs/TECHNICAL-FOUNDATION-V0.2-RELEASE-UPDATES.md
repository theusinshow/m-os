# M/OS - Technical Foundation v0.2 Release Updates

## 1. Objetivo

Este corte adiciona atualizacao manual dentro do desktop para reduzir reinstalacoes
durante dogfooding.

O objetivo e permitir:

- verificar se existe uma versao nova;
- baixar um pacote assinado;
- instalar a atualizacao sem exigir admin quando a instalacao for por usuario;
- reiniciar o M/OS depois da atualizacao.

## 2. Escopo

Deve existir:

- botao em Settings para verificar atualizacoes;
- feedback para versao atual, versao disponivel, download, instalacao e erro;
- updater oficial do Tauri;
- artefatos de updater assinados;
- `latest.json` publicado em GitHub Releases;
- workflow de pacote capaz de publicar Release quando uma tag `v*` for criada.

Nao deve existir ainda:

- atualizacao silenciosa automatica na abertura;
- canal beta/stable configuravel;
- servidor proprio de update;
- sincronizacao cloud;
- rollback automatico;
- updater portatil customizado que substitui o executavel em uso.

## 3. Decisao tecnica

O M/OS usa o Tauri Updater com endpoint estatico:

```text
https://github.com/theusinshow/m-os/releases/latest/download/latest.json
```

O app contem apenas a chave publica do updater. A chave privada fica fora do
repositorio e e usada pelo GitHub Actions atraves de secret.

## 4. Distribuicao

O workflow `Package Windows` continua publicando artifacts para teste rapido.

Quando a referencia for uma tag `v*`, o workflow tambem:

1. compila o NSIS por usuario;
2. gera assinatura do updater;
3. cria `latest.json`;
4. publica ou atualiza o GitHub Release da tag.

## 5. Limites operacionais

O updater so encontra versoes publicadas como GitHub Release. Artifacts comuns
do Actions continuam uteis para teste manual, mas expiram e nao sao fonte de
atualizacao do app instalado.

No Windows, a etapa de instalacao do updater encerra o aplicativo durante a
atualizacao. O fluxo da UI deve tratar isso como comportamento esperado.

