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
  type SettingsSchema,
  type SettingsSchemaField,
  type SettingsSchemaSection,
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
  const [settingsSchema, setSettingsSchema] = useState<SettingsSchema | null>(null);
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
      const [nextConfig, nextWorkflows, nextPlugins, nextSettingsSchema] = await Promise.all([
        api.getConfig(),
        api.getWorkflows(),
        api.getPlugins(),
        api.getSettingsSchema().catch(() => null),
      ]);

      setConfig(stripRuleGraph(nextConfig));
      setWorkflowSummaries(nextWorkflows);
      setPluginManifests(nextPlugins);
      setSettingsSchema(nextSettingsSchema);

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
      const [summary, workflows] = await Promise.all([
        api.activateWorkflow(id),
        api.getWorkflows(),
      ]);
      setWorkflowSummaries(workflows);
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
            openedWorkflow ? "px-3 py-1.5" : "hidden",
          ].join(" ")}
        >
          <div className="mx-auto flex max-w-7xl items-center justify-between gap-4">
            <div className="min-w-0">
              {openedWorkflow ? (
                <div className="flex min-w-0 items-center gap-2">
                  <div className="h-2 w-2 rounded-full bg-emerald-500" />
                  <div className="truncate text-sm font-medium text-zinc-800">
                    {openedWorkflowSummary?.name ?? openedWorkflowId ?? "Workflow"}
                  </div>
                </div>
              ) : (
                <div className="min-w-0">
                  <h1 className="truncate text-lg font-semibold tracking-tight text-zinc-950">Gallery</h1>
                </div>
              )}
            </div>
            <div />
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

        <main className={openedWorkflow ? "flex-1 px-0 pb-0 pt-0" : "flex-1 px-3 pb-3 pt-3 lg:px-4"}>
          {openedWorkflow && openedWorkflowSummary ? (
            <WorkflowEditorShell
              summary={openedWorkflowSummary}
              busy={busy}
              config={editorConfig}
              setConfig={applyEditorConfig}
              pluginManifests={pluginManifests}
              onBack={closeWorkflow}
              onSetActive={() => void activateWorkflow(openedWorkflowSummary.id)}
              onOpenSettings={openSettings}
              onReload={() => void load(openedWorkflowSummary.id)}
              onSave={() => void save()}
            />
          ) : (
            <WorkflowGallery
              workflows={workflowSummaries}
              busy={busy}
              onOpen={(id) => void openWorkflow(id)}
              onActivate={(id) => void activateWorkflow(id)}
              onCreate={() => setNewWorkflowOpen(true)}
              onOpenSettings={openSettings}
              onReload={() => void load()}
              onSave={() => void save()}
            />
          )}
        </main>

        <SettingsModal
          config={editorConfig}
          schema={settingsSchema}
          busy={busy}
          open={settingsOpen}
          setConfig={applyEditorConfig}
          onSave={() => void save()}
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
  onOpenSettings,
  onReload,
  onSave,
}: {
  workflows: WorkflowSummary[];
  busy: boolean;
  onOpen: (id: string) => void;
  onActivate: (id: string) => void;
  onCreate: () => void;
  onOpenSettings: () => void;
  onReload: () => void;
  onSave: () => void;
}) {
  const orderedWorkflows = useMemo(
    () =>
      [...workflows].sort((left, right) => left.name.localeCompare(right.name)),
    [workflows],
  );

  return (
    <div className="relative mx-auto flex max-w-7xl flex-col gap-4 pb-24">
      <section className="flex items-start justify-between gap-4 rounded-[24px] border border-zinc-200 bg-white px-5 py-4 shadow-sm">
        <div className="min-w-0">
          <div className="text-2xl font-semibold tracking-tight text-zinc-950">Workflow Gallery</div>
          <div className="mt-2 flex flex-wrap items-center gap-2 text-xs">
            <div className="rounded-full border border-zinc-200 bg-zinc-50 px-3 py-1.5 text-zinc-600">
              {orderedWorkflows.length} workflows
            </div>
            <div className="rounded-full border border-zinc-200 bg-zinc-50 px-3 py-1.5 text-zinc-600">
              {orderedWorkflows.filter((workflow) => workflow.is_active).length} active
            </div>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2 rounded-2xl border border-zinc-200 bg-zinc-50/80 p-2">
          <Button size="sm" className="gap-2" onClick={onCreate} disabled={busy}>
            <Plus className="h-4 w-4" />
            New Workflow
          </Button>
          <TopBarButton label="Settings" onClick={onOpenSettings}>
            <Settings2 className="h-4 w-4" />
          </TopBarButton>
          <TopBarButton label="Reload admin state" onClick={onReload} disabled={busy}>
            <RefreshCw className="h-4 w-4" />
          </TopBarButton>
          <TopBarButton label="Save" onClick={onSave} disabled={busy}>
            <Save className="h-4 w-4" />
          </TopBarButton>
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
              className={[
                "relative overflow-hidden rounded-[24px] border p-5 shadow-sm transition hover:-translate-y-0.5 hover:shadow-md",
                workflow.is_active
                  ? "border-emerald-200 bg-white shadow-[inset_0_0_0_1px_rgba(16,185,129,0.06)]"
                  : "border-zinc-200 bg-white",
              ].join(" ")}
            >
              {workflow.is_active ? (
                <div className="absolute inset-y-0 left-0 w-1.5 bg-emerald-500/80" />
              ) : null}
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <div className="truncate text-lg font-semibold text-zinc-950">{workflow.name}</div>
                    {workflow.is_active ? <Badge>Active</Badge> : <Badge variant="secondary">Draft</Badge>}
                  </div>
                  <div className="mt-1 text-sm text-zinc-500">
                    {workflow.description?.trim() || "No description yet."}
                  </div>
                </div>
                <div className="rounded-full bg-zinc-100 px-3 py-1 text-xs font-medium text-zinc-600">
                  {workflow.node_count} nodes
                </div>
              </div>

              <div className="mt-4 grid grid-cols-2 gap-2">
                <div className="rounded-xl bg-zinc-50 px-3 py-2">
                  <div className="text-[11px] font-medium uppercase tracking-[0.14em] text-zinc-500">Nodes</div>
                  <div className="mt-1 text-base font-semibold text-zinc-900">{workflow.node_count}</div>
                </div>
                <div className="rounded-xl bg-zinc-50 px-3 py-2">
                  <div className="text-[11px] font-medium uppercase tracking-[0.14em] text-zinc-500">Edges</div>
                  <div className="mt-1 text-base font-semibold text-zinc-900">{workflow.edge_count}</div>
                </div>
              </div>

              <div className="mt-4 flex items-center justify-between gap-3 border-t border-zinc-100 pt-4">
                <div className="min-w-0 font-mono text-[11px] text-zinc-400">{workflow.id}</div>
                <div className="flex items-center gap-2">
                  {!workflow.is_active ? (
                    <Button
                      variant="outline"
                      size="sm"
                      className="gap-2"
                      onClick={() => onActivate(workflow.id)}
                      disabled={busy}
                    >
                      <CheckCircle2 className="h-4 w-4" />
                      Set Active
                    </Button>
                  ) : null}
                  <Button size="sm" className="gap-2" onClick={() => onOpen(workflow.id)} disabled={busy}>
                    <FolderOpen className="h-4 w-4" />
                    Open
                  </Button>
                </div>
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
  busy,
  config,
  setConfig,
  pluginManifests,
  onBack,
  onSetActive,
  onOpenSettings,
  onReload,
  onSave,
}: {
  summary: WorkflowSummary;
  busy: boolean;
  config: GatewayConfig;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
  pluginManifests: WasmPluginManifestSummary[];
  onBack: () => void;
  onSetActive: () => void;
  onOpenSettings: () => void;
  onReload: () => void;
  onSave: () => void;
}) {
  return (
    <div className="relative flex h-full flex-col">
      <div className="pointer-events-none absolute inset-x-0 top-0 z-20 flex items-start justify-start px-4 py-4">
        <div className="pointer-events-auto flex flex-wrap items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            className="h-9 rounded-full border-zinc-200 bg-white/95 px-3 shadow-sm backdrop-blur"
            onClick={onBack}
          >
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <div className="hidden items-center gap-2 rounded-full border border-zinc-200 bg-white/95 px-3 py-2 text-xs text-zinc-500 shadow-sm backdrop-blur md:flex">
            {summary.is_active ? <Badge>Active</Badge> : <Badge variant="secondary">Inactive</Badge>}
            <span className="font-mono">{summary.id}</span>
          </div>
          {!summary.is_active ? (
            <Button
              variant="outline"
              size="sm"
              className="h-9 rounded-full border-zinc-200 bg-white/95 px-3 shadow-sm backdrop-blur"
              onClick={onSetActive}
            >
              <CheckCircle2 className="mr-2 h-4 w-4" />
              Set Active
            </Button>
          ) : null}
        </div>
      </div>
      <RuleGraphEditor
        busy={busy}
        config={config}
        setConfig={setConfig}
        pluginManifests={pluginManifests}
        onOpenSettings={onOpenSettings}
        onReload={onReload}
        onSave={onSave}
      />
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
  schema,
  busy,
  open,
  setConfig,
  onSave,
  onClose,
}: {
  config: GatewayConfig;
  schema: SettingsSchema | null;
  busy: boolean;
  open: boolean;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
  onSave: () => void;
  onClose: () => void;
}) {
  if (!open) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-zinc-950/35 p-4">
      <div className="max-h-[88vh] w-full max-w-6xl overflow-hidden rounded-[28px] border border-zinc-200 bg-white shadow-[0_30px_120px_rgba(15,23,42,0.18)]">
        <div className="flex items-start justify-between gap-6 border-b border-zinc-200 px-5 py-5 sm:px-6">
          <div className="min-w-0">
            <div className="font-mono text-[11px] uppercase tracking-[0.18em] text-zinc-500">
              Gateway Settings
            </div>
            <div className="mt-2 text-2xl font-semibold tracking-tight text-zinc-950">
              Configure runtime, providers, and models
            </div>
            <p className="mt-1.5 max-w-2xl text-sm leading-6 text-zinc-500">
              Runtime values stay host-controlled. Provider cards are the main workspace for upstream
              configuration and attached models.
            </p>
            <div className="mt-4 flex flex-wrap items-center gap-2 text-xs">
              <div className="rounded-full border border-zinc-200 bg-zinc-50 px-3 py-1.5 text-zinc-600">
                {config.providers.length} providers
              </div>
              <div className="rounded-full border border-zinc-200 bg-zinc-50 px-3 py-1.5 text-zinc-600">
                {config.models.length} models
              </div>
              <div className="rounded-full border border-zinc-200 bg-zinc-50 px-3 py-1.5 text-zinc-600">
                Live config state
              </div>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-zinc-200 bg-white text-zinc-500 transition hover:border-zinc-300 hover:text-zinc-900"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="max-h-[calc(88vh-186px)] space-y-6 overflow-y-auto bg-zinc-50/60 px-5 py-5 sm:px-6">
          <SettingsSection
            title="Runtime"
            description="Host-level values that power the admin shell and gateway process."
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
            title="Providers & Models"
            description="Each provider owns its headers and the models attached to it."
          >
            <ProviderModelsSection config={config} setConfig={setConfig} schema={schema} />
          </SettingsSection>
        </div>

        <div className="flex items-center justify-between gap-3 border-t border-zinc-200 bg-white/95 px-5 py-4 backdrop-blur sm:px-6">
          <div className="text-sm text-zinc-500">
            Save writes the current settings back to the gateway config.
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" onClick={onClose} disabled={busy}>
              Close
            </Button>
            <Button onClick={onSave} disabled={busy}>
              <Save className="h-4 w-4" />
              Save Changes
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

function ProviderModelsSection({
  config,
  setConfig,
  schema,
}: {
  config: GatewayConfig;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
  schema: SettingsSchema | null;
}) {
  const providerSection = schema?.providers ?? null;
  const modelSection = schema?.models ?? null;
  const providerFields =
    providerSection?.fields ?? [
      { key: "id", label: "ID", type: "text" as const },
      { key: "name", label: "Name", type: "text" as const },
      { key: "base_url", label: "Base URL", type: "text" as const },
      {
        key: "default_headers",
        label: "Default Headers",
        type: "object_list" as const,
        fields: [
          { key: "name", label: "Header", type: "text" as const },
          { key: "value", label: "Value", type: "text" as const },
          { key: "secret_env", label: "Secret Env", type: "text" as const },
          { key: "encrypted", label: "Encrypted", type: "boolean" as const },
        ],
      },
    ];
  const modelFields =
    (modelSection?.fields ?? [
      { key: "id", label: "ID", type: "text" as const },
      { key: "name", label: "Name", type: "text" as const },
      { key: "description", label: "Description", type: "textarea" as const },
    ]).filter((field) => field.key !== "provider_id");

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between gap-3 rounded-2xl border border-zinc-200 bg-white px-4 py-3">
        <div>
          <div className="text-sm font-medium text-zinc-900">Providers workspace</div>
          <p className="mt-0.5 text-sm text-zinc-500">
            Add a provider first, then attach models directly inside its card.
          </p>
        </div>
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
          {providerSection?.add_label ?? "Add Provider"}
        </Button>
      </div>

      {config.providers.length === 0 ? (
        <EmptyMiniState text={providerSection?.empty_text ?? "No providers configured."} />
      ) : (
        config.providers.map((provider, providerIndex) => {
          const providerModels = config.models.filter((model) => model.provider_id === provider.id);

          return (
            <Card key={provider.id} className="rounded-[24px] border border-zinc-200 bg-white p-5 shadow-sm">
              <SectionActions
                title={provider.name || provider.id || `Provider ${providerIndex + 1}`}
                onRemove={() =>
                  setConfig((current) => removeProviderFromConfig(current, providerIndex, provider.id))
                }
              />

              <div className="space-y-5">
                <div className="flex flex-wrap items-center gap-2 text-xs">
                  <div className="rounded-full border border-zinc-200 bg-zinc-50 px-2.5 py-1 text-zinc-600">
                    {provider.id}
                  </div>
                  <div className="rounded-full border border-zinc-200 bg-zinc-50 px-2.5 py-1 text-zinc-600">
                    {providerModels.length} models
                  </div>
                  <div className="rounded-full border border-zinc-200 bg-zinc-50 px-2.5 py-1 text-zinc-600">
                    {provider.default_headers.length} headers
                  </div>
                </div>

                <div className="grid gap-3 md:grid-cols-2">
                  {providerFields.map((field) => (
                    <SchemaFieldControl
                      key={`${provider.id}-${field.key}`}
                      field={field}
                      value={(provider as Record<string, unknown>)[field.key]}
                      providerOptions={[]}
                      onChange={(nextValue) => {
                        if (field.key === "id") {
                          setConfig((current) =>
                            renameProviderInConfig(current, providerIndex, provider.id, String(nextValue)),
                          );
                          return;
                        }

                        const nextProviders = [...config.providers];
                        nextProviders[providerIndex] = { ...provider, [field.key]: nextValue };
                        setConfig((current) => ({ ...current, providers: nextProviders }));
                      }}
                    />
                  ))}
                </div>

                <div className="space-y-3 border-t border-zinc-100 pt-5">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <div className="font-mono text-[11px] uppercase tracking-[0.16em] text-zinc-500">
                        Models
                      </div>
                      <p className="mt-1 text-sm text-zinc-500">
                        Models under {provider.name || provider.id}.
                      </p>
                    </div>
                    <Button
                      variant="outline"
                      size="sm"
                      className="gap-2"
                      onClick={() =>
                        setConfig((current) => ({
                          ...current,
                          models: [
                            ...current.models,
                            {
                              id: `model-${current.models.length + 1}`,
                              name: "New Model",
                              provider_id: provider.id,
                              description: "",
                            },
                          ],
                        }))
                      }
                    >
                      <Plus className="h-4 w-4" />
                      {modelSection?.add_label ?? "Add Model"}
                    </Button>
                  </div>

                  {providerModels.length === 0 ? (
                    <EmptyMiniState text="No models attached to this provider." />
                  ) : (
                    providerModels.map((model, modelIndex) => {
                      const absoluteModelIndex = config.models.findIndex((item) => item.id === model.id);
                      return (
                        <Card
                          key={model.id}
                          className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4 shadow-none"
                        >
                          <SectionActions
                            title={model.name || model.id || `Model ${modelIndex + 1}`}
                            onRemove={() =>
                              setConfig((current) =>
                                removeModelFromConfig(current, absoluteModelIndex, model.id),
                              )
                            }
                          />

                          <div className="grid gap-3 md:grid-cols-2">
                            {modelFields.map((field) => (
                              <SchemaFieldControl
                                key={`${model.id}-${field.key}`}
                                field={field}
                                value={(model as Record<string, unknown>)[field.key]}
                                providerOptions={[]}
                                onChange={(nextValue) => {
                                  if (field.key === "id") {
                                    setConfig((current) =>
                                      renameModelInConfig(current, absoluteModelIndex, model.id, String(nextValue)),
                                    );
                                    return;
                                  }

                                  const nextModels = [...config.models];
                                  nextModels[absoluteModelIndex] = { ...model, [field.key]: nextValue };
                                  setConfig((current) => ({ ...current, models: nextModels }));
                                }}
                              />
                            ))}
                          </div>
                        </Card>
                      );
                    })
                  )}
                </div>
              </div>
            </Card>
          );
        })
      )}
    </div>
  );
}

