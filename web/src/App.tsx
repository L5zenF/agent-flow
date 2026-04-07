import { useEffect, useMemo, useRef, useState } from "react";
import { flushSync } from "react-dom";
import {
  ArrowLeft,
  CheckCircle2,
  CircleOff,
  FolderOpen,
  Plus,
  RefreshCw,
  Save,
  Settings2,
  Sparkles,
  X,
} from "lucide-react";
import { api } from "@/lib/api";
import {
  emptyConfig,
  type GatewayConfig,
  type WasmPluginManifestSummary,
  type WorkflowDocument,
  type WorkflowSummary,
} from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { RuleGraphEditor } from "@/components/rule-graph-editor";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";

type ConfigAction = React.SetStateAction<GatewayConfig>;

export default function App() {
  const [config, setConfig] = useState<GatewayConfig>(emptyConfig);
  const [workflowSummaries, setWorkflowSummaries] = useState<WorkflowSummary[]>([]);
  const [openedWorkflowId, setOpenedWorkflowId] = useState<string | null>(null);
  const [openedWorkflow, setOpenedWorkflow] = useState<WorkflowDocument | null>(null);
  const [pluginManifests, setPluginManifests] = useState<WasmPluginManifestSummary[]>([]);
  const [status, setStatus] = useState("Loading...");
  const [busy, setBusy] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [newWorkflowOpen, setNewWorkflowOpen] = useState(false);
  const [newWorkflowDraft, setNewWorkflowDraft] = useState({
    id: "",
    name: "",
    description: "",
  });
  const latestConfigRef = useRef(config);
  const latestOpenedWorkflowRef = useRef(openedWorkflow);
  const latestOpenedWorkflowIdRef = useRef(openedWorkflowId);

  latestConfigRef.current = config;
  latestOpenedWorkflowRef.current = openedWorkflow;
  latestOpenedWorkflowIdRef.current = openedWorkflowId;

  const openedWorkflowSummary = useMemo(
    () => workflowSummaries.find((workflow) => workflow.id === openedWorkflowId) ?? null,
    [openedWorkflowId, workflowSummaries],
  );

  const editorConfig = useMemo<GatewayConfig>(() => {
    if (!openedWorkflow) {
      return config;
    }

    return {
      ...config,
      rule_graph: openedWorkflow.workflow,
    };
  }, [config, openedWorkflow]);

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

  function applyEditorConfig(action: ConfigAction) {
    const currentWorkflow = latestOpenedWorkflowRef.current;
    const baseConfig: GatewayConfig = currentWorkflow
      ? {
          ...latestConfigRef.current,
          rule_graph: currentWorkflow.workflow,
        }
      : latestConfigRef.current;
    const nextConfig = typeof action === "function" ? action(baseConfig) : action;

    setConfig(stripRuleGraph(nextConfig));
    if (currentWorkflow && nextConfig.rule_graph) {
      setOpenedWorkflow({
        workflow: nextConfig.rule_graph,
      });
    }
  }

  async function load(targetWorkflowId?: string | null) {
    setBusy(true);
    try {
      const [nextConfig, nextWorkflows, nextPlugins] = await Promise.all([
        api.getConfig(),
        api.getWorkflows(),
        api.getPlugins(),
      ]);

      setConfig(stripRuleGraph(nextConfig));
      setWorkflowSummaries(nextWorkflows);
      setPluginManifests(nextPlugins);

      const workflowId =
        targetWorkflowId === undefined ? latestOpenedWorkflowIdRef.current : targetWorkflowId;
      if (workflowId) {
        try {
          const nextWorkflow = await api.getWorkflow(workflowId);
          setOpenedWorkflowId(workflowId);
          setOpenedWorkflow(nextWorkflow);
          setStatus(`Workflow "${workflowId}" loaded.`);
        } catch (error) {
          setOpenedWorkflowId(null);
          setOpenedWorkflow(null);
          setStatus(
            error instanceof Error
              ? `${error.message} Showing workflow gallery instead.`
              : "Failed to load workflow. Showing workflow gallery instead.",
          );
        }
      } else {
        setOpenedWorkflowId(null);
        setOpenedWorkflow(null);
        setStatus("Workflow gallery loaded.");
      }
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Failed to load admin state.");
    } finally {
      setBusy(false);
    }
  }

  async function openWorkflow(id: string) {
    setBusy(true);
    try {
      const workflow = await api.getWorkflow(id);
      setOpenedWorkflowId(id);
      setOpenedWorkflow(workflow);
      setStatus(`Workflow "${id}" opened.`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Failed to open workflow.");
    } finally {
      setBusy(false);
    }
  }

  async function activateWorkflow(id: string) {
    setBusy(true);
    try {
      const summary = await api.activateWorkflow(id);
      setWorkflowSummaries((current) =>
        current.map((workflow) => ({
          ...workflow,
          is_active: workflow.id === summary.id,
        })),
      );
      setConfig((current) => ({
        ...current,
        active_workflow_id: summary.id,
      }));
      setStatus(`Workflow "${summary.name}" is now active.`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Failed to activate workflow.");
    } finally {
      setBusy(false);
    }
  }

  async function createWorkflow() {
    setBusy(true);
    try {
      const created = await api.createWorkflow({
        id: newWorkflowDraft.id,
        name: newWorkflowDraft.name,
        description: newWorkflowDraft.description || null,
      });
      setWorkflowSummaries((current) => [...current, created]);
      setConfig((current) => ({
        ...current,
        active_workflow_id: created.is_active ? created.id : current.active_workflow_id,
        workflows: current.workflows.some((workflow) => workflow.id === created.id)
          ? current.workflows
          : [
              ...current.workflows,
              {
                id: created.id,
                name: created.name,
                file: created.file,
                description: created.description ?? null,
              },
            ],
      }));
      setNewWorkflowDraft({ id: "", name: "", description: "" });
      setNewWorkflowOpen(false);
      await openWorkflow(created.id);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Failed to create workflow.");
      setBusy(false);
    }
  }

  async function save() {
    setBusy(true);
    try {
      await flushPendingEditorState();

      const workflowId = latestOpenedWorkflowIdRef.current;
      const currentWorkflow = latestOpenedWorkflowRef.current;
      if (workflowId && currentWorkflow) {
        const savedWorkflow = await api.saveWorkflow(workflowId, currentWorkflow);
        setOpenedWorkflow(savedWorkflow);
        setWorkflowSummaries((current) =>
          current.map((workflow) =>
            workflow.id === workflowId
              ? {
                  ...workflow,
                  node_count: savedWorkflow.workflow.nodes.length,
                  edge_count: savedWorkflow.workflow.edges.length,
                }
              : workflow,
          ),
        );
      }

      await api.saveConfig(stripRuleGraph(latestConfigRef.current));
      setStatus(workflowId ? `Workflow "${workflowId}" and config saved.` : "Config saved.");
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

  function closeWorkflow() {
    setOpenedWorkflowId(null);
    setOpenedWorkflow(null);
    setStatus("Workflow gallery ready.");
  }

  return (
    <div className="min-h-screen bg-zinc-50 text-zinc-900">
      <div className="flex min-h-screen flex-col">
        <header
          className={[
            "border-b border-zinc-200/80 bg-white/75 backdrop-blur-md",
            openedWorkflow ? "px-3 py-2" : "px-4 py-3",
          ].join(" ")}
        >
          <div className="mx-auto flex max-w-7xl items-center justify-between gap-4">
            <div className="min-w-0">
              {openedWorkflow ? (
                <div className="flex min-w-0 items-center gap-2">
                  <Badge>editing</Badge>
                  <div className="truncate text-sm font-medium text-zinc-900">
                    {openedWorkflowSummary?.name ?? openedWorkflowId ?? "Workflow"}
                  </div>
                </div>
              ) : (
                <>
                  <Badge>gateway switch</Badge>
                  <div className="mt-2 min-w-0">
                    <h1 className="truncate font-mono text-xl font-semibold tracking-tight">
                      LLM Gateway
                    </h1>
                    <p className="mt-0.5 truncate text-sm text-zinc-500">
                      Workflow gallery for opening, activating, and editing rule graph canvases.
                    </p>
                  </div>
                </>
              )}
            </div>
            <div className="flex items-center gap-2">
              <TopBarButton label="Settings" onClick={openSettings}>
                <Settings2 className="h-4 w-4" />
              </TopBarButton>
              <TopBarButton label="Reload admin state" onClick={() => void load()} disabled={busy}>
                <RefreshCw className="h-4 w-4" />
              </TopBarButton>
              <TopBarButton label="Save" onClick={() => void save()} disabled={busy}>
                <Save className="h-4 w-4" />
              </TopBarButton>
            </div>
          </div>
          {!openedWorkflow ? (
            <div className="mx-auto mt-2 flex max-w-7xl items-center justify-start">
              <div className={`status-chip ${busy ? "status-chip-busy" : ""}`}>
                <span className={`status-dot ${busy ? "status-dot-busy" : ""}`} />
                <span className="min-w-0 truncate">{status}</span>
              </div>
            </div>
          ) : null}
        </header>

        <main className={openedWorkflow ? "flex-1 px-2 pb-2 pt-2 lg:px-3" : "flex-1 px-3 pb-3 pt-2 lg:px-4"}>
          {openedWorkflow && openedWorkflowSummary ? (
            <WorkflowEditorShell
              summary={openedWorkflowSummary}
              config={editorConfig}
              setConfig={applyEditorConfig}
              pluginManifests={pluginManifests}
              onBack={closeWorkflow}
              onSetActive={() => void activateWorkflow(openedWorkflowSummary.id)}
            />
          ) : (
            <WorkflowGallery
              workflows={workflowSummaries}
              busy={busy}
              onOpen={(id) => void openWorkflow(id)}
              onActivate={(id) => void activateWorkflow(id)}
              onCreate={() => setNewWorkflowOpen(true)}
            />
          )}
        </main>

        <SettingsModal
          config={editorConfig}
          open={settingsOpen}
          setConfig={applyEditorConfig}
          onClose={closeSettings}
        />
        <NewWorkflowModal
          open={newWorkflowOpen}
          busy={busy}
          draft={newWorkflowDraft}
          onDraftChange={setNewWorkflowDraft}
          onCreate={() => void createWorkflow()}
          onClose={() => setNewWorkflowOpen(false)}
        />
      </div>
    </div>
  );
}

function WorkflowGallery({
  workflows,
  busy,
  onOpen,
  onActivate,
  onCreate,
}: {
  workflows: WorkflowSummary[];
  busy: boolean;
  onOpen: (id: string) => void;
  onActivate: (id: string) => void;
  onCreate: () => void;
}) {
  const orderedWorkflows = useMemo(
    () =>
      [...workflows].sort((left, right) => {
        if (left.is_active !== right.is_active) {
          return left.is_active ? -1 : 1;
        }
        return left.name.localeCompare(right.name);
      }),
    [workflows],
  );

  return (
    <div className="mx-auto flex max-w-7xl flex-col gap-4">
      <section className="rounded-[28px] border border-zinc-200 bg-[linear-gradient(135deg,rgba(255,255,255,0.96),rgba(244,244,245,0.92)_45%,rgba(228,232,240,0.92))] p-6 shadow-[0_24px_80px_rgba(15,23,42,0.08)]">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
          <div className="max-w-2xl">
            <div className="font-mono text-[11px] uppercase tracking-[0.2em] text-zinc-500">
              Workflow Gallery
            </div>
            <h2 className="mt-2 text-2xl font-semibold tracking-tight text-zinc-950">
              Choose the workflow canvas you want to work on.
            </h2>
            <p className="mt-2 text-sm leading-6 text-zinc-600">
              Global settings stay available from the top bar. Opening a workflow switches into the
              existing rule graph editor for that document only.
            </p>
          </div>
          <Button className="gap-2 self-start lg:self-auto" onClick={onCreate} disabled={busy}>
            <Plus className="h-4 w-4" />
            New Workflow
          </Button>
        </div>
      </section>

      {orderedWorkflows.length === 0 ? (
        <Card className="rounded-[24px] border-dashed border-zinc-300 bg-white/90 p-10 text-center shadow-none">
          <div className="mx-auto flex h-14 w-14 items-center justify-center rounded-full bg-zinc-100 text-zinc-500">
            <Sparkles className="h-6 w-6" />
          </div>
          <h3 className="mt-4 text-lg font-semibold text-zinc-900">No workflows yet</h3>
          <p className="mx-auto mt-2 max-w-md text-sm text-zinc-500">
            Create the first workflow to start routing requests through the graph editor.
          </p>
          <div className="mt-6">
            <Button onClick={onCreate} disabled={busy}>
              <Plus className="h-4 w-4" />
              New Workflow
            </Button>
          </div>
        </Card>
      ) : (
        <div className="grid gap-4 lg:grid-cols-2 xl:grid-cols-3">
          {orderedWorkflows.map((workflow) => (
            <Card
              key={workflow.id}
              className="rounded-[24px] border border-zinc-200/80 bg-white/90 p-5 shadow-[0_20px_60px_rgba(15,23,42,0.06)]"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <div className="truncate text-lg font-semibold text-zinc-950">{workflow.name}</div>
                    {workflow.is_active ? <Badge>Active</Badge> : <Badge variant="secondary">Draft</Badge>}
                  </div>
                  <div className="mt-1 font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
                    {workflow.id}
                  </div>
                </div>
                <div className="rounded-full border border-zinc-200 bg-zinc-50 px-3 py-1 text-xs text-zinc-600">
                  {workflow.node_count} nodes
                </div>
              </div>

              <p className="mt-4 min-h-12 text-sm leading-6 text-zinc-600">
                {workflow.description?.trim() || "No description yet."}
              </p>

              <div className="mt-4 rounded-2xl border border-zinc-200 bg-zinc-50/80 px-4 py-3 text-sm text-zinc-600">
                <div className="flex items-center justify-between gap-3">
                  <span>Edges</span>
                  <span className="font-medium text-zinc-900">{workflow.edge_count}</span>
                </div>
                <div className="mt-2 flex items-center justify-between gap-3">
                  <span>File</span>
                  <span className="truncate font-mono text-xs text-zinc-500">{workflow.file}</span>
                </div>
              </div>

              <div className="mt-5 flex flex-wrap gap-2">
                <Button className="gap-2" onClick={() => onOpen(workflow.id)} disabled={busy}>
                  <FolderOpen className="h-4 w-4" />
                  Open Workflow
                </Button>
                <Button
                  variant="outline"
                  className="gap-2"
                  onClick={() => onActivate(workflow.id)}
                  disabled={busy || workflow.is_active}
                >
                  <CheckCircle2 className="h-4 w-4" />
                  Set Active
                </Button>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

function WorkflowEditorShell({
  summary,
  config,
  setConfig,
  pluginManifests,
  onBack,
  onSetActive,
}: {
  summary: WorkflowSummary;
  config: GatewayConfig;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
  pluginManifests: WasmPluginManifestSummary[];
  onBack: () => void;
  onSetActive: () => void;
}) {
  return (
    <div className="mx-auto flex max-w-7xl flex-col gap-2">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" className="gap-2" onClick={onBack}>
            <ArrowLeft className="h-4 w-4" />
            Back
          </Button>
          {summary.is_active ? <Badge>Active</Badge> : <Badge variant="secondary">Inactive</Badge>}
          <span className="hidden font-mono text-xs text-zinc-500 md:inline">{summary.id}</span>
        </div>
        {!summary.is_active ? (
          <Button variant="outline" size="sm" className="gap-2" onClick={onSetActive}>
            <CheckCircle2 className="h-4 w-4" />
            Set Active
          </Button>
        ) : null}
      </div>
      <RuleGraphEditor config={config} setConfig={setConfig} pluginManifests={pluginManifests} />
    </div>
  );
}

function NewWorkflowModal({
  open,
  busy,
  draft,
  onDraftChange,
  onCreate,
  onClose,
}: {
  open: boolean;
  busy: boolean;
  draft: {
    id: string;
    name: string;
    description: string;
  };
  onDraftChange: React.Dispatch<
    React.SetStateAction<{
      id: string;
      name: string;
      description: string;
    }>
  >;
  onCreate: () => void;
  onClose: () => void;
}) {
  if (!open) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-zinc-950/35 p-4">
      <div className="w-full max-w-xl rounded-3xl border border-zinc-200 bg-white shadow-[0_30px_120px_rgba(15,23,42,0.18)]">
        <div className="flex items-start justify-between gap-4 border-b border-zinc-200 px-5 py-4 sm:px-6">
          <div>
            <div className="font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
              New Workflow
            </div>
            <div className="mt-1 text-lg font-semibold text-zinc-900">Create a workflow document</div>
            <p className="mt-1 text-sm text-zinc-500">
              A new workflow file will be added to the indexed workflow gallery.
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

        <div className="space-y-4 px-5 py-5 sm:px-6">
          <Field
            label="Workflow ID"
            value={draft.id}
            onChange={(value) => onDraftChange((current) => ({ ...current, id: value }))}
          />
          <Field
            label="Workflow Name"
            value={draft.name}
            onChange={(value) => onDraftChange((current) => ({ ...current, name: value }))}
          />
          <label className="block">
            <Label>Description</Label>
            <Textarea
              value={draft.description}
              onChange={(event) =>
                onDraftChange((current) => ({ ...current, description: event.target.value }))
              }
              rows={4}
              placeholder="Optional description for the gallery card."
            />
          </label>
        </div>

        <div className="flex justify-end gap-2 border-t border-zinc-200 px-5 py-4 sm:px-6">
          <Button variant="outline" onClick={onClose} disabled={busy}>
            Cancel
          </Button>
          <Button onClick={onCreate} disabled={busy}>
            Create Workflow
          </Button>
        </div>
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
            description="These values feed the same config object used by the workflow gallery and canvas editor."
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
            description="Manage upstream providers and their default headers without leaving the admin shell."
          >
            <ProvidersSection config={config} setConfig={setConfig} />
          </SettingsSection>

          <SettingsSection
            title="Models"
            description="Attach models to providers using the same shared config state consumed by the graph inspector."
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
            node.select_model?.provider_id === previousId
              ? {
                  ...node,
                  select_model: {
                    provider_id: effectiveId,
                    model_id: node.select_model?.model_id ?? "",
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
            if (node.select_model?.provider_id === providerId) {
              return {
                ...node,
                select_model: {
                  provider_id: "",
                  model_id:
                    node.select_model?.model_id && removedModelIds.has(node.select_model.model_id)
                      ? ""
                      : (node.select_model?.model_id ?? ""),
                },
              };
            }

            if (node.select_model?.model_id && removedModelIds.has(node.select_model.model_id)) {
              return {
                ...node,
                select_model: {
                  provider_id: node.select_model?.provider_id ?? "",
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
                    provider_id: node.select_model?.provider_id ?? "",
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
                    provider_id: node.select_model?.provider_id ?? "",
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

function stripRuleGraph(config: GatewayConfig): GatewayConfig {
  const { rule_graph: _ruleGraph, ...rest } = config;
  return rest;
}
