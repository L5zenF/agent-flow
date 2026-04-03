import { memo, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowRightLeft,
  CopyPlus,
  GitBranch,
  Grip,
  Hand,
  FileText,
  ListTree,
  Minus,
  Plus,
  Route,
  DatabaseZap,
  ShieldPlus,
  Split,
  Trash2,
} from "lucide-react";
import ReactFlow, {
  applyEdgeChanges,
  applyNodeChanges,
  BaseEdge,
  Background,
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
} from "@/lib/types";

type Props = {
  config: GatewayConfig;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
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
  providerOptions: SelectOption[];
  modelOptions: SelectOption[];
  templateSuggestions: string[];
  onUpdateNode: (nextNode: RuleGraphNode) => void;
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

const NODE_LIBRARY: Array<{ type: RuleGraphNodeType; label: string }> = [
  { type: "condition", label: "Condition" },
  { type: "select_model", label: "Select Model" },
  { type: "set_context", label: "Set Context" },
  { type: "router", label: "Match" },
  { type: "log", label: "Log" },
  { type: "rewrite_path", label: "Rewrite Path" },
  { type: "set_header", label: "Set Header" },
  { type: "remove_header", label: "Remove Header" },
  { type: "copy_header", label: "Copy Header" },
  { type: "set_header_if_absent", label: "Set If Absent" },
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

export function RuleGraphEditor({ config, setConfig }: Props) {
  const graph = config.rule_graph ?? emptyConfig().rule_graph!;
  const graphRef = useRef(graph);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(graph.start_node_id);
  const [selectedEdgeId, setSelectedEdgeId] = useState<string | null>(null);
  const [flowInstance, setFlowInstance] = useState<ReactFlowInstance | null>(null);
  const validation = useMemo(() => validateGraph(graph, config), [graph, config]);
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
          providerOptions,
          modelOptions,
          templateSuggestions,
          onUpdateNode: (nextNode) => {
            updateGraph(setConfig, replaceNode(graph, node.id, nextNode));
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
                : toneForNodeType(
                    graph.nodes.find((node) => node.id === edge.source)?.type ?? "start",
                  ).edge,
        },
        markerEnd: { type: MarkerType.ArrowClosed },
        className: "rule-flow-edge",
      })),
    [graph.edges, graph.nodes, selectedEdgeId],
  );

  const addNode = (type: RuleGraphNodeType, position?: { x: number; y: number }) => {
    const next = createNode(type, graph.nodes.length, graph.nodes);
    const node = {
      ...next,
      position: position ?? seedNodePosition(type, graph.nodes),
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
      sourceNode.type === "router"
        ? syncRouterTargetsWithEdges(
            setEdgeTarget(graph, connection.source, connection.sourceHandle ?? null, connection.target),
          )
        : setEdgeTarget(
            graph,
            connection.source,
            sourceNode.type === "condition" ? connection.sourceHandle ?? null : null,
            connection.target,
          );

    updateGraph(setConfig, nextGraph);
    setSelectedEdgeId(null);
  };

  return (
    <div className="min-w-0">
      <div className="rule-graph-canvas h-[calc(100dvh-8.75rem)] min-h-[560px] rounded-[24px] bg-[radial-gradient(circle_at_top,#fff_0%,#f7f7f5_46%,#eef2f7_100%)]">
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
            const type = event.dataTransfer.getData("application/rule-node-type") as RuleGraphNodeType;
            if (!type || !flowInstance) return;
            addNode(type, flowInstance.screenToFlowPosition({ x: event.clientX, y: event.clientY }));
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
              return toneForNodeType(data.nodeType).minimap;
            }}
            maskColor="rgba(255,255,255,0.72)"
          />
          <Controls showInteractive={false} />
          <Background gap={20} size={1.2} color="rgba(15, 23, 42, 0.12)" />

          <Panel position="top-left" className="!m-4">
            <div className="rounded-[20px] border border-zinc-200/80 bg-white/88 p-2 shadow-[0_18px_50px_rgba(15,23,42,0.12)] backdrop-blur">
              <div className="mb-2 px-2 pt-1 font-mono text-[9px] uppercase tracking-[0.16em] text-zinc-500">
                Nodes
              </div>
              <div className="grid grid-cols-3 gap-2 sm:grid-cols-4 xl:grid-cols-1">
                {NODE_LIBRARY.map((item) => (
                  <button
                    key={item.type}
                    type="button"
                    draggable
                    title={item.label}
                    onClick={() => addNode(item.type)}
                    onDragStart={(event) => {
                      event.dataTransfer.setData("application/rule-node-type", item.type);
                      event.dataTransfer.effectAllowed = "move";
                    }}
                    className={[
                      "group relative flex h-12 min-w-[54px] items-center justify-center rounded-xl border bg-white transition",
                      toneForNodeType(item.type).libraryButton,
                    ].join(" ")}
                  >
                    <div className="flex flex-col items-center gap-1">
                      {iconForLibraryNode(item.type)}
                      <span className="font-mono text-[8px] uppercase tracking-[0.12em] text-zinc-500">
                        {shortLabelForType(item.type)}
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
            <div
              className={[
                "inline-flex items-center gap-2 rounded-full border px-3 py-1.5 text-xs font-medium backdrop-blur",
                validationBadgeTone,
              ].join(" ")}
              title={validationBadgeTitle}
            >
              <span
                className={[
                  "inline-flex h-5 w-5 items-center justify-center rounded-full text-[11px] font-black",
                  validationIssueCount > 0 ? "bg-rose-600 text-white" : "bg-emerald-600 text-white",
                ].join(" ")}
              >
                {validationIssueCount > 0 ? "!" : "✓"}
              </span>
              <span className="whitespace-nowrap">{validationBadgeText}</span>
            </div>
          </Panel>

          <Panel position="bottom-left" className="!m-4">
            <div className="rounded-lg bg-white/82 px-3 py-2 text-xs text-zinc-600 shadow-[0_10px_30px_rgba(15,23,42,0.1)] backdrop-blur">
              Drag from the node dock or click an icon to insert a step.
            </div>
          </Panel>
        </ReactFlow>
      </div>
    </div>
  );
}

const RuleCanvasNode = memo(function RuleCanvasNode({ data, selected }: NodeProps<RuleCanvasNodeData>) {
  const [draft, setDraft] = useState<RuleGraphNode>(data.node);
  const [isEditingNote, setIsEditingNote] = useState(false);
  const noteEditorRef = useRef<HTMLTextAreaElement | null>(null);
  const routerBranches = draft.router?.rules ?? [];
  const routerHandleCount = routerBranches.length + 1;

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

  const icon =
    data.nodeType === "condition" ? (
      <GitBranch className="h-4 w-4" />
    ) : data.nodeType === "router" ? (
      <ListTree className="h-4 w-4" />
    ) : data.nodeType === "log" ? (
      <FileText className="h-4 w-4" />
    ) : data.nodeType === "set_context" ? (
      <DatabaseZap className="h-4 w-4" />
    ) : data.nodeType === "note" ? (
      <FileText className="h-4 w-4" />
    ) : data.nodeType === "start" || data.nodeType === "end" ? (
      <Grip className="h-4 w-4" />
    ) : (
      <Route className="h-4 w-4" />
    );
  const tone = toneForNodeType(data.nodeType);

  const borderTone = data.unreachable
    ? "border-rose-300"
    : data.issueCount > 0
      ? "border-amber-300"
      : tone.cardBorder;
  const bgTone = data.unreachable ? "bg-rose-50/92" : data.issueCount > 0 ? "bg-amber-50/92" : tone.cardBg;
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
          "min-w-[250px] max-w-[300px] rounded-[22px] border px-4 py-3 shadow-[0_18px_50px_rgba(15,23,42,0.08)] transition",
          isNoteNode
            ? "min-h-[160px] border-dashed bg-[linear-gradient(180deg,rgba(255,251,235,0.98)_0%,rgba(254,243,199,0.96)_100%)] shadow-[0_24px_60px_rgba(180,83,9,0.16)]"
            : "",
          borderTone,
          bgTone,
          selected ? "ring-2 ring-zinc-900/15" : "",
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
            <div className="pointer-events-none absolute left-1/2 top-0 h-5 w-16 -translate-x-1/2 -translate-y-1/2 rounded-full bg-amber-200/80 blur-[1px]" />
            <div className="pointer-events-none absolute right-0 top-0 h-10 w-10 rounded-tr-[22px] bg-[linear-gradient(135deg,rgba(255,255,255,0.9)_0%,rgba(251,191,36,0.18)_55%,rgba(217,119,6,0.28)_100%)] [clip-path:polygon(100%_0,0_0,100%_100%)]" />
          </>
        ) : null}
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <div
              className={[
                "inline-flex rounded-full px-2 py-1 font-mono text-[10px] uppercase tracking-[0.18em]",
                isNoteNode ? "bg-amber-300/70 text-amber-950" : "",
                tone.chipBg,
                tone.chipText,
              ].join(" ")}
            >
              {labelForType(data.nodeType)}
            </div>
          </div>
          <div className="flex items-center gap-2">
            <div className={tone.icon}>{icon}</div>
            {selected && draft.type !== "start" ? (
              <button
                type="button"
                onClick={(event) => {
                  event.stopPropagation();
                  data.onDeleteNode();
                }}
                className="nodrag nopan inline-flex h-8 w-8 items-center justify-center rounded-full border border-zinc-200 bg-white text-zinc-500 transition hover:border-rose-200 hover:text-rose-600"
              >
                <Trash2 className="h-4 w-4" />
              </button>
            ) : null}
          </div>
        </div>

        <div className="mt-3 space-y-2">
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
                className="nodrag nopan nowheel w-full resize-none rounded-2xl border border-amber-300/80 bg-white/70 px-3 py-3 text-sm leading-7 text-zinc-900 outline-none transition [background-image:linear-gradient(transparent_31px,rgba(217,119,6,0.12)_32px)] [background-size:100%_32px] focus:border-amber-500"
              />
            ) : (
              <button
                type="button"
                onDoubleClick={(event) => {
                  event.stopPropagation();
                  setIsEditingNote(true);
                }}
                className="nodrag nopan min-h-[110px] w-full rounded-2xl border border-amber-200/70 bg-amber-50/45 px-3 py-3 text-left text-sm leading-7 text-zinc-800 [background-image:linear-gradient(transparent_31px,rgba(217,119,6,0.1)_32px)] [background-size:100%_32px]"
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
              <div className="rounded-xl border border-indigo-200 bg-indigo-50/70 px-2.5 py-2 text-[11px] text-indigo-900">
                First match wins. Drag each branch handle on the right edge to its target.
              </div>
              {(draft.router?.rules ?? []).map((rule, ruleIndex) => (
                <div key={rule.id} className="rounded-2xl border border-zinc-200/80 bg-white/70 p-2">
                  <div className="mb-2 flex items-center gap-2">
                    <span className="inline-flex items-center gap-2 rounded-full bg-indigo-900 px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-white">
                      <span className="inline-flex h-2 w-2 rounded-full bg-white/90" />
                      Branch {ruleIndex + 1}
                    </span>
                    <span className="min-w-0 truncate rounded-full border border-indigo-200 bg-indigo-50 px-2 py-1 text-[10px] text-indigo-700">
                      {rule.target_node_id || "Drag to target"}
                    </span>
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
                      className="nodrag nopan ml-auto inline-flex h-7 w-7 items-center justify-center rounded-full border border-zinc-200 bg-white text-zinc-500 transition hover:border-rose-200 hover:text-rose-600"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>
                  {(rule.clauses ?? []).map((clause, clauseIndex) => (
                    <div key={`${rule.id}-${clauseIndex}`} className="mb-2 grid grid-cols-1 gap-2">
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
              ))}
              <button
                type="button"
                onClick={(event) => {
                  event.stopPropagation();
                  commitNode(addRouterRule(draft));
                }}
                className="nodrag nopan inline-flex h-9 w-full items-center justify-center rounded-xl border border-dashed border-zinc-300 bg-white/60 text-sm text-zinc-700 transition hover:border-zinc-500 hover:text-zinc-900"
              >
                Add Branch
              </button>
              <div className="space-y-2 rounded-2xl border border-slate-200/80 bg-slate-50/70 p-2">
                <div className="flex items-center gap-2">
                  <div className="inline-flex items-center gap-2 rounded-full bg-slate-800 px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-white">
                    <span className="inline-flex h-2 w-2 rounded-full bg-white/90" />
                    Fallback
                  </div>
                  <span className="min-w-0 truncate rounded-full border border-slate-200 bg-white px-2 py-1 text-[10px] text-slate-700">
                    {draft.router?.fallback_node_id || "Drag to target"}
                  </span>
                </div>
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
  const allowedHandlesByNode = new Map(
    graph.nodes
      .filter((node) => node.type === "router" && node.router)
      .map((node) => [
        node.id,
        new Set([
          ...node.router!.rules.map((rule) => `router:${rule.id}`),
          "router:fallback",
        ]),
      ]),
  );

  return {
    ...graph,
    edges: graph.edges.filter((edge) => {
      const allowedHandles = allowedHandlesByNode.get(edge.source);
      if (!allowedHandles) {
        return true;
      }
      if (!(edge.source_handle ?? "").startsWith("router:")) {
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
  if (sourceHandle?.startsWith("router:")) {
    const key = sourceHandle.slice("router:".length);
    return key === "fallback" ? "fallback" : key.replace(/^rule-/, "");
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
    }),
  };
}

function createNode(
  type: RuleGraphNodeType,
  index: number,
  existingNodes: RuleGraphNode[],
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

function validateGraph(graph: RuleGraphConfig, config: GatewayConfig): ValidationResult {
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
    case "note":
      return "Note";
    case "end":
      return "End";
  }
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
    case "note":
      return "Note";
    case "end":
      return "End";
    case "start":
      return "Start";
  }
}

function toneForNodeType(type: RuleGraphNodeType): NodeTone {
  switch (type) {
    case "start":
      return {
        cardBorder: "border-zinc-300",
        cardBg: "bg-zinc-200/95",
        chipBg: "bg-zinc-900",
        chipText: "text-white",
        icon: "text-zinc-700",
        libraryButton: "border-zinc-200 text-zinc-700 hover:border-zinc-900 hover:text-zinc-900",
        minimap: "#111827",
        handle: "#111827",
        edge: "#111827",
      };
    case "condition":
      return {
        cardBorder: "border-violet-300",
        cardBg: "bg-violet-200/88",
        chipBg: "bg-violet-100",
        chipText: "text-violet-800",
        icon: "text-violet-600",
        libraryButton: "border-violet-200 text-violet-700 hover:border-violet-500 hover:text-violet-900",
        minimap: "#7c3aed",
        handle: "#7c3aed",
        edge: "#7c3aed",
      };
    case "select_model":
      return {
        cardBorder: "border-blue-300",
        cardBg: "bg-blue-200/90",
        chipBg: "bg-blue-100",
        chipText: "text-blue-800",
        icon: "text-blue-600",
        libraryButton: "border-blue-200 text-blue-700 hover:border-blue-500 hover:text-blue-900",
        minimap: "#2563eb",
        handle: "#2563eb",
        edge: "#2563eb",
      };
    case "rewrite_path":
      return {
        cardBorder: "border-amber-300",
        cardBg: "bg-amber-200/88",
        chipBg: "bg-amber-100",
        chipText: "text-amber-800",
        icon: "text-amber-600",
        libraryButton: "border-amber-200 text-amber-700 hover:border-amber-500 hover:text-amber-900",
        minimap: "#d97706",
        handle: "#d97706",
        edge: "#d97706",
      };
    case "set_context":
      return {
        cardBorder: "border-fuchsia-300",
        cardBg: "bg-fuchsia-100/92",
        chipBg: "bg-fuchsia-200",
        chipText: "text-fuchsia-900",
        icon: "text-fuchsia-700",
        libraryButton: "border-fuchsia-300 text-fuchsia-800 hover:border-fuchsia-500 hover:text-fuchsia-950",
        minimap: "#c026d3",
        handle: "#c026d3",
        edge: "#c026d3",
      };
    case "router":
      return {
        cardBorder: "border-indigo-300",
        cardBg: "bg-indigo-100/92",
        chipBg: "bg-indigo-200",
        chipText: "text-indigo-900",
        icon: "text-indigo-700",
        libraryButton: "border-indigo-300 text-indigo-800 hover:border-indigo-500 hover:text-indigo-950",
        minimap: "#4f46e5",
        handle: "#4f46e5",
        edge: "#4f46e5",
      };
    case "log":
      return {
        cardBorder: "border-cyan-300",
        cardBg: "bg-cyan-100/92",
        chipBg: "bg-cyan-200",
        chipText: "text-cyan-900",
        icon: "text-cyan-700",
        libraryButton: "border-cyan-300 text-cyan-800 hover:border-cyan-500 hover:text-cyan-950",
        minimap: "#0891b2",
        handle: "#0891b2",
        edge: "#0891b2",
      };
    case "set_header":
      return {
        cardBorder: "border-emerald-300",
        cardBg: "bg-emerald-200/88",
        chipBg: "bg-emerald-100",
        chipText: "text-emerald-800",
        icon: "text-emerald-600",
        libraryButton: "border-emerald-200 text-emerald-700 hover:border-emerald-500 hover:text-emerald-900",
        minimap: "#059669",
        handle: "#059669",
        edge: "#059669",
      };
    case "set_header_if_absent":
      return {
        cardBorder: "border-lime-300",
        cardBg: "bg-lime-200/88",
        chipBg: "bg-lime-100",
        chipText: "text-lime-800",
        icon: "text-lime-600",
        libraryButton: "border-lime-200 text-lime-700 hover:border-lime-500 hover:text-lime-900",
        minimap: "#65a30d",
        handle: "#65a30d",
        edge: "#65a30d",
      };
    case "remove_header":
      return {
        cardBorder: "border-rose-300",
        cardBg: "bg-rose-200/88",
        chipBg: "bg-rose-100",
        chipText: "text-rose-800",
        icon: "text-rose-600",
        libraryButton: "border-rose-200 text-rose-700 hover:border-rose-500 hover:text-rose-900",
        minimap: "#e11d48",
        handle: "#e11d48",
        edge: "#e11d48",
      };
    case "copy_header":
      return {
        cardBorder: "border-orange-300",
        cardBg: "bg-orange-200/88",
        chipBg: "bg-orange-100",
        chipText: "text-orange-800",
        icon: "text-orange-600",
        libraryButton: "border-orange-200 text-orange-700 hover:border-orange-500 hover:text-orange-900",
        minimap: "#ea580c",
        handle: "#ea580c",
        edge: "#ea580c",
      };
    case "note":
      return {
        cardBorder: "border-amber-300",
        cardBg: "bg-amber-100/95",
        chipBg: "bg-amber-200",
        chipText: "text-amber-900",
        icon: "text-amber-700",
        libraryButton: "border-amber-300 text-amber-800 hover:border-amber-500 hover:text-amber-950",
        minimap: "#f59e0b",
        handle: "#f59e0b",
        edge: "#f59e0b",
      };
    case "end":
      return {
        cardBorder: "border-slate-300",
        cardBg: "bg-slate-200/90",
        chipBg: "bg-slate-200",
        chipText: "text-slate-800",
        icon: "text-slate-600",
        libraryButton: "border-slate-200 text-slate-700 hover:border-slate-500 hover:text-slate-900",
        minimap: "#475569",
        handle: "#475569",
        edge: "#475569",
      };
  }
}

function iconForLibraryNode(type: RuleGraphNodeType) {
  const iconClass = "h-4 w-4";
  switch (type) {
    case "condition":
      return <GitBranch className={iconClass} />;
    case "select_model":
      return <Split className={iconClass} />;
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
    case "note":
      return <FileText className={iconClass} />;
    case "end":
      return <Hand className={iconClass} />;
    case "start":
      return <ArrowRightLeft className={iconClass} />;
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
        className="nodrag nopan nowheel h-9 w-full rounded-xl border border-zinc-200 bg-white px-3 text-sm text-zinc-900 outline-none transition placeholder:text-zinc-400 focus:border-zinc-400"
      />
      {completion ? (
        <div className="absolute left-0 right-0 top-[calc(100%+6px)] z-30 overflow-hidden rounded-xl border border-zinc-200 bg-white/95 shadow-[0_18px_40px_rgba(15,23,42,0.12)] backdrop-blur">
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
        className="nodrag nopan nowheel w-full resize-none rounded-xl border border-zinc-200 bg-white px-3 py-2 text-sm text-zinc-900 outline-none transition placeholder:text-zinc-400 focus:border-zinc-400"
      />
      {completion ? (
        <div className="absolute left-0 right-0 top-[calc(100%+6px)] z-30 overflow-hidden rounded-xl border border-zinc-200 bg-white/95 shadow-[0_18px_40px_rgba(15,23,42,0.12)] backdrop-blur">
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
      className="nodrag nopan nowheel h-9 w-full rounded-xl border border-zinc-200 bg-white px-3 text-sm text-zinc-900 outline-none transition focus:border-zinc-400"
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
