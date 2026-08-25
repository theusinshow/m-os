import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "@fontsource/schibsted-grotesk/400.css";
import "@fontsource/schibsted-grotesk/500.css";
import "@fontsource/schibsted-grotesk/700.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
// Os tokens agora moram no pacote, e nao mais no caminho do handoff. A pasta
// `Design System/` continua sendo o arquivo da entrega do designer — o que
// muda e de onde o codigo em execucao le. Ver ADR-033.
import "../../../packages/design-system/all.css";
import App from "./App";

// Qual das janelas do M/OS este documento e.
//
// As janelinhas de sobreposicao ("lembrete", "reuniao-detectada") sao
// `transparent: true` no `tauri.conf.json`, mas o `body` pinta `--canvas` — e a
// margem que existe para a sombra do cartao aparecia como uma TARJA PRETA em
// volta dele, sobre o CAD. O rotulo precisa estar no documento antes do
// primeiro render, senao a tarja pisca.
const janela = getCurrentWindow().label;
document.documentElement.dataset.janela = janela;

// O caderno de ocorrencias, do lado de ca.
//
// Ate 2026-08-25 uma janela do M/OS podia abrir quebrada e nao sobrava nada:
// nem `stderr` (o autostart do logon nao tem terminal), nem tela (a janelinha
// do canto tem 420 pixels e nenhum lugar onde caber um erro). O que a pessoa
// via era "abriu com erro" — e era isso que chegava ao conserto.
//
// Estas tres linhas nao consertam nada. Elas garantem que a PROXIMA vez deixe
// uma linha em `%APPDATA%/com.codedbym.mos/logs/ocorrencias.log`, com hora,
// janela e causa. Ver `src-tauri/src/diagnostico.rs`.
//
// `void` em toda chamada, e nunca `await`: um registro que falha nao pode
// derrubar quem estava tentando registrar.
function registrar(nivel: "fatal" | "erro" | "aviso", mensagem: string) {
  void invoke("diagnostico_registrar", { nivel, origem: janela, mensagem }).catch(() => undefined);
}

window.addEventListener("error", (evento) => {
  // `evento.error` some quando o erro veio de outro origin; o `message` fica.
  const causa = evento.error instanceof Error
    ? `${evento.error.name}: ${evento.error.message} | ${evento.error.stack ?? ""}`
    : evento.message;
  registrar("erro", `${causa} (${evento.filename}:${evento.lineno})`);
});

window.addEventListener("unhandledrejection", (evento) => {
  const causa = evento.reason instanceof Error
    ? `${evento.reason.name}: ${evento.reason.message} | ${evento.reason.stack ?? ""}`
    : String(evento.reason);
  registrar("erro", `promessa rejeitada sem catch: ${causa}`);
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <App />,
);

// "Esta janela montou."
//
// Depois do `render`, e nao antes: o que interessa nao e o script ter rodado, e
// sim a interface ter chegado na tela. O vigia do lado do Rust espera doze
// segundos por este aviso e, se ele nao vier, grava QUAL janela ficou muda e em
// que endereco ela estava — que e a unica prova possivel de uma janelinha que
// abre mostrando erro de carregamento.
void invoke("diagnostico_janela_viva", { rotulo: janela }).catch(() => undefined);
