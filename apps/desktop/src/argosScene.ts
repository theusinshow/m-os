import type { ArgosPose } from "./argosPose";
import { sceneParamsFor } from "./argosPose";

/**
 * Argos com corpo (ADR-048).
 *
 * Este modulo e dono do WebGL e nao sabe que React existe — e a mesma divisao
 * que `plotGeometry.ts` e `argosPose.ts` ja praticam, com o motor de um lado e a
 * casca do outro. Quem decide a pose e o `useArgosPose`; aqui so se desenha.
 *
 * **O laco e a divida desta ADR.** A ADR-041 proibia laco, e a ADR-048 abriu a
 * excecao pagando com as pausas abaixo: sem foco nao desenha, oculto nao
 * desenha, e com `prefers-reduced-motion` nao ha laco nenhum — um quadro por
 * pose e congela.
 *
 * O ruido do corpo e soma de senos, e nao simplex de terceiro. Para um bicho de
 * 72px a diferenca nao se ve, e tres senos nao trazem GLSL de outra licenca
 * para dentro do repo — o mesmo cuidado que fez a ADR-041 recusar o desenho da
 * referencia.
 */

export type ArgosScene = {
  setPose(pose: ArgosPose): void;
  /** Ponteiro em coordenadas normalizadas, de -1 a 1, com a origem no centro. */
  setPointer(x: number, y: number): void;
  setCores(corpo: string, olho: string): void;
  pause(): void;
  resume(): void;
  dispose(): void;
};

const VERTEX_ONDA = `
  uniform float uTempo;
  uniform float uDeformacao;

  // Soma de senos em tres frequencias primas entre si: o batimento demora a se
  // repetir, e e isso que faz o corpo parecer vivo em vez de pulsante.
  float onda(vec3 p, float t) {
    return sin(p.x * 3.0 + t)
         * sin(p.y * 2.7 + t * 1.3)
         * sin(p.z * 3.3 + t * 0.7);
  }
`;

export async function criarCena(
  canvas: HTMLCanvasElement,
  reduzido: boolean,
): Promise<ArgosScene | null> {
  // Import dinamico: os ~150KB do `three` so sao pagos por quem chega a montar
  // o bicho. O boot nao paga (UX-PRINCIPLES §51).
  const THREE = await import("three");

  let renderer: import("three").WebGLRenderer;
  try {
    renderer = new THREE.WebGLRenderer({
      canvas,
      alpha: true,
      antialias: true,
      powerPreference: "low-power",
    });
  } catch {
    // Driver velho, VM, sessao remota. Quem chamou cai para o SVG.
    return null;
  }

  // Capado em 2: acima disso o custo cresce e a diferenca some num corpo de 72px.
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.setSize(canvas.clientWidth, canvas.clientHeight, false);

  const cena = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(40, 1, 0.1, 10);
  camera.position.z = 3.4;

  const uniforms = {
    uTempo: { value: 0 },
    uDeformacao: { value: 0.06 },
  };

  const material = new THREE.MeshStandardMaterial({
    color: 0xffffff,
    roughness: 0.42,
    metalness: 0.05,
  });

  material.onBeforeCompile = (shader) => {
    shader.uniforms.uTempo = uniforms.uTempo;
    shader.uniforms.uDeformacao = uniforms.uDeformacao;
    shader.vertexShader = VERTEX_ONDA + shader.vertexShader;
    shader.vertexShader = shader.vertexShader.replace(
      "#include <begin_vertex>",
      `
        #include <begin_vertex>
        float deslocamento = onda(normalize(position), uTempo) * uDeformacao;
        transformed += normal * deslocamento;
      `,
    );
  };

  // 32 subdivisoes: abaixo disso a onda facetiza e o corpo deixa de ler como mole.
  const corpo = new THREE.Mesh(new THREE.IcosahedronGeometry(1, 32), material);
  cena.add(corpo);

  const olhoMaterial = new THREE.MeshBasicMaterial({ color: 0x000000 });
  const olhoGeometria = new THREE.SphereGeometry(0.17, 24, 24);
  const olhos = [-0.33, 0.33].map((x) => {
    const olho = new THREE.Mesh(olhoGeometria, olhoMaterial);
    olho.position.set(x, 0.08, 0.92);
    corpo.add(olho);
    return olho;
  });

  cena.add(new THREE.AmbientLight(0xffffff, 1.5));
  const luz = new THREE.DirectionalLight(0xffffff, 2.2);
  luz.position.set(-1.2, 1.6, 2.4);
  cena.add(luz);

  let params = sceneParamsFor("desperto");
  let alvoAbertura = params.abertura;
  let alvoRecuo = params.recuo;
  let ponteiro = { x: 0, y: 0 };
  let quadro = 0;
  let rodando = false;
  let ultimoInstante = 0;
  let descartado = false;

  const desenhar = (agora: number) => {
    const delta = ultimoInstante ? (agora - ultimoInstante) / 1000 : 0;
    ultimoInstante = agora;

    uniforms.uTempo.value += delta * params.velocidade;
    uniforms.uDeformacao.value += (params.deformacao - uniforms.uDeformacao.value) * 0.08;

    // Aproximacao exponencial em vez de salto: a troca de pose e o unico
    // movimento que a ADR-041 ja permitia, e ela continua sendo uma transicao.
    corpo.position.z += (alvoRecuo - corpo.position.z) * 0.08;
    corpo.rotation.y += (ponteiro.x * 0.35 - corpo.rotation.y) * 0.06;
    corpo.rotation.x += (-ponteiro.y * 0.28 - corpo.rotation.x) * 0.06;

    for (const olho of olhos) {
      olho.scale.y += (alvoAbertura - olho.scale.y) * 0.14;
    }

    renderer.render(cena, camera);
    if (rodando) quadro = requestAnimationFrame(desenhar);
  };

  const umQuadro = () => {
    ultimoInstante = 0;
    desenhar(performance.now());
  };

  const api: ArgosScene = {
    setPose(pose) {
      params = sceneParamsFor(pose);
      alvoAbertura = params.abertura;
      alvoRecuo = params.recuo;
      // Sob movimento reduzido nao ha laco: cada pose desenha uma vez e para.
      if (reduzido) umQuadro();
    },
    setPointer(x, y) {
      if (reduzido) return;
      ponteiro = { x, y };
    },
    setCores(cor, olho) {
      material.color.set(cor);
      olhoMaterial.color.set(olho);
      if (reduzido) umQuadro();
    },
    pause() {
      rodando = false;
      cancelAnimationFrame(quadro);
    },
    resume() {
      if (reduzido || rodando || descartado) return;
      rodando = true;
      ultimoInstante = 0;
      quadro = requestAnimationFrame(desenhar);
    },
    dispose() {
      descartado = true;
      rodando = false;
      cancelAnimationFrame(quadro);
      corpo.geometry.dispose();
      olhoGeometria.dispose();
      material.dispose();
      olhoMaterial.dispose();
      renderer.dispose();
    },
  };

  umQuadro();
  return api;
}
