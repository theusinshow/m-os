/**
 * A regra da atualização obrigatória (ADR-076).
 *
 * Todo push que mexe no desktop vira uma versão publicada, e o M/OS não deixa
 * trabalhar numa versão velha: ao abrir, ele confere e instala antes de mostrar
 * qualquer tela; aberto, confere de tempos em tempos e reinicia sozinho.
 *
 * Três exceções, e cada uma tem motivo:
 *
 * - **Sem resposta, abre.** O M/OS guarda tudo no próprio computador; rede caída
 *   ou GitHub fora do ar não pode virar um app que não abre.
 * - **Duas falhas na mesma versão, abre.** Um instalador quebrado não pode
 *   trancar o dono do lado de fora. A terceira abertura volta a tentar só se a
 *   versão publicada mudar.
 * - **Gravando, espera.** Reunião e nota de voz guardam áudio em memória até
 *   parar; reiniciar no meio perde o que foi dito.
 */

/** Quanto a abertura espera pela resposta antes de desistir e abrir. */
export const PRAZO_DA_ABERTURA_MS = 4_000;
/** De quanto em quanto tempo o app aberto confere. */
export const INTERVALO_COM_APP_ABERTO_MS = 30 * 60_000;
/** O aviso antes do reinício com o app aberto. */
export const AVISO_ANTES_DE_REINICIAR_S = 60;
export const FALHAS_ANTES_DE_LIBERAR = 2;

const CHAVE_FALHAS = "mos.atualizacao.falhas";

type Falhas = { versao: string; vezes: number };

function lerFalhas(armazenamento: Pick<Storage, "getItem"> | null): Falhas | null {
  try {
    const bruto = armazenamento?.getItem(CHAVE_FALHAS);
    if (!bruto) return null;
    const lido = JSON.parse(bruto) as Partial<Falhas>;
    return typeof lido.versao === "string" && typeof lido.vezes === "number"
      ? { versao: lido.versao, vezes: lido.vezes }
      : null;
  } catch {
    return null;
  }
}

/** Já falhou o bastante nesta versão para não trancar mais o app por ela. */
export function liberadaPorFalhas(
  versao: string,
  armazenamento: Pick<Storage, "getItem"> | null,
): boolean {
  const falhas = lerFalhas(armazenamento);
  return falhas?.versao === versao && falhas.vezes >= FALHAS_ANTES_DE_LIBERAR;
}

/** Conta mais uma falha para a versão; uma versão nova zera a conta. */
export function registrarFalha(versao: string, armazenamento: Storage | null): number {
  const anterior = lerFalhas(armazenamento);
  const vezes = anterior?.versao === versao ? anterior.vezes + 1 : 1;
  try {
    armazenamento?.setItem(CHAVE_FALHAS, JSON.stringify({ versao, vezes }));
  } catch {
    // Sem onde guardar, cada abertura tenta de novo — que é o lado seguro.
  }
  return vezes;
}

export function esquecerFalhas(armazenamento: Storage | null) {
  try {
    armazenamento?.removeItem(CHAVE_FALHAS);
  } catch {
    // idem
  }
}

/** Reiniciar agora não perde nada que só exista em memória. */
export function podeReiniciar(gravacoes: { reuniao: unknown; voz: unknown }): boolean {
  return gravacoes.reuniao == null && gravacoes.voz == null;
}

/**
 * A promessa, ou `null` se ela não responder a tempo. Quem chama trata `null`
 * como "sem resposta" — e sem resposta o app abre.
 */
export async function comPrazo<T>(promessa: Promise<T>, ms: number): Promise<T | null> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const prazo = new Promise<null>((resolve) => {
    timer = setTimeout(() => resolve(null), ms);
  });
  try {
    return await Promise.race([promessa, prazo]);
  } finally {
    clearTimeout(timer);
  }
}

/** "Baixando 42%", ou só "Baixando" quando o servidor não diz o tamanho. */
export function rotuloDoProgresso(progresso: { downloaded: number; total: number | null }): string {
  if (!progresso.total) return "Baixando a versão nova…";
  const parte = Math.min(100, Math.round((progresso.downloaded / progresso.total) * 100));
  return `Baixando a versão nova: ${parte}%`;
}
