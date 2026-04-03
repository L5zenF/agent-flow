import { normalizeConfig, type GatewayConfig, type GatewayConfigWire } from "./types";

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
  validateConfig: (config: GatewayConfig) =>
    request<void>("/admin/validate", {
      method: "POST",
      body: JSON.stringify(config),
    }),
  saveConfig: (config: GatewayConfig) =>
    request<void>("/admin/config", {
      method: "PUT",
      body: JSON.stringify(config),
    }),
  reloadConfig: () =>
    request<void>("/admin/reload", {
      method: "POST",
    }),
};
