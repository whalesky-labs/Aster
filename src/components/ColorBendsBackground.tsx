import { useEffect, useMemo, useRef } from "react";
import { Mesh, Program, Renderer, Triangle } from "ogl";

const MAX_COLORS = 8;
const DEFAULT_COLORS: string[] = [];

type ColorBendsBackgroundProps = {
  autoRotate?: number;
  bandWidth?: number;
  className?: string;
  colors?: string[];
  frequency?: number;
  intensity?: number;
  iterations?: number;
  mouseInfluence?: number;
  noise?: number;
  parallax?: number;
  rotation?: number;
  scale?: number;
  speed?: number;
  transparent?: boolean;
  warpStrength?: number;
};

const vertexShader = `
attribute vec2 position;
varying vec2 vUv;

void main() {
  vUv = position * 0.5 + 0.5;
  gl_Position = vec4(position, 0.0, 1.0);
}
`;

// Adapted from ReactBits Color Bends for this app's existing OGL renderer.
const fragmentShader = `
precision highp float;

#define MAX_COLORS ${MAX_COLORS}

uniform vec2 uCanvas;
uniform float uTime;
uniform float uSpeed;
uniform vec2 uRot;
uniform int uColorCount;
uniform vec3 uColors[MAX_COLORS];
uniform int uTransparent;
uniform float uScale;
uniform float uFrequency;
uniform float uWarpStrength;
uniform vec2 uPointer;
uniform float uMouseInfluence;
uniform float uParallax;
uniform float uNoise;
uniform int uIterations;
uniform float uIntensity;
uniform float uBandWidth;
varying vec2 vUv;

void main() {
  float t = uTime * uSpeed;
  vec2 p = vUv * 2.0 - 1.0;
  p += uPointer * uParallax * 0.1;
  vec2 rp = vec2(p.x * uRot.x - p.y * uRot.y, p.x * uRot.y + p.y * uRot.x);
  vec2 q = vec2(rp.x * (uCanvas.x / uCanvas.y), rp.y);
  q /= max(uScale, 0.0001);
  q /= 0.5 + 0.2 * dot(q, q);
  q += 0.2 * cos(t) - 7.56;
  q += (uPointer - rp) * uMouseInfluence * 0.2;

  for (int j = 0; j < 5; j++) {
    if (j >= uIterations - 1) break;
    vec2 rr = sin(1.5 * (q.yx * uFrequency) + 2.0 * cos(q * uFrequency));
    q += (rr - q) * 0.15;
  }

  vec3 col = vec3(0.0);
  float a = 1.0;

  if (uColorCount > 0) {
    vec2 s = q;
    vec3 sumCol = vec3(0.0);
    float cover = 0.0;
    for (int i = 0; i < MAX_COLORS; ++i) {
      if (i >= uColorCount) break;
      s -= 0.01;
      vec2 r = sin(1.5 * (s.yx * uFrequency) + 2.0 * cos(s * uFrequency));
      float m0 = length(r + sin(5.0 * r.y * uFrequency - 3.0 * t + float(i)) / 4.0);
      float kBelow = clamp(uWarpStrength, 0.0, 1.0);
      float kMix = pow(kBelow, 0.3);
      float gain = 1.0 + max(uWarpStrength - 1.0, 0.0);
      vec2 warped = s + (r - s) * kBelow * gain;
      float m1 = length(warped + sin(5.0 * warped.y * uFrequency - 3.0 * t + float(i)) / 4.0);
      float m = mix(m0, m1, kMix);
      float w = 1.0 - exp(-uBandWidth / exp(uBandWidth * m));
      sumCol += uColors[i] * w;
      cover = max(cover, w);
    }
    col = clamp(sumCol, 0.0, 1.0);
    a = uTransparent > 0 ? cover : 1.0;
  } else {
    vec2 s = q;
    for (int k = 0; k < 3; ++k) {
      s -= 0.01;
      vec2 r = sin(1.5 * (s.yx * uFrequency) + 2.0 * cos(s * uFrequency));
      float m0 = length(r + sin(5.0 * r.y * uFrequency - 3.0 * t + float(k)) / 4.0);
      float kBelow = clamp(uWarpStrength, 0.0, 1.0);
      float kMix = pow(kBelow, 0.3);
      float gain = 1.0 + max(uWarpStrength - 1.0, 0.0);
      vec2 warped = s + (r - s) * kBelow * gain;
      float m1 = length(warped + sin(5.0 * warped.y * uFrequency - 3.0 * t + float(k)) / 4.0);
      float m = mix(m0, m1, kMix);
      col[k] = 1.0 - exp(-uBandWidth / exp(uBandWidth * m));
    }
    a = uTransparent > 0 ? max(max(col.r, col.g), col.b) : 1.0;
  }

  col *= uIntensity;

  if (uNoise > 0.0001) {
    float n = fract(sin(dot(gl_FragCoord.xy + vec2(uTime), vec2(12.9898, 78.233))) * 43758.5453123);
    col += (n - 0.5) * uNoise;
    col = clamp(col, 0.0, 1.0);
  }

  vec3 rgb = (uTransparent > 0) ? col * a : col;
  gl_FragColor = vec4(rgb, a);
}
`;

