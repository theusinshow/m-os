import { useCallback, useEffect, useState, type FormEvent } from "react";

import {
  concluirLogin,
  concluirRegistro,
  estadoDaPorta,
  passkeyDisponivel,
  prepararLogin,
  prepararRegistro,
  type EstadoDaPorta,
  type Preparado,
} from "./cerimonia";

/**
 * A tela de entrar.
 *
 * # A regra que manda no formato desta tela
 *
 * **A chamada do WebAuthn tem que sair junto com o toque.** O Safari cancela
 * `credentials.create/get` que não venha de uma ativação do usuário, e uma ida
 * ao servidor no meio gasta essa ativação. O erro é `NotAllowedError` — que é a
 * mesma coisa que o iPhone diz quando você simplesmente cancela o Face ID.
 *
 * Por isso o desafio é buscado ANTES: no `useEffect`, para entrar; num passo
 * separado, para registrar. Quando o dedo toca no botão, não há mais nada a
 * esperar — só o Face ID.
 *
 * # Dois modos, e quem escolhe é o servidor
 *
 * Sem aparelho registrado, o que faz sentido é registrar — um "entrar" ali
 * falharia com "nenhum aparelho registrado ainda", que é verdadeiro e inútil.
 * Com aparelho registrado, o contrário: pedir o convite todo dia transformaria
 * um segredo de uso único em senha.
 */
