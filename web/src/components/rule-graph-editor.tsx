import { memo, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  ArrowRightLeft,
  CircleHelp,
  CopyPlus,
  Database,
  FileCode2,
  Filter,
  GitBranch,
  Grip,
  Hand,
  FileText,
  ListTree,
  Minus,
  Plus,
  Puzzle,
  Route,
  DatabaseZap,
  ShieldPlus,
  Split,
  Trash2,
  WandSparkles,
  Shield,
  } from "lucide-react";
import ReactFlow, {
  applyEdgeChanges,
  applyNodeChanges,
  BaseEdge,
  Background,
  BackgroundVariant,
  ConnectionMode,
  Controls,
  EdgeLabelRenderer,
  Handle,
  MarkerType,
  MiniMap,
  Panel,
  Position,
  getBezierPath,
  type Edge,
  type EdgeChange,
  type EdgeProps,
  type Node,
  type NodeChange,
  type NodeProps,
  type OnConnect,
  type OnEdgesChange,
  type OnNodesChange,
  type ReactFlowInstance,
} from "reactflow";
import "reactflow/dist/style.css";
import {
  emptyConfig,
  type GatewayConfig,
  type RuleGraphConfig,
  type RuleGraphNode,
  type RuleGraphNodeType,
  type WasmCapability,
  type WasmPluginConfigField,
  type WasmPluginIcon,
  type WasmPluginManifestSummary,
  type WasmPluginTone,
} from "@/lib/types";

type Props = {
  config: GatewayConfig;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
  pluginManifests: WasmPluginManifestSummary[];
};

type ValidationResult = {
  globalIssues: string[];
  nodeIssues: Record<string, string[]>;
  unreachableNodeIds: Set<string>;
};

type SelectOption = {
  value: string;
  label: string;
  providerId?: string;
};

type RuleCanvasNodeData = {
  node: RuleGraphNode;
  nodeType: RuleGraphNodeType;
  issueCount: number;
  unreachable: boolean;
  validationIssues: string[];
  pluginManifest?: WasmPluginManifestSummary | null;
  pluginManifestOptions: SelectOption[];
  providerOptions: SelectOption[];
  modelOptions: SelectOption[];
  templateSuggestions: string[];
  onUpdateNode: (nextNode: RuleGraphNode) => void;
  onOpenWasmConfig: () => void;
  onOpenCodeRunnerConfig: () => void;
  onDeleteNode: () => void;
};

type RuleCanvasEdgeData = {
  label?: string;
  stroke: string;
};

type NodeTone = {
  cardBorder: string;
  cardBg: string;
  chipBg: string;
  chipText: string;
  icon: string;
  libraryButton: string;
  minimap: string;
  handle: string;
  edge: string;
};

type NodeLibraryItem = {
  type: RuleGraphNodeType;
  label: string;
  shortLabel?: string;
  pluginId?: string;
  pluginManifest?: WasmPluginManifestSummary;
};

const BASE_NODE_LIBRARY: NodeLibraryItem[] = [
  { type: "wasm_plugin", label: "Condition", shortLabel: "If", pluginId: "condition-evaluator" },
  { type: "code_runner", label: "Code Runner", shortLabel: "Code" },
  { type: "wasm_plugin", label: "Select Model", shortLabel: "Model", pluginId: "select-model" },
  { type: "wasm_plugin", label: "Route Provider", shortLabel: "Route", pluginId: "route-provider" },
  { type: "wasm_plugin", label: "Rewrite Path", shortLabel: "Path", pluginId: "rewrite-path" },
  { type: "wasm_plugin", label: "Set Header", shortLabel: "Set", pluginId: "set-header" },
  { type: "wasm_plugin", label: "Log", shortLabel: "Log", pluginId: "log-step" },
  { type: "match", label: "Match" },
  { type: "note", label: "Note" },
  { type: "end", label: "End" },
];

const CONDITION_FIELDS = ["ctx.path", "ctx.method", "ctx.header.x-target", "ctx.provider.id", "ctx.model.id"];
const CONDITION_OPERATORS = ["==", "!=", "startsWith", "contains"];
const ROUTER_SOURCES = ["ctx.path", "ctx.method", "ctx.header.x-target", "ctx.provider.id", "ctx.model.id", "ctx.route_hint", "ctx.intent"];
const TEMPLATE_SUGGESTIONS_BASE = [
  "${ctx.path}",
  "${ctx.method}",
  "${ctx.provider.id}",
  "${ctx.provider.name}",
  "${ctx.model.id}",
  "${ctx.route.id}",
  "${ctx.header.authorization}",
  "${ctx.header.x-target}",
];
const WASM_CAPABILITY_OPTIONS: Array<{ value: WasmCapability; label: string }> = [
  { value: "log", label: "Log" },
  { value: "fs", label: "FS" },
  { value: "network", label: "Network" },
];
const RESERVED_MATCH_PLUGIN_IDS = new Set([
  "match-evaluator",
  "condition-evaluator",
  "workflow-selection",
  "js-code-runner",
  "select-model",
  "route-provider",
  "rewrite-path",
  "set-header",
  "log-step",
]);

