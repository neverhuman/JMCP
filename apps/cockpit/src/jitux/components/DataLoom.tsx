import { Canvas } from "@react-three/fiber";
import { PointMaterial, Points } from "@react-three/drei";
import { useEffect, useMemo, useState } from "react";

function hasWebGl(): boolean {
  if (typeof document === "undefined") {
    return false;
  }
  if (typeof navigator !== "undefined" && navigator.userAgent.toLowerCase().includes("jsdom")) {
    return false;
  }
  try {
    const canvas = document.createElement("canvas");
    return Boolean(canvas.getContext("webgl") ?? canvas.getContext("experimental-webgl"));
  } catch {
    return false;
  }
}

function LoomPoints() {
  const positions = useMemo(() => {
    const data: number[] = [];
    for (let index = 0; index < 72; index += 1) {
      const angle = index * 0.44;
      const radius = 0.9 + (index % 9) * 0.12;
      data.push(Math.cos(angle) * radius, Math.sin(angle * 0.6) * 0.35, Math.sin(angle) * radius);
    }
    return new Float32Array(data);
  }, []);

  return (
    <Points positions={positions} stride={3} frustumCulled={false}>
      <PointMaterial transparent color="#7c3cff" size={0.035} sizeAttenuation depthWrite={false} opacity={0.46} />
    </Points>
  );
}

export function DataLoom() {
  const [enabled, setEnabled] = useState(false);

  useEffect(() => {
    setEnabled(hasWebGl());
  }, []);

  return (
    <div className="data-loom" aria-hidden="true">
      {enabled && (
        <Canvas frameloop="demand" camera={{ position: [0, 0, 4.5], fov: 45 }}>
          <ambientLight intensity={0.65} />
          <LoomPoints />
        </Canvas>
      )}
    </div>
  );
}
