import { useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import { CircleOff, Plus, RefreshCw, Save, Settings2, X } from "lucide-react";
import { api } from "@/lib/api";
import { emptyConfig, type GatewayConfig } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { RuleGraphEditor } from "@/components/rule-graph-editor";
import { Input } from "@/components/ui/input";

export default function App() {
  const [config, setConfig] = useState<GatewayConfig>(emptyConfig);
  const [status, setStatus] = useState("Loading...");
  const [busy, setBusy] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const latestConfigRef = useRef(config);

  latestConfigRef.current = config;

  useEffect(() => {
    void load();
  }, []);

  async function flushPendingEditorState() {
    const active = document.activeElement;
    if (active instanceof HTMLElement && typeof active.blur === "function") {
      flushSync(() => {
        active.blur();
      });
    }
    window.dispatchEvent(new Event("rule-graph:flush"));
    await new Promise<void>((resolve) => {
      window.setTimeout(() => resolve(), 0);
    });
  }

  async function load() {
    setBusy(true);
    try {
      const next = await api.getConfig();
      setConfig(next);
      setStatus("Config loaded from admin API.");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Failed to load config.");
    } finally {
      setBusy(false);
    }
  }

  async function save() {
    setBusy(true);
    try {
      await flushPendingEditorState();
      await api.saveConfig(latestConfigRef.current);
      setStatus("Config saved.");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Save failed.");
    } finally {
      setBusy(false);
    }
  }

  function openSettings() {
    setSettingsOpen(true);
  }

  function closeSettings() {
    setSettingsOpen(false);
  }

  return (
    <div className="min-h-screen bg-zinc-50 text-zinc-900">
      <div className="flex min-h-screen flex-col">
        <header className="border-b border-zinc-200/80 bg-white/75 px-4 py-3 backdrop-blur-md">
          <div className="mx-auto flex max-w-7xl items-center justify-between gap-4">
            <div className="min-w-0">
              <Badge>gateway switch</Badge>
              <div className="mt-2 min-w-0">
                <h1 className="truncate font-mono text-xl font-semibold tracking-tight">
                  LLM Gateway
                </h1>
                <p className="mt-0.5 truncate text-sm text-zinc-500">
                  Canvas-first admin shell for the rule graph workspace.
                </p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <TopBarButton label="Settings" onClick={openSettings}>
                <Settings2 className="h-4 w-4" />
              </TopBarButton>
              <TopBarButton label="Load config" onClick={load} disabled={busy}>
                <RefreshCw className="h-4 w-4" />
              </TopBarButton>
              <TopBarButton label="Save config" onClick={save} disabled={busy}>
                <Save className="h-4 w-4" />
              </TopBarButton>
            </div>
          </div>
          <div className="mx-auto mt-2 flex max-w-7xl items-center justify-start">
            <div className={`status-chip ${busy ? "status-chip-busy" : ""}`}>
              <span className={`status-dot ${busy ? "status-dot-busy" : ""}`} />
              <span className="min-w-0 truncate">{status}</span>
            </div>
          </div>
        </header>

        <main className="flex-1 px-3 pb-3 pt-2 lg:px-4">
          <RuleGraphEditor config={config} setConfig={setConfig} />
        </main>

        <SettingsModal
          config={config}
          open={settingsOpen}
          setConfig={setConfig}
          onClose={closeSettings}
        />
      </div>
    </div>
  );
}

function SettingsModal({
  config,
  open,
  setConfig,
  onClose,
}: {
  config: GatewayConfig;
  open: boolean;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
  onClose: () => void;
}) {
  if (!open) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-zinc-950/35 p-4">
      <div className="max-h-[85vh] w-full max-w-4xl overflow-hidden rounded-3xl border border-zinc-200 bg-white shadow-[0_30px_120px_rgba(15,23,42,0.18)]">
        <div className="flex items-start justify-between gap-4 border-b border-zinc-200 px-5 py-4 sm:px-6">
          <div>
            <div className="font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
              Settings
            </div>
            <div className="mt-1 text-lg font-semibold text-zinc-900">
              Gateway configuration
            </div>
            <p className="mt-1 text-sm text-zinc-500">
              Global config, providers, and models share the same live config state.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="inline-flex h-10 w-10 items-center justify-center rounded-full border border-zinc-200 bg-white text-zinc-500 transition hover:border-zinc-300 hover:text-zinc-900"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="max-h-[calc(85vh-88px)] space-y-5 overflow-y-auto px-5 py-5 sm:px-6">
          <SettingsSection
            title="Global config"
            description="These values feed the same config object used by the canvas editor and save flow."
          >
            <div className="grid gap-4 md:grid-cols-2">
              <Field
                label="Listen"
                value={config.listen}
                onChange={(value) => setConfig((current) => ({ ...current, listen: value }))}
              />
              <Field
                label="Admin Listen"
                value={config.admin_listen}
                onChange={(value) => setConfig((current) => ({ ...current, admin_listen: value }))}
              />
              <div className="md:col-span-2">
                <Field
                  label="Default Secret Env"
                  value={config.default_secret_env ?? ""}
                  onChange={(value) =>
                    setConfig((current) => ({
                      ...current,
                      default_secret_env: value || null,
                    }))
                  }
                />
              </div>
            </div>
          </SettingsSection>

          <SettingsSection
            title="Providers"
            description="Manage upstream providers and their default headers without leaving the canvas shell."
          >
            <ProvidersSection config={config} setConfig={setConfig} />
          </SettingsSection>

          <SettingsSection
            title="Models"
            description="Attach models to providers using the shared config state consumed by the graph inspector."
          >
            <ModelsSection config={config} setConfig={setConfig} />
          </SettingsSection>
        </div>
      </div>
    </div>
  );
}

