import { useEffect, useState, type FormEvent } from "react";

import {
  entrar,
  estadoDaPorta,
  passkeyDisponivel,
  registrar,
  type EstadoDaPorta,
} from "./cerimonia";

/**
 * A tela de entrar.
 *
 * # Ela tem dois modos, e quem escolhe é o servidor
 *
 * Sem nenhum aparelho registrado, o que faz sentido é **registrar** — e um botão
 * "entrar" ali falharia com "nenhum aparelho registrado ainda", que é uma frase
 * verdadeira e inútil. Com aparelho registrado, o contrário: pedir o convite de
 * novo, todo dia, seria transformar um segredo de uso único em senha.
 *
 * # Por que o convite é um campo de texto comum
 *
 * Não é senha: ele não se repete, não identifica ninguém e é usado uma vez por
 * aparelho. Escondê-lo atrás de bolinhas obrigaria a digitar 32 caracteres
 * aleatórios no escuro, num teclado de celular — e o erro de digitação viraria
 * "convite inválido", indistinguível do convite errado.
 */
export function Porta({ aoEntrar }: { aoEntrar: () => void }) {
  const [estado, setEstado] = useState<EstadoDaPorta | null>(null);
  const [convite, setConvite] = useState("");
  const [apelido, setApelido] = useState("iPhone");
  const [recado, setRecado] = useState("");
  const [erro, setErro] = useState(false);
  const [ocupado, setOcupado] = useState(false);

  useEffect(() => {
    void estadoDaPorta()
      .then(setEstado)
      .catch(() => setEstado(null));
  }, []);

  function contar(mensagem: string, falhou = false) {
    setRecado(mensagem);
    setErro(falhou);
  }

  async function tentar(acao: () => Promise<void>) {
    setOcupado(true);
    try {
      await acao();
      aoEntrar();
    } catch (causa) {
      contar(traduzir(causa), true);
    }
    setOcupado(false);
  }

  async function aoRegistrar(evento: FormEvent) {
    evento.preventDefault();
    if (!convite.trim()) return contar("O convite é obrigatório.", true);
    await tentar(async () => {
      await registrar(convite.trim(), apelido.trim() || "Aparelho");
      // Registrar não abre sessão — o servidor só guardou a credencial. Entrar
      // em seguida é o que produz o cookie, e fazer isso aqui poupa um segundo
      // Face ID que não explicaria a si mesmo.
      await entrar();
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
            disabled={ocupado}
            onClick={() => void tentar(entrar)}
          >
            Entrar
          </button>
          <p className="rotulo">
            OUTRO APARELHO? USE O CONVITE PARA REGISTRÁ-LO NELE.
          </p>
        </>
      ) : (
        <form className="registro" onSubmit={aoRegistrar}>
          <p className="explicacao">
            Nenhum aparelho registrado ainda. O convite é o que decide quem pode
            se tornar o dono — ele está no <code>/etc/mos-web.env</code> da VPS.
          </p>
          <label className="campo">
            <span>Convite</span>
            <input
              value={convite}
              onChange={(evento) => setConvite(evento.currentTarget.value)}
              autoCapitalize="off"
              autoCorrect="off"
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
            Registrar este aparelho
          </button>
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
 * `NotAllowedError` é o mesmo erro para "você cancelou", "expirou" e "este
 * aparelho recusou" — e mostrá-lo cru manda a pessoa procurar defeito onde
 * provavelmente não há.
 */
function traduzir(causa: unknown): string {
  if (causa instanceof DOMException) {
    if (causa.name === "NotAllowedError") {
      return "Cancelado, ou o tempo acabou. Tente de novo.";
    }
    if (causa.name === "InvalidStateError") {
      return "Este aparelho já está registrado. Toque em Entrar.";
    }
    if (causa.name === "SecurityError") {
      return "O endereço não confere com o que a passkey espera. Abra pelo mesmo domínio em que registrou.";
    }
  }
  return causa instanceof Error ? causa.message : String(causa);
}