export function RuleGraphEditor({ config, setConfig, pluginManifests }: Props) {
  const graph = config.rule_graph ?? emptyConfig().rule_graph!;
  const graphRef = useRef(graph);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(graph.start_node_id);
  const [selectedEdgeId, setSelectedEdgeId] = useState<string | null>(null);
  const [wasmConfigNodeId, setWasmConfigNodeId] = useState<string | null>(null);
  const [codeRunnerConfigNodeId, setCodeRunnerConfigNodeId] = useState<string | null>(null);
  const [flowInstance, setFlowInstance] = useState<ReactFlowInstance | null>(null);
  const validation = useMemo(() => validateGraph(graph, config, pluginManifests), [graph, config, pluginManifests]);
  const providerOptions = useMemo(
    () => config.providers.map((provider) => ({ value: provider.id, label: provider.id })),
    [config.providers],
  );
  const modelOptions = useMemo(
    () =>
      config.models.map((model) => ({
        value: model.id,
        label: model.id,
        providerId: model.provider_id,
      })),
    [config.models],
  );
  const pluginManifestMap = useMemo(
    () => new Map(pluginManifests.map((plugin) => [plugin.id, plugin])),
    [pluginManifests],
  );
  const pluginManifestOptions = useMemo(
    () =>
      pluginManifests.map((plugin) => ({
        value: plugin.id,
        label: `${plugin.name || plugin.id} · ${plugin.version}`,
      })),
    [pluginManifests],
  );
  const sortedPluginManifests = useMemo(
    () =>
      [...pluginManifests].sort((left, right) => {
        const leftOrder = left.ui.order ?? Number.MAX_SAFE_INTEGER;
        const rightOrder = right.ui.order ?? Number.MAX_SAFE_INTEGER;
        if (leftOrder !== rightOrder) {
          return leftOrder - rightOrder;
        }
        return (left.name || left.id).localeCompare(right.name || right.id, "zh-Hans-CN");
      }),
    [pluginManifests],
  );
  const visibleWasmPluginManifests = useMemo(
    () => sortedPluginManifests.filter((plugin) => !RESERVED_MATCH_PLUGIN_IDS.has(plugin.id)),
    [sortedPluginManifests],
  );
  const nodeLibrary = useMemo<NodeLibraryItem[]>(
    () => [
      ...BASE_NODE_LIBRARY.map((item) => ({
        ...item,
        pluginManifest: item.pluginId ? (pluginManifestMap.get(item.pluginId) ?? item.pluginManifest) : item.pluginManifest,
      })),
      ...visibleWasmPluginManifests.map((plugin) => ({
        type: "wasm_plugin" as const,
        label: plugin.name || plugin.id,
        shortLabel: shortLabelForPlugin(plugin.name || plugin.id),
        pluginId: plugin.id,
        pluginManifest: plugin,
      })),
    ],
    [pluginManifestMap, visibleWasmPluginManifests],
  );
  const templateSuggestions = useMemo(() => {
    const customCtxKeys = graph.nodes
      .map((node) => node.set_context?.key?.trim() ?? "")
      .filter((key) => key.length > 0)
      .map((key) => `\${ctx.${key}}`);
    return Array.from(new Set([...TEMPLATE_SUGGESTIONS_BASE, ...customCtxKeys]));
  }, [graph.nodes]);

  const validationIssueCount =
    validation.globalIssues.length +
    Object.values(validation.nodeIssues).reduce((count, issues) => count + issues.length, 0);
  const validationBadgeTone =
    validationIssueCount > 0
      ? "border-rose-200 bg-rose-50 text-rose-700 shadow-[0_10px_30px_rgba(244,63,94,0.14)]"
      : "border-emerald-200 bg-emerald-50 text-emerald-700";
  const validationBadgeText =
    validationIssueCount > 0
      ? `${validationIssueCount} issue${validationIssueCount === 1 ? "" : "s"}`
      : "Graph valid";
  const validationBadgeTitle =
    validationIssueCount > 0
      ? [
          ...validation.globalIssues,
          ...Object.entries(validation.nodeIssues).flatMap(([nodeId, issues]) =>
            issues.map((issue) => `${nodeId}: ${issue}`),
          ),
        ].join("\n")
      : "Graph validation passed.";
  const modalWasmNode = useMemo(
    () =>
      graph.nodes.find(
        (node) =>
          node.id === wasmConfigNodeId &&
          (node.type === "wasm_plugin" || node.type === "match"),
      ) ?? null,
    [graph.nodes, wasmConfigNodeId],
  );
  const modalWasmManifest = useMemo(
    () =>
      modalWasmNode && getWasmRuntimeConfig(modalWasmNode)?.plugin_id
        ? (pluginManifestMap.get(getWasmRuntimeConfig(modalWasmNode)!.plugin_id) ?? null)
        : null,
    [modalWasmNode, pluginManifestMap],
  );
  const modalCodeRunnerNode = useMemo(
    () =>
      graph.nodes.find(
        (node) => node.id === codeRunnerConfigNodeId && node.type === "code_runner",
      ) ?? null,
    [codeRunnerConfigNodeId, graph.nodes],
  );

  const flowNodes = useMemo<Array<Node<RuleCanvasNodeData>>>(
    () =>
      graph.nodes.map((node) => ({
        id: node.id,
        type: "ruleNode",
        position: node.position,
        data: {
          node,
          nodeType: node.type,
          issueCount: (validation.nodeIssues[node.id] ?? []).length,
          unreachable: validation.unreachableNodeIds.has(node.id),
          validationIssues: validation.nodeIssues[node.id] ?? [],
          pluginManifest: getWasmRuntimeConfig(node)?.plugin_id
            ? (pluginManifestMap.get(getWasmRuntimeConfig(node)!.plugin_id) ?? null)
            : null,
          pluginManifestOptions,
          providerOptions,
          modelOptions,
          templateSuggestions,
          onUpdateNode: (nextNode) => {
            updateGraph(setConfig, replaceNode(graph, node.id, nextNode));
          },
          onOpenWasmConfig: () => {
            setSelectedNodeId(node.id);
            setWasmConfigNodeId(node.id);
          },
          onOpenCodeRunnerConfig: () => {
            setSelectedNodeId(node.id);
            setCodeRunnerConfigNodeId(node.id);
          },
          onDeleteNode: () => {
            const nextGraph = removeNodeFromGraph(graph, node.id);
            updateGraph(setConfig, nextGraph);
            setSelectedNodeId(nextGraph.start_node_id);
          },
        },
        selected: node.id === selectedNodeId,
        draggable: true,
      })),
    [
      graph,
      modelOptions,
      pluginManifestMap,
      pluginManifestOptions,
      providerOptions,
      selectedNodeId,
      setConfig,
      templateSuggestions,
      validation.nodeIssues,
      validation.unreachableNodeIds,
    ],
  );

  const [canvasNodes, setCanvasNodes] = useState<Array<Node<RuleCanvasNodeData>>>(flowNodes);
  const canvasNodesRef = useRef(canvasNodes);

  useEffect(() => {
    setCanvasNodes(flowNodes);
  }, [flowNodes]);

  useEffect(() => {
    canvasNodesRef.current = canvasNodes;
  }, [canvasNodes]);

  useEffect(() => {
    graphRef.current = graph;
  }, [graph]);

  const flowEdges = useMemo<Array<Edge<RuleCanvasEdgeData>>>(
    () =>
      graph.edges.map((edge) => ({
        id: edge.id,
        source: edge.source,
        target: edge.target,
        type: "ruleEdge",
        sourceHandle: edge.source_handle ?? undefined,
        selected: edge.id === selectedEdgeId,
        data: {
          label: edgeLabelForHandle(edge.source_handle ?? null),
          stroke:
            edge.source_handle === "true"
              ? "#10b981"
                : edge.source_handle === "false"
                  ? "#f43f5e"
                : toneForGraphNode(
                    graph.nodes.find((node) => node.id === edge.source) ?? null,
                    pluginManifestMap,
                  ).edge,
        },
        markerEnd: { type: MarkerType.ArrowClosed },
        className: "rule-flow-edge",
      })),
    [graph.edges, graph.nodes, selectedEdgeId],
  );

  const addNode = (item: NodeLibraryItem, position?: { x: number; y: number }) => {
    const next = createNode(item.type, graph.nodes.length, graph.nodes, item.pluginId);
    const node = {
      ...next,
      position: position ?? seedNodePosition(item.type, graph.nodes),
    };

    updateGraph(setConfig, {
      ...graph,
      nodes: [...graph.nodes, node],
    });
    setSelectedNodeId(node.id);
  };

  const syncCanvasPositionsToConfig = () => {
    const positions = new Map(
      canvasNodesRef.current.map((node) => [node.id, node.position] as const),
    );
    const currentGraph = graphRef.current;

    updateGraph(setConfig, {
      ...currentGraph,
      nodes: currentGraph.nodes.map((node) => ({
        ...node,
        position: positions.get(node.id) ?? node.position,
      })),
    });
  };

  useEffect(() => {
    const flushListener = () => {
      syncCanvasPositionsToConfig();
    };

    window.addEventListener("rule-graph:flush", flushListener);
    return () => {
      window.removeEventListener("rule-graph:flush", flushListener);
    };
  }, []);

  useEffect(() => {
    if (selectedEdgeId && !graph.edges.some((edge) => edge.id === selectedEdgeId)) {
      setSelectedEdgeId(null);
    }
  }, [graph.edges, selectedEdgeId]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!selectedEdgeId || (event.key !== "Delete" && event.key !== "Backspace")) {
        return;
      }

      const target = event.target as HTMLElement | null;
      const tagName = target?.tagName?.toLowerCase();
      const isEditable =
        target?.isContentEditable ||
        tagName === "input" ||
        tagName === "textarea" ||
        tagName === "select";
      if (isEditable) {
        return;
      }

      event.preventDefault();
      updateGraph(setConfig, syncRouterTargetsWithEdges({
        ...graphRef.current,
        edges: graphRef.current.edges.filter((edge) => edge.id !== selectedEdgeId),
      }));
      setSelectedEdgeId(null);
    };

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [selectedEdgeId, setConfig]);

  const onNodesChange: OnNodesChange = (changes: NodeChange[]) => {
    setCanvasNodes((current) => applyNodeChanges(changes, current));

    const removals = changes
      .filter((change): change is Extract<NodeChange, { type: "remove" }> => change.type === "remove")
      .map((change) => change.id);

    if (removals.length > 0) {
      let nextGraph = graph;
      for (const nodeId of removals) {
        nextGraph = removeNodeFromGraph(nextGraph, nodeId);
      }
      updateGraph(setConfig, nextGraph);
      if (selectedNodeId && removals.includes(selectedNodeId)) {
        setSelectedNodeId(nextGraph.start_node_id);
      }
    }

    const selected = changes.find(
      (change): change is Extract<NodeChange, { type: "select" }> =>
        change.type === "select" && change.selected,
    );
    if (selected) {
      setSelectedNodeId(selected.id);
      setSelectedEdgeId(null);
    }
  };

  const onEdgesChange: OnEdgesChange = (changes: EdgeChange[]) => {
    const nextEdges = applyEdgeChanges(
      changes,
      graph.edges.map((edge) => ({
        id: edge.id,
        source: edge.source,
        target: edge.target,
        sourceHandle: edge.source_handle ?? undefined,
      })),
    );

    updateGraph(setConfig, syncRouterTargetsWithEdges({
      ...graph,
      edges: nextEdges.map((edge) => ({
        id: edge.id,
        source: edge.source,
        target: edge.target,
        source_handle: edge.sourceHandle ?? null,
      })),
    }));
  };

  const onConnect: OnConnect = (connection) => {
    if (!connection.source || !connection.target || connection.source === connection.target) {
      return;
    }

    const sourceNode = graph.nodes.find((node) => node.id === connection.source);
    const targetNode = graph.nodes.find((node) => node.id === connection.target);
    if (!sourceNode || !targetNode || sourceNode.type === "note" || targetNode.type === "note") {
      return;
    }

    const nextGraph =
      sourceNode.type === "router" || sourceNode.type === "match"
        ? syncRouterTargetsWithEdges(
            setEdgeTarget(graph, connection.source, connection.sourceHandle ?? null, connection.target),
          )
        : setEdgeTarget(
            graph,
            connection.source,
            sourceNode.type === "condition" || sourceNode.type === "wasm_plugin"
              ? connection.sourceHandle ?? null
              : null,
            connection.target,
          );

    updateGraph(setConfig, nextGraph);
    setSelectedEdgeId(null);
  };

  return (
    <>
      <div className="min-w-0">
        <div className="rule-graph-canvas h-[calc(100dvh-6.5rem)] min-h-[680px] rounded-[24px] border border-zinc-200 bg-zinc-50 shadow-sm">
          <ReactFlow
            nodes={canvasNodes}
            edges={flowEdges}
            nodeTypes={nodeTypes}
            edgeTypes={edgeTypes}
            onInit={setFlowInstance}
            fitView
            minZoom={0.3}
            maxZoom={1.4}
            connectionMode={ConnectionMode.Loose}
            onDragOver={(event) => {
              event.preventDefault();
              event.dataTransfer.dropEffect = "move";
            }}
            onDrop={(event) => {
              event.preventDefault();
              const serializedItem = event.dataTransfer.getData("application/rule-node-template");
              if (!serializedItem || !flowInstance) return;
              const item = parseNodeLibraryItem(serializedItem);
              if (!item) return;
              addNode(item, flowInstance.screenToFlowPosition({ x: event.clientX, y: event.clientY }));
            }}
            onNodeClick={(_, node) => setSelectedNodeId(node.id)}
            onEdgeClick={(_, edge) => {
              setSelectedNodeId(null);
              setSelectedEdgeId(edge.id);
            }}
            onPaneClick={() => {
              setSelectedNodeId(null);
              setSelectedEdgeId(null);
            }}
            onNodesChange={onNodesChange}
            onNodeDragStop={(_, node) => {
              canvasNodesRef.current = canvasNodesRef.current.map((item) =>
                item.id === node.id ? { ...item, position: node.position } : item,
              );
              updateGraph(setConfig, {
                ...graph,
                nodes: graph.nodes.map((item) =>
                  item.id === node.id
                    ? {
                        ...item,
                        position: node.position,
                      }
                    : item,
                ),
              });
            }}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            defaultEdgeOptions={{ markerEnd: { type: MarkerType.ArrowClosed } }}
            proOptions={{ hideAttribution: true }}
          >
          <MiniMap
            pannable
            zoomable
            nodeColor={(node) => {
              const data = node.data as RuleCanvasNodeData;
              if (data.unreachable) return "#f43f5e";
              if (data.issueCount > 0) return "#f59e0b";
              return toneForWasmAwareType(data.nodeType, data.pluginManifest).minimap;
            }}
            maskColor="rgba(255,255,255,0.72)"
          />
          <Controls showInteractive={false} />
          <Background
            id="rule-grid-fine"
            gap={24}
            color="rgba(15, 23, 42, 0.06)"
            variant={BackgroundVariant.Lines}
          />
          <Background
            id="rule-grid-major"
            gap={120}
            color="rgba(15, 23, 42, 0.12)"
            variant={BackgroundVariant.Lines}
          />

          <Panel position="top-left" className="!m-4">
            <div className="w-[76px] rounded-2xl border border-zinc-200 bg-white p-2 shadow-sm">
              <div className="mb-2 px-1 text-center text-[10px] font-medium uppercase tracking-[0.14em] text-zinc-500">
                Library
              </div>
              <div className="grid grid-cols-1 gap-2">
                {nodeLibrary.map((item) => (
                  <button
                    key={`${item.type}:${item.pluginId ?? item.label}`}
                    type="button"
                    draggable
                    title={item.label}
                    onClick={() => addNode(item)}
                    onDragStart={(event) => {
                      event.dataTransfer.setData(
                        "application/rule-node-template",
                        JSON.stringify(item),
                      );
                      event.dataTransfer.effectAllowed = "move";
                    }}
                    className={[
                      "group relative flex h-12 w-full items-center justify-center rounded-xl border bg-white transition",
                      toneForLibraryItem(item).libraryButton,
                    ].join(" ")}
                  >
                    <div className="flex flex-col items-center gap-1">
                      {iconForLibraryItem(item)}
                      <span className="font-mono text-[8px] uppercase tracking-[0.12em] text-zinc-500">
                        {item.shortLabel ?? shortLabelForType(item.type)}
                      </span>
                    </div>
                    <span className="pointer-events-none absolute left-[calc(100%+10px)] top-1/2 z-20 hidden -translate-y-1/2 whitespace-nowrap rounded-lg border border-zinc-200 bg-white px-2 py-1 text-[11px] font-medium text-zinc-700 opacity-0 shadow-[0_10px_30px_rgba(15,23,42,0.12)] transition group-hover:opacity-100 xl:block">
                      {item.label}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          </Panel>

          <Panel position="top-right" className="!m-4">
            <div title={validationBadgeTitle}>
              <span
                className={[
                  "inline-flex items-center rounded-full border px-2.5 py-1 text-xs font-medium shadow-sm",
                  validationBadgeTone,
                ].join(" ")}
              >
                {validationBadgeText}
              </span>
            </div>
          </Panel>

          <Panel position="bottom-left" className="!m-4">
            <div className="rounded-lg bg-white/90 px-2.5 py-1.5 text-[11px] text-zinc-500 shadow-sm">
              Drag or click to add
            </div>
          </Panel>
          </ReactFlow>
        </div>
      </div>
      <WasmConfigModal
          node={modalWasmNode}
          pluginManifest={modalWasmManifest}
          pluginManifestOptions={pluginManifestOptions}
          onClose={() => setWasmConfigNodeId(null)}
          onUpdateNode={(nextNode) => {
            if (!modalWasmNode || !isWasmNodeType(modalWasmNode.type)) {
              return;
            }
            updateGraph(setConfig, replaceNode(graph, modalWasmNode.id, nextNode));
          }}
        />
      <CodeRunnerConfigModal
        node={modalCodeRunnerNode}
        onClose={() => setCodeRunnerConfigNodeId(null)}
        onUpdateNode={(nextNode) => {
          if (!modalCodeRunnerNode || modalCodeRunnerNode.type !== "code_runner") {
            return;
          }
          updateGraph(setConfig, replaceNode(graph, modalCodeRunnerNode.id, nextNode));
        }}
      />
    </>
  );
}

const RuleCanvasNode = memo(function RuleCanvasNode({ data, selected }: NodeProps<RuleCanvasNodeData>) {
  const [draft, setDraft] = useState<RuleGraphNode>(data.node);
  const [isEditingNote, setIsEditingNote] = useState(false);
  const noteEditorRef = useRef<HTMLTextAreaElement | null>(null);
  const routerBranches = draft.router?.rules ?? [];
  const routerHandleCount = routerBranches.length + 1;
  const codeRunner = draft.code_runner ?? null;
  const wasmRuntimeConfig = getWasmRuntimeConfig(draft);
  const wasmSchemaFields =
    draft.type === "wasm_plugin" ? (data.pluginManifest?.config_schema?.fields ?? []) : [];
  const pluginOutputPorts =
    data.nodeType === "wasm_plugin"
      ? data.pluginManifest?.supported_output_ports?.length
        ? Array.from(new Set(["default", ...data.pluginManifest.supported_output_ports]))
        : ["default"]
      : [];
  const matchBranches = draft.match?.branches ?? [];
  const matchHandleCount = matchBranches.length + 1;

  useEffect(() => {
    setDraft(data.node);
    setIsEditingNote(false);
  }, [data.node]);

  useEffect(() => {
    if (isEditingNote) {
      noteEditorRef.current?.focus();
      noteEditorRef.current?.select();
    }
  }, [isEditingNote]);

  const icon = iconForWasmAwareType(data.nodeType, data.pluginManifest);
  const tone = toneForWasmAwareType(data.nodeType, data.pluginManifest);

  const borderTone = data.unreachable
    ? "border-rose-300"
    : data.issueCount > 0
      ? "border-amber-300"
      : tone.cardBorder;
  const bgTone = tone.cardBg;
  const isNoteNode = data.nodeType === "note";

  const commitNode = (nextNode: RuleGraphNode) => {
    setDraft(nextNode);
    data.onUpdateNode(nextNode);
  };

  return (
    <>
      {data.nodeType === "note" ? null : (
        <Handle
          type="target"
          position={Position.Left}
          className="!h-3 !w-3 !border-2 !border-white"
          style={{ backgroundColor: tone.handle }}
        />
      )}
      {data.nodeType === "condition" ? (
        <>
          <Handle
            id="true"
            type="source"
            position={Position.Right}
            style={{ top: "34%", backgroundColor: "#10b981" }}
            className="!h-3 !w-3 !border-2 !border-white"
          />
          <Handle
            id="false"
            type="source"
            position={Position.Right}
            style={{ top: "68%", backgroundColor: "#f43f5e" }}
            className="!h-3 !w-3 !border-2 !border-white"
          />
        </>
      ) : data.nodeType === "router" ? (
        <>
          {routerBranches.map((rule, index) => (
            <Handle
              key={rule.id}
              id={`router:${rule.id}`}
              type="source"
              position={Position.Right}
              style={{
                top: `${((index + 1) / (routerHandleCount + 1)) * 100}%`,
                backgroundColor: "#4f46e5",
              }}
              className="!h-3 !w-3 !border-2 !border-white"
            />
          ))}
          <Handle
            id="router:fallback"
            type="source"
            position={Position.Right}
            style={{
              top: `${(routerHandleCount / (routerHandleCount + 1)) * 100}%`,
              backgroundColor: "#475569",
            }}
            className="!h-3 !w-3 !border-2 !border-white"
          />
        </>
      ) : data.nodeType === "wasm_plugin" ? (
        <>
          {pluginOutputPorts.map((port, index) => (
            <Handle
              key={port}
              id={port}
              type="source"
              position={Position.Right}
              title={port === "default" ? "Default output" : `Output: ${port}`}
              aria-label={port === "default" ? "Default output" : `Output ${port}`}
              style={{
                top: `${((index + 1) / (pluginOutputPorts.length + 1)) * 100}%`,
                backgroundColor: port === "default" ? "#0f766e" : "#0ea5e9",
              }}
              className="!h-3 !w-3 !border-2 !border-white"
            />
          ))}
        </>
      ) : data.nodeType === "match" ? (
        <>
          {matchBranches.map((branch, index) => (
            <Handle
              key={branch.id}
              id={`match:${branch.id}`}
              type="source"
              position={Position.Right}
              title={`Branch: ${branch.id}`}
              aria-label={`Branch ${branch.id}`}
              style={{
                top: `${((index + 1) / (matchHandleCount + 1)) * 100}%`,
                backgroundColor: "#0284c7",
              }}
              className="!h-3 !w-3 !border-2 !border-white"
            />
          ))}
          <Handle
            id="match:fallback"
            type="source"
            position={Position.Right}
            title="Fallback"
            aria-label="Fallback"
            style={{
              top: `${(matchHandleCount / (matchHandleCount + 1)) * 100}%`,
              backgroundColor: "#475569",
            }}
            className="!h-3 !w-3 !border-2 !border-white"
          />
        </>
      ) : data.nodeType !== "end" && data.nodeType !== "note" ? (
        <Handle
          type="source"
          position={Position.Right}
          className="!h-3 !w-3 !border-2 !border-white"
          style={{ backgroundColor: tone.handle }}
        />
      ) : null}

      <div
        className={[
          "min-w-[250px] max-w-[300px] rounded-2xl border px-4 py-3 shadow-sm transition",
          isNoteNode
            ? "min-h-[160px] border-dashed shadow-sm"
            : "",
          borderTone,
          bgTone,
          selected ? "ring-1 ring-zinc-300 shadow-md" : "",
        ].join(" ")}
        onDoubleClick={(event) => {
          if (draft.type === "note") {
            event.stopPropagation();
            setIsEditingNote(true);
          }
        }}
      >
        {isNoteNode ? (
          <>
            <div className="pointer-events-none absolute right-0 top-0 h-10 w-10 rounded-tr-2xl bg-amber-100 [clip-path:polygon(100%_0,0_0,100%_100%)]" />
          </>
        ) : null}
        <div className="flex items-start justify-between gap-3 border-b border-zinc-100 pb-3">
          <div className="min-w-0">
            <div
              className={[
                "inline-flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-[11px] font-medium",
                isNoteNode ? "bg-amber-100 text-amber-900" : "",
                tone.chipBg,
                tone.chipText,
              ].join(" ")}
            >
              <span className={tone.icon}>{icon}</span>
              <span className="truncate">{labelForNode(draft, data.pluginManifest)}</span>
            </div>
          </div>
          <div className="flex items-center gap-1 rounded-lg border border-zinc-200 bg-zinc-50 p-1">
            {data.nodeType === "code_runner" ? (
              <button
                type="button"
                onClick={(event) => {
                  event.stopPropagation();
                  data.onOpenCodeRunnerConfig();
                }}
                className="nodrag nopan inline-flex h-7 w-7 items-center justify-center rounded-md bg-white text-zinc-500 shadow-sm transition hover:text-zinc-900"
                aria-label="Configure code runner"
                title="Configure code runner"
              >
                <FileCode2 className="h-3.5 w-3.5" />
              </button>
            ) : data.nodeType === "wasm_plugin" || data.nodeType === "match" ? (
              <>
                <button
                  type="button"
                  onClick={(event) => {
                    event.stopPropagation();
                    data.onOpenWasmConfig();
                  }}
                  className="nodrag nopan inline-flex h-7 w-7 items-center justify-center rounded-md bg-white text-zinc-500 shadow-sm transition hover:text-zinc-900"
                  aria-label="Configure plugin permissions"
                  title="Configure permissions"
                >
                  <Puzzle className="h-3.5 w-3.5" />
                </button>
                <div className="group relative">
                  <button
                    type="button"
                    tabIndex={-1}
                    onClick={(event) => event.preventDefault()}
                    className="nodrag nopan inline-flex h-7 w-7 items-center justify-center rounded-md bg-white text-zinc-500 shadow-sm transition hover:text-zinc-700"
                    aria-label="Show plugin details"
                    title="Plugin details"
                  >
                    <CircleHelp className="h-3.5 w-3.5" />
                  </button>
                  <div className="pointer-events-none absolute right-0 top-[calc(100%+10px)] z-30 hidden w-64 rounded-xl border border-zinc-200 bg-white p-3 text-left shadow-lg group-hover:block">
                    <div className="text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-400">
                      Plugin Details
                    </div>
                      <div className="mt-2 text-sm font-semibold text-zinc-950">
                      {data.pluginManifest?.name ?? getWasmRuntimeConfig(draft)?.plugin_id ?? "Unknown plugin"}
                      </div>
                    <p className="mt-2 text-[12px] leading-5 text-zinc-600">
                      {data.pluginManifest?.description || "Custom wasm plugin without a registry description."}
                    </p>
                    <div className="mt-3 text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-400">
                      Outputs
                    </div>
                    <div className="mt-1 flex flex-wrap gap-1.5">
                      {pluginOutputPorts.map((port) => (
                        <span
                          key={port}
                          className="inline-flex rounded-full border border-zinc-200 bg-zinc-50 px-2 py-0.5 text-[10px] font-medium text-zinc-700"
                        >
                          {port}
                        </span>
                      ))}
                    </div>
                    <div className="mt-3 text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-400">
                      Uses
                    </div>
                    <div className="mt-1 flex flex-wrap gap-1.5">
                      {getWasmRuntimeConfig(draft)?.granted_capabilities.length ? (
                        getWasmRuntimeConfig(draft)!.granted_capabilities.map((capability) => (
                          <span
                            key={capability}
                            className="inline-flex rounded-full border border-zinc-200 bg-zinc-50 px-2 py-0.5 text-[10px] font-medium text-zinc-700"
                          >
                            {capability}
                          </span>
                        ))
                      ) : (
                        <span className="text-[11px] text-zinc-500">No extra access</span>
                      )}
                    </div>
                    <div className="mt-3 text-[11px] leading-5 text-zinc-500">
                      Hover output handles for port names. Use the plugin icon to jump to permissions.
                    </div>
                  </div>
                </div>
              </>
            ) : (
              <div className={`inline-flex h-7 w-7 items-center justify-center rounded-md bg-white shadow-sm ${tone.icon}`}>{icon}</div>
            )}
            {selected && draft.type !== "start" ? (
              <button
                type="button"
                onClick={(event) => {
                  event.stopPropagation();
                  data.onDeleteNode();
                }}
                className="nodrag nopan inline-flex h-7 w-7 items-center justify-center rounded-md bg-white text-zinc-500 shadow-sm transition hover:text-rose-600"
              >
                <Trash2 className="h-3.5 w-3.5" />
              </button>
            ) : null}
          </div>
        </div>

        <div className="mt-3 space-y-3">
          {draft.type === "note" ? (
            isEditingNote ? (
              <textarea
                ref={noteEditorRef}
                value={draft.note_node?.text ?? ""}
                rows={5}
                onClick={(event) => event.stopPropagation()}
                onPointerDown={(event) => event.stopPropagation()}
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    note_node: { text: event.target.value },
                  }))
                }
                onBlur={(event) => {
                  commitNode({
                    ...draft,
                    note_node: { text: event.target.value },
                  });
                  setIsEditingNote(false);
                }}
                className="nodrag nopan nowheel w-full resize-none rounded-xl border border-amber-200 bg-amber-50 px-3 py-3 text-sm leading-7 text-zinc-900 outline-none transition focus:border-amber-400 focus:ring-2 focus:ring-amber-100"
              />
            ) : (
              <button
                type="button"
                onDoubleClick={(event) => {
                  event.stopPropagation();
                  setIsEditingNote(true);
                }}
                className="nodrag nopan min-h-[110px] w-full rounded-xl border border-amber-200 bg-amber-50 px-3 py-3 text-left text-sm leading-7 text-zinc-800"
              >
                {(draft.note_node?.text ?? "").trim() || "Double-click to edit"}
              </button>
            )
          ) : null}

          {draft.type === "condition" ? (
            <>
              <InlineSelect
                value={draft.condition?.mode ?? "expression"}
                options={[
                  { value: "expression", label: "Expression" },
                  { value: "builder", label: "Builder" },
                ]}
                onChange={(value) => {
                  commitNode({
                    ...draft,
                    condition: {
                      mode: value as "builder" | "expression",
                      expression: draft.condition?.expression ?? 'path.startsWith("/v1/")',
                      builder: draft.condition?.builder ?? {
                        field: "path",
                        operator: "startsWith",
                        value: "/v1/",
                      },
                    },
                  });
                }}
              />
              {draft.condition?.mode === "builder" ? (
                <div className="grid grid-cols-1 gap-2">
                  <InlineSelect
                    value={draft.condition?.builder?.field ?? "path"}
                    options={CONDITION_FIELDS.map((item) => ({ value: item, label: item }))}
                    onChange={(value) => {
                      commitNode({
                        ...draft,
                        condition: {
                          ...draft.condition!,
                          builder: {
                            field: value,
                            operator: draft.condition?.builder?.operator ?? "startsWith",
                            value: draft.condition?.builder?.value ?? "/v1/",
                          },
                        },
                      });
                    }}
                  />
                  <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-2">
                    <InlineSelect
                      value={draft.condition?.builder?.operator ?? "startsWith"}
                      options={CONDITION_OPERATORS.map((item) => ({ value: item, label: item }))}
                      onChange={(value) => {
                        commitNode({
                          ...draft,
                          condition: {
                            ...draft.condition!,
                            builder: {
                              field: draft.condition?.builder?.field ?? "path",
                              operator: value,
                              value: draft.condition?.builder?.value ?? "/v1/",
                            },
                          },
                        });
                      }}
                    />
                    <InlineInput
                      value={draft.condition?.builder?.value ?? ""}
                      placeholder="value"
                      onChange={(value) =>
                        setDraft((current) => ({
                          ...current,
                          condition: {
                            ...current.condition!,
                            builder: {
                              field: current.condition?.builder?.field ?? "path",
                              operator: current.condition?.builder?.operator ?? "startsWith",
                              value,
                            },
                          },
                        }))
                      }
                      onCommit={(value) =>
                        commitNode({
                          ...draft,
                          condition: {
                            ...draft.condition!,
                            builder: {
                              field: draft.condition?.builder?.field ?? "path",
                              operator: draft.condition?.builder?.operator ?? "startsWith",
                              value,
                            },
                          },
                        })
                      }
                    />
                  </div>
                </div>
              ) : (
                <InlineTextarea
                  value={draft.condition?.expression ?? ""}
                  placeholder='path.startsWith("/v1/")'
                  suggestions={data.templateSuggestions}
                  onChange={(value) =>
                    setDraft((current) => ({
                      ...current,
                      condition: {
                        mode: "expression",
                        expression: value,
                        builder: current.condition?.builder ?? null,
                      },
                    }))
                  }
                  onCommit={(value) =>
                    commitNode({
                      ...draft,
                      condition: {
                        mode: "expression",
                        expression: value,
                        builder: draft.condition?.builder ?? null,
                      },
                    })
                  }
                />
              )}
            </>
          ) : null}

          {draft.type === "select_model" ? (
            <div className="grid grid-cols-1 gap-2">
              <InlineSelect
                value={draft.select_model?.provider_id ?? ""}
                options={data.providerOptions}
                onChange={(value) => {
                  const selectedModel = data.modelOptions.find(
                    (option) => option.value === (draft.select_model?.model_id ?? ""),
                  );
                  const nextModelId =
                    selectedModel && selectedModel.providerId === value
                      ? selectedModel.value
                      : "";
                  commitNode({
                    ...draft,
                    select_model: {
                      provider_id: value,
                      model_id: nextModelId,
                    },
                  });
                }}
                placeholder="Select provider"
              />
              <InlineSelect
                value={draft.select_model?.model_id ?? ""}
                options={data.modelOptions.filter(
                  (option) =>
                    !draft.select_model?.provider_id ||
                    option.providerId === draft.select_model.provider_id,
                )}
                onChange={(value) => {
                  commitNode({
                    ...draft,
                    select_model: {
                      provider_id: draft.select_model?.provider_id ?? "",
                      model_id: value,
                    },
                  });
                }}
                placeholder="Select model"
              />
            </div>
          ) : null}

          {draft.type === "set_context" ? (
            <div className="grid grid-cols-1 gap-2">
              <InlineInput
                value={draft.set_context?.key ?? ""}
                placeholder="context key"
                onChange={(value) =>
                  setDraft((current) => ({
                    ...current,
                    set_context: {
                      key: value,
                      value_template: current.set_context?.value_template ?? "",
                    },
                  }))
                }
                onCommit={(value) =>
                  commitNode({
                    ...draft,
                    set_context: {
                      key: value,
                      value_template: draft.set_context?.value_template ?? "",
                    },
                  })
                }
              />
              <InlineInput
                value={draft.set_context?.value_template ?? ""}
                placeholder="${ctx.header.x-target}"
                suggestions={data.templateSuggestions}
                onChange={(value) =>
                  setDraft((current) => ({
                    ...current,
                    set_context: {
                      key: current.set_context?.key ?? "",
                      value_template: value,
                    },
                  }))
                }
                onCommit={(value) =>
                  commitNode({
                    ...draft,
                    set_context: {
                      key: draft.set_context?.key ?? "",
                      value_template: value,
                    },
                  })
                }
              />
            </div>
          ) : null}

          {draft.type === "router" ? (
            <div className="space-y-2">
              <div className="flex items-center justify-between px-1">
                <div className="text-[11px] font-medium text-zinc-700">Rules</div>
                <div className="text-[10px] text-zinc-500">First match wins</div>
              </div>
              {(draft.router?.rules ?? []).map((rule, ruleIndex) => (
                <div key={rule.id} className="rounded-lg bg-zinc-50 px-2.5 py-2">
                  <div className="mb-2 flex items-start gap-2">
                    <span className="inline-flex h-7 min-w-10 items-center justify-center rounded-md bg-indigo-50 px-2 text-[11px] font-medium text-indigo-700">
                      R{ruleIndex + 1}
                    </span>
                    <div className="min-w-0 flex-1 space-y-2">
                      {(rule.clauses ?? []).map((clause, clauseIndex) => (
                        <div key={`${rule.id}-${clauseIndex}`} className="grid grid-cols-1 gap-2">
                          <InlineSelect
                            value={clause.source}
                            options={ROUTER_SOURCES.map((item) => ({ value: item, label: item }))}
                            onChange={(value) =>
                              commitNode(updateRouterClause(draft, ruleIndex, clauseIndex, { source: value }))
                            }
                          />
                          <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-2">
                            <InlineSelect
                              value={clause.operator}
                              options={CONDITION_OPERATORS.map((item) => ({ value: item, label: item }))}
                              onChange={(value) =>
                                commitNode(updateRouterClause(draft, ruleIndex, clauseIndex, { operator: value }))
                              }
                            />
                            <InlineInput
                              value={clause.value}
                              placeholder="value"
                              onChange={(value) =>
                                setDraft(updateRouterClause(draft, ruleIndex, clauseIndex, { value }))
                              }
                              onCommit={(value) =>
                                commitNode(updateRouterClause(draft, ruleIndex, clauseIndex, { value }))
                              }
                            />
                          </div>
                        </div>
                      ))}
                    </div>
                    <button
                      type="button"
                      onClick={(event) => {
                        event.stopPropagation();
                        commitNode({
                          ...draft,
                          router: {
                            rules: (draft.router?.rules ?? []).filter((_, index) => index !== ruleIndex),
                            fallback_node_id: draft.router?.fallback_node_id ?? null,
                          },
                        });
                      }}
                      className="nodrag nopan inline-flex h-7 w-7 items-center justify-center rounded-md border border-zinc-200 bg-white text-zinc-500 transition hover:border-rose-200 hover:text-rose-600"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>
                  <div className="mt-2 flex items-center gap-2 border-t border-zinc-200 pt-2">
                    <span className="text-[10px] uppercase tracking-[0.14em] text-zinc-400">Target</span>
                    <span className="min-w-0 flex-1 truncate rounded-md bg-white px-2.5 py-1 text-[11px] text-zinc-700 ring-1 ring-zinc-200">
                      {rule.target_node_id || "Drag to target"}
                    </span>
                  </div>
                </div>
              ))}
              <button
                type="button"
                onClick={(event) => {
                  event.stopPropagation();
                  commitNode(addRouterRule(draft));
                }}
                className="nodrag nopan inline-flex h-9 w-full items-center justify-center rounded-lg border border-dashed border-zinc-300 bg-zinc-50 text-sm text-zinc-700 transition hover:border-zinc-400 hover:bg-white hover:text-zinc-900"
              >
                Add Branch
              </button>
              <div className="flex items-center gap-2 border-t border-zinc-100 px-1 pt-2">
                <span className="inline-flex h-7 items-center rounded-md bg-slate-100 px-2.5 text-[11px] font-medium text-slate-700">
                  Fallback
                </span>
                <span className="min-w-0 flex-1 truncate rounded-md bg-zinc-50 px-2.5 py-1 text-[11px] text-zinc-700 ring-1 ring-zinc-200">
                  {draft.router?.fallback_node_id || "Drag to target"}
                </span>
              </div>
            </div>
          ) : null}

          {draft.type === "rewrite_path" ? (
            <InlineInput
              value={draft.rewrite_path?.value ?? ""}
              placeholder="/v1/chat/completions"
              suggestions={data.templateSuggestions}
              onChange={(value) =>
                setDraft((current) => ({
                  ...current,
                  rewrite_path: { value },
                }))
              }
              onCommit={(value) =>
                commitNode({
                  ...draft,
                  rewrite_path: { value },
                })
              }
            />
          ) : null}

          {draft.type === "log" ? (
            <InlineTextarea
              value={draft.log?.message ?? ""}
              placeholder="route=${ctx.route_hint}"
              suggestions={data.templateSuggestions}
              onChange={(value) =>
                setDraft((current) => ({
                  ...current,
                  log: { message: value },
                }))
              }
              onCommit={(value) =>
                commitNode({
                  ...draft,
                  log: { message: value },
                })
              }
            />
          ) : null}

          {draft.type === "code_runner" ? (
            <div className="space-y-2">
              <div className="px-1">
                <div className="text-[11px] font-medium text-zinc-700">JavaScript Runtime</div>
                <p className="mt-1 line-clamp-4 text-[12px] leading-5 text-zinc-600">
                  {describeCodeRunner(draft)}
                </p>
                {codeRunner ? (
                  <div className="mt-3 flex flex-wrap gap-1.5">
                    <span className="inline-flex rounded-full bg-emerald-50 px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-emerald-700 ring-1 ring-emerald-200">
                      JavaScript
                    </span>
                    <span className="inline-flex rounded-full bg-zinc-50 px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-600 ring-1 ring-zinc-200">
                      {codeRunner.timeout_ms} ms
                    </span>
                    <span className="inline-flex rounded-full bg-zinc-50 px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-600 ring-1 ring-zinc-200">
                      {formatMemoryBytes(codeRunner.max_memory_bytes)}
                    </span>
                  </div>
                ) : null}
              </div>
            </div>
          ) : null}

          {draft.type === "wasm_plugin" || draft.type === "match" ? (
            <div className="space-y-2">
              {draft.type === "wasm_plugin" && wasmRuntimeConfig && wasmSchemaFields.length > 0 ? (
                <div className="space-y-3 px-1">
                  {wasmSchemaFields.map((field) => {
                    const configObject = normalizePluginConfigObject(wasmRuntimeConfig.config);
                    const value = configObject[field.key];
                    const options = selectOptionsForSchemaField(
                      field,
                      configObject,
                      data.providerOptions,
                      data.modelOptions,
                    );
                    const commitFieldValue = (nextValue: unknown) =>
                      commitNode(
                        updateWasmRuntimeConfig(draft, {
                          ...wasmRuntimeConfig,
                          config: updatePluginConfigField(wasmRuntimeConfig.config, field.key, nextValue),
                        }),
                      );

                    return (
                      <div key={field.key} className="grid grid-cols-1 gap-2">
                        <FieldLabel label={field.label} />
                        {field.type === "select" ? (
                          <InlineSelect
                            value={typeof value === "string" ? value : ""}
                            options={options}
                            onChange={(nextValue) => commitFieldValue(nextValue)}
                            placeholder={field.placeholder ?? `Select ${field.label.toLowerCase()}`}
                          />
                        ) : field.type === "textarea" ? (
                          <InlineTextarea
                            value={typeof value === "string" ? value : ""}
                            placeholder={field.placeholder ?? ""}
                            onChange={(nextValue) => commitFieldValue(nextValue)}
                            onCommit={(nextValue) => commitFieldValue(nextValue)}
                          />
                        ) : field.type === "boolean" ? (
                          <label className="inline-flex items-center gap-2 rounded-md border border-zinc-200 bg-white px-3 py-2 text-sm text-zinc-700">
                            <input
                              type="checkbox"
                              checked={Boolean(value)}
                              onChange={(event) => commitFieldValue(event.target.checked)}
                              className="h-4 w-4 rounded border-zinc-300 text-zinc-900"
                            />
                            <span>{field.placeholder ?? "Enabled"}</span>
                          </label>
                        ) : (
                          <InlineInput
                            value={typeof value === "string" ? value : ""}
                            placeholder={field.placeholder ?? ""}
                            onChange={(nextValue) => commitFieldValue(nextValue)}
                            onCommit={(nextValue) => commitFieldValue(nextValue)}
                          />
                        )}
                        {field.help_text ? (
                          <p className="text-xs leading-5 text-zinc-500">{field.help_text}</p>
                        ) : null}
                      </div>
                    );
                  })}
                </div>
              ) : null}
            </div>
          ) : null}

          {draft.type === "match" ? (
            <div className="space-y-2">
              <div className="flex items-center justify-between px-1">
                <div className="text-[11px] font-medium text-zinc-700">Branches</div>
                <div className="text-[10px] text-zinc-500">Drag right handles to connect</div>
              </div>
              {(draft.match?.branches ?? []).map((branch, branchIndex) => (
                <div key={branch.id} className="rounded-lg bg-zinc-50 px-2.5 py-2">
                  <div className="flex items-center gap-2">
                    <span className="inline-flex h-7 min-w-10 items-center justify-center rounded-md bg-sky-50 px-2 text-[11px] font-medium text-sky-700">
                      B{branchIndex + 1}
                    </span>
                    <div className="min-w-0 flex-1">
                      <InlineInput
                        value={branch.expr}
                        placeholder='expr, e.g. ctx.header.x-target == "chat"'
                        suggestions={data.templateSuggestions}
                        onChange={(value) =>
                          setDraft(updateWasmMatchBranch(draft, branchIndex, { expr: value }))
                        }
                        onCommit={(value) =>
                          commitNode(updateWasmMatchBranch(draft, branchIndex, { expr: value }))
                        }
                      />
                    </div>
                  </div>
                  <div className="mt-2 flex items-center gap-2 border-t border-zinc-200 pt-2">
                    <span className="text-[10px] uppercase tracking-[0.14em] text-zinc-400">Target</span>
                    <span className="min-w-0 flex-1 truncate rounded-md bg-white px-2.5 py-1 text-[11px] text-zinc-700 ring-1 ring-zinc-200">
                      {branch.target_node_id || "Drag to target"}
                    </span>
                    <button
                      type="button"
                      onClick={(event) => {
                        event.stopPropagation();
                        commitNode({
                          ...draft,
                          match: {
                            ...draft.match!,
                            branches: (draft.match?.branches ?? []).filter((_, index) => index !== branchIndex),
                            fallback_node_id: draft.match?.fallback_node_id ?? null,
                          },
                        });
                      }}
                      className="nodrag nopan inline-flex h-7 w-7 items-center justify-center rounded-md border border-zinc-200 bg-white text-zinc-500 transition hover:border-rose-200 hover:text-rose-600"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>
                </div>
              ))}
              <button
                type="button"
                onClick={(event) => {
                  event.stopPropagation();
                  commitNode(addWasmMatchBranch(draft));
                }}
                className="nodrag nopan inline-flex h-9 w-full items-center justify-center rounded-md border border-zinc-200 bg-white text-sm text-zinc-700 transition hover:border-zinc-300 hover:bg-zinc-50 hover:text-zinc-900"
              >
                Add Branch
              </button>
              <div className="flex items-center gap-2 border-t border-zinc-100 px-1 pt-2">
                <span className="inline-flex h-7 items-center rounded-md bg-slate-100 px-2.5 text-[11px] font-medium text-slate-700">
                  Fallback
                </span>
                <span className="min-w-0 flex-1 truncate rounded-md bg-zinc-50 px-2.5 py-1 text-[11px] text-zinc-700 ring-1 ring-zinc-200">
                  {draft.match?.fallback_node_id || "Drag to target"}
                </span>
              </div>
            </div>
          ) : null}

          {(draft.type === "set_header" || draft.type === "set_header_if_absent") && (
            <div className="grid grid-cols-1 gap-2">
              <InlineInput
                value={
                  draft.type === "set_header"
                    ? draft.set_header?.name ?? ""
                    : draft.set_header_if_absent?.name ?? ""
                }
                placeholder="Header name"
                onChange={(value) =>
                  setDraft((current) => ({
                    ...current,
                    [current.type]:
                      current.type === "set_header"
                        ? { name: value, value: current.set_header?.value ?? "" }
                        : { name: value, value: current.set_header_if_absent?.value ?? "" },
                  }))
                }
                onCommit={(value) =>
                  commitNode(
                    draft.type === "set_header"
                      ? {
                          ...draft,
                          set_header: { name: value, value: draft.set_header?.value ?? "" },
                        }
                      : {
                          ...draft,
                          set_header_if_absent: {
                            name: value,
                            value: draft.set_header_if_absent?.value ?? "",
                          },
                        },
                  )
                }
              />
              <InlineInput
                value={
                  draft.type === "set_header"
                    ? draft.set_header?.value ?? ""
                    : draft.set_header_if_absent?.value ?? ""
                }
                placeholder="Header value"
                suggestions={data.templateSuggestions}
                onChange={(value) =>
                  setDraft((current) => ({
                    ...current,
                    [current.type]:
                      current.type === "set_header"
                        ? { name: current.set_header?.name ?? "", value }
                        : { name: current.set_header_if_absent?.name ?? "", value },
                  }))
                }
                onCommit={(value) =>
                  commitNode(
                    draft.type === "set_header"
                      ? {
                          ...draft,
                          set_header: { name: draft.set_header?.name ?? "", value },
                        }
                      : {
                          ...draft,
                          set_header_if_absent: {
                            name: draft.set_header_if_absent?.name ?? "",
                            value,
                          },
                        },
                  )
                }
              />
            </div>
          )}

          {draft.type === "remove_header" ? (
            <InlineInput
              value={draft.remove_header?.name ?? ""}
              placeholder="Header name"
              onChange={(value) =>
                setDraft((current) => ({
                  ...current,
                  remove_header: { name: value },
                }))
              }
              onCommit={(value) =>
                commitNode({
                  ...draft,
                  remove_header: { name: value },
                })
              }
            />
          ) : null}

          {draft.type === "copy_header" ? (
            <div className="grid grid-cols-1 gap-2">
              <InlineInput
                value={draft.copy_header?.from ?? ""}
                placeholder="From header"
                onChange={(value) =>
                  setDraft((current) => ({
                    ...current,
                    copy_header: {
                      from: value,
                      to: current.copy_header?.to ?? "",
                    },
                  }))
                }
                onCommit={(value) =>
                  commitNode({
                    ...draft,
                    copy_header: {
                      from: value,
                      to: draft.copy_header?.to ?? "",
                    },
                  })
                }
              />
              <InlineInput
                value={draft.copy_header?.to ?? ""}
                placeholder="To header"
                onChange={(value) =>
                  setDraft((current) => ({
                    ...current,
                    copy_header: {
                      from: current.copy_header?.from ?? "",
                      to: value,
                    },
                  }))
                }
                onCommit={(value) =>
                  commitNode({
                    ...draft,
                    copy_header: {
                      from: draft.copy_header?.from ?? "",
                      to: value,
                    },
                  })
                }
              />
            </div>
          ) : null}
        </div>

        {data.validationIssues.length > 0 ? (
          <div className="mt-3 space-y-1.5">
            {data.validationIssues.map((issue) => (
              <div
                key={issue}
                className="rounded-xl border border-amber-200 bg-amber-50 px-2.5 py-2 text-[11px] text-amber-900"
              >
                {issue}
              </div>
            ))}
          </div>
        ) : data.unreachable ? (
          <div className="mt-3 rounded-xl border border-rose-200 bg-rose-50 px-2.5 py-2 text-[11px] text-rose-800">
            Unreachable from start.
          </div>
        ) : null}
      </div>
    </>
  );
});

