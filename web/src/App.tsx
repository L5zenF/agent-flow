import { useEffect, useMemo, useState } from "react";
import {
  CircleOff,
  Copy,
  Pencil,
  Plus,
  RefreshCw,
  Save,
  TestTubeDiagonal,
  X,
} from "lucide-react";
import { api } from "@/lib/api";
import { emptyConfig, type GatewayConfig, type HeaderAction } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { RuleGraphEditor } from "@/components/rule-graph-editor";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";

type TabKey = "providers" | "rule_graph" | "models" | "routes" | "rules" | "raw";
type DialogTab = "basic" | "models" | "headers" | "routes" | "rules";

const tabs: Array<{ key: TabKey; label: string }> = [
  { key: "providers", label: "Providers" },
  { key: "rule_graph", label: "Rule Graph" },
  { key: "models", label: "Models" },
  { key: "routes", label: "Routes" },
  { key: "rules", label: "Rules" },
  { key: "raw", label: "Raw" },
];

export default function App() {
  const [tab, setTab] = useState<TabKey>("providers");
  const [config, setConfig] = useState<GatewayConfig>(emptyConfig);
  const [status, setStatus] = useState("Loading...");
  const [busy, setBusy] = useState(false);
  const [editingProviderId, setEditingProviderId] = useState<string | null>(null);
  const [dialogTab, setDialogTab] = useState<DialogTab>("basic");

  useEffect(() => {
    void load();
  }, []);

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

  async function validate() {
    setBusy(true);
    try {
      await api.validateConfig(config);
      setStatus("Validation passed.");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Validation failed.");
    } finally {
      setBusy(false);
    }
  }

  async function save() {
    setBusy(true);
    try {
      await api.saveConfig(config);
      setStatus("Config saved.");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Save failed.");
    } finally {
      setBusy(false);
    }
  }

  async function reload() {
    setBusy(true);
    try {
      await api.reloadConfig();
      await load();
      setStatus("Config reloaded from disk.");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Reload failed.");
      setBusy(false);
    }
  }

  const editingProvider = useMemo(
    () => config.providers.find((provider) => provider.id === editingProviderId) ?? null,
    [config.providers, editingProviderId],
  );

  return (
    <div className="min-h-screen bg-zinc-50 text-zinc-900">
      <div className="mx-auto flex max-w-7xl flex-col gap-4 px-4 py-6 lg:px-8">
        <header className="space-y-4">
          <div className="flex flex-col gap-4 border-b border-zinc-200 pb-4 lg:flex-row lg:items-center lg:justify-between">
            <div className="space-y-2">
              <Badge>gateway switch</Badge>
              <div>
                <h1 className="font-mono text-2xl font-semibold tracking-tight">
                  LLM Gateway
                </h1>
                <p className="mt-1 max-w-2xl text-sm text-zinc-600">
                  Provider-first switcher. Main screen stays clean. Editing happens in a focused dialog.
                </p>
              </div>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button onClick={load} disabled={busy} className="gap-2">
                <RefreshCw className="h-4 w-4" />
                Load
              </Button>
              <Button onClick={validate} disabled={busy} className="gap-2">
                <TestTubeDiagonal className="h-4 w-4" />
                Validate
              </Button>
              <Button onClick={save} disabled={busy} className="gap-2">
                <Save className="h-4 w-4" />
                Save
              </Button>
              <Button onClick={reload} disabled={busy}>
                Reload
              </Button>
            </div>
          </div>

          <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <div className="flex flex-wrap gap-2">
              {tabs.map((item) => (
                <button
                  key={item.key}
                  onClick={() => setTab(item.key)}
                  className={`rounded-full border px-4 py-2 text-sm transition ${
                    tab === item.key
                      ? "border-zinc-900 bg-zinc-900 text-white"
                      : "border-zinc-200 bg-white text-zinc-600 hover:border-zinc-300 hover:text-zinc-900"
                  }`}
                >
                  {item.label}
                </button>
              ))}
            </div>

            <div className="grid gap-1 text-sm text-zinc-500 lg:text-right">
              <div>Gateway: {config.listen}</div>
              <div>Admin: {config.admin_listen}</div>
              <div>Secret Env: {config.default_secret_env || "<unset>"}</div>
            </div>
          </div>

          <div className="rounded-md border border-zinc-200 bg-white px-3 py-2 text-sm text-zinc-600">
            {status}
          </div>
        </header>

        {tab === "providers" ? (
          <ProvidersView
            config={config}
            setConfig={setConfig}
            onEditProvider={(providerId, nextTab = "basic") => {
              setEditingProviderId(providerId);
              setDialogTab(nextTab);
            }}
          />
        ) : tab === "rule_graph" ? (
          <SimpleResourceView
            title="Rule Graph"
            description="Global visual rule graph. This becomes the primary route and header rule entrypoint."
          >
            <RuleGraphEditor config={config} setConfig={setConfig} />
          </SimpleResourceView>
        ) : tab === "models" ? (
          <SimpleResourceView title="Models" description="Direct list view for models.">
            {config.models.map((model, index) => (
              <EditorRow
                key={model.id}
                title={model.id}
                onRemove={() =>
                  setConfig((current) => ({
                    ...current,
                    models: current.models.filter((_, item) => item !== index),
                  }))
                }
              >
                <Field
                  label="ID"
                  value={model.id}
                  onChange={(value) =>
                    updateItem(config.models, index, setConfig, "models", { ...model, id: value })
                  }
                />
                <Field
                  label="Name"
                  value={model.name}
                  onChange={(value) =>
                    updateItem(config.models, index, setConfig, "models", { ...model, name: value })
                  }
                />
                <Field
                  label="Provider ID"
                  value={model.provider_id}
                  onChange={(value) =>
                    updateItem(config.models, index, setConfig, "models", {
                      ...model,
                      provider_id: value,
                    })
                  }
                />
              </EditorRow>
            ))}
          </SimpleResourceView>
        ) : tab === "routes" ? (
          <SimpleResourceView title="Routes" description="Direct list view for routes.">
            {config.routes.map((route, index) => (
              <EditorRow
                key={route.id}
                title={route.id}
                onRemove={() =>
                  setConfig((current) => ({
                    ...current,
                    routes: current.routes.filter((_, item) => item !== index),
                  }))
                }
              >
                <Field
                  label="ID"
                  value={route.id}
                  onChange={(value) =>
                    updateItem(config.routes, index, setConfig, "routes", { ...route, id: value })
                  }
                />
                <Field
                  label="Provider ID"
                  value={route.provider_id}
                  onChange={(value) =>
                    updateItem(config.routes, index, setConfig, "routes", {
                      ...route,
                      provider_id: value,
                    })
                  }
                />
                <div className="md:col-span-2">
                  <Label>Match Expression</Label>
                  <Textarea
                    value={route.matcher}
                    onChange={(event) =>
                      updateItem(config.routes, index, setConfig, "routes", {
                        ...route,
                        matcher: event.target.value,
                      })
                    }
                  />
                </div>
              </EditorRow>
            ))}
          </SimpleResourceView>
        ) : tab === "rules" ? (
          <SimpleResourceView title="Rules" description="Direct list view for rules.">
            {config.header_rules.map((rule, index) => (
              <EditorRow
                key={rule.id}
                title={rule.id}
                onRemove={() =>
                  setConfig((current) => ({
                    ...current,
                    header_rules: current.header_rules.filter((_, item) => item !== index),
                  }))
                }
              >
                <Field
                  label="Scope"
                  value={rule.scope}
                  onChange={(value) =>
                    updateItem(config.header_rules, index, setConfig, "header_rules", {
                      ...rule,
                      scope: value as typeof rule.scope,
                    })
                  }
                />
                <Field
                  label="Target ID"
                  value={rule.target_id ?? ""}
                  onChange={(value) =>
                    updateItem(config.header_rules, index, setConfig, "header_rules", {
                      ...rule,
                      target_id: value,
                    })
                  }
                />
              </EditorRow>
            ))}
          </SimpleResourceView>
        ) : (
          <Card>
            <div className="mb-3 font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
              Raw Config
            </div>
            <pre className="overflow-x-auto rounded-md border border-zinc-200 bg-zinc-950 p-4 font-mono text-xs text-zinc-100">
              {JSON.stringify(config, null, 2)}
            </pre>
          </Card>
        )}
      </div>

      {editingProvider && (
        <ProviderDialog
          provider={editingProvider}
          dialogTab={dialogTab}
          setDialogTab={setDialogTab}
          config={config}
          setConfig={setConfig}
          onClose={() => setEditingProviderId(null)}
        />
      )}
    </div>
  );
}

