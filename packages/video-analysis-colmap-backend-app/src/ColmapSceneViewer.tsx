import { useEffect, useRef } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";

import type { SurfaceResponse } from "@moritzbrantner/video-analysis-ui/package-surface";

interface ColmapScene {
  cameras?: Array<{ id: number; name: string; position: [number, number, number]; forward: [number, number, number] }>;
  cameraPath?: Array<[number, number, number]>;
  points?: Array<{ id: number; position: [number, number, number]; color: [number, number, number]; error?: number; trackLength?: number }>;
  bounds?: { min: [number, number, number]; max: [number, number, number] };
}

export function ColmapSceneViewer({ response }: { response: SurfaceResponse | null }) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const scene = extractScene(response);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !scene) {
      return;
    }

    const renderer = new THREE.WebGLRenderer({ antialias: true, canvas });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setClearColor(0xffffff, 1);

    const root = new THREE.Scene();
    root.add(new THREE.GridHelper(10, 10, 0xd4d4d8, 0xe4e4e7));
    root.add(new THREE.AxesHelper(1.5));

    const camera = new THREE.PerspectiveCamera(55, 1, 0.01, 10000);
    const controls = new OrbitControls(camera, canvas);
    controls.enableDamping = true;

    const pointsGeometry = new THREE.BufferGeometry();
    const points = scene.points ?? [];
    if (points.length > 0) {
      const pointPositions = new Float32Array(points.length * 3);
      const pointColors = new Float32Array(points.length * 3);
      points.forEach((point, index) => {
        pointPositions.set(point.position, index * 3);
        pointColors.set(
          [
            (point.color?.[0] ?? 255) / 255,
            (point.color?.[1] ?? 255) / 255,
            (point.color?.[2] ?? 255) / 255,
          ],
          index * 3,
        );
      });
      pointsGeometry.setAttribute("position", new THREE.BufferAttribute(pointPositions, 3));
      pointsGeometry.setAttribute("color", new THREE.BufferAttribute(pointColors, 3));
      root.add(
        new THREE.Points(
          pointsGeometry,
          new THREE.PointsMaterial({
            size: fitSize(scene.bounds) * 0.006,
            sizeAttenuation: true,
            vertexColors: true,
          }),
        ),
      );
    }

    const cameraMaterial = new THREE.LineBasicMaterial({ color: 0x0f766e });
    for (const view of scene.cameras ?? []) {
      root.add(cameraFrustum(view.position, view.forward, fitSize(scene.bounds) * 0.04, cameraMaterial));
    }

    if ((scene.cameraPath?.length ?? 0) > 1) {
      const pathGeometry = new THREE.BufferGeometry().setFromPoints(scene.cameraPath!.map(vector));
      root.add(new THREE.Line(pathGeometry, new THREE.LineBasicMaterial({ color: 0x2563eb })));
    }

    const resizeObserver = new ResizeObserver(() => {
      const rect = canvas.getBoundingClientRect();
      renderer.setSize(rect.width, rect.height, false);
      camera.aspect = rect.width / Math.max(rect.height, 1);
      camera.updateProjectionMatrix();
    });
    resizeObserver.observe(canvas);

    fitCamera(camera, controls, scene.bounds);

    let frame = 0;
    const render = () => {
      frame = requestAnimationFrame(render);
      controls.update();
      renderer.render(root, camera);
    };
    render();

    return () => {
      cancelAnimationFrame(frame);
      resizeObserver.disconnect();
      controls.dispose();
      pointsGeometry.dispose();
      root.traverse((object) => {
        const mesh = object as THREE.Mesh;
        mesh.geometry?.dispose?.();
      });
      renderer.dispose();
    };
  }, [scene]);

  if (!response) {
    return <ViewerState message="Run COLMAP to render the sparse scene." />;
  }
  if (!scene) {
    return <ViewerState message="The response does not include scene data." />;
  }
  if (!scene.points?.length && !scene.cameras?.length) {
    return <ViewerState message="No cameras or sparse points are available yet." />;
  }

  return (
    <div className="mt-4 overflow-hidden rounded-md border border-zinc-200 bg-white">
      <canvas ref={canvasRef} className="block h-[32rem] w-full" aria-label="COLMAP sparse 3D scene" />
      <div className="flex flex-wrap gap-4 border-t border-zinc-200 px-4 py-3 text-xs text-zinc-600">
        <span>{scene.cameras?.length ?? 0} cameras</span>
        <span>{scene.points?.length ?? 0} sparse points</span>
      </div>
    </div>
  );
}

function ViewerState({ message }: { message: string }) {
  return (
    <div className="mt-4 rounded-md border border-zinc-200 bg-zinc-50 p-6 text-sm text-zinc-600">
      {message}
    </div>
  );
}

function extractScene(response: SurfaceResponse | null): ColmapScene | null {
  const value = response?.value;
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  const scene = (value as { scene?: unknown }).scene;
  if (!scene || typeof scene !== "object" || Array.isArray(scene)) {
    return null;
  }
  return scene as ColmapScene;
}

function vector(value: [number, number, number]): THREE.Vector3 {
  return new THREE.Vector3(value[0], value[1], value[2]);
}

function fitSize(bounds?: ColmapScene["bounds"]): number {
  if (!bounds) {
    return 1;
  }
  const min = vector(bounds.min);
  const max = vector(bounds.max);
  return Math.max(max.distanceTo(min), 1);
}

function fitCamera(camera: THREE.PerspectiveCamera, controls: OrbitControls, bounds?: ColmapScene["bounds"]) {
  const min = bounds ? vector(bounds.min) : new THREE.Vector3(-1, -1, -1);
  const max = bounds ? vector(bounds.max) : new THREE.Vector3(1, 1, 1);
  const center = min.clone().add(max).multiplyScalar(0.5);
  const size = Math.max(max.distanceTo(min), 1);
  camera.position.copy(center.clone().add(new THREE.Vector3(size, size * 0.7, size)));
  camera.near = size / 1000;
  camera.far = size * 1000;
  camera.updateProjectionMatrix();
  controls.target.copy(center);
  controls.update();
}

function cameraFrustum(
  position: [number, number, number],
  forward: [number, number, number],
  size: number,
  material: THREE.LineBasicMaterial,
): THREE.LineSegments {
  const center = vector(position);
  const direction = vector(forward).normalize();
  const upHint = Math.abs(direction.y) > 0.9 ? new THREE.Vector3(1, 0, 0) : new THREE.Vector3(0, 1, 0);
  const right = new THREE.Vector3().crossVectors(upHint, direction).normalize();
  const up = new THREE.Vector3().crossVectors(direction, right).normalize();
  const tip = center.clone().add(direction.multiplyScalar(size * 1.8));
  const corners = [
    tip.clone().add(right.clone().multiplyScalar(size)).add(up.clone().multiplyScalar(size * 0.65)),
    tip.clone().add(right.clone().multiplyScalar(-size)).add(up.clone().multiplyScalar(size * 0.65)),
    tip.clone().add(right.clone().multiplyScalar(-size)).add(up.clone().multiplyScalar(-size * 0.65)),
    tip.clone().add(right.clone().multiplyScalar(size)).add(up.clone().multiplyScalar(-size * 0.65)),
  ];
  const vertices = [
    center,
    corners[0],
    center,
    corners[1],
    center,
    corners[2],
    center,
    corners[3],
    corners[0],
    corners[1],
    corners[1],
    corners[2],
    corners[2],
    corners[3],
    corners[3],
    corners[0],
  ];
  return new THREE.LineSegments(new THREE.BufferGeometry().setFromPoints(vertices), material);
}
