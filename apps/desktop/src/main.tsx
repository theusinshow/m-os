import ReactDOM from "react-dom/client";
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
document.documentElement.dataset.janela = getCurrentWindow().label;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <App />,
);
