import { DragEvent, Fragment, FormEvent, KeyboardEvent, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Amplitude, PendingVoice, useVoiceHud, VoiceFooter, VoiceSurface } from "./Voice";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api, appError } from "./api";
import { DotField } from "./DotField";
/* O arranjo da Home mora fora daqui para poder ser testado: sem DOM no runner
   (ver `vitest.config.ts`), o que da para verificar tem de ser funcao pura. */
import { arrangeHome, fillBand, HOME_SECTIONS, HOME_SIZES, HOME_WIDGETS, moveInArrangement, placementsFor, touchedSections, type ArrangedWidget, type HomeWidgetRole, type HomeWidgetSpan, type PlacedWidget } from "./homeLayout";
import { resolveFunctionTarget, type FunctionIntentTarget } from "./functionIntents";
import { hermes, type HermesConnectionState, type HermesFailure, type HermesStatus } from "./hermes";
import { HermesPage } from "./HermesPage";
import { AppIcon } from "./AppIcon";
import { Argos, useArgosPose, useArgosPresenca } from "./Argos";
import { ProcessingBar } from "./ProcessingBar";
import { deveEsperarAbertura, esperaDaTentativa } from "./abertura";
import { cantoPara } from "./argosCorner";
import { Button } from "./Button";
import { ActionMenu, ContextPath, EmptyState, Inspector, PaneHeader, Panel, StateMessage } from "./Surface";
import { CalendarPage } from "./CalendarPage";
import { AcademicPage, AcademicWidget } from "./AcademicPage";
import { DailyFocusWidget, DailySessionView, useDaily } from "./DailySession";
import { dataPorExtenso } from "./daily";
import { atividadePorProject, diasPorTask, mexidoHoje, paradasVisiveis, projectsParados, rotuloDeDias } from "./stale";
import { EndMyDayFlow, StartMyDayFlow } from "./DailyFlows";
import { MeetingSettings } from "./MeetingSettings";
import { MeetingsPage } from "./MeetingsPage";
import { RecordingBar } from "./RecordingBar";
import { Reminder } from "./Reminder";
import { ReuniaoDetectada } from "./ReuniaoDetectada";
import { AttentionCenter } from "./AttentionCenter";
import { ReminderComposer } from "./ReminderComposer";
import { BudgetRing, hoursLabel, TodayHours, useTrackedTime, WeekByProject, weekSummary } from "./TimeWidgets";
import { TempoPage } from "./TempoPage";
import { FinancePage } from "./FinancePage";
import { finance } from "./finance";
import { Timer } from "./Timer";
import { Icon, type IconName } from "./Icon";
import { DropZone } from "./DropZone";
import { contextoDoDrop } from "./dropIngest";
import { Leque } from "./Leque";
import { LequeSeletor } from "./LequeSeletor";
import { Ring, RingLabel } from "./Ring";
import { monthActivity, MonthDensity, TaskProgressRing, WeekRings } from "./Widgets";
import { MosSymbol } from "./Symbol";
import { AnimatePresence, LazyMotion, m } from "framer-motion";
import { AnimatedList, AnimatedListItem } from "./motion/AnimatedList";
import { SpotlightCard } from "./motion/SpotlightCard";
import { MOTION_DURATIONS, MOTION_EASINGS } from "./motion";
import type { AcademicDashboard, AppCapabilities, AppCatalogEntry, AppLaunchKind, AppStatus, BackupInspection, Capture, DailyContext, DailyToday, FunctionDefinition, HiddenWidget, Ingestion, ObjectiveLink, Week, WidgetPlacement, RadialPin, Page, ImportReport, Project, RegisteredApp, Resource, ResourceKind, ResourceWorkspace, Parada, SearchItem, StaleView, Task, TaskState, UpdateInfo, UpdateProgress, Workspace , DeliveryEvent, UnivirtusStatus, SyncReport } from "./types";
import { SCREEN_LABEL } from "./types";
import "./App.css";

const loadMotionFeatures = () => import("./motionFeatures").then((module) => module.default);

/* `apps` continua sendo uma pagina, e so deixou de ser um destino do rail
   (ADR-038). Ela e alcancada pelo Command, pelo widget APPS da Home e pelos
   Workspaces — a pagina existe, o icone no rail e que saiu. */
type UndoAction = { message: string; run: () => Promise<unknown> };

/**
 * O que a Home precisa saber sobre o dia.
 *
 * Um objeto so, e nao dez props soltas: a lista de props da `HomePage` ja tem
 * trinta e duas, e acrescentar dez trocaria a legibilidade do que sobrou por
 * nada — a Home nao usa nenhuma delas, ela so repassa para o widget.
 */
type DailyProps = {
  dia: DailyToday | null;
  contexto: DailyContext | null;
  carregando: boolean;
  erro: string;
  iniciar: () => void;
  abrirSessao: () => void;
  encerrarAntigo: () => void;
  concluirObjetivo: (id: string) => void;
  abrirVinculo: (link: ObjectiveLink) => void;
  semanaPendente: Week | null;
  abrirSemana: () => void;
};

/**
 * Os atalhos que existem de verdade.
 *
 * A auditoria deu 1 de 10 em "ajuda e documentação", e o motivo não era falta
 * de recurso: era o contrário. O app é operável quase inteiro pelo teclado e
 * nada disso estava escrito em lugar nenhum — quem não descobrisse por acidente
 * nunca saberia.
 *
 * A lista é escrita à mão de propósito. Derivá-la dos handlers daria uma
 * garantia falsa de sincronia e produziria rótulos como "keydown ctrl+k"; o que
 * falta documentar aqui é o QUE a tecla faz, e isso só existe na cabeça de quem
 * escreveu. O preço é manutenção: atalho novo entra aqui na mão.
 */
const SHORTCUTS: { keys: string; does: string }[] = [
  { keys: "Ctrl + K", does: "Abrir a busca e os comandos" },
  { keys: "Ctrl + Z", does: "Desfazer a última ação, enquanto o recibo estiver na tela" },
  { keys: "Ctrl + 1…9", does: "Abrir o app na posição correspondente, na Home" },
  { keys: "Esc", does: "Fechar, cancelar ou interromper o que estiver em curso" },
  { keys: "↑ ↓ Home End", does: "Navegar entre as linhas de uma lista" },
  { keys: "Ctrl + N", does: "Nova conversa, no Hermes" },
  { keys: "Ctrl + /", does: "Mostrar ou ocultar a coluna de conversas, no Hermes" },
  { keys: "↑ (campo vazio)", does: "Editar a última pergunta enviada, no Hermes" },
  { keys: "Shift + Enter", does: "Quebrar linha em vez de enviar, no Hermes" },
  { keys: "Ctrl + Alt + G", does: "Segurar para falar, de qualquer lugar do Windows" },
  { keys: "Alt (segurado)", does: "Falar, com a Captura rápida já aberta" },
];
type Theme = "dark" | "light";
type CommandResult = SearchItem | { kind: "function"; function: FunctionDefinition };
type FunctionIntent = { target: FunctionIntentTarget; key: number };

// A ordem e a ordem das colunas do kanban. DOING e a unica coluna em sodio:
// e o estado que importa.
/* Espelha o teto que list_inbox pede em src-tauri/src/lib.rs:84. Se mudar la, muda
   aqui: e o que permite a Home admitir que a contagem esta truncada. */
const INBOX_PAGE = 200;

const stateOrder: TaskState[] = ["inbox", "backlog", "planned", "doing", "review", "done"];
const stateLabels: Record<TaskState, string> = { inbox: "Inbox", backlog: "Backlog", planned: "Planned", doing: "Doing", review: "Review", done: "Done" };
const functionCategories: FunctionDefinition["category"][] = ["capture", "daily", "work", "time", "memory", "app", "data", "system"];
const functionCategoryLabels: Record<FunctionDefinition["category"], string> = { capture: "CAPTURE", daily: "DIA", work: "WORK", time: "TEMPO", attention: "ATENÇÃO", memory: "MEMORY", meeting: "REUNIÕES", app: "APP", data: "DATA", system: "SYSTEM" };
const functionRiskLabels: Record<FunctionDefinition["risk"], string> = { low: "baixo", medium: "medio", high: "alto" };
const functionConfirmationLabels: Record<FunctionDefinition["confirmation"], string> = { none: "sem confirmacao", explicit: "confirmacao explicita" };
const relativeFormatter = new Intl.RelativeTimeFormat("pt-BR", { numeric: "auto" });

function relativeTime(value: string) {
  const seconds = Math.round((new Date(value).getTime() - Date.now()) / 1_000);
  if (Math.abs(seconds) < 60) return "agora";
  const minutes = Math.round(seconds / 60);
  if (Math.abs(minutes) < 60) return relativeFormatter.format(minutes, "minute");
  const hours = Math.round(minutes / 60);
  if (Math.abs(hours) < 24) return relativeFormatter.format(hours, "hour");
  return relativeFormatter.format(Math.round(hours / 24), "day");
}

function sourceLabel(source: Capture["source"]) {
  /* A origem falada aparece com o mesmo peso das outras. Ela nao ganha etiqueta
     especial nem icone: uma Capture falada e uma Capture, e trata-la como
     categoria a parte seria contrariar o §Voz do design system logo na primeira
     tela em que ela aparece. */
  if (source === "quick_capture") return "Quick Capture";
  if (source === "drop") return "Drop";
  if (source === "voice") return "Voz";
  return "Home";
}

