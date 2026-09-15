import { createRoot } from "react-dom/client";
import { useState } from "react";

import "@fontsource/schibsted-grotesk/400.css";
import "@fontsource/schibsted-grotesk/500.css";
import "@fontsource/jetbrains-mono/500.css";
import "../../../packages/design-system/tokens.css";
import "./App.css";

import { AutopilotToast } from "./AutopilotToast";
import { HomePiloto, TaskAtivaChip, type AcoesDoPiloto } from "./HomePiloto";
import { SyncChip, SyncHealth } from "./SyncHealth";
import type { EstadoDeSaude, ItemDeAtencao, Panorama, SaudeDoSync } from "./types";

/**
 * A bancada do piloto: os estados da Home que exigiriam dias de dado real —
 * ausência de quatro dias, sync offline há horas, sete itens de atenção — todos
 * numa página, nos dois temas (`?tema=light`).
 *
 * Como a `bancada.tsx`, ela responde por FORMA. As escritas não vão a lugar
 * nenhum: os botões chamam `invoke` num navegador sem Tauri e falham em
 * silêncio, o que é exatamente o que se quer numa vitrine.
 */

const AGORA = new Date();
const daqui = (min: number) => new Date(AGORA.getTime() + min * 60_000).toISOString();

const acoes: AcoesDoPiloto = {
  abrirTask: () => undefined, abrirInbox: () => undefined, abrirSync: () => undefined, abrirAcademico: () => undefined,
  abrirLembrete: () => undefined, montarDia: () => undefined, iniciarDiaManual: () => undefined, encerrarDia: () => undefined,
  abrirResgate: () => undefined, atualizar: () => undefined,
};

function item(mudanca: Partial<ItemDeAtencao>): ItemDeAtencao {
  return { tipo: "overdue", severidade: "alta", titulo: "Item", descricao: "", alvo: { kind: "task", id: "t" }, razoes: ["vence hoje"], desde: null, acao: { acao: "abrir_task", id: "t" }, peso: 0, ...mudanca };
}

const ATENCAO: ItemDeAtencao[] = [
  item({ titulo: "Trabalho de Estruturas — memorial descritivo com título longo o bastante para quebrar", severidade: "urgente", razoes: ["vencida há 3 dias", "prioridade alta"], descricao: "167-25 · Caixa 01", acao: { acao: "comecar_task", id: "1" } }),
  item({ tipo: "academic_deadline", titulo: "Prova de Hidráulica", severidade: "urgente", descricao: "prova · Hidráulica II", razoes: ["vence hoje", "ainda pendente"], alvo: { kind: "academic_exam", id: "e" }, acao: { acao: "abrir_academico", tipo: "exam", id: "e" } }),
  item({ tipo: "stale_waiting_for", titulo: "Tipos de base", severidade: "alta", descricao: "Aguardando Victor", razoes: ["aguardando Victor há 4 dias", "o follow-up já passou"], acao: { acao: "cobrar", id: "2", quem: "Victor" } }),
  item({ tipo: "unsynced_changes", titulo: "Sem sincronizar há 5h", severidade: "media", descricao: "Suas alterações estão salvas aqui e sobem quando a conexão voltar.", razoes: ["3 alterações na fila"], alvo: { kind: "", id: "" }, acao: { acao: "abrir_sync" } }),
  item({ tipo: "unprocessed_capture", titulo: "12 captures para organizar", severidade: "media", descricao: "a mais antiga há 9 dias", razoes: ["12 na Inbox"], alvo: { kind: "", id: "" }, acao: { acao: "processar_inbox" } }),
  item({ tipo: "upcoming_deadline", titulo: "Enviar PDF", severidade: "media", razoes: ["vence amanhã"], acao: { acao: "comecar_task", id: "3" } }),
  item({ tipo: "stale_task", titulo: "Ajustar selo", severidade: "baixa", razoes: ["planejada para 2026-09-12 e não foi movida"], acao: { acao: "reagendar_task", id: "4", para: "2026-09-15" } }),
];