function WasmConfigModal({
  node,
  pluginManifest,
  pluginManifestOptions,
  onClose,
  onUpdateNode,
}: {
  node: RuleGraphNode | null;
  pluginManifest: WasmPluginManifestSummary | null;
  pluginManifestOptions: SelectOption[];
  onClose: () => void;
  onUpdateNode: (nextNode: RuleGraphNode) => void;
}) {
  if (!node || !isWasmNodeType(node.type)) return null;

  const plugin = getWasmRuntimeConfig(node)!;
  const schemaFields = pluginManifest?.config_schema?.fields ?? [];
  const outputPorts = pluginManifest?.supported_output_ports?.length
    ? Array.from(new Set(["default", ...pluginManifest.supported_output_ports]))
    : ["default"];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-zinc-950/35 px-4 py-6 backdrop-blur-[2px]">
      <div
        className="absolute inset-0"
        onClick={onClose}
        aria-hidden="true"
      />
      <div className="relative z-10 flex max-h-[90vh] w-full max-w-3xl flex-col overflow-hidden rounded-[28px] border border-zinc-200 bg-white shadow-[0_30px_90px_rgba(15,23,42,0.18)]">
        <div className="flex items-start justify-between gap-4 border-b border-zinc-100 px-6 py-5">
          <div className="min-w-0">
            <div className="text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-400">Wasm Node Config</div>
            <div className="mt-2 text-xl font-semibold text-zinc-950">{pluginManifest?.name ?? plugin.plugin_id}</div>
            <div className="mt-1 text-sm leading-6 text-zinc-600">
              {describeWasmPlugin(pluginManifest, node)}
            </div>
            <div className="mt-3 flex flex-wrap gap-2 text-[10px] text-zinc-500">
              <span className="rounded-full border border-zinc-200 bg-zinc-50 px-2.5 py-1">
                {pluginManifest?.version ?? "unversioned"}
              </span>
              <span className="rounded-full border border-zinc-200 bg-zinc-50 px-2.5 py-1 font-mono">
                {plugin.plugin_id}
              </span>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="inline-flex h-9 w-9 items-center justify-center rounded-full border border-zinc-200 bg-white text-zinc-500 transition hover:border-zinc-300 hover:text-zinc-900"
            aria-label="Close modal"
          >
            <Minus className="h-4 w-4 rotate-45" />
          </button>
        </div>
        <div className="overflow-y-auto px-6 py-5">
      {!pluginManifest ? (
        <InspectorSection title="Plugin" subtitle="Resolve the missing registry entry before editing other fields.">
          <InlineSelect
            value={plugin.plugin_id}
            options={pluginManifestOptions}
            onChange={(value) =>
              onUpdateNode({
                ...updateWasmRuntimeConfig(node, {
                  ...plugin,
                  plugin_id: value,
                }),
              })
            }
            placeholder="Resolve plugin"
          />
        </InspectorSection>
      ) : null}

      <InspectorSection title="Outputs" subtitle="These are the ports this plugin can emit from the right edge.">
        <div className="flex flex-wrap gap-2">
          {outputPorts.map((port) => (
            <div
              key={port}
              className={[
                "rounded-full border px-2.5 py-1 text-[10px] font-semibold uppercase tracking-[0.16em]",
                port === "default"
                  ? "border-teal-300 bg-teal-50 text-teal-800"
                  : "border-sky-200 bg-sky-50 text-sky-700",
              ].join(" ")}
            >
              {port}
            </div>
          ))}
        </div>
      </InspectorSection>

      <InspectorSection title="Permissions" subtitle="Grant only the capabilities and scope the plugin actually needs.">
        <div className="space-y-3">
          <div className="flex flex-wrap gap-2">
            {WASM_CAPABILITY_OPTIONS.map((option) => {
              const enabled = plugin.granted_capabilities.includes(option.value);
              const declared = pluginManifest?.capabilities.includes(option.value) ?? true;
              const nextCapabilities = enabled
                ? plugin.granted_capabilities.filter((item) => item !== option.value)
                : [...plugin.granted_capabilities, option.value];

              return (
                <button
                  key={option.value}
                  type="button"
                  onClick={() =>
                    onUpdateNode({
                      ...updateWasmRuntimeConfig(node, {
                        ...plugin,
                        granted_capabilities: nextCapabilities,
                      }),
                    })
                  }
                  className={[
                    "rounded-full border px-3 py-1.5 text-xs font-semibold transition",
                    enabled
                      ? "border-zinc-900 bg-zinc-900 text-white"
                      : "border-zinc-200 bg-white text-zinc-700 hover:border-zinc-400",
                    declared ? "" : "opacity-50",
                  ].join(" ")}
                  title={declared ? option.label : "Not declared by selected plugin manifest"}
                >
                  {option.label}
                </button>
              );
            })}
          </div>

          {plugin.granted_capabilities.includes("fs") ? (
            <div className="grid grid-cols-1 gap-2">
              <FieldLabel label="Readable Directories" />
              <InlineTextarea
                value={formatListForTextarea(plugin.read_dirs ?? [])}
                placeholder={"plugins-data/common\ndata/rules"}
                onChange={(value) =>
                  onUpdateNode({
                    ...updateWasmRuntimeConfig(node, {
                      ...plugin,
                      read_dirs: parseTextareaList(value),
                    }),
                  })
                }
                onCommit={(value) =>
                  onUpdateNode({
                    ...updateWasmRuntimeConfig(node, {
                      ...plugin,
                      read_dirs: parseTextareaList(value),
                    }),
                  })
                }
              />
              <FieldLabel label="Writable Directories" />
              <InlineTextarea
                value={formatListForTextarea(plugin.write_dirs ?? [])}
                placeholder={"plugins-data/runtime"}
                onChange={(value) =>
                  onUpdateNode({
                    ...updateWasmRuntimeConfig(node, {
                      ...plugin,
                      write_dirs: parseTextareaList(value),
                    }),
                  })
                }
                onCommit={(value) =>
                  onUpdateNode({
                    ...updateWasmRuntimeConfig(node, {
                      ...plugin,
                      write_dirs: parseTextareaList(value),
                    }),
                  })
                }
              />
            </div>
          ) : null}

          {plugin.granted_capabilities.includes("network") ? (
            <div className="grid grid-cols-1 gap-2">
              <FieldLabel label="Allowed Hosts" />
              <InlineTextarea
                value={formatListForTextarea(plugin.allowed_hosts ?? [])}
                placeholder={"api.example.com:443\n127.0.0.1:8080"}
                onChange={(value) =>
                  onUpdateNode({
                    ...updateWasmRuntimeConfig(node, {
                      ...plugin,
                      allowed_hosts: parseTextareaList(value),
                    }),
                  })
                }
                onCommit={(value) =>
                  onUpdateNode({
                    ...updateWasmRuntimeConfig(node, {
                      ...plugin,
                      allowed_hosts: parseTextareaList(value),
                    }),
                  })
                }
              />
            </div>
          ) : null}
        </div>
      </InspectorSection>

      <InspectorSection title="Runtime Limits" subtitle="Guardrails for latency, compute budget, and memory.">
        <div className="grid grid-cols-1 gap-2">
          <FieldLabel label="Timeout (ms)" />
          <InlineInput
            value={String(plugin.timeout_ms ?? 20)}
            placeholder="20"
            onChange={(value) =>
              onUpdateNode({
                ...updateWasmRuntimeConfig(node, {
                  ...plugin,
                  timeout_ms: value === "" ? 0 : Number(value),
                }),
              })
            }
            onCommit={(value) =>
              onUpdateNode({
                ...updateWasmRuntimeConfig(node, {
                  ...plugin,
                  timeout_ms: value === "" ? 0 : Number(value),
                }),
              })
            }
          />
          <FieldLabel label="Fuel" />
          <InlineInput
            value={plugin.fuel != null ? String(plugin.fuel) : ""}
            placeholder="optional fuel"
            onChange={(value) =>
              onUpdateNode({
                ...updateWasmRuntimeConfig(node, {
                  ...plugin,
                  fuel: value === "" ? null : Number(value),
                }),
              })
            }
            onCommit={(value) =>
              onUpdateNode({
                ...updateWasmRuntimeConfig(node, {
                  ...plugin,
                  fuel: value === "" ? null : Number(value),
                }),
              })
            }
          />
          <FieldLabel label="Max Memory Bytes" />
          <InlineInput
            value={String(plugin.max_memory_bytes ?? 16_777_216)}
            placeholder="16777216"
            onChange={(value) =>
              onUpdateNode({
                ...updateWasmRuntimeConfig(node, {
                  ...plugin,
                  max_memory_bytes: value === "" ? 0 : Number(value),
                }),
              })
            }
            onCommit={(value) =>
              onUpdateNode({
                ...updateWasmRuntimeConfig(node, {
                  ...plugin,
                  max_memory_bytes: value === "" ? 0 : Number(value),
                }),
              })
            }
          />
        </div>
      </InspectorSection>

      {schemaFields.length === 0 ? (
        <InspectorSection title="Plugin Config" subtitle="Raw JSON passed through to the wasm plugin.">
          <InlineTextarea
            value={formatPluginConfig(plugin.config ?? {})}
            placeholder={'{\n  "prompt": "classify request intent"\n}'}
            onChange={(value) =>
              onUpdateNode({
                ...updateWasmRuntimeConfig(node, {
                  ...plugin,
                  config: parsePluginConfig(value, plugin.config ?? {}),
                }),
              })
            }
            onCommit={(value) =>
              onUpdateNode({
                ...updateWasmRuntimeConfig(node, {
                  ...plugin,
                  config: parsePluginConfig(value, plugin.config ?? {}),
                }),
              })
            }
          />
        </InspectorSection>
      ) : null}
        </div>
        <div className="flex items-center justify-end gap-3 border-t border-zinc-100 px-6 py-4">
          <button
            type="button"
            onClick={onClose}
            className="inline-flex h-10 items-center justify-center rounded-full border border-zinc-200 bg-white px-4 text-sm font-medium text-zinc-700 transition hover:border-zinc-300 hover:text-zinc-950"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

function CodeRunnerConfigModal({
  node,
  onClose,
  onUpdateNode,
}: {
  node: RuleGraphNode | null;
  onClose: () => void;
  onUpdateNode: (nextNode: RuleGraphNode) => void;
}) {
  if (!node || node.type !== "code_runner") return null;

  const codeRunner = getCodeRunnerConfig(node);
  if (!codeRunner) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-zinc-950/35 px-4 py-6 backdrop-blur-[2px]">
      <div className="absolute inset-0" onClick={onClose} aria-hidden="true" />
      <div className="relative z-10 flex max-h-[90vh] w-full max-w-3xl flex-col overflow-hidden rounded-[28px] border border-zinc-200 bg-white shadow-[0_30px_90px_rgba(15,23,42,0.18)]">
        <div className="flex items-start justify-between gap-4 border-b border-zinc-100 px-6 py-5">
          <div className="min-w-0">
            <div className="text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-400">Code Runner Config</div>
            <div className="mt-2 text-xl font-semibold text-zinc-950">JavaScript transform node</div>
            <div className="mt-1 text-sm leading-6 text-zinc-600">
              Edit the script and runtime limits for this code runner node.
            </div>
            <div className="mt-3 flex flex-wrap gap-2 text-[10px] text-zinc-500">
              <span className="rounded-full border border-zinc-200 bg-zinc-50 px-2.5 py-1 uppercase tracking-[0.16em] text-zinc-600">
                JavaScript
              </span>
              <span className="rounded-full border border-zinc-200 bg-zinc-50 px-2.5 py-1 font-mono">
                {node.id}
              </span>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="inline-flex h-9 w-9 items-center justify-center rounded-full border border-zinc-200 bg-white text-zinc-500 transition hover:border-zinc-300 hover:text-zinc-900"
            aria-label="Close modal"
          >
            <Minus className="h-4 w-4 rotate-45" />
          </button>
        </div>

        <div className="overflow-y-auto px-6 py-5">
          <InspectorSection title="Overview" subtitle="This node runs a JavaScript transform before routing continues.">
            <div className="flex flex-wrap gap-2">
              <span className="rounded-full border border-emerald-200 bg-emerald-50 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-emerald-700">
                Runtime: JavaScript
              </span>
              <span className="rounded-full border border-zinc-200 bg-zinc-50 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-600">
                Transform only
              </span>
            </div>
            <p className="mt-3 text-sm leading-6 text-zinc-600">
              The script should export <span className="font-mono text-zinc-900">run(input)</span> and return
              context, header, or routing updates.
            </p>
          </InspectorSection>

          <InspectorSection title="Code" subtitle="Write the JavaScript transformation logic here.">
            <div className="grid grid-cols-1 gap-2">
              <FieldLabel label="JavaScript Source" />
              <textarea
                value={codeRunner.code}
                onChange={(event) =>
                  onUpdateNode(
                    updateCodeRunnerConfig(node, {
                      ...codeRunner,
                      code: event.target.value,
                    }),
                  )
                }
                onBlur={(event) =>
                  onUpdateNode(
                    updateCodeRunnerConfig(node, {
                      ...codeRunner,
                      code: event.target.value,
                    }),
                  )
                }
                rows={14}
                spellCheck={false}
                className="nodrag nopan nowheel w-full resize-y rounded-2xl border border-zinc-200 bg-zinc-950 px-4 py-3 font-mono text-[12px] leading-6 text-zinc-100 outline-none transition placeholder:text-zinc-500 focus:border-emerald-400"
                placeholder={'export function run(input) {\n  return {\n    logs: [{ level: "info", message: "hello" }],\n  };\n}'}
              />
            </div>
          </InspectorSection>

          <InspectorSection title="Runtime Limits" subtitle="Keep the script fast and bounded.">
            <div className="grid grid-cols-1 gap-2">
              <FieldLabel label="Timeout (ms)" />
              <InlineInput
                value={String(codeRunner.timeout_ms)}
                placeholder="20"
                onChange={(value) =>
                  onUpdateNode(
                    updateCodeRunnerConfig(node, {
                      ...codeRunner,
                      timeout_ms: value === "" ? 0 : Number(value),
                    }),
                  )
                }
                onCommit={(value) =>
                  onUpdateNode(
                    updateCodeRunnerConfig(node, {
                      ...codeRunner,
                      timeout_ms: value === "" ? 0 : Number(value),
                    }),
                  )
                }
              />
              <FieldLabel label="Max Memory Bytes" />
              <InlineInput
                value={String(codeRunner.max_memory_bytes)}
                placeholder="16777216"
                onChange={(value) =>
                  onUpdateNode(
                    updateCodeRunnerConfig(node, {
                      ...codeRunner,
                      max_memory_bytes: value === "" ? 0 : Number(value),
                    }),
                  )
                }
                onCommit={(value) =>
                  onUpdateNode(
                    updateCodeRunnerConfig(node, {
                      ...codeRunner,
                      max_memory_bytes: value === "" ? 0 : Number(value),
                    }),
                  )
                }
              />
            </div>
          </InspectorSection>
        </div>

        <div className="flex items-center justify-end gap-3 border-t border-zinc-100 px-6 py-4">
          <button
            type="button"
            onClick={onClose}
            className="inline-flex h-10 items-center justify-center rounded-full border border-zinc-200 bg-white px-4 text-sm font-medium text-zinc-700 transition hover:border-zinc-300 hover:text-zinc-950"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

function InspectorSection({
  id,
  title,
  subtitle,
  children,
}: {
  id?: string;
  title: string;
  subtitle: string;
  children: ReactNode;
}) {
  return (
    <section id={id} className="border-t border-zinc-100 py-4 first:border-t-0 first:pt-0">
      <div className="text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-400">{title}</div>
      <p className="mt-1 text-sm leading-6 text-zinc-600">{subtitle}</p>
      <div className="mt-3">{children}</div>
    </section>
  );
}

function FieldLabel({ label }: { label: string }) {
  return <div className="text-[11px] font-medium tracking-[0.01em] text-zinc-600">{label}</div>;
}

const RuleCanvasEdge = memo(function RuleCanvasEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  markerEnd,
  data,
  selected,
}: EdgeProps<RuleCanvasEdgeData>) {
  const [path, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
    curvature: 0.35,
  });

  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        markerEnd={markerEnd}
        style={{
          stroke: data?.stroke ?? "#a1a1aa",
          strokeWidth: selected ? 3 : 1.75,
          filter: selected ? "drop-shadow(0 0 8px rgba(15,23,42,0.18))" : undefined,
        }}
      />
      {data?.label ? (
        <EdgeLabelRenderer>
          <div
            className="nodrag nopan absolute rounded-full border border-zinc-200 bg-white px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-600 shadow-[0_6px_18px_rgba(15,23,42,0.08)]"
            style={{
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
            }}
          >
            {data.label}
          </div>
        </EdgeLabelRenderer>
      ) : null}
    </>
  );
});

