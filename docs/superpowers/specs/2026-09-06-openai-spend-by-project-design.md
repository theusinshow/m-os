# OpenAI na faixa — gasto, limite e restante por projeto

**Status:** aprovado para implementação

**Data:** 2026-09-06

**Origem:** NexoDoc, Truss e Hermes usam a mesma organização da OpenAI. O dono
precisa saber quanto cada projeto gastou no mês, quanto ainda cabe no limite de
cada projeto e quanto resta no limite total da organização.

## 1. O fato que torna a leitura possível

A API administrativa oficial da OpenAI expõe quatro peças que antes não
existiam juntas:

- `GET /v1/organization/projects` lista os projetos e seus nomes;
- `GET /v1/organization/costs`, agrupado por `project_id`, devolve o custo
  financeiro reconciliado;
- `GET /v1/organization/spend_limit` devolve o limite rígido mensal da
  organização;
- `GET /v1/organization/projects/{project_id}/spend_limit` devolve o limite
  rígido mensal de cada projeto.

As quatro exigem uma Admin API Key. A chave não entra no `settings.json`, não
atravessa de volta ao renderer e não vai para logs: vive no armazenamento seguro
da plataforma.

O restante é derivado, nunca recebido como se fosse saldo bancário:

```text
restante do mês = limite rígido mensal - custo reconciliado do mês
```

Por isso a tela escreve **limite mensal**, e não “créditos da conta”. Crédito
pré-pago, expiração, recarga e ajuste de fatura são conceitos diferentes e a API
consultada não os promete.

## 2. A unidade

O backend trabalha em **micros de dólar** (`US$ 1 = 1.000.000 micros`). Custo de
modelo pode ser menor que um centavo; arredondar cada bucket para centavos antes
de somar perderia dinheiro real. O limite chega em centavos e é convertido para
micros apenas para a conta.

O renderer recebe inteiros. `f64` fica confinado à borda que desserializa o JSON
da OpenAI e é arredondado uma vez ao entrar.

## 3. Leitura e falha

`crates/mos-usage::openai` conhece formatos e cálculos, mas não rede, Tauri,
Credential Manager nem Keychain. O shell faz os pedidos e entrega os corpos ao
parser puro.

Uma leitura reúne projetos, custos e limites num retrato em memória. Ela não vai
para SQLite: a OpenAI já é a fonte, e persistir uma cópia criaria um número velho
com aparência de atual. Ao falhar, o último retrato bom continua visível marcado
como desatualizado; a mensagem de erro fica em Settings.

Atualização automática a cada cinco minutos e botão explícito em Settings. Não
há pedido de rede no caminho de Capture, boot do banco ou interação da faixa.

## 4. A faixa

OpenAI ocupa um dos três anéis previstos pela ADR-063. A régua é:

```text
custo da organização no mês / limite rígido mensal
```

Sem limite configurado, a fonte continua aparecendo no painel com custo
absoluto, mas o anel não pinta proporção. Ausência nunca vira zero.

O painel mostra uma síntese da organização e uma lista compacta de projetos,
ordenada por maior gasto. Cada linha diz gasto, limite e restante quando o
projeto possui limite; sem ele, diz somente o gasto e “sem limite”. Não nasce
uma nova dashboard nem um item no rail.

## 5. Atribuição correta

O custo só pode ser atribuído corretamente se NexoDoc, Truss e Hermes forem
projetos distintos na organização da OpenAI e usarem chaves pertencentes aos
respectivos projetos. Uma chave compartilhada entre os três apaga a
proveniência na origem; o M/OS não tenta adivinhar depois.

Resultados sem `project_id` aparecem como **Sem projeto**, em vez de serem
distribuídos artificialmente.

## 6. Checklist multi-device

- **Core:** não se aplica. É leitura de integração externa; nenhuma regra do
  cérebro pessoal nasce em `mos-core`.
- **Database:** nenhuma migration. O retrato financeiro é remoto e efêmero.
- **Sync:** não sincroniza. Cada dispositivo consulta a mesma fonte autoritativa;
  sincronizar snapshots duplicaria cache. O segredo permanece por dispositivo.
- **Desktop:** Admin API Key em Settings/Conexões; anel OpenAI e detalhamento no
  painel da faixa; atualização automática e manual.
- **iOS:** mesma leitura e os mesmos cálculos, com requisição nativa e chave no
  Keychain. A manifestação será uma lista compacta; a faixa flutuante não se
  aplica porque iOS não oferece janela always-on-top. O shell iOS ainda não
  existe e não pode ser compilado nesta máquina Windows.
- **Notifications:** nenhuma nesta versão. A faixa já é a superfície ambiente;
  alertas remotos exigiriam política própria e push, que ainda não existe.
- **Hermes:** não age. Leitura futura poderá responder “quanto o Hermes gastou
  este mês?”, sem expor a Admin API Key ao agente.
- **Tests:** parser de projetos, custos, limites e paginação; conversão para
  micros; soma por projeto; restante abaixo e acima do limite; régua sem teto;
  tipos/renderer; e verificação visual da faixa e Settings.

## 7. Fora desta versão

- editar limites ou criar alertas na OpenAI;
- mostrar saldo de crédito pré-pago;
- histórico local navegável ou gráficos;
- custo por modelo, chave ou usuário;
- converter USD para BRL;
- notificar ou bloquear projetos.