function SettingsSection({
  title,
  description,
  children,
}: React.PropsWithChildren<{ title: string; description: string }>) {
  return (
    <section className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4 sm:p-5">
      <div className="mb-4">
        <div className="font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
          {title}
        </div>
        <p className="mt-1 text-sm text-zinc-600">{description}</p>
      </div>
      <div className="space-y-4">{children}</div>
    </section>
  );
}

function ProvidersSection({
  config,
  setConfig,
}: {
  config: GatewayConfig;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
}) {
  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <Button
          className="gap-2"
          onClick={() =>
            setConfig((current) => ({
              ...current,
              providers: [
                ...current.providers,
                {
                  id: `provider-${current.providers.length + 1}`,
                  name: "New Provider",
                  base_url: "https://example.com",
                  default_headers: [],
                },
              ],
            }))
          }
        >
          <Plus className="h-4 w-4" />
          Add Provider
        </Button>
      </div>

      {config.providers.length === 0 ? (
        <EmptyMiniState text="No providers configured." />
      ) : (
        config.providers.map((provider, providerIndex) => (
          <Card key={provider.id} className="rounded-2xl border border-zinc-200">
            <SectionActions
              title={provider.name || provider.id || `Provider ${providerIndex + 1}`}
              onRemove={() =>
                setConfig((current) => removeProviderFromConfig(current, providerIndex, provider.id))
              }
            />

            <div className="grid gap-3 md:grid-cols-2">
              <Field
                label="ID"
                value={provider.id}
                onChange={(value) =>
                  setConfig((current) => renameProviderInConfig(current, providerIndex, provider.id, value))
                }
              />
              <Field
                label="Name"
                value={provider.name}
                onChange={(value) =>
                  updateItem(config.providers, providerIndex, setConfig, "providers", {
                    ...provider,
                    name: value,
                  })
                }
              />
              <div className="md:col-span-2">
                <Field
                  label="Base URL"
                  value={provider.base_url}
                  onChange={(value) =>
                    updateItem(config.providers, providerIndex, setConfig, "providers", {
                      ...provider,
                      base_url: value,
                    })
                  }
                />
              </div>
            </div>

            <div className="mt-4 space-y-3">
              <div className="flex items-center justify-between gap-3">
                <div className="font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
                  Default headers
                </div>
                <Button
                  onClick={() =>
                    updateItem(config.providers, providerIndex, setConfig, "providers", {
                      ...provider,
                      default_headers: [
                        ...provider.default_headers,
                        { name: "X-New-Header", value: { value: "" } },
                      ],
                    })
                  }
                >
                  Add Header
                </Button>
              </div>

              {provider.default_headers.length === 0 ? (
                <EmptyMiniState text="No default headers for this provider." />
              ) : (
                provider.default_headers.map((header, headerIndex) => (
                  <Card key={`${provider.id}-${headerIndex}`} className="rounded-2xl border border-zinc-200">
                    <SectionActions
                      title={header.name || `Header ${headerIndex + 1}`}
                      onRemove={() =>
                        updateItem(config.providers, providerIndex, setConfig, "providers", {
                          ...provider,
                          default_headers: provider.default_headers.filter(
                            (_, item) => item !== headerIndex,
                          ),
                        })
                      }
                    />

                    <div className="grid gap-3 md:grid-cols-2">
                      <Field
                        label="Header"
                        value={header.name}
                        onChange={(value) => {
                          const nextHeaders = [...provider.default_headers];
                          nextHeaders[headerIndex] = { ...header, name: value };
                          updateItem(config.providers, providerIndex, setConfig, "providers", {
                            ...provider,
                            default_headers: nextHeaders,
                          });
                        }}
                      />
                      <Field
                        label="Value"
                        value={header.value.value}
                        onChange={(value) => {
                          const nextHeaders = [...provider.default_headers];
                          nextHeaders[headerIndex] = {
                            ...header,
                            value: { ...header.value, value },
                          };
                          updateItem(config.providers, providerIndex, setConfig, "providers", {
                            ...provider,
                            default_headers: nextHeaders,
                          });
                        }}
                      />
                      <Field
                        label="Secret Env"
                        value={"secret_env" in header.value ? header.value.secret_env ?? "" : ""}
                        onChange={(value) => {
                          const nextHeaders = [...provider.default_headers];
                          nextHeaders[headerIndex] = {
                            ...header,
                            value: {
                              value: header.value.value,
                              encrypted: "encrypted" in header.value ? header.value.encrypted : false,
                              secret_env: value || null,
                            },
                          };
                          updateItem(config.providers, providerIndex, setConfig, "providers", {
                            ...provider,
                            default_headers: nextHeaders,
                          });
                        }}
                      />
                      <label>
                        <Label>Encrypted</Label>
                        <button
                          type="button"
                          onClick={() => {
                            const nextHeaders = [...provider.default_headers];
                            nextHeaders[headerIndex] = {
                              ...header,
                              value: {
                                value: header.value.value,
                                encrypted:
                                  !("encrypted" in header.value && header.value.encrypted),
                                secret_env:
                                  "secret_env" in header.value ? header.value.secret_env ?? null : null,
                              },
                            };
                            updateItem(config.providers, providerIndex, setConfig, "providers", {
                              ...provider,
                              default_headers: nextHeaders,
                            });
                          }}
                          className={`inline-flex h-10 items-center rounded-md border px-3 text-sm transition ${
                            "encrypted" in header.value && header.value.encrypted
                              ? "border-zinc-900 bg-zinc-900 text-white"
                              : "border-zinc-200 bg-white text-zinc-700 hover:border-zinc-300"
                          }`}
                        >
                          {"encrypted" in header.value && header.value.encrypted ? "Yes" : "No"}
                        </button>
                      </label>
                    </div>
                  </Card>
                ))
              )}
            </div>
          </Card>
        ))
      )}
    </div>
  );
}