const nodeTypes = {
  ruleNode: RuleCanvasNode,
};

const edgeTypes = {
  ruleEdge: RuleCanvasEdge,
};

function replaceNode(graph: RuleGraphConfig, previousId: string, nextNode: RuleGraphNode): RuleGraphConfig {
  const nextGraph =
    previousId === nextNode.id
      ? {
          ...graph,
          nodes: graph.nodes.map((item) => (item.id === previousId ? nextNode : item)),
        }
      : renameNodeInGraph(graph, previousId, nextNode.id);

  return syncRouterTargetsWithEdges(pruneInvalidRouterEdges({
    ...nextGraph,
    nodes: nextGraph.nodes.map((item) => (item.id === nextNode.id ? nextNode : item)),
  }));
}

function renameNodeInGraph(
  graph: RuleGraphConfig,
  previousId: string,
  nextId: string,
): RuleGraphConfig {
  if (!nextId.trim() || previousId === nextId || graph.nodes.some((node) => node.id === nextId)) {
    return graph;
  }

  return {
    ...graph,
    start_node_id: graph.start_node_id === previousId ? nextId : graph.start_node_id,
    nodes: graph.nodes.map((node) => (node.id === previousId ? { ...node, id: nextId } : node)),
    edges: graph.edges.map((edge) => ({
      ...edge,
      id: edge.id.startsWith(`${previousId}-`) ? edge.id.replace(previousId, nextId) : edge.id,
      source: edge.source === previousId ? nextId : edge.source,
      target: edge.target === previousId ? nextId : edge.target,
    })),
  };
}