function panorama(mudanca: Partial<Panorama>): Panorama {
  return {
    day: "2026-09-15", saudacao: "Boa tarde.",
    estadoDoDia: { kind: "active", startedAt: AGORA.toISOString(), feitos: 3, total: 7 },
    hoje: { concluidas: 3, restantes: 4, progresso: 43 },
    agora: { agora: { taskId: "1", titulo: "Revisar armaduras da Caixa 01", projeto: "167-25 · Caixa 01", estimativa: "~25 min", estimateMinutes: 25, prioridade: "high", comecada: false, pontos: 120, razoes: ["vence hoje", "leva ~25 min e você tem 45 min livres", "objetivo principal do dia"] }, seguintes: [{ taskId: "2", titulo: "Enviar arquivos para Victor", projeto: "", estimativa: "", estimateMinutes: null, prioridade: "normal", comecada: false, pontos: 60, razoes: [] }], minutosLivres: 45, vazio: null },
    taskAtiva: null,
    proximos: [{ hora: "16:30", titulo: "Reunião — Projeto X", tipo: "meeting", at: daqui(45) }, { hora: "18:00", titulo: "Faculdade", tipo: "study", at: daqui(135) }],
    atencao: ATENCAO, resumoDeAtencao: { urgentes: 2, altos: 1, total: 7 },
    proposta: null, resgate: null, sync: { kind: "em_dia" }, vazio: null,
    ...mudanca,
  };
}

const CENARIOS: { titulo: string; p: Panorama }[] = [
  { titulo: "Dia ativo, Task começada, muita atenção", p: panorama({ agora: { ...panorama({}).agora, agora: { ...panorama({}).agora.agora!, comecada: true } }, taskAtiva: { taskId: "1", titulo: "Revisar armaduras da Caixa 01", startedAt: AGORA.toISOString(), minutos: 18 } }) },
  { titulo: "Dia não iniciado, proposta pronta", p: panorama({ estadoDoDia: { kind: "not_started" }, hoje: { concluidas: 0, restantes: 0, progresso: 0 }, proposta: { day: "2026-09-15", saudacao: "Bom dia.", contagens: { compromissos: 2, tarefasImportantes: 4, vencidas: 1, lembretes: 2, entregasAcademicas: 0, capturesNaInbox: 3 }, principal: { draft: { title: "Revisar Caixa 01" }, razoes: ["vence hoje"], estimativa: "~25 min", projeto: "167-25" }, secundarios: [{ draft: { title: "Enviar arquivos para Victor" }, razoes: [], estimativa: "", projeto: "" }, { draft: { title: "Finalizar trabalho da faculdade" }, razoes: [], estimativa: "~1h30", projeto: "" }], agenda: [{ hora: "14:00", titulo: "reunião", tipo: "meeting", at: daqui(60) }, { hora: "18:00", titulo: "compromisso", tipo: "meeting", at: daqui(300) }], input: { main: null, secondaries: [] }, nota: "" } }) },
  { titulo: "Ausência de 4 dias (Rescue Mode)", p: panorama({ estadoDoDia: { kind: "not_started" }, resgate: { dias: 4, desde: null, tarefasAtrasadas: 7, captures: 12, waitingFor: 3, deadlinesProximos: 2, lembretesVencidos: 0 } }) },
  { titulo: "Nada a fazer (vazio útil)", p: panorama({ agora: { agora: null, seguintes: [], minutosLivres: 45, vazio: "Nada precisa da sua atenção agora. Próximo compromisso às 16:30." }, atencao: [], resumoDeAtencao: { urgentes: 0, altos: 0, total: 0 }, vazio: "Nada precisa da sua atenção agora. Próximo compromisso às 16:30.", hoje: { concluidas: 5, restantes: 0, progresso: 100 } }) },
  { titulo: "Dia encerrado", p: panorama({ estadoDoDia: { kind: "ended", endedAt: AGORA.toISOString(), feitos: 5, total: 7 }, atencao: [] }) },
];

const ESTADOS: { estado: EstadoDeSaude; ok: string | null }[] = [
  { estado: { kind: "em_dia" }, ok: daqui(-1) },
  { estado: { kind: "pendente", pendentes: 2 }, ok: daqui(-30) },
  { estado: { kind: "sincronizando", pendentes: 3 }, ok: daqui(-30) },
  { estado: { kind: "offline", pendentes: 4, proximaTentativaEm: daqui(2) }, ok: daqui(-125) },
  { estado: { kind: "erro", pendentes: 4, tipo: "credencial", mensagem: "O hub respondeu 401." }, ok: daqui(-1500) },
  { estado: { kind: "desligado" }, ok: null },
];

