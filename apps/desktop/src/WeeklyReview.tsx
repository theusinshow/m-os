import { useCallback, useEffect, useState } from "react";
import { api, appError } from "./api";
import { Button } from "./Button";
import { EmptyState, StateMessage } from "./Surface";
import { podeFechar, rotuloDaSemana, secoesDaSemana, semanaVizinha } from "./weekly";
import type { Week, WeekSummary } from "./types";

/**
 * O fecho da semana.
 *
 * **Nenhum placar.** Não existe `X de Y` aqui, e a ausência é a decisão:
 * `ATTENTION-SYSTEM.md` §19 proíbe resumo de produtividade em digest semanal.
 * A única contagem é "N dias com sessão", que é fato sobre o uso do sistema e
 * não sobre o trabalho.
 *
 * Abre na semana **pendente** quando há uma; senão, na corrente. Abrir sempre
 * na corrente faria a linha da Home levar a uma tela que não é a que ela
 * anunciou.
 */
export function WeeklyReviewPanel({ semanaInicial }: { semanaInicial: Week | null }) {
  const [semana, setSemana] = useState<Week | null>(semanaInicial);
  const [resumo, setResumo] = useState<WeekSummary | null>(null);
  const [texto, setTexto] = useState("");
  const [carregando, setCarregando] = useState(true);
  const [salvando, setSalvando] = useState(false);
  const [erro, setErro] = useState("");

  const carregar = useCallback(async (alvo: Week | null) => {
    setCarregando(true);
    try {
      const proximo = await api.weeklyWeek(alvo ?? undefined);
      setResumo(proximo);
      setSemana(proximo.week);
      // O texto salvo entra no campo: editar é a única mudança possível num
      // registro, e começar com o campo vazio pareceria que ele se perdeu.
      setTexto(proximo.review?.summary ?? "");
      setErro("");
    } catch (falha) {
      setErro(appError(falha).message);
    } finally {
      setCarregando(false);
    }
  }, []);

  useEffect(() => {
    void carregar(semanaInicial);
  }, [carregar, semanaInicial]);

  async function fechar() {
    if (!resumo || salvando) return;
    setSalvando(true);
    try {
      const fechado = await api.weeklyClose(resumo.week, texto);
      setResumo(fechado);
      setErro("");
    } catch (falha) {
      setErro(appError(falha).message);
    } finally {
      setSalvando(false);
    }
  }

  if (carregando && !resumo) return <StateMessage state="loading" label="Lendo a semana..." />;
  if (erro && !resumo) {
    return <StateMessage state="error" label="A semana não pôde ser lida." detail={erro} />;
  }
  if (!resumo) return null;

  const secoes = secoesDaSemana(resumo);
  const fechada = Boolean(resumo.review);

  return (
    <div className="daily-session-body" data-busy={carregando || undefined}>
      <div className="weekly-head">
        <strong>{rotuloDaSemana(resumo.week)}</strong>
        <div className="weekly-nav">
          <button
            type="button"
            aria-label="Semana anterior"
            onClick={() => void carregar(semanaVizinha(semana ?? resumo.week, -1))}
          >
            ‹
          </button>
          <button
            type="button"
            aria-label="Próxima semana"
            onClick={() => void carregar(semanaVizinha(semana ?? resumo.week, 1))}
          >
            ›
          </button>
        </div>
      </div>

      {resumo.empty ? (
        <EmptyState>Nenhum dia iniciado nesta semana. Não há o que revisar.</EmptyState>
      ) : (
        <>
          <p className="daily-widget-quiet">
            {resumo.daysWithSession}{" "}
            {resumo.daysWithSession === 1 ? "dia com sessão" : "dias com sessão"}
          </p>

          {secoes.map((secao) => (
            <section className="weekly-secao" key={secao.chave}>
              <span className="micro-label">{secao.titulo}</span>
              <ul>
                {secao.linhas.map((linha, indice) => (
                  <li key={`${secao.chave}-${indice}-${linha.texto}`}>
                    <span className="weekly-linha-texto">{linha.texto}</span>
                    {linha.detalhe ? <span className="micro-label">{linha.detalhe}</span> : null}
                  </li>
                ))}
              </ul>
            </section>
          ))}

          {!secoes.length ? (
            <EmptyState>A semana teve sessões, e nada nela pede uma frase.</EmptyState>
          ) : null}

          <section className="daily-reflection">
            <span className="micro-label">COMO FOI A SEMANA?</span>
            <textarea
              rows={3}
              value={texto}
              aria-label="Como foi a semana"
              placeholder="Opcional"
              onChange={(evento) => setTexto(evento.currentTarget.value)}
            />
          </section>

          {erro ? (
            <p className="inline-error" role="alert">
              ! {erro}
            </p>
          ) : null}

          {podeFechar(resumo) ? (
            <div className="form-actions">
              <Button variant="primary" disabled={salvando} onClick={() => void fechar()}>
                {salvando ? "Salvando" : fechada ? "Salvar" : "Encerrar a semana"}
              </Button>
            </div>
          ) : null}
        </>
      )}
    </div>
  );
}