function removeNodeFromGraph(graph: RuleGraphConfig, nodeId: string): RuleGraphConfig {
  if (nodeId === graph.start_node_id) {
    return graph;
  }

  return {
    ...graph,
    nodes: graph.nodes.filter((item) => item.id !== nodeId),
    edges: graph.edges.filter((edge) => edge.source !== nodeId && edge.target !== nodeId),
  };
}

function pruneInvalidRouterEdges(graph: RuleGraphConfig): RuleGraphConfig {
  const allowedHandlesByNode = new Map<string, Set<string>>(
    graph.nodes.flatMap((node) => {
      if (node.type === "router" && node.router) {
        return [[
          node.id,
          new Set([
            ...node.router.rules.map((rule) => `router:${rule.id}`),
            "router:fallback",
          ]),
        ]];
      }
      if (node.type === "match" && node.match) {
        return [[
          node.id,
          new Set([
            ...node.match.branches.map((branch) => `match:${branch.id}`),
            "match:fallback",
          ]),
        ]];
      }
      return [];
    }),
  );

  return {
    ...graph,
    edges: graph.edges.filter((edge) => {
      const allowedHandles = allowedHandlesByNode.get(edge.source);
      if (!allowedHandles) {
        return true;
      }
      const sourceHandle = edge.source_handle ?? "";
      if (!sourceHandle.startsWith("router:") && !sourceHandle.startsWith("match:")) {
        return true;
      }
      return allowedHandles.has(edge.source_handle ?? "");
    }),
  };
}

function getEdgeTarget(graph: RuleGraphConfig, sourceId: string, sourceHandle: string | null) {
  return (
    graph.edges.find(
      (edge) =>
        edge.source === sourceId && (edge.source_handle ?? null) === (sourceHandle ?? null),
    )?.target ?? ""
  );
}

function setEdgeTarget(
  graph: RuleGraphConfig,
  sourceId: string,
  sourceHandle: string | null,
  targetId: string,
): RuleGraphConfig {
  const nextEdges = graph.edges.filter(
    (edge) =>
      !(
        edge.source === sourceId &&
        (edge.source_handle ?? null) === (sourceHandle ?? null)
      ),
  );

  if (!targetId) {
    return {
      ...graph,
      edges: nextEdges,
    };
  }

  return {
    ...graph,
    edges: [
      ...nextEdges,
      {
        id: `${sourceId}-${sourceHandle ?? "next"}`,
        source: sourceId,
        target: targetId,
        source_handle: sourceHandle,
      },
    ],
  };
}

function edgeLabelForHandle(sourceHandle: string | null) {
  if (sourceHandle === "true" || sourceHandle === "false") {
    return sourceHandle;
  }
  if (sourceHandle === "default") {
    return "default";
  }
  if (sourceHandle?.startsWith("router:")) {
    const key = sourceHandle.slice("router:".length);
    return key === "fallback" ? "fallback" : key.replace(/^rule-/, "");
  }
  if (sourceHandle?.startsWith("match:")) {
    const key = sourceHandle.slice("match:".length);
    return key === "fallback" ? "fallback" : key;
  }
  return "";
}

function syncRouterTargetsWithEdges(graph: RuleGraphConfig): RuleGraphConfig {
  return {
    ...graph,
    nodes: graph.nodes.map((node) => {
      if (node.type !== "router" || !node.router) {
        return node;
      }

      return {
        ...node,
        router: {
          rules: node.router.rules.map((rule) => ({
            ...rule,
            target_node_id:
              graph.edges.find(
                (edge) =>
                  edge.source === node.id && (edge.source_handle ?? null) === `router:${rule.id}`,
              )?.target ?? "",
          })),
          fallback_node_id:
            graph.edges.find(
              (edge) => edge.source === node.id && (edge.source_handle ?? null) === "router:fallback",
            )?.target ?? null,
        },
      };
    }).map((node) => {
      if (node.type !== "match" || !node.match) {
        return node;
      }

      return {
        ...node,
        match: {
          ...node.match,
          branches: node.match.branches.map((branch) => ({
            ...branch,
            target_node_id:
              graph.edges.find(
                (edge) =>
                  edge.source === node.id &&
                  (edge.source_handle ?? null) === `match:${branch.id}`,
              )?.target ?? "",
          })),
          fallback_node_id:
            graph.edges.find(
              (edge) => edge.source === node.id && (edge.source_handle ?? null) === "match:fallback",
            )?.target ?? null,
        },
      };
    }),
  };
}

