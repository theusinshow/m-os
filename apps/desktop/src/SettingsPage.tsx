/**
 * A pagina de configuracoes.
 *
 * Saiu do `App.tsx` porque ela era UMA linha de JSX de dezenas de milhares de
 * caracteres, dentro de um arquivo de 4000 linhas. Isso nao e estetica: um diff
 * que cabe numa linha nao e revisavel, e a pagina precisava ser reagrupada.
 *
 * O que mora aqui sao os paineis que SO o Settings usa. O que a busca tambem
 * usa foi para o `functionLabels.ts`; o que qualquer pagina usa continua no
 * `Surface.tsx`.
 *
 * Os componentes foram movidos VERBATIM na extracao — nenhum corpo editado —
 * porque so assim "nada mudou na tela" e uma afirmacao que alguem pode conferir.
 */
import { type FormEvent, type ReactNode, useCallback, useEffect, useRef, useState } from "react";
import { api, appError, type SinaisNaTela } from "./api";
import { alinhamento, type Alinhamento } from "./malha";
import { open, save } from "@tauri-apps/plugin-dialog";
import { finance } from "./finance";
import { hermes, type HermesStatus, type NivelDeAutonomia, type TunelStatus } from "./hermes";
import { Button } from "./Button";
import { AtualizacaoPanel } from "./AtualizacaoPanel";
import { MeetingSettings } from "./MeetingSettings";
import { PaneHeader, Panel, StateMessage } from "./Surface";
import type { FunctionIntentTarget } from "./functionIntents";
import { functionCategoryLabels, functionConfirmationLabels, functionRiskLabels } from "./functionLabels";
import { relativeTime } from "./relativeTime";
import { SETTINGS_GROUPS, SETTINGS_SECTIONS, secaoInicial, type AlvoDoSettings } from "./settingsNav";
import type {
  AppStatus, BackupInspection, Capture, FunctionDefinition, ImportReport, Ocorrencia,
  AparelhoNaMalha, Project, RegisteredApp, Resource, SyncReport, SyncStatus, Task,
  UnivirtusStatus,
  Workspace,
} from "./types";

/* O tema e um vocabulario da casca do app, e nao um conceito de dominio: ele
   nao vai para o banco nem viaja no sync. Por isso vive aqui e no `App.tsx`,
   e nao no `types.ts` — que e o espelho do que o backend fala. */
export type Theme = "dark" | "light";

/* O intent que a busca dispara para focar um controle desta pagina. Espelha o
   do `App.tsx`, que e quem o produz. */
type FunctionIntent = { target: FunctionIntentTarget; key: number };

/* A ordem das categorias no painel FUNCTIONS. Escrita a mao porque e ordem de
   LEITURA, e nao a do enum. */