export function Porta({ aoEntrar }: { aoEntrar: () => void }) {
  const [estado, setEstado] = useState<EstadoDaPorta | null>(null);
  const [convite, setConvite] = useState("");
  const [apelido, setApelido] = useState("iPhone");
  const [preparado, setPreparado] = useState<Preparado | null>(null);
  const [recado, setRecado] = useState("");
  const [erro, setErro] = useState(false);
  const [ocupado, setOcupado] = useState(false);

  function contar(mensagem: string, falhou = false) {
    setRecado(mensagem);
    setErro(falhou);
  }

  /** Busca um desafio de login e o deixa pronto para o próximo toque. */
  const armarLogin = useCallback(async () => {
    try {
      setPreparado(await prepararLogin());
    } catch (causa) {
      contar(causa instanceof Error ? causa.message : String(causa), true);
    }
  }, []);

  useEffect(() => {
    void estadoDaPorta()
      .then(async (proximo) => {
        setEstado(proximo);
        // Com aparelho registrado, o desafio já vem agora — para o botão de
        // entrar não ter nada a esperar quando o dedo chegar nele.
        if (proximo.registrado && proximo.passkey) await armarLogin();
      })
      .catch(() => setEstado(null));
  }, [armarLogin]);

  /** O passo de rede do registro: valida o convite e traz o desafio. */
  async function conferirConvite(evento: FormEvent) {
    evento.preventDefault();
    if (!convite.trim()) return contar("O convite é obrigatório.", true);
    setOcupado(true);
    try {
      setPreparado(await prepararRegistro(convite.trim(), apelido.trim() || "Aparelho"));
      // Sem recado: a tela inteira ja trocou para dizer que o convite passou, e
      // repetir isso embaixo do botao e ruido em cima de ruido.
      contar("");
    } catch (causa) {
      contar(causa instanceof Error ? causa.message : String(causa), true);
    }
    setOcupado(false);
  }

  /**
   * O toque. Nada de `await` antes da chamada do WebAuthn lá dentro — é isso
   * que mantém a ativação do usuário viva.
   */
  function registrar() {
    if (!preparado) return;
    setOcupado(true);
    concluirRegistro(preparado, apelido.trim() || "Aparelho")
      .then(aoEntrar)
      .catch(async (causa) => {
        contar(traduzir(causa), true);
        // O desafio foi gasto na tentativa. Sem um novo, o segundo toque
        // falharia com "Desafio expirado" — e o culpado pareceria ser o iPhone.
        setPreparado(null);
        setOcupado(false);
      });
  }

  function entrar() {
    if (!preparado) return;
    setOcupado(true);
    concluirLogin(preparado)
      .then(aoEntrar)
      .catch(async (causa) => {
        contar(traduzir(causa), true);
        setPreparado(null);
        await armarLogin();
        setOcupado(false);
      });
  }

  if (!estado) {
    return (
      <div className="porta">
        <p className="explicacao">Conferindo a porta…</p>
      </div>
    );
  }

  if (!estado.porta || !estado.passkey) {
    return (
      <div className="porta">
        <h1 className="marca">M/OS</h1>
        <p className="explicacao">
          Este servidor não tem a porta configurada, então não há como entrar por
          aqui. Quem autentica é o proxy à frente dele.
        </p>
      </div>
    );
  }

  if (!passkeyDisponivel()) {
    return (
      <div className="porta">
        <h1 className="marca">M/OS</h1>
        <p className="explicacao">
          Este navegador não sabe usar passkey. No iPhone, adicione o app à Tela
          de Início e abra por ele.
        </p>
      </div>
    );
  }

  return (
    <div className="porta">
      <h1 className="marca">M/OS</h1>

      {estado.registrado ? (
        <>
          <p className="explicacao">Entre com o Face ID deste aparelho.</p>
          <button
            className="botao"
            type="button"
            disabled={ocupado || !preparado}
            onClick={entrar}
          >
            Entrar
          </button>
        </>
      ) : preparado ? (
        <>
          <p className="explicacao">
            Convite aceito. Toque abaixo e confirme com o Face ID — é ele que
            passa a ser a sua entrada, e o convite não será pedido de novo neste
            aparelho.
          </p>
          <button className="botao" type="button" disabled={ocupado} onClick={registrar}>
            Confirmar com Face ID
          </button>
        </>
      ) : (
        <form className="registro" onSubmit={conferirConvite}>
          <p className="explicacao">
            Nenhum aparelho registrado ainda. O convite é o que decide quem pode
            se tornar o dono — ele está no <code>/etc/mos-web.env</code> da VPS,
            na linha <code>MOS_WEB_INVITE=</code>.
          </p>
          <label className="campo">
            <span>Convite</span>
            <input
              value={convite}
              onChange={(evento) => setConvite(evento.currentTarget.value)}
              autoCapitalize="off"
              autoCorrect="off"
              autoComplete="off"
              spellCheck={false}
            />
          </label>
          <label className="campo">
            <span>Como chamar este aparelho</span>
            <input
              value={apelido}
              onChange={(evento) => setApelido(evento.currentTarget.value)}
            />
          </label>
          <button className="botao" type="submit" disabled={ocupado}>
            Conferir convite
          </button>
          {/* Colar existe porque o convite tem 32 caracteres aleatórios, e
              digitá-los num teclado de celular erra. O botão só aparece onde o
              navegador deixa ler a área de transferência. */}
          {typeof navigator.clipboard?.readText === "function" ? (
            <button
              className="botao"
              data-variante="quieto"
              type="button"
              onClick={() => {
                void navigator.clipboard
                  .readText()
                  .then((texto) => setConvite(texto.trim()))
                  .catch(() => contar("O aparelho não deixou ler a área de transferência.", true));
              }}
            >
              Colar convite
            </button>
          ) : null}
        </form>
      )}

      <p className="recado" data-estado={erro ? "erro" : "ok"} aria-live="polite">
        {recado}
      </p>
    </div>
  );
}

/**
 * O que o navegador diz, e o que a pessoa precisa ouvir.
 *
 * `NotAllowedError` é o mesmo erro para "você cancelou", "expirou" e "o toque
 * não valeu" — e mostrá-lo cru manda a pessoa procurar defeito onde não há.
 */
function traduzir(causa: unknown): string {
  if (causa instanceof DOMException) {
    if (causa.name === "NotAllowedError") {
      return "O Face ID não completou. Toque de novo e confirme na hora — sem trocar de app no meio.";
    }
    if (causa.name === "InvalidStateError") {
      return "Este aparelho já está registrado. Recarregue e toque em Entrar.";
    }
    if (causa.name === "SecurityError") {
      return "O endereço não confere com o que a passkey espera. Abra pelo mesmo domínio em que registrou.";
    }
  }
  return causa instanceof Error ? causa.message : String(causa);
}
