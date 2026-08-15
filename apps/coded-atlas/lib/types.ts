/** Opções de captura por geração (sobrescrevem os defaults de lib/config.ts). */
export interface CaptureOptions {
  video?: boolean;    // gravar o vídeo de scroll
  sections?: boolean; // fotografar seções individuais
}

/** Entrada do formulário / corpo da requisição. */
export interface ProjectInput {
  url: string;
  name: string;
  slug: string;
  category: string;
  client?: string;
  description?: string;
  options?: CaptureOptions; // ausência = usa os defaults do config
  pages?: string[];         // páginas extras (paths "/sobre" ou URLs completas), v1.5
  states?: StateInput[];    // estados de interação (clicar seletor), v1.5
}

/** Estado de interação a capturar: clica um seletor e fotografa (v1.5). */
export interface StateInput {
  name: string;     // "Menu aberto"
  selector: string; // ".menu-toggle"
}

/** Configuração de um viewport de captura. */
export interface ViewportConfig {
  label: string; // "desktop" | "mobile"
  width: number;
  height: number;
  deviceScaleFactor: number;
}

/** Resultado das capturas de um único device. */
export interface DeviceCapture {
  viewport: string; // "1440x900"
  screenshot: string; // caminho público, ex: /generated/slug/screenshots/desktop-1440x900.png
  fullpage: string; // caminho público
}

/** Inspeção da página: paleta, tipografia e tech stack (v0.3). */
export interface SiteInspection {
  colors: string[];     // hex "#1a2b3c"
  fonts: string[];      // "Inter", "Poppins"
  techStack: string[];  // "Next.js", "WordPress"
  ogImage?: string;     // URL da og:image / twitter:image do site
}

/** Estrutura final persistida em catalog.json. */
export interface Catalog {
  version: string; // versão do Atlas que gerou (ex: "0.1.0")
  project: ProjectInput;
  captures: {
    desktop: DeviceCapture;
    mobile: DeviceCapture;
  };
  thumbnails: {
    main: string; // caminho público
    mobile: string; // caminho público
  };
  videos?: {
    desktop?: string;
    mobile?: string;
  };
  sections: CatalogSection[];
  pages?: PageCapture[];       // capturas de páginas extras (v1.5)
  states?: StateCapture[];     // estados de interação capturados (v1.5)
  inspection?: SiteInspection; // v0.3 — paleta, fontes e tech stack
  cover?: {
    image: string;                        // caminho público /generated/.../thumbnails/cover.webp
    source: "og-image" | "smart-crop";   // como foi gerada
  };
  compositions?: CompositionAsset[];     // composições para redes (v1.4)
  mockups?: MockupAsset[];               // mockups com moldura (v1.4)
  meta: CatalogMeta;
  createdAt: string; // new Date().toISOString()
}

/** Composição para redes: a captura sobre fundo da marca num formato pronto (v1.4). */
export interface CompositionAsset {
  name: string;   // "social-1x1"
  label: string;  // "Quadrado (1:1)"
  width: number;
  height: number;
  image: string;  // caminho público
}

/** Mockup: a captura dentro de uma moldura (navegador ou celular) (v1.4). */
export interface MockupAsset {
  name: string;   // "browser" | "phone"
  label: string;  // "Navegador" | "Celular"
  image: string;  // caminho público (PNG com fundo transparente)
}

/** Captura de uma página extra do mesmo site (v1.5): viewport + full page, por device. */
export interface PageCapture {
  path: string;          // entrada original ("/sobre" ou URL completa)
  url: string;           // URL absoluta capturada
  desktop: DeviceCapture;
  mobile: DeviceCapture;
}

/** Estado de interação capturado (v1.5): desktop viewport após clicar o seletor. */
export interface StateCapture {
  name: string;       // "Menu aberto"
  selector: string;   // seletor clicado
  screenshot: string; // caminho público
}

/** Seção capturada por detecção semântica + scroll (v0.3). */
export interface CatalogSection {
  device: "desktop" | "mobile";
  name: string;           // "section-001"
  y: number;              // scroll Y no momento da captura
  height: number;         // altura do viewport
  screenshot: string;     // caminho público
  heading?: string;       // primeiro heading visível no viewport
  // v0.3 — detecção semântica
  suggestedName?: string; // "Hero", "Sobre Nós" — nome derivado do DOM
  semanticTag?: string;   // "header", "section", "footer", "article"
  sectionId?: string;     // valor do id / data-section do elemento
}

/**
 * Fragmento que o portfólio da Coded by M consome para montar `/cases/[slug]`
 * e a Paisagem Digital da Home (v1.0). Derivado inteiramente do Catalog —
 * sem dados novos. Caminhos são públicos (`/generated/...`).
 */
export interface PortfolioManifest {
  slug: string;
  name: string;
  category: string;
  description?: string;
  url: string;
  thumbnail: string;        // thumb principal (640×400)
  thumbnailMobile: string;  // thumb mobile (320×640)
  cover?: string;           // capa 1.91:1, quando existir
  accent?: string;          // cor/acento dominante do site (de inspection.colors[0])
  palette: string[];        // paleta completa detectada
  techStack: string[];
  hasVideo: boolean;        // se há gravação de scroll para usar no case
  date: string;             // YYYY-MM-DD da geração
  atlasVersion: string;
}

/** Metadados da execução — útil para debug e reprocessamento. */
export interface CatalogMeta {
  captureDelayMs: number;
  navTimeoutMs: number;
  durationMs: number; // tempo total da geração
  userAgent: string;
}

/** Passos do pipeline — alimentam o progresso na UI. */
export type CaptureStep =
  | "validating"
  | "launching"
  | "capturing-desktop"
  | "capturing-fullpage-desktop"
  | "capturing-sections-desktop"
  | "recording-video-desktop"
  | "capturing-mobile"
  | "capturing-fullpage-mobile"
  | "capturing-sections-mobile"
  | "recording-video-mobile"
  | "capturing-pages"
  | "capturing-states"
  | "generating-thumbnails"
  | "writing-catalog"
  | "done"
  | "error";

/** Evento de progresso emitido pela engine e transmitido pela API. */
export interface ProgressEvent {
  step: CaptureStep;
  message: string; // texto pronto para exibir (PT-BR)
  progress: number; // 0–100
}

/** Evento final de sucesso. */
export interface ResultEvent {
  step: "done";
  catalog: Catalog;
  projectUrl: string; // /projects/[slug]
}

/** Código de erro estruturado da aplicação. */
export type AtlasErrorCode =
  | "INVALID_URL"
  | "UNREACHABLE"
  | "NAV_TIMEOUT"
  | "BLOCKED"
  | "RENDER_TIMEOUT"
  | "CAPTURE_FAILED"
  | "STORAGE_FAILED"
  | "SLUG_CONFLICT"
  | "SERVER_DOWN"
  | "UNKNOWN";

export interface AtlasErrorPayload {
  step: "error";
  code: AtlasErrorCode;
  message: string; // mensagem amigável em PT-BR
  detail?: string; // detalhe técnico para log (não exibir cru ao usuário)
}