function hexToRgb(hex: string) {
  const normalized = hex.replace("#", "").trim();
  const value =
    normalized.length === 3
      ? normalized
          .split("")
          .map((char) => char + char)
          .join("")
      : normalized;
  const numeric = Number.parseInt(value.padEnd(6, "0").slice(0, 6), 16);
  return [
    ((numeric >> 16) & 255) / 255,
    ((numeric >> 8) & 255) / 255,
    (numeric & 255) / 255,
  ];
}

function normalizeColors(colors: string[]) {
  return Array.from({ length: MAX_COLORS }, (_, index) =>
    hexToRgb(colors[index] ?? "#000000"),
  );
}

export function ColorBendsBackground({
  autoRotate = 0,
  bandWidth = 6,
  className,
  colors = DEFAULT_COLORS,
  frequency = 1,
  intensity = 1.5,
  iterations = 1,
  mouseInfluence = 1,
  noise = 0.15,
  parallax = 0.5,
  rotation = 90,
  scale = 1,
  speed = 0.2,
  transparent = true,
  warpStrength = 1,
}: ColorBendsBackgroundProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const pointerTargetRef = useRef<[number, number]>([0, 0]);
  const pointerCurrentRef = useRef<[number, number]>([0, 0]);
  const colorUniforms = useMemo(() => normalizeColors(colors), [colors]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const backgroundElement = container;

    const renderer = new Renderer({
      alpha: true,
      antialias: false,
      dpr: Math.min(window.devicePixelRatio || 1, 2),
    });
    const gl = renderer.gl;
    gl.canvas.setAttribute("aria-hidden", "true");
    gl.canvas.className = "color-bends-canvas";
    backgroundElement.appendChild(gl.canvas);

    const geometry = new Triangle(gl);
    const program = new Program(gl, {
      vertex: vertexShader,
      fragment: fragmentShader,
      depthTest: false,
      depthWrite: false,
      uniforms: {
        uBandWidth: { value: bandWidth },
        uCanvas: { value: [1, 1] },
        uColorCount: { value: Math.min(colors.length, MAX_COLORS) },
        uColors: { value: colorUniforms },
        uFrequency: { value: frequency },
        uIntensity: { value: intensity },
        uIterations: { value: iterations },
        uMouseInfluence: { value: mouseInfluence },
        uNoise: { value: noise },
        uParallax: { value: parallax },
        uPointer: { value: [0, 0] },
        uRot: { value: [1, 0] },
        uScale: { value: scale },
        uSpeed: { value: speed },
        uTime: { value: 0 },
        uTransparent: { value: transparent ? 1 : 0 },
        uWarpStrength: { value: warpStrength },
      },
      transparent,
    });
    const mesh = new Mesh(gl, { geometry, program });

    let animationId = 0;
    let disposed = false;
    let lastTime = performance.now();

    function resize() {
      const width = Math.max(backgroundElement.clientWidth, 1);
      const height = Math.max(backgroundElement.clientHeight, 1);
      renderer.setSize(width, height);
      program.uniforms.uCanvas.value = [width, height];
    }

    function handlePointerMove(event: PointerEvent) {
      const rect = backgroundElement.getBoundingClientRect();
      const x = ((event.clientX - rect.left) / (rect.width || 1)) * 2 - 1;
      const y = -(((event.clientY - rect.top) / (rect.height || 1)) * 2 - 1);
      pointerTargetRef.current = [x, y];
    }

    function animate(time: number) {
      if (disposed) return;
      const elapsed = time * 0.001;
      const delta = Math.min((time - lastTime) * 0.001, 0.1);
      lastTime = time;

      const degrees = ((rotation % 360) + autoRotate * elapsed) * (Math.PI / 180);
      program.uniforms.uRot.value = [Math.cos(degrees), Math.sin(degrees)];

      const current = pointerCurrentRef.current;
      const target = pointerTargetRef.current;
      const amount = Math.min(1, delta * 8);
      current[0] += (target[0] - current[0]) * amount;
      current[1] += (target[1] - current[1]) * amount;
      program.uniforms.uPointer.value = current;
      program.uniforms.uTime.value = elapsed;

      renderer.render({ scene: mesh });
      animationId = window.requestAnimationFrame(animate);
    }

    const observer = new ResizeObserver(resize);
    observer.observe(backgroundElement);
    window.addEventListener("pointermove", handlePointerMove);
    resize();
    animationId = window.requestAnimationFrame(animate);

    return () => {
      disposed = true;
      observer.disconnect();
      window.removeEventListener("pointermove", handlePointerMove);
      window.cancelAnimationFrame(animationId);
      program.remove();
      geometry.remove();
      gl.getExtension("WEBGL_lose_context")?.loseContext();
      gl.canvas.remove();
    };
  }, [
    autoRotate,
    bandWidth,
    colorUniforms,
    colors.length,
    frequency,
    intensity,
    iterations,
    mouseInfluence,
    noise,
    parallax,
    rotation,
    scale,
    speed,
    warpStrength,
    transparent,
  ]);

  return <div className={className} ref={containerRef} />;
}
