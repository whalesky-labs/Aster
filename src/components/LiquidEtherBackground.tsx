import { useEffect, useRef } from "react";
import { Mesh, Program, Renderer, Triangle } from "ogl";

type LiquidEtherBackgroundProps = {
  className?: string;
};

const vertexShader = `
attribute vec2 position;
varying vec2 vUv;

void main() {
  vUv = position * 0.5 + 0.5;
  gl_Position = vec4(position, 0.0, 1.0);
}
`;

const fragmentShader = `
precision highp float;

uniform float uTime;
uniform vec2 uResolution;
varying vec2 vUv;

mat2 rotate2d(float angle) {
  float s = sin(angle);
  float c = cos(angle);
  return mat2(c, -s, s, c);
}

float ether(vec2 p) {
  float value = 0.0;
  float amplitude = 0.56;
  float frequency = 1.0;

  for (int i = 0; i < 5; i++) {
    p *= rotate2d(0.42 + float(i) * 0.16);
    value += amplitude * sin(p.x * frequency + uTime * (0.42 + float(i) * 0.09));
    value += amplitude * cos(p.y * frequency * 1.12 - uTime * (0.34 + float(i) * 0.07));
    p += vec2(sin(p.y + uTime * 0.18), cos(p.x - uTime * 0.16)) * 0.32;
    frequency *= 1.68;
    amplitude *= 0.54;
  }

  return value;
}

void main() {
  vec2 uv = vUv;
  vec2 aspect = vec2(uResolution.x / uResolution.y, 1.0);
  vec2 p = (uv - 0.5) * aspect * 2.15;

  float fieldA = ether(p + vec2(-0.18, 0.06));
  float fieldB = ether(p * 1.28 + vec2(1.7, -0.6));
  float flow = smoothstep(-0.26, 0.96, fieldA * 0.56 + fieldB * 0.34);
  float vein = smoothstep(0.18, 0.92, sin(fieldA * 2.2 + fieldB * 1.4));
  float glow = 1.0 - smoothstep(0.12, 1.18, length(p - vec2(-0.72, 0.38)));

  vec3 base = vec3(0.914, 0.946, 0.972);
  vec3 mint = vec3(0.690, 0.895, 0.842);
  vec3 blue = vec3(0.530, 0.710, 0.960);
  vec3 ink = vec3(0.210, 0.280, 0.360);
  vec3 light = vec3(0.985, 0.995, 1.000);

  vec3 color = mix(base, mint, flow * 0.58);
  color = mix(color, blue, vein * 0.26);
  color = mix(color, light, glow * 0.38);
  color = mix(color, ink, smoothstep(1.05, 1.76, length(p)) * 0.08);
  color += 0.028 * sin(vec3(0.0, 1.2, 2.4) + fieldA + uTime * 0.22);

  gl_FragColor = vec4(color, 1.0);
}
`;

export function LiquidEtherBackground({ className }: LiquidEtherBackgroundProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const renderer = new Renderer({
      alpha: true,
      antialias: true,
      dpr: Math.min(window.devicePixelRatio, 2),
    });
    const gl = renderer.gl;
    gl.canvas.setAttribute("aria-hidden", "true");
    gl.canvas.className = "liquid-ether-canvas";
    container.appendChild(gl.canvas);

    const geometry = new Triangle(gl);
    const program = new Program(gl, {
      vertex: vertexShader,
      fragment: fragmentShader,
      uniforms: {
        uTime: { value: 0 },
        uResolution: { value: [1, 1] },
      },
    });
    const mesh = new Mesh(gl, { geometry, program });

    let animationId = 0;
    let disposed = false;

    function resize() {
      if (!container) return;
      const width = container.clientWidth;
      const height = container.clientHeight;
      renderer.setSize(width, height);
      program.uniforms.uResolution.value = [width, height];
    }

    function animate(time: number) {
      if (disposed) return;
      program.uniforms.uTime.value = time * 0.001;
      renderer.render({ scene: mesh });
      animationId = window.requestAnimationFrame(animate);
    }

    const observer = new ResizeObserver(resize);
    observer.observe(container);
    resize();
    animationId = window.requestAnimationFrame(animate);

    return () => {
      disposed = true;
      observer.disconnect();
      window.cancelAnimationFrame(animationId);
      gl.getExtension("WEBGL_lose_context")?.loseContext();
      gl.canvas.remove();
    };
  }, []);

  return <div className={className} ref={containerRef} />;
}
