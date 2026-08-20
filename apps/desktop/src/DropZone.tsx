import { useCallback, useEffect, useRef, useState } from "react";
import { AnimatePresence, LazyMotion, m, useReducedMotion } from "framer-motion";
import { api, appError } from "./api";
import { Button } from "./Button";
import {
  CHUNK_BYTES,
  conteudoDoDrop,
  fatias,
  loteTerminou,
  painelEspera,
  pareceUrl,
  primeiraUrl,
  recibo,
  type ItemDoLote,
} from "./dropIngest";
import type { DropContext, IngestionReceipt, Project } from "./types";
import { MOTION_DURATIONS, MOTION_EASINGS } from "./motion";

const loadMotionFeatures = () => import("./motionFeatures").then((module) => module.default);

/**
 * A Universal Drop Zone do M/OS com Motion Signature.
 *
 * Não é um `<input type="file">` disfarçado: não há botão, não há formulário e
 * não há tela de upload. O gesto é soltar sobre a janela, e a superfície só
 * existe enquanto o gesto acontece.
 *
 * Motion stages:
 * 1. idle -> hover/dragover (backdrop com blur espacial e mensagem com scaleFade)
 * 2. drop -> reconhecimento -> ingestão com lote em AnimatePresence
 * 3. status transitions suaves por item (esperando -> lendo -> entendendo -> guardado)
 * 4. desfecho com recibo acessível e saída fluida
 */
