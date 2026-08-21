/**
 * O que esta plataforma pode fazer.
 *
 * # A regra
 *
 * Pergunte o que a plataforma **pode fazer**, nunca **qual ela é**.
 *
 * ```ts
 * if (plataforma === "ios") mostrarCompartilhar();   // errado
 * if (capacidades().nativeShare) mostrarCompartilhar(); // certo
 * ```
 *
 * A diferença aparece no dia em que o macOS entrar: o primeiro esconde o botão
 * numa plataforma que tem share nativo; o segundo acerta sem ninguém tocar
 * nele. É o §35 da missão multi-device, e é o que impede `if iOS` de se
 * espalhar pelo projeto até ninguém mais conseguir remover uma plataforma.
 *
 * # Por que aqui, e não no Rust
 *
 * Estas são as capacidades que a INTERFACE consulta para decidir o que
 * desenhar. As capacidades que o backend consulta para decidir o que executar
 * vivem do lado Rust, atrás dos serviços de plataforma. Um mesmo fato pode
 * aparecer nos dois lugares — o que não pode é a interface adivinhar por conta
 * própria.
 */

export type Capacidade =
  /** Atalho registrado no sistema operacional inteiro. */
  | "globalShortcut"
  /** Soltar arquivo na janela. */
  | "fileDrop"
  /** Saber quais programas estão abertos. */
  | "processMonitoring"
  /** Capturar o áudio que sai do sistema (loopback). */
  | "systemAudioCapture"
  /** Ícone na bandeja, com o app vivo em segundo plano. */
  | "tray"
  /** Folha de compartilhamento do sistema. */
  | "nativeShare"
  | "haptics"
  | "biometrics"
  | "camera"
  /** Notificação remota, vinda de servidor. */
  | "pushRemote"
  /** Notificação agendada pelo próprio dispositivo. */
  | "localNotifications"
  /** Guardar segredo fora do alcance de leitura comum. */
  | "secureStorage"
  | "microphone";

export type Capacidades = Record<Capacidade, boolean>;

/**
 * O que o Windows tem hoje.
 *
 * Escrito à mão, e não detectado: detecção em tempo de execução responde "o
 * plugin está instalado?", que é outra pergunta. O que interessa aqui é o que
 * o produto decidiu suportar nesta plataforma.
 */
const WINDOWS: Capacidades = {
  globalShortcut: true,
  fileDrop: true,
  processMonitoring: true,
  systemAudioCapture: true,
  tray: true,
  nativeShare: false,
  haptics: false,
  biometrics: false,
  camera: false,
  pushRemote: false,
  localNotifications: true,
  secureStorage: true,
  microphone: true,
};

/**
 * O que o iOS terá.
 *
 * Declarado ANTES de o cliente existir, de propósito: é esta tabela que faz o
 * desktop parar de assumir que toda capacidade existe. Um `false` aqui é o que
 * força a interface a ter um caminho alternativo — e é melhor descobrir isso
 * agora do que na primeira build para iPhone.
 *
 * `pushRemote` está `false` porque não há plugin oficial de APNs no Tauri 2 e
 * a infraestrutura não existe. Vira `true` quando as duas coisas mudarem, e
 * não antes: capacidade prometida e ausente é pior que capacidade ausente.
 */
const IOS: Capacidades = {
  globalShortcut: false,
  fileDrop: false,
  processMonitoring: false,
  systemAudioCapture: false,
  tray: false,
  nativeShare: true,
  haptics: true,
  biometrics: true,
  camera: true,
  pushRemote: false,
  localNotifications: true,
  secureStorage: true,
  microphone: true,
};

export type NomeDePlataforma = "windows" | "ios" | "macos" | "android" | "desconhecida";

/**
 * A plataforma em que este renderer está rodando.
 *
 * Lê do Tauri quando disponível e cai no `userAgent` fora dele — a bancada
 * headless e o Vite no navegador precisam de uma resposta, e travar ali
 * transformaria um teste de layout em erro de plataforma.
 */
export function plataformaAtual(): NomeDePlataforma {
  const tauri = (globalThis as { __TAURI_OS_PLUGIN_INTERNALS__?: { platform?: string } })
    .__TAURI_OS_PLUGIN_INTERNALS__;
  const declarada = tauri?.platform;
  if (declarada === "windows" || declarada === "ios" || declarada === "macos" || declarada === "android") {
    return declarada;
  }
  if (typeof navigator !== "undefined") {
    const agente = navigator.userAgent;
    if (/iPhone|iPad|iPod/i.test(agente)) return "ios";
    if (/Android/i.test(agente)) return "android";
    if (/Mac OS X/i.test(agente)) return "macos";
    if (/Windows/i.test(agente)) return "windows";
  }
  return "desconhecida";
}

/**
 * As capacidades da plataforma corrente.
 *
 * Uma plataforma desconhecida recebe o conjunto **mais restrito** que faz
 * sentido: nada de exclusivo de desktop, nada de exclusivo de mobile. O
 * princípio é o mesmo de todo o resto do M/OS — na dúvida, não prometa.
 */
export function capacidades(plataforma: NomeDePlataforma = plataformaAtual()): Capacidades {
  switch (plataforma) {
    case "windows":
      return WINDOWS;
    case "ios":
      return IOS;
    case "macos":
      // Ainda não é alvo. Herda o desktop menos o que é do Windows, e ganha o
      // share nativo — o suficiente para a interface não mentir se alguém
      // rodar ali antes de a plataforma existir de verdade.
      return { ...WINDOWS, processMonitoring: false, systemAudioCapture: false, nativeShare: true };
    case "android":
      return { ...IOS, biometrics: true };
    default:
      return {
        ...WINDOWS,
        globalShortcut: false,
        fileDrop: false,
        processMonitoring: false,
        systemAudioCapture: false,
        tray: false,
        secureStorage: false,
      };
  }
}

/** Açúcar para o caso de uma capacidade só, que é a maioria. */
export function pode(capacidade: Capacidade, plataforma?: NomeDePlataforma): boolean {
  return capacidades(plataforma)[capacidade];
}