function SettingsSection({
  title,
  description,
  children,
}: React.PropsWithChildren<{ title: string; description: string }>) {
  return (
    <section className="rounded-[24px] border border-zinc-200 bg-white p-5 shadow-sm sm:p-6">
      <div className="mb-5">
        <div className="font-mono text-[11px] uppercase tracking-[0.18em] text-zinc-500">
          {title}
        </div>
        <p className="mt-1.5 max-w-2xl text-sm leading-6 text-zinc-600">{description}</p>
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

type SchemaListKind = "providers" | "models";

function SchemaListSection({
  config,
  setConfig,
  section,
  kind,
}: {
  config: GatewayConfig;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
  section: SettingsSchemaSection;
  kind: SchemaListKind;
}) {
  const items = kind === "providers" ? config.providers : config.models;
  const providerOptions = config.providers.map((provider) => ({
    value: provider.id,
    label: provider.name || provider.id,
  }));

  const addItem = () => {
    setConfig((current) => {
      if (kind === "providers") {
        return {
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
        };
      }

      return {
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
      };
    });
  };

  const removeItem = (index: number, itemId: string) => {
    setConfig((current) =>
      kind === "providers"
        ? removeProviderFromConfig(current, index, itemId)
        : removeModelFromConfig(current, index, itemId),
    );
  };

  const updateField = (
    index: number,
    fieldKey: string,
    value: unknown,
    currentItem: GatewayConfig["providers"][number] | GatewayConfig["models"][number],
  ) => {
    setConfig((current) => {
      if (kind === "providers") {
        const provider = current.providers[index];
        if (!provider) {
          return current;
        }
        if (fieldKey === "id") {
          return renameProviderInConfig(current, index, provider.id, String(value));
        }
        const nextProviders = [...current.providers];
        nextProviders[index] = { ...provider, [fieldKey]: value };
        return { ...current, providers: nextProviders };
      }

      const model = current.models[index];
      if (!model) {
        return current;
      }
      if (fieldKey === "id") {
        return renameModelInConfig(current, index, model.id, String(value));
      }
      const nextModels = [...current.models];
      nextModels[index] = { ...model, [fieldKey]: value };
      return { ...current, models: nextModels };
    });
  };

  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <Button className="gap-2" onClick={addItem}>
          <Plus className="h-4 w-4" />
          {section.add_label ?? `Add ${section.title}`}
        </Button>
      </div>

      {items.length === 0 ? (
        <EmptyMiniState text={section.empty_text ?? `No ${section.title.toLowerCase()} configured.`} />
      ) : (
        items.map((item, index) => (
          <Card key={item.id} className="rounded-2xl border border-zinc-200">
            <SectionActions
              title={item.name || item.id || `${section.title} ${index + 1}`}
              onRemove={() => removeItem(index, item.id)}
            />

            <div className="space-y-4">
              {section.fields.map((field) => (
                <SchemaFieldControl
                  key={field.key}
                  field={field}
                  value={(item as Record<string, unknown>)[field.key]}
                  providerOptions={providerOptions}
                  onChange={(nextValue) => updateField(index, field.key, nextValue, item)}
                />
              ))}
            </div>
          </Card>
        ))
      )}
    </div>
  );
}

function SchemaFieldControl({
  field,
  value,
  providerOptions,
  onChange,
}: {
  field: SettingsSchemaField;
  value: unknown;
  providerOptions: Array<{ value: string; label: string }>;
  onChange: (value: unknown) => void;
}) {
  if (field.key === "default_headers" && field.type === "object_list") {
    const items = Array.isArray(value) ? (value as Array<Record<string, unknown>>) : [];

    const updateHeader = (index: number, patch: { name?: string; rawValue?: string; secretEnv?: string; encrypted?: boolean }) => {
      const nextItems = items.map((item, itemIndex) => {
        if (itemIndex !== index) {
          return item;
        }

        const currentValue =
          item.value && typeof item.value === "object" ? (item.value as Record<string, unknown>) : {};

        return {
          ...item,
          name: patch.name ?? (typeof item.name === "string" ? item.name : ""),
          value: {
            value: patch.rawValue ?? (typeof currentValue.value === "string" ? currentValue.value : ""),
            secret_env:
              patch.secretEnv !== undefined
                ? patch.secretEnv || null
                : (typeof currentValue.secret_env === "string" ? currentValue.secret_env : null),
            encrypted:
              patch.encrypted ?? (typeof currentValue.encrypted === "boolean" ? currentValue.encrypted : false),
          },
        };
      });

      onChange(nextItems);
    };

    return (
      <div className="space-y-3">
        <div className="flex items-center justify-between gap-3">
          <div>
            <Label>{field.label}</Label>
            {field.help_text ? <p className="mt-1 text-sm text-zinc-500">{field.help_text}</p> : null}
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() =>
              onChange([
                ...items,
                {
                  name: "X-New-Header",
                  value: { value: "", secret_env: null, encrypted: false },
                },
              ])
            }
          >
            Add Header
          </Button>
        </div>

        {items.length === 0 ? (
          <EmptyMiniState text="No default headers configured." />
        ) : (
          <div className="overflow-hidden rounded-2xl border border-zinc-200 bg-white">
            <div className="grid grid-cols-[minmax(0,1.1fr)_minmax(0,1.2fr)_minmax(0,0.9fr)_120px_44px] gap-3 border-b border-zinc-200 bg-zinc-50 px-4 py-2.5 text-[11px] font-medium uppercase tracking-[0.14em] text-zinc-500">
              <div>Header</div>
              <div>Value</div>
              <div>Secret Env</div>
              <div>Encrypted</div>
              <div />
            </div>
            <div className="divide-y divide-zinc-100">
              {items.map((item, index) => {
                const currentValue =
                  item.value && typeof item.value === "object" ? (item.value as Record<string, unknown>) : {};

                return (
                  <div
                    key={`${field.key}-${index}`}
                    className="grid grid-cols-[minmax(0,1.1fr)_minmax(0,1.2fr)_minmax(0,0.9fr)_120px_44px] gap-3 px-4 py-3"
                  >
                    <Input
                      value={typeof item.name === "string" ? item.name : ""}
                      onChange={(event) => updateHeader(index, { name: event.target.value })}
                      placeholder="X-Header-Name"
                    />
                    <Input
                      value={typeof currentValue.value === "string" ? currentValue.value : ""}
                      onChange={(event) => updateHeader(index, { rawValue: event.target.value })}
                      placeholder="Header value"
                    />
                    <Input
                      value={typeof currentValue.secret_env === "string" ? currentValue.secret_env : ""}
                      onChange={(event) => updateHeader(index, { secretEnv: event.target.value })}
                      placeholder="PROXY_SECRET"
                    />
                    <button
                      type="button"
                      onClick={() =>
                        updateHeader(index, {
                          encrypted: !(typeof currentValue.encrypted === "boolean" && currentValue.encrypted),
                        })
                      }
                      className={`inline-flex h-10 items-center justify-center rounded-md border px-3 text-sm transition ${
                        typeof currentValue.encrypted === "boolean" && currentValue.encrypted
                          ? "border-zinc-900 bg-zinc-900 text-white"
                          : "border-zinc-200 bg-white text-zinc-700 hover:border-zinc-300"
                      }`}
                    >
                      {typeof currentValue.encrypted === "boolean" && currentValue.encrypted ? "Yes" : "No"}
                    </button>
                    <button
                      type="button"
                      onClick={() => onChange(items.filter((_, itemIndex) => itemIndex !== index))}
                      className="inline-flex h-10 w-10 items-center justify-center rounded-md border border-zinc-200 bg-white text-zinc-500 transition hover:border-zinc-300 hover:text-zinc-900"
                      aria-label="Remove header"
                      title="Remove header"
                    >
                      <X className="h-4 w-4" />
                    </button>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>
    );
  }

  if (field.type === "object_list") {
    const items = Array.isArray(value) ? (value as Array<Record<string, unknown>>) : [];
    const nestedFields = field.fields ?? [];

    const addItem = () => {
      const nextItem = Object.fromEntries(
        nestedFields.map((nestedField) => [
          nestedField.key,
          nestedField.type === "boolean" ? false : "",
        ]),
      );
      onChange([...items, nextItem]);
    };

    return (
      <div className="space-y-3">
        <div className="flex items-center justify-between gap-3">
          <div>
            <Label>{field.label}</Label>
            {field.help_text ? <p className="mt-1 text-sm text-zinc-500">{field.help_text}</p> : null}
          </div>
          <Button variant="outline" size="sm" onClick={addItem}>
            Add
          </Button>
        </div>

        {items.length === 0 ? (
          <EmptyMiniState text={`No ${field.label.toLowerCase()} configured.`} />
        ) : (
          items.map((item, index) => (
            <Card key={`${field.key}-${index}`} className="rounded-2xl border border-zinc-200">
              <SectionActions
                title={(item.name as string | undefined) || `${field.label} ${index + 1}`}
                onRemove={() => onChange(items.filter((_, itemIndex) => itemIndex !== index))}
              />
              <div className="grid gap-3 md:grid-cols-2">
                {nestedFields.map((nestedField) => (
                  <SchemaFieldControl
                    key={`${field.key}-${index}-${nestedField.key}`}
                    field={nestedField}
                    value={item[nestedField.key]}
                    providerOptions={providerOptions}
                    onChange={(nextValue) => {
                      const nextItems = [...items];
                      nextItems[index] = { ...item, [nestedField.key]: nextValue };
                      onChange(nextItems);
                    }}
                  />
                ))}
              </div>
            </Card>
          ))
        )}
      </div>
    );
  }

  if (field.type === "textarea") {
    return (
      <label className="block">
        <Label>{field.label}</Label>
        <Textarea
          rows={4}
          value={typeof value === "string" ? value : ""}
          placeholder={field.placeholder ?? undefined}
          onChange={(event) => onChange(event.target.value)}
        />
      </label>
    );
  }

  if (field.type === "boolean") {
    const enabled = Boolean(value);
    return (
      <label className="block">
        <Label>{field.label}</Label>
        <button
          type="button"
          onClick={() => onChange(!enabled)}
          className={`inline-flex h-10 items-center rounded-md border px-3 text-sm transition ${
            enabled
              ? "border-zinc-900 bg-zinc-900 text-white"
              : "border-zinc-200 bg-white text-zinc-700 hover:border-zinc-300"
          }`}
        >
          {enabled ? "Yes" : "No"}
        </button>
      </label>
    );
  }

  if (field.type === "select") {
    const options = field.option_source === "providers" ? providerOptions : [];
    return (
      <label className="block">
        <Label>{field.label}</Label>
        <select
          value={typeof value === "string" ? value : ""}
          onChange={(event) => onChange(event.target.value)}
          className="h-10 w-full rounded-md border border-zinc-200 bg-white px-3 text-sm text-zinc-900 outline-none transition focus:border-zinc-300"
        >
          <option value="">Select...</option>
          {options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </label>
    );
  }

  return (
    <label className="block">
      <Label>{field.label}</Label>
      <Input
        value={typeof value === "string" ? value : ""}
        placeholder={field.placeholder ?? undefined}
        onChange={(event) => onChange(event.target.value)}
      />
    </label>
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
    <div className="rounded-2xl border border-dashed border-zinc-200 bg-zinc-50/80 px-4 py-6 text-sm text-zinc-500">
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
        <div className="min-w-0">
          <div className="truncate text-base font-semibold text-zinc-950">{title}</div>
        </div>
      ) : (
        <div />
      )}
      <Button variant="outline" onClick={onRemove} className="gap-2 bg-white text-zinc-700">
        <CircleOff className="h-4 w-4" />
        Remove
      </Button>
    </div>
  );
}

function Label({ children }: React.PropsWithChildren) {
  return (
    <div className="mb-1.5 text-[11px] font-medium uppercase tracking-[0.14em] text-zinc-500">
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
    <label className="block">
      <Label>{label}</Label>
      <Input value={value} onChange={(event) => onChange(event.target.value)} />
    </label>
  );
}

function stripRuleGraph(config: GatewayConfig): GatewayConfig {
  const { rule_graph: _ruleGraph, ...rest } = config;
  return rest;
}