function createNode(
  type: RuleGraphNodeType,
  index: number,
  existingNodes: RuleGraphNode[],
  pluginId?: string,
): RuleGraphNode {
  const base: RuleGraphNode = {
    id: nextNodeId(type, existingNodes, index + 1),
    type,
    position: { x: 0, y: 0 },
  };

  switch (type) {
    case "condition":
      return {
        ...base,
        condition: {
          mode: "expression",
          expression: 'path.startsWith("/v1/")',
        },
      };
    case "select_model":
      return { ...base, select_model: { provider_id: "", model_id: "" } };
    case "rewrite_path":
      return { ...base, rewrite_path: { value: "/v1/chat/completions" } };
    case "set_context":
      return { ...base, set_context: { key: "route_hint", value_template: "${ctx.header.x-target}" } };
    case "router":
      return {
        ...base,
        router: {
          rules: [
            {
              id: "rule-1",
              clauses: [{ source: "ctx.route_hint", operator: "==", value: "kimi" }],
              target_node_id: "",
            },
          ],
          fallback_node_id: null,
        },
      };
    case "log":
      return { ...base, log: { message: "route=${ctx.route_hint} model=${ctx.model.id}" } };
    case "set_header":
      return { ...base, set_header: { name: "X-Header", value: "" } };
    case "remove_header":
      return { ...base, remove_header: { name: "X-Header" } };
    case "copy_header":
      return { ...base, copy_header: { from: "Authorization", to: "X-Authorization" } };
    case "set_header_if_absent":
      return { ...base, set_header_if_absent: { name: "X-Header", value: "" } };
    case "wasm_plugin":
      if (pluginId === "condition-evaluator") {
        return {
          ...base,
          wasm_plugin: {
            plugin_id: pluginId,
            timeout_ms: 20,
            fuel: null,
            max_memory_bytes: 16_777_216,
            granted_capabilities: ["log"],
            read_dirs: [],
            write_dirs: [],
            allowed_hosts: [],
            config: {
              expr: 'method == "POST" && path.contains("/chat")',
            },
          },
        };
      }
      if (pluginId === "rewrite-path") {
        return {
          ...base,
          wasm_plugin: {
            plugin_id: pluginId,
            timeout_ms: 20,
            fuel: null,
            max_memory_bytes: 16_777_216,
            granted_capabilities: ["log"],
            read_dirs: [],
            write_dirs: [],
            allowed_hosts: [],
            config: {
              path_rewrite: "/v1/chat/completions",
            },
          },
        };
      }
      if (pluginId === "select-model") {
        return {
          ...base,
          wasm_plugin: {
            plugin_id: pluginId,
            timeout_ms: 20,
            fuel: null,
            max_memory_bytes: 16_777_216,
            granted_capabilities: ["log"],
            read_dirs: [],
            write_dirs: [],
            allowed_hosts: [],
            config: {
              provider_id: "",
              model_id: "",
            },
          },
        };
      }
      if (pluginId === "route-provider") {
        return {
          ...base,
          wasm_plugin: {
            plugin_id: pluginId,
            timeout_ms: 20,
            fuel: null,
            max_memory_bytes: 16_777_216,
            granted_capabilities: ["log"],
            read_dirs: [],
            write_dirs: [],
            allowed_hosts: [],
            config: {
              provider_id: "",
            },
          },
        };
      }
      if (pluginId === "set-header") {
        return {
          ...base,
          wasm_plugin: {
            plugin_id: pluginId,
            timeout_ms: 20,
            fuel: null,
            max_memory_bytes: 16_777_216,
            granted_capabilities: ["log"],
            read_dirs: [],
            write_dirs: [],
            allowed_hosts: [],
            config: {
              set_headers: [{ op: "set", name: "x-route-engine", value: "wasm" }],
            },
          },
        };
      }
      if (pluginId === "log-step") {
        return {
          ...base,
          wasm_plugin: {
            plugin_id: pluginId,
            timeout_ms: 20,
            fuel: null,
            max_memory_bytes: 16_777_216,
            granted_capabilities: ["log"],
            read_dirs: [],
            write_dirs: [],
            allowed_hosts: [],
            config: {
              log_message: "path=${path}",
            },
          },
        };
      }
      return {
        ...base,
        wasm_plugin: {
          plugin_id: pluginId ?? "",
          timeout_ms: 20,
          fuel: null,
          max_memory_bytes: 16_777_216,
          granted_capabilities: [],
          read_dirs: [],
          write_dirs: [],
          allowed_hosts: [],
          config: {},
        },
      };
    case "match":
      return {
        ...base,
        match: {
          plugin_id: pluginId ?? "",
          timeout_ms: 20,
          fuel: null,
          max_memory_bytes: 16_777_216,
          granted_capabilities: [],
          read_dirs: [],
          write_dirs: [],
          allowed_hosts: [],
          config: {},
          branches: [
            {
              id: "match",
              expr: 'ctx.header.x-target == "chat"',
              target_node_id: "",
            },
          ],
          fallback_node_id: null,
        },
      };
    case "code_runner":
      return {
        ...base,
        code_runner: {
          language: "javascript",
          timeout_ms: 20,
          max_memory_bytes: 16_777_216,
          code: [
            "export function run(input) {",
            "  return {",
            '    logs: [{ level: "info", message: "code runner executed" }],',
            "  };",
            "}",
          ].join("\n"),
        },
      };
    case "note":
      return { ...base, note_node: { text: "" } };
    default:
      return base;
  }
}

function nextNodeId(type: RuleGraphNodeType, existingNodes: RuleGraphNode[], seed: number) {
  let index = seed;
  let candidate = `${type}-${index}`;
  while (existingNodes.some((node) => node.id === candidate)) {
    index += 1;
    candidate = `${type}-${index}`;
  }
  return candidate;
}

function parseNodeLibraryItem(value: string): NodeLibraryItem | null {
  try {
    const parsed = JSON.parse(value) as Partial<NodeLibraryItem>;
    if (!parsed.type || !parsed.label) {
      return null;
    }
    return {
      type: parsed.type,
      label: parsed.label,
      shortLabel: parsed.shortLabel,
      pluginId: parsed.pluginId,
    };
  } catch {
    return null;
  }
}

function addRouterRule(node: RuleGraphNode): RuleGraphNode {
  const existingRules = node.router?.rules ?? [];
  return {
    ...node,
    router: {
      rules: [
        ...existingRules,
        {
          id: `rule-${existingRules.length + 1}`,
          clauses: [{ source: "ctx.path", operator: "startsWith", value: "/v1/" }],
          target_node_id: "",
        },
      ],
      fallback_node_id: node.router?.fallback_node_id ?? null,
    },
  };
}

function addWasmMatchBranch(node: RuleGraphNode): RuleGraphNode {
  const existingBranches = node.match?.branches ?? [];
  return {
    ...node,
    match: {
      ...node.match!,
      branches: [
        ...existingBranches,
        {
          id: `branch-${existingBranches.length + 1}`,
          expr: "",
          target_node_id: "",
        },
      ],
      fallback_node_id: node.match?.fallback_node_id ?? null,
    },
  };
}

function updateRouterClause(
  node: RuleGraphNode,
  ruleIndex: number,
  clauseIndex: number,
  patch: Partial<{ source: string; operator: string; value: string }>,
): RuleGraphNode {
  return {
    ...node,
    router: {
      rules: (node.router?.rules ?? []).map((rule, currentRuleIndex) =>
        currentRuleIndex === ruleIndex
          ? {
              ...rule,
              clauses: rule.clauses.map((clause, currentClauseIndex) =>
                currentClauseIndex === clauseIndex ? { ...clause, ...patch } : clause,
              ),
            }
          : rule,
      ),
      fallback_node_id: node.router?.fallback_node_id ?? null,
    },
  };
}

function updateWasmMatchBranch(
  node: RuleGraphNode,
  branchIndex: number,
  patch: Partial<{ id: string; expr: string; target_node_id: string }>,
): RuleGraphNode {
  return {
    ...node,
    match: {
      ...node.match!,
      branches: (node.match?.branches ?? []).map((branch, currentBranchIndex) =>
        currentBranchIndex === branchIndex ? { ...branch, ...patch } : branch,
      ),
      fallback_node_id: node.match?.fallback_node_id ?? null,
    },
  };
}

function seedNodePosition(type: RuleGraphNodeType, existingNodes: RuleGraphNode[]) {
  const similarNodes = existingNodes.filter((node) => node.type === type);
  const lane = Math.max(0, similarNodes.length);
  const lastNode = existingNodes.at(-1);

  return {
    x: lastNode ? lastNode.position.x + 260 : 320,
    y: 120 + lane * 120,
  };
}

function updateGraph(
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>,
  graph: RuleGraphConfig,
) {
  setConfig((current) => ({
    ...current,
    rule_graph: graph,
  }));
}

function validateGraph(
  graph: RuleGraphConfig,
  config: GatewayConfig,
  pluginManifests: WasmPluginManifestSummary[],
): ValidationResult {
  const globalIssues: string[] = [];
  const nodeIssues: Record<string, string[]> = {};
  const nodeMap = new Map(graph.nodes.map((node) => [node.id, node]));
  const startNodes = graph.nodes.filter((node) => node.type === "start");

  if (startNodes.length !== 1) {
    globalIssues.push(`Exactly one start node is required. Found ${startNodes.length}.`);
  }

  if (!nodeMap.has(graph.start_node_id)) {
    globalIssues.push(`Configured start node '${graph.start_node_id}' does not exist.`);
  }

  const reachable = new Set<string>();
  if (nodeMap.has(graph.start_node_id)) {
    const queue = [graph.start_node_id];
    while (queue.length > 0) {
      const current = queue.shift()!;
      if (reachable.has(current)) continue;
      reachable.add(current);
      for (const edge of graph.edges.filter((item) => item.source === current)) {
        queue.push(edge.target);
      }
    }
  }

  for (const node of graph.nodes) {
    const issues: string[] = [];

    if (node.type !== "note" && !reachable.has(node.id)) {
      issues.push("Node is unreachable from start.");
    }

    if (node.type === "condition") {
      const trueEdge = graph.edges.some(
        (edge) => edge.source === node.id && edge.source_handle === "true",
      );
      const falseEdge = graph.edges.some(
        (edge) => edge.source === node.id && edge.source_handle === "false",
      );
      if (!trueEdge) issues.push("Missing true branch.");
      if (!falseEdge) issues.push("Missing false branch.");
      if (node.condition?.mode === "expression" && !node.condition.expression?.trim()) {
        issues.push("Expression is empty.");
      }
      if (node.condition?.mode === "builder") {
        if (!node.condition.builder?.field) issues.push("Builder field is required.");
        if (!node.condition.builder?.operator) issues.push("Builder operator is required.");
        if (!node.condition.builder?.value) issues.push("Builder value is required.");
      }
    }

    if (node.type === "select_model") {
      if (!node.select_model?.provider_id) {
        issues.push("Provider is required.");
      } else if (
        !config.providers.some((provider) => provider.id === (node.select_model?.provider_id ?? ""))
      ) {
        issues.push("Provider does not exist.");
      }
      if (!node.select_model?.model_id) {
        issues.push("Model is required.");
      } else if (!config.models.some((model) => model.id === (node.select_model?.model_id ?? ""))) {
        issues.push("Model does not exist.");
      } else {
        const model = config.models.find((item) => item.id === (node.select_model?.model_id ?? ""));
        if (model && model.provider_id !== node.select_model?.provider_id) {
          issues.push("Model does not belong to selected provider.");
        }
      }
    }

    if (node.type === "rewrite_path" && !node.rewrite_path?.value?.trim()) {
      issues.push("Path rewrite value is required.");
    }

    if (node.type === "set_context") {
      if (!node.set_context?.key?.trim()) issues.push("Context key is required.");
      if (!node.set_context?.value_template?.trim()) issues.push("Context template is required.");
    }

    if (node.type === "router") {
      const rules = node.router?.rules ?? [];
      if (rules.length === 0) {
        issues.push("At least one match branch is required.");
      }
      for (const rule of rules) {
        if (!rule.target_node_id) issues.push(`Match branch '${rule.id}' is missing a target.`);
        if (!graph.nodes.some((item) => item.id === rule.target_node_id)) {
          issues.push(`Match branch '${rule.id}' target does not exist.`);
        }
        if (!rule.clauses?.length) issues.push(`Match branch '${rule.id}' needs a clause.`);
        for (const clause of rule.clauses ?? []) {
          if (!clause.source || !clause.operator || !clause.value) {
            issues.push(`Match branch '${rule.id}' has an incomplete clause.`);
          }
        }
      }
      if (
        node.router?.fallback_node_id &&
        !graph.nodes.some((item) => item.id === node.router?.fallback_node_id)
      ) {
        issues.push("Match fallback target does not exist.");
      }
    }

    if (node.type === "log" && !node.log?.message?.trim()) {
      issues.push("Log message is required.");
    }

    if (
      (node.type === "set_header" || node.type === "set_header_if_absent") &&
      !(node.type === "set_header" ? node.set_header?.name : node.set_header_if_absent?.name)
    ) {
      issues.push("Header name is required.");
    }

    if (
      (node.type === "set_header" || node.type === "set_header_if_absent") &&
      !(node.type === "set_header" ? node.set_header?.value : node.set_header_if_absent?.value)
    ) {
      issues.push("Header value is required.");
    }

    if (node.type === "remove_header" && !node.remove_header?.name) {
      issues.push("Header name is required.");
    }

    if (node.type === "copy_header") {
      if (!node.copy_header?.from) issues.push("Source header is required.");
      if (!node.copy_header?.to) issues.push("Target header is required.");
    }

    if (node.type === "wasm_plugin" || node.type === "match") {
      const wasmConfig = getWasmRuntimeConfig(node);
      const manifest = pluginManifests.find(
        (plugin) => plugin.id === (wasmConfig?.plugin_id ?? ""),
      );
      if (!wasmConfig?.plugin_id?.trim()) {
        issues.push("Plugin id is required.");
      } else if (!manifest) {
        issues.push("Selected plugin is not loaded.");
      }
      if ((wasmConfig?.timeout_ms ?? 0) <= 0) {
        issues.push("Timeout must be greater than zero.");
      }
      if ((wasmConfig?.fuel ?? 1) === 0) {
        issues.push("Fuel must be greater than zero when set.");
      }
      if ((wasmConfig?.max_memory_bytes ?? 0) <= 0) {
        issues.push("Memory limit must be greater than zero.");
      }

      const granted = new Set(wasmConfig?.granted_capabilities ?? []);
      if (manifest) {
        for (const capability of wasmConfig?.granted_capabilities ?? []) {
          if (!manifest.capabilities.includes(capability)) {
            issues.push(`Capability '${capability}' is not declared by the selected plugin.`);
          }
        }
      }

      if (granted.has("fs")) {
        if (!(wasmConfig?.read_dirs.length || wasmConfig?.write_dirs.length)) {
          issues.push("FS capability requires read or write directories.");
        }
      } else if (wasmConfig?.read_dirs.length || wasmConfig?.write_dirs.length) {
        issues.push("FS directories require the fs capability.");
      }

      if (granted.has("network")) {
        if (!wasmConfig?.allowed_hosts.length) {
          issues.push("Network capability requires allowed hosts.");
        }
      } else if (wasmConfig?.allowed_hosts.length) {
        issues.push("Allowed hosts require the network capability.");
      }

      if (node.type === "match") {
        const branches = node.match?.branches ?? [];
        if (branches.length === 0) {
          issues.push("At least one match branch is required.");
        }
        const seenBranchIds = new Set<string>();
        for (const branch of branches) {
          if (!branch.id.trim()) {
            issues.push("Match branch id is required.");
            continue;
          }
          if (!branch.expr.trim()) {
            issues.push(`Match branch '${branch.id}' expr is required.`);
          }
          if (seenBranchIds.has(branch.id)) {
            issues.push(`Match branch '${branch.id}' is duplicated.`);
          }
          seenBranchIds.add(branch.id);
          if (!branch.target_node_id) {
            issues.push(`Match branch '${branch.id}' is missing a target.`);
          } else if (!graph.nodes.some((item) => item.id === branch.target_node_id)) {
            issues.push(`Match branch '${branch.id}' target does not exist.`);
          }
        }
        if (node.match?.fallback_node_id && !graph.nodes.some((item) => item.id === node.match?.fallback_node_id)) {
          issues.push("Match fallback target does not exist.");
        }
      }
    }

    if (node.type === "code_runner") {
      const codeRunner = node.code_runner;
      if (!codeRunner) {
        issues.push("Code runner config is required.");
      } else {
        if (codeRunner.language !== "javascript") {
          issues.push("Only JavaScript is supported.");
        }
        if (!codeRunner.code.trim()) {
          issues.push("Code is required.");
        }
        if (codeRunner.timeout_ms <= 0) {
          issues.push("Timeout must be greater than zero.");
        }
        if (codeRunner.max_memory_bytes <= 0) {
          issues.push("Memory limit must be greater than zero.");
        }
      }
    }

    if (node.type !== "note" && issues.length > 0) {
      nodeIssues[node.id] = issues;
    }
  }

  return {
    globalIssues,
    nodeIssues,
    unreachableNodeIds: new Set(
      graph
        .nodes
        .filter((node) => node.type !== "note" && !reachable.has(node.id))
        .map((node) => node.id),
    ),
  };
}

