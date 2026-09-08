import type { Capture, EstadoDoAparelho, Lembrete, Task } from "./api";

/**
 * O banco de mentira da bancada.
 *
 * Os textos sao realistas de proposito: "task 1" cabe em qualquer largura, e e
 * exatamente por isso que ela nao prova nada. Titulo longo, acento e numero
 * grande sao o que quebra layout de verdade.
 *
 * Este arquivo NAO entra no binario: so a bancada o importa, e a bancada e uma
 * entrada que o `vite build` nao monta.
 */
// O agora e o de VERDADE, e nao um instante fixo: com data cravada, "daqui a 4
// horas" virava "venceu ha 1 h" no dia seguinte, e a bancada passava a mostrar
// um estado que o codigo nunca produz.
const AGORA = new Date();

function ha(minutos: number): string {
  return new Date(AGORA.getTime() - minutos * 60_000).toISOString();
}

function daqui(minutos: number): string {
  return new Date(AGORA.getTime() + minutos * 60_000).toISOString();
}

export const FALSO: {
  capturas: Capture[];
  tasks: Task[];
  lembretes: Lembrete[];
  estado: EstadoDoAparelho;
} = {
  capturas: [
    {
      id: "c1",
      content: "Ligar para o cliente do Rancho Queimado sobre a prancha 04",
      capturedAt: ha(12),
    },
    { id: "c2", content: "Comprar cabo HDMI", capturedAt: ha(180) },
    {
      id: "c3",
      content: "Ideia: o CronoCAD podia sugerir a hora esquecida",
      capturedAt: ha(1500),
    },
  ],
  /* As tres tasks cobrem os tres casos que o desenho precisa aguentar: uma com
     checklist a meio e prazo vencendo hoje, uma NUA (sem nada da 0039, que e a
     Task que ja existia antes dela) e uma concluida. Se a bancada so tivesse
     tasks completas, ninguem veria que o cartao continua limpo sem estrutura. */
  tasks: [
    {
      id: "t1",
      title: "Fechar o levantamento do Rancho Queimado",
      description: "conferir o nível do radier antes de fechar o quantitativo",
      state: "doing",
      projectId: "p1",
      lifecycleState: "active",
      dueAt: daqui(240),
      priority: "high",
      estimateMinutes: 90,
      parentTaskId: null,
      blockedByTaskId: null,
      waitingFor: "",
      followUpAt: null,
      checklistTotal: 5,
      checklistDone: 2,
      createdAt: ha(3000),
      completedAt: null,
    },
    {
      id: "t2",
      title: "Revisar a planta do quiosque",
      description: "",
      state: "planned",
      projectId: null,
      lifecycleState: "active",
      dueAt: null,
      priority: "normal",
      estimateMinutes: null,
      parentTaskId: null,
      blockedByTaskId: null,
      waitingFor: "Victor",
      followUpAt: daqui(2880),
      checklistTotal: 0,
      checklistDone: 0,
      createdAt: ha(2000),
      completedAt: null,
    },
    {
      id: "t3",
      title: "Mandar a fatura de agosto",
      description: "",
      state: "done",
      projectId: "p1",
      lifecycleState: "active",
      dueAt: ha(4000),
      priority: "normal",
      estimateMinutes: null,
      parentTaskId: null,
      blockedByTaskId: null,
      waitingFor: "",
      followUpAt: null,
      checklistTotal: 3,
      checklistDone: 3,
      createdAt: ha(5000),
      completedAt: ha(100),
    },
  ],
  lembretes: [
    {
      id: "l1",
      title: "Mandar a fatura de agosto",
      body: "",
      target: { type: "task", id: "t3" },
      status: "due",
      priority: "high",
      nextDueAt: ha(30),
      snoozeCount: 0,
      kind: "standard",
      waitingFor: "",
      persistent: true,
      recurrence: null,
      createdAt: ha(600),
      updatedAt: ha(600),
      lifecycleState: "active",
    },
    {
      id: "l2",
      title: "Reuniao com o Juliano",
      body: "",
      target: null,
      status: "scheduled",
      priority: "normal",
      nextDueAt: daqui(240),
      snoozeCount: 1,
      kind: "standard",
      waitingFor: "",
      persistent: false,
      recurrence: null,
      createdAt: ha(2000),
      updatedAt: ha(2000),
      lifecycleState: "active",
    },
  ],
  estado: { pendentes: 3, sincroniza: true, chavePush: "chave-falsa", aparelhosAvisados: 1 },
};