function ProvidersView({
  config,
  setConfig,
  onEditProvider,
}: {
  config: GatewayConfig;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
  onEditProvider: (providerId: string, nextTab?: DialogTab) => void;
}) {
  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <div>
          <div className="font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
            Providers
          </div>
          <p className="mt-1 text-sm text-zinc-600">
            Main view stays card-only. Click settings to open a focused provider dialog.
          </p>
        </div>
        <Button
          className="gap-2"
          onClick={() =>
            setConfig((current) => {
              const nextProvider = {
                id: `provider-${current.providers.length + 1}`,
                name: "New Provider",
                base_url: "https://example.com",
                default_headers: [],
              };
              onEditProvider(nextProvider.id, "basic");
              return {
                ...current,
                providers: [...current.providers, nextProvider],
              };
            })
          }
        >
          <Plus className="h-4 w-4" />
          Add Provider
        </Button>
      </div>

      <div className="space-y-3">
        {config.providers.map((provider, index) => {
          const modelCount = config.models.filter((model) => model.provider_id === provider.id).length;
          const routeCount = config.routes.filter((route) => route.provider_id === provider.id).length;
          const ruleCount = config.header_rules.filter(
            (rule) =>
              (rule.scope === "provider" && rule.target_id === provider.id) ||
              (rule.scope === "model" &&
                config.models.some(
                  (model) => model.provider_id === provider.id && model.id === rule.target_id,
                )),
          ).length;

          return (
            <Card key={provider.id} className="rounded-2xl border border-zinc-200 transition hover:border-zinc-300">
              <div className="flex items-start justify-between gap-4">
                <div className="flex min-w-0 items-start gap-3">
                  <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-zinc-200 bg-zinc-100 font-mono text-sm font-semibold">
                    {provider.name.slice(0, 1).toUpperCase()}
                  </div>
                  <div className="min-w-0">
                    <div className="text-base font-semibold">{provider.name}</div>
                    <div className="truncate text-sm text-blue-600">{provider.base_url}</div>
                  </div>
                </div>

                <div className="flex shrink-0 items-center gap-2">
                  <IconButton label="Edit" onClick={() => onEditProvider(provider.id, "basic")}>
                    <Pencil className="h-4 w-4" />
                  </IconButton>
                  <IconButton
                    label="Clone"
                    onClick={() =>
                      setConfig((current) => ({
                        ...current,
                        providers: [
                          ...current.providers,
                          {
                            ...provider,
                            id: `${provider.id}-copy`,
                            name: `${provider.name} Copy`,
                          },
                        ],
                      }))
                    }
                  >
                    <Copy className="h-4 w-4" />
                  </IconButton>
                  <IconButton
                    label="Delete"
                    onClick={() =>
                      setConfig((current) => ({
                        ...current,
                        providers: current.providers.filter((_, item) => item !== index),
                      }))
                    }
                  >
                    <CircleOff className="h-4 w-4" />
                  </IconButton>
                </div>
              </div>

              <div className="mt-4 flex flex-wrap gap-2">
                <MetricPill>{modelCount} models</MetricPill>
                <MetricPill>{routeCount} routes</MetricPill>
                <MetricPill>{ruleCount} rules</MetricPill>
                <MetricPill>{provider.default_headers.length} headers</MetricPill>
              </div>

              <div className="mt-4 flex flex-wrap gap-2">
                <GhostAction onClick={() => onEditProvider(provider.id, "models")}>
                  Models
                </GhostAction>
                <GhostAction onClick={() => onEditProvider(provider.id, "headers")}>
                  Headers
                </GhostAction>
                <GhostAction onClick={() => onEditProvider(provider.id, "routes")}>
                  Routes
                </GhostAction>
                <GhostAction onClick={() => onEditProvider(provider.id, "rules")}>
                  Rules
                </GhostAction>
              </div>
            </Card>
          );
        })}
      </div>
    </div>
  );
}