/** Tamanho legivel. Uma coluna de bytes crus nao informa nada a ninguem. */
function fileSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(bytes < 10 * 1024 * 1024 ? 1 : 0)} MB`;
}

/**
 * O que o M/OS conseguiu ler de dentro do arquivo.
 *
 * `empty` e `unsupported` NAO sao erro, e a copia diz isso: um PDF escaneado ou
 * um .zip continuam guardados e reencontraveis pelo nome. Chamar aquilo de falha
 * ensinaria a desconfiar de um sistema que fez o que prometeu.
 */
function extractionLabel(ingestion: Ingestion) {
  switch (ingestion.extractionState) {
    case "pending":
      return "Lendo o conteúdo…";
    case "done":
      return ingestion.pageCount ? `Conteúdo indexado · ${ingestion.pageCount} páginas` : "Conteúdo indexado";
    case "empty":
      return "Sem texto para indexar — guardado do mesmo jeito";
    case "unsupported":
      return "Conteúdo não é lido nesta versão — guardado do mesmo jeito";
    case "failed":
      return `Não deu para ler o conteúdo: ${ingestion.extractionError}`;
  }
}

function resourceHost(url: string) {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}


function IconButton({ label, icon, active = false, disabled = false, onClick }: { label: string; icon: IconName; active?: boolean; disabled?: boolean; onClick: () => void }) {
  return <button className="icon-button" type="button" aria-label={label} title={label} disabled={disabled} onClick={onClick}><Icon name={icon} filled={active} /></button>;
}


/* Cuida so da moldura e do rodape. O rotulo continua no Panel, e a POSICAO na
   grade agora vem resolvida de fora — o widget nao sabe mais qual e a largura
   dele, porque ela pode ter sido escolhida pela pessoa. */
function Widget({ id, role, span, footLeft, footRight, children }: { id: string; role: HomeWidgetRole; span: number; footLeft?: string; footRight?: string; children: ReactNode }) {
  return (
    <SpotlightCard className="widget" data-widget={id} data-role={role} data-span={span}>
      {children}
      {/* Escala à esquerda, extremo à direita — o rodapé diz contra o que a forma
          mede. Lista não tem escala, e por isso lista não recebe rodapé. A
          manchete NÃO mora aqui: ela precisa ficar entre o rótulo e o conteúdo,
          que são o mesmo bloco dentro do Panel. */}
      {footLeft || footRight ? <p className="widget-foot"><span>{footLeft}</span><span>{footRight}</span></p> : null}
    </SpotlightCard>
  );
}

/**
 * A Home inteira: as faixas, e os widgets dentro delas na ordem escolhida.
 *
 * Os widgets chegam numa lista PLANA, e nao aninhados na faixa a que pertencem.
 * Foi o que mover entre faixas exigiu: com o JSX declarando a faixa, o widget
 * so podia ser desenhado onde estava escrito, e a escolha da pessoa nao teria
 * onde caber.
 *
 * A faixa vazia some sozinha, em vez de cada chamada listar a mao quem mora
 * nela. Aquela lista virou mentira no instante em que um widget pode mudar de
 * faixa — e mentira em codigo de visibilidade some com widget na tela.
 */
function HomeBoard({ widgets, arrangement, arranging, hiddenIds, onMove, onResize, onHide }: { widgets: { id: string; available?: boolean; footLeft?: string; footRight?: string; node: ReactNode }[]; arrangement: ArrangedWidget[]; arranging: boolean; hiddenIds: Set<string>; onMove: (id: string, section: string, before: string | null) => void; onResize: (id: string, span: HomeWidgetSpan | null) => void; onHide: (id: string, hidden: boolean) => void }) {
  const nodes = new Map(widgets.map((widget) => [widget.id, widget] as const));

  return <>{HOME_SECTIONS.map((section, sectionIndex) => {
    const moram = arrangement.filter((slot) => {
      const node = nodes.get(slot.id);
      return slot.section === section.id && node !== undefined && node.available !== false;
    });
    /* Arrumando, o widget OCULTO continua na grade. Sem isso, esconder seria uma
       porta de mao unica: o widget sumia e o unico caminho de volta era o
       inspetor de Workspace — que e justamente a tela de onde este controle
       saiu. O que se esconde precisa continuar alcancavel de onde se escondeu.

       `fillBand` fecha a ultima linha da faixa. Um widget que se esconde sozinho
       — a META, quando nenhum Project tem meta — deixaria uma sobra que ninguem
       escolheu, e nao existe arranjo de tamanhos fixos que feche com e sem ele. */
    const slots = fillBand(arranging ? moram : moram.filter((slot) => !hiddenIds.has(slot.id)));
    /* Arrumando, a faixa vazia FICA: ela e o alvo de quem quer mover um widget
       para ca. Em repouso ela some, porque um titulo sobre o nada nao informa. */
    if (!slots.length && !arranging) return null;
    const headingId = `home-${section.id}-heading`;

    return <section className="home-section" data-section={section.id} data-arranging={arranging || undefined} key={section.id} aria-labelledby={headingId}>
      <header className="home-section-heading"><h2 id={headingId}>{section.title}</h2></header>
      <div
        className="home-grid"
        onDragOver={arranging ? (event) => { event.preventDefault(); event.dataTransfer.dropEffect = "move"; } : undefined}
        onDrop={arranging ? (event) => { event.preventDefault(); const dragged = event.dataTransfer.getData("text/mos-widget"); if (dragged) onMove(dragged, section.id, null); } : undefined}
      >
        {slots.map((slot, index) => {
          const node = nodes.get(slot.id);
          if (!node) return null;
          const widget = <Widget id={slot.id} role={slot.role} span={slot.renderSpan} footLeft={node.footLeft} footRight={node.footRight}>{node.node}</Widget>;
          /* Em repouso o widget E o item da grade: nenhum involucro, nenhum
             controle, nenhum listener de arrasto. A ADR-034 pede a Home lida em
             meio segundo, e o jeito mais barato de honrar isso e o modo de
             leitura nao carregar o peso do modo de edicao. */
          if (!arranging) return <Fragment key={slot.id}>{widget}</Fragment>;
          return <Arrangeable
            key={slot.id}
            slot={slot}
            section={section.id}
            first={index === 0}
            last={index === slots.length - 1}
            previous={index > 0 ? slots[index - 1].id : null}
            next={index + 1 < slots.length ? slots[index + 1].id : null}
            afterNext={index + 2 < slots.length ? slots[index + 2].id : null}
            hidden={hiddenIds.has(slot.id)}
            onHide={onHide}
            bandAbove={sectionIndex > 0 ? HOME_SECTIONS[sectionIndex - 1].id : null}
            bandBelow={sectionIndex + 1 < HOME_SECTIONS.length ? HOME_SECTIONS[sectionIndex + 1].id : null}
            onMove={onMove}
            onResize={onResize}
          >{widget}</Arrangeable>;
        })}
        {arranging && !slots.length ? <p className="home-drop-hint">Faixa vazia. Arraste um widget para cá.</p> : null}
      </div>
    </section>;
  })}</>;
}

/**
 * Um widget no modo de arrumar: a barra de controle, e o gesto de arrastar.
 *
 * A barra e uma LINHA DE VERDADE acima do card, e nao uma camada flutuando
 * sobre ele. A primeira tentativa flutuava no canto superior direito, que e
 * exatamente onde o `.panel-header` desenha "Ver todos" e "Gerenciar" — passar
 * o mouse por um widget engolia o botao de acao dele. Uma linha propria nao tem
 * como colidir com nada, e so existe enquanto se esta arrumando.
 *
 * Arrastar sai do PUNHO, e nao do card inteiro. Um card inteiro `draggable`
 * cobre rows clicaveis, os tiles de app e o cronometro: qualquer tentativa de
 * selecionar texto virava um arrasto. O Kanban ja segue essa regra.
 *
 * As setas NAO sao enfeite: a `DESIGN-FOUNDATIONS.md` §12 diz que "nenhum fluxo
 * critico depende de drag and drop". Arrastar e o caminho rapido; as setas sao
 * o caminho que sempre existe — inclusive para mover entre faixas, que e o que
 * ↑ e ↓ fazem.
 */
function Arrangeable({ slot, section, first, last, previous, next, afterNext, bandAbove, bandBelow, hidden, onMove, onResize, onHide, children }: { slot: PlacedWidget; section: string; first: boolean; last: boolean; previous: string | null; next: string | null; afterNext: string | null; bandAbove: string | null; bandBelow: string | null; hidden: boolean; onMove: (id: string, section: string, before: string | null) => void; onResize: (id: string, span: HomeWidgetSpan | null) => void; onHide: (id: string, hidden: boolean) => void; children: ReactNode }) {
  const [over, setOver] = useState<"before" | "after" | null>(null);

  /* Qual metade do card o cursor esta pedindo. Sem isso o alvo do arrasto e o
     card inteiro, e nao da para dizer se o widget cai antes ou depois dele. */
  function side(event: DragEvent<HTMLDivElement>): "before" | "after" {
    const box = event.currentTarget.getBoundingClientRect();
    return event.clientX < box.left + box.width / 2 ? "before" : "after";
  }

  return (
    <div
      className="arrangeable"
      data-hidden={hidden || undefined}
      data-span={slot.renderSpan}
      data-over={over ?? undefined}
      onDragOver={(event) => { event.preventDefault(); event.stopPropagation(); event.dataTransfer.dropEffect = "move"; setOver(side(event)); }}
      onDragLeave={() => setOver(null)}
      onDrop={(event) => {
        event.preventDefault();
        /* Sem isto o `onDrop` da faixa dispara logo depois e manda o widget
           para o fim, desfazendo a mira que a pessoa acabou de fazer. */
        event.stopPropagation();
        const onde = side(event);
        setOver(null);
        const dragged = event.dataTransfer.getData("text/mos-widget");
        if (!dragged || dragged === slot.id) return;
        onMove(dragged, section, onde === "before" ? slot.id : next);
      }}
    >
      <div className="arrange-bar">
        {/* O punho e o unico pedaco arrastavel, e some do leitor de tela: o
            caminho por teclado sao as setas ao lado, e anunciar os dois faria a
            mesma acao existir duas vezes. */}
        <span className="arrange-grip" aria-hidden="true" draggable onDragStart={(event) => { event.dataTransfer.setData("text/mos-widget", slot.id); event.dataTransfer.effectAllowed = "move"; }} onDragEnd={() => setOver(null)}>⠿</span>
        {/* O nome do widget NAO se repete aqui: o `.panel-header` logo abaixo ja
            o diz, e escrever duas vezes a mesma palavra a 20px de distancia so
            gasta a linha. Ele continua nos `aria-label` dos botoes, que e onde
            faz falta — "Alargar" sozinho nao diz alargar o que. */}
        {/* Vao. A palavra "OCULTO" ja morou aqui e nao cabia: num widget de uma
            unidade a barra leva punho, tres tamanhos, quatro setas e o botao de
            esconder, e o vao encolhia ate cortar a palavra num "O" que parecia
            um zero. O rotulo foi para o card, que tem espaco. */}
        <span className="arrange-name" aria-hidden="true" />
        {/* Tres tamanhos prontos, e nao uma largura que se ajusta de a um. E o
            modelo da tela do iPhone: escolher um FORMATO, nao acertar uma
            medida. O numero na etiqueta e quantos quartos da linha o widget
            ocupa, que e a mesma conta que a pessoa faz olhando. */}
        <span className="arrange-group" role="group" aria-label={`Tamanho de ${slot.label}`}>
          {HOME_SIZES.map((size) => <button
            key={size.units}
            className="icon-button"
            type="button"
            aria-label={`${size.label} — ${slot.label}`}
            title={size.label}
            aria-pressed={slot.span === size.span}
            data-selected={slot.span === size.span || undefined}
            onClick={() => onResize(slot.id, size.span)}
          >{size.units}</button>)}
        </span>
        <span className="arrange-group" role="group" aria-label={`Mover ${slot.label}`}>
          <button className="icon-button" type="button" aria-label={`Mover ${slot.label} para trás`} disabled={first} onClick={() => onMove(slot.id, section, previous)}>←</button>
          <button className="icon-button" type="button" aria-label={`Mover ${slot.label} para frente`} disabled={last} onClick={() => onMove(slot.id, section, afterNext)}>→</button>
          <button className="icon-button" type="button" aria-label={`Mover ${slot.label} para a faixa acima`} disabled={!bandAbove} onClick={() => { if (bandAbove) onMove(slot.id, bandAbove, null); }}>↑</button>
          <button className="icon-button" type="button" aria-label={`Mover ${slot.label} para a faixa abaixo`} disabled={!bandBelow} onClick={() => { if (bandBelow) onMove(slot.id, bandBelow, null); }}>↓</button>
        </span>
        {/* Esconder nao apaga nada — a escolha e uma linha no banco, e trazer de
            volta e o mesmo botao. Por isso o rotulo fala em Home, e nao em
            excluir: "×" aqui tira da tela, e nao do sistema.

            `data-function-action` amarra o botao ao registro de Functions, que
            e onde a capacidade esta declarada. Ela morava no inspetor de
            Workspace e mudou de nome junto com o lugar. */}
        <button className="icon-button" data-function-action="home.set_widget" type="button" aria-label={hidden ? `Mostrar ${slot.label} na Home` : `Ocultar ${slot.label} da Home`} title={hidden ? "Mostrar na Home" : "Ocultar da Home"} onClick={() => onHide(slot.id, !hidden)}>{hidden ? "+" : "×"}</button>
      </div>
      {/* Apagado sozinho diria "desligado" ou "carregando" tao bem quanto diria
          "oculto". A palavra tira a duvida, e e decorativa de proposito: quem
          usa leitor de tela ja recebe o estado pelo `aria-label` do botao, e
          ouvi-lo duas vezes por widget seria pior que nao ouvir. */}
      {hidden ? <span className="arrange-hidden-mark" aria-hidden="true">OCULTO</span> : null}
      {/* `inert` e nao `pointer-events`: arrumando, o conteudo do widget sai do
          foco tambem. Sem isso, tabular pela Home em modo de edicao passaria por
          cada row de cada lista antes de chegar no proximo widget. */}
      <div className="arrangeable-body" inert>{children}</div>
    </div>
  );
}

/* A promessa central do produto e confianca: o que entrou esta guardado. Quando
   tudo esta bem este e o elemento mais silencioso da tela; so ganha peso ao falhar. */
function SystemHealth({ status }: { status: AppStatus | null }) {
  const [hermesState, setHermesState] = useState<HermesConnectionState>("offline");
  useEffect(() => {
    void hermes.status().then((next) => setHermesState(next.state)).catch(() => undefined);
    const subscription = hermes.onState((next) => setHermesState(next.state));
    return () => { void subscription.then((dispose) => dispose()); };
  }, []);
  const saved = status?.storage.integrity === "ok";
  // Falha fechado de proposito. `snapshot` e texto livre vindo de schedule_snapshot
  // (src-tauri/src/lib.rs:801) e tem tres formas: "Snapshot diario criado.",
  // "Snapshot diario ja existe." e "Falha no snapshot diario: ...". Marcar como ok
  // qualquer string nao vazia pintaria a falha de verde — justamente o caso em que
  // este widget precisa ser honesto. Casar com o sucesso, e nao com a falha, faz
  // qualquer mensagem futura desconhecida ficar sem o verde em vez de ganha-lo.
  const backupOk = status?.snapshot.startsWith("Snapshot") ?? false;
  return <dl className="health-list">
    <div><dt>Dados</dt><dd data-ok={saved || undefined}>{saved ? "Salvos" : status ? status.storage.integrity : "—"}</dd></div>
    <div><dt>Backup</dt><dd data-ok={backupOk || undefined}>{status?.snapshot || "—"}</dd></div>
    <div><dt>Hermes</dt><dd data-ok={hermesState === "online" || undefined}>{hermesState === "online" ? "Online" : hermesState === "connecting" ? "Conectando" : "Offline"}</dd></div>
  </dl>;
}

/* O vazio de um painel com escopo tem duas causas que a mensagem antiga confundia:
   nada cadastrado, ou nada vinculado ao Workspace ativo. Sem separar as duas, a Home
   afirma que o usuario nao tem apps enquanto esconde os que ele tem. */
function ScopedEmptyState({ total, workspace, noun, onLink, linkLabel = "Vincular" }: { total: number; workspace: Workspace | null; noun: "app" | "project" | "resource"; onLink: () => void; linkLabel?: string }) {
  if (total === 0 || !workspace) {
    return <EmptyState>{noun === "app" ? "Apps cadastrados aparecerão aqui." : noun === "resource" ? "Referências salvas aparecerão aqui." : "Projects criados aparecerão aqui."}</EmptyState>;
  }
  const counted = noun === "app"
    ? `${total} ${total === 1 ? "app cadastrado" : "apps cadastrados"}`
    : noun === "resource"
      ? `${total} ${total === 1 ? "resource salvo" : "resources salvos"}`
      : `${total} ${total === 1 ? "Project criado" : "Projects criados"}`;
  return <div className="scoped-empty"><EmptyState>{`${counted}, nenhum em ${workspace.name}.`}</EmptyState><Button variant="outline" size="sm" onClick={onLink}>{linkLabel}</Button></div>;
}

function CaptureComposer({ onSaved, focusKey }: { onSaved: (capture: Capture) => void; focusKey?: number }) {
  const [content, setContent] = useState("");
  const [state, setState] = useState<"idle" | "saving" | "success" | "error">("idle");
  const [feedback, setFeedback] = useState("");
  const [focused, setFocused] = useState(false);
  const input = useRef<HTMLTextAreaElement>(null);
  useEffect(() => { if (focusKey !== undefined) input.current?.focus(); }, [focusKey]);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!content.trim() || state === "saving") return;
    setState("saving");
    setFeedback("Salvando localmente...");
    try {
      const capture = await api.createCapture(content, "home");
      setContent("");
      setState("success");
      setFeedback("Salvo na Inbox");
      onSaved(capture);
    } catch (error) {
      setState("error");
      setFeedback(`${appError(error).message} O texto continua aqui.`);
    }
  }

  // Sem caixa e sem borda: a excecao deliberada do sistema. O caret de bloco so
  // aparece com o campo vazio, como convite — a partir do primeiro caractere
  // quem manda e o caret nativo, e dois caretes na tela seria mentira visual.
  // Sem caixa e sem borda: a excecao deliberada do sistema.
  //
  // O placeholder e o caret sao desenhados por cima do textarea, e nao pelo
  // atributo placeholder, porque o desenho poe o caret de bloco colado no fim
  // do texto. Com placeholder nativo o caret so poderia ficar na borda do
  // campo, que e onde ele estava antes desta correcao. Some ao focar: dali em
  // diante quem manda e o caret nativo, e dois caretes seriam mentira visual.
  return <form className="capture-field" data-state={state} onSubmit={submit}>
    <div className="capture-line">
      <span className="capture-bar" aria-hidden="true" />
      <div className="capture-input">
        <textarea ref={input} aria-label="Conteúdo da captura" value={content} onChange={(event) => { setContent(event.currentTarget.value); if (state !== "idle") { setState("idle"); setFeedback(""); } }} onFocus={() => setFocused(true)} onBlur={() => setFocused(false)} onKeyDown={(event) => { if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); event.currentTarget.form?.requestSubmit(); } }} rows={1} />
        {!content && !focused ? <span className="capture-ghost" aria-hidden="true">What's on your mind?<i className="capture-caret" /></span> : null}
      </div>
      <Button className="capture-save" variant="primary" type="submit" disabled={!content.trim() || state === "saving"}>{state === "saving" ? "Salvando" : "Salvar ⏎"}</Button>
    </div>
    {feedback && state !== "idle" ? <StateMessage state={state === "success" ? "saved" : state} label={feedback} /> : null}
  </form>;
}

/** Progresso do Project: barra de 2px mais a contagem. A barra diz o quanto
 *  falta antes de o olho ler o numero — e o numero confirma. */
function RowProgress({ done, total }: { done: number; total: number }) {
  return <><span className="row-progress" aria-hidden="true"><i style={{ transform: `scaleX(${total ? done / total : 0})` }} /></span><span className="row-progress-count">{done}/{total}</span></>;
}

/** `secondaryKind` decide a familia da segunda linha. Origem de captura e tipo
 *  de lancamento sao dado de sistema e vao em mono; descricao de Project e
 *  texto do usuario e vai em grotesk. O AGENTS.md e explicito: mono nunca
 *  vaza para conteudo. */
function DataRow({ primary, meta, secondary, secondaryKind = "text", marker, progress, selected = false, completed = false, saved = false, dragging = false, stale = false, onClick, onKeyDown, onPointerDown, draggable, onDragStart, onDragEnd }: { primary: string; meta?: string; secondary?: string; secondaryKind?: "text" | "system"; marker?: ReactNode; progress?: { done: number; total: number }; selected?: boolean; completed?: boolean; saved?: boolean; dragging?: boolean; stale?: boolean; onClick?: () => void; onKeyDown?: (event: KeyboardEvent<HTMLButtonElement>) => void; onPointerDown?: React.PointerEventHandler<HTMLButtonElement>; draggable?: boolean; onDragStart?: React.DragEventHandler<HTMLButtonElement>; onDragEnd?: React.DragEventHandler<HTMLButtonElement> }) {
  return <button className="data-row" type="button" aria-current={selected ? "true" : undefined} data-selected={selected || undefined} data-completed={completed || undefined} data-saved={saved || undefined} data-dragging={dragging || undefined} data-stale={stale || undefined} onClick={onClick} onKeyDown={onKeyDown} onPointerDown={onPointerDown} draggable={draggable} onDragStart={onDragStart} onDragEnd={onDragEnd}>{marker}<span className="row-copy"><strong>{primary}</strong>{secondary ? <small data-system={secondaryKind === "system" || undefined}>{secondary}</small> : null}</span>{progress ? <RowProgress done={progress.done} total={progress.total} /> : null}{meta ? <span className="row-meta">{meta}</span> : null}</button>;
}

function moveListFocus(event: KeyboardEvent<HTMLButtonElement>) {
  if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return null;
  const rows = Array.from(event.currentTarget.closest(".row-list")?.querySelectorAll<HTMLButtonElement>(".data-row") ?? []);
  const currentIndex = rows.indexOf(event.currentTarget);
  if (currentIndex < 0 || !rows.length) return null;
  event.preventDefault();
  const nextIndex = event.key === "Home"
    ? 0
    : event.key === "End"
      ? rows.length - 1
      : Math.max(0, Math.min(rows.length - 1, currentIndex + (event.key === "ArrowDown" ? 1 : -1)));
  rows[nextIndex]?.focus();
  return nextIndex;
}

function HomePage({ recent, inbox, projects, tasks, stale, academic, workspaces, apps, resources, resourceWorkspaces, status, hiddenWidgets, setHiddenWidgets, widgetPlacements, setWidgetPlacements, refresh, openCapture, openProject, openWorkspace, openTask, openApp, openResource, openInbox, openTasksPage, openTempoPage, openProjectsPage, openLibraryPage, openAppsPage, openFinancePage, openCalendarPage, openMeetingsPage, openAcademicPage, currentWorkspaceId, setCurrentWorkspaceId, currentWorkspace, intent, daily }: { recent: Capture[]; inbox: Capture[]; projects: Project[]; tasks: Task[]; stale: StaleView; academic: AcademicDashboard | null; workspaces: Workspace[]; apps: RegisteredApp[]; resources: Resource[]; resourceWorkspaces: ResourceWorkspace[]; status: AppStatus | null; hiddenWidgets: HiddenWidget[]; setHiddenWidgets: (next: HiddenWidget[]) => void; widgetPlacements: WidgetPlacement[]; setWidgetPlacements: (next: WidgetPlacement[]) => void; refresh: () => Promise<void>; openCapture: (capture: Capture) => void; openProject: (project: Project) => void; openWorkspace: (workspace: Workspace) => void; openTask: (task: Task) => void; openApp: (app: RegisteredApp) => void; openResource: (resource: Resource) => void; openInbox: () => void; openTasksPage: () => void; openTempoPage: () => void; openProjectsPage: () => void; openAppsPage: () => void; openLibraryPage: () => void; openFinancePage: () => void; openCalendarPage: () => void; openMeetingsPage: () => void; openAcademicPage: () => void; currentWorkspaceId: string; setCurrentWorkspaceId: (id: string) => void; currentWorkspace: Workspace | null; intent?: FunctionIntent; daily: DailyProps }) {
  const activeWorkspaces = workspaces.filter((workspace) => workspace.lifecycleState === "active");
  const [workspaceProjects, setWorkspaceProjects] = useState<Project[]>([]);
  const [workspaceApps, setWorkspaceApps] = useState<RegisteredApp[]>([]);
  // O contexto ativo e a persistencia dele moram no componente raiz desde que a
  // Library passou a filtrar por ele. Aqui so fica o que e da Home: buscar os
  // Projects e Apps daquele contexto.
  useEffect(() => {
    if (!currentWorkspaceId || !activeWorkspaces.some((workspace) => workspace.id === currentWorkspaceId)) {
      setWorkspaceProjects([]);
      setWorkspaceApps([]);
      return;
    }
    void Promise.all([api.workspaceProjects(currentWorkspaceId), api.workspaceApps(currentWorkspaceId)])
      .then(([nextProjects, nextApps]) => {
        setWorkspaceProjects(nextProjects);
        setWorkspaceApps(nextApps);
      })
      .catch(() => {
        setWorkspaceProjects([]);
        setWorkspaceApps([]);
      });
  }, [currentWorkspaceId, workspaces]);
  const scopedProjectIds = new Set(workspaceProjects.map((project) => project.id));
  const scopedProjects = currentWorkspace ? workspaceProjects : projects.filter((project) => project.lifecycleState === "active");
  const scopedApps = currentWorkspace ? workspaceApps : apps.filter((app) => app.lifecycleState === "active");
  const doing = tasks.filter((task) => task.state === "doing" && task.lifecycleState === "active" && (!currentWorkspace || (task.projectId && scopedProjectIds.has(task.projectId)))).slice(0, 5);
  /* A semana de Tasks: a mesma janela que o `WeekRings` desenha, calculada aqui
     só para a manchete e o rodapé. O widget continua dono do próprio cálculo —
     estes dois números existem porque o `<Widget>` não tem acesso ao que
     acontece dentro dele. */
  const taskWeek = useMemo(() => {
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    start.setDate(start.getDate() - ((start.getDay() + 6) % 7));

    const perDay = new Array(7).fill(0);
    for (const task of tasks) {
      if (!task.completedAt) continue;
      const at = new Date(task.completedAt);
      if (at < start) continue;
      const index = Math.floor((at.getTime() - start.getTime()) / 86_400_000);
      if (index >= 0 && index < 7) perDay[index] += 1;
    }
    return { done: perDay.reduce((sum, value) => sum + value, 0), peak: Math.max(...perDay) };
  }, [tasks]);
  /* O mes: a mesma agregacao que a densidade desenha, chamada aqui so para o
     total e o pico do rodape. A funcao e a MESMA — duplicar a conta em dois
     arquivos e como as duas versoes divergem. */
  const month = useMemo(() => {
    const values = [...monthActivity(tasks, recent).values()];
    return {
      records: values.reduce((sum, value) => sum + value, 0),
      peak: values.length ? Math.max(...values) : 0,
    };
  }, [tasks, recent]);
  const activeApps = scopedApps
    .filter((app) => app.lifecycleState === "active")
    .sort((left, right) => {
      const leftDate = left.lastOpenedAt ?? left.updatedAt;
      const rightDate = right.lastOpenedAt ?? right.updatedAt;
      return rightDate.localeCompare(leftDate);
    })
    .slice(0, 5);
  /** O atalho que o rótulo do tile promete.
   *
   *  Até aqui `app-shortcut` só era DESENHADO: a Home exibia ⌘1, ⌘2… e nenhum
   *  handler escutava. Rótulo que mente é pior que rótulo ausente — quem tenta
   *  uma vez e não funciona para de acreditar nos outros atalhos do app. E ⌘ é
   *  notação de macOS num aplicativo Windows.
   *
   *  Vive na Home, e não global, porque é aqui que os tiles estão: um atalho
   *  para a "quinta posição" de uma lista que você não está vendo não teria
   *  como ser previsto. */
  useEffect(() => {
    function handler(event: globalThis.KeyboardEvent) {
      if (!event.ctrlKey || event.altKey || event.shiftKey) return;
      const index = Number(event.key) - 1;
      if (!Number.isInteger(index) || index < 0 || index >= activeApps.length) return;
      event.preventDefault();
      openApp(activeApps[index]);
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [activeApps, openApp]);
  // Efemero: marca a Capture recem-criada para o savedWash e some. Nao e
  // estado de dominio, entao nao vale persistir nem subir para o App.
  const [savedIds, setSavedIds] = useState<Set<string>>(() => new Set());
  function markSaved(capture: Capture) {
    setSavedIds((current) => new Set(current).add(capture.id));
    window.setTimeout(() => setSavedIds((current) => { const next = new Set(current); next.delete(capture.id); return next; }), 900);
  }
  // Tres dias e o limiar do catalogo (IDEAS.md 155).
  //
  // Nao existe contagem verdadeira da Inbox no front: list_inbox pede 200
  // (src-tauri/src/lib.rs:84) e o proprio AppStatus.inboxCount e calculado com o
  // mesmo teto (lib.rs:685). Pior, a query ordena por captured_at DESC
  // (repository.rs:105), entao o corte descarta as capturas MAIS ANTIGAS — que sao
  // exatamente as que este widget existe para denunciar.
  //
  // Enquanto o core nao expuser um COUNT real, a saida honesta e admitir o teto:
  // no limite, mostrar "200+" em vez de "200" e "N+" em vez de "N".
  const staleInbox = inbox.filter((capture) => Date.now() - new Date(capture.capturedAt).getTime() > 3 * 24 * 60 * 60 * 1000).length;
  const inboxCapped = inbox.length >= INBOX_PAGE;
  const paradasDaHome = paradasVisiveis(stale.paradas);
  /* Task abre a gaveta; Project abre o Project. A parada e onde se NOTA, e o
     clique leva a onde se AGE — sem acao em massa aqui, porque uma lista que se
     resolve num clique convida a limpar sem decidir. */
  function abrirParada(parada: Parada) {
    if (parada.kind === "task") {
      const alvo = tasks.find((task) => task.id === parada.id);
      if (alvo) openTask(alvo);
      return;
    }
    const alvo = projects.find((project) => project.id === parada.id);
    if (alvo) openProject(alvo);
  }
  /* Cada contexto esconde os proprios widgets, "Todos" inclusive — ele nao e
     mais a visao sem escolha nenhuma (migration 0019). O banco guarda "Todos"
     como NULL e o seletor carrega string vazia; e o mesmo encontro de
     vocabularios que o `arrangeHome` faz do outro lado. */
  const hiddenIds = useMemo(() => new Set(hiddenWidgets.filter((entry) => (entry.workspaceId ?? "") === currentWorkspaceId).map((entry) => entry.widgetId)), [hiddenWidgets, currentWorkspaceId]);
  const allWidgetsHidden = HOME_WIDGETS.every((widget) => hiddenIds.has(widget.id));

  /* O arranjo desta Home, resolvido uma vez por render: faixa, largura e ordem
     de cada widget, ja com o que foi guardado por cima do desenho.

     Sem Workspace selecionado ("Todos") nao ha arranjo a aplicar, e a funcao
     devolve o desenho — pelo mesmo motivo que "Todos" nao oculta nada. */
  const arrangement = useMemo(() => arrangeHome(widgetPlacements, currentWorkspaceId), [widgetPlacements, currentWorkspaceId]);

  /* O modo de arrumar e local e nao persiste: ele descreve o que a pessoa esta
     fazendo agora, e nao uma preferencia.

     Trocar de contexto o desliga. Nao por falta de onde gravar — "Todos" tem o
     proprio arranjo desde a migration 0018 —, mas porque cada contexto tem a
     propria Home: continuar em modo de edicao depois de trocar seria oferecer
     controles sobre um arranjo que a pessoa nao veio arrumar. */
  const [arranging, setArranging] = useState(false);
  /* O registro de Functions manda `home.set_widget` para ca. Chegar na Home e
     metade do caminho: os controles so existem arrumando, entao o intent tem de
     abrir o modo, e nao apenas trocar de pagina. */
  useEffect(() => { if (intent?.target === "home_arrange") setArranging(true); }, [intent?.key, intent?.target]);
  const [layoutError, setLayoutError] = useState("");
  useEffect(() => { setArranging(false); setLayoutError(""); }, [currentWorkspaceId]);

  /* Grava as faixas que mudaram, e nao o movimento.

     Tres cuidados, e cada um veio de um jeito de errar:

     1. escreve as DUAS faixas quando o widget muda de casa. A de origem tambem
        mudou — quem ficou la subiu uma posicao — e gravar so o destino deixaria
        a origem com um buraco na numeracao;
     2. repassa `savedSpan`, e nao `span`. A escrita e autoritativa: mandar o
        span resolvido congelaria o desenho de hoje, e mandar `null` apagaria a
        largura que a pessoa escolheu. E o contrato que o teste
        `reordering_must_carry_the_stored_width_along` fixa no Rust;
     3. aplica na hora e reconcilia com o que o banco devolve. Esperar o
        round-trip para so entao mexer o widget faria cada clique de seta
        parecer engasgado, e a escrita e pequena o bastante para o otimismo
        valer. Se ela falhar, o estado anterior volta inteiro. */
  const commitLayout = useCallback((next: ArrangedWidget[], touched: string[]) => {
    const placements = placementsFor(next, touched);
    if (!placements.length) return;

    // O banco guarda "Todos" como NULL; o seletor carrega string vazia.
    const escopo = currentWorkspaceId || null;
    const anterior = widgetPlacements;
    const mexidos = new Set(placements.map((entry) => entry.widgetId));
    setWidgetPlacements([
      ...anterior.filter((entry) => (entry.workspaceId ?? "") !== currentWorkspaceId || !mexidos.has(entry.widgetId)),
      ...placements.map((entry) => ({ workspaceId: escopo, ...entry })),
    ]);
    void api.setWidgetLayout(escopo, placements)
      .then((gravado) => { setWidgetPlacements(gravado); setLayoutError(""); })
      /* A falha PRECISA aparecer. Sem isto o widget volta sozinho para onde
         estava e nada explica por que — o pior tipo de erro, o que a pessoa
         acha que foi ela que errou. */
      .catch((error) => { setWidgetPlacements(anterior); setLayoutError(appError(error).message); });
  }, [currentWorkspaceId, widgetPlacements, setWidgetPlacements]);

  const moveWidget = useCallback((widgetId: string, section: string, before: string | null) => {
    commitLayout(moveInArrangement(arrangement, widgetId, section, before), touchedSections(arrangement, widgetId, section));
  }, [arrangement, commitLayout]);

  /* Redimensiona. `null` devolve a largura do desenho, e e o unico jeito de
     desfazer um redimensionamento sem apagar o arranjo inteiro. */
  const resizeWidget = useCallback((widgetId: string, span: HomeWidgetSpan | null) => {
    const atual = arrangement.find((slot) => slot.id === widgetId);
    if (!atual) return;
    commitLayout(arrangement.map((slot) => slot.id === widgetId ? { ...slot, savedSpan: span } : slot), [atual.section]);
  }, [arrangement, commitLayout]);

  /* Esconde ou traz de volta, no contexto atual.

     Otimista como o arranjo, e pelo mesmo motivo: esperar o round-trip faria o
     clique parecer engasgado. Diferente do arranjo, aqui o backend nao devolve
     o estado novo — entao a fonte da verdade depois da escrita e o `refresh()`,
     e o otimismo existe so para cobrir o intervalo. */
  const setWidgetHidden = useCallback((widgetId: string, hidden: boolean) => {
    const anterior = hiddenWidgets;
    const escopo = currentWorkspaceId || null;
    setHiddenWidgets(hidden
      ? [...anterior, { workspaceId: escopo, widgetId }]
      : anterior.filter((entry) => (entry.workspaceId ?? "") !== currentWorkspaceId || entry.widgetId !== widgetId));
    void api.setWorkspaceWidget(widgetId, escopo, !hidden)
      .then(() => { setLayoutError(""); return refresh(); })
      .catch((error) => { setHiddenWidgets(anterior); setLayoutError(appError(error).message); });
  }, [currentWorkspaceId, hiddenWidgets, setHiddenWidgets, refresh]);

  /* Devolve a Home ao desenho apagando as linhas. Apagar e diferente de gravar
     o catalogo por cima: gravar petrificaria o desenho de HOJE, e um widget que
     mudasse de largura depois nunca mais alcancaria este Workspace.

     Em duas etapas, e nao num clique so. Nao ha Desfazer aqui, e o que se perde
     e o trabalho de posicionar quinze widgets — refazer isso e caro o bastante
     para um clique acidental doer. Duas etapas em vez de um `<dialog>` porque a
     acao e reversivel pelas maos da pessoa, so trabalhosa: o peso da confirmacao
     acompanha o peso do estrago. */
  const [confirmingRestore, setConfirmingRestore] = useState(false);
  useEffect(() => { if (!arranging) setConfirmingRestore(false); }, [arranging]);
  const restoreLayout = useCallback(() => {
    setConfirmingRestore(false);
    void api.resetWidgetLayout(currentWorkspaceId || null)
      .then((gravado) => { setWidgetPlacements(gravado); setLayoutError(""); })
      .catch((error) => setLayoutError(appError(error).message));
  }, [currentWorkspaceId, setWidgetPlacements]);

  // O tempo carrega por fora do `refresh()`: aquele é o caminho de boot do app
  // inteiro, e um erro no rastreio não pode ser motivo para a Home não abrir.
  const trackedTime = useTrackedTime();
  const weekTime = useMemo(() => weekSummary(trackedTime, projects), [trackedTime, projects]);
  const hasBudget = trackedTime.tracking.some((entry) => entry.budgetMinutes > 0);
  // resources(true) traz arquivado junto — a Home so mostra o acervo vivo. A ordem
  // ja vem do banco por updated_at DESC (resource_repository.rs:185).
  const allActiveResources = resources.filter((resource) => resource.lifecycleState === "active");
  // Mesma regra dos vizinhos: com contexto ativo, so o que pertence a ele.
  const scopedResourceIds = new Set(currentWorkspace ? resourceWorkspaces.filter((link) => link.workspaceId === currentWorkspace.id).map((link) => link.resourceId) : []);
  const activeResources = currentWorkspace ? allActiveResources.filter((resource) => scopedResourceIds.has(resource.id)) : allActiveResources;
  const projectName = (id: string | null) => projects.find((project) => project.id === id)?.name;
  /* A atividade vem das Tasks do Project, e nao do `updatedAt` dele.
     Aquele campo so muda quando o Project e EDITADO: criar Task, mover no
     Kanban e concluir nao o tocam. O ponto acendia ao RENOMEAR. */
  const atividade = atividadePorProject(stale.activity);
  const parados = projectsParados(stale.paradas);
  const atividadeDe = (project: Project) => atividade.get(project.id) ?? project.updatedAt;
  const isActiveToday = (project: Project) => mexidoHoje(atividadeDe(project));
  return <div className="page home-page">
    <DotField />
    <ContextPath segments={["M", "HOME"]} />
    <CaptureComposer onSaved={(capture) => { markSaved(capture); void refresh(); }} focusKey={intent?.target === "home_capture" ? intent.key : undefined} />
    <section className="home-context" aria-labelledby="home-context-heading">
      <span className="micro-label" id="home-context-heading">CONTEXTO ATUAL</span>
      <div className="context-switcher" role="group" aria-label="Selecionar contexto da Home">
        <button type="button" aria-pressed={!currentWorkspace} data-selected={!currentWorkspace || undefined} onClick={() => setCurrentWorkspaceId("")}><strong>Todos</strong></button>
        {activeWorkspaces.map((workspace) => <button key={workspace.id} type="button" aria-pressed={workspace.id === currentWorkspaceId} data-selected={workspace.id === currentWorkspaceId || undefined} title={`Selecionar ${workspace.name}; clique duplo para abrir`} onClick={() => setCurrentWorkspaceId(workspace.id)} onDoubleClick={() => openWorkspace(workspace)}><strong>{workspace.name}</strong></button>)}
      </div>
      {/* Cada contexto arruma a propria Home, "Todos" inclusive. Ele nao e um
          estado degradado a espera de um Workspace: para quem nunca criou
          nenhum — e da para usar o M/OS inteiro assim — "Todos" e A Home, e
          deixar o botao desligado ali tirava a feature do alcance dessa pessoa
          sem dizer por que. Ver a migration 0018. */}
      <div className="home-arrange">
        {arranging ? <Button variant={confirmingRestore ? "danger" : "ghost"} size="sm" title={confirmingRestore ? "Apaga o arranjo deste Workspace. Não há Desfazer." : undefined} onClick={() => { if (confirmingRestore) restoreLayout(); else setConfirmingRestore(true); }}>{confirmingRestore ? "Confirmar: apagar o arranjo" : "Restaurar o desenho"}</Button> : null}
        <Button variant={arranging ? "primary" : "outline"} size="sm" title={currentWorkspace ? `Arrumar a Home de ${currentWorkspace.name}.` : "Arrumar a Home de Todos. Cada contexto guarda o próprio arranjo."} onClick={() => setArranging((ligado) => !ligado)}>{arranging ? "Concluir" : "Arrumar"}</Button>
      </div>
    </section>

    {arranging ? <p className="home-arrange-hint" role="status">{layoutError ? `Não deu para gravar: ${layoutError}` : "Arraste pelo punho, ou use as setas: ← → dentro da faixa, ↑ ↓ entre faixas. Os números 1, 2 e 4 são o tamanho."}</p> : null}

    {/* A ordem em que os widgets aparecem AQUI nao decide mais nada: quem decide
        e o catalogo, em `HOME_WIDGETS`, junto com o que a pessoa arrumou. Esta
        lista so diz o que cada widget desenha. */}
    <HomeBoard
      arrangement={arrangement}
      arranging={arranging}
      hiddenIds={hiddenIds}
      onMove={moveWidget}
      onResize={resizeWidget}
      onHide={setWidgetHidden}
      widgets={[
        /* O dia abre a Home. Ele NAO e um card fixo acima do quadro: tudo que
           mora na Home do M/OS e um widget arrumavel, e uma excecao seria a
           unica coisa da tela que nao se pode mover nem esconder. */
        { id: "daily_session", node: <Panel label="HOJE"><DailyFocusWidget dia={daily.dia} contexto={daily.contexto} carregando={daily.carregando} erro={daily.erro} iniciar={daily.iniciar} abrirSessao={daily.abrirSessao} encerrarAntigo={daily.encerrarAntigo} concluirObjetivo={daily.concluirObjetivo} abrirVinculo={daily.abrirVinculo} semanaPendente={daily.semanaPendente} abrirSemana={daily.abrirSemana} /></Panel> },
        { id: "academic", ...(academic?.overdue ? { footLeft: "ATRASADO", footRight: String(academic.overdue) } : {}), node: <Panel label="FACULDADE" value={academic?.semester ? String(academic.upcoming.length) : "—"} unit={academic?.semester ? (academic.upcoming.length === 1 ? "compromisso" : "compromissos") : "sem semestre"} action={academic?.semester ? <Button variant="ghost" onClick={() => openAcademicPage()}>Abrir</Button> : undefined}><AcademicWidget dashboard={academic} abrir={() => openAcademicPage()} /></Panel> },
        { id: "now", node: <Panel label="EM ANDAMENTO" value={String(doing.length)} unit="em andamento" count={doing.length ? String(doing.length) : undefined}>{doing.length ? doing.map((task) => <DataRow key={task.id} primary={task.title} meta={projectName(task.projectId)} onClick={() => openTask(task)} />) : <EmptyState>Nada em andamento. Uma Task movida para Doing aparece aqui.</EmptyState>}</Panel> },
        { id: "timer", node: <Panel label="CRONÔMETRO"><Timer projects={projects} onChanged={() => void refresh()} /></Panel> },
        { id: "today_hours", footLeft: "7 DIAS · CONTRA O PICO", footRight: `PICO ${hoursLabel(weekTime.peakSeconds)}`, node: <Panel label="HORAS HOJE"><TodayHours time={trackedTime} /></Panel> },
        /* O numero cru vira anel. A proporcao mostrada e o que esta ENVELHECENDO
           dentro da Inbox, nao o tamanho dela: uma Inbox grande e processada hoje e
           saudavel, e uma pequena parada ha uma semana nao e. O anel vazio com o
           numero no centro le exatamente como "nada envelhecendo", que e o estado
           bom — e e por isso que zero nao desenha ponto de sodio.

           O rodape e tudo-ou-nada pelo mesmo motivo. "ENVELHECENDO" e a legenda
           de um numero, e nao uma frase: com a Inbox vazia sobrava a regua e a
           palavra sozinha, sem nada a direita — meia frase pendurada no pe do
           card. Sem numero a dizer, o rodape inteiro sai. */
        { id: "inbox_pulse", ...(inbox.length ? { footLeft: "ENVELHECENDO", footRight: `${staleInbox} DE ${inbox.length}${inboxCapped ? "+" : ""}` } : {}), node: <Panel label="INBOX"><button type="button" className="pulse" onClick={() => openInbox()}><Ring size={88} segments={[{ value: inbox.length ? staleInbox / inbox.length : 0 }]}><RingLabel value={inboxCapped ? `${INBOX_PAGE}+` : String(inbox.length)} /></Ring><small>{inbox.length === 1 ? "capture por processar" : "captures por processar"}</small>{staleInbox ? <small className="pulse-stale">{staleInbox === 1 && !inboxCapped ? "1 com mais de 3 dias" : `${staleInbox}${inboxCapped ? "+" : ""} com mais de 3 dias`}</small> : null}</button></Panel> },
        { id: "stale", ...(paradasDaHome.restantes ? { footLeft: "MOSTRANDO 5", footRight: `E MAIS ${paradasDaHome.restantes}` } : {}), node: <Panel label="PARADAS" value={String(stale.paradas.length)} unit={stale.paradas.length === 1 ? "parada" : "paradas"}>{paradasDaHome.visiveis.map((parada) => <DataRow key={`${parada.kind}-${parada.id}`} primary={parada.title} secondary={parada.context || undefined} meta={rotuloDeDias(parada.days)} onClick={() => abrirParada(parada)} />)}{/* Vazio nao e falha, e o texto diz isso: "nada parado" e um bom resultado, e um estado vazio de erro faria o widget parecer quebrado no dia em que tudo esta em dia. */}{!stale.paradas.length ? <p className="empty-state">Nada parado.</p> : null}</Panel> },
        { id: "recent", node: <Panel label="RECENTES" value={String(recent.length)} unit={recent.length === 1 ? "captura" : "capturas"}>{recent.length ? recent.map((capture) => <DataRow key={capture.id} primary={capture.content} meta={relativeTime(capture.capturedAt)} saved={savedIds.has(capture.id)} onClick={() => openCapture(capture)} />) : <EmptyState>Nada capturado ainda. O que você escrever no campo acima aparece aqui.</EmptyState>}</Panel> },
        { id: "projects", node: <Panel label="PROJECTS" value={String(scopedProjects.length)} unit="ativos" action={scopedProjects.length > 5 ? <Button variant="ghost" onClick={() => openProjectsPage()}>Ver todos</Button> : undefined}>{scopedProjects.slice(0, 5).map((project) => <DataRow key={project.id} primary={project.name} marker={<span className="project-dot" data-active={isActiveToday(project) || undefined} data-stale={parados.has(project.id) || undefined} aria-hidden="true" />} meta={relativeTime(atividadeDe(project))} onClick={() => openProject(project)} />)}{!scopedProjects.length ? <ScopedEmptyState total={projects.filter((project) => project.lifecycleState === "active").length} workspace={currentWorkspace} noun="project" onLink={() => { if (currentWorkspace) openWorkspace(currentWorkspace); }} /> : null}</Panel> },
        { id: "month_density", footLeft: "MÊS CORRENTE · 4 DEGRAUS", footRight: `PICO ${month.peak}`, node: <Panel label="MÊS" value={String(month.records)} unit="registros"><MonthDensity tasks={tasks} captures={recent} /></Panel> },
        { id: "week_rings", footLeft: "SEG–DOM · CONTRA O PICO", footRight: `PICO ${taskWeek.peak}`, node: <Panel label="TASKS NA SEMANA" value={String(taskWeek.done)} unit="concluídas"><WeekRings tasks={tasks} onOpen={openTasksPage} /></Panel> },
        { id: "week_by_project", footLeft: `${weekTime.projectCount} PROJECTS · 7 DIAS`, footRight: weekTime.topProject ? `MAIOR: ${weekTime.topProject}` : undefined, node: <Panel label="HORAS POR PROJECT" value={hoursLabel(weekTime.seconds)} unit="na semana"><WeekByProject time={trackedTime} projects={projects} onOpen={openTempoPage} /></Panel> },
        { id: "task_progress", node: <Panel label="CONCLUÍDO"><TaskProgressRing tasks={tasks} /></Panel> },
        // Indisponivel quando nenhum Project tem meta: um anel preenchido contra
        // um alvo que ninguem definiu ensinaria a confiar numa medida inexistente.
        { id: "budget_ring", available: hasBudget, footLeft: "CONTRA A META", node: <Panel label="META"><BudgetRing time={trackedTime} projects={projects} onOpen={openProject} /></Panel> },
        { id: "recent_resources", node: <Panel label="RECURSOS" value={String(activeResources.length)} unit={activeResources.length === 1 ? "recurso" : "recursos"} action={activeResources.length > 5 ? <Button variant="ghost" onClick={() => openLibraryPage()}>Ver todos</Button> : undefined}>{activeResources.length ? activeResources.slice(0, 5).map((resource) => <DataRow key={resource.id} primary={resource.title} secondary={resourceHost(resource.url)} meta={relativeTime(resource.updatedAt)} onClick={() => openResource(resource)} />) : <ScopedEmptyState total={allActiveResources.length} workspace={currentWorkspace} noun="resource" onLink={() => openLibraryPage()} linkLabel="Ver tudo" />}</Panel> },
        // O nome do app nao entra: o icone com a inicial e o atalho ja o
        // identificam, e a linha de nomes competiria com as rows ao lado. O botao
        // "Gerenciar" e a porta de Apps desde que ele saiu do rail (ADR-038) — sem
        // ele, com zero apps cadastrados a busca do Command nao acha nada e a
        // pagina fica inalcancavel para criar o primeiro.
        { id: "apps", node: <Panel label="APPS" value={String(activeApps.length)} unit={activeApps.length === 1 ? "app" : "apps"} action={<Button variant="ghost" onClick={() => openAppsPage()}>Gerenciar</Button>}><div className="app-row">{activeApps.map((app, index) => <button key={app.id} type="button" className="app-tile" onClick={() => openApp(app)} title={app.name} aria-label={app.name}><AppIcon app={app} />{index < 9 ? <span className="app-shortcut">Ctrl {index + 1}</span> : null}</button>)}</div>{!activeApps.length ? <ScopedEmptyState total={apps.filter((app) => app.lifecycleState === "active").length} workspace={currentWorkspace} noun="app" onLink={() => { if (currentWorkspace) openWorkspace(currentWorkspace); }} /> : null}</Panel> },
        { id: "quick_actions", node: <Panel label="AÇÕES"><div className="quick-actions"><Button variant="outline" size="sm" onClick={() => void api.showQuickCapture()}>Capturar</Button><Button variant="outline" size="sm" onClick={() => openTasksPage()}>Nova Task</Button><Button variant="outline" size="sm" onClick={() => openProjectsPage()}>Novo Project</Button>{/* As tres portas dos destinos que sairam do rail (ADR-045). Entram JUNTO
            com a saida, e nao depois: a ADR-038 registrou que tirar Apps do rail
            sem porta nova deixaria "a pagina inalcancavel", e o leque sozinho
            nao basta — uma petala pode ser trocada por outra coisa.

            Ficam aqui, e nao em widgets proprios, porque ACOES ja e o lugar de
            "ir fazer uma coisa" e nenhum dos tres tem widget na Home: criar tres
            widgets para tres botoes seria caro demais para a divida que a
            ADR-038 pagou com um botao so. */}<Button variant="outline" size="sm" onClick={() => openCalendarPage()}>Calendário</Button><Button variant="outline" size="sm" onClick={() => openFinancePage()}>Finance</Button><Button variant="outline" size="sm" onClick={() => openMeetingsPage()}>Reuniões</Button></div></Panel> },
        // SISTEMA nao duplica INTEGRIDADE das Settings — aquele e diagnostico
        // (schema, WAL), este responde "esta salvo?".
        { id: "system_health", node: <Panel label="SISTEMA"><SystemHealth status={status} /></Panel> },
      ]}
    />
    {/* Ocultar todos e escolha legitima. O que nao pode e a Home virar um branco
        sem explicacao — quem escondeu tudo precisa do caminho de volta, e agora
        ele aponta para o modo de arrumar, que e de onde se esconde e onde o
        oculto continua visivel. Antes apontava para o inspetor de Workspace, o
        que era uma porta fechada para quem nao tem Workspace nenhum.

        `!arranging` porque arrumando eles estao todos na tela, apagados: a
        frase seria desmentida pelo que esta logo abaixo dela. */}
    {allWidgetsHidden && !arranging ? <div className="scoped-empty"><EmptyState>Todos os widgets estão ocultos neste contexto.</EmptyState><Button variant="outline" size="sm" onClick={() => setArranging(true)}>Arrumar</Button></div> : null}
  </div>;
}

function CaptureTaskForm({ capture, projects, onCreated, cancel }: { capture: Capture; projects: Project[]; onCreated: (task: Task) => void; cancel: () => void }) {
  const [title, setTitle] = useState(capture.content);
  const [description, setDescription] = useState("");
  const [projectId, setProjectId] = useState("");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  async function submit(event: FormEvent) {
    event.preventDefault();
    setSaving(true);
    try {
      const task = await api.createTask(title, description, projectId || null, capture.id);
      onCreated(task);
    } catch (nextError) {
      setError(appError(nextError).message);
      setSaving(false);
    }
  }
  return <form className="stack-form" onSubmit={submit} aria-busy={saving}>
    <label><span>TÍTULO</span><input value={title} onChange={(event) => setTitle(event.currentTarget.value)} autoFocus /></label>
    <label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={3} /></label>
    <label><span>PROJECT</span><select value={projectId} onChange={(event) => setProjectId(event.currentTarget.value)}><option value="">Sem Project</option>{projects.filter((project) => project.lifecycleState === "active").map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}</select></label>
    {saving ? <StateMessage state="saving" label="Salvando Task..." /> : error ? <StateMessage state="error" label={error} /> : null}
    <div className="form-actions"><Button variant="ghost" onClick={cancel}>Cancelar</Button><Button variant="primary" type="submit" disabled={!title.trim() || saving}>{saving ? "Salvando" : "Criar Task"}</Button></div>
  </form>;
}

function InboxPage({ captures, projects, refresh, receipt, openTask, openResource, intent }: { captures: Capture[]; projects: Project[]; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; openTask: (task: Task) => void; openResource: (resource: Resource) => void; intent?: FunctionIntent }) {
  const [selectedId, setSelectedId] = useState(captures[0]?.id ?? "");
  const [narrowPane, setNarrowPane] = useState<"list" | "detail">("list");
  const [taskForm, setTaskForm] = useState(false);
  const [resourceForm, setResourceForm] = useState(false);
  const [error, setError] = useState("");
  const listPane = useRef<HTMLElement>(null);
  const inspector = useRef<HTMLElement>(null);
  const detailActions = useRef<HTMLDivElement>(null);
  useEffect(() => { if (!captures.some((capture) => capture.id === selectedId)) setSelectedId(captures[0]?.id ?? ""); }, [captures, selectedId]);
  const selected = captures.find((capture) => capture.id === selectedId) ?? null;
  useEffect(() => {
    if (!intent || !selected) return;
    if (intent.target === "inbox_create_task") {
      setTaskForm(true);
      setResourceForm(false);
      return;
    }
    if (intent.target === "inbox_create_resource") {
      setTaskForm(false);
      setResourceForm(true);
      return;
    }
    if (intent.target === "inbox_process") {
      setTaskForm(false);
      window.requestAnimationFrame(() => detailActions.current?.querySelector<HTMLButtonElement>("[data-function-action='capture.mark_processed']")?.focus());
    }
  }, [intent?.key, selected?.id]);

  async function mutate(action: "processed" | "archive" | "trash") {
    if (!selected) return;
    try {
      if (action === "processed") {
        await api.markProcessed(selected.id);
        receipt({ message: "Capture marcada como processada.", run: () => api.moveToInbox(selected.id) });
      } else if (action === "archive") {
        await api.archive(selected.id);
        receipt({ message: "Capture arquivada.", run: () => api.restore(selected.id) });
      } else {
        await api.trash(selected.id);
        receipt({ message: "Capture movida para a Lixeira.", run: () => api.restore(selected.id) });
      }
      setError("");
      await refresh();
    } catch (nextError) { setError(appError(nextError).message); }
  }

  function selectCapture(capture: Capture) {
    setSelectedId(capture.id);
    setTaskForm(false);
    setResourceForm(false);
    setNarrowPane("detail");
    if (window.matchMedia("(max-width: 960px)").matches) requestAnimationFrame(() => inspector.current?.focus());
  }

  function returnToInboxList() {
    setTaskForm(false);
    setResourceForm(false);
    setNarrowPane("list");
    requestAnimationFrame(() => listPane.current?.querySelector<HTMLButtonElement>(".data-row[data-selected]")?.focus());
  }

  if (!captures.length) return <div className="page inbox-empty-page">
    <ContextPath segments={["M", "INBOX"]} />
    {/* Antes do vazio, e nao depois: uma Inbox que se diz limpa enquanto ha
        audio esperando estaria mentindo sobre a unica coisa que ela promete. */}
    <PendingVoice />
    <section className="inbox-empty-view" aria-labelledby="inbox-empty-title">
      <span className="micro-label">0 ITENS</span>
      <h1 id="inbox-empty-title">Inbox limpa.</h1>
      <p>Novas captures aparecem aqui até você decidir o que fazer com elas.</p>
      <Button variant="primary" onClick={() => void api.showQuickCapture()}>Capturar</Button>
    </section>
  </div>;
  return <div className="split-page inspector-page inbox-page" data-pane={narrowPane}>
    <section ref={listPane} className="list-pane" aria-label="Captures na Inbox">
      <PaneHeader
        segments={["M", "INBOX"]}
        meta={`${captures.length} ${captures.length === 1 ? "ITEM" : "ITENS"}`}
        actions={<Button variant="ghost" size="sm" onClick={() => void api.showQuickCapture()}>Capturar</Button>}
      />
      <PendingVoice />
      <AnimatedList className="row-list">{captures.map((capture) => <AnimatedListItem key={capture.id} itemKey={capture.id}><DataRow
        primary={capture.content}
        secondary={sourceLabel(capture.source)}
        secondaryKind="system"
        meta={relativeTime(capture.capturedAt)}
        selected={capture.id === selectedId}
        onClick={() => selectCapture(capture)}
        onKeyDown={(event) => {
          const nextIndex = moveListFocus(event);
          if (nextIndex === null) return;
          const nextCapture = captures[nextIndex];
          if (!nextCapture) return;
          setSelectedId(nextCapture.id);
          setTaskForm(false);
          setResourceForm(false);
        }}
      /></AnimatedListItem>)}</AnimatedList>
    </section>
    {selected ? <Inspector ref={inspector} label="Detalhe da Capture" open={narrowPane === "detail"} onBack={returnToInboxList} onEscape={returnToInboxList}><header className="detail-header"><div><span className="micro-label">CAPTURE</span><h1>{selected.content}</h1><div className="chip-line"><span className="chip">{sourceLabel(selected.source)}</span><span className="chip">{relativeTime(selected.capturedAt)}</span></div></div><ActionMenu trigger={<Icon name="more" />} items={[{ label: "Arquivar", onSelect: () => void mutate("archive") }, { label: "Mover para a Lixeira", danger: true, onSelect: () => void mutate("trash") }]} /></header>
      {error ? <p className="inline-error" role="alert">! {error}</p> : null}
      {/* Moldura pronta, conteudo honesto. A interpretacao do Hermes e a fase 3
          da integracao; ate la este bloco diz o que e, em vez de fabricar uma
          interpretacao falsa para a tela parecer completa. */}
      <section className="hermes-block" aria-label="Interpretação do Hermes">
        <span className="micro-label">HERMES</span>
        <p className="hermes-empty">Interpretação automática ainda não está ligada. Classifique manualmente abaixo — nada se perde.</p>
      </section>
      {taskForm ? <CaptureTaskForm capture={selected} projects={projects} cancel={() => setTaskForm(false)} onCreated={(task) => { setTaskForm(false); void refresh(); openTask(task); }} /> : resourceForm ? <ResourceForm capture={selected} cancel={() => setResourceForm(false)} saved={(resource) => { setResourceForm(false); void refresh(); openResource(resource); }} /> : <div ref={detailActions} className="detail-actions"><Button variant="primary" onClick={() => { setTaskForm(true); setResourceForm(false); }}>Criar Task</Button><Button variant="secondary" onClick={() => { setTaskForm(false); setResourceForm(true); }}>Salvar Resource</Button><Button variant="secondary" data-function-action="capture.mark_processed" onClick={() => void mutate("processed")}>Marcar processada</Button></div>}
    </Inspector> : null}
  </div>;
}

function ProjectForm({ project, cancel, saved }: { project?: Project; cancel: () => void; saved: (project: Project) => void }) {
  const [name, setName] = useState(project?.name ?? "");
  const [description, setDescription] = useState(project?.description ?? "");
  const [repository, setRepository] = useState(project?.repository ?? "");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (saving) return;
    setSaving(true);
    setError("");
    try { saved(project ? await api.updateProject(project.id, name, description, repository) : await api.createProject(name, description, repository)); }
    catch (nextError) { setError(appError(nextError).message); setSaving(false); }
  }
  return <form className="stack-form" onSubmit={submit} aria-busy={saving}>
    <label><span>NOME</span><input value={name} onChange={(event) => setName(event.currentTarget.value)} autoFocus /></label>
    <label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={4} /></label>
    <label><span>REPOSITÓRIO</span><input className="mono-input" value={repository} onChange={(event) => setRepository(event.currentTarget.value)} placeholder="usuario/repo ou URL" /></label>
    {saving ? <StateMessage state="saving" label="Salvando Project..." /> : error ? <StateMessage state="error" label={error} /> : null}
    <div className="form-actions"><Button variant="ghost" onClick={cancel} disabled={saving}>Cancelar</Button><Button variant="primary" type="submit" disabled={!name.trim() || saving}>{saving ? "Salvando" : "Salvar"}</Button></div>
  </form>;
}

function DirectTaskForm({ projectId = null, projects, cancel, saved }: { projectId?: string | null; projects: Project[]; cancel: () => void; saved: (task: Task) => void }) {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [selectedProject, setSelectedProject] = useState(projectId ?? "");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (saving) return;
    setSaving(true);
    setError("");
    try { saved(await api.createTask(title, description, selectedProject || null)); }
    catch (nextError) { setError(appError(nextError).message); setSaving(false); }
  }
  return <form className="stack-form compact-form" onSubmit={submit} aria-busy={saving}>
    <label><span>TÍTULO</span><input value={title} onChange={(event) => setTitle(event.currentTarget.value)} autoFocus /></label>
    <label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={2} /></label>
    <label><span>PROJECT</span><select value={selectedProject} onChange={(event) => setSelectedProject(event.currentTarget.value)}><option value="">Sem Project</option>{projects.filter((project) => project.lifecycleState === "active").map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}</select></label>
    {saving ? <StateMessage state="saving" label="Salvando Task..." /> : error ? <StateMessage state="error" label={error} /> : null}
    <div className="form-actions"><Button variant="ghost" onClick={cancel} disabled={saving}>Cancelar</Button><Button variant="primary" type="submit" disabled={!title.trim() || saving}>{saving ? "Salvando" : "Criar Task"}</Button></div>
  </form>;
}

function ProjectsPage({ projects, tasks, initialProjectId, refresh, receipt, openTask, intent }: { projects: Project[]; tasks: Task[]; initialProjectId: string; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; openTask: (task: Task) => void; intent?: FunctionIntent }) {
  const activeProjects = projects.filter((project) => project.lifecycleState === "active");
  const [selectedId, setSelectedId] = useState(initialProjectId || activeProjects[0]?.id || "");
  const [mode, setMode] = useState<"view" | "edit" | "new" | "task">("view");
  const [narrowPane, setNarrowPane] = useState<"list" | "detail">(initialProjectId || intent?.target === "projects_create" ? "detail" : "list");
  const [pendingAction, setPendingAction] = useState<"archive" | null>(null);
  const [error, setError] = useState("");
  const listPane = useRef<HTMLElement>(null);
  const inspector = useRef<HTMLElement>(null);

  useEffect(() => {
    if (!initialProjectId) return;
    setSelectedId(initialProjectId);
    setMode("view");
    setNarrowPane("detail");
  }, [initialProjectId]);

  useEffect(() => {
    if (intent?.target !== "projects_create") return;
    setMode("new");
    setNarrowPane("detail");
    setError("");
  }, [intent?.key]);

  useEffect(() => {
    if (mode === "new") return;
    if (!activeProjects.some((project) => project.id === selectedId)) {
      setSelectedId(activeProjects[0]?.id ?? "");
      if (!activeProjects.length) setNarrowPane("list");
    }
  }, [activeProjects, selectedId, mode]);

  const selected = activeProjects.find((project) => project.id === selectedId) ?? null;
  const relatedTasks = tasks.filter((task) => task.projectId === selectedId && task.lifecycleState === "active");
  const projectsEmpty = !activeProjects.length && mode !== "new";

  function projectProgress(projectId: string) {
    const owned = tasks.filter((task) => task.projectId === projectId && task.lifecycleState === "active");
    return { done: owned.filter((task) => task.state === "done").length, total: owned.length };
  }

  function selectProject(project: Project) {
    setSelectedId(project.id);
    setMode("view");
    setError("");
    setNarrowPane("detail");
    if (window.matchMedia("(max-width: 960px)").matches) requestAnimationFrame(() => inspector.current?.focus());
  }

  function startNew() {
    setMode("new");
    setError("");
    setNarrowPane("detail");
    if (window.matchMedia("(max-width: 960px)").matches) requestAnimationFrame(() => inspector.current?.focus());
  }

  function returnToList() {
    setMode("view");
    setError("");
    setNarrowPane("list");
    requestAnimationFrame(() => {
      const selectedRow = listPane.current?.querySelector<HTMLButtonElement>(".data-row[data-selected]");
      const createAction = listPane.current?.querySelector<HTMLButtonElement>(".pane-heading-meta .button");
      (selectedRow ?? createAction)?.focus();
    });
  }

  async function archiveProject(project: Project) {
    setPendingAction("archive");
    setError("");
    try {
      await api.setProjectArchived(project.id, true);
      receipt({ message: "Project arquivado.", run: () => api.setProjectArchived(project.id, false) });
      setSelectedId(activeProjects.find((candidate) => candidate.id !== project.id)?.id ?? "");
      setMode("view");
      setNarrowPane("list");
      await refresh();
    } catch (nextError) {
      setError(appError(nextError).message);
    } finally {
      setPendingAction(null);
    }
  }

  if (projectsEmpty) {
    return <div className="page projects-empty-page">
      <ContextPath segments={["M", "PROJECTS"]} />
      <section className="projects-empty-view" aria-labelledby="projects-empty-title">
        <span className="micro-label">0 PROJECTS</span>
        <h1 id="projects-empty-title">Nenhum Project ainda.</h1>
        <p>Reúna Tasks e contexto de um trabalho num único lugar.</p>
        <Button variant="primary" onClick={startNew}>Novo Project</Button>
      </section>
    </div>;
  }

  return <div className="split-page inspector-page projects-page" data-pane={narrowPane}>
    <section ref={listPane} className="list-pane" aria-label="Projects ativos">
      <PaneHeader
        segments={["M", "PROJECTS"]}
        meta={`${activeProjects.length} ${activeProjects.length === 1 ? "ATIVO" : "ATIVOS"}`}
        actions={<Button variant="ghost" size="sm" onClick={startNew}>Novo Project</Button>}
      />
      <div className="row-list">{activeProjects.map((project) => <DataRow
        key={project.id}
        primary={project.name}
        secondary={project.description || undefined}
        progress={projectProgress(project.id)}
        selected={project.id === selectedId && mode !== "new"}
        onClick={() => selectProject(project)}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            selectProject(project);
            return;
          }
          const nextIndex = moveListFocus(event);
          if (nextIndex === null) return;
          const nextProject = activeProjects[nextIndex];
          if (!nextProject) return;
          setSelectedId(nextProject.id);
          setMode("view");
          setError("");
        }}
      />)}</div>
    </section>
    <Inspector
      ref={inspector}
      label="Detalhe do Project"
      open={narrowPane === "detail"}
      onBack={returnToList}
      onEscape={mode === "view" || mode === "new" ? returnToList : undefined}
    >
      {mode === "new" ? <>
        <span className="micro-label">NOVO PROJECT</span>
        <ProjectForm
          cancel={() => {
            if (selected) {
              setMode("view");
              setNarrowPane("detail");
              requestAnimationFrame(() => inspector.current?.focus());
            } else {
              returnToList();
            }
          }}
          saved={(project) => {
            setSelectedId(project.id);
            setMode("view");
            setNarrowPane("detail");
            void refresh().then(() => requestAnimationFrame(() => inspector.current?.focus()));
          }}
        />
      </> : selected ? mode === "edit" ? <>
        <span className="micro-label">EDITAR PROJECT</span>
        <ProjectForm
          project={selected}
          cancel={() => { setMode("view"); requestAnimationFrame(() => inspector.current?.focus()); }}
          saved={() => { setMode("view"); void refresh().then(() => requestAnimationFrame(() => inspector.current?.focus())); }}
        />
      </> : <>
        <header className="detail-header">
          <div>
            <span className="micro-label">PROJECT</span>
            <h1>{selected.name}</h1>
            <p>{selected.description || "Sem descrição."}</p>
          </div>
          <ActionMenu
            trigger={<Icon name="more" />}
            items={[
              { label: "Editar", disabled: pendingAction !== null, onSelect: () => setMode("edit") },
              { label: pendingAction === "archive" ? "Arquivando" : "Arquivar", danger: true, disabled: pendingAction !== null, onSelect: () => void archiveProject(selected) },
            ]}
          />
        </header>
        {error ? <p className="inline-error" role="alert">! {error}</p> : null}
        <dl className="fact-grid">
          <div>
            <dt>REPOSITÓRIO</dt>
            <dd className="mono-value">{selected.repository || <span className="fact-empty">Nenhum associado</span>}</dd>
          </div>
          <div>
            <dt>ATUALIZADO</dt>
            <dd>{relativeTime(selected.updatedAt)}</dd>
          </div>
        </dl>
        {mode === "task" ? <DirectTaskForm
          projectId={selected.id}
          projects={projects}
          cancel={() => { setMode("view"); requestAnimationFrame(() => inspector.current?.focus()); }}
          saved={(task) => {
            setMode("view");
            void refresh();
            openTask(task);
          }}
        /> : <Panel label="TASKS" action={<Button variant="primary" onClick={() => setMode("task")} disabled={pendingAction !== null}>Criar Task</Button>}>
          {relatedTasks.length
            ? relatedTasks.map((task) => <DataRow key={task.id} primary={task.title} meta={stateLabels[task.state]} completed={task.state === "done"} onClick={() => openTask(task)} />)
            : <EmptyState>Nenhuma Task neste Project.</EmptyState>}
        </Panel>}
      </> : null}
    </Inspector>
  </div>;
}

function WorkspaceForm({ workspace, cancel, saved }: { workspace?: Workspace; cancel: () => void; saved: (workspace: Workspace) => void }) {
  const [name, setName] = useState(workspace?.name ?? "");
  const [description, setDescription] = useState(workspace?.description ?? "");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (saving) return;
    setSaving(true);
    setError("");
    try { saved(workspace ? await api.updateWorkspace(workspace.id, name, description) : await api.createWorkspace(name, description)); }
    catch (nextError) { setError(appError(nextError).message); setSaving(false); }
  }
  return <form className="stack-form" onSubmit={submit} aria-busy={saving}>
    <label><span>NOME</span><input value={name} onChange={(event) => setName(event.currentTarget.value)} autoFocus /></label>
    <label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={4} /></label>
    {saving ? <StateMessage state="saving" label="Salvando Workspace..." /> : error ? <StateMessage state="error" label={error} /> : null}
    <div className="form-actions"><Button variant="ghost" onClick={cancel} disabled={saving}>Cancelar</Button><Button variant="primary" type="submit" disabled={!name.trim() || saving}>{saving ? "Salvando" : "Salvar"}</Button></div>
  </form>;
}

function WorkspacesPage({ workspaces, projects, apps, initialWorkspaceId, refresh, receipt, openProject, openApp, openHome, intent }: { workspaces: Workspace[]; projects: Project[]; apps: RegisteredApp[]; initialWorkspaceId: string; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; openProject: (project: Project) => void; openApp: (app: RegisteredApp) => void; openHome: (workspace: Workspace) => void; intent?: FunctionIntent }) {
  const activeWorkspaces = workspaces.filter((workspace) => workspace.lifecycleState === "active");
  const activeProjects = projects.filter((project) => project.lifecycleState === "active");
  const activeApps = apps.filter((app) => app.lifecycleState === "active");
  const [selectedId, setSelectedId] = useState(initialWorkspaceId || activeWorkspaces[0]?.id || "");
  const [mode, setMode] = useState<"view" | "edit" | "new">("view");
  const [narrowPane, setNarrowPane] = useState<"list" | "detail">(initialWorkspaceId || intent?.target === "workspaces_create" ? "detail" : "list");
  const [workspaceProjects, setWorkspaceProjects] = useState<Project[]>([]);
  const [workspaceApps, setWorkspaceApps] = useState<RegisteredApp[]>([]);
  const [message, setMessage] = useState("");
  const [pendingAction, setPendingAction] = useState<"archive" | null>(null);
  const [error, setError] = useState("");
  const listPane = useRef<HTMLElement>(null);
  const inspector = useRef<HTMLElement>(null);

  useEffect(() => {
    if (!initialWorkspaceId) return;
    setSelectedId(initialWorkspaceId);
    setMode("view");
    setNarrowPane("detail");
  }, [initialWorkspaceId]);

  useEffect(() => {
    if (!intent) return;
    if (intent.target === "workspaces_create" || !activeWorkspaces.length) {
      setMode("new");
      setNarrowPane("detail");
      return;
    }
    const sections: Partial<Record<FunctionIntentTarget, string>> = {
      workspaces_link_project: "workspace.link_project",
      workspaces_link_app: "workspace.link_app",
    };
    const relation = sections[intent.target];
    if (relation) {
      setMode("view");
      setNarrowPane("detail");
      window.requestAnimationFrame(() => document.querySelector<HTMLElement>(`[data-function-section='${relation}'] input`)?.focus());
    }
  }, [intent?.key]);

  useEffect(() => {
    if (mode === "new") return;
    if (!activeWorkspaces.some((workspace) => workspace.id === selectedId)) {
      setSelectedId(activeWorkspaces[0]?.id ?? "");
      if (!activeWorkspaces.length) setNarrowPane("list");
    }
  }, [activeWorkspaces, selectedId, mode]);

  const selected = activeWorkspaces.find((workspace) => workspace.id === selectedId) ?? null;
  const linkedProjectIds = new Set(workspaceProjects.map((project) => project.id));
  const linkedAppIds = new Set(workspaceApps.map((app) => app.id));
  const workspacesEmpty = !activeWorkspaces.length && mode !== "new";

  const refreshLinks = useCallback(async () => {
    if (!selectedId) {
      setWorkspaceProjects([]);
      setWorkspaceApps([]);
      return;
    }
    const [nextProjects, nextApps] = await Promise.all([api.workspaceProjects(selectedId), api.workspaceApps(selectedId)]);
    setWorkspaceProjects(nextProjects);
    setWorkspaceApps(nextApps);
  }, [selectedId]);
  useEffect(() => { void refreshLinks().catch((nextError) => setMessage(appError(nextError).message)); }, [refreshLinks]);

  function selectWorkspace(workspace: Workspace) {
    setSelectedId(workspace.id);
    setMode("view");
    setMessage("");
    setError("");
    setNarrowPane("detail");
    if (window.matchMedia("(max-width: 960px)").matches) requestAnimationFrame(() => inspector.current?.focus());
  }

  function startNew() {
    setMode("new");
    setMessage("");
    setError("");
    setNarrowPane("detail");
    if (window.matchMedia("(max-width: 960px)").matches) requestAnimationFrame(() => inspector.current?.focus());
  }

  function returnToList() {
    setMode("view");
    setMessage("");
    setError("");
    setNarrowPane("list");
    requestAnimationFrame(() => {
      const selectedRow = listPane.current?.querySelector<HTMLButtonElement>(".data-row[data-selected]");
      const createAction = listPane.current?.querySelector<HTMLButtonElement>(".pane-heading-meta .button");
      (selectedRow ?? createAction)?.focus();
    });
  }

  async function toggleProject(project: Project, linked: boolean) {
    if (!selected) return;
    try {
      await api.setProjectWorkspace(project.id, selected.id, linked);
      setMessage(linked ? "Project vinculado." : "Project removido do Workspace.");
      await refreshLinks();
    } catch (nextError) { setMessage(appError(nextError).message); }
  }

  async function toggleApp(app: RegisteredApp, linked: boolean) {
    if (!selected) return;
    try {
      await api.setAppWorkspace(app.id, selected.id, linked);
      setMessage(linked ? "App vinculado." : "App removido do Workspace.");
      await refreshLinks();
    } catch (nextError) { setMessage(appError(nextError).message); }
  }

  // `refresh` e nao `refreshLinks`: o dado dos ocultos vem do componente raiz,
  // nao do estado local desta pagina.
  async function archiveWorkspace(workspace: Workspace) {
    setPendingAction("archive");
    setError("");
    try {
      await api.setWorkspaceArchived(workspace.id, true);
      receipt({ message: "Workspace arquivado.", run: () => api.setWorkspaceArchived(workspace.id, false) });
      setSelectedId(activeWorkspaces.find((candidate) => candidate.id !== workspace.id)?.id ?? "");
      setMode("view");
      setNarrowPane("list");
      await refresh();
    } catch (nextError) {
      setError(appError(nextError).message);
    } finally {
      setPendingAction(null);
    }
  }

  if (workspacesEmpty) {
    return <div className="page workspaces-empty-page">
      <ContextPath segments={["M", "WORKSPACES"]} />
      <section className="workspaces-empty-view" aria-labelledby="workspaces-empty-title">
        <span className="micro-label">0 WORKSPACES</span>
        <h1 id="workspaces-empty-title">Nenhum Workspace ainda.</h1>
        <p>Crie contextos amplos como Engineering, Finance ou Learning.</p>
        <Button variant="primary" onClick={startNew}>Novo Workspace</Button>
      </section>
    </div>;
  }

  return <div className="split-page inspector-page workspaces-page" data-pane={narrowPane}>
    <section ref={listPane} className="list-pane" aria-label="Workspaces ativos">
      <PaneHeader
        segments={["M", "WORKSPACES"]}
        meta={`${activeWorkspaces.length} ${activeWorkspaces.length === 1 ? "ATIVO" : "ATIVOS"}`}
        actions={<Button variant="ghost" size="sm" onClick={startNew}>Novo Workspace</Button>}
      />
      <div className="row-list">{activeWorkspaces.map((workspace) => <DataRow
        key={workspace.id}
        primary={workspace.name}
        secondary={workspace.description || undefined}
        meta={relativeTime(workspace.updatedAt)}
        selected={workspace.id === selectedId && mode !== "new"}
        onClick={() => selectWorkspace(workspace)}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            selectWorkspace(workspace);
            return;
          }
          const nextIndex = moveListFocus(event);
          if (nextIndex === null) return;
          const nextWorkspace = activeWorkspaces[nextIndex];
          if (!nextWorkspace) return;
          setSelectedId(nextWorkspace.id);
          setMode("view");
          setMessage("");
          setError("");
        }}
      />)}</div>
    </section>
    <Inspector
      ref={inspector}
      label="Detalhe do Workspace"
      open={narrowPane === "detail"}
      onBack={returnToList}
      onEscape={mode === "view" || mode === "new" ? returnToList : undefined}
    >
      {mode === "new" ? <>
        <span className="micro-label">NOVO WORKSPACE</span>
        <WorkspaceForm
          cancel={() => {
            if (selected) {
              setMode("view");
              setNarrowPane("detail");
              requestAnimationFrame(() => inspector.current?.focus());
            } else {
              returnToList();
            }
          }}
          saved={(workspace) => {
            setSelectedId(workspace.id);
            setMode("view");
            setNarrowPane("detail");
            void refresh().then(() => requestAnimationFrame(() => inspector.current?.focus()));
          }}
        />
      </> : selected ? mode === "edit" ? <>
        <span className="micro-label">EDITAR WORKSPACE</span>
        <WorkspaceForm
          workspace={selected}
          cancel={() => { setMode("view"); requestAnimationFrame(() => inspector.current?.focus()); }}
          saved={() => { setMode("view"); void refresh().then(() => requestAnimationFrame(() => inspector.current?.focus())); }}
        />
      </> : <>
        <header className="detail-header">
          <div>
            <span className="micro-label">WORKSPACE</span>
            <h1>{selected.name}</h1>
            <p>{selected.description || "Sem descrição."}</p>
          </div>
          <ActionMenu
            trigger={<Icon name="more" />}
            items={[
              { label: "Editar", disabled: pendingAction !== null, onSelect: () => setMode("edit") },
              { label: pendingAction === "archive" ? "Arquivando" : "Arquivar", danger: true, disabled: pendingAction !== null, onSelect: () => void archiveWorkspace(selected) },
            ]}
          />
        </header>
        {error ? <p className="inline-error" role="alert">! {error}</p> : null}
        <div className="workspace-grid">
          <div data-function-section="workspace.link_project">
            <Panel label="PROJECTS">
              {activeProjects.length
                ? activeProjects.map((project) => <div className="relation-row" key={project.id}><label><input type="checkbox" checked={linkedProjectIds.has(project.id)} onChange={(event) => void toggleProject(project, event.currentTarget.checked)} /><span><strong>{project.name}</strong><small>{project.description || "Sem descrição."}</small></span></label><button type="button" onClick={() => openProject(project)}>Abrir</button></div>)
                : <EmptyState>Projects ativos aparecerão aqui.</EmptyState>}
            </Panel>
          </div>
          <div data-function-section="workspace.link_app">
            <Panel label="APPS">
              {activeApps.length
                ? activeApps.map((app) => <div className="relation-row" key={app.id}><label><input type="checkbox" checked={linkedAppIds.has(app.id)} onChange={(event) => void toggleApp(app, event.currentTarget.checked)} /><span><strong>{app.name}</strong><small>{app.description || app.launchTarget || "Sem descrição."}</small></span></label><button type="button" onClick={() => openApp(app)}>Abrir</button></div>)
                : <EmptyState>Apps ativos aparecerão aqui.</EmptyState>}
            </Panel>
          </div>
          {/* A lista de caixinhas morava aqui e foi para a Home.

              Nao era duplicacao inofensiva: a mesma escolha em dois lugares e
              como eles divergem. E a versao da Home sabe mais — la se ve O QUE
              se esconde, onde ele fica e o que a faixa vira sem ele; aqui era
              uma lista de rotulos que nao mostrava nada disso.

              Fica o caminho, porque quem procurou aqui uma vez vai procurar de
              novo. O botao leva ao contexto certo; abrir o modo de arrumar e o
              clique seguinte, e ele esta a vista. */}
          <div>
            <Panel label="WIDGETS DA HOME">
              <p className="support-copy">A Home de cada contexto se arruma na própria Home: lá dá para esconder, mover e mudar o tamanho de cada widget vendo o resultado.</p>
              <div className="button-line"><Button variant="outline" size="sm" onClick={() => openHome(selected)}>Abrir a Home de {selected.name}</Button></div>
            </Panel>
          </div>
        </div>
        {message ? <p className="settings-message" aria-live="polite">{message}</p> : null}
      </> : null}
    </Inspector>
  </div>;
}


function launchKindLabel(kind: AppLaunchKind | null) {
  if (kind === "url") return "URL";
  if (kind === "path") return "Path";
  return "Sem alvo";
}

function RegisteredAppForm({ app, cancel, saved }: { app?: RegisteredApp; cancel: () => void; saved: (app: RegisteredApp) => void }) {
  const [name, setName] = useState(app?.name ?? "");
  const [description, setDescription] = useState(app?.description ?? "");
  const [sourceUrl, setSourceUrl] = useState(app?.sourceUrl ?? "");
  const [launchKind, setLaunchKind] = useState<AppLaunchKind | "">((app?.launchKind ?? "") as AppLaunchKind | "");
  const [launchTarget, setLaunchTarget] = useState(app?.launchTarget ?? "");
  // Um app com alvo de lancamento ja abre — declarar o contrario seria mentir
  // sobre uma capacidade em uso. Mesma regra da migration 0007.
  const [capabilities, setCapabilities] = useState<AppCapabilities>(() => app ? { canOpen: app.canOpen, canRead: app.canRead, canWrite: app.canWrite, canAutomate: app.canAutomate } : { canOpen: true, canRead: false, canWrite: false, canAutomate: false });
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  async function choosePath(directory: boolean) {
    const selected = await open({ multiple: false, directory });
    if (typeof selected === "string") setLaunchTarget(selected);
  }
  async function submit(event: FormEvent) {
    event.preventDefault();
    setSaving(true);
    const kind = launchKind || null;
    const target = launchTarget.trim() ? launchTarget : null;
    const source = sourceUrl.trim() ? sourceUrl : null;
    try {
      saved(app ? await api.updateRegisteredApp(app.id, name, description, source, kind, target, capabilities) : await api.createRegisteredApp(name, description, source, kind, target));
    } catch (nextError) {
      setError(appError(nextError).message);
      setSaving(false);
    }
  }
  return <form className="stack-form" onSubmit={submit} aria-busy={saving}>
    <label><span>NOME</span><input value={name} onChange={(event) => setName(event.currentTarget.value)} autoFocus /></label>
    <label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={4} /></label>
    <label><span>ORIGEM</span><input value={sourceUrl} onChange={(event) => setSourceUrl(event.currentTarget.value)} placeholder="https://github.com/..." /></label>
    <label><span>TIPO DE ABERTURA</span><select value={launchKind} onChange={(event) => { setLaunchKind(event.currentTarget.value as AppLaunchKind | ""); if (!event.currentTarget.value) setLaunchTarget(""); }}><option value="">Sem alvo por enquanto</option><option value="url">URL</option><option value="path">Path local</option></select></label>
    {launchKind ? <label><span>ALVO</span>{launchKind === "path" ? <div className="target-picker"><input value={launchTarget} onChange={(event) => setLaunchTarget(event.currentTarget.value)} placeholder={"C:\\Apps\\app.exe"} /><Button variant="outline" onClick={() => void choosePath(false)}>Escolher arquivo</Button><Button variant="ghost" onClick={() => void choosePath(true)}>Escolher pasta</Button></div> : <input value={launchTarget} onChange={(event) => setLaunchTarget(event.currentTarget.value)} placeholder="https://..." />}</label> : null}
    <fieldset className="capability-fieldset"><legend className="micro-label">CAPACIDADES</legend>{([["canOpen", "OPEN"], ["canRead", "READ"], ["canWrite", "WRITE"], ["canAutomate", "AUTOMATE"]] as const).map(([key, label]) => <label className="capability-check" key={key}><input type="checkbox" checked={capabilities[key]} onChange={(event) => setCapabilities((current) => ({ ...current, [key]: event.currentTarget.checked }))} /><span className="micro-label">{label}</span></label>)}</fieldset>
    {saving ? <StateMessage state="saving" label="Salvando App..." /> : error ? <StateMessage state="error" label={error} /> : null}
    <div className="form-actions"><Button variant="ghost" onClick={cancel} disabled={saving}>Cancelar</Button><Button variant="primary" type="submit" disabled={!name.trim() || saving}>{saving ? "Salvando" : "Salvar"}</Button></div>
  </form>;
}

function AppsPage({ apps, initialAppId, refresh, receipt, intent }: { apps: RegisteredApp[]; initialAppId: string; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; intent?: FunctionIntent }) {
  const visibleApps = apps.filter((app) => app.lifecycleState === "active" || app.id === initialAppId);
  const [selectedId, setSelectedId] = useState(initialAppId || visibleApps[0]?.id || "");
  const [mode, setMode] = useState<"view" | "edit" | "new">("view");
  const [narrowPane, setNarrowPane] = useState<"list" | "detail">(initialAppId || intent?.target === "apps_register" ? "detail" : "list");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [pendingAction, setPendingAction] = useState<"open" | "archive" | null>(null);
  const [creatingSuggestions, setCreatingSuggestions] = useState(false);
  const [catalog, setCatalog] = useState<AppCatalogEntry[]>([]);
  const listPane = useRef<HTMLElement>(null);
  const inspector = useRef<HTMLElement>(null);
  const missingSuggestions = catalog.filter((suggestion) => !apps.some((app) => app.sourceUrl === suggestion.sourceUrl || app.name.toLowerCase() === suggestion.name.toLowerCase()));

  useEffect(() => { void api.appCatalog().then(setCatalog).catch((nextError) => setMessage(appError(nextError).message)); }, []);

  useEffect(() => {
    if (!initialAppId) return;
    setSelectedId(initialAppId);
    setMode("view");
    setNarrowPane("detail");
  }, [initialAppId]);

  useEffect(() => {
    if (intent?.target !== "apps_register") return;
    setMode("new");
    setNarrowPane("detail");
  }, [intent?.key]);

  useEffect(() => {
    if (mode === "new") return;
    if (!visibleApps.some((app) => app.id === selectedId)) {
      setSelectedId(visibleApps[0]?.id ?? "");
      if (!visibleApps.length) setNarrowPane("list");
    }
  }, [visibleApps, selectedId, mode]);

  const selected = visibleApps.find((app) => app.id === selectedId) ?? null;
  const appsEmpty = !visibleApps.length && mode !== "new";

  function selectApp(app: RegisteredApp) {
    setSelectedId(app.id);
    setMode("view");
    setMessage("");
    setError("");
    setNarrowPane("detail");
    if (window.matchMedia("(max-width: 960px)").matches) requestAnimationFrame(() => inspector.current?.focus());
  }

  function startNew() {
    setMode("new");
    setMessage("");
    setError("");
    setNarrowPane("detail");
    if (window.matchMedia("(max-width: 960px)").matches) requestAnimationFrame(() => inspector.current?.focus());
  }

  function returnToList() {
    setMode("view");
    setMessage("");
    setError("");
    setNarrowPane("list");
    requestAnimationFrame(() => {
      const selectedRow = listPane.current?.querySelector<HTMLButtonElement>(".data-row[data-selected]");
      const createAction = listPane.current?.querySelector<HTMLButtonElement>(".pane-heading-meta .button");
      (selectedRow ?? createAction)?.focus();
    });
  }

  async function openApp(app: RegisteredApp) {
    setPendingAction("open");
    setMessage("");
    setError("");
    try {
      await api.openRegisteredApp(app.id);
      setMessage("App aberto.");
      await refresh();
    } catch (nextError) {
      setError(appError(nextError).message);
    } finally {
      setPendingAction(null);
    }
  }

  async function archiveApp(app: RegisteredApp) {
    setPendingAction("archive");
    setError("");
    try {
      await api.setRegisteredAppArchived(app.id, true);
      receipt({ message: "App arquivado.", run: () => api.setRegisteredAppArchived(app.id, false) });
      setSelectedId(visibleApps.find((candidate) => candidate.id !== app.id)?.id ?? "");
      setMode("view");
      setNarrowPane("list");
      await refresh();
    } catch (nextError) {
      setError(appError(nextError).message);
    } finally {
      setPendingAction(null);
    }
  }

  async function addSuggestions() {
    if (!missingSuggestions.length || creatingSuggestions) return;
    setCreatingSuggestions(true);
    try {
      const created = await api.registerAppCatalog(missingSuggestions.map((suggestion) => suggestion.id));
      const lastCreated = created[created.length - 1] ?? null;
      if (lastCreated) {
        setSelectedId(lastCreated.id);
        setNarrowPane("detail");
      }
      setMode("view");
      setMessage(`${missingSuggestions.length} Apps conhecidos adicionados.`);
      await refresh();
    } catch (nextError) {
      setMessage(appError(nextError).message);
    } finally {
      setCreatingSuggestions(false);
    }
  }

  if (appsEmpty) {
    return <div className="page apps-empty-page">
      <ContextPath segments={["M", "APPS"]} />
      <section className="apps-empty-view" aria-labelledby="apps-empty-title">
        <span className="micro-label">0 APPS</span>
        <h1 id="apps-empty-title">Nenhum App cadastrado.</h1>
        <p>Cadastre as ferramentas que você usa para não depender da memória.</p>
        <div className="button-line">
          <Button variant="primary" onClick={startNew}>Novo App</Button>
          {missingSuggestions.length ? <Button variant="ghost" onClick={() => void addSuggestions()} disabled={creatingSuggestions}>{creatingSuggestions ? "Adicionando" : "Adicionar meus Apps"}</Button> : null}
        </div>
      </section>
    </div>;
  }

  return <div className="split-page inspector-page apps-page" data-pane={narrowPane}>
    <section ref={listPane} className="list-pane" aria-label="Apps registrados">
      <PaneHeader
        segments={["M", "APPS"]}
        meta={`${visibleApps.length} ${visibleApps.length === 1 ? "ITEM" : "ITENS"}`}
        actions={
          <div className="pane-heading-actions">
            {missingSuggestions.length ? <Button variant="ghost" size="sm" onClick={() => void addSuggestions()} disabled={creatingSuggestions}>{creatingSuggestions ? "Adicionando" : "Adicionar meus Apps"}</Button> : null}
            <Button variant="ghost" size="sm" onClick={startNew}>Novo App</Button>
          </div>
        }
      />
      <div className="row-list">{visibleApps.map((app) => <DataRow
        key={app.id}
        primary={app.name}
        secondary={app.description || app.launchTarget || undefined}
        meta={app.lifecycleState === "archived" ? "ARQUIVADO" : launchKindLabel(app.launchKind)}
        selected={app.id === selectedId && mode !== "new"}
        onClick={() => selectApp(app)}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            selectApp(app);
            return;
          }
          const nextIndex = moveListFocus(event);
          if (nextIndex === null) return;
          const nextApp = visibleApps[nextIndex];
          if (!nextApp) return;
          setSelectedId(nextApp.id);
          setMode("view");
          setMessage("");
          setError("");
        }}
      />)}</div>
    </section>
    <Inspector
      ref={inspector}
      label="Detalhe do App"
      open={narrowPane === "detail"}
      onBack={returnToList}
      onEscape={mode === "view" || mode === "new" ? returnToList : undefined}
    >
      {mode === "new" ? <>
        <span className="micro-label">NOVO APP</span>
        <RegisteredAppForm
          cancel={() => {
            if (selected) {
              setMode("view");
              setNarrowPane("detail");
              requestAnimationFrame(() => inspector.current?.focus());
            } else {
              returnToList();
            }
          }}
          saved={(app) => {
            setSelectedId(app.id);
            setMode("view");
            setNarrowPane("detail");
            void refresh().then(() => requestAnimationFrame(() => inspector.current?.focus()));
          }}
        />
      </> : selected ? mode === "edit" ? <>
        <span className="micro-label">EDITAR APP</span>
        <RegisteredAppForm
          app={selected}
          cancel={() => { setMode("view"); requestAnimationFrame(() => inspector.current?.focus()); }}
          saved={() => { setMode("view"); void refresh().then(() => requestAnimationFrame(() => inspector.current?.focus())); }}
        />
      </> : <>
        <header className="detail-header">
          <div>
            <span className="micro-label">APP{selected.lifecycleState === "archived" ? " · ARQUIVADO" : ""}</span>
            <div className="app-identity">
              <AppIcon app={selected} />
              <div>
                <h1>{selected.name}</h1>
                <p>{selected.description || "Sem descrição."}</p>
              </div>
            </div>
          </div>
          <ActionMenu
            trigger={<Icon name="more" />}
            items={[
              { label: "Editar", disabled: pendingAction !== null, onSelect: () => setMode("edit") },
              { label: pendingAction === "archive" ? "Arquivando" : "Arquivar", danger: true, disabled: pendingAction !== null, onSelect: () => void archiveApp(selected) },
            ]}
          />
        </header>
        {error ? <p className="inline-error" role="alert">! {error}</p> : null}
        <div className="detail-actions">
          <Button variant="primary" onClick={() => void openApp(selected)} disabled={!selected.launchTarget || selected.lifecycleState !== "active" || pendingAction !== null}>{pendingAction === "open" ? "Abrindo" : "Abrir"}</Button>
          <Button variant="ghost" onClick={() => setMode("edit")} disabled={pendingAction !== null}>Editar</Button>
        </div>
        <dl className="fact-grid" data-framed>
          <div><dt>TIPO</dt><dd>{launchKindLabel(selected.launchKind)}</dd></div>
          <div><dt>ORIGEM</dt><dd className="mono-value">{selected.sourceUrl || <span className="fact-empty">Não definida</span>}</dd></div>
          <div><dt>DESTINO</dt><dd className="mono-value">{selected.launchTarget || <span className="fact-empty">Não definido</span>}</dd></div>
          <div><dt>ÚLTIMA ABERTURA</dt><dd>{selected.lastOpenedAt ? relativeTime(selected.lastOpenedAt) : <span className="fact-empty">Nunca</span>}</dd></div>
        </dl>
        <Panel label="CAPACIDADES" className="capability-panel">
          {([["OPEN", selected.canOpen], ["READ", selected.canRead], ["WRITE", selected.canWrite], ["AUTOMATE", selected.canAutomate]] as const).map(([label, granted]) => (
            <div className="capability-row" key={label}>
              <span className="micro-label">{label}</span>
              <span data-granted={granted || undefined}>{granted ? "Concedida" : "Não"}</span>
            </div>
          ))}
        </Panel>
        {message ? <p className="settings-message" aria-live="polite">{message}</p> : null}
      </> : null}
    </Inspector>
  </div>;
}


function ResourceForm({ resource, capture, cancel, saved }: { resource?: Resource; capture?: Capture; cancel: () => void; saved: (resource: Resource) => void }) {
  const captureContent = capture?.content.trim() ?? "";
  const captureIsUrl = /^https?:\/\//i.test(captureContent);
  const [url, setUrl] = useState(resource?.url ?? (captureIsUrl ? captureContent : ""));
  const [title, setTitle] = useState(resource?.title ?? "");
  const [note, setNote] = useState(resource?.note ?? (captureIsUrl ? "" : captureContent));
  // Uma Capture que nao e URL vira Note por padrao: o texto ja e o conteudo.
  const [kind, setKind] = useState<ResourceKind>(resource?.kind ?? (capture && !captureIsUrl ? "note" : "site"));
  const needsUrl = kind !== "note" && kind !== "file";
  /* Um Resource de arquivo nao troca de tipo. O tipo dele nao e uma preferencia
     de catalogacao: e o fato de existir um arquivo guardado apontando para ele,
     e um seletor que permitisse "virar site" produziria um Resource orfao do
     proprio conteudo. Editar titulo e motivo continua liberado. */
  const kindIsFixed = resource?.kind === "file";
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (saving) return;
    if (needsUrl && !url.trim()) return;
    if (!needsUrl && !title.trim()) return;
    setSaving(true);
    try {
      const next = resource
        ? await api.updateResource(resource.id, kind, title, needsUrl ? url : "", note)
        : await api.createResource(kind, title, needsUrl ? url : "", note, capture?.id ?? null);
      saved(next);
    } catch (nextError) {
      setError(appError(nextError).message);
      setSaving(false);
    }
  }
  return <form className="stack-form" onSubmit={submit} aria-busy={saving}>
    <fieldset className="form-fields" disabled={saving}>
      {kindIsFixed ? <p className="micro-label">TIPO · ARQUIVO</p> : <label><span>TIPO</span><select value={kind} onChange={(event) => setKind(event.currentTarget.value as ResourceKind)}><option value="site">Site</option><option value="library">Library</option><option value="image">Imagem</option><option value="note">Nota</option></select></label>}
      {needsUrl ? <label><span>{kind === "image" ? "ENDEREÇO OU CAMINHO" : "URL"}</span><input value={url} onChange={(event) => setUrl(event.currentTarget.value)} placeholder={kind === "image" ? "https://... ou C:\\imagens\\hero.png" : "https://..."} autoFocus /></label> : null}
      <label><span>TÍTULO</span><input value={title} onChange={(event) => setTitle(event.currentTarget.value)} placeholder={needsUrl ? "Opcional · usa a URL quando vazio" : "Obrigatório para uma nota"} autoFocus={!needsUrl} /></label>
      <label><span>POR QUÊ?</span><textarea value={note} onChange={(event) => setNote(event.currentTarget.value)} placeholder="O que merece ser lembrado sobre este link?" rows={4} /></label>
      {capture ? <div className="provenance"><span className="micro-label">ORIGEM PRESERVADA</span><span>{capture.content}</span><small>{sourceLabel(capture.source)} · {relativeTime(capture.capturedAt)}</small></div> : null}
      {saving ? <StateMessage state="saving" label="Salvando Resource..." /> : error ? <StateMessage state="error" label={error} detail="Os campos continuam preenchidos para uma nova tentativa." /> : null}
      <div className="form-actions"><Button variant="ghost" onClick={cancel}>Cancelar</Button><Button variant="primary" type="submit" disabled={saving || (needsUrl ? !url.trim() : !title.trim())}>{saving ? "Salvando" : "Salvar Resource"}</Button></div>
    </fieldset>
  </form>;
}

function LibraryPage({ resources, workspaces, resourceWorkspaces, ingestions, currentWorkspace, initialResourceId, initialResourceKey, refresh, receipt, openCapture, intent }: { resources: Resource[]; workspaces: Workspace[]; resourceWorkspaces: ResourceWorkspace[]; ingestions: Ingestion[]; currentWorkspace: Workspace | null; initialResourceId: string; initialResourceKey: number; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; openCapture: (capture: Capture) => void; intent?: FunctionIntent }) {
  const activeResources = resources.filter((resource) => resource.lifecycleState === "active");
  const [selectedId, setSelectedId] = useState(initialResourceId || activeResources[0]?.id || "");
  const [mode, setMode] = useState<"view" | "edit" | "new">("view");
  const [narrowPane, setNarrowPane] = useState<"list" | "detail">(initialResourceId ? "detail" : "list");
  const [source, setSource] = useState<Capture | null>(null);
  const [sourceError, setSourceError] = useState(false);
  const [message, setMessage] = useState("");
  const [pendingAction, setPendingAction] = useState<"open" | "archive" | "trash" | "restore" | null>(null);
  const list = useRef<HTMLDivElement>(null);
  const detail = useRef<HTMLElement>(null);
  // Filtro e apresentacao sao preferencia de leitura, nao dado: vivem aqui e
  // nao no banco. O alternador GRID/LISTA e do proprio design.
  const [kindFilter, setKindFilter] = useState<ResourceKind | "all">("all");
  const [view, setView] = useState<"grid" | "list">("grid");
  // Ligado por padrao quando ha contexto: o caminho anuncia o recorte, entao a
  // lista tem que cumpri-lo. Sem contexto ativo o estado nao tem efeito.
  const [scoped, setScoped] = useState(true);
  const activeWorkspaces = workspaces.filter((workspace) => workspace.lifecycleState === "active");
  /* O arquivo de cada Resource, quando ele veio por drop. O mapa e montado uma
     vez por render em vez de uma consulta por linha: sao poucas dezenas de
     ingestoes, e uma chamada por card faria a lista piscar. */
  const arquivoDe = new Map(ingestions.filter((item) => item.resourceId).map((item) => [item.resourceId as string, item]));
  const linkedWorkspaceIds = new Set(resourceWorkspaces.filter((link) => link.resourceId === selectedId).map((link) => link.workspaceId));
  // O `currentWorkspace !== null` repetido abaixo nao e redundancia: o tsc nao
  // estreita um objeto a partir de um boolean derivado guardado em variavel.
  const scoping = scoped && currentWorkspace !== null;
  // O caminho so anuncia o recorte quando ele esta de fato aplicado. Anunciar
  // sem aplicar foi o que este ciclo veio corrigir.
  const workspaceSegment = scoping && currentWorkspace ? currentWorkspace.name.toUpperCase() : null;
  const scopedResourceIds = new Set(currentWorkspace ? resourceWorkspaces.filter((link) => link.workspaceId === currentWorkspace.id).map((link) => link.resourceId) : []);
  const liveResources = resources.filter((resource) => resource.lifecycleState === "active" || resource.id === selectedId);
  // O selecionado nunca some da lista, mesmo fora do recorte: ele esta aberto ao
  // lado, e sumir da lista o que esta aberto e desorientador.
  const contextResources = scoping ? liveResources.filter((resource) => scopedResourceIds.has(resource.id) || resource.id === selectedId) : liveResources;
  const visibleResources = kindFilter === "all" ? contextResources : contextResources.filter((resource) => resource.kind === kindFilter || resource.id === selectedId);
  const selected = visibleResources.find((resource) => resource.id === selectedId) ?? null;
  const arquivo = selected ? arquivoDe.get(selected.id) ?? null : null;

  useEffect(() => {
    if (!initialResourceId) return;
    setSelectedId(initialResourceId);
    setMode("view");
    setNarrowPane("detail");
  }, [initialResourceId, initialResourceKey]);

  useEffect(() => {
    if (intent?.target !== "library_create") return;
    setMode("new");
    setNarrowPane("detail");
  }, [intent?.key]);

  useEffect(() => {
    const nextVisibleResources = resources.filter((resource) => resource.lifecycleState === "active" || resource.id === selectedId);
    if (nextVisibleResources.some((resource) => resource.id === selectedId)) return;
    const nextId = nextVisibleResources[0]?.id ?? "";
    setSelectedId(nextId);
    if (!nextId) setNarrowPane("list");
  }, [resources, selectedId]);

  useEffect(() => {
    setSource(null);
    setSourceError(false);
    if (selected?.sourceCaptureId) void api.getCapture(selected.sourceCaptureId).then(setSource).catch(() => setSourceError(true));
  }, [selected?.id, selected?.sourceCaptureId]);

  function startNew() {
    setMode("new");
    setNarrowPane("detail");
    setMessage("");
    if (window.matchMedia("(max-width: 960px)").matches) requestAnimationFrame(() => detail.current?.focus());
  }

  function selectResource(resource: Resource) {
    setSelectedId(resource.id);
    setMode("view");
    setNarrowPane("detail");
    setMessage("");
    if (window.matchMedia("(max-width: 960px)").matches) requestAnimationFrame(() => detail.current?.focus());
  }

  function returnToList() {
    setMode("view");
    setNarrowPane("list");
    requestAnimationFrame(() => {
      const selectedTile = document.querySelector<HTMLButtonElement>(".library-page .tile[data-selected]");
      const selectedRow = list.current?.querySelector<HTMLButtonElement>(".data-row[data-selected]");
      const emptyAction = document.querySelector<HTMLButtonElement>(".library-page .library-empty .button");
      const createAction = document.querySelector<HTMLButtonElement>(".library-page .pane-heading-meta .button");
      (selectedTile ?? selectedRow ?? emptyAction ?? createAction)?.focus();
    });
  }

  function moveCollectionFocus(event: KeyboardEvent<HTMLButtonElement>, resourcesList: Resource[]) {
    if (!["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return null;
    const container = event.currentTarget.closest(".tile-grid, .row-list");
    if (!container) return null;
    const items = Array.from(container.querySelectorAll<HTMLButtonElement>(".tile, .data-row"));
    const currentIndex = items.indexOf(event.currentTarget);
    if (currentIndex < 0 || !items.length) return null;
    event.preventDefault();
    let nextIndex = currentIndex;
    if (event.key === "Home") nextIndex = 0;
    else if (event.key === "End") nextIndex = items.length - 1;
    else if (event.key === "ArrowUp" || event.key === "ArrowLeft") nextIndex = Math.max(0, currentIndex - 1);
    else nextIndex = Math.min(items.length - 1, currentIndex + 1);
    items[nextIndex]?.focus();
    return resourcesList[nextIndex] ?? null;
  }

  // Sem mensagem de sucesso: a caixa marcada ja e a confirmacao, e uma frase a
  // cada clique numa lista de cinco viraria ruido. O erro continua falando,
  // porque ai o silencio mentiria.
  async function toggleWorkspace(workspaceId: string, linked: boolean) {
    if (!selected) return;
    try {
      await api.setResourceWorkspace(selected.id, workspaceId, linked);
      await refresh();
    } catch (nextError) { setMessage(appError(nextError).message); }
  }

  async function openLink(resource: Resource) {
    setPendingAction("open");
    setMessage("");
    try {
      await api.openResource(resource.id);
      setMessage("Link aberto no navegador padrão.");
    } catch (nextError) {
      setMessage(appError(nextError).message);
    } finally {
      setPendingAction(null);
    }
  }

  /* O M/OS pede ao Windows para abrir; ele nunca executa nada por conta
     propria, e recusa por completo o que o shell trataria como programa. */
  async function abrirArquivo(resourceId: string) {
    setPendingAction("open");
    setMessage("");
    try {
      await api.openIngestedFile(resourceId);
      setMessage("Arquivo aberto no programa padrão.");
    } catch (nextError) {
      setMessage(appError(nextError).message);
    } finally {
      setPendingAction(null);
    }
  }

  async function mostrarNaPasta(resourceId: string) {
    setMessage("");
    try {
      await api.revealIngestedFile(resourceId);
    } catch (nextError) {
      setMessage(appError(nextError).message);
    }
  }

  async function archive(resource: Resource) {
    setPendingAction("archive");
    try {
      await api.setResourceArchived(resource.id, true);
      receipt({ message: "Resource arquivado.", run: () => api.setResourceArchived(resource.id, false) });
      setSelectedId(activeResources.find((candidate) => candidate.id !== resource.id)?.id ?? "");
      setNarrowPane("list");
      setMessage("");
      await refresh();
    } catch (nextError) {
      setMessage(appError(nextError).message);
    } finally {
      setPendingAction(null);
    }
  }

  async function trash(resource: Resource) {
    setPendingAction("trash");
    try {
      await api.trashResource(resource.id);
      receipt({ message: "Resource movido para a Lixeira.", run: () => api.restoreResource(resource.id) });
      setSelectedId(activeResources.find((candidate) => candidate.id !== resource.id)?.id ?? "");
      setNarrowPane("list");
      setMessage("");
      await refresh();
    } catch (nextError) {
      setMessage(appError(nextError).message);
    } finally {
      setPendingAction(null);
    }
  }

  async function restore(resource: Resource) {
    setPendingAction("restore");
    setMessage("");
    try {
      await api.setResourceArchived(resource.id, false);
      setMessage("Resource restaurado para a Library.");
      await refresh();
    } catch (nextError) {
      setMessage(appError(nextError).message);
    } finally {
      setPendingAction(null);
    }
  }

  const libraryIsEmpty = !visibleResources.length && mode === "view";

  const kindLabels: Record<ResourceKind, string> = { site: "SITE", library: "LIBRARY", image: "IMAGEM", note: "NOTA", file: "ARQUIVO" };

  return <div className="split-page inspector-page library-page" data-pane={narrowPane} data-empty={libraryIsEmpty || undefined} data-view={view}>
    <section className="list-pane" aria-labelledby="library-title">
      <h1 id="library-title" className="visually-hidden">Library</h1>
      {/* O caminho carrega o workspace ativo quando existe: M / WEB-DESIGN /
          LIBRARY. E o que diz de qual acervo voce esta olhando. */}
      <PaneHeader
        segments={workspaceSegment ? ["M", workspaceSegment, "LIBRARY"] : ["M", "LIBRARY"]}
        meta={`${contextResources.length} ${contextResources.length === 1 ? "ITEM" : "ITENS"}`}
        actions={visibleResources.length ? <Button variant="ghost" size="sm" onClick={startNew}>Novo Resource</Button> : undefined}
      />
      {/* Filtros sao texto, nao chip: um chip por tipo viraria cinco caixas
          competindo com o acervo, que e o que importa nesta tela. */}
      <div className="filter-bar">
        {/* Trocar de contexto continua sendo coisa da Home; aqui so se decide se
            o contexto vigente se aplica. Sem contexto ativo o grupo nao aparece:
            botao que nao muda nada e pior que botao nenhum. */}
        {currentWorkspace ? <div className="filter-group" role="group" aria-label="Filtrar por contexto">
          {([[true, "NESTE CONTEXTO"], [false, "TUDO"]] as const).map(([value, label]) => <button key={label} type="button" className="filter-label" data-active={scoped === value || undefined} aria-pressed={scoped === value} onClick={() => setScoped(value)}>{label}</button>)}
        </div> : null}
        <div className="filter-group" role="group" aria-label="Filtrar por tipo">
          {([["all", "TUDO"], ["site", "SITES"], ["library", "LIBRARIES"], ["image", "IMAGENS"], ["note", "NOTAS"], ["file", "ARQUIVOS"]] as const).map(([value, label]) => <button key={value} type="button" className="filter-label" data-active={kindFilter === value || undefined} aria-pressed={kindFilter === value} onClick={() => setKindFilter(value)}>{label}</button>)}
        </div>
        <div className="filter-group" role="group" aria-label="Apresentação">
          {([["grid", "GRID"], ["list", "LISTA"]] as const).map(([value, label]) => <button key={value} type="button" className="filter-label" data-active={view === value || undefined} aria-pressed={view === value} onClick={() => setView(value)}>{label}</button>)}
        </div>
      </div>
      {view === "grid" ? <div className="tile-grid" aria-label="Resources salvos">{visibleResources.map((resource) => <button
        key={resource.id}
        type="button"
        className="tile"
        data-selected={resource.id === selectedId || undefined}
        aria-current={resource.id === selectedId ? "true" : undefined}
        onClick={() => selectResource(resource)}
        onDoubleClick={() => { if (resource.url) void api.openResource(resource.id); }}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            selectResource(resource);
            return;
          }
          const nextResource = moveCollectionFocus(event, visibleResources);
          if (!nextResource) return;
          setSelectedId(nextResource.id);
          setMode("view");
          setMessage("");
        }}
      ><span className="tile-face" aria-hidden="true"><span className="tile-kind">{kindLabels[resource.kind]}</span></span><strong className="tile-title">{resource.title}</strong>{/* O motivo e o que torna o acervo recuperavel: ele nunca e omitido. */}<span className="tile-reason" data-missing={resource.note ? undefined : true}>{resource.note || "Sem motivo registrado — abra e diga por que isto merece ser lembrado."}</span><span className="tile-origin">{resourceHost(resource.url) || "LOCAL"}</span></button>)}</div> : <div ref={list} className="row-list" aria-label="Resources salvos">
        {visibleResources.map((resource) => <DataRow
          key={resource.id}
          primary={resource.title}
          secondary={resourceHost(resource.url) || kindLabels[resource.kind]}
          meta={resource.lifecycleState === "archived" ? "ARQUIVADO" : relativeTime(resource.updatedAt)}
          selected={resource.id === selectedId}
          onClick={() => selectResource(resource)}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              selectResource(resource);
              return;
            }
            const nextResource = moveCollectionFocus(event, visibleResources);
            if (!nextResource) return;
            setSelectedId(nextResource.id);
            setMode("view");
            setMessage("");
          }}
        />)}
      </div>}
      {/* Dois vazios diferentes. Acervo vazio pede o primeiro link; recorte
          vazio com acervo cheio e o estado de TODO Workspace no dia seguinte a
          migration, e precisa dizer que o acervo esta intacto em vez de parecer
          perda de dado. */}
      {!visibleResources.length && mode !== "new" ? (scoping && liveResources.length ? <div className="library-empty"><ScopedEmptyState total={liveResources.length} workspace={currentWorkspace} noun="resource" onLink={() => setScoped(false)} linkLabel="Ver tudo" /></div> : <div className="library-empty"><EmptyState>Guarde um link junto do motivo pelo qual ele merece ser lembrado.</EmptyState><Button variant="primary" onClick={startNew}>Salvar primeiro link</Button></div>) : null}
    </section>
    <Inspector
      ref={detail}
      label="Detalhe do Resource"
      open={narrowPane === "detail"}
      onBack={returnToList}
      onEscape={mode === "view" || mode === "new" ? returnToList : undefined}
    >
      {mode === "new" ? <>
        <span className="micro-label">NOVO RESOURCE</span>
        <ResourceForm
          cancel={() => {
            if (selected) {
              setMode("view");
              setNarrowPane("detail");
              requestAnimationFrame(() => detail.current?.focus());
            } else {
              returnToList();
            }
          }}
          saved={(resource) => {
            setSelectedId(resource.id);
            setMode("view");
            setNarrowPane("detail");
            void refresh().then(() => requestAnimationFrame(() => detail.current?.focus()));
          }}
        />
      </> : selected ? mode === "edit" ? <>
        <span className="micro-label">EDITAR RESOURCE</span>
        <ResourceForm
          resource={selected}
          cancel={() => { setMode("view"); requestAnimationFrame(() => detail.current?.focus()); }}
          saved={() => { setMode("view"); void refresh().then(() => requestAnimationFrame(() => detail.current?.focus())); }}
        />
      </> : <>
        <header className="detail-header">
          <div>
            <span className="micro-label">RESOURCE · {kindLabels[selected.kind]}{selected.lifecycleState === "archived" ? " · ARQUIVADO" : ""}</span>
            <h1>{selected.title}</h1>
            {selected.url ? <p className="resource-url">{selected.url}</p> : null}
          </div>
          <ActionMenu
            trigger={<Icon name="more" />}
            items={[
              { label: "Editar", disabled: pendingAction !== null, onSelect: () => setMode("edit") },
              ...(selected.lifecycleState === "active" ? [
                { label: pendingAction === "archive" ? "Arquivando" : "Arquivar", disabled: pendingAction !== null, onSelect: () => void archive(selected) },
                { label: pendingAction === "trash" ? "Movendo" : "Mover para a Lixeira", danger: true, disabled: pendingAction !== null, onSelect: () => void trash(selected) },
              ] : [{ label: pendingAction === "restore" ? "Restaurando" : "Restaurar", disabled: pendingAction !== null, onSelect: () => void restore(selected) }]),
            ]}
          />
        </header>
        <div className="resource-note"><span className="micro-label">POR QUÊ?</span><p>{selected.note || "Nenhum contexto adicional foi registrado."}</p></div>
        {/* As duas perguntas se leem juntas: por que guardei isto, e a que lente
            pertence. Sem Workspace ativo o bloco nao aparece — marcar nada em
            lugar nenhum nao e escolha, e confusao. */}
        {activeWorkspaces.length ? <div className="resource-context"><span className="micro-label">CONTEXTO</span><div>{activeWorkspaces.map((workspace) => <label key={workspace.id}><input type="checkbox" checked={linkedWorkspaceIds.has(workspace.id)} onChange={(event) => void toggleWorkspace(workspace.id, event.currentTarget.checked)} /><span>{workspace.name}</span></label>)}</div></div> : null}
        {source ? <div className="provenance"><span className="micro-label">ORIGEM</span><button type="button" onClick={() => openCapture(source)}>{source.content}</button><small>{sourceLabel(source.source)} · {relativeTime(source.capturedAt)}</small></div> : null}
        {sourceError ? <p className="inline-error" role="status">Não foi possível carregar a Capture de origem agora.</p> : null}
        {arquivo ? <div className="resource-file"><span className="micro-label">ARQUIVO</span><dl>
          <div><dt>Tamanho</dt><dd>{fileSize(arquivo.byteSize)}</dd></div>
          <div><dt>Tipo</dt><dd>{arquivo.mime || "desconhecido"}</dd></div>
          {arquivo.imageSize ? <div><dt>Dimensões</dt><dd>{arquivo.imageSize.width} × {arquivo.imageSize.height}</dd></div> : null}
          <div><dt>Conteúdo</dt><dd>{extractionLabel(arquivo)}</dd></div>
        </dl></div> : null}
        <div className="detail-actions">
          {selected.url ? <Button variant="primary" onClick={() => void openLink(selected)} disabled={selected.lifecycleState !== "active" || pendingAction !== null}>{pendingAction === "open" ? "Abrindo" : "Abrir link"}</Button> : null}
          {/* O M/OS nao abre o que o Windows executaria. Quando ele se recusa,
              a mensagem diz o motivo e "Mostrar na pasta" continua ali: o
              arquivo e da pessoa, e chegar ate ele nunca deixa de ser possivel. */}
          {arquivo ? <>
            <Button variant="primary" onClick={() => void abrirArquivo(selected.id)} disabled={selected.lifecycleState !== "active" || pendingAction !== null}>Abrir arquivo</Button>
            <Button variant="ghost" onClick={() => void mostrarNaPasta(selected.id)}>Mostrar na pasta</Button>
          </> : null}
          <Button variant={selected.url || arquivo ? "ghost" : "primary"} onClick={() => setMode("edit")} disabled={pendingAction !== null}>Editar</Button>
        </div>
        {message ? <p className="settings-message" aria-live="polite">{message}</p> : null}
      </> : null}
    </Inspector>
  </div>;
}

function BoardPage({ tasks, projects, stale, refresh, openTask, intent }: { tasks: Task[]; projects: Project[]; stale: StaleView; refresh: () => Promise<void>; openTask: (task: Task) => void; intent?: FunctionIntent }) {
  const [creating, setCreating] = useState(false);
  const [draggingTaskId, setDraggingTaskId] = useState<string | null>(null);
  const [dragOverState, setDragOverState] = useState<TaskState | null>(null);
  const pointerDrag = useRef<{ taskId: string; x: number; y: number; active: boolean } | null>(null);
  const suppressClickTaskId = useRef<string | null>(null);
  const board = useRef<HTMLDivElement>(null);
  const activeTasks = tasks.filter((task) => task.lifecycleState === "active");
  /* Id da Task para dias parados. O quadro e onde se AGE: a marca fica ao lado
     do card que se arrasta, e nao numa lista a parte. */
  const diasParados = diasPorTask(stale.paradas);
  useEffect(() => {
    if (intent?.target === "tasks_create") setCreating(true);
    if (intent?.target === "tasks_move") window.requestAnimationFrame(() => board.current?.focus());
  }, [intent?.key]);
  async function move(task: Task, state: TaskState) { if (task.state !== state) await api.setTaskState(task.id, state).then(refresh); }
  function draggedTask(event: DragEvent<HTMLElement>) {
    const id = event.dataTransfer.getData("text/task-id") || event.dataTransfer.getData("text/plain") || draggingTaskId;
    return tasks.find((item) => item.id === id);
  }
  /* Encerra QUALQUER arrasto, incluindo o registro do ponteiro.
     Nao limpar `pointerDrag.current` aqui era o bug mais grave do kanban: depois
     de um arrasto nativo o registro ficava armado com a task antiga, o proximo
     movimento de mouse passava dos 6px e o marcava como ativo, e o clique
     seguinte — em qualquer lugar do quadro — movia aquela task para a coluna sob
     o cursor. A task andava sozinha. */
  function finishDrag() {
    pointerDrag.current = null;
    setDraggingTaskId(null);
    setDragOverState(null);
  }
  function keyboardMove(event: KeyboardEvent<HTMLButtonElement>, task: Task) {
    if (!event.altKey || (event.key !== "ArrowLeft" && event.key !== "ArrowRight")) return;
    event.preventDefault();
    const index = stateOrder.indexOf(task.state);
    const next = stateOrder[Math.max(0, Math.min(stateOrder.length - 1, index + (event.key === "ArrowRight" ? 1 : -1)))];
    void move(task, next);
  }
  /* Dois caminhos de arrasto, com hierarquia explicita.
     O nativo e o primario: da previa do cartao de graca e e o que o Windows
     espera. O do ponteiro e FALLBACK, acrescentado em ca5a831 porque o nativo
     nao dispara de forma confiavel dentro do WebView.
     A regra que faltava: quando o nativo comeca, ele desarma o registro do
     ponteiro no proprio onDragStart. Assim os dois nunca agem sobre o mesmo
     gesto — antes eles ficavam armados juntos e discordavam. */
  useEffect(() => {
    function columnFromPoint(x: number, y: number) {
      const column = document.elementFromPoint(x, y)?.closest<HTMLElement>(".kanban-column");
      const state = column?.dataset.kanbanState;
      return stateOrder.includes(state as TaskState) ? state as TaskState : null;
    }
    function handlePointerMove(event: PointerEvent) {
      const drag = pointerDrag.current;
      if (!drag) return;
      if (!drag.active && Math.hypot(event.clientX - drag.x, event.clientY - drag.y) < 6) return;
      drag.active = true;
      suppressClickTaskId.current = drag.taskId;
      setDraggingTaskId(drag.taskId);
      setDragOverState(columnFromPoint(event.clientX, event.clientY));
    }
    function handlePointerUp(event: PointerEvent) {
      const drag = pointerDrag.current;
      if (!drag) return;
      pointerDrag.current = null;
      if (!drag.active) return;
      const targetState = columnFromPoint(event.clientX, event.clientY);
      const task = tasks.find((item) => item.id === drag.taskId);
      finishDrag();
      if (task && targetState) void move(task, targetState);
      window.setTimeout(() => { if (suppressClickTaskId.current === drag.taskId) suppressClickTaskId.current = null; }, 0);
    }
    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
    window.addEventListener("pointercancel", finishDrag);
    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("pointercancel", finishDrag);
    };
  }, [tasks, refresh]);
  return <div className="page board-page" data-empty={!activeTasks.length && !creating || undefined}>
    <header className="board-heading">
      <ContextPath segments={["M", "TASKS"]} />
      <div className="board-heading-actions">
        {activeTasks.length ? <span className="micro-label">{activeTasks.length} {activeTasks.length === 1 ? "TASK ATIVA" : "TASKS ATIVAS"}</span> : null}
        {/* A alternativa ao arrasto precisa estar perto do quadro, sem se colar
            ao breadcrumb como se fizesse parte da localizacao da pagina. */}
        {activeTasks.length ? <span className="board-hint" aria-label="Mover Task com Alt e seta para a esquerda ou direita"><kbd>ALT</kbd><span aria-hidden="true">← →</span><span>MOVER</span></span> : null}
        {!creating && activeTasks.length ? <Button variant="primary" onClick={() => setCreating(true)}>Criar Task</Button> : null}
      </div>
    </header>
    {creating ? <section className="task-create-panel" aria-label="Nova Task"><span className="micro-label">NOVA TASK</span><DirectTaskForm projects={projects} cancel={() => setCreating(false)} saved={() => { setCreating(false); void refresh(); }} /></section> : null}
    {!activeTasks.length && !creating ? <section className="tasks-empty">
      <span className="micro-label">0 TASKS ATIVAS</span>
      <h1>Nenhuma Task ativa.</h1>
      <p>Crie uma Task para começar a organizar o trabalho no quadro.</p>
      <Button variant="primary" onClick={() => setCreating(true)}>Criar Task</Button>
    </section> : null}
    {activeTasks.length ? <div ref={board} className="kanban" tabIndex={-1} aria-label="Kanban de Tasks">{stateOrder.map((state) => {
      const column = activeTasks.filter((task) => task.state === state);
      const visible = column.slice(0, 20);
      return <section key={state} className="kanban-column" data-kanban-state={state} data-drop-target={dragOverState === state || undefined} onDragEnter={(event) => { event.preventDefault(); setDragOverState(state); }} onDragOver={(event) => { event.preventDefault(); event.dataTransfer.dropEffect = "move"; setDragOverState(state); }} onDragLeave={(event) => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDragOverState(null); }} onDrop={(event) => { event.preventDefault(); const task = draggedTask(event); finishDrag(); if (task) void move(task, state); }}>
        <header><h2>{stateLabels[state]}</h2><span>{column.length}</span></header>
        <AnimatedList>{visible.map((task) => <AnimatedListItem key={task.id} itemKey={task.id}><DataRow primary={task.title} secondary={projects.find((project) => project.id === task.projectId)?.name} meta={rotuloDeDias(diasParados.get(task.id) ?? 0)} stale={diasParados.has(task.id)} completed={task.state === "done"} dragging={draggingTaskId === task.id} onClick={() => { if (suppressClickTaskId.current === task.id) { suppressClickTaskId.current = null; return; } openTask(task); }} onKeyDown={(event) => keyboardMove(event, task)} onPointerDown={(event) => { if (event.button !== 0) return; pointerDrag.current = { taskId: task.id, x: event.clientX, y: event.clientY, active: false }; }} draggable onDragStart={(event) => { pointerDrag.current = null; setDraggingTaskId(task.id); event.dataTransfer.effectAllowed = "move"; event.dataTransfer.setData("text/task-id", task.id); event.dataTransfer.setData("text/plain", task.id); }} onDragEnd={finishDrag} /></AnimatedListItem>)}{!column.length ? <p className="kanban-empty">Vazio</p> : null}{column.length > visible.length ? <p className="more-count">+ {column.length - visible.length} mais</p> : null}</AnimatedList>
      </section>;
    })}</div> : null}
  </div>;
}

function TaskDrawer({ task, projects, close, refresh, receipt, openCapture }: { task: Task; projects: Project[]; close: () => void; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; openCapture: (capture: Capture) => void }) {
  const [title, setTitle] = useState(task.title);
  const [description, setDescription] = useState(task.description);
  const [projectId, setProjectId] = useState(task.projectId ?? "");
  const [state, setState] = useState(task.state);
  const [source, setSource] = useState<Capture | null>(null);
  const [error, setError] = useState("");
  const [pending, setPending] = useState<"save" | "archive" | null>(null);
  const drawer = useRef<HTMLElement>(null);
  const titleInput = useRef<HTMLInputElement>(null);
  const returnFocus = useRef<HTMLElement | null>(document.activeElement instanceof HTMLElement ? document.activeElement : null);
  useEffect(() => {
    titleInput.current?.focus();
    if (task.sourceCaptureId) void api.getCapture(task.sourceCaptureId).then(setSource);
    return () => {
      const target = returnFocus.current;
      if (target?.isConnected) requestAnimationFrame(() => target.focus());
    };
  }, [task.sourceCaptureId]);
  async function submit(event: FormEvent) {
    event.preventDefault();
    setPending("save");
    setError("");
    try { await api.updateTask(task.id, title, description, projectId || null); if (state !== task.state) await api.setTaskState(task.id, state); await refresh(); close(); }
    catch (nextError) { setPending(null); setError(appError(nextError).message); }
  }
  async function archive() {
    setPending("archive");
    setError("");
    try {
      await api.setTaskArchived(task.id, true);
      receipt({ message: "Task arquivada.", run: () => api.setTaskArchived(task.id, false) });
      await refresh();
      close();
    } catch (nextError) {
      setPending(null);
      setError(appError(nextError).message);
    }
  }
  return <LazyMotion features={loadMotionFeatures} strict>
    <m.aside
      ref={drawer}
      className="task-drawer"
      aria-label="Detalhe da Task"
      aria-busy={pending !== null}
      tabIndex={-1}
      initial={{ opacity: 0, x: 24 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: 24 }}
      transition={{ duration: MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
      onKeyDown={(event) => { if (event.key === "Escape" && !pending) close(); }}
    >
      <header><span className="micro-label">DETALHE DA TASK</span><IconButton label="Fechar" icon="close" disabled={pending !== null} onClick={close} /></header>
      <form className="stack-form" onSubmit={submit}>
        <label><span>TÍTULO</span><input ref={titleInput} value={title} onChange={(event) => setTitle(event.currentTarget.value)} disabled={pending !== null} /></label>
        <label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={3} disabled={pending !== null} /></label>
        <label><span>PROJECT</span><select value={projectId} onChange={(event) => setProjectId(event.currentTarget.value)} disabled={pending !== null}><option value="">Sem Project</option>{projects.filter((project) => project.lifecycleState === "active").map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}</select></label>
        <label><span>ESTADO</span><select value={state} onChange={(event) => setState(event.currentTarget.value as TaskState)} disabled={pending !== null}>{stateOrder.map((value) => <option key={value} value={value}>{stateLabels[value]}</option>)}</select></label>
        {source ? <div className="provenance"><span className="micro-label">ORIGEM</span><button type="button" onClick={() => openCapture(source)}>{source.content}</button><small>{sourceLabel(source.source)} · {relativeTime(source.capturedAt)}</small></div> : null}
        {pending === "save" ? <StateMessage state="saving" label="Salvando Task..." /> : pending === "archive" ? <StateMessage state="saving" label="Arquivando Task..." /> : error ? <StateMessage state="error" label={error} /> : null}
        <div className="form-actions spread"><Button variant="danger" onClick={() => void archive()} disabled={pending !== null}>{pending === "archive" ? "Arquivando" : "Arquivar"}</Button><Button variant="primary" type="submit" disabled={!title.trim() || pending !== null}>{pending === "save" ? "Salvando" : "Salvar"}</Button></div>
      </form>
    </m.aside>
  </LazyMotion>;
}

function CaptureViewer({ capture, close }: { capture: Capture; close: () => void }) {
  const dialog = useRef<HTMLElement>(null);
  useEffect(() => dialog.current?.focus(), []);
  return <LazyMotion features={loadMotionFeatures} strict>
    <m.div
      className="overlay-backdrop"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: MOTION_DURATIONS.enter }}
      onMouseDown={(event) => { if (event.target === event.currentTarget) close(); }}
    >
      <m.article
        ref={dialog}
        className="entity-viewer"
        role="dialog"
        aria-modal="true"
        tabIndex={-1}
        initial={{ opacity: 0, scale: 0.98, y: -4 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.98, y: -2 }}
        transition={{ duration: MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
        onKeyDown={(event) => { if (event.key === "Escape") close(); }}
      >
        <header><span className="micro-label">CAPTURE</span><IconButton label="Fechar" icon="close" onClick={close} /></header>
        <h1>{capture.content}</h1>
        <dl>
          <div><dt>ORIGEM</dt><dd>{sourceLabel(capture.source)}</dd></div>
          <div><dt>ESTADO</dt><dd>{capture.lifecycleState === "archived" ? "Arquivada" : capture.processingState === "processed" ? "Processada" : "Na Inbox"}</dd></div>
          <div><dt>CAPTURADA</dt><dd>{new Date(capture.capturedAt).toLocaleString("pt-BR")}</dd></div>
        </dl>
      </m.article>
    </m.div>
  </LazyMotion>;
}

/* O Command NAO tem mais modo Hermes.
 *
 * Ele e esta tela assinavam `hermes-event` no mesmo barramento global, e o
 * Command monta POR CIMA da pagina — com as duas abertas, os deltas da mesma
 * resposta se dividiam entre dois estados independentes. Alem do defeito, eram
 * duas implementacoes da mesma thread.
 *
 * A divisao agora segue `UX-PRINCIPLES.md` §13: o Command encontra e executa, a
 * pagina Hermes conversa. Quem quer perguntar vai para a pagina — que e onde a
 * conversa fica guardada. */
function CommandSurface({ close, closing = false, openCapture, openTask, openProject, openWorkspace, openApp, openResource, openDailySession, routeFunction }: {
  closing?: boolean; close: () => void; openCapture: (capture: Capture) => void; openTask: (task: Task) => void; openProject: (project: Project) => void; openWorkspace: (workspace: Workspace) => void; openApp: (app: RegisteredApp) => void; openResource: (resource: Resource) => void; openDailySession: (sessionId: string) => void; routeFunction: (definition: FunctionDefinition) => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<CommandResult[]>([]);
  const [includeArchived, setIncludeArchived] = useState(false);
  const [error, setError] = useState("");
  const [searching, setSearching] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  // O desfoque de borda so aparece quando ha conteudo abaixo do corte. Sem esta
  // medida ele seria decoracao: uma nevoa sobre espaco vazio, dizendo que ha
  // mais quando nao ha. Mede em scroll e resize, sem cronometro e sem RAF.
  const resultsPane = useRef<HTMLDivElement>(null);
  const [hasMoreBelow, setHasMoreBelow] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const previousFocus = useRef(document.activeElement as HTMLElement | null);
  const searchSequence = useRef(0);
  useEffect(() => { input.current?.focus(); return () => previousFocus.current?.focus(); }, []);
  useEffect(() => {
    const pane = resultsPane.current;
    if (!pane) { setHasMoreBelow(false); return; }
    const measure = () => setHasMoreBelow(pane.scrollTop + pane.clientHeight < pane.scrollHeight - 1);
    measure();
    pane.addEventListener("scroll", measure, { passive: true });
    const resize = new ResizeObserver(measure);
    resize.observe(pane);
    return () => { pane.removeEventListener("scroll", measure); resize.disconnect(); };
  }, [results.length, error]);

  async function searchCommand(requestId: number) {
    try {
      const [items, resources, functions] = await Promise.all([api.search(query, includeArchived), api.searchResources(query, includeArchived), api.searchFunctions(query)]);
      if (requestId !== searchSequence.current) return;
      setResults([...items, ...resources.map((resource) => ({ kind: "resource" as const, resource })), ...functions.map((definition) => ({ kind: "function" as const, function: definition }))]);
      setActiveIndex(0);
      setError("");
    } catch (nextError) {
      if (requestId !== searchSequence.current) return;
      setResults([]);
      setError(appError(nextError).message);
    } finally {
      if (requestId === searchSequence.current) setSearching(false);
    }
  }
  useEffect(() => {
    const requestId = ++searchSequence.current;
    setResults([]);
    setActiveIndex(0);
    setError("");
    setSearching(Boolean(query.trim()));
    if (!query.trim()) {
      return;
    }
    const timeout = window.setTimeout(() => void searchCommand(requestId), 80);
    return () => window.clearTimeout(timeout);
  }, [query, includeArchived]);
  useEffect(() => { document.getElementById(`command-result-${activeIndex}`)?.scrollIntoView({ block: "nearest" }); }, [activeIndex]);
  function openItem(item: CommandResult) {
    close();
    if (item.kind === "function") routeFunction(item.function);
    else if (item.kind === "project") openProject(item.project);
    else if (item.kind === "workspace") openWorkspace(item.workspace);
    else if (item.kind === "task") openTask(item.task);
    else if (item.kind === "app") openApp(item.app);
    else if (item.kind === "resource") openResource(item.resource);
    // Um objetivo abre O DIA dele, e nao ele sozinho: "o que eu estava fazendo
    // terca?" tem como resposta o dia inteiro, e um objetivo fora do dia dele
    // nao responde nada.
    else if (item.kind === "daily_objective") openDailySession(item.objective.sessionId);
    else if (item.derivedTask) openTask(item.derivedTask);
    else openCapture(item.capture);
  }
  function handleInputKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.nativeEvent.isComposing) return;
    // `Tab` volta a mover foco estrutural (`DESIGN-FOUNDATIONS.md` §12). Ele
    // trocava de modo, e o modo deixou de existir aqui.
    if (!results.length) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const direction = event.key === "ArrowDown" ? 1 : -1;
      setActiveIndex((current) => (current + direction + results.length) % results.length);
    }
    if (event.key === "Home" || event.key === "End") {
      event.preventDefault();
      setActiveIndex(event.key === "Home" ? 0 : results.length - 1);
    }
    if (event.key === "Enter") {
      event.preventDefault();
      openItem(results[activeIndex] ?? results[0]);
    }
  }
  return <LazyMotion features={loadMotionFeatures} strict>
    <m.div
      className="overlay-backdrop command-backdrop"
      data-closing={closing || undefined}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: MOTION_DURATIONS.enter }}
      onMouseDown={(event) => { if (event.target === event.currentTarget) close(); }}
    >
      <m.section
        className="command-surface"
        role="dialog"
        aria-modal="true"
        aria-label="Command"
        initial={{ opacity: 0, scale: 0.98, y: -6 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.98, y: -4 }}
        transition={{ duration: MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
        onKeyDown={(event) => { if (event.key === "Escape") close(); }}
      >
        <div className="command-input"><span className="slash">/</span><input ref={input} role="combobox" aria-autocomplete="list" aria-expanded={results.length > 0} aria-controls="command-results" aria-activedescendant={results.length ? `command-result-${activeIndex}` : undefined} value={query} onChange={(event) => setQuery(event.currentTarget.value)} onKeyDown={handleInputKeyDown} placeholder="Buscar ou executar comando" aria-label="Buscar no M/OS" spellCheck={false} autoCorrect="off" autoCapitalize="off" /><span className="micro-label">COMMAND</span></div>
        {query ? <div className="command-options"><label className="check-control"><input type="checkbox" checked={includeArchived} onChange={(event) => setIncludeArchived(event.currentTarget.checked)} /><span>Incluir arquivados</span></label><span className="command-status" role="status" aria-live="polite">{searching ? "BUSCANDO" : error ? "FALHA" : `${results.length} ${results.length === 1 ? "RESULTADO" : "RESULTADOS"}`}</span></div> : null}
        <div ref={resultsPane} id="command-results" className="command-results" role={results.length ? "listbox" : undefined} aria-label="Resultados" aria-busy={searching}>
          {error ? <div className="command-error"><p>! {error}</p><Button variant="outline" onClick={() => { setSearching(true); void searchCommand(++searchSequence.current); }}>Tentar novamente</Button></div> : null}
          {!query ? <div className="command-prompt"><span className="micro-label">ENCONTRAR E EXECUTAR</span><p>Busque Tasks, Projects, Captures, Resources, Apps e comandos.</p></div> : null}
          {query && !searching && !error && !results.length ? <div className="command-prompt"><span className="micro-label">SEM RESULTADOS</span><p>Nada corresponde a “{query}”.</p></div> : null}
          {results.map((item, index) => {
            const type = item.kind === "function" ? "FUNCTION" : item.kind === "project" ? "PROJECT" : item.kind === "workspace" ? "WORKSPACE" : item.kind === "task" ? "TASK" : item.kind === "app" ? "APP" : item.kind === "resource" ? "RESOURCE" : item.kind === "daily_objective" ? "OBJETIVO" : item.derivedTask ? "TASK + CAPTURE" : "CAPTURE";
            const title = item.kind === "function" ? item.function.name : item.kind === "project" ? item.project.name : item.kind === "workspace" ? item.workspace.name : item.kind === "task" ? item.task.title : item.kind === "app" ? item.app.name : item.kind === "resource" ? item.resource.title : item.kind === "daily_objective" ? item.objective.title : item.derivedTask?.title ?? item.capture.content;
            const context = item.kind === "function" ? `${item.function.id} · risco ${functionRiskLabels[item.function.risk]}` : item.kind === "project" ? item.project.description : item.kind === "workspace" ? item.workspace.description : item.kind === "task" ? item.project?.name : item.kind === "app" ? item.app.description || item.app.launchTarget || "" : item.kind === "resource" ? `${resourceHost(item.resource.url)}${item.resource.note ? ` · ${item.resource.note}` : ""}` : item.kind === "daily_objective" ? dataPorExtenso(item.day) : item.project?.name ?? item.capture.content;
            return <button id={`command-result-${index}`} role="option" aria-selected={index === activeIndex} data-active={index === activeIndex || undefined} key={`${item.kind}-${index}-${title}`} className="command-row" onFocus={() => setActiveIndex(index)} onMouseEnter={() => setActiveIndex(index)} onClick={() => openItem(item)}><span>{type}</span><strong>{title}</strong><small>{context}</small></button>;
          })}
        </div>
        {/* Tres camadas de desfoque crescente, ancoradas acima do rodape. So
            existem enquanto houver resultado abaixo do corte. */}
        {hasMoreBelow ? <div className="command-fade" aria-hidden="true"><i /><i /><i /></div> : null}
        <div className="command-footer">{["↑↓ NAVEGA", "⏎ ABRE", "/ COMANDO", "ESC FECHA"].map((hint) => <span key={hint}>{hint}</span>)}</div>
      </m.section>
    </m.div>
  </LazyMotion>;
}

/** A integração com o Univirtus.
 *
 *  O botão Conectar NÃO abre um formulário de RU e senha: ele abre a página
 *  oficial da UNINTER numa janela do app, e o M/OS recolhe de lá só o que a API
 *  exige. É o que a investigação mediu — não existe endpoint que troque
 *  credencial por token (`docs/UNIVIRTUS-INTEGRATION.md` §2) —, e o efeito é que
 *  a senha de ninguém passa por aqui.
 *
 *  O estado nunca é escondido: quem sincroniza um portal externo precisa saber
 *  quando foi a última vez e se ainda está conectado, senão passa a confiar em
 *  dados velhos sem perceber. */
function UnivirtusSettings() {
  const [status, setStatus] = useState<UnivirtusStatus | null>(null);
  const [busy, setBusy] = useState<"idle" | "connecting" | "syncing">("idle");
  const [message, setMessage] = useState("");
  const [messageState, setMessageState] = useState<"loading" | "saved" | "error">("saved");
  const [report, setReport] = useState<SyncReport | null>(null);

  const load = useCallback(async () => {
    try { setStatus(await api.univirtusStatus()); } catch { /* a tela sobrevive sem o estado */ }
  }, []);
  useEffect(() => { void load(); }, [load]);

  function notify(state: "loading" | "saved" | "error", text: string) {
    setMessageState(state);
    setMessage(text);
  }

  async function connect() {
    setBusy("connecting");
    notify("loading", "Entre no Univirtus na janela que abriu. O M/OS espera.");
    try {
      setStatus(await api.univirtusConnect());
      notify("saved", "Conectado. Sincronize para trazer o semestre.");
    } catch (error) { notify("error", appError(error).message); }
    finally { setBusy("idle"); }
  }

  async function sync() {
    setBusy("syncing");
    notify("loading", "Sincronizando com o Univirtus...");
    try {
      const next = await api.univirtusSync();
      setReport(next);
      const resumo = resumoDoSync(next);
      notify(next.outcome === "completed" ? "saved" : "error", resumo);
      await load();
    } catch (error) { notify("error", appError(error).message); await load(); }
    finally { setBusy("idle"); }
  }

  async function disconnect() {
    try {
      await api.univirtusDisconnect();
      setReport(null);
      notify("saved", "Desconectado. O que já foi sincronizado continua no M/Academic.");
      await load();
    } catch (error) { notify("error", appError(error).message); }
  }

  const conectado = status?.connection === "connected";
  const expirado = status?.connection === "expired";
  const estadoLabel = conectado ? "Conectado" : expirado ? "Sessão expirada" : "Desconectado";

  return <Panel label="UNIVIRTUS">
    <p className="support-copy">
      A faculdade como fonte de dados, e não como um segundo aplicativo. O M/OS lê disciplinas,
      prazos, notas e materiais — e nunca escreve nada no portal: não entrega trabalho, não inicia
      prova e não marca conteúdo como acessado.
    </p>
    <p className="support-copy">
      Conectar abre a página oficial da UNINTER numa janela. Você entra lá; o M/OS não pede nem
      guarda sua senha.
    </p>

    <dl className="fact-grid">
      <div><dt>ESTADO</dt><dd>{estadoLabel}</dd></div>
      <div><dt>CURSO</dt><dd>{status?.courseName || <span className="fact-empty">—</span>}</dd></div>
      <div><dt>DISCIPLINAS</dt><dd>{status?.tracked?.subject ?? <span className="fact-empty">—</span>}</dd></div>
      <div><dt>ÚLTIMA SINCRONIZAÇÃO</dt><dd>{status?.lastSyncAt ? relativeTime(status.lastSyncAt) : <span className="fact-empty">Nunca</span>}</dd></div>
    </dl>

    {expirado ? <p className="support-copy">
      A sessão do Univirtus caiu — elas não se renovam sozinhas. Os dados já sincronizados
      continuam no M/Academic; reconecte quando quiser trazer o que mudou.
    </p> : null}

    {report?.warnings?.length ? <ul className="academic-lista">
      {report.warnings.map((aviso: string) => <li key={aviso} className="support-copy">{aviso}</li>)}
    </ul> : null}

    <div className="button-line">
      {conectado
        ? <Button variant="primary" onClick={() => void sync()} disabled={busy !== "idle"}>
            {busy === "syncing" ? "Sincronizando" : "Sincronizar agora"}
          </Button>
        : <Button variant="primary" onClick={() => void connect()} disabled={busy !== "idle"}>
            {busy === "connecting" ? "Aguardando login" : expirado ? "Reconectar" : "Conectar"}
          </Button>}
      {status?.hasSession || conectado || expirado
        ? <Button variant="ghost" onClick={() => void disconnect()} disabled={busy !== "idle"}>Desconectar</Button>
        : null}
    </div>
    {message ? <StateMessage state={messageState} label={message} /> : null}
  </Panel>;
}

/** A frase de um sync. Vazia não existe aqui: mesmo "tudo em dia" precisa
 *  responder ao clique, senão o botão parece morto. */
function resumoDoSync(report: SyncReport): string {
  const partes: string[] = [];
  const add = (n: number, singular: string, plural: string) => {
    if (n > 0) partes.push(`+${n} ${n === 1 ? singular : plural}`);
  };
  add(report.subjects.created, "disciplina", "disciplinas");
  add(report.assessments.created, "avaliação", "avaliações");
  add(report.assignments.created, "trabalho", "trabalhos");
  add(report.materials.created, "material", "materiais");
  const atualizados = report.assessments.updated + report.assignments.updated;
  if (atualizados > 0) partes.push(`~${atualizados} ${atualizados === 1 ? "atualizado" : "atualizados"}`);
  const sumiram = report.assessments.unavailable + report.assignments.unavailable;
  if (sumiram > 0) partes.push(`${sumiram} fora do portal (mantido)`);
  if (!partes.length) return "Tudo em dia. Nada mudou no Univirtus.";
  return partes.join(" · ");
}

function HermesSettings() {
  const [status, setStatus] = useState<HermesStatus | null>(null);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [message, setMessage] = useState("");
  const [messageState, setMessageState] = useState<"saving" | "saved" | "error">("saved");
  useEffect(() => {
    void hermes.status().then((next) => { setStatus(next); setBaseUrl(next.baseUrl); }).catch(() => undefined);
    const subscription = hermes.onState(setStatus);
    return () => { void subscription.then((dispose) => dispose()); };
  }, []);

  async function save(event: FormEvent) {
    event.preventDefault();
    setMessageState("saving");
    setMessage("Salvando conexão...");
    try {
      if (baseUrl.trim()) await hermes.setBaseUrl(baseUrl);
      // O provider `basic` do Hermes exige usuario E senha (o config.yaml
      // declara username como required). Antes, faltando um dos dois, a
      // chamada simplesmente nao acontecia — e a mensagem de sucesso aparecia
      // mesmo assim, afirmando ter guardado o que nunca foi guardado. Quem
      // preenchia so a senha clicava em Salvar, lia "Credencial guardada" e
      // ficava Offline para sempre, sem nada na tela explicando.
      const wantsCredential = username.trim().length > 0 || password.length > 0;
      if (wantsCredential && !(username.trim() && password)) {
        setMessage(username.trim()
          ? "Falta a senha. O Hermes exige usuário e senha."
          : "Falta o usuário. O Hermes exige usuário e senha — normalmente o mesmo login do dashboard.");
        return;
      }
      if (wantsCredential) {
        await hermes.setCredentials(username, password);
        // A senha some da memoria do renderer assim que sai daqui. Ela vive no
        // Credential Manager, e nem o proprio campo a mantem.
        setPassword("");
        setMessage("Credencial guardada no Windows Credential Manager.");
        // Conectar agora. O supervisor do raiz desiste em silencio quando nao ha
        // credencial, e nada o reagenda: sem este empurrao, guardar a senha nao
        // produziria efeito nenhum ate reabrir o app.
        void hermes.connect().catch(() => undefined);
      } else {
        setMessage("Endereço salvo.");
      }
      setStatus(await hermes.status());
      setMessageState("saved");
    } catch (error) { setMessageState("error"); setMessage(String(error)); }
  }

  const stateLabel = status?.state === "online" ? "Conectado" : status?.state === "connecting" ? "Conectando" : "Desconectado";
  return <Panel label="HERMES">
    <p className="support-copy">O M/OS é mais uma superfície do Hermes que já roda na sua VPS — a mesma que você usa pelo WhatsApp, numa conversa separada. O acesso é pelo túnel SSH; o M/OS não abre porta nem inicia o túnel.</p>
    <form className="stack-form" onSubmit={save}>
      <label><span>ENDEREÇO LOCAL</span><input className="mono-input" value={baseUrl} onChange={(event) => setBaseUrl(event.currentTarget.value)} placeholder="http://127.0.0.1:9119" /></label>
      <label><span>USUÁRIO</span><input value={username} onChange={(event) => setUsername(event.currentTarget.value)} autoComplete="off" /></label>
      <label><span>SENHA</span><input type="password" value={password} onChange={(event) => setPassword(event.currentTarget.value)} autoComplete="off" /></label>
      <div className="form-actions">
        <Button variant="ghost" onClick={() => void hermes.clearCredentials().then(() => hermes.status()).then(setStatus).catch(() => undefined)}>Remover credencial</Button>
        <Button variant="primary" type="submit">Salvar</Button>
      </div>
    </form>
    <dl className="fact-grid">
      <div><dt>ESTADO</dt><dd>{stateLabel}</dd></div>
      <div><dt>CREDENCIAL</dt><dd>{status?.hasCredentials ? "Configurada" : <span className="fact-empty">Não configurada</span>}</dd></div>
    </dl>
    {status?.detail ? <p className="support-copy">{status.detail}</p> : null}
    {message ? <StateMessage state={messageState} label={message} /> : null}
  </Panel>;
}

function FinanceActionSettings() {
  const [configured, setConfigured] = useState(false);
  const [secret, setSecret] = useState("");
  const [message, setMessage] = useState("");
  const [messageState, setMessageState] = useState<"saving" | "saved" | "error">("saved");

  useEffect(() => {
    void finance.actionSecretConfigured().then(setConfigured).catch(() => undefined);
  }, []);

  async function save(event: FormEvent) {
    event.preventDefault();
    if (!secret.trim()) return;
    setMessageState("saving");
    setMessage("Salvando secret...");
    try {
      await finance.setActionSecret(secret);
      setSecret("");
      setConfigured(true);
      setMessage("Secret guardado no Windows Credential Manager.");
      setMessageState("saved");
    } catch (error) {
      setMessageState("error");
      setMessage(String(error));
    }
  }

  async function clear() {
    await finance.clearActionSecret().catch(() => undefined);
    setConfigured(false);
  }

  return (
    <Panel label="AÇÕES DO HERMES NO M-FINANCE">
      <p className="support-copy">
        O Hermes pode propor criar contas no M-Finance quando você pedir — nunca sem confirmação
        explícita. Isto guarda o secret que autoriza o M/OS a chamar a Action API do M-Finance
        (mesmo secret configurado como variável de ambiente lá, do lado do M-Finance).
      </p>
      <form className="stack-form" onSubmit={save}>
        <label><span>SECRET</span><input type="password" value={secret} onChange={(event) => setSecret(event.currentTarget.value)} autoComplete="off" /></label>
        <div className="form-actions">
          <Button variant="ghost" onClick={() => void clear()}>Remover secret</Button>
          <Button variant="primary" type="submit">Salvar</Button>
        </div>
      </form>
      <dl className="fact-grid">
        <div><dt>SECRET</dt><dd>{configured ? "Configurado" : <span className="fact-empty">Não configurado</span>}</dd></div>
      </dl>
      {message ? <StateMessage state={messageState} label={message} /> : null}
    </Panel>
  );
}

/**
 * Iniciar com o Windows (ADR-043).
 *
 * O toggle de cima PERGUNTA AO SISTEMA a cada vez que a tela abre, em vez de
 * espelhar uma configuração nossa. O `auto-launch` grava também na chave que o
 * Gerenciador de Tarefas usa, e o usuário pode desligar por lá sem nos avisar —
 * um booleano nosso divergiria no primeiro clique feito fora daqui, e a tela
 * passaria a afirmar "ligado" sobre algo desligado.
 *
 * O de baixo é preferência nossa e mora em settings.json: o Windows sabe iniciar
 * o programa, não com que cara.
 */
function StartupSettings() {
  const [enabled, setEnabled] = useState(false);
  const [minimized, setMinimized] = useState(false);
  const [message, setMessage] = useState("");
  const [messageState, setMessageState] = useState<"saving" | "saved" | "error">("saved");

  const load = useCallback(async () => {
    try {
      const [system, ours] = await Promise.all([api.autostartEnabled(), api.startMinimized()]);
      setEnabled(system);
      setMinimized(ours);
    } catch (error) {
      setMessageState("error");
      setMessage(appError(error).message);
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  async function toggleAutostart(next: boolean) {
    setMessageState("saving");
    setMessage(next ? "Ligando..." : "Desligando...");
    try {
      // O backend devolve o que o SISTEMA passou a dizer, e nao o que foi
      // pedido: se a gravacao no registro nao pegar, a tela mostra a verdade.
      setEnabled(await api.setAutostart(next));
      setMessageState("saved");
      setMessage(next ? "O M/OS vai iniciar com o Windows." : "O M/OS nao inicia sozinho.");
    } catch (error) {
      setMessageState("error");
      setMessage(appError(error).message);
      void load();
    }
  }

  async function toggleMinimized(next: boolean) {
    try {
      setMinimized(await api.setStartMinimized(next));
    } catch (error) {
      setMessageState("error");
      setMessage(appError(error).message);
    }
  }

  return (
    <Panel label="INICIALIZAÇÃO">
      <p className="support-copy">
        Lembretes só disparam com o M/OS aberto. Ligando isto, ele sobe junto com o Windows e
        continua no tray — sem isso, um lembrete das 9h não avisa se você abrir o app às 11h.
      </p>
      <div className="setting-row">
        <div>
          <strong>Iniciar com o Windows</strong>
          <p>Pode ser desligado também pelo Gerenciador de Tarefas, na aba Inicializar.</p>
        </div>
        <label className="switch">
          <input
            aria-label="Iniciar com o Windows"
            checked={enabled}
            onChange={(event) => void toggleAutostart(event.currentTarget.checked)}
            type="checkbox"
          />
          <span />
        </label>
      </div>
      <div className="setting-row">
        <div>
          <strong>Iniciar minimizado</strong>
          <p>Sobe direto para o tray, sem abrir a janela. Só vale quando o item acima está ligado.</p>
        </div>
        <label className="switch">
          <input
            aria-label="Iniciar minimizado"
            checked={minimized}
            disabled={!enabled}
            onChange={(event) => void toggleMinimized(event.currentTarget.checked)}
            type="checkbox"
          />
          <span />
        </label>
      </div>
      {message ? <StateMessage state={messageState} label={message} /> : null}
    </Panel>
  );
}

function SettingsPage({ theme, setTheme, status, capturesArchived, capturesTrashed, projects, tasks, workspaces, apps, resources, trashedResources, refresh, intent }: { theme: Theme; setTheme: (theme: Theme) => void; status: AppStatus | null; capturesArchived: Capture[]; capturesTrashed: Capture[]; projects: Project[]; tasks: Task[]; workspaces: Workspace[]; apps: RegisteredApp[]; resources: Resource[]; trashedResources: Resource[]; refresh: () => Promise<void>; intent?: FunctionIntent }) {
  const [shortcut, setShortcut] = useState("Ctrl+Shift+Space");
  const [voiceShortcut, setVoiceShortcut] = useState("Ctrl+Alt+G");
  const [message, setMessage] = useState("");
  const [messageState, setMessageState] = useState<"saved" | "error">("saved");
  const [inspection, setInspection] = useState<BackupInspection | null>(null);
  const [restorePath, setRestorePath] = useState("");
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateState, setUpdateState] = useState<"idle" | "checking" | "current" | "available" | "installing" | "error">("idle");
  // A resposta do botao precisa morar ao lado do botao. Ela caia em `message`,
  // que renderiza no rodape da pagina — depois de Archive/Trash e Integridade,
  // varios scrolls abaixo de quem acabou de clicar. Estar atualizado e a
  // resposta MAIS comum, e era justamente a invisivel: clicar e nao ver nada
  // acontecer se le como botao morto.
  const [updateNote, setUpdateNote] = useState("");
  const [importing, setImporting] = useState(false);
  const [importReport, setImportReport] = useState<ImportReport | null>(null);
  const [importNote, setImportNote] = useState("");
  // Pergunta ao banco, e não à memória da sessão: fechar e reabrir o app não
  // deveria reabilitar um botão que não pode mais ser clicado.
  const [importedAt, setImportedAt] = useState<string | null>(null);
  useEffect(() => { void api.cronocadImportedAt().then(setImportedAt).catch(() => undefined); }, []);
  const [updateProgress, setUpdateProgress] = useState<UpdateProgress>({ downloaded: 0, total: null });
  const [functions, setFunctions] = useState<FunctionDefinition[]>([]);
  const dialog = useRef<HTMLDialogElement>(null);
  // Exclusao definitiva nao tem Undo, entao nao pode seguir a regra de
  // "executar e oferecer desfazer" que vale no resto do app (UX-PRINCIPLES 21).
  // Aqui vale a outra: acao destrutiva e inequivoca (UX-PRINCIPLES 54). O
  // dialogo nomeia o item e diz que o caminho de volta e o backup anterior.
  const deleteDialog = useRef<HTMLDialogElement>(null);
  const [pendingDelete, setPendingDelete] = useState<{ noun: string; label: string; run: () => Promise<unknown> } | null>(null);
  function notify(state: "saved" | "error", nextMessage: string) {
    setMessageState(state);
    setMessage(nextMessage);
  }
  function askDelete(noun: string, label: string, run: () => Promise<unknown>) {
    setPendingDelete({ noun, label, run });
    deleteDialog.current?.showModal();
  }
  async function confirmDelete() {
    const target = pendingDelete;
    deleteDialog.current?.close();
    setPendingDelete(null);
    if (!target) return;
    try {
      await target.run();
      notify("saved", `${target.noun} excluído definitivamente.`);
      await refresh();
    } catch (error) { notify("error", appError(error).message); }
  }
  useEffect(() => { void api.functions().then(setFunctions).catch((error) => notify("error", appError(error).message)); }, []);
  async function backup() { const path = await save({ defaultPath: "m-os-backup.mos-backup", filters: [{ name: "M/OS Backup", extensions: ["mos-backup"] }] }); if (path) void api.createBackup(path).then((receipt) => notify("saved", `Backup criado: ${receipt.path}`)).catch((error) => notify("error", appError(error).message)); }
  async function exportData() { const path = await save({ defaultPath: "m-os-export.json", filters: [{ name: "JSON", extensions: ["json"] }] }); if (path) void api.exportJson(path).then((receipt) => notify("saved", `Export criado: ${receipt.path}`)).catch((error) => notify("error", appError(error).message)); }
  async function chooseRestore() { const path = await open({ multiple: false, filters: [{ name: "M/OS Backup", extensions: ["mos-backup"] }] }); if (!path) return; try { setInspection(await api.inspectBackup(path)); setRestorePath(path); dialog.current?.showModal(); } catch (error) { notify("error", appError(error).message); } }
  async function confirmRestore() { try { const safety = await api.restoreBackup(restorePath); dialog.current?.close(); notify("saved", `Dados restaurados. Safety backup: ${safety.path}`); await refresh(); } catch (error) { notify("error", appError(error).message); } }
  /** Traz as horas do CronoCAD. Caminho de mão única, roda uma vez.
   *
   *  O diálogo abre já no arquivo padrão quando o CronoCAD está instalado: o
   *  usuário não deveria precisar saber que `com.cronocad.app` existe. */
  async function importCronocad() {
    const suggested = await api.defaultCronocadPath().catch(() => null);
    const path = await open({
      multiple: false,
      defaultPath: suggested ?? undefined,
      filters: [{ name: "Banco do CronoCAD", extensions: ["sqlite", "db"] }],
    });
    if (!path) return;
    setImporting(true);
    setImportNote("");
    try {
      setImportReport(await api.importCronocad(path));
      setImportedAt(await api.cronocadImportedAt().catch(() => null));
      await refresh();
    } catch (error) {
      setImportNote(appError(error).message);
    }
    setImporting(false);
  }
  async function checkUpdates() {
    setUpdateState("checking");
    setUpdateInfo(null);
    setUpdateProgress({ downloaded: 0, total: null });
    setMessage("");
    setUpdateNote("Consultando o GitHub Releases…");
    try {
      const update = await api.checkForUpdate();
      setUpdateInfo(update);
      setUpdateState(update ? "available" : "current");
      setUpdateNote(update ? "" : "M/OS já está atualizado.");
    } catch (error) {
      setUpdateState("error");
      setUpdateNote(appError(error).message);
    }
  }
  useEffect(() => {
    if (intent?.target === "updates_check") void checkUpdates();
    if (intent?.target === "function_registry") window.requestAnimationFrame(() => document.querySelector<HTMLElement>("[data-panel='FUNCTIONS']")?.scrollIntoView({ block: "start" }));
  }, [intent?.key]);
  async function installUpdate() {
    setUpdateState("installing");
    setUpdateNote("");
    try {
      await api.installUpdate(setUpdateProgress);
      setUpdateNote("Atualização instalada. Reiniciando M/OS…");
    } catch (error) {
      setUpdateState("error");
      setUpdateNote(appError(error).message);
    }
  }
  /** Uma linha so, sempre no mesmo lugar: o progresso enquanto baixa, o
   *  recado nos demais estados. */
  function updateStatusLine() {
    if (updateNote) return updateNote;
    if (updateState !== "installing") return null;
    if (!updateProgress.total) return "Baixando pacote de atualização…";
    const percent = Math.min(100, Math.round((updateProgress.downloaded / updateProgress.total) * 100));
    return `Baixando atualização: ${percent}%`;
  }
  const archivedProjects = projects.filter((project) => project.lifecycleState === "archived");
  const archivedTasks = tasks.filter((task) => task.lifecycleState === "archived");
  const archivedApps = apps.filter((app) => app.lifecycleState === "archived");
  const archivedResources = resources.filter((resource) => resource.lifecycleState === "archived");
  const archivedWorkspaces = workspaces.filter((workspace) => workspace.lifecycleState === "archived");
  const functionsByCategory = functionCategories.map((category) => ({ category, items: functions.filter((item) => item.category === category) })).filter((group) => group.items.length);
  return <div className="page settings-page"><PaneHeader segments={["M", "SETTINGS"]} meta="SISTEMA" /><section className="settings-section" aria-labelledby="settings-connection"><h2 id="settings-connection" className="settings-section-title">Conexão e aparência</h2><HermesSettings /><UnivirtusSettings /><FinanceActionSettings /><Panel label="APARÊNCIA"><div className="setting-row"><div><strong>Tema claro</strong><p>Dark permanece o padrão do sistema.</p></div><label className="switch"><input type="checkbox" aria-label="Tema claro" checked={theme === "light"} onChange={(event) => setTheme(event.currentTarget.checked ? "light" : "dark")} /><span /></label></div></Panel></section><section className="settings-section" aria-labelledby="settings-updates"><h2 id="settings-updates" className="settings-section-title">Atualizações e entrada</h2><StartupSettings /><Panel label="ATUALIZAÇÕES"><div className="setting-row"><div><strong>Atualizar M/OS</strong><p>{updateInfo ? `Versão instalada: ${updateInfo.currentVersion} · disponível: ${updateInfo.version}` : "Procura uma versão assinada publicada no GitHub Releases."}</p>{updateInfo?.body ? <p className="support-copy">{updateInfo.body}</p> : null}{updateStatusLine() ? <StateMessage state={updateState === "error" ? "error" : updateState === "checking" || updateState === "installing" ? "loading" : "saved"} label={updateStatusLine() ?? ""} /> : null}</div><div className="button-line"><Button variant="secondary" onClick={() => void checkUpdates()} disabled={updateState === "checking" || updateState === "installing"}>{updateState === "checking" ? "Verificando" : "Verificar atualizações"}</Button>{updateState === "available" || updateState === "installing" ? <Button variant="primary" onClick={() => void installUpdate()} disabled={updateState === "installing"}>{updateState === "installing" ? "Instalando" : "Atualizar agora"}</Button> : null}</div></div></Panel><Panel label="CAPTURA RÁPIDA"><form className="setting-row" onSubmit={(event) => { event.preventDefault(); void api.setShortcut(shortcut).then((nextMessage) => notify("saved", nextMessage)).catch((error) => notify("error", appError(error).message)); }}><div><label htmlFor="shortcut">Atalho global</label><p>{status?.shortcut}</p></div><div className="inline-form"><input id="shortcut" value={shortcut} onChange={(event) => setShortcut(event.currentTarget.value)} /><Button variant="primary" type="submit">Aplicar</Button></div></form>{/* A voz mora no mesmo Panel porque ela e a mesma captura por outra
     porta — separa-la num painel proprio a transformaria numa feature
     ao lado, que e exatamente o que o §Voz do design system recusa. */}<form className="setting-row" onSubmit={(event) => { event.preventDefault(); void api.setVoiceShortcut(voiceShortcut).then((nextMessage) => notify("saved", nextMessage)).catch((error) => notify("error", appError(error).message)); }}><div><label htmlFor="voice-shortcut">Atalho da voz</label><p>{status?.voiceShortcut}</p><p className="support-copy">Segure para falar, solte para guardar. Vale de qualquer lugar do Windows, e o microfone só abre enquanto a tecla está pressionada.</p></div><div className="inline-form"><input id="voice-shortcut" value={voiceShortcut} onChange={(event) => setVoiceShortcut(event.currentTarget.value)} /><Button variant="primary" type="submit">Aplicar</Button></div></form></Panel><Panel label="ATALHOS"><p className="support-copy">O M/OS é operável quase inteiro pelo teclado. Nada aqui precisa ser decorado — esta lista existe para quando você quiser.</p><dl className="shortcut-list">{SHORTCUTS.map((entry) => <div key={entry.keys}><dt>{entry.keys}</dt><dd>{entry.does}</dd></div>)}</dl></Panel></section><section className="settings-section" aria-labelledby="settings-meetings"><h2 id="settings-meetings" className="micro-label">REUNIÕES</h2><MeetingSettings /></section><section className="settings-section" aria-labelledby="settings-data"><h2 id="settings-data" className="settings-section-title">Dados e ciclo de vida</h2><Panel label="DADOS E PORTABILIDADE"><p className="support-copy">Backups e exports podem conter dados pessoais em texto claro.</p><div className="button-line"><Button variant="secondary" onClick={() => void backup()}>Criar backup</Button><Button variant="outline" onClick={() => void chooseRestore()}>Restaurar backup</Button><Button variant="outline" onClick={() => void exportData()}>Exportar JSON</Button></div></Panel><Panel label="ARCHIVE E TRASH"><details className="disclosure"><summary>Captures arquivadas <span>{capturesArchived.length}</span></summary>{capturesArchived.map((capture) => <div className="restore-row" key={capture.id}><span>{capture.content}</span><Button variant="ghost" onClick={() => void api.restore(capture.id).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Capture", capture.content, () => api.deleteCapture(capture.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Lixeira de Captures <span>{capturesTrashed.length}</span></summary>{capturesTrashed.map((capture) => <div className="restore-row" key={capture.id}><span>{capture.content}</span><Button variant="ghost" onClick={() => void api.restore(capture.id).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Capture", capture.content, () => api.deleteCapture(capture.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Projects arquivados <span>{archivedProjects.length}</span></summary>{archivedProjects.map((project) => <div className="restore-row" key={project.id}><span>{project.name}</span><Button variant="ghost" onClick={() => void api.setProjectArchived(project.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Project", project.name, () => api.deleteProject(project.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Workspaces arquivados <span>{archivedWorkspaces.length}</span></summary>{archivedWorkspaces.map((workspace) => <div className="restore-row" key={workspace.id}><span>{workspace.name}</span><Button variant="ghost" onClick={() => void api.setWorkspaceArchived(workspace.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Workspace", workspace.name, () => api.deleteWorkspace(workspace.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Apps arquivados <span>{archivedApps.length}</span></summary>{archivedApps.map((app) => <div className="restore-row" key={app.id}><span>{app.name}</span><Button variant="ghost" onClick={() => void api.setRegisteredAppArchived(app.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("App", app.name, () => api.deleteRegisteredApp(app.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Resources arquivados <span>{archivedResources.length}</span></summary>{archivedResources.map((resource) => <div className="restore-row" key={resource.id}><span>{resource.title}</span><Button variant="ghost" onClick={() => void api.setResourceArchived(resource.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Resource", resource.title, () => api.deleteResource(resource.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Lixeira de Resources <span>{trashedResources.length}</span></summary>{trashedResources.map((resource) => <div className="restore-row" key={resource.id}><span>{resource.title}</span><Button variant="ghost" onClick={() => void api.restoreResource(resource.id).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Resource", resource.title, () => api.deleteResource(resource.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Tasks arquivadas <span>{archivedTasks.length}</span></summary>{archivedTasks.map((task) => <div className="restore-row" key={task.id}><span>{task.title}</span><Button variant="ghost" onClick={() => void api.setTaskArchived(task.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Task", task.title, () => api.deleteTask(task.id))}>Excluir</Button></div>)}</details></Panel><Panel label="INTEGRIDADE"><dl className="health-list"><div><dt>Banco</dt><dd>{status?.storage.integrity === "ok" ? "Íntegro" : status?.storage.integrity}</dd></div><div><dt>Schema</dt><dd>v{status?.storage.schemaVersion}</dd></div><div><dt>Durabilidade</dt><dd>{status?.storage.journalMode.toUpperCase()} / {status?.storage.synchronous}</dd></div><div><dt>Snapshot</dt><dd>{status?.snapshot}</dd></div></dl></Panel>{message ? <StateMessage state={messageState} label={message} /> : null}<dialog ref={deleteDialog} className="restore-dialog" onCancel={() => { deleteDialog.current?.close(); setPendingDelete(null); }}><span className="micro-label">EXCLUSÃO DEFINITIVA</span><h2>Excluir {pendingDelete?.noun.toLowerCase()} “{pendingDelete?.label}”?</h2><p>Isto apaga o registro do banco. Não há Desfazer: o único caminho de volta é restaurar um backup anterior a esta ação.</p><div className="form-actions"><Button variant="ghost" onClick={() => { deleteDialog.current?.close(); setPendingDelete(null); }}>Cancelar</Button><Button variant="danger" onClick={() => void confirmDelete()}>Excluir</Button></div></dialog><dialog ref={dialog} className="restore-dialog" onCancel={() => dialog.current?.close()}><span className="micro-label">RESTORE</span><h2>Substituir o dataset local?</h2><p>Um safety backup será criado primeiro. O arquivo contém {inspection?.captureCount} Captures e usa schema v{inspection?.schemaVersion}.</p><div className="form-actions"><Button variant="ghost" onClick={() => dialog.current?.close()}>Cancelar</Button><Button variant="danger" onClick={() => void confirmRestore()}>Restaurar</Button></div></dialog></section><section className="settings-section" aria-labelledby="settings-advanced"><h2 id="settings-advanced" className="settings-section-title">Avançado</h2><Panel label="FUNCTIONS"><p className="support-copy">Registro local das capacidades internas ja existentes. Esta base nao executa automacoes, plugins ou Hermes.</p><div className="function-registry">{functionsByCategory.map((group) => <section key={group.category}><span className="micro-label">{functionCategoryLabels[group.category]}</span>{group.items.map((item) => <div className="function-row" key={item.id}><div><strong>{item.name}</strong><code>{item.id}</code><p>{item.description}</p></div><small>{functionRiskLabels[item.risk]} · {functionConfirmationLabels[item.confirmation]}</small></div>)}</section>)}</div></Panel><Panel label="CRONOCAD"><div className="setting-row"><div><strong>Importar horas do CronoCAD</strong><p>Traz projetos, sessões e pendências para o M/OS. As horas passam a pertencer aos Projects daqui, e o valor/hora de cada sessão é preservado como estava na época.</p><p className="support-copy">Vem tudo: sessões, pendências, programas monitorados, o histórico observado pelo sistema e a sua configuração de arredondamento — sem ela o valor cobrável aqui daria diferente do que o CronoCAD mostra. Roda uma vez, e o banco de origem é aberto somente para leitura. Compare o total com a tela dele antes de desinstalar.</p>{importReport ? <p className="support-copy" aria-live="polite">{importReport.projects} {importReport.projects === 1 ? "project" : "projects"} · {importReport.entries} {importReport.entries === 1 ? "sessão" : "sessões"} · {importReport.tasks} {importReport.tasks === 1 ? "task" : "tasks"} · <strong>{(importReport.trackedSeconds / 3600).toFixed(1)} h</strong>{importReport.activityEvents ? ` · ${importReport.activityEvents} eventos observados` : ""}{importReport.monitoredApps ? ` · ${importReport.monitoredApps} programas` : ""}{importReport.clients ? ` · ${importReport.clients} clientes` : ""}</p> : null}{importNote ? <p className="support-copy" aria-live="polite">{importNote}</p> : null}</div><div className="button-line"><Button variant="secondary" onClick={() => void importCronocad()} disabled={importing || Boolean(importedAt)}>{importing ? "Importando" : importedAt ? "Importado" : "Importar"}</Button></div></div></Panel></section></div>;
}

function QuickCapture() {
  const [content, setContent] = useState("");
  const [state, setState] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [feedback, setFeedback] = useState("Enter para salvar · Esc para fechar");
  const input = useRef<HTMLTextAreaElement>(null);
  /* Fechar a janela e a unica coisa que a voz precisa do lado de fora dela: o
     recibo termina e o overlay some, do mesmo jeito que o texto salvo faz. */
  const voice = useVoiceHud(() => void api.hideQuickCapture());
  const speaking = voice.state.stage !== "idle";

  useEffect(() => {
    input.current?.focus();
    /* O fuso vai junto com a montagem. Quem conhece o fuso e a tela, e sem ele
       "amanha as nove" seria resolvido contra UTC — e cairia no dia errado a
       cada virada de noite. Esta janela publica porque ela e a que sempre
       existe quando alguem fala: o atalho global a revela antes de gravar. */
    void api.surfaceSetLocale().catch(() => undefined);
    const unlisten = listen("window-revealed", () => input.current?.focus());
    return () => { void unlisten.then((dispose) => dispose()); };
  }, []);

  /* O foco volta ao campo quando a fala termina. Sem isto, o `Esc` seguinte
     nao fecharia a janela: quem o escuta e o textarea, e depois da voz nao ha
     nada em foco para receber a tecla. */
  useEffect(() => { if (!speaking) input.current?.focus(); }, [speaking]);

  async function submit(event: FormEvent) { event.preventDefault(); if (!content.trim() || state === "saving") return; setState("saving"); setFeedback("Salvando localmente..."); try { await api.createCapture(content, "quick_capture"); setContent(""); setState("saved"); setFeedback("Salvo na Inbox"); window.setTimeout(() => void api.hideQuickCapture(), 160); } catch (error) { setState("error"); setFeedback(`${appError(error).message} O texto continua aqui.`); } }

  /* As teclas mudam de significado com o estagio, e e por isso que elas moram
     aqui e nao no textarea: durante a fala nao ha campo em foco para receber
     `keydown`. Esc cancela a gravacao antes de fechar a janela — fechar
     primeiro deixaria o microfone aberto atras de um overlay invisivel. */
  useEffect(() => {
    function onKey(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape" && speaking) {
        event.preventDefault();
        if (voice.state.stage === "listening") void voice.cancel();
        else void voice.dismiss();
        return;
      }
      if (voice.state.stage !== "result") return;
      if (event.key === "Enter" && !voice.state.result.executed) {
        event.preventDefault();
        void voice.accept();
      }
      if (event.key.toLowerCase() === "z" && event.ctrlKey && voice.state.result.undo) {
        event.preventDefault();
        void voice.undo();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [speaking, voice]);

  /* Os tracos de amplitude sao a unica presenca da voz — sem icone de
     microfone. Apagados em repouso; em sodio e reagindo enquanto ela existe. */
  return <main className="quick-shell" data-speaking={speaking || undefined}><form className="quick-capture" onSubmit={submit}>
    <div className="capture-line">
      <span className="capture-bar" aria-hidden="true" />
      {speaking
        ? <VoiceSurface state={voice.state} undone={voice.undone} />
        : <><textarea ref={input} value={content} onChange={(event) => { setContent(event.currentTarget.value); if (state !== "idle") { setState("idle"); setFeedback(""); } }} onKeyDown={(event) => { if (event.key === "Escape") void api.hideQuickCapture(); if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); event.currentTarget.form?.requestSubmit(); } }} aria-label="Texto da captura" placeholder="What's on your mind?" rows={1} />
          {content ? null : <span className="capture-caret" aria-hidden="true" />}</>}
      <Amplitude level={voice.state.stage === "listening" ? voice.state.tick?.level ?? 0 : 0} active={voice.state.stage === "listening"} />
    </div>
    <div className="capture-footer">
      {speaking
        ? <VoiceFooter state={voice.state} undone={voice.undone} onAccept={() => void voice.accept()} onUndo={() => void voice.undo()} onRetry={() => void voice.retry()} onDiscard={() => void voice.discard()} />
        : <span className="micro-label">⏎ SALVA · ALT FALA · ESC CANCELA</span>}
      {!speaking && state !== "idle" ? <StateMessage state={state} label={feedback} /> : null}
    </div>
  </form></main>;
}

function DesktopApp() {
  const [page, setPage] = useState<Page>("home");
  /* Qual reuniao abrir ao entrar em Reunioes. A barra de gravacao e o aviso de
     recuperacao escrevem aqui; a pagina le uma vez e segue com o proprio
     estado. */
  const [focusedMeetingId, setFocusedMeetingId] = useState<string | null>(null);
  const [railExpanded, setRailExpanded] = useState(() => localStorage.getItem("m-os-rail-expanded") === "true");
  const [recent, setRecent] = useState<Capture[]>([]);
  const [inbox, setInbox] = useState<Capture[]>([]);
  const [archived, setArchived] = useState<Capture[]>([]);
  const [trashed, setTrashed] = useState<Capture[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [apps, setApps] = useState<RegisteredApp[]>([]);
  const [resources, setResources] = useState<Resource[]>([]);
  const [trashedResources, setTrashedResources] = useState<Resource[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  /* O que esta parado ha tempo demais, e a atividade real de cada Project.
     As duas vem juntas do mesmo comando: a tela precisa das duas no mesmo
     render. */
  const [stale, setStale] = useState<StaleView>({ paradas: [], activity: [] });
  const [academic, setAcademic] = useState<AcademicDashboard | null>(null);
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [hiddenWidgets, setHiddenWidgets] = useState<HiddenWidget[]>([]);
  const [widgetPlacements, setWidgetPlacements] = useState<WidgetPlacement[]>([]);
  /* Lista VAZIA nao significa leque vazio: `resolverPetalas` devolve o padrao
     de fabrica quando nao ha linha (migration 0021). */
  const [radialPins, setRadialPins] = useState<RadialPin[]>([]);
  const [slotEmEscolha, setSlotEmEscolha] = useState<number | null>(null);
  const [resourceWorkspaces, setResourceWorkspaces] = useState<ResourceWorkspace[]>([]);
  /* O que cada Resource de arquivo e: tamanho, tipo, estado da leitura. Vem
     junto do refresh porque a Library precisa disso na PRIMEIRA pintura — uma
     consulta por card faria a lista aparecer sem os fatos e depois piscar. */
  const [ingestions, setIngestions] = useState<Ingestion[]>([]);
  // O contexto ativo deixou de ser assunto da Home: a Library filtra por ele.
  // Continua em localStorage porque e preferencia de leitura, nao dado do core.
  const [currentWorkspaceId, setCurrentWorkspaceId] = useState(() => localStorage.getItem("m-os-current-workspace") ?? "");
  const [commandOpen, setCommandOpen] = useState(false);
  const [attentionOpen, setAttentionOpen] = useState(false);
  const [composerOpen, setComposerOpen] = useState(false);
  // O badge conta itens que ESPERAM ACAO, e nao notificacoes nao lidas.
  // Um numero que sobe com coisa que nao pede acao e um numero que se
  // aprende a ignorar. Quem decide o que conta e o backend (§21.1).
  const [attentionCount, setAttentionCount] = useState(0);
  const [delivered, setDelivered] = useState<DeliveryEvent | null>(null);
  /* O dia carrega FORA do `refresh()`, e a separacao e deliberada: aquele e o
     caminho de boot do app inteiro, e uma falha ao ler a sessao do dia nao pode
     ser motivo para a Home nao abrir. Mesma decisao do `useTrackedTime`, e e
     tambem o que o §39 pede — a Home nao fica lenta por causa desta camada. */
  const daily = useDaily();
  /* Qual sobreposicao do dia esta aberta. Uma so por vez, e por isso um estado
     e nao tres booleanos: dois fluxos abertos ao mesmo tempo escreveriam no
     mesmo dia por dois caminhos. `encerrar` carrega a sessao alvo, que e o que
     distingue "encerrar hoje" de "encerrar o dia que ficou aberto". */
  const [fluxoDoDia, setFluxoDoDia] = useState<{ tipo: "iniciar" } | { tipo: "sessao"; carregada?: DailyToday; aba?: "hoje" | "historico" | "semana" } | { tipo: "encerrar"; dia: DailyToday; sessao: string | null } | null>(null);

  // O agendador vive no backend e avisa quando algo vence. A tela nunca
  // agenda nada: um `setTimeout` morreria no primeiro reload, e o lembrete
  // se perderia justamente quando ninguem estivesse olhando.
  useEffect(() => {
    const delivery = listen<DeliveryEvent>("attention-delivered", (event) => {
      setDelivered(event.payload);
    });
    const counter = listen<number>("attention-count", (event) => {
      setAttentionCount(event.payload);
    });
    void api.attentionCount().then(setAttentionCount).catch(() => undefined);
    return () => {
      void delivery.then((stop) => stop());
      void counter.then((stop) => stop());
    };
  }, []);
  // O overlay continua montado durante os 90ms de saida. Desmontar na hora
  // cortaria a animacao pela metade, que e pior que nao ter animacao.
  const [commandClosing, setCommandClosing] = useState(false);
  const [viewedCapture, setViewedCapture] = useState<Capture | null>(null);
  const [drawerTask, setDrawerTask] = useState<Task | null>(null);
  const [selectedProjectId, setSelectedProjectId] = useState("");
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState("");
  const [selectedAppId, setSelectedAppId] = useState("");
  const [selectedResourceId, setSelectedResourceId] = useState("");
  const [resourceOpenKey, setResourceOpenKey] = useState(0);
  const [functionIntent, setFunctionIntent] = useState<FunctionIntent | null>(null);
  const [undo, setUndo] = useState<UndoAction | null>(null);
  // Alimenta o indicador da topbar. Ate agora so existia implicito nas
  // chamadas de api.ts; o design exige que o sistema diga quando esta ocupado.
  const [busy, setBusy] = useState(false);
  const [bootState, setBootState] = useState<"loading" | "ready" | "error">("loading");
  const [bootMessage, setBootMessage] = useState("");
  const [showBootLoading, setShowBootLoading] = useState(false);
  const [theme, setThemeState] = useState<Theme>(() => localStorage.getItem("m-os-theme") === "light" ? "light" : "dark");
  /* A pose de Argos vive aqui porque `busy` e `bootState` vivem aqui. O
     cronometro e o Hermes ele assina sozinho — ver `useArgosPose`. */
  const argosPose = useArgosPose({ busy, boot: bootState });
  /* A presenca do Hermes vem separada da pose de proposito: pose e fato do
     trabalho, presenca e se ele esta la. Ver `argosPresenca.ts`. */
  const argosPresenca = useArgosPresenca();
  const [dropOcupado, setDropOcupado] = useState(false);
  /* A ocupacao vem do estado que o shell ja tem: `delivered` e o toast, `undo` e
     o recibo, `dropOcupado` e o painel. Nada aqui mede a tela. */
  /* Argos aparece em TODA tela, sem excecao. A colisao com o composer do Hermes
     nao se resolve tirando o bicho: quem se muda e o layout. `.hermes-main`
     reserva o canto dele, e a coluna da conversa desliza o tanto que faltar —
     zero pixel numa janela larga, onde nunca houve disputa. Ver `App.css`. */
  /* A gaveta entra na ocupacao da direita: ela e ancorada la e vai ate o
     rodape, entao Argos ficava por cima do botao primario dela. Vale para as
     duas — a da Task e a da sessao do dia. */
  const gavetaAberta = Boolean(drawerTask) || fluxoDoDia?.tipo === "sessao";
  const argosCanto = cantoPara({
    direitaOcupada: Boolean(delivered) || dropOcupado || gavetaAberta,
    esquerdaOcupada: Boolean(undo),
  });
  const undoTimer = useRef<number | null>(null);
  const functionIntentKey = useRef(0);

  const refresh = useCallback(async () => {
    setBusy(true);
    try {
      const [nextRecent, nextInbox, nextArchived, nextTrashed, nextProjects, nextWorkspaces, nextApps, nextResources, nextTrashedResources, nextTasks, nextStatus, nextHiddenWidgets, nextResourceWorkspaces, nextWidgetPlacements, nextRadialPins, nextIngestions, nextStale, nextAcademic] = await Promise.all([api.recent(), api.inbox(), api.archived(), api.trashed(), api.projects(true), api.workspaces(true), api.registeredApps(true), api.resources(true), api.trashedResources(), api.tasks(true), api.status(), api.hiddenWidgets(), api.resourceWorkspaces(), api.widgetPlacements(), api.radialPins(), api.ingestions(), api.staleList(), api.academicDashboard()]);
      setRecent(nextRecent); setInbox(nextInbox); setArchived(nextArchived); setTrashed(nextTrashed); setProjects(nextProjects); setWorkspaces(nextWorkspaces); setApps(nextApps); setResources(nextResources); setTrashedResources(nextTrashedResources); setTasks(nextTasks); setStatus(nextStatus); setHiddenWidgets(nextHiddenWidgets);
      setWidgetPlacements(nextWidgetPlacements); setResourceWorkspaces(nextResourceWorkspaces); setRadialPins(nextRadialPins); setIngestions(nextIngestions); setStale(nextStale); setAcademic(nextAcademic);
      setDrawerTask((current) => current ? nextTasks.find((task) => task.id === current.id) ?? null : null);
    } finally {
      setBusy(false);
    }
  }, []);
  const initialize = useCallback(async () => {
    setBootState("loading");
    setBootMessage("");
    setShowBootLoading(false);
    const loadingTimer = window.setTimeout(() => setShowBootLoading(true), 150);
    try {
      /* O Tauri abre a janela ANTES de o `setup` terminar, e o primeiro
         `refresh` chega enquanto o banco ainda esta abrindo. O backend recusa
         isso — com razao —, e ate hoje a recusa virava a tela de "os dados nao
         abriram com seguranca", que trava o app e ainda mente: os dados estao
         intactos. Ver `abertura.ts`. */
      for (let tentativa = 0; ; tentativa += 1) {
        try {
          await refresh();
          break;
        } catch (error) {
          const problema = appError(error);
          if (!deveEsperarAbertura(problema, tentativa)) throw error;
          await new Promise((resolve) => window.setTimeout(resolve, esperaDaTentativa(tentativa)));
        }
      }
      setBootState("ready");
    } catch (error) {
      setBootMessage(appError(error).message);
      setBootState("error");
    } finally {
      window.clearTimeout(loadingTimer);
      setShowBootLoading(false);
    }
  }, [refresh]);
  useEffect(() => {
    // O endereco do gateway e reaplicado antes de qualquer conexao: ele vive no
    // renderer porque nao e segredo, e sem isto voltava ao padrao a cada boot.
    void hermes.restoreBaseUrl();
    void initialize();
    const refreshFromEvent = () => {
      void refresh().catch((error) => {
        setBootMessage(appError(error).message);
        setBootState("error");
      });
      /* O dia acompanha, e a razao e concreta: mover uma Task para Done conclui
         o objetivo vinculado a ela dentro da MESMA transacao (§11), e sem esta
         releitura o widget continuaria mostrando o objetivo pendente ate a
         proxima abertura. */
      void daily.recarregar();
    };
    const events = [listen("capture-changed", refreshFromEvent), listen("data-changed", refreshFromEvent), listen("ingestion-extracted", refreshFromEvent), listen("dataset-restored", refreshFromEvent), listen("snapshot-status-changed", refreshFromEvent)];
    return () => { events.forEach((event) => void event.then((dispose) => dispose())); };
  }, [initialize, refresh, daily.recarregar]);
  useEffect(() => { document.documentElement.dataset.theme = theme; localStorage.setItem("m-os-theme", theme); }, [theme]);
  /* Supervisor da ponte do Hermes.
   *
   * Antes a conexao era preguicosa e unica: uma tentativa, so ao entrar no modo
   * Hermes, com um ref impedindo a segunda. O freio existia por um motivo real
   * — sem ele, uma falha anunciava Offline, o efeito se redisparava, e com
   * senha errada isso martelava o login ate o gateway responder 429 e travar a
   * conta do dashboard.
   *
   * O freio agora e outro, e mais preciso: so a falha de TRANSPORTE e repetida.
   * `retriable` vem tipado da ponte (HermesError::retriable), entao a decisao
   * nao depende de ler substring de mensagem. Tunel fechado tenta de novo,
   * espaçando ate cinco minutos; credencial recusada e rate limit param, e
   * param para sempre — quem mexe em credencial e o usuario, em Settings, e e
   * a acao dele que deve destravar a proxima tentativa.
   *
   * Sem credencial configurada nao ha nem primeira tentativa. */
  useEffect(() => {
    let cancelled = false;
    let timer = 0;
    let delay = 5_000;
    // Trava definitiva. A ponte anuncia Offline em TODA falha, inclusive nas que
    // nao se deve repetir — sem esta trava o onState abaixo reagendaria mesmo
    // depois de uma credencial recusada, e o laco do 429 voltaria por outra
    // porta. Só acao do usuario em Settings destrava, remontando a aplicacao.
    let stopped = false;
    // Instante em que a conexao ficou de pe. Serve para distinguir uma sessao
    // que durou de uma que caiu no nascimento — sem essa distincao a espera
    // nunca crescia no caso pior.
    let onlineSince = 0;
    // A espera cresce a CADA agendamento, e nao so quando connect() lanca.
    //
    // Este era o defeito que produzia o laco: hermes_connect devolve Ok assim
    // que o handshake passa, e a sessao pode morrer logo depois. Nesse caminho
    // o catch nunca roda, a espera ficava em 5s, e o app refazia o login
    // inteiro a cada cinco segundos — ate o gateway responder 429.
    function schedule() {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => void attempt(), delay);
      delay = Math.min(delay * 2, 300_000);
    }
    async function attempt() {
      if (cancelled || stopped) return;
      try {
        const current = await hermes.status();
        if (cancelled) return;
        // Ja online ou a caminho: nada a fazer. Se cair, onState reage.
        if (current.state !== "offline") return;
        if (!current.hasCredentials) return;
        await hermes.connect();
      } catch (error) {
        if (cancelled) return;
        const failure = error as Partial<HermesFailure> | null;
        if (!failure?.retriable) { stopped = true; return; }
        schedule();
      }
    }
    void attempt();
    const subscription = hermes.onState((next) => {
      // A queda do socket rearma o supervisor. O timer unico impede que varios
      // anuncios de Offline em sequencia virem varias tentativas paralelas.
      if (next.state === "online") { onlineSince = Date.now(); return; }
      if (next.state !== "offline" || stopped) return;
      // So uma conexao que se sustentou merece recomecar a contagem do zero.
      // Um minuto de pe significa que o problema anterior passou; cair em
      // seguida significa que nao passou, e insistir no mesmo ritmo e o que
      // vira martelo.
      if (onlineSince && Date.now() - onlineSince > 60_000) delay = 5_000;
      onlineSince = 0;
      schedule();
    });
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
      void subscription.then((dispose) => dispose());
    };
  }, []);
  // A chave `m-os-current-workspace-name` deixou de ser escrita: existia so para
  // a Library desenhar o segmento do caminho sem ter o objeto. Com o Workspace
  // chegando por prop, guardar o nome seria uma segunda fonte de verdade.
  const currentWorkspace = workspaces.find((workspace) => workspace.id === currentWorkspaceId && workspace.lifecycleState === "active") ?? null;
  useEffect(() => {
    if (!currentWorkspace) {
      localStorage.removeItem("m-os-current-workspace");
      return;
    }
    localStorage.setItem("m-os-current-workspace", currentWorkspace.id);
  }, [currentWorkspace]);
  /* O contexto que a voz E o Hermes usam quando a frase nao cita nada.
   *
   * Publicado daqui, e nao carregado no comando, porque o atalho GLOBAL dispara
   * do lado do Rust — naquele caminho nao ha chamada do renderer para levar
   * contexto junto. A tela diz o que esta olhando; o backend guarda.
   *
   * A Task aberta tem precedencia sobre o Project da pagina: quem esta com uma
   * Task na frente e falando esta, quase sempre, falando sobre ela. E o Project
   * so vale quando a pagina de Projects esta aberta com um selecionado — o
   * `selectedProjectId` sobrevive a navegacao, e usa-lo em qualquer pagina
   * carimbaria um Project em falas que nao tem nada a ver com ele. */
  useEffect(() => {
    void api.surfaceSetLocale().catch(() => undefined);
  }, []);
  useEffect(() => {
    const projectId = drawerTask?.projectId ?? (page === "projects" ? selectedProjectId || null : null);
    const project = projects.find((candidate) => candidate.id === projectId) ?? null;
    void api
      .surfaceSetContext({
        screen: SCREEN_LABEL[page],
        projectId,
        projectLabel: project?.name ?? null,
        taskId: drawerTask?.id ?? null,
        taskLabel: drawerTask?.title ?? null,
        workspaceId: currentWorkspace?.id ?? null,
        workspaceLabel: currentWorkspace?.name ?? null,
      })
      .catch(() => undefined);
  }, [currentWorkspace, drawerTask, page, projects, selectedProjectId]);
  useEffect(() => { const handler = (event: globalThis.KeyboardEvent) => { if (event.ctrlKey && event.key.toLowerCase() === "k") { event.preventDefault(); setCommandOpen(true); } if (event.ctrlKey && event.key.toLowerCase() === "z" && undo) { event.preventDefault(); void undo.run().then(() => { setUndo(null); return refresh(); }); } }; window.addEventListener("keydown", handler); return () => window.removeEventListener("keydown", handler); }, [refresh, undo]);

  // ~5s: tempo de ler e decidir desfazer, sem virar mobilia na tela.
  function closeCommand() {
    setCommandClosing(true);
    window.setTimeout(() => { setCommandOpen(false); setCommandClosing(false); }, 90);
  }
  function showReceipt(action: UndoAction) { setUndo(action); if (undoTimer.current) window.clearTimeout(undoTimer.current); undoTimer.current = window.setTimeout(() => setUndo(null), 5_000); }
  /* Posicao de leitura por pagina.
   *
   * `UX-PRINCIPLES` §37 pede preservar contexto ao navegar — filtros, posicao,
   * selecao — e a posicao era a que se perdia: voltar para a Inbox depois de
   * abrir um Project devolvia o topo da lista, e reencontrar onde se estava e
   * exatamente o custo mental que o produto existe para remover.
   *
   * A posicao e guardada continuamente sob a pagina que esta na tela, e nao no
   * momento do clique, porque `setPage` e chamado de sete lugares diferentes —
   * salvar em cada um deles seria uma linha para esquecer na oitava. */
  const contentRef = useRef<HTMLElement>(null);
  const scrollByPage = useRef(new Map<Page, number>());
  const shownPage = useRef<Page>(page);

  useEffect(() => {
    const node = contentRef.current;
    if (!node) return;
    const remember = () => scrollByPage.current.set(shownPage.current, node.scrollTop);
    node.addEventListener("scroll", remember, { passive: true });
    return () => node.removeEventListener("scroll", remember);
  }, []);

  // `useLayoutEffect` para restaurar antes da pintura: com `useEffect` a pagina
  // aparece no topo e pula para a posicao guardada, que e pior que nao guardar.
  useLayoutEffect(() => {
    const node = contentRef.current;
    if (!node) return;
    shownPage.current = page;
    node.scrollTop = scrollByPage.current.get(page) ?? 0;
  }, [page]);

  function navigate(page: Page) { setFunctionIntent(null); setPage(page); }
  function toggleRail() {
    setRailExpanded((current) => {
      const next = !current;
      localStorage.setItem("m-os-rail-expanded", String(next));
      return next;
    });
  }
  function openProject(project: Project) { setFunctionIntent(null); setSelectedProjectId(project.id); setPage("projects"); }
  /* Abrir o que um objetivo aponta. E a ponte que o §41 pede: o dia nao guarda
     copia de nada, entao clicar num objetivo tem de levar a entidade de
     verdade. Vinculo que aponta para algo apagado nao faz nada — e melhor que
     abrir uma tela vazia dizendo que o item nao existe. */
  function abrirVinculoDoDia(link: ObjectiveLink) {
    if (link.kind === "task") { const task = tasks.find((candidate) => candidate.id === link.id); if (task) setDrawerTask(task); return; }
    if (link.kind === "project") { const project = projects.find((candidate) => candidate.id === link.id); if (project) openProject(project); return; }
    if (link.kind === "capture") { const capture = [...recent, ...inbox, ...archived].find((candidate) => candidate.id === link.id); if (capture) setViewedCapture(capture); return; }
    if (link.kind === "resource") { const resource = resources.find((candidate) => candidate.id === link.id); if (resource) openResource(resource); return; }
    if (link.kind === "meeting") { setFocusedMeetingId(link.id); setPage("reunioes"); }
  }
  /* Concluir pelo widget da Home. A escrita e do backend, e a tela so releh o
     que ele devolveu — o progresso nunca e recalculado aqui. */
  function concluirObjetivoDoDia(id: string) {
    void api.dailySetObjectiveStatus(id, "completed").then(daily.setDia).catch(() => void daily.recarregar());
  }
  const dailyProps: DailyProps = {
    dia: daily.dia,
    contexto: daily.contexto,
    carregando: daily.carregando,
    erro: daily.erro,
    iniciar: () => setFluxoDoDia({ tipo: "iniciar" }),
    abrirSessao: () => setFluxoDoDia({ tipo: "sessao" }),
    /* Encerrar o dia que ficou aberto resolve os objetivos DAQUELE dia, e o dia
       alvo e montado aqui — onde os dados estao — em vez de o fluxo tentar
       deduzi-lo. Deduzir dava certo hoje por acidente: dependia de `stale`
       continuar preenchido, e ele so existe enquanto hoje nao comecou. */
    encerrarAntigo: () => {
      const velha = daily.dia?.stale;
      if (!daily.dia || !velha) return;
      setFluxoDoDia({
        tipo: "encerrar",
        dia: { ...daily.dia, day: velha.day, session: velha, objectives: daily.dia.staleObjectives, reflection: null },
        sessao: velha.id,
      });
    },
    concluirObjetivo: concluirObjetivoDoDia,
    abrirVinculo: abrirVinculoDoDia,
    semanaPendente: daily.semanaPendente,
    /* Abre a gaveta JA na aba da semana. Levar para a aba da sessao obrigaria
       um segundo clique logo depois de a linha ter dito o que ia acontecer. */
    abrirSemana: () => setFluxoDoDia({ tipo: "sessao", aba: "semana" }),
  };
  function openWorkspace(workspace: Workspace) { setFunctionIntent(null); setSelectedWorkspaceId(workspace.id); setPage("workspaces"); }
  function openRegisteredApp(app: RegisteredApp) { setFunctionIntent(null); setSelectedAppId(app.id); setPage("apps"); }
  function openResource(resource: Resource) { setFunctionIntent(null); setSelectedResourceId(resource.id); setResourceOpenKey((key) => key + 1); setPage("library"); }
  function routeFunction(definition: FunctionDefinition) {
    const target = resolveFunctionTarget(definition);
    if (target === "quick_capture") {
      void api.showQuickCapture();
      return;
    }
    // O compositor e sobreposicao e nao pagina: criar lembrete nao tira a
    // pessoa de onde ela estava. Sair da tela para agendar algo e exatamente
    // a interrupcao que o §85 do UX-PRINCIPLES manda medir e reduzir.
    if (target === "attention_create") {
      setComposerOpen(true);
      return;
    }
    /* Os do dia sao sobreposicao pelo mesmo motivo do compositor de lembrete:
       comecar, ver e encerrar o dia sao gestos curtos, e tirar a pessoa da tela
       em que ela estava para fazer isso e a interrupcao que o §85 do
       UX-PRINCIPLES manda reduzir.

       `daily_add_objective` abre a SESSAO, e nao um formulario avulso: o botao
       de acrescentar vive la, ao lado do que ja existe — e escolher o proximo
       objetivo sem ver os outros e escolher no escuro. */
    if (target === "daily_start") { setFluxoDoDia({ tipo: "iniciar" }); return; }
    if (target === "daily_view" || target === "daily_add_objective") { setFluxoDoDia({ tipo: "sessao" }); return; }
    if (target === "daily_end") { if (daily.dia) setFluxoDoDia({ tipo: "encerrar", dia: daily.dia, sessao: null }); return; }
    functionIntentKey.current += 1;
    setFunctionIntent({ target, key: functionIntentKey.current });
    if (target === "home_capture" || target === "home_arrange") setPage("home");
    else if (target === "inbox_process" || target === "inbox_create_task") setPage("inbox");
    else if (target === "tasks_create" || target === "tasks_move") setPage("tasks");
    else if (target === "projects_create") setPage("projects");
    else if (target === "library_create") setPage("library");
    else if (target === "inbox_create_resource") setPage("inbox");
    else if (target === "workspaces_create" || target === "workspaces_link_project" || target === "workspaces_link_app") setPage("workspaces");
    else if (target === "apps_register") setPage("apps");
    else setPage("settings");
  }
  // Seis destinos, na ordem do design: home · inbox · board · projects · library · apps.
  // O sistema tem oito paginas e o rail aceita seis. Workspaces entra pelo
  // Command; Settings fica no rodape do rail, que o README permite.
  // Sete destinos: o Hermes entrou como tela propria no desenho v0.1 do chat
  // completo, logo depois da Home. Ele deixou de ser so camada dentro do
  // Command — continua alcancavel por la, mas agora tem endereco.
  /* OITO destinos, e a ADR-045 explica por que o teto parou de subir.
     Ele foi de seis a oito (ADR-031), nove (036), dez (038), onze (039) e doze
     (044) — cinco revisoes em pouco mais de duas semanas, cada uma com um bom
     argumento e nenhuma segurando o conjunto, porque o teto era um numero e nao
     um caminho.

     Calendario, Finance e Reunioes sairam para o leque, e a regra nova e que
     destino novo NASCE la: ele so sobe ao rail quando provar ser renda ou
     memoria, pelo criterio que a ADR-036 escreveu. As tres paginas continuam
     existindo, e as portas delas entraram JUNTO com a saida — no leque e no
     widget ACOES da Home —, que e a divida que a ADR-038 registrou ao tirar
     Apps daqui. */
  const nav: { page: Page; label: string; icon: IconName; count?: number }[] = [{ page: "home", label: "Home", icon: "home" }, { page: "hermes", label: "Hermes", icon: "hermes" }, { page: "tasks", label: "Tasks", icon: "board" }, { page: "projects", label: "Projects", icon: "projects" },
  /* Entrou pela ADR-036, e o argumento nao era frequencia: o usuario fatura por
     hora, entao tempo rastreado e o registro de onde sai a renda dele. Fica ao
     lado de Projects porque a hora sempre pertence a um.

     Chamava-se "Tempo". Passou a chamar-se "CronoCAD" pela ADR-050: o produto
     que ele absorveu ja tinha nome, e o dono usou esse nome por meses. "Tempo"
     descrevia a materia; "CronoCAD" nomeia a ferramenta que a pessoa procura.

     O identificador da pagina segue `"tempo"` de proposito. Ele nao aparece em
     tela nenhuma, e renomea-lo tocaria roteamento, leque, Command e widget da
     Home para trocar uma string que ninguem le. */
  { page: "tempo", label: "CronoCAD", icon: "cronocad" },
  { page: "academic", label: "Academic", icon: "academic" },
  /* Workspaces e a lente sobre tudo (ADR-038). Ele ja foi rebaixado uma vez e
     ficou "invisivel para quem nao conhece o Command, ate ser promovido de
     volta" — a ADR-031 registra isso, e e por isso que ele NAO foi para o
     leque nesta troca. */
  { page: "workspaces", label: "Workspaces", icon: "workspaces" },
  { page: "inbox", label: "Inbox", icon: "inbox", count: inbox.length },
  { page: "library", label: "Library", icon: "library" }];
  /* Os grupos usam o vocabulario que a ADR-038 fixou ao definir o que e item de
     rail: "Library e memoria, Inbox e a entrada dela, Workspaces e a lente sobre
     tudo, e Tempo e de onde sai a renda".

     Antes eram tres, SETE e um. Sete itens sob um rotulo e uma lista, nao um
     grupo — o rotulo para de informar. E Inbox ficava em GERAL, longe da Library
     que ele alimenta, enquanto Workspaces sumia no meio dos sete.

     No rail colapsado os grupos existem apenas semanticamente; os rotulos
     aparecem quando o usuario pede contexto expandindo a navegacao. */
  const navGroups = [
    { label: "GERAL", items: nav.slice(0, 2) },
    { label: "TRABALHO", items: nav.slice(2, 6) },
    { label: "MEMÓRIA", items: nav.slice(6) },
  ];
  const pageLabels: Record<Page, string> = { home: "Home", hermes: "Hermes", inbox: "Inbox", tasks: "Tasks", projects: "Projects", tempo: "CronoCAD", calendario: "Calendário", academic: "Academic", finance: "Finance", reunioes: "Reuniões", library: "Library", apps: "Apps", workspaces: "Workspaces", settings: "Settings" };
  const pageMeta = useMemo(() => {
    if (page !== "home") return pageLabels[page].toUpperCase();
    return new Intl.DateTimeFormat("pt-BR", { weekday: "short", day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit" }).format(new Date()).toUpperCase().replace(",", " ·");
  }, [page]);
  const pageContent = useMemo(() => {
    if (page === "hermes") return <HermesPage inbox={inbox} projects={projects} tasks={tasks} receipt={showReceipt} openProject={openProject} openResource={(id) => { const resource = resources.find((candidate) => candidate.id === id); if (resource) openResource(resource); }} openTask={(id) => { const task = tasks.find((candidate) => candidate.id === id); if (task) setDrawerTask(task); }} />;
    if (page === "home") return <HomePage recent={recent} inbox={inbox} projects={projects} tasks={tasks} stale={stale} academic={academic} workspaces={workspaces} apps={apps} resources={resources} resourceWorkspaces={resourceWorkspaces} status={status} hiddenWidgets={hiddenWidgets} setHiddenWidgets={setHiddenWidgets} widgetPlacements={widgetPlacements} setWidgetPlacements={setWidgetPlacements} refresh={refresh} openCapture={setViewedCapture} openProject={openProject} openWorkspace={openWorkspace} openTask={setDrawerTask} openApp={openRegisteredApp} openResource={openResource} openInbox={() => setPage("inbox")} openTasksPage={() => setPage("tasks")} openTempoPage={() => setPage("tempo")} openProjectsPage={() => setPage("projects")} openLibraryPage={() => setPage("library")} openAppsPage={() => setPage("apps")} openFinancePage={() => setPage("finance")} openCalendarPage={() => setPage("calendario")} openMeetingsPage={() => setPage("reunioes")} openAcademicPage={() => setPage("academic")} currentWorkspaceId={currentWorkspaceId} setCurrentWorkspaceId={setCurrentWorkspaceId} currentWorkspace={currentWorkspace} intent={functionIntent ?? undefined} daily={dailyProps} />;
    if (page === "tempo") return <TempoPage projects={projects} openProject={openProject} receipt={showReceipt} />;
    if (page === "finance") return <FinancePage />;
    if (page === "academic") return <AcademicPage refresh={refresh} />;
    if (page === "calendario") return <CalendarPage />;
    if (page === "reunioes") return <MeetingsPage projects={projects} focus={focusedMeetingId} receipt={showReceipt} refresh={refresh} />;
    if (page === "inbox") return <InboxPage captures={inbox} projects={projects} refresh={refresh} receipt={showReceipt} openTask={setDrawerTask} openResource={openResource} intent={functionIntent ?? undefined} />;
    if (page === "projects") return <ProjectsPage projects={projects} tasks={tasks} initialProjectId={selectedProjectId} refresh={refresh} receipt={showReceipt} openTask={setDrawerTask} intent={functionIntent ?? undefined} />;
    if (page === "workspaces") return <WorkspacesPage workspaces={workspaces} projects={projects} apps={apps} initialWorkspaceId={selectedWorkspaceId} refresh={refresh} receipt={showReceipt} openProject={openProject} openApp={openRegisteredApp} openHome={(workspace) => { setCurrentWorkspaceId(workspace.id); setPage("home"); }} intent={functionIntent ?? undefined} />;
    if (page === "apps") return <AppsPage apps={apps} initialAppId={selectedAppId} refresh={refresh} receipt={showReceipt} intent={functionIntent ?? undefined} />;
    if (page === "library") return <LibraryPage resources={resources} workspaces={workspaces} resourceWorkspaces={resourceWorkspaces} ingestions={ingestions} currentWorkspace={currentWorkspace} initialResourceId={selectedResourceId} initialResourceKey={resourceOpenKey} refresh={refresh} receipt={showReceipt} openCapture={setViewedCapture} intent={functionIntent ?? undefined} />;
    if (page === "tasks") return <BoardPage tasks={tasks} projects={projects} stale={stale} refresh={refresh} openTask={setDrawerTask} intent={functionIntent ?? undefined} />;
    return <SettingsPage theme={theme} setTheme={setThemeState} status={status} capturesArchived={archived} capturesTrashed={trashed} projects={projects} tasks={tasks} workspaces={workspaces} apps={apps} resources={resources} trashedResources={trashedResources} refresh={refresh} intent={functionIntent ?? undefined} />;
  // ATENCAO: esta lista e manual e nao ha lint que a verifique. Um estado novo
  // que chegue as paginas por prop e nao entre aqui fica CONGELADO na tela: o
  // clique atualiza o estado, o memo devolve a arvore antiga, e nada acontece.
  //
  // Foi exatamente o que aconteceu com currentWorkspaceId quando o contexto
  // subiu para o raiz — trocar de Workspace parou de funcionar. Os outros tres
  // se salvavam por acidente, porque suas acoes chamam refresh() e o refresh
  // troca a identidade de workspaces/apps/resources, forcando o recalculo.
  // Contexto nao chama refresh, entao travava sozinho e para sempre.
  }, [page, recent, projects, workspaces, apps, resources, trashedResources, tasks, refresh, inbox, selectedProjectId, selectedWorkspaceId, selectedAppId, selectedResourceId, resourceOpenKey, theme, status, archived, trashed, functionIntent, currentWorkspaceId, currentWorkspace, hiddenWidgets, resourceWorkspaces, ingestions, focusedMeetingId, dailyProps]);
  const content = bootState === "ready"
    ? pageContent
    : bootState === "error"
      ? <section className="page startup-state"><h1>M/OS não abriu os dados locais com segurança.</h1><StateMessage state="error" label="Os dados locais permaneceram intactos." detail={bootMessage} /><Button variant="primary" onClick={() => void initialize()}>Tentar novamente</Button></section>
      : showBootLoading
        ? <section className="page startup-state"><StateMessage state="loading" label="Abrindo dados locais..." /></section>
        : null;

  return <div className="app-shell" data-rail-expanded={railExpanded || undefined}><aside className="nav-rail" data-expanded={railExpanded || undefined} aria-label="Navegação do M/OS"><button className="rail-toggle" type="button" aria-label={railExpanded ? "Recolher navegação" : "Expandir navegação"} aria-expanded={railExpanded} onClick={toggleRail}><span className="rail-symbol" aria-hidden="true"><MosSymbol size={16} /></span><span className="rail-brand" aria-hidden="true">M/OS</span></button><nav className="rail-navigation" aria-label="Navegação principal">{navGroups.map((group) => <div className="rail-group" role="group" aria-label={group.label} key={group.label}><span className="rail-group-label" aria-hidden="true">{group.label}</span>{group.items.map((item) => <button className="rail-destination" key={item.page} aria-current={page === item.page ? "page" : undefined} aria-label={item.label} onClick={() => navigate(item.page)}><Icon name={item.icon} filled={page === item.page} /><span className="rail-label" aria-hidden="true">{item.label}</span><span className="rail-tooltip" aria-hidden="true">{item.label}</span>{/* Sem badge de contagem: o desenho nao tem, e um numero permanente no rail
    vira ansiedade de fundo. A contagem da Inbox aparece na Home e na propria
    tela, onde ela leva a uma acao. */}</button>)}</div>)}</nav><div className="rail-footer"><button className="rail-utility" type="button" aria-label={attentionCount > 0 ? `Atencao, ${attentionCount} itens` : "Atencao"} onClick={() => setAttentionOpen(true)}><span className="rail-icon-slot"><Icon name="attention" filled={attentionCount > 0} />{attentionCount > 0 ? <span className="rail-badge">{attentionCount > 9 ? "9+" : attentionCount}</span> : null}</span><span className="rail-label" aria-hidden="true">Atencao</span><span className="rail-tooltip" aria-hidden="true">Atencao</span></button><button className="rail-utility" type="button" aria-label="Quick Capture" onClick={() => void api.showQuickCapture()}><Icon name="capture" /><span className="rail-label" aria-hidden="true">Quick Capture</span><span className="rail-tooltip" aria-hidden="true">Quick Capture</span></button><button className="rail-utility" type="button" aria-current={page === "settings" ? "page" : undefined} aria-label="Settings" onClick={() => navigate("settings")}><Icon name="settings" filled={page === "settings"} /><span className="rail-label" aria-hidden="true">Settings</span><span className="rail-tooltip" aria-hidden="true">Settings</span></button></div></aside><div className="main-column"><header className="topbar"><button className="command-trigger" onClick={() => setCommandOpen(true)}><span className="slash">/</span><span>Command</span><kbd>CTRL K</kbd></button>{/* O estado de sistema nao substitui o meta da pagina: os dois convivem, e o
    indicador de ocupado entra antes sem apagar onde voce esta. */}
<RecordingBar
      onStopped={(meeting) => { setFocusedMeetingId(meeting.id); navigate("reunioes"); }}
      openMeeting={(id) => { setFocusedMeetingId(id); navigate("reunioes"); }}
    />{/* A irma da barra de gravacao: uma para gravar, outra para processar. Mesmo
    lugar, e pelo mesmo motivo — transcrever leva minutos em que a pessoa vai
    estar noutra pagina. */}
<ProcessingBar abrirReuniao={(id) => { setFocusedMeetingId(id); navigate("reunioes"); }} />{/* A barra vive no shell, e nao numa pagina: navegar para a Home nao pode
    apagar da vista o fato de que o microfone esta aberto (§17.2). */}
<div className="system-state" aria-live="polite" data-busy={busy || undefined}>{busy ? <><MosSymbol size={16} spinning /><span className="micro-label">SINCRONIZANDO</span></> : null}<span className="page-meta">{pageMeta}</span></div></header><main className="content" ref={contentRef} data-busy={busy || undefined}><div className="page-surface" key={bootState === "ready" ? page : bootState}>{content}</div></main>{/* O leque vive na coluna principal, e nao sobre o rail: ele e o gesto que
    o rail perdeu quando voltou a oito, e competir com a navegacao ao lado
    seria desfazer a troca. Ver ADR-045. */}
<Leque pins={radialPins} workspaceId={currentWorkspaceId || null} apps={apps} onNavegar={navigate} onAbrirApp={openRegisteredApp} onAcao={(target) => { if (target === "attention_create") setComposerOpen(true); else void api.showQuickCapture(); }} onFixar={(slot) => setSlotEmEscolha(slot)} /></div>{/* Os tres estados do ciclo do dia, um por vez. A `AnimatePresence` de saida
    fica dentro de cada fluxo — eles ja se desmontam com a propria animacao. */}
{fluxoDoDia?.tipo === "iniciar" ? <StartMyDayFlow close={() => setFluxoDoDia(null)} concluido={(proximo) => { daily.setDia(proximo); void daily.recarregar(); }} /> : null}{fluxoDoDia?.tipo === "sessao" && (fluxoDoDia.carregada ?? daily.dia) ? <DailySessionView
      /* A sessao CARREGADA vem da busca ou do historico e pode ser de outro
         dia; sem ela, o que abre e o dia de hoje. As duas passam pelo mesmo
         componente porque sao a mesma tela — o que muda e a data. */
      dia={(fluxoDoDia.carregada ?? daily.dia)!}
      close={() => setFluxoDoDia(null)}
      atualizado={(proximo) => { if (fluxoDoDia.carregada) setFluxoDoDia({ tipo: "sessao", carregada: proximo }); else daily.setDia(proximo); void daily.recarregar(); }}
      /* O dia que a gaveta mostra e o dia que vai ser encerrado. `sessao` so
         viaja quando ele NAO e o de hoje: o backend resolve hoje pela data, e
         mandar o id junto seria dizer a mesma coisa duas vezes. */
      encerrar={() => { const alvo = fluxoDoDia.carregada ?? daily.dia; if (alvo) setFluxoDoDia({ tipo: "encerrar", dia: alvo, sessao: fluxoDoDia.carregada ? (alvo.session?.id ?? null) : null }); }}
      abrirVinculo={abrirVinculoDoDia}
      semanaPendente={daily.semanaPendente}
      abaInicial={fluxoDoDia.aba}
    /> : null}{fluxoDoDia?.tipo === "encerrar" ? <EndMyDayFlow
      dia={fluxoDoDia.dia}
      sessaoAntiga={fluxoDoDia.sessao}
      close={() => setFluxoDoDia(null)}
      concluido={(proximo) => { daily.setDia(proximo); void daily.recarregar(); }}
    /> : null}{composerOpen ? <ReminderComposer close={() => setComposerOpen(false)} created={() => { void api.attentionCount().then(setAttentionCount).catch(() => undefined); setAttentionOpen(true); }} /> : null}{attentionOpen ? <AttentionCenter compose={() => { setAttentionOpen(false); setComposerOpen(true); }} close={() => { setAttentionOpen(false); void api.attentionCount().then(setAttentionCount).catch(() => undefined); }} /> : null}{delivered ? <AttentionToast event={delivered} close={() => setDelivered(null)} open={() => { setDelivered(null); setAttentionOpen(true); }} /> : null}{/* A Drop Zone vive no shell, ao lado das outras sobreposicoes: soltar algo
    em QUALQUER lugar do M/OS tem que funcionar — inclusive sobre o rail —, e e
    o shell quem sabe onde a pessoa estava quando soltou. */}
{<DropZone
      contexto={contextoDoDrop({
        pagina: page,
        projectId: page === "projects" ? selectedProjectId : null,
        workspaceId: currentWorkspaceId,
        taskId: drawerTask?.id ?? null,
        taskProjectId: drawerTask?.projectId ?? null,
      })}
      projects={projects}
      onRecibo={(message, run) => showReceipt({ message, run })}
      refresh={refresh}
      onOcupacao={setDropOcupado}
    />}{commandOpen ? <CommandSurface closing={commandClosing} close={closeCommand} openCapture={setViewedCapture} openTask={setDrawerTask} openProject={openProject} openWorkspace={openWorkspace} openApp={openRegisteredApp} openResource={openResource} openDailySession={(sessionId) => { void api.dailySession(sessionId).then((carregada) => setFluxoDoDia({ tipo: "sessao", carregada })).catch(() => undefined); }} routeFunction={routeFunction} /> : null}{viewedCapture ? <CaptureViewer capture={viewedCapture} close={() => setViewedCapture(null)} /> : null}{drawerTask ? <TaskDrawer key={drawerTask.id} task={drawerTask} projects={projects} close={() => setDrawerTask(null)} refresh={refresh} receipt={showReceipt} openCapture={(capture) => { setDrawerTask(null); setViewedCapture(capture); }} /> : null}{slotEmEscolha !== null ? <LequeSeletor slot={slotEmEscolha} workspaceId={currentWorkspaceId || null} apps={apps} onGravado={setRadialPins} onFechar={() => setSlotEmEscolha(null)} /> : null}<Argos pose={argosPose} presenca={argosPresenca} canto={argosCanto} onAbrir={() => setAttentionOpen(true)} onAbrirHermes={() => navigate("hermes")} /><LazyMotion features={loadMotionFeatures} strict><AnimatePresence>{undo ? <m.div className="receipt" role="status" initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: 8 }} transition={{ duration: MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}><span>{undo.message}</span><button onClick={() => void undo.run().then(() => { setUndo(null); return refresh(); })}>DESFAZER · CTRL Z</button></m.div> : null}</AnimatePresence></LazyMotion></div>;
}

/**
 * As três janelas do M/OS partem do mesmo bundle e se separam pelo rótulo.
 *
 * `main` é o aplicativo; `quick-capture` é a linha de captura global; `lembrete`
 * é a janelinha que aparece sobre o CAD quando o sistema percebe que o trabalho
 * começou sem cronômetro.
 */
/**
 * O aviso in-app de que algo venceu.
 *
 * Nao e o toast do Windows — esse chega no P1. Este aparece dentro da janela,
 * e some sozinho depois de um tempo SEM resolver nada: o Reminder continua no
 * Attention Center. Descartar uma entrega nunca descarta a intencao, que e a
 * separacao inteira entre Reminder e Notification.
 */
function AttentionToast({ event, close, open }: { event: DeliveryEvent; close: () => void; open: () => void }) {
  useEffect(() => {
    if (event.missed) return;
    const timer = window.setTimeout(close, 12000);
    return () => window.clearTimeout(timer);
  }, [event, close]);

  const late = event.overdueSeconds > 60
    ? `atrasado ${Math.round(event.overdueSeconds / 60)} min`
    : "agora";

  return (
    <LazyMotion features={loadMotionFeatures} strict>
      <m.div
        className="attention-toast"
        role="status"
        initial={{ opacity: 0, y: 16, scale: 0.96 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        exit={{ opacity: 0, y: 8, scale: 0.96 }}
        transition={{ duration: MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
      >
        <div>
          <span className="micro-label">{event.missed ? "PERDIDO" : "LEMBRETE"}</span>
          <strong>{event.title}</strong>
          {event.body ? <p>{event.body}</p> : null}
          <span className="attention-when">{late}</span>
        </div>
        <div className="button-line">
          <Button onClick={open} variant="secondary">Ver</Button>
          <Button onClick={close} variant="ghost">Dispensar</Button>
        </div>
      </m.div>
    </LazyMotion>
  );
}

export default function App() {
  switch (getCurrentWindow().label) {
    case "quick-capture":
      return <QuickCapture />;
    case "reuniao-detectada":
      return <ReuniaoDetectada />;
    case "lembrete":
      return <Reminder />;
    default:
      return <DesktopApp />;
  }
}