const functionCategories: FunctionDefinition["category"][] = ["capture", "daily", "work", "time", "memory", "app", "data", "system"];

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
  { keys: "Alt + ← / →", does: "Voltar e avançar entre as telas por onde você passou" },
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
  const [tunel, setTunel] = useState<TunelStatus | null>(null);
  const [abrindo, setAbrindo] = useState(false);
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

  // O tunel e conferido ao abrir a tela e a cada quinze segundos: ele morre por
  // fora do app — VPS que cai, notebook que dorme — e um botao que so soubesse
  // do estado da montagem diria "aberto" sobre um tunel que ja nao existe.
  useEffect(() => {
    const olhar = () => void hermes.tunnelStatus().then(setTunel).catch(() => undefined);
    olhar();
    const relogio = window.setInterval(olhar, 15_000);
    return () => window.clearInterval(relogio);
  }, []);

  /**
   * Abre o tunel e tenta conectar em seguida.
   *
   * As duas coisas juntas porque quem toca aqui quer FALAR com o Hermes, e nao
   * abrir um tunel — abrir e o meio. Parar no meio deixaria a pessoa com o
   * tunel de pe e a tela ainda dizendo Desconectado.
   */
  async function abrirTunel() {
    setAbrindo(true);
    setMessageState("saving");
    setMessage("Abrindo o túnel...");
    try {
      setTunel(await hermes.tunnelOpen());
      setMessageState("saving");
      setMessage("Túnel aberto. Conectando ao Hermes...");
      await hermes.connect().catch(() => undefined);
      setStatus(await hermes.status().catch(() => null));
      setMessageState("saved");
      setMessage("Túnel aberto.");
    } catch (error) {
      setMessageState("error");
      setMessage(appError(error).message);
    }
    setAbrindo(false);
  }

  /** Túnel de pé e Hermes caído: o gesto é reconectar, e não reabrir. */
  async function reconectar() {
    setAbrindo(true);
    setMessageState("saving");
    setMessage("Reconectando ao Hermes...");
    try {
      await hermes.connect();
      setStatus(await hermes.status());
      setMessageState("saved");
      setMessage("Conectado.");
    } catch (error) {
      setMessageState("error");
      setMessage(appError(error).message);
    }
    setAbrindo(false);
  }

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
  const tunelFechado = tunel !== null && !tunel.aberto;
  const caiu = tunel?.aberto && status?.state === "offline";
  const estadoDoCartao = status?.state === "online" ? "ok" : tunelFechado || caiu ? "atencao" : "neutro";
  return <>
    {/* O cartão de estado vem PRIMEIRO, com a ação ao lado do problema: quem
        abre esta seção quase sempre veio porque o Hermes parou de responder,
        e o motivo mais comum é o túnel fechado sem querer. */}
    <Panel label="ESTADO">
      <div className="hermes-estado" data-estado={estadoDoCartao}>
        <dl className="fact-grid">
          <div><dt>HERMES</dt><dd>{stateLabel}</dd></div>
          {/* Offline com o túnel aberto e Offline com ele fechado são dois
              problemas diferentes, e a tela precisa dizer qual é. */}
          <div><dt>TÚNEL SSH</dt><dd>{tunel === null ? "…" : tunel.aberto ? "Aberto" : <span className="fact-empty">Fechado</span>}</dd></div>
          <div><dt>CREDENCIAL</dt><dd>{status?.hasCredentials ? "Configurada" : <span className="fact-empty">Não configurada</span>}</dd></div>
        </dl>
        <div className="button-line">
          {tunelFechado ? (
            <Button variant="primary" disabled={abrindo || !tunel?.temChave} onClick={() => void abrirTunel()}>
              {abrindo ? "Abrindo…" : "Abrir túnel"}
            </Button>
          ) : null}
          {caiu ? (
            <Button variant="primary" disabled={abrindo} onClick={() => void reconectar()}>
              {abrindo ? "Reconectando…" : "Reconectar"}
            </Button>
          ) : null}
          {status?.state === "online" ? <p className="support-copy">Tudo de pé. O túnel é conferido a cada 15 segundos.</p> : null}
        </div>
        {tunelFechado && !tunel?.temChave ? (
          <p className="support-copy">Não há chave SSH sem passphrase em <code>~/.ssh</code>, então o M/OS não consegue abrir o túnel sozinho. Abra-o pelo terminal ou gere uma chave.</p>
        ) : null}
        {status?.detail ? <p className="support-copy">{status.detail}</p> : null}
        {message ? <StateMessage state={messageState} label={message} /> : null}
      </div>
    </Panel>
    <AutonomiaDoHermes />
    <Panel label="CONEXÃO">
      <p className="support-copy">O M/OS é mais uma superfície do Hermes que já roda na sua VPS — a mesma que você usa pelo WhatsApp, numa conversa separada. O acesso é pelo túnel SSH: o M/OS não abre porta nenhuma.</p>
      {/* Aberto por padrão só quando falta a credencial: é o único caso em que
          alguém PRECISA mexer aqui. No resto do tempo é detalhe. */}
      <details className="disclosure" open={status !== null && !status.hasCredentials}>
        <summary>Endereço e credencial</summary>
        <form className="stack-form" onSubmit={save}>
          <label><span>ENDEREÇO LOCAL</span><input className="mono-input" value={baseUrl} onChange={(event) => setBaseUrl(event.currentTarget.value)} placeholder="http://127.0.0.1:9119" /></label>
          <label><span>USUÁRIO</span><input value={username} onChange={(event) => setUsername(event.currentTarget.value)} autoComplete="off" /></label>
          <label><span>SENHA</span><input type="password" value={password} onChange={(event) => setPassword(event.currentTarget.value)} autoComplete="off" /></label>
          <div className="form-actions">
            <Button variant="ghost" onClick={() => void hermes.clearCredentials().then(() => hermes.status()).then(setStatus).catch(() => undefined)}>Remover credencial</Button>
            <Button variant="primary" type="submit">Salvar</Button>
          </div>
        </form>
      </details>
    </Panel>
  </>;
}

const NIVEIS: { id: NivelDeAutonomia; titulo: string; descricao: string }[] = [
  { id: "perguntar", titulo: "Perguntar sempre", descricao: "Toda ação vira um cartão para você confirmar." },
  { id: "reversivel", titulo: "Agir no que tem desfazer", descricao: "Cria, move, começa e organiza sozinho, com recibo e Desfazer. Pergunta no que não tem volta (encerrar o dia, adiar lembrete) e no M-Finance." },
  { id: "tudo", titulo: "Agir em tudo do M/OS", descricao: "Executa toda ação dentro do M/OS na hora. Só o M-Finance, que fica fora, pede confirmação." },
];

/**
 * Até onde o Hermes age sozinho (ADR-087). Salva ao escolher: é uma decisão de
 * um toque, e um botão "Salvar" a mais seria cerimônia.
 */
function AutonomiaDoHermes() {
  const [nivel, setNivel] = useState<NivelDeAutonomia | null>(null);
  const [erro, setErro] = useState("");
  useEffect(() => { void hermes.autonomia().then(setNivel).catch(() => setNivel("reversivel")); }, []);
  function escolher(proximo: NivelDeAutonomia) {
    const anterior = nivel;
    setNivel(proximo);
    setErro("");
    void hermes.setAutonomia(proximo).catch((causa) => { setNivel(anterior); setErro(appError(causa).message); });
  }
  return <Panel label="AUTONOMIA">
    <p className="support-copy">Quanto o Hermes pode fazer sem pedir licença. Tudo que ele faz aparece com recibo na conversa.</p>
    <div className="autonomia-opcoes" role="radiogroup" aria-label="Autonomia do Hermes">
      {NIVEIS.map((opcao) => <label key={opcao.id} className="autonomia-opcao" data-selected={nivel === opcao.id || undefined}>
        <input type="radio" name="autonomia-hermes" checked={nivel === opcao.id} onChange={() => escolher(opcao.id)} />
        <span><strong>{opcao.titulo}</strong><span className="support-copy">{opcao.descricao}</span></span>
      </label>)}
    </div>
    {erro ? <StateMessage state="error" label={erro} /> : null}
  </Panel>;
}

