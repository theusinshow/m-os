import type { Conversation } from "../hermes";

/**
 * O que esta unidade precisa do mundo: criar uma conversa e dizer ao lado Rust
 * qual e a corrente. Nada mais — e por isso ela e testavel sem Tauri.
 */
export interface PortaDaConversa {
  create(): Promise<Conversation>;
  selecionar(id: string): Promise<void>;
}

/**
 * A conversa nasce quando a pessoa fala, e nao quando a tela abre.
 *
 * Criar no clique enchia o banco de conversas vazias — e elas atravessavam a
 * sincronizacao para o celular. Aqui a criacao e adiada ate haver pergunta, e a
 * criacao em curso e lembrada para que duas perguntas no mesmo instante nao
 * virem duas conversas.
 */
export function conversaSobDemanda(porta: PortaDaConversa) {
  let nascendo: Promise<Conversation | null> | null = null;

  return {
    /**
     * A conversa onde a pergunta deve cair. Devolve `null` so quando a criacao
     * falha — a tela entao avisa, e a proxima tentativa tenta de novo.
     */
    async garantir(atual: Conversation | null): Promise<Conversation | null> {
      if (atual) return atual;
      if (!nascendo) {
        nascendo = (async () => {
          const criada = await porta.create();
          await porta.selecionar(criada.id);
          return criada;
        })().catch(() => {
          // Uma promessa falhada guardada travaria a tela ate trocar de pagina.
          nascendo = null;
          return null;
        });
      }
      return nascendo;
    },

    /** Recomeca do zero: o "+" e o Ctrl+N voltam ao rascunho. */
    reiniciar() {
      nascendo = null;
    },
  };
}
