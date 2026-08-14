# M/OS — Technical Foundation v0.2 App Catalog

## 1. Objetivo

Este corte permite que ferramentas próprias já conhecidas entrem no App Registry sem repetir cadastro manual e sem confundir o repositório de origem com o alvo usado para abrir o App.

O catálogo inicial é deliberadamente pequeno e contém:

- CronoCAD;
- M Finance;
- Coded Atlas.

## 2. Evidências observadas

Os três projetos pertencem à conta GitHub `theusinshow`.

| App | Repositório | Forma observada de uso |
|---|---|---|
| CronoCAD | `theusinshow/cronocad` | desktop Tauri; não possui release publicada |
| M Finance | `theusinshow/m-finance` | aplicação web publicada |
| Coded Atlas | `theusinshow/coded-atlas` | aplicação Next.js com URL publicada e execução local opcional |

CronoCAD não recebe um alvo remoto inventado. O catálogo preserva sua origem e deixa o alvo de abertura vazio até existir instalação ou caminho local escolhido pelo usuário.

## 3. Modelo

`RegisteredApp` passa a aceitar `source_url` opcional.

As responsabilidades ficam separadas:

- `source_url`: endereço informativo da origem do software, inicialmente um repositório HTTPS;
- `launch_kind` e `launch_target`: alvo operacional aceito pelo sistema para abrir o App;
- uma origem nunca é executada implicitamente;
- ausência de alvo não impede cadastro, busca ou relação com Workspace.

`source_url` é metadata inerte. Ela não cria autenticação, leitura de repositório, sincronização, Issues, Pull Requests ou qualquer outra Integration com GitHub.

## 4. Catálogo conhecido pelo Core

O catálogo é exposto como dados estáveis do Core para que clientes não mantenham listas divergentes.

Cada entrada possui:

- identificador estável;
- nome;
- descrição;
- origem HTTPS;
- alvo inicial opcional.

O cadastro é explícito e idempotente:

1. o usuário escolhe adicionar Apps conhecidos;
2. o backend valida os IDs solicitados;
3. registros existentes são identificados por origem ou nome normalizado;
4. metadata ausente pode ser enriquecida, mas alvo já configurado pelo usuário não é sobrescrito;
5. Apps ausentes são criados na mesma transação;
6. repetir a operação não cria duplicatas.

## 5. Busca, backup e segurança

- nome, descrição, alvo e origem participam da projeção FTS local;
- `source_url` participa do export JSON e do banco contido em backups;
- somente `http://` e `https://` são aceitos como origem;
- abrir origem e abrir App continuam sendo intenções diferentes;
- a política nativa de abertura existente continua valendo somente para `launch_target`;
- nenhuma chamada à API do GitHub acontece em runtime.

## 6. Limites deste corte

Não entram:

- descoberta automática de todos os repositórios da conta;
- tokens ou OAuth do GitHub;
- clonagem, pull ou build de repositórios;
- associação Project → Repository;
- importação de Issues, commits ou Pull Requests;
- integração profunda com dados internos de CronoCAD, M Finance ou Coded Atlas;
- detecção genérica de executáveis instalados.

Essas capacidades pertencem às fases de Integration e Cross-App e exigem decisões próprias.

## 7. Próximo passo

O catálogo foi validado com os três Apps conhecidos. A fundação backend seguinte, `Resource`, está documentada em `TECHNICAL-FOUNDATION-V0.2-RESOURCES.md`. Library deve continuar sendo projeção de Resources, não uma entidade nem um container proprietário.