export function DropZone({
  contexto,
  projects,
  onRecibo,
  refresh,
  onOcupacao,
}: {
  contexto: DropContext;
  projects: Project[];
  /** Mostra o recibo com desfazer, no mesmo lugar em que o resto do app mostra. */
  onRecibo: (mensagem: string, desfazer: () => Promise<void>) => void;
  refresh: () => Promise<void>;
  /* Argos precisa saber que o painel tomou o canto. O estado mora aqui dentro, e
     medir o DOM lá fora leria cache — por isso o aviso sobe em vez de descer. */
  onOcupacao: (ocupado: boolean) => void;
}) {
  const [pairando, setPairando] = useState(false);
  const [itens, setItens] = useState<ItemDoLote[]>([]);
  const reducedMotion = useReducedMotion();
  /* Contador de enter/leave: o `dragleave` dispara ao atravessar CADA elemento
     filho, e sem o contador a superfície piscaria a cada movimento do mouse. */
  const profundidade = useRef(0);
  const ocupado = useRef(false);
  /* Tudo que os ouvintes leem passa por ref.
     Eles são montados uma vez só (ver o efeito abaixo), então qualquer valor
     capturado direto da prop congelaria no primeiro render — e o drop usaria o
     contexto de onde a pessoa ESTAVA quando o app abriu. */
  const contextoAtual = useRef(contexto);
  contextoAtual.current = contexto;
  const projectsAtual = useRef(projects);
  projectsAtual.current = projects;
  /* O painel tomou o canto inferior direito, e Argos precisa ceder. */
  useEffect(() => { onOcupacao(itens.length > 0); }, [itens.length, onOcupacao]);
  const reciboAtual = useRef(onRecibo);
  reciboAtual.current = onRecibo;
  const refreshAtual = useRef(refresh);
  refreshAtual.current = refresh;

  const atualizar = useCallback((chave: string, mudanca: Partial<ItemDoLote>) => {
    setItens((atuais) => atuais.map((item) => (item.chave === chave ? { ...item, ...mudanca } : item)));
  }, []);

  useEffect(() => {
    function entrou(evento: DragEvent) {
      if (conteudoDoDrop(Array.from(evento.dataTransfer?.types ?? [])) === "nenhum") return;
      profundidade.current += 1;
      setPairando(true);
    }
    function sobre(evento: DragEvent) {
      if (conteudoDoDrop(Array.from(evento.dataTransfer?.types ?? [])) === "nenhum") return;
      /* Sem o `preventDefault` o WebView2 abre o arquivo solto na própria
         janela, substituindo o M/OS pelo PDF. */
      evento.preventDefault();
      if (evento.dataTransfer) evento.dataTransfer.dropEffect = "copy";
    }
    function saiu() {
      profundidade.current = Math.max(0, profundidade.current - 1);
      if (profundidade.current === 0) setPairando(false);
    }
    function soltou(evento: DragEvent) {
      const tipos = Array.from(evento.dataTransfer?.types ?? []);
      const conteudo = conteudoDoDrop(tipos);
      profundidade.current = 0;
      setPairando(false);
      if (conteudo === "nenhum" || !evento.dataTransfer) return;
      evento.preventDefault();
      void receber(evento.dataTransfer, conteudo);
    }

    window.addEventListener("dragenter", entrou);
    window.addEventListener("dragover", sobre);
    window.addEventListener("dragleave", saiu);
    window.addEventListener("drop", soltou);
    return () => {
      window.removeEventListener("dragenter", entrou);
      window.removeEventListener("dragover", sobre);
      window.removeEventListener("dragleave", saiu);
      window.removeEventListener("drop", soltou);
    };
    // As funções abaixo leem estado por ref, então o efeito monta uma vez só:
    // remontar os listeners a cada render perderia um drop em curso.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /** Um item por arquivo, e o lote inteiro numa fila só. */
  async function receber(dados: DataTransfer, conteudo: ReturnType<typeof conteudoDoDrop>) {
    if (ocupado.current) return;
    const arquivos = conteudo === "arquivos" ? Array.from(dados.files) : [];
    const texto = conteudo === "arquivos" ? "" : dados.getData("text/uri-list") || dados.getData("text/plain");
    if (!arquivos.length && !texto.trim()) return;

    ocupado.current = true;
    const lote: ItemDoLote[] = arquivos.length
      ? arquivos.map((arquivo, indice) => ({
          chave: `${indice}-${arquivo.name}`,
          nome: arquivo.name,
          status: "esperando" as const,
        }))
      : [{ chave: "texto", nome: rotuloDeTexto(texto), status: "esperando" as const }];
    setItens(lote);

    /* Sequencial de propósito: dois arquivos grandes lidos ao mesmo tempo
       dobram a memória do renderer sem chegar antes — o gargalo é o disco. O
       isolamento que o §15 exige está no `try` de cada volta, e não em
       paralelismo. */
    const concluidos: ItemDoLote[] = [];
    for (const item of lote) {
      const arquivo = arquivos.find((candidato, indice) => `${indice}-${candidato.name}` === item.chave);
      try {
        concluidos.push(arquivo ? await enviarArquivo(item, arquivo) : await enviarTexto(item, texto));
      } catch (erro) {
        const falho = { ...item, status: "erro" as const, erro: appError(erro).message };
        atualizar(item.chave, falho);
        concluidos.push(falho);
      }
    }

    ocupado.current = false;
    await refreshAtual.current().catch(() => undefined);
    anunciar(concluidos);
  }

  async function enviarArquivo(item: ItemDoLote, arquivo: File): Promise<ItemDoLote> {
    atualizar(item.chave, { status: "lendo" });
    const ingestao = await api.ingestBegin({
      name: arquivo.name,
      mime: arquivo.type,
      size: arquivo.size,
      context: contextoAtual.current,
    });
    atualizar(item.chave, { ingestionId: ingestao.id });

    try {
      for (const [inicio, fim] of fatias(arquivo.size, CHUNK_BYTES)) {
        await api.ingestChunk(ingestao.id, await arquivo.slice(inicio, fim).arrayBuffer());
      }
    } catch (erro) {
      // O que falhou foi a leitura no renderer. Avisar o backend é o que fecha
      // a transferência e apaga o pedaço no staging — sem isso ele ficaria
      // aberto até o app fechar.
      await api.ingestAbort(ingestao.id, appError(erro).message).catch(() => undefined);
      throw erro;
    }

    atualizar(item.chave, { status: "entendendo" });
    const feito = await api.ingestFinish(ingestao.id);
    return concluir({ ...item, ingestionId: ingestao.id }, feito);
  }

  async function enviarTexto(item: ItemDoLote, texto: string): Promise<ItemDoLote> {
    atualizar(item.chave, { status: "entendendo" });
    const url = primeiraUrl(texto) || (pareceUrl(texto) ? texto.trim() : "");
    const feito = url
      ? await api.ingestUrl(url, contextoAtual.current)
      : await api.ingestText(texto, contextoAtual.current);
    return concluir({ ...item, ingestionId: feito.ingestion.id }, feito);
  }

  function concluir(item: ItemDoLote, feito: IngestionReceipt): ItemDoLote {
    const sugerido = feito.ingestion.suggestedProjectId;
    const projeto = sugerido
      ? projectsAtual.current.find((candidato) => candidato.id === sugerido)
      : undefined;
    const pronto: ItemDoLote = {
      ...item,
      status: feito.duplicate ? "repetido" : "guardado",
      destino: feito.destination,
      sugestao: projeto ? { projectId: projeto.id, nome: projeto.name } : undefined,
    };
    atualizar(item.chave, pronto);
    return pronto;
  }

  /** O recibo sai no mesmo lugar de sempre, com o mesmo desfazer de sempre. */
  function anunciar(concluidos: readonly ItemDoLote[]) {
    const mensagem = recibo(concluidos);
    const desfaziveis = concluidos
      .filter((item) => item.ingestionId && (item.status === "guardado" || item.status === "repetido"))
      .map((item) => item.ingestionId as string);
    if (!mensagem || !desfaziveis.length) return;
    reciboAtual.current(mensagem, async () => {
      for (const id of desfaziveis) await api.ingestUndo(id).catch(() => undefined);
      setItens((anteriores) =>
        anteriores.map((item) =>
          item.ingestionId && desfaziveis.includes(item.ingestionId)
            ? { ...item, status: "desfeito", sugestao: undefined }
            : item,
        ),
      );
    });
  }

  /* O painel some sozinho quando não tem mais nada a dizer. Erro e sugestão
     esperam a pessoa; sucesso não espera, porque o recibo já falou. */
  useEffect(() => {
    if (!itens.length || !loteTerminou(itens) || painelEspera(itens)) return;
    const relogio = window.setTimeout(() => setItens([]), 2400);
    return () => window.clearTimeout(relogio);
  }, [itens]);

  async function aceitarSugestao(item: ItemDoLote) {
    if (!item.ingestionId) return;
    try {
      await api.ingestAcceptSuggestion(item.ingestionId);
      atualizar(item.chave, { destino: item.sugestao?.nome, sugestao: undefined });
      await refreshAtual.current();
    } catch (erro) {
      atualizar(item.chave, { erro: appError(erro).message });
    }
  }

  return (
    <LazyMotion features={loadMotionFeatures} strict>
      <AnimatePresence>
        {pairando ? (
          <m.div
            className="drop-surface"
            role="presentation"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: reducedMotion ? 0 : MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
          >
            <m.div
              className="drop-message"
              initial={reducedMotion ? { opacity: 0 } : { opacity: 0, scale: 0.96, y: 6 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={reducedMotion ? { opacity: 0 } : { opacity: 0, scale: 0.96, y: -4 }}
              transition={{ duration: reducedMotion ? 0 : MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
            >
              <strong>Solte no M/OS</strong>
              <span>A gente descobre onde isso mora.</span>
            </m.div>
          </m.div>
        ) : null}
      </AnimatePresence>

      <AnimatePresence>
        {itens.length ? (
          <m.section
            className="drop-panel"
            aria-live="polite"
            aria-label="Entrada de conteúdo"
            initial={reducedMotion ? { opacity: 0 } : { opacity: 0, y: 16, scale: 0.97 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={reducedMotion ? { opacity: 0 } : { opacity: 0, y: 10, scale: 0.97 }}
            transition={{ duration: reducedMotion ? 0 : MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
          >
            <div className="drop-items-list">
              <AnimatePresence initial={false} mode="popLayout">
                {itens.map((item) => (
                  <m.article
                    className="drop-item"
                    key={item.chave}
                    data-status={item.status}
                    layout={reducedMotion ? false : "position"}
                    initial={reducedMotion ? { opacity: 0 } : { opacity: 0, y: -4 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={reducedMotion ? { opacity: 0 } : { opacity: 0, scale: 0.97 }}
                    transition={{ duration: reducedMotion ? 0 : MOTION_DURATIONS.state, ease: MOTION_EASINGS.enter }}
                  >
                    <span className="drop-item-mark" aria-hidden="true">
                      {marca(item)}
                    </span>
                    <div className="drop-item-copy">
                      <strong>{item.nome}</strong>
                      <small>{legenda(item)}</small>
                    </div>
                    {item.sugestao && item.status !== "desfeito" ? (
                      <Button
                        variant="ghost"
                        onClick={() => void aceitarSugestao(item)}
                      >{`Relacionar a ${item.sugestao.nome}`}</Button>
                    ) : null}
                  </m.article>
                ))}
              </AnimatePresence>
            </div>
            {painelEspera(itens) ? (
              <Button variant="ghost" onClick={() => setItens([])}>
                Fechar
              </Button>
            ) : null}
          </m.section>
        ) : null}
      </AnimatePresence>
    </LazyMotion>
  );
}

function marca(item: ItemDoLote) {
  if (item.status === "guardado") return "✓";
  if (item.status === "repetido") return "=";
  if (item.status === "erro") return "!";
  if (item.status === "desfeito") return "↺";
  return "○";
}

/**
 * O que a linha diz enquanto o item anda.
 *
 * "Lendo" e "entendendo" são etapas diferentes de verdade, e não decoração: a
 * primeira é o arquivo atravessando, a segunda é o M/OS decidindo o que ele é.
 */
function legenda(item: ItemDoLote) {
  switch (item.status) {
    case "esperando":
      return "Na fila";
    case "lendo":
      return "Lendo…";
    case "entendendo":
      return "Entendendo…";
    case "guardado":
      return item.destino && item.destino !== "Library"
        ? `Guardado · ${item.destino}`
        : "Guardado na Library";
    case "repetido":
      return item.destino && item.destino !== "Library"
        ? `Já estava aqui · relacionado a ${item.destino}`
        : "Já estava aqui";
    case "desfeito":
      return "Desfeito";
    case "erro":
      return item.erro ?? "Falhou";
  }
}

function rotuloDeTexto(texto: string) {
  const limpo = texto.trim().replace(/\s+/g, " ");
  return limpo.length > 60 ? `${limpo.slice(0, 59)}…` : limpo;
}