/**
 * A sincronizacao entre dispositivos.
 *
 * O ENDERECO e visivel e editavel; o SEGREDO entra e nunca volta. Ele mora no
 * Credential Manager do Windows, pelo mesmo caminho da credencial do Hermes —
 * um segredo que a tela pode ler e um segredo que aparece num screenshot.
 *
 * Ate 28/08 a rodada era MANUAL, e o comentario aqui defendia isso: "dizer
 * isso na tela e mais honesto que um automatico que ninguem pediu". Estava
 * certo, e deixou de estar no dia em que alguem pediu — o fluxo casa >
 * trabalho > celular tinha o elo do meio na mao enquanto o celular ja
 * sincronizava sozinho.
 *
 * O botao ficou, e mudou de papel: nao e mais o unico caminho, e sim o que
 * ADIANTA a proxima rodada para quem esta de saida e nao quer esperar o
 * proximo gatilho.
 */
/**
 * O interruptor do Autopilot. Ligado por default: a feature nasce ligada em
 * toda maquina, e desliga-la e uma decisao — nao uma configuracao a descobrir.
 */
function AutopilotSettings() {
  const [ligado, setLigado] = useState<boolean | null>(null);
  useEffect(() => { void api.autopilotStatus().then((s) => setLigado(s.ligado)).catch(() => setLigado(null)); }, []);
  return <Panel label="AUTOPILOT">
    <p className="support-copy">O Autopilot observa o M/OS e traz o que precisa de você: o dia por começar, a Task planejada e não começada, a entrega de amanhã, o follow-up de hoje, o sync parado. Avisa com moderação — nunca a mesma coisa duas vezes, nunca de madrugada, no máximo quatro por hora.</p>
    <dl className="fact-grid">
      <div><dt>AUTOPILOT</dt><dd>{ligado === null ? "—" : ligado ? "ON" : "OFF"}</dd></div>
    </dl>
    <div className="button-line">
      <Button variant={ligado ? "ghost" : "primary"} disabled={ligado === null} onClick={() => void api.autopilotSet(!ligado).then((s) => setLigado(s.ligado)).catch(() => undefined)}>{ligado ? "Desligar avisos" : "Ligar avisos"}</Button>
    </div>
  </Panel>;
}

