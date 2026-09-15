/**
 * O piloto do lado da tela: só o que dá para verificar.
 *
 * Nenhuma regra de DOMÍNIO mora aqui — "o que é urgente", "o que fazer agora"
 * e "quanto esperar" vivem em `mos_core::piloto`, com teste. O que este arquivo
 * carrega é regra de APRESENTAÇÃO: como um estado vira frase curta, que cor
 * um selo tem, que rótulo o botão mostra. Os `.tsx` só desenham o resultado.
 */
import type {
  AcaoRecomendada,
  EstadoDeSaude,
  ItemDeAtencao,
  Panorama,
  SaudeDoSync,
  SeveridadeDeAtencao,
  SyncNoRetrato,
} from "./types";
import { relativeTime } from "./relativeTime";

/** O selo discreto do cabeçalho. `tom` é o que o CSS lê. */
export type SeloDeSync = {
  icone: "✓" | "↻" | "⚠" | "✕" | "○";
  texto: string;
  tom: "calmo" | "girando" | "aviso" | "erro" | "mudo";
  /** Se vale um clique — desligado abre o Settings, o resto o Sync Health. */
  detalhe: string;
};

/**
 * O selo, a partir do estado. A ORDEM das perguntas é a do `mos-sync`:
 * desligado sai mudo; girando ganha de tudo; erro ganha de offline; offline
 * ganha de pendente; pendente ganha de em dia.
 */
export function seloDeSync(estado: EstadoDeSaude | null, ultimoOkEm: string | null): SeloDeSync {
  if (!estado || estado.kind === "desligado") {
    return { icone: "○", texto: "Sync desligado", tom: "mudo", detalhe: "Configure o hub em Ajustes para sincronizar entre aparelhos." };
  }
  switch (estado.kind) {
    case "sincronizando":
      return {
        icone: "↻",
        texto: estado.pendentes > 0 ? `Sincronizando ${estado.pendentes} ${estado.pendentes === 1 ? "alteração" : "alterações"}...` : "Sincronizando...",
        tom: "girando",
        detalhe: "Uma rodada está em curso.",
      };
    case "erro":
      return { icone: "✕", texto: "Erro de sincronização", tom: "erro", detalhe: estado.mensagem };
    case "offline": {
      const desde = ultimoOkEm ? ` há ${relativeTime(ultimoOkEm).replace(/^há /, "")}` : "";
      return {
        icone: "⚠",
        texto: `Não sincronizado${desde}`,
        tom: "aviso",
        detalhe: "Sem alcançar o hub. Suas alterações estão salvas aqui e sobem sozinhas quando a conexão voltar.",
      };
    }
    case "pendente":
      return {
        icone: "↻",
        texto: `${estado.pendentes} ${estado.pendentes === 1 ? "alteração esperando" : "alterações esperando"}`,
        tom: "calmo",
        detalhe: "Sobem na próxima rodada, em instantes.",
      };
    case "em_dia":
      return {
        icone: "✓",
        texto: ultimoOkEm ? `Sincronizado ${relativeTime(ultimoOkEm)}` : "Sincronizado",
        tom: "calmo",
        detalhe: "Tudo subiu e desceu.",
      };
  }
}

/** A frase de erro que informa, e não a que assusta. */
export function fraseDeErroDeSync(saude: SaudeDoSync): string {
  const r = saude.registro;
  switch (r.tipoDoErro) {
    case "credencial":
      return "O hub recusou o segredo. Suas alterações estão salvas neste dispositivo; corrija o segredo em Ajustes para elas subirem.";
    case "contrato":
      return "Este M/OS e o hub falam versões diferentes. Suas alterações estão salvas aqui; atualize o aplicativo mais antigo.";
    case "offline":
    case "timeout":
    case "hub":
      return "Sem conexão com o hub. Suas alterações estão salvas neste dispositivo e serão sincronizadas automaticamente.";
    case "local":
      return "O banco local recusou a operação. Nada foi perdido; vai tentar de novo sozinho.";
    case "desconhecida":
      return r.ultimoErro ?? "A última rodada parou. Vai tentar de novo sozinho.";
    default:
      return "";
  }
}