function ProviderDialog({
  provider,
  dialogTab,
  setDialogTab,
  config,
  setConfig,
  onClose,
}: {
  provider: GatewayConfig["providers"][number];
  dialogTab: DialogTab;
  setDialogTab: (value: DialogTab) => void;
  config: GatewayConfig;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
  onClose: () => void;
}) {
  const providerIndex = config.providers.findIndex((item) => item.id === provider.id);
  const models = config.models.filter((model) => model.provider_id === provider.id);
  const routes = config.routes.filter((route) => route.provider_id === provider.id);
  const rules = config.header_rules.filter(
    (rule) =>
      (rule.scope === "provider" && rule.target_id === provider.id) ||
      (rule.scope === "model" &&
        config.models.some(
          (model) => model.provider_id === provider.id && model.id === rule.target_id,
        )),
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-zinc-950/40 p-4">
      <div className="max-h-[90vh] w-full max-w-5xl overflow-hidden rounded-2xl border border-zinc-200 bg-white shadow-2xl">
        <div className="flex items-start justify-between gap-4 border-b border-zinc-200 px-6 py-4">
          <div>
            <div className="font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
              Provider Settings
            </div>
            <div className="mt-1 text-lg font-semibold">{provider.name}</div>
            <div className="text-sm text-zinc-500">{provider.base_url}</div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-md border border-zinc-200 p-2 text-zinc-500 transition hover:bg-zinc-50 hover:text-zinc-900"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="border-b border-zinc-200 px-6 py-3">
          <div className="flex flex-wrap gap-2">
            {([
              ["basic", "Basic"],
              ["models", "Models"],
              ["headers", "Headers"],
              ["routes", "Routes"],
              ["rules", "Rules"],
            ] as Array<[DialogTab, string]>).map(([key, label]) => (
              <button
                key={key}
                onClick={() => setDialogTab(key)}
                className={`rounded-full border px-3 py-1.5 text-sm transition ${
                  dialogTab === key
                    ? "border-zinc-900 bg-zinc-900 text-white"
                    : "border-zinc-200 bg-white text-zinc-600 hover:border-zinc-300 hover:text-zinc-900"
                }`}
              >
                {label}
              </button>
            ))}
          </div>
        </div>

        <div className="max-h-[calc(90vh-140px)] overflow-y-auto px-6 py-5">
          {dialogTab === "basic" && (
            <div className="grid gap-4 md:grid-cols-2">
              <Field
                label="ID"
                value={provider.id}
                onChange={(value) =>
                  updateItem(config.providers, providerIndex, setConfig, "providers", {
                    ...provider,
                    id: value,
                  })
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
          )}

          {dialogTab === "models" && (
            <div className="space-y-3">
              <div className="flex justify-end">
                <Button
                  onClick={() =>
                    setConfig((current) => ({
                      ...current,
                      models: [
                        ...current.models,
                        {
                          id: `${provider.id}-model-${models.length + 1}`,
                          name: "New Model",
                          provider_id: provider.id,
                          description: "",
                        },
                      ],
                    }))
                  }
                >
                  Add Model
                </Button>
              </div>
              {models.map((model) => {
                const modelIndex = config.models.findIndex((item) => item.id === model.id);
                return (
                  <Card key={model.id}>
                    <SectionActions
                      title={model.id}
                      onRemove={() =>
                        setConfig((current) => ({
                          ...current,
                          models: current.models.filter((item) => item.id !== model.id),
                        }))
                      }
                    />
                    <div className="grid gap-3 md:grid-cols-2">
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
                );
              })}
            </div>
          )}

          {dialogTab === "headers" && (
            <div className="space-y-3">
              <div className="flex justify-end">
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
              {provider.default_headers.map((header, headerIndex) => (
                <Card key={`${provider.id}-${headerIndex}`}>
                  <SectionActions
                    title={header.name || `Header ${headerIndex + 1}`}
                    onRemove={() => {
                      const next = provider.default_headers.filter((_, item) => item !== headerIndex);
                      updateItem(config.providers, providerIndex, setConfig, "providers", {
                        ...provider,
                        default_headers: next,
                      });
                    }}
                  />
                  <div className="grid gap-3">
                    <Field
                      label="Header"
                      value={header.name}
                      onChange={(value) => {
                        const next = [...provider.default_headers];
                        next[headerIndex] = { ...header, name: value };
                        updateItem(config.providers, providerIndex, setConfig, "providers", {
                          ...provider,
                          default_headers: next,
                        });
                      }}
                    />
                    <Field
                      label="Value"
                      value={header.value.value}
                      onChange={(value) => {
                        const next = [...provider.default_headers];
                        next[headerIndex] = { ...header, value: { ...header.value, value } };
                        updateItem(config.providers, providerIndex, setConfig, "providers", {
                          ...provider,
                          default_headers: next,
                        });
                      }}
                    />
                    <Field
                      label="Secret Env"
                      value={"secret_env" in header.value ? header.value.secret_env ?? "" : ""}
                      onChange={(value) => {
                        const next = [...provider.default_headers];
                        next[headerIndex] = {
                          ...header,
                          value: {
                            value: header.value.value,
                            encrypted: "encrypted" in header.value ? header.value.encrypted : false,
                            secret_env: value || null,
                          },
                        };
                        updateItem(config.providers, providerIndex, setConfig, "providers", {
                          ...provider,
                          default_headers: next,
                        });
                      }}
                    />
                    <div>
                      <Label>Mode</Label>
                      <div className="flex gap-2">
                        <Button
                          className={
                            "encrypted" in header.value && header.value.encrypted
                              ? "border-zinc-900 bg-zinc-900 text-white hover:bg-zinc-800"
                              : ""
                          }
                          onClick={() => {
                            const next = [...provider.default_headers];
                            next[headerIndex] = {
                              ...header,
                              value: {
                                value: header.value.value,
                                encrypted: !("encrypted" in header.value && header.value.encrypted),
                                secret_env:
                                  "secret_env" in header.value ? header.value.secret_env ?? null : null,
                              },
                            };
                            updateItem(config.providers, providerIndex, setConfig, "providers", {
                              ...provider,
                              default_headers: next,
                            });
                          }}
                        >
                          {("encrypted" in header.value && header.value.encrypted)
                            ? "Encrypted"
                            : "Plain"}
                        </Button>
                      </div>
                    </div>
                  </div>
                </Card>
              ))}
            </div>
          )}

          {dialogTab === "routes" && (
            <div className="space-y-3">
              <div className="flex justify-end">
                <Button
                  onClick={() =>
                    setConfig((current) => ({
                      ...current,
                      routes: [
                        ...current.routes,
                        {
                          id: `${provider.id}-route-${routes.length + 1}`,
                          priority: 100,
                          enabled: true,
                          matcher: 'method == "POST"',
                          provider_id: provider.id,
                          model_id: models[0]?.id ?? "",
                          path_rewrite: "",
                        },
                      ],
                    }))
                  }
                >
                  Add Route
                </Button>
              </div>
              {routes.length === 0 ? (
                <EmptyMiniState text="No routes bound to this provider." />
              ) : (
                routes.map((route) => {
                  const routeIndex = config.routes.findIndex((item) => item.id === route.id);
                  return (
                    <Card key={route.id}>
                      <SectionActions
                        title={route.id}
                        onRemove={() =>
                          setConfig((current) => ({
                            ...current,
                            routes: current.routes.filter((item) => item.id !== route.id),
                          }))
                        }
                      />
                      <div className="grid gap-3 md:grid-cols-2">
                        <Field
                          label="ID"
                          value={route.id}
                          onChange={(value) =>
                            updateItem(config.routes, routeIndex, setConfig, "routes", {
                              ...route,
                              id: value,
                            })
                          }
                        />
                        <Field
                          label="Priority"
                          value={String(route.priority)}
                          onChange={(value) =>
                            updateItem(config.routes, routeIndex, setConfig, "routes", {
                              ...route,
                              priority: Number(value) || 0,
                            })
                          }
                        />
                        <Field
                          label="Model ID"
                          value={route.model_id ?? ""}
                          onChange={(value) =>
                            updateItem(config.routes, routeIndex, setConfig, "routes", {
                              ...route,
                              model_id: value,
                            })
                          }
                        />
                        <Field
                          label="Path Rewrite"
                          value={route.path_rewrite ?? ""}
                          onChange={(value) =>
                            updateItem(config.routes, routeIndex, setConfig, "routes", {
                              ...route,
                              path_rewrite: value,
                            })
                          }
                        />
                        <div className="md:col-span-2">
                          <Label>Match Expression</Label>
                          <Textarea
                            value={route.matcher}
                            onChange={(event) =>
                              updateItem(config.routes, routeIndex, setConfig, "routes", {
                                ...route,
                                matcher: event.target.value,
                              })
                            }
                          />
                        </div>
                        <Field
                          label="Enabled"
                          value={String(route.enabled)}
                          onChange={(value) =>
                            updateItem(config.routes, routeIndex, setConfig, "routes", {
                              ...route,
                              enabled: value !== "false",
                            })
                          }
                        />
                      </div>
                    </Card>
                  );
                })
              )}
            </div>
          )}

          {dialogTab === "rules" && (
            <div className="space-y-3">
              <div className="flex justify-end">
                <Button
                  onClick={() =>
                    setConfig((current) => ({
                      ...current,
                      header_rules: [
                        ...current.header_rules,
                        {
                          id: `${provider.id}-rule-${rules.length + 1}`,
                          enabled: true,
                          scope: "provider",
                          target_id: provider.id,
                          when: "",
                          actions: [{ type: "set", name: "X-Debug", value: "on" }],
                        },
                      ],
                    }))
                  }
                >
                  Add Rule
                </Button>
              </div>
              {rules.length === 0 ? (
                <EmptyMiniState text="No provider-related rules." />
              ) : (
                rules.map((rule) => {
                  const ruleIndex = config.header_rules.findIndex((item) => item.id === rule.id);
                  return (
                    <Card key={rule.id}>
                      <SectionActions
                        title={rule.id}
                        onRemove={() =>
                          setConfig((current) => ({
                            ...current,
                            header_rules: current.header_rules.filter((item) => item.id !== rule.id),
                          }))
                        }
                      />
                      <div className="grid gap-3 md:grid-cols-2">
                        <Field
                          label="ID"
                          value={rule.id}
                          onChange={(value) =>
                            updateItem(config.header_rules, ruleIndex, setConfig, "header_rules", {
                              ...rule,
                              id: value,
                            })
                          }
                        />
                        <Field
                          label="Scope"
                          value={rule.scope}
                          onChange={(value) =>
                            updateItem(config.header_rules, ruleIndex, setConfig, "header_rules", {
                              ...rule,
                              scope: value as typeof rule.scope,
                            })
                          }
                        />
                        <Field
                          label="Target ID"
                          value={rule.target_id ?? ""}
                          onChange={(value) =>
                            updateItem(config.header_rules, ruleIndex, setConfig, "header_rules", {
                              ...rule,
                              target_id: value,
                            })
                          }
                        />
                        <Field
                          label="Enabled"
                          value={String(rule.enabled)}
                          onChange={(value) =>
                            updateItem(config.header_rules, ruleIndex, setConfig, "header_rules", {
                              ...rule,
                              enabled: value !== "false",
                            })
                          }
                        />
                        <div className="md:col-span-2">
                          <Label>When</Label>
                          <Textarea
                            value={rule.when ?? ""}
                            onChange={(event) =>
                              updateItem(config.header_rules, ruleIndex, setConfig, "header_rules", {
                                ...rule,
                                when: event.target.value,
                              })
                            }
                          />
                        </div>
                      </div>

                      <div className="mt-4 space-y-3">
                        <div className="flex items-center justify-between">
                          <div className="font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
                            Actions
                          </div>
                          <Button
                            onClick={() => {
                              updateItem(config.header_rules, ruleIndex, setConfig, "header_rules", {
                                ...rule,
                                actions: [
                                  ...rule.actions,
                                  { type: "remove", name: "X-New-Header" },
                                ],
                              });
                            }}
                          >
                            Add Action
                          </Button>
                        </div>
                        {rule.actions.map((action, actionIndex) => (
                          <RuleActionEditor
                            key={`${rule.id}-${actionIndex}`}
                            action={action}
                            onChange={(nextAction) => {
                              const nextActions = [...rule.actions];
                              nextActions[actionIndex] = nextAction;
                              updateItem(config.header_rules, ruleIndex, setConfig, "header_rules", {
                                ...rule,
                                actions: nextActions,
                              });
                            }}
                            onRemove={() => {
                              const nextActions = rule.actions.filter((_, item) => item !== actionIndex);
                              updateItem(config.header_rules, ruleIndex, setConfig, "header_rules", {
                                ...rule,
                                actions: nextActions,
                              });
                            }}
                          />
                        ))}
                      </div>
                    </Card>
                  );
                })
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function SimpleResourceView({
  title,
  description,
  children,
}: React.PropsWithChildren<{ title: string; description: string }>) {
  return (
    <Card>
      <div className="mb-4">
        <div className="font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
          {title}
        </div>
        <p className="mt-1 text-sm text-zinc-600">{description}</p>
      </div>
      <div className="space-y-4">{children}</div>
    </Card>
  );
}

function IconButton({
  children,
  label,
  onClick,
}: React.PropsWithChildren<{
  label: string;
  onClick: () => void;
}>) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className="rounded-md border border-transparent p-2 text-zinc-500 transition hover:border-zinc-200 hover:bg-zinc-50 hover:text-zinc-900"
    >
      {children}
    </button>
  );
}

function GhostAction({
  children,
  onClick,
}: React.PropsWithChildren<{ onClick: () => void }>) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="rounded-full border border-zinc-200 bg-zinc-50 px-3 py-1.5 text-sm text-zinc-600 transition hover:border-zinc-300 hover:text-zinc-900"
    >
      {children}
    </button>
  );
}

function MetricPill({ children }: React.PropsWithChildren) {
  return (
    <span className="rounded-full border border-zinc-200 bg-zinc-50 px-3 py-1 text-xs text-zinc-600">
      {children}
    </span>
  );
}

function EmptyMiniState({ text }: { text: string }) {
  return (
    <div className="rounded-lg border border-dashed border-zinc-200 px-4 py-6 text-sm text-zinc-500">
      {text}
    </div>
  );
}

function ActionPreview({ action }: { action: HeaderAction }) {
  const text =
    action.type === "set"
      ? `set ${action.name} = ${action.value}`
      : action.type === "remove"
        ? `remove ${action.name}`
        : action.type === "copy"
          ? `copy ${action.from} -> ${action.to}`
          : `set_if_absent ${action.name} = ${action.value}`;

  return (
    <div className="rounded-md border border-zinc-200 bg-zinc-50 px-3 py-2 font-mono text-xs text-zinc-700">
      {text}
    </div>
  );
}

function RuleActionEditor({
  action,
  onChange,
  onRemove,
}: {
  action: HeaderAction;
  onChange: (value: HeaderAction) => void;
  onRemove: () => void;
}) {
  return (
    <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-3">
      <div className="grid gap-3 md:grid-cols-4">
        <Field
          label="Type"
          value={action.type}
          onChange={(value) => {
            if (value === "set") {
              onChange({ type: "set", name: "X-Header", value: "" });
            } else if (value === "copy") {
              onChange({ type: "copy", from: "Authorization", to: "X-Authorization" });
            } else if (value === "set_if_absent") {
              onChange({ type: "set_if_absent", name: "X-Header", value: "" });
            } else {
              onChange({ type: "remove", name: "X-Header" });
            }
          }}
        />
        {"name" in action && (
          <Field
            label="Name"
            value={action.name}
            onChange={(value) => onChange({ ...action, name: value } as HeaderAction)}
          />
        )}
        {"value" in action && (
          <Field
            label="Value"
            value={action.value}
            onChange={(value) => onChange({ ...action, value } as HeaderAction)}
          />
        )}
        {"from" in action && (
          <Field
            label="From"
            value={action.from}
            onChange={(value) => onChange({ ...action, from: value } as HeaderAction)}
          />
        )}
        {"to" in action && (
          <Field
            label="To"
            value={action.to}
            onChange={(value) => onChange({ ...action, to: value } as HeaderAction)}
          />
        )}
      </div>
      <div className="mt-3">
        <Button onClick={onRemove} className="bg-white text-zinc-900">
          Remove Action
        </Button>
      </div>
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

function EditorRow({
  title,
  onRemove,
  children,
}: React.PropsWithChildren<{ title: string; onRemove: () => void }>) {
  return (
    <Card>
      <SectionActions onRemove={onRemove} title={title} />
      <div className="grid gap-3 md:grid-cols-2">{children}</div>
    </Card>
  );
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