function saude(estado: EstadoDeSaude, ok: string | null): SaudeDoSync {
  const erro = estado.kind === "erro" ? estado : null;
  return {
    estado, ligado: estado.kind !== "desligado", rodando: estado.kind === "sincronizando", pendentes: "pendentes" in estado ? estado.pendentes : 0, emRetry: erro ? 4 : 0, conflitosAbertos: erro ? 1 : 0,
    registro: { ultimoOkEm: ok, ultimaRodadaEm: daqui(-1), ultimoErro: erro ? erro.mensagem : estado.kind === "offline" ? "Sem alcancar o hub: connection refused" : null, tipoDoErro: erro ? "credencial" : estado.kind === "offline" ? "offline" : null, falhasSeguidas: erro ? 6 : estado.kind === "offline" ? 3 : 0, proximaTentativaEm: estado.kind === "offline" ? daqui(2) : null },
    dispositivos: [
      { id: "a", name: "DESKTOP-MATHEUS", platform: "windows", appVersion: "0.5.1", lastSyncAt: daqui(-1), isThisDevice: true },
      { id: "b", name: "M/OS de bolso", platform: "web", appVersion: "0.5.0", lastSyncAt: daqui(-400), isThisDevice: false },
    ],
    deviceId: "a", appVersion: "0.5.1",
  };
}

function Bancada() {
  const inicial = new URLSearchParams(location.search).get("tema") === "light" ? "light" : "dark";
  const [tema, setTema] = useState<"dark" | "light">(inicial);
  const [health, setHealth] = useState<number | null>(null);
  /* `?cenario=N` mostra um só: o shell trava a rolagem da página, e a foto
     headless precisa alcançar cada estado sem rolar. */
  const so = new URLSearchParams(location.search).get("cenario");
  const cenarios = so === null ? CENARIOS : CENARIOS.filter((_, i) => String(i) === so);
  return (
    <div data-theme={tema} style={{ minHeight: "100vh", background: "var(--canvas)", color: "var(--text)" }}>
      <div className="page" style={{ padding: "var(--space-4)", display: "flex", flexDirection: "column", gap: "var(--space-6)", maxWidth: 1100 }}>
        <header style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span className="micro-label">BANCADA DO PILOTO</span>
          <button className="button secondary sm" type="button" onClick={() => setTema(tema === "dark" ? "light" : "dark")}>{tema === "dark" ? "Tema claro" : "Tema escuro"}</button>
        </header>

        <section>
          <span className="micro-label">O SELO DO SYNC, NOS SEIS ESTADOS</span>
          <div style={{ display: "flex", flexWrap: "wrap", gap: "var(--space-3)", marginTop: "var(--space-2)", alignItems: "center" }}>
            {ESTADOS.map((e, i) => <span key={e.estado.kind} style={{ display: "inline-flex", gap: 8, alignItems: "center" }}><SyncChip saude={saude(e.estado, e.ok)} abrir={() => setHealth(i)} />{e.estado.kind === "desligado" ? <span className="page-meta">(desligado sai mudo)</span> : null}</span>)}
            <TaskAtivaChip ativa={{ taskId: "1", titulo: "Revisar armaduras da Caixa 01", startedAt: AGORA.toISOString(), minutos: 78 }} abrir={() => undefined} atualizar={() => undefined} />
          </div>
        </section>

        {cenarios.map((c) => <section key={c.titulo}>
          <span className="micro-label">{c.titulo.toUpperCase()}</span>
          <div className="home-page" style={{ marginTop: "var(--space-2)" }}>
            <HomePiloto panorama={c.p} acoes={acoes} resgateDispensado={false} dispensarResgate={() => undefined} diaDispensado={false} dispensarDia={() => undefined} />
          </div>
        </section>)}

        <AutopilotToast aviso={{ chave: "k", tipo: "forgotten_task", titulo: "Finalizar trabalho da faculdade", corpo: "Você planejou fazer isso hoje e ainda não começou. Estimativa: 1h30.", alvo: { kind: "task", id: "1" }, adiar: [{ rotulo: "10 min", minutos: 10 }, { rotulo: "30 min", minutos: 30 }, { rotulo: "1 hora", minutos: 60 }, { rotulo: "Hoje à noite", minutos: -1 }, { rotulo: "Amanhã", minutos: -2 }, { rotulo: "Escolher horário", minutos: null }], acaoPrincipal: "Fazer agora" }} fechar={() => undefined} agir={() => undefined} />
        {health !== null ? <SyncHealth inicial={saude(ESTADOS[health].estado, ESTADOS[health].ok)} close={() => setHealth(null)} abrirAjustes={() => setHealth(null)} /> : null}
      </div>
    </div>
  );
}

createRoot(document.getElementById("raiz")!).render(<Bancada />);
