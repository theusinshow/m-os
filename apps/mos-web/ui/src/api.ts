/**
 * A conversa com o servidor.
 *
 * Um arquivo, e todas as chamadas nele — mesma disciplina do `api.ts` do
 * desktop. O resto da interface não sabe que existe rede.
 */

export type Capture = {
  id: string;
  content: string;
  capturedAt: string;
};

export type Task = {
  id: string;
  title: string;
  description: string;
  state: "inbox" | "backlog" | "planned" | "doing" | "review" | "done";
};

export type EstadoDoAparelho = {
  pendentes: number;
  sincroniza: boolean;
};

/**
 * O erro chega como `{ erro }` do servidor; o que aparece na tela é essa
 * frase, e não "Failed to fetch". A pessoa está na rua com uma ideia na
 * cabeça — ela precisa saber se deve tentar de novo ou se perdeu.
 */
async function pedir<T>(caminho: string, init?: RequestInit): Promise<T> {
  let resposta: Response;
  try {
    resposta = await fetch(caminho, {
      ...init,
      headers: { "content-type": "application/json", ...(init?.headers ?? {}) },
    });
  } catch {
    throw new Error("Sem conexão com o M/OS.");
  }
  if (!resposta.ok) {
    const corpo = await resposta.json().catch(() => null);
    throw new Error(corpo?.erro ?? `O servidor respondeu ${resposta.status}.`);
  }
  return (await resposta.json()) as T;
}

export const api = {
  capturar(texto: string) {
    return pedir<{ id: string }>("/api/capturar", {
      method: "POST",
      body: JSON.stringify({ texto }),
    });
  },
  inbox() {
    return pedir<Capture[]>("/api/inbox");
  },
  tasks() {
    return pedir<Task[]>("/api/tasks");
  },
  criarTask(titulo: string) {
    return pedir<Task>("/api/tasks", {
      method: "POST",
      body: JSON.stringify({ titulo }),
    });
  },
  mudarEstado(id: string, estado: Task["state"]) {
    return pedir<Task>(`/api/tasks/${id}/estado`, {
      method: "POST",
      body: JSON.stringify({ estado }),
    });
  },
  estado() {
    return pedir<EstadoDoAparelho>("/api/estado");
  },
};
