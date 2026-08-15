import ReactDOM from "react-dom/client";
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

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <App />,
);