function ModelsSection({
  config,
  setConfig,
}: {
  config: GatewayConfig;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
}) {
  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <Button
          className="gap-2"
          onClick={() =>
            setConfig((current) => ({
              ...current,
              models: [
                ...current.models,
                {
                  id: `model-${current.models.length + 1}`,
                  name: "New Model",
                  provider_id: current.providers[0]?.id ?? "",
                  description: "",
                },
              ],
            }))
          }
        >
          <Plus className="h-4 w-4" />
          Add Model
        </Button>
      </div>

      {config.models.length === 0 ? (
        <EmptyMiniState text="No models configured." />
      ) : (
        config.models.map((model, modelIndex) => (
          <Card key={model.id} className="rounded-2xl border border-zinc-200">
            <SectionActions
              title={model.name || model.id || `Model ${modelIndex + 1}`}
              onRemove={() =>
                setConfig((current) => removeModelFromConfig(current, modelIndex, model.id))
              }
            />

            <div className="grid gap-3 md:grid-cols-2">
              <Field
                label="ID"
                value={model.id}
                onChange={(value) =>
                  setConfig((current) => renameModelInConfig(current, modelIndex, model.id, value))
                }
              />
              <Field
                label="Name"
                value={model.name}
                onChange={(value) =>
                  updateItem(config.models, modelIndex, setConfig, "models", {
                    ...model,
                    name: value,
                  })
                }
              />
              <Field
                label="Provider ID"
                value={model.provider_id}
                onChange={(value) =>
                  updateItem(config.models, modelIndex, setConfig, "models", {
                    ...model,
                    provider_id: value,
                  })
                }
              />
              <Field
                label="Description"
                value={model.description ?? ""}
                onChange={(value) =>
                  updateItem(config.models, modelIndex, setConfig, "models", {
                    ...model,
                    description: value,
                  })
                }
              />
            </div>
          </Card>
        ))
      )}
    </div>
  );
}

function TopBarButton({
  children,
  disabled,
  label,
  onClick,
}: React.PropsWithChildren<{
  disabled?: boolean;
  label: string;
  onClick: () => void;
}>) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      disabled={disabled}
      className="inline-flex h-9 w-9 items-center justify-center rounded-full border border-zinc-200 bg-white text-zinc-600 shadow-sm transition hover:border-zinc-300 hover:text-zinc-900 hover:shadow disabled:cursor-not-allowed disabled:opacity-50"
    >
      {children}
    </button>
  );
}

