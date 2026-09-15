/**
 * O painel operacional da Home: o que faço agora, o que tenho hoje, o próximo
 * compromisso, o que precisa de atenção.
 *
 * Só desenha. Tudo que decide — a recomendação, a severidade, a proposta do
 * dia — veio pronto do `piloto_panorama`, e as frases vêm do `piloto.ts`. O
 * componente não recalcula um número sequer: se a Home mostrar algo errado, o
 * erro está no motor, onde há teste.
 *
 * # Por que fixo, e não widget
 *
 * A mesma exceção da `FaixaSync`, pelo mesmo motivo estendido: este bloco é a
 * resposta às seis perguntas do §71 — o que faço, o que tenho, o próximo, o
 * urgente, o atrasado, o sincronizado. Um widget se esconde, e uma Home que
 * esconde a resposta a "o que faço agora?" deixou de ser o painel do sistema.
 * O que continua arrumável é tudo que vem abaixo dele.
 */
import { useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { ATENCAO_NA_HOME, fraseDeAusencia, fraseDeHoje, linhaDoItem, linhasDeAusencia, minutosCurto, rotuloDaAcao, seloDeSeveridade } from "./piloto";
import type { AcaoRecomendada, Candidata, Panorama } from "./types";

export type AcoesDoPiloto = {
  abrirTask: (id: string) => void;
  abrirInbox: () => void;
  abrirSync: () => void;
  abrirAcademico: () => void;
  abrirLembrete: (id: string) => void;
  montarDia: () => void;
  iniciarDiaManual: () => void;
  encerrarDia: () => void;
  abrirResgate: () => void;
  /** Depois de qualquer escrita: relê o panorama. */
  atualizar: () => void;
};

export function HomePiloto({ panorama, acoes, resgateDispensado, dispensarResgate, diaDispensado, dispensarDia }: {
  panorama: Panorama | null;
  acoes: AcoesDoPiloto;
  resgateDispensado: boolean;
  dispensarResgate: () => void;
  diaDispensado: boolean;
  dispensarDia: () => void;
}) {
  const [ocupado, setOcupado] = useState<string | null>(null);
  const [porque, setPorque] = useState(false);
  if (!panorama) return null;

  async function correr(chave: string, acao: () => Promise<unknown>) {
    if (ocupado) return;
    setOcupado(chave);
    try { await acao(); } catch { /* o panorama conta */ }
    setOcupado(null);
    acoes.atualizar();
  }

  function executar(acao: AcaoRecomendada) {
    switch (acao.acao) {
      case "comecar_task": return void correr(acao.id, () => api.taskStart(acao.id));
      case "abrir_task": return acoes.abrirTask(acao.id);
      case "reagendar_task": return void correr(acao.id, () => api.taskPlan(acao.id, acao.para, true));
      case "cobrar": return acoes.abrirTask(acao.id);
      case "processar_inbox": return acoes.abrirInbox();
      case "abrir_sync": return acoes.abrirSync();
      case "abrir_academico": return acoes.abrirAcademico();
      case "encerrar_dia": return acoes.encerrarDia();
      case "iniciar_dia": return acoes.montarDia();
      case "abrir_lembrete": return acoes.abrirLembrete(acao.id);
      case "nenhuma": return;
    }
  }

  const { agora, hoje, proximos, atencao, estadoDoDia, resgate } = panorama;
  const diaNaoIniciado = estadoDoDia.kind === "not_started" || estadoDoDia.kind === "stale_open";
  const feitas = fraseDeHoje(hoje);
  const mostrarResgate = resgate && !resgateDispensado;
  const mostrarDia = diaNaoIniciado && !diaDispensado && !mostrarResgate;
  const visiveis = atencao.filter((item) => item.tipo !== "day_not_started");
  const lista = visiveis.slice(0, ATENCAO_NA_HOME);
  const restantes = Math.max(0, visiveis.length - ATENCAO_NA_HOME);

  return <section className="piloto" aria-label="Painel do dia">
    <header className="piloto-cabecalho">
      <p className="piloto-saudacao">{panorama.saudacao}</p>
    </header>

    {mostrarResgate ? <div className="piloto-cartao piloto-resgate" role="region" aria-label="Rescue Mode">
      <span className="micro-label">VOCÊ ESTEVE FORA</span>
      <p className="piloto-titulo">{fraseDeAusencia(resgate.dias)}</p>
      <ul className="piloto-linhas">{linhasDeAusencia(resgate).map((linha) => <li key={linha}>{linha}</li>)}</ul>
      <div className="piloto-acoes">
        <Button variant="primary" onClick={acoes.abrirResgate}>Organizar para mim</Button>
        <Button variant="ghost" onClick={dispensarResgate}>Eu mesmo organizo</Button>
      </div>
    </div> : null}

    {mostrarDia ? <div className="piloto-cartao piloto-dia" role="region" aria-label="Dia não iniciado">
      <span className="micro-label">{estadoDoDia.kind === "stale_open" ? `O DIA ${estadoDoDia.day} FICOU ABERTO` : "SEU DIA AINDA NÃO FOI INICIADO"}</span>
      <p className="piloto-titulo">Posso montar tudo automaticamente.</p>
      {panorama.proposta ? <ul className="piloto-linhas">
        {panorama.proposta.principal ? <li>□ {panorama.proposta.principal.draft.title}{panorama.proposta.principal.estimativa ? <span className="piloto-mudo"> {panorama.proposta.principal.estimativa}</span> : null}</li> : null}
        {panorama.proposta.secundarios.map((s) => <li key={s.draft.title}>□ {s.draft.title}{s.estimativa ? <span className="piloto-mudo"> {s.estimativa}</span> : null}</li>)}
        {panorama.proposta.agenda.slice(0, 3).map((l) => <li key={l.at} className="piloto-mudo">{l.hora} — {l.titulo}</li>)}
        {!panorama.proposta.principal && panorama.proposta.agenda.length === 0 ? <li className="piloto-mudo">Nada planejado ainda — o dia começa vazio e você acrescenta o que quiser.</li> : null}
      </ul> : null}
      <div className="piloto-acoes">
        <Button variant="primary" disabled={ocupado === "dia"} onClick={() => void correr("dia", () => api.pilotoIniciarDia())}>{ocupado === "dia" ? "Montando..." : "Montar meu dia"}</Button>
        <Button variant="secondary" onClick={acoes.iniciarDiaManual}>Escolher eu mesmo</Button>
        <Button variant="ghost" onClick={dispensarDia}>Agora não</Button>
      </div>
    </div> : null}

    <div className="piloto-grade">
      <div className="piloto-cartao piloto-agora" role="region" aria-label="Agora">
        <span className="micro-label">AGORA</span>
        {agora.agora ? <AgoraCard candidata={agora.agora} ocupado={ocupado} porque={porque} setPorque={setPorque} abrir={acoes.abrirTask} comecar={(id) => void correr(id, () => api.taskStart(id))} concluir={(id) => void correr(id, () => api.setTaskState(id, "done"))} parar={(id) => void correr(id, () => api.taskStop(id))} /> : <p className="piloto-vazio">{agora.vazio ?? "Nada precisa da sua atenção agora."}</p>}
        {agora.seguintes.length ? <details className="piloto-seguintes">
          <summary className="micro-label">DEPOIS DISSO</summary>
          <ul>{agora.seguintes.map((c) => <li key={c.taskId}><button type="button" className="piloto-link" onClick={() => acoes.abrirTask(c.taskId)}>{c.titulo}</button>{c.estimativa ? <span className="piloto-mudo"> {c.estimativa}</span> : null}</li>)}</ul>
        </details> : null}
      </div>

      <div className="piloto-coluna">
        <div className="piloto-cartao piloto-hoje" role="region" aria-label="Hoje">
          <span className="micro-label">HOJE</span>
          {estadoDoDia.kind === "ended" ? <p className="piloto-vazio">Dia encerrado · {estadoDoDia.feitos} de {estadoDoDia.total} {estadoDoDia.total === 1 ? "concluído" : "concluídos"}.</p> : hoje.concluidas + hoje.restantes === 0 ? <p className="piloto-vazio">{diaNaoIniciado ? "Nada planejado para hoje." : "Nenhum objetivo no dia."}</p> : <>
            <p className="piloto-placar"><strong>✓ {feitas.feitas}</strong><span>□ {feitas.restantes}</span></p>
            <div className="piloto-barra" role="progressbar" aria-valuenow={hoje.progresso} aria-valuemin={0} aria-valuemax={100}><span style={{ width: `${hoje.progresso}%` }} /></div>
          </>}
          {estadoDoDia.kind === "active" ? <div className="piloto-acoes"><Button variant="ghost" size="sm" onClick={acoes.encerrarDia}>Encerrar dia</Button></div> : null}
        </div>

        <div className="piloto-cartao piloto-proximo" role="region" aria-label="Próximo">
          <span className="micro-label">PRÓXIMO</span>
          {proximos.length ? <ul className="piloto-agenda">{proximos.map((l) => <li key={l.at}><span className="piloto-hora">{l.hora}</span><span>{l.titulo}</span></li>)}</ul> : <p className="piloto-vazio">Nada marcado para o resto de hoje.</p>}
        </div>
      </div>
    </div>

    {lista.length ? <div className="piloto-cartao piloto-atencao" role="region" aria-label="Precisa de atenção">
      <span className="micro-label">PRECISA DE ATENÇÃO</span>
      <ul className="piloto-itens">
        {lista.map((item) => {
          const rotulo = rotuloDaAcao(item.acao);
          const selo = seloDeSeveridade(item.severidade);
          return <li key={`${item.tipo}:${item.alvo.kind}:${item.alvo.id}:${item.titulo}`} data-severidade={item.severidade}>
            <span className="piloto-marca" aria-hidden="true" />
            <div className="piloto-item-texto">
              <span className="piloto-item-titulo">{item.titulo}{selo ? <span className="piloto-selo">{selo}</span> : null}</span>
              <span className="piloto-mudo">{linhaDoItem(item)}{item.razoes.length > 1 || (item.descricao && item.razoes[0]) ? ` · ${item.razoes.filter((r) => r !== item.descricao).join(" · ")}` : ""}</span>
            </div>
            {rotulo ? <Button variant="ghost" size="sm" disabled={ocupado !== null} onClick={() => executar(item.acao)}>{rotulo}</Button> : null}
          </li>;
        })}
      </ul>
      {restantes > 0 ? <p className="piloto-mudo">+ {restantes} {restantes === 1 ? "item" : "itens"}</p> : null}
    </div> : null}
  </section>;
}

function AgoraCard({ candidata, ocupado, porque, setPorque, abrir, comecar, concluir, parar }: {
  candidata: Candidata;
  ocupado: string | null;
  porque: boolean;
  setPorque: (v: boolean) => void;
  abrir: (id: string) => void;
  comecar: (id: string) => void;
  concluir: (id: string) => void;
  parar: (id: string) => void;
}) {
  const busy = ocupado === candidata.taskId;
  return <div className="piloto-agora-corpo" data-comecada={candidata.comecada || undefined}>
    <button type="button" className="piloto-agora-titulo" onClick={() => abrir(candidata.taskId)}>{candidata.titulo}</button>
    <p className="piloto-mudo">{[candidata.projeto, candidata.estimativa, candidata.prioridade === "urgent" ? "urgente" : candidata.prioridade === "high" ? "prioridade alta" : ""].filter(Boolean).join(" · ")}</p>
    <div className="piloto-acoes">
      {candidata.comecada ? <>
        <Button variant="primary" disabled={busy} onClick={() => concluir(candidata.taskId)}>✓ Concluir</Button>
        <Button variant="ghost" disabled={busy} onClick={() => parar(candidata.taskId)}>Continuar depois</Button>
      </> : <Button variant="primary" disabled={busy} onClick={() => comecar(candidata.taskId)}>{busy ? "..." : "Começar"}</Button>}
      <button type="button" className="piloto-link piloto-porque" aria-expanded={porque} onClick={() => setPorque(!porque)}>Por que agora?</button>
    </div>
    {porque ? <ul className="piloto-razoes">{candidata.razoes.map((r) => <li key={r}>• {r}</li>)}</ul> : null}
  </div>;
}

/** O indicador global da Task ativa. Vive no cabeçalho, em toda página. */
export function TaskAtivaChip({ ativa, abrir, atualizar }: { ativa: Panorama["taskAtiva"]; abrir: (id: string) => void; atualizar: () => void }) {
  const [ocupado, setOcupado] = useState(false);
  if (!ativa) return null;
  async function acao(fn: () => Promise<unknown>) {
    if (ocupado) return;
    setOcupado(true);
    try { await fn(); } catch { /* o panorama conta */ }
    setOcupado(false);
    atualizar();
  }
  return <span className="task-ativa" title="Task ativa">
    <span className="task-ativa-ponto" aria-hidden="true">●</span>
    <button type="button" className="piloto-link" onClick={() => abrir(ativa.taskId)}>{ativa.titulo}</button>
    <span className="page-meta">{minutosCurto(ativa.minutos)}</span>
    <button type="button" className="task-ativa-botao" disabled={ocupado} title="Concluir" onClick={() => void acao(() => api.setTaskState(ativa.taskId, "done"))}>✓</button>
    <button type="button" className="task-ativa-botao" disabled={ocupado} title="Pausar" onClick={() => void acao(() => api.taskStop(ativa.taskId))}>‖</button>
  </span>;
}
