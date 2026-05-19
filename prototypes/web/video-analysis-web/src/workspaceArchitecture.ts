export type PackageKind = "rust" | "frontend";

export type PackageCapabilityKind = "library" | "cli" | "api" | "ui";

export type PackageDomain =
  | "facade"
  | "apps"
  | "ui"
  | "video"
  | "audio"
  | "image"
  | "text"
  | "vector"
  | "three-d"
  | "comfyui"
  | "data"
  | "bindings"
  | "support";

export interface WorkspaceArchitecturePackage {
  name: string;
  kind: PackageKind;
  domain: PackageDomain;
  path: string | null;
  description: string;
  role: string;
  exposes: string[];
  consumedBy: string[];
  tags: string[];
  capabilities: WorkspaceArchitectureCapability[];
}

export interface WorkspaceArchitectureCapability {
  kind: PackageCapabilityKind;
  entrypoint: string;
}

export interface WorkspaceArchitectureDependency {
  source: string;
  target: string;
  optional: boolean;
}

export interface WorkspaceArchitectureInterop {
  packages: [string, string];
  directDependency: boolean;
  sharedTags: string[];
  reasons: string[];
  strength: number;
}

export interface WorkspaceArchitectureResponse {
  generatedAt: string;
  packages: WorkspaceArchitecturePackage[];
  dependencies: WorkspaceArchitectureDependency[];
  interop: WorkspaceArchitectureInterop[];
}

export interface ContractTagDefinition {
  id: string;
  label: string;
  terms: string[];
}

export const contractTagDefinitions: ContractTagDefinition[] = [
  {
    id: "video_frames",
    label: "video frames",
    terms: ["video frame", "video frames", "ownedvideoframe", "videoframe", "pixel format"],
  },
  {
    id: "audio_frames",
    label: "audio frames",
    terms: ["audio frame", "audio frames", "ownedaudioframe", "audioframe", "audio buffer"],
  },
  {
    id: "text_segments",
    label: "text segments",
    terms: ["text segment", "text segments", "transcript segment", "transcript segments", "text document"],
  },
  {
    id: "scenes",
    label: "scenes and cuts",
    terms: ["scene detector", "scene detectors", "scene", "scenes", "cut", "cuts"],
  },
  {
    id: "observations",
    label: "observations",
    terms: ["observation", "observations", "observationkind"],
  },
  {
    id: "analysis_events",
    label: "analysis events",
    terms: ["analysisevent", "analysis event", "analysis events", "audio events", "text events", "event"],
  },
  {
    id: "data_records",
    label: "data records",
    terms: ["datarecord", "data record", "data records", "stream summaries"],
  },
  {
    id: "data_buckets",
    label: "data buckets",
    terms: ["bucket", "buckets", "bucket summaries"],
  },
  {
    id: "datasets",
    label: "datasets",
    terms: ["dataset", "datasets", "manifest", "record", "records"],
  },
  {
    id: "images",
    label: "images",
    terms: ["image", "images", "pixel", "pixels", "rgb", "jpeg", "png", "webp"],
  },
  {
    id: "masks_and_detections",
    label: "masks and detections",
    terms: ["mask", "masks", "segmentation", "detection", "detections", "bounding box"],
  },
  {
    id: "vectors_and_embeddings",
    label: "vectors and embeddings",
    terms: ["vector", "vectors", "embedding", "embeddings", "cosine similarity"],
  },
  {
    id: "tensors_and_latents",
    label: "tensors and latents",
    terms: ["tensor", "tensors", "latent", "latents", "conditioning"],
  },
  {
    id: "model_predictions",
    label: "model predictions",
    terms: ["prediction", "predictions", "model request", "model requests", "backend", "onnx", "classifier"],
  },
  {
    id: "reports_and_outputs",
    label: "reports and outputs",
    terms: ["report", "reports", "json report", "csv", "html", "writer", "output"],
  },
  {
    id: "comfyui_workflows",
    label: "ComfyUI workflows",
    terms: ["comfyui", "workflow json", "prompt graph", "socket type", "workflow node"],
  },
  {
    id: "comfyui_models",
    label: "ComfyUI models",
    terms: ["model folder", "model ref", "extra model paths", "checkpoints", "loras", "vae"],
  },
  {
    id: "poses_and_keypoints",
    label: "poses and keypoints",
    terms: ["pose", "poses", "posture", "keypoint", "keypoints", "skeleton", "stick figure"],
  },
  {
    id: "three_d_geometry",
    label: "3D geometry",
    terms: ["3d", "mesh", "meshes", "point cloud", "point clouds", "obj", "ply", "gltf", "quaternion"],
  },
  {
    id: "radiance_assets",
    label: "radiance assets",
    terms: ["radiance", "gaussian splat", "gaussian splats", "nerfstudio", "colmap", "camera pose"],
  },
];

export const packageDomainOrder: PackageDomain[] = [
  "facade",
  "apps",
  "ui",
  "video",
  "audio",
  "image",
  "text",
  "vector",
  "three-d",
  "comfyui",
  "data",
  "bindings",
  "support",
];

export const packageDomainLabels: Record<PackageDomain, string> = {
  facade: "Facade",
  apps: "Apps",
  ui: "UI",
  video: "Video",
  audio: "Audio",
  image: "Image",
  text: "Text",
  vector: "Vector",
  "three-d": "3D",
  comfyui: "ComfyUI",
  data: "Data",
  bindings: "Bindings",
  support: "Support",
};

export function packageDomainFor(name: string, path?: string | null): PackageDomain {
  if (name === "video-analysis") {
    return "facade";
  }
  if (name === "@video-analysis/ui") {
    return "ui";
  }
  if (name === "@video-analysis/web") {
    return "apps";
  }
  if (name.startsWith("video-analysis-")) {
    return "video";
  }
  if (name.startsWith("audio-analysis-")) {
    return "audio";
  }
  if (name.startsWith("image-analysis-")) {
    return "image";
  }
  if (name.startsWith("text-analysis-")) {
    return "text";
  }
  if (name.startsWith("vector-analysis-")) {
    return "vector";
  }
  if (name.startsWith("three-d-")) {
    return "three-d";
  }
  if (name.startsWith("comfyui-")) {
    return "comfyui";
  }
  if (
    name === "data-inversion-core" ||
    name === "graph-analysis-core" ||
    name === "dense-data" ||
    name === "numbers-core" ||
    name === "tensor-data"
  ) {
    return "data";
  }
  if (path?.includes("/bindings/")) {
    return "bindings";
  }
  if (name.includes("test-support") || path?.includes("/test-support/")) {
    return "support";
  }
  return "data";
}

export function packageShortName(name: string): string {
  return name
    .replace(/^@video-analysis\//, "")
    .replace(/^video-analysis-/, "")
    .replace(/^audio-analysis-/, "audio:")
    .replace(/^image-analysis-/, "image:")
    .replace(/^text-analysis-/, "text:")
    .replace(/^vector-analysis-/, "vector:")
    .replace(/^three-d-processing-/, "3d:")
    .replace(/^comfyui-/, "comfyui:");
}

export function slugifyPackageName(name: string): string {
  return name
    .replace(/^@/, "")
    .replace(/\//g, "-")
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

export function contractTagLabel(tagId: string): string {
  return contractTagDefinitions.find((definition) => definition.id === tagId)?.label ?? tagId;
}