function EmptyMiniState({ text }: { text: string }) {
  return (
    <div className="rounded-lg border border-dashed border-zinc-200 px-4 py-6 text-sm text-zinc-500">
      {text}
    </div>
  );
}

function updateItem<T, K extends keyof GatewayConfig>(
  items: T[],
  index: number,
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>,
  key: K,
  nextItem: GatewayConfig[K] extends T[] ? T : never,
) {
  const next = [...items];
  next[index] = nextItem as T;
  setConfig((current) => ({ ...current, [key]: next }));
}

function renameProviderInConfig(
  current: GatewayConfig,
  providerIndex: number,
  previousId: string,
  nextId: string,
) {
  const trimmed = nextId.trim();
  const effectiveId = trimmed || previousId;
  const nextProviders = [...current.providers];
  nextProviders[providerIndex] = { ...nextProviders[providerIndex], id: effectiveId };

  return {
    ...current,
    providers: nextProviders,
    models: current.models.map((model) =>
      model.provider_id === previousId ? { ...model, provider_id: effectiveId } : model,
    ),
    rule_graph: current.rule_graph
      ? {
          ...current.rule_graph,
          nodes: current.rule_graph.nodes.map((node) =>
            node.route_provider?.provider_id === previousId
              ? {
                  ...node,
                  route_provider: {
                    provider_id: effectiveId,
                  },
                }
              : node,
          ),
        }
      : current.rule_graph,
  };
}

function removeProviderFromConfig(
  current: GatewayConfig,
  providerIndex: number,
  providerId: string,
) {
  const removedModelIds = new Set(
    current.models.filter((model) => model.provider_id === providerId).map((model) => model.id),
  );

  return {
    ...current,
    providers: current.providers.filter((_, item) => item !== providerIndex),
    models: current.models.filter((model) => model.provider_id !== providerId),
    rule_graph: current.rule_graph
      ? {
          ...current.rule_graph,
          nodes: current.rule_graph.nodes.map((node) => {
            if (node.route_provider?.provider_id === providerId) {
              return {
                ...node,
                route_provider: {
                  provider_id: "",
                },
              };
            }

            if (node.select_model?.model_id && removedModelIds.has(node.select_model.model_id)) {
              return {
                ...node,
                select_model: {
                  model_id: "",
                },
              };
            }

            return node;
          }),
        }
      : current.rule_graph,
  };
}

function renameModelInConfig(
  current: GatewayConfig,
  modelIndex: number,
  previousId: string,
  nextId: string,
) {
  const trimmed = nextId.trim();
  const effectiveId = trimmed || previousId;
  const nextModels = [...current.models];
  nextModels[modelIndex] = { ...nextModels[modelIndex], id: effectiveId };

  return {
    ...current,
    models: nextModels,
    rule_graph: current.rule_graph
      ? {
          ...current.rule_graph,
          nodes: current.rule_graph.nodes.map((node) =>
            node.select_model?.model_id === previousId
              ? {
                  ...node,
                  select_model: {
                    model_id: effectiveId,
                  },
                }
              : node,
          ),
        }
      : current.rule_graph,
  };
}

function removeModelFromConfig(
  current: GatewayConfig,
  modelIndex: number,
  modelId: string,
) {
  return {
    ...current,
    models: current.models.filter((_, item) => item !== modelIndex),
    rule_graph: current.rule_graph
      ? {
          ...current.rule_graph,
          nodes: current.rule_graph.nodes.map((node) =>
            node.select_model?.model_id === modelId
              ? {
                  ...node,
                  select_model: {
                    model_id: "",
                  },
                }
              : node,
          ),
        }
      : current.rule_graph,
  };
}

function SectionActions({
  onRemove,
  title,
}: {
  onRemove: () => void;
  title?: string;
}) {
  return (
    <div className="mb-4 flex items-center justify-between gap-3">
      {title ? (
        <div className="font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
          {title}
        </div>
      ) : (
        <div />
      )}
      <Button onClick={onRemove} className="bg-white text-zinc-900">
        <CircleOff className="mr-2 h-4 w-4" />
        Remove
      </Button>
    </div>
  );
}

function Label({ children }: React.PropsWithChildren) {
  return (
    <div className="mb-1 font-mono text-[11px] uppercase tracking-[0.16em] text-zinc-500">
      {children}
    </div>
  );
}

function Field({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label>
      <Label>{label}</Label>
      <Input value={value} onChange={(event) => onChange(event.target.value)} />
    </label>
  );
}
