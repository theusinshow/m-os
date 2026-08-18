/**
 * Fuma a Action API do M-Finance — a porta pela qual o Hermes cria contas.
 *
 * Existe porque o QA dessa feature e caro: exige o M/OS aberto, o secret no
 * Credential Manager, `can_write` marcado no App e uma conversa com o Hermes.
 * A metade servidor do fluxo, porem, nao precisa de nada disso — e so um POST
 * autenticado. Este script cobre essa metade em segundos, deixando para o
 * teste manual so o que e de fato interface.
 *
 * Os casos negativos nao escrevem nada: sao recusados antes do INSERT. Por
 * isso rodam por padrao, inclusive contra producao. A criacao de verdade fica
 * atras de `--create`, porque essa sim deixa uma linha no banco do usuario.
 *
 *   MOS_ACTION_SECRET=... node scripts/smoke-m-finance-action.mjs
 *   MOS_ACTION_SECRET=... node scripts/smoke-m-finance-action.mjs --create
 *
 * Sai com codigo 1 se algum caso reprovar, para o CI poder barrar.
 */

const URL_PADRAO = "https://m-finance-silk.vercel.app/api/mos/actions";

const url = process.env.MOS_ACTION_URL ?? URL_PADRAO;
const secret = process.env.MOS_ACTION_SECRET ?? "";
const criar = process.argv.includes("--create");

if (!secret) {
  console.error("Falta MOS_ACTION_SECRET no ambiente — e o mesmo valor que esta na Vercel.");
  process.exit(1);
}

async function chamar(body, authorization) {
  const resposta = await fetch(url, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      ...(authorization ? { authorization } : {}),
    },
    body: JSON.stringify(body),
  });

  const texto = await resposta.text();
  let json = null;
  try {
    json = JSON.parse(texto);
  } catch {
    // Deixa `json` nulo: um corpo nao-JSON ja e o achado interessante.
  }

  return { status: resposta.status, json, texto };
}

const CONTA_VALIDA = {
  amountCents: 100,
  description: "TESTE M/OS — pode apagar",
  dueDay: 15,
  isRecurring: false,
};

const casos = [
  {
    nome: "sem header de authorization e recusado",
    esperado: 401,
    executar: () => chamar({ actionId: "m-finance.create_bill", args: CONTA_VALIDA }),
  },
  {
    nome: "secret errado e recusado",
    esperado: 401,
    executar: () =>
      chamar({ actionId: "m-finance.create_bill", args: CONTA_VALIDA }, "Bearer secret-errado"),
  },
  {
    nome: "secret sem o prefixo Bearer e recusado",
    esperado: 401,
    executar: () => chamar({ actionId: "m-finance.create_bill", args: CONTA_VALIDA }, secret),
  },
  {
    nome: "acao desconhecida e recusada",
    esperado: 400,
    executar: () => chamar({ actionId: "m-finance.apagar_tudo", args: {} }, `Bearer ${secret}`),
  },
  {
    nome: "valor zero e recusado antes de gravar",
    esperado: 422,
    executar: () =>
      chamar(
        { actionId: "m-finance.create_bill", args: { ...CONTA_VALIDA, amountCents: 0 } },
        `Bearer ${secret}`,
      ),
  },
  {
    nome: "dia 32 e recusado antes de gravar",
    esperado: 422,
    executar: () =>
      chamar(
        { actionId: "m-finance.create_bill", args: { ...CONTA_VALIDA, dueDay: 32 } },
        `Bearer ${secret}`,
      ),
  },
  {
    nome: "descricao vazia e recusada antes de gravar",
    esperado: 422,
    executar: () =>
      chamar(
        { actionId: "m-finance.create_bill", args: { ...CONTA_VALIDA, description: "   " } },
        `Bearer ${secret}`,
      ),
  },
];

if (criar) {
  casos.push({
    nome: "cria a conta de teste de verdade",
    esperado: 200,
    escreve: true,
    executar: () =>
      chamar({ actionId: "m-finance.create_bill", args: CONTA_VALIDA }, `Bearer ${secret}`),
  });
}

console.log(`Alvo: ${url}`);
console.log(criar ? "Modo: com escrita (--create)\n" : "Modo: so leitura (use --create para gravar)\n");

let reprovados = 0;

for (const caso of casos) {
  let resultado;
  try {
    resultado = await caso.executar();
  } catch (error) {
    console.log(`  FALHOU  ${caso.nome}`);
    console.log(`          nao consegui falar com o servidor: ${error.message}`);
    reprovados += 1;
    continue;
  }

  const passou = resultado.status === caso.esperado;
  console.log(`  ${passou ? "ok    " : "FALHOU"}  ${caso.nome}`);

  if (!passou) {
    console.log(`          esperava ${caso.esperado}, veio ${resultado.status}`);
    console.log(`          corpo: ${resultado.texto.slice(0, 300)}`);
    reprovados += 1;
    continue;
  }

  if (caso.escreve && resultado.json?.billId) {
    console.log(`          conta criada: ${resultado.json.billId} — apague no M-Finance`);
  }
}

console.log(`\n${casos.length - reprovados}/${casos.length} passaram.`);

// `process.exitCode` e nao `process.exit()`: sair a forca com as conexoes
// keep-alive do fetch ainda abertas derruba o libuv no Windows
// ("Assertion failed: !(handle->flags & UV_HANDLE_CLOSING)") e o codigo de
// saida vira 127, escondendo o 1 que o CI precisa ler. Assim o processo
// termina sozinho quando os sockets fecham, com o codigo certo.
process.exitCode = reprovados > 0 ? 1 : 0;