function labelForType(type: RuleGraphNodeType) {
  switch (type) {
    case "start":
      return "Start";
    case "condition":
      return "Condition";
    case "select_model":
      return "Select Model";
    case "rewrite_path":
      return "Rewrite Path";
    case "set_context":
      return "Set Context";
    case "router":
      return "Match";
    case "log":
      return "Log";
    case "set_header":
      return "Set Header";
    case "remove_header":
      return "Remove Header";
    case "copy_header":
      return "Copy Header";
    case "set_header_if_absent":
      return "Set If Absent";
    case "wasm_plugin":
      return "Wasm Plugin";
    case "match":
      return "Match";
    case "code_runner":
      return "Code Runner";
    case "note":
      return "Note";
    case "end":
      return "End";
  }
}

function isWasmNodeType(type: RuleGraphNodeType) {
  return type === "wasm_plugin" || type === "match";
}

function getWasmRuntimeConfig(node: RuleGraphNode) {
  if (node.type === "wasm_plugin") {
    return node.wasm_plugin ?? null;
  }
  if (node.type === "match") {
    return node.match
      ? {
          plugin_id: node.match.plugin_id,
          timeout_ms: node.match.timeout_ms,
          fuel: node.match.fuel,
          max_memory_bytes: node.match.max_memory_bytes,
          granted_capabilities: node.match.granted_capabilities,
          read_dirs: node.match.read_dirs,
          write_dirs: node.match.write_dirs,
          allowed_hosts: node.match.allowed_hosts,
          config: node.match.config,
        }
      : null;
  }
  return null;
}

function getWasmMatchConfig(node: RuleGraphNode) {
  return node.type === "match" ? node.match ?? null : null;
}

function updateWasmRuntimeConfig(
  node: RuleGraphNode,
  config: NonNullable<RuleGraphNode["wasm_plugin"]>,
): RuleGraphNode {
  if (node.type === "wasm_plugin") {
    return {
      ...node,
      wasm_plugin: config,
    };
  }
  if (node.type === "match") {
    return {
      ...node,
      match: {
        ...node.match!,
        ...config,
      },
    };
  }
  return node;
}

function labelForNode(node: RuleGraphNode, pluginManifest?: WasmPluginManifestSummary | null) {
  if (node.type === "wasm_plugin" || node.type === "match") {
    return pluginManifest?.name ?? getWasmRuntimeConfig(node)?.plugin_id ?? labelForType(node.type);
  }
  return labelForType(node.type);
}

function describeWasmPlugin(
  pluginManifest: WasmPluginManifestSummary | null | undefined,
  node: RuleGraphNode,
) {
  const description = pluginManifest?.description?.trim();
  if (description) {
    return description;
  }

  const pluginId = getWasmRuntimeConfig(node)?.plugin_id?.trim();
  if (!pluginId) {
    return "Runs a wasm plugin step inside the request flow.";
  }

  return `Runs the ${pluginId} wasm plugin as a workflow step.`;
}

function isCodeRunnerNodeType(type: RuleGraphNodeType) {
  return type === "code_runner";
}

function getCodeRunnerConfig(node: RuleGraphNode) {
  return isCodeRunnerNodeType(node.type) ? node.code_runner ?? null : null;
}

function updateCodeRunnerConfig(
  node: RuleGraphNode,
  config: NonNullable<RuleGraphNode["code_runner"]>,
): RuleGraphNode {
  if (!isCodeRunnerNodeType(node.type)) {
    return node;
  }

  return {
    ...node,
    code_runner: config,
  };
}

function describeCodeRunner(node: RuleGraphNode) {
  const config = getCodeRunnerConfig(node);
  const language = config?.language ?? "javascript";

  if (language !== "javascript") {
    return "Runs a script to normalize request data before routing.";
  }

  return "Run JavaScript to normalize request data, rewrite fields, and choose the next port.";
}

function formatMemoryBytes(bytes: number) {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }

  const mb = bytes / (1024 * 1024);
  if (mb >= 1 && Number.isInteger(mb)) {
    return `${mb} MB`;
  }

  if (mb >= 1) {
    return `${mb.toFixed(1)} MB`;
  }

  const kb = bytes / 1024;
  if (kb >= 1 && Number.isInteger(kb)) {
    return `${kb} KB`;
  }

  if (kb >= 1) {
    return `${kb.toFixed(1)} KB`;
  }

  return `${bytes} B`;
}

function shortLabelForType(type: RuleGraphNodeType) {
  switch (type) {
    case "condition":
      return "If";
    case "select_model":
      return "Model";
    case "rewrite_path":
      return "Path";
    case "set_context":
      return "Context";
    case "router":
      return "Match";
    case "log":
      return "Log";
    case "set_header":
      return "Set";
    case "remove_header":
      return "Drop";
    case "copy_header":
      return "Copy";
    case "set_header_if_absent":
      return "Guard";
    case "wasm_plugin":
      return "Wasm";
    case "match":
      return "Match";
    case "code_runner":
      return "Code";
    case "note":
      return "Note";
    case "end":
      return "End";
    case "start":
      return "Start";
  }
}

function shortLabelForPlugin(name: string) {
  const compact = name.replace(/[^a-zA-Z0-9]/g, "");
  if (compact) {
    return compact.slice(0, 8);
  }
  const fallback = name.replace(/\s+/g, "");
  return (fallback || "Wasm").slice(0, 6);
}

function toneForManifestTone(tone: WasmPluginTone | null | undefined): NodeTone | null {
  const neutralCard = {
    cardBorder: "border-zinc-200",
    cardBg: "bg-white",
  };
  switch (tone) {
    case "slate":
      return {
        ...neutralCard,
        chipBg: "bg-slate-100",
        chipText: "text-slate-700",
        icon: "text-slate-700",
        libraryButton: "border-zinc-200 bg-white text-slate-700 hover:border-zinc-300 hover:text-slate-950",
        minimap: "#64748b",
        handle: "#64748b",
        edge: "#64748b",
      };
    case "blue":
      return {
        ...neutralCard,
        chipBg: "bg-blue-50",
        chipText: "text-blue-700",
        icon: "text-blue-700",
        libraryButton: "border-zinc-200 bg-white text-blue-700 hover:border-zinc-300 hover:text-blue-950",
        minimap: "#2563eb",
        handle: "#2563eb",
        edge: "#2563eb",
      };
    case "sky":
      return {
        ...neutralCard,
        chipBg: "bg-sky-50",
        chipText: "text-sky-700",
        icon: "text-sky-700",
        libraryButton: "border-zinc-200 bg-white text-sky-700 hover:border-zinc-300 hover:text-sky-950",
        minimap: "#0284c7",
        handle: "#0284c7",
        edge: "#0284c7",
      };
    case "teal":
      return {
        ...neutralCard,
        chipBg: "bg-teal-50",
        chipText: "text-teal-700",
        icon: "text-teal-700",
        libraryButton: "border-zinc-200 bg-white text-teal-700 hover:border-zinc-300 hover:text-teal-950",
        minimap: "#0f766e",
        handle: "#0f766e",
        edge: "#0f766e",
      };
    case "emerald":
      return {
        ...neutralCard,
        chipBg: "bg-emerald-50",
        chipText: "text-emerald-700",
        icon: "text-emerald-700",
        libraryButton: "border-zinc-200 bg-white text-emerald-700 hover:border-zinc-300 hover:text-emerald-950",
        minimap: "#10b981",
        handle: "#10b981",
        edge: "#10b981",
      };
    case "amber":
      return {
        ...neutralCard,
        chipBg: "bg-amber-50",
        chipText: "text-amber-700",
        icon: "text-amber-700",
        libraryButton: "border-zinc-200 bg-white text-amber-700 hover:border-zinc-300 hover:text-amber-950",
        minimap: "#d97706",
        handle: "#d97706",
        edge: "#d97706",
      };
    case "rose":
      return {
        ...neutralCard,
        chipBg: "bg-rose-50",
        chipText: "text-rose-700",
        icon: "text-rose-700",
        libraryButton: "border-zinc-200 bg-white text-rose-700 hover:border-zinc-300 hover:text-rose-950",
        minimap: "#e11d48",
        handle: "#e11d48",
        edge: "#e11d48",
      };
    case "violet":
      return {
        ...neutralCard,
        chipBg: "bg-violet-50",
        chipText: "text-violet-700",
        icon: "text-violet-700",
        libraryButton: "border-zinc-200 bg-white text-violet-700 hover:border-zinc-300 hover:text-violet-950",
        minimap: "#7c3aed",
        handle: "#7c3aed",
        edge: "#7c3aed",
      };
    default:
      return null;
  }
}

function toneForWasmAwareType(
  type: RuleGraphNodeType,
  pluginManifest?: WasmPluginManifestSummary | null,
): NodeTone {
  if (type === "wasm_plugin" || type === "match") {
    return toneForManifestTone(pluginManifest?.ui.tone) ?? toneForNodeType(type);
  }
  return toneForNodeType(type);
}

function toneForLibraryItem(item: NodeLibraryItem): NodeTone {
  return toneForWasmAwareType(item.type, item.pluginManifest);
}

function toneForGraphNode(
  node: RuleGraphNode | null,
  pluginManifestMap: Map<string, WasmPluginManifestSummary>,
): NodeTone {
  if (!node) {
    return toneForNodeType("start");
  }
  if (node.type === "wasm_plugin" || node.type === "match") {
    const manifest = getWasmRuntimeConfig(node)?.plugin_id
      ? pluginManifestMap.get(getWasmRuntimeConfig(node)!.plugin_id)
      : null;
    return toneForWasmAwareType(node.type, manifest);
  }
  return toneForNodeType(node.type);
}

function toneForNodeType(type: RuleGraphNodeType): NodeTone {
  const neutralCard = {
    cardBorder: "border-zinc-200",
    cardBg: "bg-white",
  };
  switch (type) {
    case "start":
      return {
        ...neutralCard,
        chipBg: "bg-zinc-900",
        chipText: "text-white",
        icon: "text-zinc-700",
        libraryButton: "border-zinc-200 bg-white text-zinc-700 hover:border-zinc-300 hover:text-zinc-900",
        minimap: "#111827",
        handle: "#111827",
        edge: "#111827",
      };
    case "condition":
      return {
        ...neutralCard,
        chipBg: "bg-violet-50",
        chipText: "text-violet-700",
        icon: "text-violet-600",
        libraryButton: "border-zinc-200 bg-white text-violet-700 hover:border-zinc-300 hover:text-violet-900",
        minimap: "#7c3aed",
        handle: "#7c3aed",
        edge: "#7c3aed",
      };
    case "select_model":
      return {
        ...neutralCard,
        chipBg: "bg-blue-50",
        chipText: "text-blue-700",
        icon: "text-blue-600",
        libraryButton: "border-zinc-200 bg-white text-blue-700 hover:border-zinc-300 hover:text-blue-900",
        minimap: "#2563eb",
        handle: "#2563eb",
        edge: "#2563eb",
      };
    case "route_provider":
      return {
        ...neutralCard,
        chipBg: "bg-cyan-50",
        chipText: "text-cyan-800",
        icon: "text-cyan-700",
        libraryButton: "border-zinc-200 bg-white text-cyan-800 hover:border-zinc-300 hover:text-cyan-950",
        minimap: "#0ea5e9",
        handle: "#0ea5e9",
        edge: "#0ea5e9",
      };
    case "rewrite_path":
      return {
        ...neutralCard,
        chipBg: "bg-amber-50",
        chipText: "text-amber-700",
        icon: "text-amber-600",
        libraryButton: "border-zinc-200 bg-white text-amber-700 hover:border-zinc-300 hover:text-amber-900",
        minimap: "#d97706",
        handle: "#d97706",
        edge: "#d97706",
      };
    case "set_context":
      return {
        ...neutralCard,
        chipBg: "bg-fuchsia-50",
        chipText: "text-fuchsia-700",
        icon: "text-fuchsia-700",
        libraryButton: "border-zinc-200 bg-white text-fuchsia-800 hover:border-zinc-300 hover:text-fuchsia-950",
        minimap: "#c026d3",
        handle: "#c026d3",
        edge: "#c026d3",
      };
    case "router":
      return {
        ...neutralCard,
        chipBg: "bg-indigo-50",
        chipText: "text-indigo-700",
        icon: "text-indigo-700",
        libraryButton: "border-zinc-200 bg-white text-indigo-800 hover:border-zinc-300 hover:text-indigo-950",
        minimap: "#4f46e5",
        handle: "#4f46e5",
        edge: "#4f46e5",
      };
    case "log":
      return {
        ...neutralCard,
        chipBg: "bg-cyan-50",
        chipText: "text-cyan-800",
        icon: "text-cyan-700",
        libraryButton: "border-zinc-200 bg-white text-cyan-800 hover:border-zinc-300 hover:text-cyan-950",
        minimap: "#0891b2",
        handle: "#0891b2",
        edge: "#0891b2",
      };
    case "set_header":
      return {
        ...neutralCard,
        chipBg: "bg-emerald-50",
        chipText: "text-emerald-700",
        icon: "text-emerald-600",
        libraryButton: "border-zinc-200 bg-white text-emerald-700 hover:border-zinc-300 hover:text-emerald-900",
        minimap: "#059669",
        handle: "#059669",
        edge: "#059669",
      };
    case "set_header_if_absent":
      return {
        ...neutralCard,
        chipBg: "bg-lime-50",
        chipText: "text-lime-700",
        icon: "text-lime-600",
        libraryButton: "border-zinc-200 bg-white text-lime-700 hover:border-zinc-300 hover:text-lime-900",
        minimap: "#65a30d",
        handle: "#65a30d",
        edge: "#65a30d",
      };
    case "remove_header":
      return {
        ...neutralCard,
        chipBg: "bg-rose-50",
        chipText: "text-rose-700",
        icon: "text-rose-600",
        libraryButton: "border-zinc-200 bg-white text-rose-700 hover:border-zinc-300 hover:text-rose-900",
        minimap: "#e11d48",
        handle: "#e11d48",
        edge: "#e11d48",
      };
    case "copy_header":
      return {
        ...neutralCard,
        chipBg: "bg-orange-50",
        chipText: "text-orange-700",
        icon: "text-orange-600",
        libraryButton: "border-zinc-200 bg-white text-orange-700 hover:border-zinc-300 hover:text-orange-900",
        minimap: "#ea580c",
        handle: "#ea580c",
        edge: "#ea580c",
      };
    case "note":
      return {
        cardBorder: "border-amber-200",
        cardBg: "bg-amber-50",
        chipBg: "bg-amber-100",
        chipText: "text-amber-900",
        icon: "text-amber-700",
        libraryButton: "border-zinc-200 bg-white text-amber-800 hover:border-zinc-300 hover:text-amber-950",
        minimap: "#f59e0b",
        handle: "#f59e0b",
        edge: "#f59e0b",
      };
    case "wasm_plugin":
      return {
        ...neutralCard,
        chipBg: "bg-teal-50",
        chipText: "text-teal-700",
        icon: "text-teal-700",
        libraryButton: "border-zinc-200 bg-white text-teal-800 hover:border-zinc-300 hover:text-teal-950",
        minimap: "#0f766e",
        handle: "#0f766e",
        edge: "#0f766e",
      };
    case "match":
      return {
        ...neutralCard,
        chipBg: "bg-sky-50",
        chipText: "text-sky-700",
        icon: "text-sky-700",
        libraryButton: "border-zinc-200 bg-white text-sky-800 hover:border-zinc-300 hover:text-sky-950",
        minimap: "#0284c7",
        handle: "#0284c7",
        edge: "#0284c7",
      };
    case "code_runner":
      return {
        ...neutralCard,
        chipBg: "bg-emerald-50",
        chipText: "text-emerald-700",
        icon: "text-emerald-700",
        libraryButton: "border-zinc-200 bg-white text-emerald-800 hover:border-zinc-300 hover:text-emerald-950",
        minimap: "#10b981",
        handle: "#10b981",
        edge: "#10b981",
      };
    case "end":
      return {
        ...neutralCard,
        chipBg: "bg-slate-100",
        chipText: "text-slate-800",
        icon: "text-slate-600",
        libraryButton: "border-zinc-200 bg-white text-slate-700 hover:border-zinc-300 hover:text-slate-900",
        minimap: "#475569",
        handle: "#475569",
        edge: "#475569",
      };
    default:
      return {
        ...neutralCard,
        chipBg: "bg-zinc-100",
        chipText: "text-zinc-900",
        icon: "text-zinc-700",
        libraryButton: "border-zinc-200 bg-white text-zinc-700 hover:border-zinc-300 hover:text-zinc-900",
        minimap: "#71717a",
        handle: "#71717a",
        edge: "#71717a",
      };
  }
}

