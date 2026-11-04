"use client";

// The talk key as a real object: a rounded keycap with a dished top, lit from
// the top-left, that tilts towards the pointer and sinks when pressed.
import { ContactShadows, RoundedBox } from "@react-three/drei";
import { Canvas, useFrame } from "@react-three/fiber";
import { useMemo, useRef } from "react";
import * as THREE from "three";

const UMBER = "#2a1206";
const UMBER_TOP = "#3b1a0b";
const APRICOT = "#ffb88a";

/** The legend ("Ctrl", "Right") drawn in the page's own Anybody font. */
function useLegend() {
  return useMemo(() => {
    const canvas = document.createElement("canvas");
    canvas.width = 1024;
    canvas.height = 768;
    const ctx = canvas.getContext("2d")!;
    const family = getComputedStyle(document.documentElement).getPropertyValue("--font-anybody").trim() || "sans-serif";
    ctx.fillStyle = APRICOT;
    ctx.textBaseline = "top";
    ctx.font = `800 260px ${family}`;
    ctx.fillText("Ctrl", 96, 90);
    ctx.globalAlpha = 0.75;
    ctx.font = `600 120px ${family}`;
    ctx.textAlign = "right";
    ctx.textBaseline = "bottom";
    ctx.fillText("Right", 928, 680);
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.anisotropy = 8;
    return texture;
  }, []);
}

function Key({ down }: { down: boolean }) {
  const group = useRef<THREE.Group>(null);
  const legend = useLegend();
  const travel = useRef(0);

  useFrame((state, delta) => {
    const g = group.current;
    if (!g) return;
    // Press travel: quick down, springy up.
    const target = down ? -0.22 : 0;
    travel.current += (target - travel.current) * Math.min(1, delta * (down ? 30 : 14));
    g.position.y = travel.current;
    // Tilt gently towards the pointer.
    const { x, y } = state.pointer;
    g.rotation.x += (0.62 - y * 0.12 - g.rotation.x) * Math.min(1, delta * 4);
    g.rotation.y += (-0.32 + x * 0.25 - g.rotation.y) * Math.min(1, delta * 4);
  });

  return (
    <group ref={group} rotation={[0.62, -0.32, 0]}>
      {/* Skirt */}
      <RoundedBox args={[2.6, 1.0, 2.0]} radius={0.28} smoothness={6} castShadow>
        <meshStandardMaterial color={UMBER} roughness={0.55} metalness={0.05} />
      </RoundedBox>
      {/* Top face, slightly inset and lighter */}
      <RoundedBox args={[2.2, 0.2, 1.62]} radius={0.12} smoothness={6} position={[0, 0.48, -0.04]}>
        <meshStandardMaterial color={UMBER_TOP} roughness={0.42} metalness={0.05} />
      </RoundedBox>
      {/* Legend */}
      <mesh position={[0, 0.64, -0.04]} rotation={[-Math.PI / 2, 0, 0]}>
        <planeGeometry args={[2.0, 1.5]} />
        <meshBasicMaterial map={legend} transparent toneMapped={false} />
      </mesh>
    </group>
  );
}

export default function KeyCap3D({ down }: { down: boolean }) {
  return (
    <Canvas
      dpr={[1, 2]}
      camera={{ position: [0, 0.2, 7], fov: 30 }}
      gl={{ alpha: true, antialias: true }}
      style={{ position: "absolute", inset: 0, pointerEvents: "none" }}
      aria-hidden="true"
    >
      <ambientLight intensity={0.55} color="#ffd9c2" />
      <directionalLight position={[-3, 5, 3]} intensity={2.2} color="#fff1e6" />
      <directionalLight position={[4, 2, -2]} intensity={0.6} color="#ff5a1f" />
      <Key down={down} />
      <ContactShadows position={[0, -1.15, 0]} opacity={0.5} scale={7} blur={2.6} far={2} color="#2a1206" />
    </Canvas>
  );
}