function SyncSettings() {
  const [status, setStatus] = useState<SyncStatus | null>(null);
  const [malha, setMalha] = useState<AparelhoNaMalha[]>([]);
  const [endpoint, setEndpoint] = useState("");
  const [token, setToken] = useState("");
  const [message, setMessage] = useState("");
  const [messageState, setMessageState] = useState<"saving" | "saved" | "error">("saved");
  const [running, setRunning] = useState(false);

  const refresh = useCallback(async () => {
    const next = await api.syncStatus();
    setStatus(next);
    setEndpoint(next.endpoint);
    // A malha falha em silêncio de propósito: o hub pode estar fora, e uma
    // seção vazia é melhor que a tela de sincronização inteira recusando abrir.
    setMalha(await api.syncMalha().catch(() => []));
  }, []);
  useEffect(() => { void refresh().catch(() => undefined); }, [refresh]);

  async function save(event: FormEvent) {
    event.preventDefault();
    setMessageState("saving");
    setMessage("Salvando...");
    try {
      await api.syncSetEndpoint(endpoint);
      // Campo vazio significa "nao mexi no segredo", e nao "apague o segredo":
      // quem quer apagar tem o botao ao lado, e trocar so o endereco nao pode
      // custar a credencial.
      if (token.trim()) { await api.syncSetToken(token); setToken(""); }
      await refresh();
      setMessageState("saved");
      setMessage("Salvo.");
    } catch (error) { setMessageState("error"); setMessage(appError(error).message); }
  }

  /** Materializa o que chegou e não virou linha. */
  async function reparar() {
    setMessageState("saving");
    setMessage("Reparando...");
    try {
      const reparo = await api.syncReparar();
      await refresh();
      /* Só o que CONTINUA pendente é erro.
         O que o banco recusou saiu da fila e não vai voltar — o lugar já estava
         ocupado por uma linha equivalente, e é ela que a tela mostra. Pintar
         isso de vermelho seria alarmar sobre algo que acabou de ser resolvido. */
      setMessageState(reparo.falharam.length > 0 ? "error" : "saved");
      const partes: string[] = [];
      if (reparo.reparadas > 0) {
        partes.push(`${reparo.reparadas} de ${reparo.examinadas} voltaram a aparecer.`);
      }
      /* "Trecho de conversa", e não "duplicata": eram a parte antiga e a que a
         substituiu, e quem estava na tela era a ANTIGA. Dizer que "o conteúdo
         já estava aqui" era falso, e escondia justamente o que foi corrigido. */
      if (reparo.desocupadas > 0) {
        partes.push(
          reparo.desocupadas === 1
            ? "1 trecho de conversa estava congelado numa versão antiga e foi atualizado."
            : `${reparo.desocupadas} trechos de conversa estavam congelados numa versão antiga e foram atualizados.`,
        );
      }
      if (reparo.abandonadas.length > 0) {
        partes.push(
          reparo.abandonadas.length === 1
            ? "1 duplicata saiu da fila; o conteúdo dela já estava aqui."
            : `${reparo.abandonadas.length} duplicatas saíram da fila; o conteúdo delas já estava aqui.`,
        );
      }
      if (reparo.falharam.length > 0) {
        partes.push(`${reparo.falharam.length} dependem de algo que não chegou.`);
      }
      setMessage(partes.length > 0 ? partes.join(" ") : "Nada estava faltando neste aparelho.");
    } catch (error) { setMessageState("error"); setMessage(appError(error).message); }
  }

  async function run() {
    setRunning(true);
    setMessageState("saving");
    setMessage("Sincronizando...");
    try {
      const round = await api.syncNow();
      await refresh();
      // O erro parcial NAO vira sucesso. O que ja foi feito permanece feito, e
      // a linha conta as duas coisas — esconder a falha faria o proximo clique
      // parecer o primeiro.
      setMessageState(round.error ? "error" : "saved");
      const feito = `${round.sent} enviadas · ${round.received} recebidas`;
      const conflitos = round.conflicts ? ` · ${round.conflicts} em conflito` : "";
      setMessage(round.error ? `${feito}${conflitos}. Parou em: ${round.error}` : `${feito}${conflitos}.`);
    } catch (error) { setMessageState("error"); setMessage(appError(error).message); }
    setRunning(false);
  }

  // Calculado uma vez por render: a lista tem três linhas, e refazer a
  // comparação dentro do `map` seria o mesmo trabalho três vezes.
  const alinhamentos = alinhamento(malha, status?.deviceId ?? "");

  return <Panel label="SINCRONIZAÇÃO">
    <p className="support-copy">O M/OS guarda tudo aqui e funciona inteiro sem isto. O hub só serve para dois aparelhos se alcançarem quando não estão na mesma rede — ele não decide nada, apenas guarda em ordem e devolve.</p>
    <p className="support-copy">Sincroniza sozinho: ao abrir, ao voltar para a frente, depois de você mexer em algo, e a cada 15 minutos. O botão abaixo só adianta a próxima.</p>
    <form className="stack-form" onSubmit={save}>
      <label><span>ENDEREÇO DO HUB</span><input className="mono-input" value={endpoint} onChange={(event) => setEndpoint(event.currentTarget.value)} placeholder="http://127.0.0.1:9120" /></label>
      <label><span>SEGREDO</span><input type="password" value={token} onChange={(event) => setToken(event.currentTarget.value)} autoComplete="off" placeholder={status?.hasToken ? "Guardado — digite para trocar" : "Ao menos 32 caracteres"} /></label>
      <div className="form-actions">
        <Button variant="ghost" onClick={() => void api.syncClearToken().then(refresh).catch(() => undefined)}>Remover segredo</Button>
        <Button variant="primary" type="submit">Salvar</Button>
      </div>
    </form>
    <dl className="fact-grid">
      <div><dt>SEGREDO</dt><dd>{status?.hasToken ? "Guardado" : <span className="fact-empty">Não configurado</span>}</dd></div>
      <div><dt>NA FILA</dt><dd>{status ? `${status.pending}` : "—"}</dd></div>
      {/* Sem a hora da ultima rodada, um sync que parou de funcionar parece
          igual a um que funciona. */}
      <div><dt>ÚLTIMA</dt><dd>{status?.lastSyncAt ? relativeTime(status.lastSyncAt) : <span className="fact-empty">Nunca</span>}</dd></div>
    </dl>
    {malha.length > 0 ? <>
      <p className="rotulo">A MALHA</p>
      <ul className="malha">
        {malha.map((aparelho) => {
          const euMesmo = aparelho.id === status?.deviceId;
          const divergente = aparelho.versao !== status?.appVersion;
          const situacao: Alinhamento | undefined = alinhamentos.find((linha) => linha.id === aparelho.id);
          return <li key={aparelho.id} data-divergente={divergente || undefined}>
            <span className="malha-nome">{aparelho.nome}</span>
            <span className="malha-versao">{aparelho.versao}</span>
            {/* Aviso, e não bloqueio: versão diferente não impede sincronizar,
                e a frase é o que encerra a investigação. */}
            <span className="malha-visto">{euMesmo ? "este aparelho" : relativeTime(aparelho.vistoEm)}{divergente ? " · em versão diferente" : ""}</span>
            {/* O alinhamento fala por cor E por palavra: cor sozinha não diz
                nada a quem não distingue âmbar de verde. */}
            <span className="malha-alinhamento" data-estado={situacao?.estado}>
              {situacao?.estado === "alinhado" ? (situacao.detalhe || "alinhado") : situacao?.detalhe}
            </span>
          </li>;
        })}
      </ul>
    </> : null}
    <div className="button-line">
      <Button variant="secondary" disabled={running || !status?.endpoint || !status?.hasToken} onClick={() => void run()}>{running ? "Sincronizando" : "Sincronizar agora"}</Button>
      {/* O reparo aparece SEMPRE, e não só quando a malha acusa divergência: o
          defeito que ele conserta é invisível por definição — a entidade está
          no banco de sincronização e não na tela —, e esconder o botão até
          alguém provar que ele é necessário esconderia a única saída. */}
      <Button variant="ghost" onClick={() => void reparar()}>Reparar este aparelho</Button>
    </div>
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
        O Hermes pode propor lançamentos no M-Finance — contas, compras no cartão, receitas,
        pagamentos, assinaturas, metas e políticas — nunca sem confirmação explícita. Isto guarda
        o secret que autoriza o M/OS a chamar a Action API do M-Finance (o mesmo
        <code> MOS_ACTION_SECRET</code> configurado do lado do M-Finance). Ele também serve para
        leitura quando o secret de leitura abaixo não está configurado.
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
 * O secret de LEITURA do M-Finance (ADR-088): o Hermes lê o estado financeiro
 * por ele e não consegue escrever nada. Separado do de ação de propósito —
 * vazar este não lança conta nenhuma.
 */
function FinanceReadSettings() {
  const [configured, setConfigured] = useState(false);
  const [secret, setSecret] = useState("");
  const [message, setMessage] = useState("");
  const [messageState, setMessageState] = useState<"saving" | "saved" | "error">("saved");

  useEffect(() => {
    void finance.readSecretConfigured().then(setConfigured).catch(() => undefined);
  }, []);

  async function save(event: FormEvent) {
    event.preventDefault();
    if (!secret.trim()) return;
    setMessageState("saving");
    setMessage("Salvando secret...");
    try {
      await finance.setReadSecret(secret);
      setSecret("");
      setConfigured(true);
      setMessage("Secret de leitura guardado no Windows Credential Manager.");
      setMessageState("saved");
    } catch (error) {
      setMessageState("error");
      setMessage(String(error));
    }
  }

  async function clear() {
    await finance.clearReadSecret().catch(() => undefined);
    setConfigured(false);
  }

  return (
    <Panel label="LEITURA DO M-FINANCE PELO HERMES">
      <p className="support-copy">
        Com isto o Hermes responde perguntas financeiras com os números reais — mês, Safe-to-Spend,
        faturas, cenários — e a Home ganha o widget FINANÇAS. Os dados só descem quando a pergunta
        é sobre dinheiro, e cada envio fica registrado na conversa. O secret é o
        <code> MOS_FINANCE_READ_SECRET</code> do M-Finance; ele não escreve nada.
      </p>
      <form className="stack-form" onSubmit={save}>
        <label><span>SECRET DE LEITURA</span><input type="password" value={secret} onChange={(event) => setSecret(event.currentTarget.value)} autoComplete="off" /></label>
        <div className="form-actions">
          <Button variant="ghost" onClick={() => void clear()}>Remover secret</Button>
          <Button variant="primary" type="submit">Salvar</Button>
        </div>
      </form>
      <dl className="fact-grid">
        <div><dt>LEITURA</dt><dd>{configured ? "Secret próprio" : <span className="fact-empty">Usando o secret de ação, se houver</span>}</dd></div>
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

/**
 * O caderno de ocorrencias, na tela.
 *
 * # Por que ele existe aqui
 *
 * O M/OS passou a gravar panico do Rust, erro nao tratado da interface e janela
 * que abriu sem montar (ver `src-tauri/src/diagnostico.rs`). Um log que so o
 * desenvolvedor alcanca e um log que so serve depois que alguem ja perguntou —
 * e as duas falhas que motivaram tudo isso acontecem no LOGON, longe de
 * qualquer terminal aberto.
 *
 * # Por que fechado por padrao
 *
 * `<details>`, e nao um painel sempre aberto. §14, quiet UI: uma lista de erros
 * permanente em Settings ensina ansiedade sobre um app que esta funcionando. O
 * caderno so interessa em dois momentos — quando algo quebrou, e quando alguem
 * quer conferir que nada quebrou.
 */
/**
 * Sinais de uso (ADR-082): onde o M/OS ainda pede trabalho manual.
 *
 * Fechado por padrão pelo mesmo motivo do caderno de ocorrências — não é
 * placar, e uma tela de números sempre aberta viraria cobrança. Tudo local, só
 * contagens e idades; nada do que foi escrito entra aqui.
 */
function SinaisPanel() {
  const [dados, setDados] = useState<SinaisNaTela | null>(null);
  const [erro, setErro] = useState("");
  const ler = useCallback(() => {
    setErro("");
    void api.sinaisDeUso().then(setDados).catch((causa) => setErro(appError(causa).message));
  }, []);
  const s = dados?.sinais;
  const horas = (h: number | null) => (h === null ? "—" : h >= 48 ? `${Math.round(h / 24)} dias` : `${h} h`);
  return (
    <Panel label="SINAIS DE USO">
      <p className="support-copy">Onde o M/OS ainda pede trabalho manual, nos últimos 14 dias. Fica neste computador.</p>
      <details className="disclosure" onToggle={(event) => { if (event.currentTarget.open && !dados) ler(); }}>
        <summary>Ver sinais {dados ? <span>{dados.leituras.length}</span> : null}</summary>
        {erro ? <StateMessage state="error" label="Não deu para ler os sinais." detail={erro} /> : null}
        {!dados && !erro ? <p className="support-copy">Lendo…</p> : null}
        {dados && s ? <>
          {dados.leituras.length ? <ul className="support-copy">{dados.leituras.map((frase) => <li key={frase}>{frase}</li>)}</ul> : <p className="support-copy">Nada pedindo atenção agora.</p>}
          <dl className="health-list">
            <div><dt>Inbox</dt><dd>{s.inbox.pendentes} pendentes · mediana {horas(s.inbox.idadeMedianaHoras)}</dd></div>
            <div><dt>Propostas do Hermes</dt><dd>{s.propostas.executadas} feitas · {s.propostas.canceladas} canceladas · {s.propostas.falharam + s.propostas.recusadas} sem efeito{s.propostas.incertas ? ` · ${s.propostas.incertas} incertas` : ""}</dd></div>
            <div><dt>Reuniões</dt><dd>{s.reunioes.prontas} prontas{s.reunioes.ateProntaMedianaMinutos !== null ? ` em ~${s.reunioes.ateProntaMedianaMinutos} min` : ""} · {s.reunioes.pedemAtencao} paradas</dd></div>
            <div><dt>Adiamentos</dt><dd>{s.adiamentos.lembretesAdiadosNaJanela} lembretes adiados · {s.adiamentos.tasksAdiadas3x} Tasks 3×</dd></div>
            <div><dt>Sync</dt><dd>{s.sync.fila} na fila{s.sync.maisAntigaHoras !== null ? ` · mais antiga ${horas(s.sync.maisAntigaHoras)}` : ""}</dd></div>
            <div><dt>Dia</dt><dd>{s.dia.diasIniciados} começados · {s.dia.diasEncerrados} encerrados</dd></div>
          </dl>
        </> : null}
      </details>
    </Panel>
  );
}

function DiagnosticoPanel() {
  const [ocorrencias, setOcorrencias] = useState<Ocorrencia[] | null>(null);
  const [caminho, setCaminho] = useState("");
  const [carregando, setCarregando] = useState(false);

  const ler = useCallback(async () => {
    setCarregando(true);
    try {
      const [linhas, arquivo] = await Promise.all([
        api.diagnosticoRecente(40),
        api.diagnosticoCaminho(),
      ]);
      setOcorrencias(linhas);
      setCaminho(arquivo);
    } catch {
      setOcorrencias([]);
    } finally {
      setCarregando(false);
    }
  }, []);

  return (
    <Panel label="DIAGNÓSTICO">
      <p className="support-copy">
        O M/OS registra aqui o que quebrou: pânico do Rust, erro da interface e janela que
        abriu sem carregar. Nada do que você escreveu ou gravou entra neste arquivo.
      </p>
      <details className="disclosure" onToggle={(event) => { if (event.currentTarget.open && !ocorrencias) void ler(); }}>
        <summary>Últimas ocorrências {ocorrencias ? <span>{ocorrencias.length}</span> : null}</summary>
        {carregando ? (
          <p className="support-copy">Lendo o caderno…</p>
        ) : ocorrencias && ocorrencias.length === 0 ? (
          <p className="support-copy">Nada registrado. É o que se espera.</p>
        ) : (
          <dl className="health-list">
            {(ocorrencias ?? []).map((linha, posicao) => (
              <div key={`${linha.quando}-${posicao}`}>
                <dt>{new Date(linha.quando).toLocaleString("pt-BR")} · {linha.nivel} · {linha.origem}</dt>
                <dd>{linha.mensagem}</dd>
              </div>
            ))}
          </dl>
        )}
        {caminho ? <p className="support-copy"><code>{caminho}</code></p> : null}
        <div className="button-line">
          <Button variant="ghost" onClick={() => void ler()} disabled={carregando}>Atualizar</Button>
        </div>
      </details>
    </Panel>
  );
}

export function SettingsPage({ theme, setTheme, status, capturesArchived, capturesTrashed, projects, tasks, workspaces, apps, resources, trashedResources, refresh, intent }: { theme: Theme; setTheme: (theme: Theme) => void; status: AppStatus | null; capturesArchived: Capture[]; capturesTrashed: Capture[]; projects: Project[]; tasks: Task[]; workspaces: Workspace[]; apps: RegisteredApp[]; resources: Resource[]; trashedResources: Resource[]; refresh: () => Promise<void>; intent?: FunctionIntent }) {
  const [shortcut, setShortcut] = useState("Ctrl+Shift+Space");
  const [voiceShortcut, setVoiceShortcut] = useState("Ctrl+Alt+G");
  const [message, setMessage] = useState("");
  const [messageState, setMessageState] = useState<"saved" | "error">("saved");
  const [inspection, setInspection] = useState<BackupInspection | null>(null);
  const [restorePath, setRestorePath] = useState("");
  const [importing, setImporting] = useState(false);
  const [importReport, setImportReport] = useState<ImportReport | null>(null);
  const [importNote, setImportNote] = useState("");
  // Pergunta ao banco, e não à memória da sessão: fechar e reabrir o app não
  // deveria reabilitar um botão que não pode mais ser clicado.
  const [importedAt, setImportedAt] = useState<string | null>(null);
  useEffect(() => { void api.cronocadImportedAt().then(setImportedAt).catch(() => undefined); }, []);
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
  const archivedProjects = projects.filter((project) => project.lifecycleState === "archived");
  const archivedTasks = tasks.filter((task) => task.lifecycleState === "archived");
  const archivedApps = apps.filter((app) => app.lifecycleState === "archived");
  const archivedResources = resources.filter((resource) => resource.lifecycleState === "archived");
  const archivedWorkspaces = workspaces.filter((workspace) => workspace.lifecycleState === "archived");
  const functionsByCategory = functionCategories.map((category) => ({ category, items: functions.filter((item) => item.category === category) })).filter((group) => group.items.length);
  /* Uma secao por vez. A escolhida fica lembrada neste computador: voltar ao
     Settings e cair de novo na primeira seria refazer o caminho. */
  const [secao, setSecao] = useState(() => secaoInicial(alvoDoSettings(intent?.target), lerSecaoSalva()));
  useEffect(() => {
    const alvo = alvoDoSettings(intent?.target);
    if (alvo) setSecao(secaoInicial(alvo, null));
  }, [intent?.key]);
  const escolher = useCallback((id: string) => {
    setSecao(id);
    try { localStorage.setItem(CHAVE_SECAO, id); } catch { /* sem armazenamento: so nao lembra */ }
    document.querySelector<HTMLElement>(".settings-content")?.scrollTo({ top: 0 });
  }, []);
  const atual = SETTINGS_SECTIONS.find((item) => item.id === secao) ?? SETTINGS_SECTIONS[0];

  /* A pagina itera o CATALOGO, e nao uma lista escrita aqui. Uma segunda copia
     da ordem envelheceria em silencio — foi o que aconteceu com o
     `arrange_widgets`, que existiu em Rust e em TypeScript ao mesmo tempo. */
  const conteudo: Record<string, ReactNode> = {
    sync: <><SyncSettings /><AutopilotSettings /></>,
    hermes: <HermesSettings />,
    integracoes: <><UnivirtusSettings /><FinanceActionSettings /><FinanceReadSettings /></>,
    aparencia: <><Panel label="APARÊNCIA"><div className="setting-row"><div><strong>Tema claro</strong><p>Dark permanece o padrão do sistema.</p></div><label className="switch"><input type="checkbox" aria-label="Tema claro" checked={theme === "light"} onChange={(event) => setTheme(event.currentTarget.checked ? "light" : "dark")} /><span /></label></div></Panel><Panel label="CAPTURA RÁPIDA"><form className="setting-row" onSubmit={(event) => { event.preventDefault(); void api.setShortcut(shortcut).then((nextMessage) => notify("saved", nextMessage)).catch((error) => notify("error", appError(error).message)); }}><div><label htmlFor="shortcut">Atalho global</label><p>{status?.shortcut}</p></div><div className="inline-form"><input id="shortcut" value={shortcut} onChange={(event) => setShortcut(event.currentTarget.value)} /><Button variant="primary" type="submit">Aplicar</Button></div></form>{/* A voz mora no mesmo Panel porque ela e a mesma captura por outra
     porta — separa-la num painel proprio a transformaria numa feature
     ao lado, que e exatamente o que o §Voz do design system recusa. */}<form className="setting-row" onSubmit={(event) => { event.preventDefault(); void api.setVoiceShortcut(voiceShortcut).then((nextMessage) => notify("saved", nextMessage)).catch((error) => notify("error", appError(error).message)); }}><div><label htmlFor="voice-shortcut">Atalho da voz</label><p>{status?.voiceShortcut}</p><p className="support-copy">Segure para falar, solte para guardar. Vale de qualquer lugar do Windows, e o microfone só abre enquanto a tecla está pressionada.</p></div><div className="inline-form"><input id="voice-shortcut" value={voiceShortcut} onChange={(event) => setVoiceShortcut(event.currentTarget.value)} /><Button variant="primary" type="submit">Aplicar</Button></div></form></Panel><Panel label="ATALHOS"><p className="support-copy">O M/OS é operável quase inteiro pelo teclado. Nada aqui precisa ser decorado — esta lista existe para quando você quiser.</p><dl className="shortcut-list">{SHORTCUTS.map((entry) => <div key={entry.keys}><dt>{entry.keys}</dt><dd>{entry.does}</dd></div>)}</dl></Panel></>,
    inicio: <><StartupSettings /><AtualizacaoPanel verificarAoAbrir={intent?.target === "updates_check"} /></>,
    reunioes: <><MeetingSettings /></>,
    dados: <><Panel label="DADOS E PORTABILIDADE"><p className="support-copy">Backups e exports podem conter dados pessoais em texto claro.</p><div className="button-line"><Button variant="secondary" onClick={() => void backup()}>Criar backup</Button><Button variant="outline" onClick={() => void chooseRestore()}>Restaurar backup</Button><Button variant="outline" onClick={() => void exportData()}>Exportar JSON</Button></div></Panel><Panel label="ARCHIVE E TRASH"><details className="disclosure"><summary>Captures arquivadas <span>{capturesArchived.length}</span></summary>{capturesArchived.map((capture) => <div className="restore-row" key={capture.id}><span>{capture.content}</span><Button variant="ghost" onClick={() => void api.restore(capture.id).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Capture", capture.content, () => api.deleteCapture(capture.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Lixeira de Captures <span>{capturesTrashed.length}</span></summary>{capturesTrashed.map((capture) => <div className="restore-row" key={capture.id}><span>{capture.content}</span><Button variant="ghost" onClick={() => void api.restore(capture.id).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Capture", capture.content, () => api.deleteCapture(capture.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Projects arquivados <span>{archivedProjects.length}</span></summary>{archivedProjects.map((project) => <div className="restore-row" key={project.id}><span>{project.name}</span><Button variant="ghost" onClick={() => void api.setProjectArchived(project.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Project", project.name, () => api.deleteProject(project.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Workspaces arquivados <span>{archivedWorkspaces.length}</span></summary>{archivedWorkspaces.map((workspace) => <div className="restore-row" key={workspace.id}><span>{workspace.name}</span><Button variant="ghost" onClick={() => void api.setWorkspaceArchived(workspace.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Workspace", workspace.name, () => api.deleteWorkspace(workspace.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Apps arquivados <span>{archivedApps.length}</span></summary>{archivedApps.map((app) => <div className="restore-row" key={app.id}><span>{app.name}</span><Button variant="ghost" onClick={() => void api.setRegisteredAppArchived(app.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("App", app.name, () => api.deleteRegisteredApp(app.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Resources arquivados <span>{archivedResources.length}</span></summary>{archivedResources.map((resource) => <div className="restore-row" key={resource.id}><span>{resource.title}</span><Button variant="ghost" onClick={() => void api.setResourceArchived(resource.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Resource", resource.title, () => api.deleteResource(resource.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Lixeira de Resources <span>{trashedResources.length}</span></summary>{trashedResources.map((resource) => <div className="restore-row" key={resource.id}><span>{resource.title}</span><Button variant="ghost" onClick={() => void api.restoreResource(resource.id).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Resource", resource.title, () => api.deleteResource(resource.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Tasks arquivadas <span>{archivedTasks.length}</span></summary>{archivedTasks.map((task) => <div className="restore-row" key={task.id}><span>{task.title}</span><Button variant="ghost" onClick={() => void api.setTaskArchived(task.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Task", task.title, () => api.deleteTask(task.id))}>Excluir</Button></div>)}</details></Panel></>,
    saude: <><Panel label="INTEGRIDADE"><dl className="health-list"><div><dt>Banco</dt><dd>{status?.storage.integrity === "ok" ? "Íntegro" : status?.storage.integrity}</dd></div><div><dt>Schema</dt><dd>v{status?.storage.schemaVersion}</dd></div><div><dt>Durabilidade</dt><dd>{status?.storage.journalMode.toUpperCase()} / {status?.storage.synchronous}</dd></div><div><dt>Snapshot</dt><dd>{status?.snapshot}</dd></div></dl></Panel><DiagnosticoPanel /><SinaisPanel /></>,
  };
  const dialogos = <><dialog ref={deleteDialog} className="restore-dialog" onCancel={() => { deleteDialog.current?.close(); setPendingDelete(null); }}><span className="micro-label">EXCLUSÃO DEFINITIVA</span><h2>Excluir {pendingDelete?.noun.toLowerCase()} “{pendingDelete?.label}”?</h2><p>Isto apaga o registro do banco. Não há Desfazer: o único caminho de volta é restaurar um backup anterior a esta ação.</p><div className="form-actions"><Button variant="ghost" onClick={() => { deleteDialog.current?.close(); setPendingDelete(null); }}>Cancelar</Button><Button variant="danger" onClick={() => void confirmDelete()}>Excluir</Button></div></dialog><dialog ref={dialog} className="restore-dialog" onCancel={() => dialog.current?.close()}><span className="micro-label">RESTORE</span><h2>Substituir o dataset local?</h2><p>Um safety backup será criado primeiro. O arquivo contém {inspection?.captureCount} Captures e usa schema v{inspection?.schemaVersion}.</p><div className="form-actions"><Button variant="ghost" onClick={() => dialog.current?.close()}>Cancelar</Button><Button variant="danger" onClick={() => void confirmRestore()}>Restaurar</Button></div></dialog></>;
  const conteudoResto: Record<string, ReactNode> = {
    avancado: <><Panel label="FUNCTIONS"><p className="support-copy">Registro local das capacidades internas ja existentes. Esta base nao executa automacoes, plugins ou Hermes.</p><div className="function-registry">{functionsByCategory.map((group) => <section key={group.category}><span className="micro-label">{functionCategoryLabels[group.category]}</span>{group.items.map((item) => <div className="function-row" key={item.id}><div><strong>{item.name}</strong><code>{item.id}</code><p>{item.description}</p></div><small>{functionRiskLabels[item.risk]} · {functionConfirmationLabels[item.confirmation]}</small></div>)}</section>)}</div></Panel><Panel label="CRONOCAD"><div className="setting-row"><div><strong>Importar horas do CronoCAD</strong><p>Traz projetos, sessões e pendências para o M/OS. As horas passam a pertencer aos Projects daqui, e o valor/hora de cada sessão é preservado como estava na época.</p><p className="support-copy">Vem tudo: sessões, pendências, programas monitorados, o histórico observado pelo sistema e a sua configuração de arredondamento — sem ela o valor cobrável aqui daria diferente do que o CronoCAD mostra. Roda uma vez, e o banco de origem é aberto somente para leitura. Compare o total com a tela dele antes de desinstalar.</p>{importReport ? <p className="support-copy" aria-live="polite">{importReport.projects} {importReport.projects === 1 ? "project" : "projects"} · {importReport.entries} {importReport.entries === 1 ? "sessão" : "sessões"} · {importReport.tasks} {importReport.tasks === 1 ? "task" : "tasks"} · <strong>{(importReport.trackedSeconds / 3600).toFixed(1)} h</strong>{importReport.activityEvents ? ` · ${importReport.activityEvents} eventos observados` : ""}{importReport.monitoredApps ? ` · ${importReport.monitoredApps} programas` : ""}{importReport.clients ? ` · ${importReport.clients} clientes` : ""}</p> : null}{importNote ? <p className="support-copy" aria-live="polite">{importNote}</p> : null}</div><div className="button-line"><Button variant="secondary" onClick={() => void importCronocad()} disabled={importing || Boolean(importedAt)}>{importing ? "Importando" : importedAt ? "Importado" : "Importar"}</Button></div></div></Panel></>,
  };

  const paineis = { ...conteudo, ...conteudoResto };

  return <div className="page settings-page">
    <PaneHeader segments={["M", "SETTINGS", atual.title.toUpperCase()]} meta="SISTEMA" />
    <div className="settings-layout">
      {/* Navegacao de SECAO, e nao o rail do app: o rail troca de pagina, e
          isto troca o que a pagina mostra. Agrupada, para nove itens nao
          virarem uma lista sem forma. */}
      <nav className="settings-nav" aria-label="Seções das configurações">
        {SETTINGS_GROUPS.map((grupo) => <div className="settings-nav-grupo" key={grupo.id}>
          <span className="micro-label">{grupo.title}</span>
          {SETTINGS_SECTIONS.filter((item) => item.grupo === grupo.id).map((item) => <button
            type="button"
            key={item.id}
            aria-current={item.id === atual.id ? "page" : undefined}
            data-selected={item.id === atual.id || undefined}
            onClick={() => escolher(item.id)}
          >{item.title}</button>)}
        </div>)}
      </nav>
      <div className="settings-content">
        <section className="settings-section" aria-labelledby="settings-atual-title">
          <header className="settings-section-head">
            <h2 id="settings-atual-title" className="settings-section-title">{atual.title}</h2>
            <p className="support-copy">{atual.descricao}</p>
          </header>
          {paineis[atual.id]}
        </section>
        {message ? <StateMessage state={messageState} label={message} /> : null}
        {dialogos}
      </div>
    </div>
  </div>;
}

const CHAVE_SECAO = "mos-settings-secao";

function lerSecaoSalva(): string | null {
  try { return localStorage.getItem(CHAVE_SECAO); } catch { return null; }
}

/** Os alvos de function que caem numa secao do Settings. */
function alvoDoSettings(target: string | undefined): AlvoDoSettings {
  return target === "function_registry" || target === "updates_check" || target === "hermes" || target === "sync"
    ? target
    : undefined;
}