function iconForPluginIcon(icon: WasmPluginIcon | null | undefined) {
  const iconClass = "h-4 w-4";
  switch (icon) {
    case "split":
      return <GitBranch className={iconClass} />;
    case "route":
      return <Route className={iconClass} />;
    case "wand":
      return <WandSparkles className={iconClass} />;
    case "shield":
      return <Shield className={iconClass} />;
    case "code":
      return <FileCode2 className={iconClass} />;
    case "filter":
      return <Filter className={iconClass} />;
    case "database":
      return <Database className={iconClass} />;
    case "file_text":
      return <FileText className={iconClass} />;
    case "puzzle":
      return <Puzzle className={iconClass} />;
    default:
      return null;
  }
}

function iconForWasmAwareType(
  type: RuleGraphNodeType,
  pluginManifest?: WasmPluginManifestSummary | null,
) {
  if (type === "wasm_plugin" || type === "match") {
    return iconForPluginIcon(pluginManifest?.ui.icon) ?? iconForLibraryNode(type);
  }
  return iconForLibraryNode(type);
}

function iconForLibraryItem(item: NodeLibraryItem) {
  return iconForWasmAwareType(item.type, item.pluginManifest);
}

function iconForLibraryNode(type: RuleGraphNodeType) {
  const iconClass = "h-4 w-4";
  switch (type) {
    case "condition":
      return <GitBranch className={iconClass} />;
    case "select_model":
      return <Split className={iconClass} />;
    case "route_provider":
      return <Route className={iconClass} />;
    case "rewrite_path":
      return <Route className={iconClass} />;
    case "set_context":
      return <DatabaseZap className={iconClass} />;
    case "router":
      return <ListTree className={iconClass} />;
    case "log":
      return <FileText className={iconClass} />;
    case "set_header":
      return <Plus className={iconClass} />;
    case "remove_header":
      return <Minus className={iconClass} />;
    case "copy_header":
      return <CopyPlus className={iconClass} />;
    case "set_header_if_absent":
      return <ShieldPlus className={iconClass} />;
    case "wasm_plugin":
      return <Puzzle className={iconClass} />;
    case "match":
      return <Puzzle className={iconClass} />;
    case "code_runner":
      return <FileCode2 className={iconClass} />;
    case "note":
      return <FileText className={iconClass} />;
    case "end":
      return <Hand className={iconClass} />;
    case "start":
      return <ArrowRightLeft className={iconClass} />;
    default:
      return <Grip className={iconClass} />;
  }
}

type CompletionMatch = {
  start: number;
  end: number;
  mode: "template" | "context";
  query: string;
};

function findCompletionMatch(value: string, caret: number): CompletionMatch | null {
  const beforeCaret = value.slice(0, caret);
  const templateMatch = beforeCaret.match(/\$\{[a-zA-Z0-9._-]*$/);
  if (templateMatch) {
    return {
      start: caret - templateMatch[0].length,
      end: caret,
      mode: "template",
      query: templateMatch[0].slice(2).toLowerCase(),
    };
  }

  const contextMatch = beforeCaret.match(/ctx\.[a-zA-Z0-9._-]*$/);
  if (contextMatch) {
    return {
      start: caret - contextMatch[0].length,
      end: caret,
      mode: "context",
      query: contextMatch[0].toLowerCase(),
    };
  }

  return null;
}

function contextSuggestionValue(suggestion: string) {
  return suggestion.replace(/^\$\{/, "").replace(/\}$/, "");
}

function filterSuggestions(suggestions: string[] | undefined, match: CompletionMatch | null) {
  if (!suggestions?.length || !match) {
    return [];
  }

  const candidatePool =
    match.mode === "template"
      ? suggestions
      : suggestions.map((suggestion) => contextSuggestionValue(suggestion));

  return candidatePool.filter((candidate) => candidate.toLowerCase().startsWith(match.query));
}

function parseTextareaList(value: string) {
  return value
    .split(/\r?\n|,/)
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

function formatListForTextarea(items: string[]) {
  return items.join("\n");
}

function formatPluginConfig(config: Record<string, unknown>) {
  try {
    return JSON.stringify(config, null, 2);
  } catch {
    return "{}";
  }
}

function parsePluginConfig(value: string, fallback: Record<string, unknown>) {
  if (!value.trim()) {
    return {};
  }

  try {
    const parsed = JSON.parse(value);
    return parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : fallback;
  } catch {
    return fallback;
  }
}

function normalizePluginConfigObject(config: unknown): Record<string, unknown> {
  return config && typeof config === "object" && !Array.isArray(config)
    ? ({ ...config } as Record<string, unknown>)
    : {};
}

function updatePluginConfigField(config: unknown, key: string, value: unknown): Record<string, unknown> {
  return {
    ...normalizePluginConfigObject(config),
    [key]: value,
  };
}

function selectOptionsForSchemaField(
  field: WasmPluginConfigField,
  configObject: Record<string, unknown>,
  providerOptions: SelectOption[],
  modelOptions: SelectOption[],
) {
  if (field.data_source === "providers") {
    return providerOptions;
  }

  if (field.data_source === "models") {
    const providerId =
      field.depends_on && typeof configObject[field.depends_on] === "string"
        ? (configObject[field.depends_on] as string)
        : "";

    if (!providerId) {
      return modelOptions;
    }

    return modelOptions.filter((option) => option.providerId === providerId);
  }

  return [];
}

function InlineInput({
  value,
  placeholder,
  suggestions,
  onChange,
  onCommit,
}: {
  value: string;
  placeholder?: string;
  suggestions?: string[];
  onChange: (value: string) => void;
  onCommit: (value: string) => void;
}) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [completion, setCompletion] = useState<{
    match: CompletionMatch;
    items: string[];
    activeIndex: number;
  } | null>(null);

  const refreshCompletion = (nextValue: string, caret: number | null) => {
    if (caret === null) {
      setCompletion(null);
      return;
    }

    const match = findCompletionMatch(nextValue, caret);
    const items = filterSuggestions(suggestions, match);
    if (!match || items.length === 0) {
      setCompletion(null);
      return;
    }

    setCompletion((current) => ({
      match,
      items,
      activeIndex: current ? Math.min(current.activeIndex, items.length - 1) : 0,
    }));
  };

  const applyCompletion = (selected: string) => {
    if (!completion) {
      return;
    }

    const replacement =
      completion.match.mode === "template" ? selected : contextSuggestionValue(selected);
    const nextValue =
      value.slice(0, completion.match.start) +
      replacement +
      value.slice(completion.match.end);
    const nextCaret = completion.match.start + replacement.length;

    onChange(nextValue);
    setCompletion(null);
    requestAnimationFrame(() => {
      inputRef.current?.focus();
      inputRef.current?.setSelectionRange(nextCaret, nextCaret);
    });
  };

  return (
    <div className="relative">
      <input
        ref={inputRef}
        value={value}
        placeholder={placeholder}
        onClick={(event) => event.stopPropagation()}
        onPointerDown={(event) => event.stopPropagation()}
        onChange={(event) => {
          onChange(event.target.value);
          refreshCompletion(event.target.value, event.target.selectionStart);
        }}
        onKeyDown={(event) => {
          if (!completion) {
            return;
          }
          if (event.key === "ArrowDown") {
            event.preventDefault();
            setCompletion((current) =>
              current
                ? { ...current, activeIndex: (current.activeIndex + 1) % current.items.length }
                : current,
            );
          } else if (event.key === "ArrowUp") {
            event.preventDefault();
            setCompletion((current) =>
              current
                ? {
                    ...current,
                    activeIndex:
                      (current.activeIndex - 1 + current.items.length) % current.items.length,
                  }
                : current,
            );
          } else if (event.key === "Enter" || event.key === "Tab") {
            event.preventDefault();
            applyCompletion(completion.items[completion.activeIndex]);
          } else if (event.key === "Escape") {
            event.preventDefault();
            setCompletion(null);
          }
        }}
        onKeyUp={(event) => {
          if (["ArrowDown", "ArrowUp", "Enter", "Tab", "Escape"].includes(event.key)) {
            return;
          }
          refreshCompletion(event.currentTarget.value, event.currentTarget.selectionStart);
        }}
        onBlur={(event) => {
          setCompletion(null);
          onCommit(event.target.value);
        }}
        className="nodrag nopan nowheel h-9 w-full rounded-md border border-zinc-200 bg-white px-3 text-sm text-zinc-900 outline-none transition placeholder:text-zinc-400 focus:border-zinc-400 focus:ring-2 focus:ring-zinc-100"
      />
      {completion ? (
        <div className="absolute left-0 right-0 top-[calc(100%+6px)] z-30 overflow-hidden rounded-md border border-zinc-200 bg-white shadow-lg">
          {completion.items.slice(0, 8).map((suggestion, index) => (
            <button
              key={suggestion}
              type="button"
              onMouseDown={(event) => {
                event.preventDefault();
                event.stopPropagation();
                applyCompletion(suggestion);
              }}
              className={[
                "block w-full px-3 py-2 text-left font-mono text-xs text-zinc-700 transition",
                index === completion.activeIndex ? "bg-zinc-900 text-white" : "hover:bg-zinc-100",
              ].join(" ")}
            >
              {suggestion}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function InlineTextarea({
  value,
  placeholder,
  suggestions,
  onChange,
  onCommit,
}: {
  value: string;
  placeholder?: string;
  suggestions?: string[];
  onChange: (value: string) => void;
  onCommit: (value: string) => void;
}) {
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const [completion, setCompletion] = useState<{
    match: CompletionMatch;
    items: string[];
    activeIndex: number;
  } | null>(null);

  const refreshCompletion = (nextValue: string, caret: number | null) => {
    if (caret === null) {
      setCompletion(null);
      return;
    }

    const match = findCompletionMatch(nextValue, caret);
    const items = filterSuggestions(suggestions, match);
    if (!match || items.length === 0) {
      setCompletion(null);
      return;
    }

    setCompletion((current) => ({
      match,
      items,
      activeIndex: current ? Math.min(current.activeIndex, items.length - 1) : 0,
    }));
  };

  const applyCompletion = (selected: string) => {
    if (!completion) {
      return;
    }

    const replacement =
      completion.match.mode === "template" ? selected : contextSuggestionValue(selected);
    const nextValue =
      value.slice(0, completion.match.start) +
      replacement +
      value.slice(completion.match.end);
    const nextCaret = completion.match.start + replacement.length;

    onChange(nextValue);
    setCompletion(null);
    requestAnimationFrame(() => {
      textareaRef.current?.focus();
      textareaRef.current?.setSelectionRange(nextCaret, nextCaret);
    });
  };

  return (
    <div className="relative">
      <textarea
        ref={textareaRef}
        value={value}
        placeholder={placeholder}
        rows={3}
        onClick={(event) => event.stopPropagation()}
        onPointerDown={(event) => event.stopPropagation()}
        onChange={(event) => {
          onChange(event.target.value);
          refreshCompletion(event.target.value, event.target.selectionStart);
        }}
        onKeyDown={(event) => {
          if (!completion) {
            return;
          }
          if (event.key === "ArrowDown") {
            event.preventDefault();
            setCompletion((current) =>
              current
                ? { ...current, activeIndex: (current.activeIndex + 1) % current.items.length }
                : current,
            );
          } else if (event.key === "ArrowUp") {
            event.preventDefault();
            setCompletion((current) =>
              current
                ? {
                    ...current,
                    activeIndex:
                      (current.activeIndex - 1 + current.items.length) % current.items.length,
                  }
                : current,
            );
          } else if (event.key === "Tab") {
            event.preventDefault();
            applyCompletion(completion.items[completion.activeIndex]);
          } else if (event.key === "Escape") {
            event.preventDefault();
            setCompletion(null);
          }
        }}
        onKeyUp={(event) => {
          if (["ArrowDown", "ArrowUp", "Tab", "Escape"].includes(event.key)) {
            return;
          }
          refreshCompletion(event.currentTarget.value, event.currentTarget.selectionStart);
        }}
        onBlur={(event) => {
          setCompletion(null);
          onCommit(event.target.value);
        }}
        className="nodrag nopan nowheel w-full resize-none rounded-md border border-zinc-200 bg-white px-3 py-2 text-sm text-zinc-900 outline-none transition placeholder:text-zinc-400 focus:border-zinc-400 focus:ring-2 focus:ring-zinc-100"
      />
      {completion ? (
        <div className="absolute left-0 right-0 top-[calc(100%+6px)] z-30 overflow-hidden rounded-md border border-zinc-200 bg-white shadow-lg">
          {completion.items.slice(0, 8).map((suggestion, index) => (
            <button
              key={suggestion}
              type="button"
              onMouseDown={(event) => {
                event.preventDefault();
                event.stopPropagation();
                applyCompletion(suggestion);
              }}
              className={[
                "block w-full px-3 py-2 text-left font-mono text-xs text-zinc-700 transition",
                index === completion.activeIndex ? "bg-zinc-900 text-white" : "hover:bg-zinc-100",
              ].join(" ")}
            >
              {completion.match.mode === "template" ? suggestion : contextSuggestionValue(suggestion)}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function InlineSelect({
  value,
  options,
  onChange,
  placeholder,
}: {
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
  placeholder?: string;
}) {
  return (
    <select
      value={value}
      onClick={(event) => event.stopPropagation()}
      onPointerDown={(event) => event.stopPropagation()}
      onChange={(event) => onChange(event.target.value)}
      className="nodrag nopan nowheel h-9 w-full rounded-md border border-zinc-200 bg-white px-3 text-sm text-zinc-900 outline-none transition focus:border-zinc-400 focus:ring-2 focus:ring-zinc-100"
    >
      <option value="">{placeholder ?? "Select..."}</option>
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}
