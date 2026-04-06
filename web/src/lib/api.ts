import {
  normalizeConfig,
  toWireConfig,
  type WorkflowDocument,
  type WorkflowSummary,
  type GatewayConfig,
  type GatewayConfigWire,
  type WasmPluginManifestSummary,
} from "./types";

async function request<T>(input: string, init?: RequestInit): Promise<T> {
  const response = await fetch(input, {
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
    ...init,
  });

  if (!response.ok) {
    throw new Error(await response.text());
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

export const api = {
  getConfig: async () => normalizeConfig(await request<GatewayConfigWire>("/admin/config")),
  getWorkflows: () => request<WorkflowSummary[]>("/admin/workflows"),
  getWorkflow: (id: string) => request<WorkflowDocument>(`/admin/workflows/${encodeURIComponent(id)}`),
  createWorkflow: (input: { id: string; name: string; description?: string | null }) =>
    request<WorkflowSummary>("/admin/workflows", {
      method: "POST",
      body: JSON.stringify(input),
    }),
  saveWorkflow: (id: string, workflow: WorkflowDocument) =>
    request<WorkflowDocument>(`/admin/workflows/${encodeURIComponent(id)}`, {
      method: "PUT",
      body: JSON.stringify(workflow),
    }),
  activateWorkflow: (id: string) =>
    request<WorkflowSummary>(`/admin/workflows/${encodeURIComponent(id)}/activate`, {
      method: "POST",
    }),
  getPlugins: () => request<WasmPluginManifestSummary[]>("/admin/plugins"),
  validateConfig: (config: GatewayConfig) =>
    request<void>("/admin/validate", {
      method: "POST",
      body: JSON.stringify(toWireConfig(config)),
    }),
  saveConfig: (config: GatewayConfig) =>
    request<void>("/admin/config", {
      method: "PUT",
      body: JSON.stringify(toWireConfig(config)),
    }),
  reloadConfig: () =>
    request<void>("/admin/reload", {
      method: "POST",
    }),
};
