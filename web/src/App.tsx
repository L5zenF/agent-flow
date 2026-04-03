import { useEffect, useState } from "react";
import {
  ChevronRight,
  CircleOff,
  RefreshCw,
  Save,
  TestTubeDiagonal,
} from "lucide-react";
import { api } from "@/lib/api";
import { emptyConfig, type GatewayConfig, type HeaderAction } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";

type TabKey = "providers" | "models" | "routes" | "rules" | "raw";

const tabs: Array<{ key: TabKey; label: string }> = [
  { key: "providers", label: "Providers" },
  { key: "models", label: "Models" },
  { key: "routes", label: "Routes" },
  { key: "rules", label: "Header Rules" },
  { key: "raw", label: "Raw Config" },
];

export default function App() {
  const [tab, setTab] = useState<TabKey>("providers");
  const [config, setConfig] = useState<GatewayConfig>(emptyConfig);
  const [status, setStatus] = useState("Loading...");
  const [busy, setBusy] = useState(false);

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

  return (
    <div className="min-h-screen bg-zinc-50 text-zinc-900">
      <div className="mx-auto flex max-w-7xl flex-col gap-4 px-4 py-6 lg:px-8">
        <header className="space-y-4">
          <div className="flex flex-col gap-3 border-b border-zinc-200 pb-4 lg:flex-row lg:items-start lg:justify-between">
            <div className="space-y-2">
              <Badge>admin/ui</Badge>
              <div>
                <h1 className="font-mono text-2xl font-semibold tracking-tight">
                  LLM Gateway
                </h1>
                <p className="mt-1 max-w-2xl text-sm text-zinc-600">
                  Generic gateway for request routing, path rewrite, and header mutation.
                  No protocol adaptation. No body rewriting.
                </p>
              </div>
            </div>
            <div className="grid gap-1 text-sm text-zinc-600">
              <div>Gateway: {config.listen}</div>
              <div>Admin: {config.admin_listen}</div>
              <div>Secret Env: {config.default_secret_env || "<unset>"}</div>
            </div>
          </div>
          <div className="rounded-md border border-zinc-200 bg-white px-3 py-2 text-sm text-zinc-600">
            {status}
          </div>
        </header>

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
          <Button onClick={reload} disabled={busy} className="gap-2">
            <ChevronRight className="h-4 w-4" />
            Reload from Disk
          </Button>
        </div>

        <div className="grid gap-4 lg:grid-cols-[220px_minmax(0,1fr)]">
          <Card className="h-fit p-2">
            <nav className="space-y-1">
              {tabs.map((item) => (
                <button
                  key={item.key}
                  onClick={() => setTab(item.key)}
                  className={`flex w-full items-center justify-between rounded-md px-3 py-2 text-left text-sm transition ${
                    tab === item.key
                      ? "bg-zinc-900 text-white"
                      : "text-zinc-600 hover:bg-zinc-100 hover:text-zinc-900"
                  }`}
                >
                  {item.label}
                  <ChevronRight className="h-4 w-4" />
                </button>
              ))}
            </nav>
          </Card>

          <div className="space-y-6">
            {tab === "providers" && (
              <ResourceCard
                title="Providers"
                description="Upstream base URLs and default headers."
                onAdd={() =>
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
                {config.providers.map((provider, index) => (
                  <Card key={provider.id}>
                    <SectionActions
                      onRemove={() =>
                        setConfig((current) => ({
                          ...current,
                          providers: current.providers.filter((_, item) => item !== index),
                        }))
                      }
                    />
                    <div className="grid gap-3 md:grid-cols-3">
                      <Field
                        label="ID"
                        value={provider.id}
                        onChange={(value) =>
                          updateItem(config.providers, index, setConfig, "providers", {
                            ...provider,
                            id: value,
                          })
                        }
                      />
                      <Field
                        label="Name"
                        value={provider.name}
                        onChange={(value) =>
                          updateItem(config.providers, index, setConfig, "providers", {
                            ...provider,
                            name: value,
                          })
                        }
                      />
                      <Field
                        label="Base URL"
                        value={provider.base_url}
                        onChange={(value) =>
                          updateItem(config.providers, index, setConfig, "providers", {
                            ...provider,
                            base_url: value,
                          })
                        }
                      />
                    </div>
                    <div className="mt-4 space-y-3">
                      <div className="flex items-center justify-between">
                        <div className="font-mono text-xs uppercase tracking-[0.18em] text-zinc-500">
                          default headers
                        </div>
                        <Button
                          className="bg-white text-zinc-900"
                          onClick={() =>
                            updateItem(config.providers, index, setConfig, "providers", {
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
                        <div key={`${provider.id}-${headerIndex}`} className="grid gap-3 md:grid-cols-4">
                          <Field
                            label="Header"
                            value={header.name}
                            onChange={(value) => {
                              const next = [...provider.default_headers];
                              next[headerIndex] = { ...header, name: value };
                              updateItem(config.providers, index, setConfig, "providers", {
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
                              updateItem(config.providers, index, setConfig, "providers", {
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
                              updateItem(config.providers, index, setConfig, "providers", {
                                ...provider,
                                default_headers: next,
                              });
                            }}
                          />
                          <div className="flex items-end gap-2">
                            <Button
                              className={`flex-1 ${
                                "encrypted" in header.value && header.value.encrypted
                                  ? "border-zinc-900 bg-zinc-900 text-white hover:bg-zinc-800"
                                  : "bg-white text-zinc-900"
                              }`}
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
                                updateItem(config.providers, index, setConfig, "providers", {
                                  ...provider,
                                  default_headers: next,
                                });
                              }}
                            >
                              {("encrypted" in header.value && header.value.encrypted)
                                ? "Encrypted"
                                : "Plain"}
                            </Button>
                            <Button
                              className="bg-white text-zinc-900"
                              onClick={() => {
                                const next = provider.default_headers.filter((_, item) => item !== headerIndex);
                                updateItem(config.providers, index, setConfig, "providers", {
                                  ...provider,
                                  default_headers: next,
                                });
                              }}
                            >
                              Remove
                            </Button>
                          </div>
                        </div>
                      ))}
                    </div>
                  </Card>
                ))}
              </ResourceCard>
            )}

            {tab === "models" && (
              <ResourceCard
                title="Models"
                description="Logical models bound to providers."
                onAdd={() =>
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
                    <Field
                      label="Description"
                      value={model.description ?? ""}
                      onChange={(value) =>
                        updateItem(config.models, index, setConfig, "models", {
                          ...model,
                          description: value,
                        })
                      }
                    />
                  </EditorRow>
                ))}
              </ResourceCard>
            )}

            {tab === "routes" && (
              <ResourceCard
                title="Routes"
                description="Priority-based matching and optional path rewrite."
                onAdd={() =>
                  setConfig((current) => ({
                    ...current,
                    routes: [
                      ...current.routes,
                      {
                        id: `route-${current.routes.length + 1}`,
                        priority: 100,
                        enabled: true,
                        matcher: 'method == "POST"',
                        provider_id: current.providers[0]?.id ?? "",
                        model_id: current.models[0]?.id ?? "",
                        path_rewrite: "/v1/chat/completions",
                      },
                    ],
                  }))
                }
              >
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
                      label="Priority"
                      value={String(route.priority)}
                      onChange={(value) =>
                        updateItem(config.routes, index, setConfig, "routes", {
                          ...route,
                          priority: Number(value) || 0,
                        })
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
                    <Field
                      label="Model ID"
                      value={route.model_id ?? ""}
                      onChange={(value) =>
                        updateItem(config.routes, index, setConfig, "routes", {
                          ...route,
                          model_id: value,
                        })
                      }
                    />
                    <Field
                      label="Path Rewrite"
                      value={route.path_rewrite ?? ""}
                      onChange={(value) =>
                        updateItem(config.routes, index, setConfig, "routes", {
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
                          updateItem(config.routes, index, setConfig, "routes", {
                            ...route,
                            matcher: event.target.value,
                          })
                        }
                      />
                    </div>
                  </EditorRow>
                ))}
              </ResourceCard>
            )}

            {tab === "rules" && (
              <ResourceCard
                title="Header Rules"
                description="Declarative header mutation with optional conditions."
                onAdd={() =>
                  setConfig((current) => ({
                    ...current,
                    header_rules: [
                      ...current.header_rules,
                      {
                        id: `rule-${current.header_rules.length + 1}`,
                        enabled: true,
                        scope: "global",
                        target_id: "",
                        when: "",
                        actions: [{ type: "set", name: "X-Debug", value: "on" }],
                      },
                    ],
                  }))
                }
              >
                {config.header_rules.map((rule, index) => (
                  <Card key={rule.id}>
                    <SectionActions
                      onRemove={() =>
                        setConfig((current) => ({
                          ...current,
                          header_rules: current.header_rules.filter((_, item) => item !== index),
                        }))
                      }
                    />
                    <div className="grid gap-3 md:grid-cols-4">
                      <Field
                        label="ID"
                        value={rule.id}
                        onChange={(value) =>
                          updateItem(config.header_rules, index, setConfig, "header_rules", {
                            ...rule,
                            id: value,
                          })
                        }
                      />
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
                      <Field
                        label="Enabled"
                        value={String(rule.enabled)}
                        onChange={(value) =>
                          updateItem(config.header_rules, index, setConfig, "header_rules", {
                            ...rule,
                            enabled: value !== "false",
                          })
                        }
                      />
                    </div>
                    <div className="mt-4">
                      <Label>When</Label>
                      <Textarea
                        value={rule.when ?? ""}
                        onChange={(event) =>
                          updateItem(config.header_rules, index, setConfig, "header_rules", {
                            ...rule,
                            when: event.target.value,
                          })
                        }
                      />
                    </div>
                    <div className="mt-4 space-y-3">
                      <div className="flex items-center justify-between">
                        <div className="font-mono text-xs uppercase tracking-[0.18em] text-zinc-500">
                          actions
                        </div>
                        <Button
                          className="bg-white text-zinc-900"
                          onClick={() => {
                            updateItem(config.header_rules, index, setConfig, "header_rules", {
                              ...rule,
                              actions: [...rule.actions, { type: "remove", name: "X-New-Header" }],
                            });
                          }}
                        >
                          Add Action
                        </Button>
                      </div>
                      {rule.actions.map((action, actionIndex) => (
                        <ActionEditor
                          key={`${rule.id}-${actionIndex}`}
                          action={action}
                          onChange={(nextAction) => {
                            const next = [...rule.actions];
                            next[actionIndex] = nextAction;
                            updateItem(config.header_rules, index, setConfig, "header_rules", {
                              ...rule,
                              actions: next,
                            });
                          }}
                          onRemove={() => {
                            const next = rule.actions.filter((_, item) => item !== actionIndex);
                            updateItem(config.header_rules, index, setConfig, "header_rules", {
                              ...rule,
                              actions: next,
                            });
                          }}
                        />
                      ))}
                    </div>
                  </Card>
                ))}
              </ResourceCard>
            )}

            {tab === "raw" && (
              <ResourceCard
                title="Raw Config Snapshot"
                description="Live JSON view of the current editor state."
                onAdd={() => undefined}
                hideAdd
              >
                <pre className="overflow-x-auto rounded-md border border-zinc-200 bg-zinc-950 p-4 font-mono text-xs text-zinc-100">
                  {JSON.stringify(config, null, 2)}
                </pre>
              </ResourceCard>
            )}
          </div>
        </div>
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

function ResourceCard({
  title,
  description,
  onAdd,
  children,
  hideAdd,
}: React.PropsWithChildren<{
  title: string;
  description: string;
  onAdd: () => void;
  hideAdd?: boolean;
}>) {
  return (
    <Card className="space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <div className="font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
            {title}
          </div>
          <p className="mt-1 max-w-2xl text-sm text-zinc-600">{description}</p>
        </div>
        {!hideAdd && (
          <Button onClick={onAdd} className="shrink-0">
            Add
          </Button>
        )}
      </div>
      <div className="space-y-4">{children}</div>
    </Card>
  );
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

function ActionEditor({
  action,
  onChange,
  onRemove,
}: {
  action: HeaderAction;
  onChange: (action: HeaderAction) => void;
  onRemove: () => void;
}) {
  return (
    <div className="rounded-md border border-zinc-200 bg-zinc-50 p-3">
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
        <div className="flex items-end">
          <Button onClick={onRemove} className="w-full bg-white text-zinc-900">
            Remove
          </Button>
        </div>
      </div>
    </div>
  );
}
