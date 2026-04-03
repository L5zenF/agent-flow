import { useEffect, useState } from "react";
import {
  ChevronRight,
  CircleOff,
  RefreshCw,
  Save,
  ShieldEllipsis,
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
    <div className="min-h-screen bg-[radial-gradient(circle_at_top_left,_rgba(198,93,22,0.16),_transparent_26%),linear-gradient(180deg,_#f5efe3_0%,_#e7decd_100%)] text-ink">
      <div className="mx-auto flex max-w-7xl flex-col gap-6 px-4 py-6 lg:px-8">
        <header className="grid gap-4 lg:grid-cols-[1.3fr_0.7fr]">
          <Card className="overflow-hidden border-ink bg-ink text-paper">
            <div className="flex items-start justify-between gap-4">
              <div className="space-y-3">
                <Badge className="border-paper/20 bg-paper/10 text-paper">
                  practical admin panel
                </Badge>
                <div>
                  <h1 className="font-mono text-3xl uppercase tracking-[0.18em]">
                    LLM Gateway
                  </h1>
                  <p className="mt-2 max-w-2xl text-sm text-paper/72">
                    Generic request gateway for providers, models, route matching,
                    path rewrite, and header injection. No body adaptation.
                  </p>
                </div>
              </div>
              <ShieldEllipsis className="h-10 w-10 text-ember" />
            </div>
          </Card>
          <Card className="border-steel/25 bg-white/70">
            <div className="space-y-4">
              <div>
                <div className="font-mono text-xs uppercase tracking-[0.3em] text-steel">
                  runtime
                </div>
                <div className="mt-2 grid gap-2 text-sm">
                  <div>Gateway: {config.listen}</div>
                  <div>Admin: {config.admin_listen}</div>
                  <div>Secret Env: {config.default_secret_env || "<unset>"}</div>
                </div>
              </div>
              <div className="rounded-sm border border-steel/20 bg-paper/50 p-3 text-sm">
                {status}
              </div>
            </div>
          </Card>
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

        <div className="grid gap-6 lg:grid-cols-[220px_minmax(0,1fr)]">
          <Card className="h-fit border-steel/20 bg-white/60 p-2">
            <nav className="space-y-1">
              {tabs.map((item) => (
                <button
                  key={item.key}
                  onClick={() => setTab(item.key)}
                  className={`flex w-full items-center justify-between rounded-sm px-3 py-2 text-left text-sm ${
                    tab === item.key
                      ? "bg-ink text-paper"
                      : "text-steel hover:bg-white/60 hover:text-ink"
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
                  <Card key={provider.id} className="bg-white/70">
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
                        <div className="font-mono text-xs uppercase tracking-[0.24em] text-steel">
                          default headers
                        </div>
                        <Button
                          className="bg-white text-ink"
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
                                  ? "bg-ember text-white"
                                  : "bg-white text-ink"
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
                              className="bg-white text-ink"
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
                  <Card key={rule.id} className="bg-white/70">
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
                        <div className="font-mono text-xs uppercase tracking-[0.24em] text-steel">
                          actions
                        </div>
                        <Button
                          className="bg-white text-ink"
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
                <pre className="overflow-x-auto rounded-sm border border-steel/20 bg-ink p-4 font-mono text-xs text-paper">
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
    <Card className="space-y-4 border-ink/10 bg-white/55">
      <div className="flex items-start justify-between gap-4">
        <div>
          <div className="font-mono text-xs uppercase tracking-[0.28em] text-steel">
            {title}
          </div>
          <p className="mt-2 max-w-2xl text-sm text-steel">{description}</p>
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
    <Card className="bg-white/70">
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
        <div className="font-mono text-xs uppercase tracking-[0.24em] text-steel">
          {title}
        </div>
      ) : (
        <div />
      )}
      <Button onClick={onRemove} className="bg-white text-ink">
        <CircleOff className="mr-2 h-4 w-4" />
        Remove
      </Button>
    </div>
  );
}

function Label({ children }: React.PropsWithChildren) {
  return (
    <div className="mb-1 font-mono text-[11px] uppercase tracking-[0.24em] text-steel">
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
    <div className="rounded-sm border border-steel/20 bg-paper/60 p-3">
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
          <Button onClick={onRemove} className="w-full bg-white text-ink">
            Remove
          </Button>
        </div>
      </div>
    </div>
  );
}
