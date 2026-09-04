import type { EstadoDoAparelho } from "../api";
import type { Situacao } from "../notificacoes";

/**
 * O indice, e nao uma gaveta.
 *
 * Gaveta esconde o que e novo atras de um gesto que ninguem descobre sozinho, e
 * o preco disso aparece como "o app nao tem isso". Aqui as paginas que nao
 * couberam na barra ficam listadas, e a configuracao do canal mora embaixo
 * delas — resolvida, ela deixa de merecer o topo de qualquer tela.
 */
export function Mais({
  estado,
  avisos,
  ocupado,
  cobrando,
  aoAtivar,
  aoTestar,
  aoAbrirLembretes,
  aoAbrirAgenda,
}: {
  estado: EstadoDoAparelho | null;
  avisos: Situacao | null;
  ocupado: boolean;
  cobrando: number;
  aoAtivar: () => void;
  aoTestar: () => void;
  aoAbrirLembretes: () => void;
  aoAbrirAgenda: () => void;
}) {
  const canal = avisos?.estado ?? null;
  return (
    <div className="mais">
      <ul className="lista">
        <li className="item">
          <button className="linha-destino" type="button" onClick={aoAbrirAgenda}>
            <div className="item-corpo">
              <p>Agenda</p>
              <small>de ontem a uma semana</small>
            </div>
          </button>
        </li>
        <li className="item">
          <button className="linha-destino" type="button" onClick={aoAbrirLembretes}>
            <div className="item-corpo">
              <p>Lembretes</p>
              <small>
                {cobrando > 0
                  ? `${cobrando} esperando resposta`
                  : "nenhum cobrando agora"}
              </small>
            </div>
          </button>
        </li>
      </ul>

      <section className="canal" data-estado={canal ?? "conferindo"}>
        {canal === "ativo" ? (
          <>
            <p className="canal-linha">
              <i aria-hidden="true" />
              Notificações ativas ·{" "}
              {estado?.aparelhosAvisados === 1
                ? "1 aparelho"
                : `${estado?.aparelhosAvisados ?? 0} aparelhos`}
            </p>
            <button
              className="botao"
              data-variante="quieto"
              type="button"
              disabled={ocupado}
              onClick={aoTestar}
            >
              Enviar um teste agora
            </button>
          </>
        ) : (
          <>
            <p className="rotulo">NOTIFICAÇÃO</p>
            {/* A frase vem antes do botao de proposito: no iPhone o botao so
                funciona depois de instalar na tela de inicio, e um botao que
                falha calado e pior que um botao ausente. */}
            <p className="explicacao">{porQueNaoAtivo(avisos)}</p>

            {/* O que ele vai avisar, dito antes de voce decidir ativar.
                Permissao de notificacao e um sim ou nao sem volta facil no
                iPhone, e conceder sem saber o que vai chegar e o comeco do app
                que a pessoa silencia na semana seguinte. */}
            {canal === "impossivel" ? null : (
              <ul className="promessas">
                <li>Lembretes, na hora em que vencerem.</li>
                <li>Quando o computador mandar coisa nova.</li>
              </ul>
            )}

            {canal === "pronto" ? (
              <button
                className="botao"
                type="button"
                disabled={ocupado}
                onClick={aoAtivar}
              >
                Ativar notificações
              </button>
            ) : null}
          </>
        )}
      </section>

      <p className="rodape-sync">
        {estado?.sincroniza === false
          ? "Este aparelho não alcança o hub. O que você escrever fica guardado aqui até ele voltar."
          : `Sincronizando com o hub · ${estado?.pendentes ?? 0} na fila.`}
      </p>
    </div>
  );
}

/**
 * A frase do canal desligado.
 *
 * Ela existe como funcao para o `motivo` continuar amarrado ao braco que o tem:
 * `pronto` nao carrega motivo nenhum — nao ha o que explicar quando so falta
 * tocar —, e ler o campo com `?.` faria o compilador aceitar um estado a mais do
 * que o tipo descreve.
 */
function porQueNaoAtivo(avisos: Situacao | null): string {
  if (!avisos) return "Conferindo…";
  switch (avisos.estado) {
    case "pronto":
      return "Ative para receber aqui os lembretes que vencerem, mesmo com o app fechado.";
    case "falta":
    case "impossivel":
      return avisos.motivo;
    case "ativo":
      // Inalcancavel: com o canal ativo esta secao nao e desenhada. Devolver uma
      // frase honesta e melhor que um `throw` numa tela que ja abriu.
      return "Notificações ativas.";
  }
}