/** "em 30 s", "em 2 min", ou vazio quando já passou. */
export function proximaTentativa(registro: { proximaTentativaEm: string | null }, agora = Date.now()): string {
  if (!registro.proximaTentativaEm) return "";
  const ms = new Date(registro.proximaTentativaEm).getTime() - agora;
  if (!Number.isFinite(ms) || ms <= 0) return "agora";
  const s = Math.round(ms / 1000);
  if (s < 90) return `em ${s} s`;
  const m = Math.round(s / 60);
  if (m < 60) return `em ${m} min`;
  return `em ${Math.round(m / 60)} h`;
}

/** A frase do "Hoje" da Home. */
export function fraseDeHoje(hoje: Panorama["hoje"]): { feitas: string; restantes: string } {
  return {
    feitas: `${hoje.concluidas} ${hoje.concluidas === 1 ? "concluída" : "concluídas"}`,
    restantes: `${hoje.restantes} ${hoje.restantes === 1 ? "restante" : "restantes"}`,
  };
}

/** O selo de severidade, em palavra. Cor sozinha não diz nada. */
export function seloDeSeveridade(severidade: SeveridadeDeAtencao): string {
  switch (severidade) {
    case "urgente": return "URGENTE";
    case "alta": return "ALTA";
    case "media": return "ATENÇÃO";
    case "baixa": return "";
  }
}

/** O rótulo do botão de cada ação recomendada. Vazio é "sem botão". */
export function rotuloDaAcao(acao: AcaoRecomendada): string {
  switch (acao.acao) {
    case "comecar_task": return "Começar";
    case "abrir_task": return "Abrir";
    case "reagendar_task": return "Planejar para hoje";
    case "cobrar": return `Cobrar ${acao.quem}`;
    case "processar_inbox": return "Organizar";
    case "abrir_sync": return "Ver sync";
    case "abrir_academico": return "Abrir";
    case "encerrar_dia": return "Encerrar dia";
    case "iniciar_dia": return "Montar meu dia";
    case "abrir_lembrete": return "Abrir";
    case "nenhuma": return "";
  }
}

/** A linha secundária de um item de atenção: descrição, ou a razão. */
export function linhaDoItem(item: ItemDeAtencao): string {
  if (item.descricao) return item.descricao;
  return item.razoes[0] ?? "";
}

/** O sync como frase curta para o cartão de erro da Home. */
export function fraseDoSyncNoRetrato(sync: SyncNoRetrato): string {
  switch (sync.kind) {
    case "erro": return "Erro de sincronização";
    case "offline": return "Sem conexão — salvo neste dispositivo";
    case "pendente": return `${sync.pendentes} esperando`;
    default: return "";
  }
}

/**
 * Quantos itens de atenção a Home mostra antes do "ver todos". Cinco: os
 * urgentes cabem, e a lista não vira o dashboard corporativo que o §47 recusa.
 */
export const ATENCAO_NA_HOME = 5;

/** "18 min", "1h05". */
export function minutosCurto(minutos: number): string {
  if (minutos < 60) return `${minutos} min`;
  const h = Math.floor(minutos / 60);
  const m = minutos % 60;
  return m === 0 ? `${h}h` : `${h}h${String(m).padStart(2, "0")}`;
}

/** A frase do cartão de resgate. */
export function fraseDeAusencia(dias: number): string {
  return `Você não revisa seu M/OS há ${dias} dias.`;
}

/** As linhas de contagem do resgate, só as que não são zero. */
export function linhasDeAusencia(a: { tarefasAtrasadas: number; captures: number; waitingFor: number; deadlinesProximos: number; lembretesVencidos: number }): string[] {
  const linhas: string[] = [];
  const plural = (n: number, um: string, muitos: string) => `${n} ${n === 1 ? um : muitos}`;
  if (a.tarefasAtrasadas) linhas.push(plural(a.tarefasAtrasadas, "tarefa atrasada", "tarefas atrasadas"));
  if (a.captures) linhas.push(plural(a.captures, "Capture", "Captures"));
  if (a.waitingFor) linhas.push(plural(a.waitingFor, "Waiting For", "Waiting For"));
  if (a.deadlinesProximos) linhas.push(plural(a.deadlinesProximos, "deadline próximo", "deadlines próximos"));
  if (a.lembretesVencidos) linhas.push(plural(a.lembretesVencidos, "lembrete vencido", "lembretes vencidos"));
  return linhas;
}
