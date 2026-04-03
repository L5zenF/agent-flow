import { memo, useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowRightLeft,
  CopyPlus,
  GitBranch,
  Grip,
  Hand,
  Minus,
  Network,
  Plus,
  Route,
  ShieldPlus,
  Split,
  Trash2,
} from "lucide-react";
import ReactFlow, {
  applyNodeChanges,
  applyEdgeChanges,
  Background,
  ConnectionMode,
  Controls,
  Handle,
  MarkerType,
  MiniMap,
  Position,
  type Edge,
  type EdgeChange,
  type Node,
  type NodeChange,
  type NodeProps,
  type OnConnect,
  type OnEdgesChange,
  type OnNodesChange,
  type ReactFlowInstance,
} from "reactflow";
import "reactflow/dist/style.css";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  emptyConfig,
  type GatewayConfig,
  type RuleGraphConfig,
  type RuleGraphEdge,
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

type RuleCanvasNodeData = {
  label: string;
  subtitle: string;
  nodeType: RuleGraphNodeType;
  issueCount: number;
  unreachable: boolean;
};

const NODE_LIBRARY: Array<{ type: RuleGraphNodeType; label: string }> = [
  { type: "condition", label: "Condition" },
  { type: "route_provider", label: "Route Provider" },
  { type: "select_model", label: "Select Model" },
  { type: "rewrite_path", label: "Rewrite Path" },
  { type: "set_header", label: "Set Header" },
  { type: "remove_header", label: "Remove Header" },
  { type: "copy_header", label: "Copy Header" },
  { type: "set_header_if_absent", label: "Set If Absent" },
  { type: "end", label: "End" },
];

const CONDITION_FIELDS = ["path", "method", 'header["x-target"]', "provider.id", "model.id"];
const CONDITION_OPERATORS = ["==", "!=", "startsWith", "contains"];

export function RuleGraphEditor({ config, setConfig }: Props) {
  const graph = config.rule_graph ?? emptyConfig().rule_graph!;
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(graph.start_node_id);
  const [flowInstance, setFlowInstance] = useState<ReactFlowInstance | null>(null);
  const validation = useMemo(() => validateGraph(graph, config), [graph, config]);
  const selectedNode = graph.nodes.find((node) => node.id === selectedNodeId) ?? null;

  const flowNodes = useMemo<Array<Node<RuleCanvasNodeData>>>(() => graph.nodes.map((node) => ({
    id: node.id,
    type: "ruleNode",
    position: node.position,
    data: {
      label: labelForType(node.type),
      subtitle: subtitleForNode(node),
      nodeType: node.type,
      issueCount: (validation.nodeIssues[node.id] ?? []).length,
      unreachable: validation.unreachableNodeIds.has(node.id),
    },
    selected: node.id === selectedNodeId,
    draggable: true,
  })), [graph.nodes, selectedNodeId, validation.nodeIssues, validation.unreachableNodeIds]);

  const [canvasNodes, setCanvasNodes] = useState<Array<Node<RuleCanvasNodeData>>>(flowNodes);
  const canvasNodesRef = useRef(canvasNodes);

  useEffect(() => {
    setCanvasNodes(flowNodes);
  }, [flowNodes]);

  useEffect(() => {
    canvasNodesRef.current = canvasNodes;
  }, [canvasNodes]);

  const flowEdges = useMemo<Edge[]>(() => graph.edges.map((edge) => ({
    id: edge.id,
    source: edge.source,
    target: edge.target,
    sourceHandle: edge.source_handle ?? undefined,
    label: edge.source_handle === "true" ? "true" : edge.source_handle === "false" ? "false" : "",
    markerEnd: { type: MarkerType.ArrowClosed },
    className: "rule-flow-edge",
  })), [graph.edges]);

  const nodeOptions = useMemo(() => graph.nodes.map((node) => ({
    value: node.id,
    label: `${node.id} · ${labelForType(node.type)}`,
  })), [graph.nodes]);

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

    updateGraph(setConfig, {
      ...graph,
      nodes: graph.nodes.map((node) => ({
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
  });

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

    updateGraph(setConfig, {
      ...graph,
      edges: nextEdges.map((edge) => ({
        id: edge.id,
        source: edge.source,
        target: edge.target,
        source_handle: edge.sourceHandle ?? null,
      })),
    });
  };

  const onConnect: OnConnect = (connection) => {
    if (!connection.source || !connection.target || connection.source === connection.target) {
      return;
    }

    const sourceNode = graph.nodes.find((node) => node.id === connection.source);
    if (!sourceNode) {
      return;
    }

    let nextGraph = graph;
    if (sourceNode.type === "condition") {
      nextGraph = setEdgeTarget(
        graph,
        connection.source,
        connection.sourceHandle ?? null,
        connection.target,
      );
    } else {
      nextGraph = setEdgeTarget(graph, connection.source, null, connection.target);
    }

    updateGraph(setConfig, nextGraph);
  };

  return (
    <div className="grid gap-4 lg:grid-cols-[72px_minmax(0,1fr)_360px]">
      <div className="space-y-3">
        <Card className="group overflow-visible p-2">
          <div className="mb-2 px-1 pt-1 text-center font-mono text-[10px] uppercase tracking-[0.16em] text-zinc-500">
            Tools
          </div>
          <div className="space-y-2">
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
                className="relative flex h-12 w-full items-center justify-center rounded-xl border border-zinc-200 bg-white text-zinc-700 transition hover:border-zinc-900 hover:text-zinc-900"
              >
                <div className="flex flex-col items-center gap-1">
                  {iconForLibraryNode(item.type)}
                  <span className="font-mono text-[8px] uppercase tracking-[0.12em] text-zinc-500">
                    {shortLabelForType(item.type)}
                  </span>
                </div>
                <span className="pointer-events-none absolute left-[calc(100%+10px)] top-1/2 z-20 -translate-y-1/2 whitespace-nowrap rounded-lg border border-zinc-200 bg-white px-2 py-1 text-[11px] font-medium text-zinc-700 opacity-0 shadow-[0_10px_30px_rgba(15,23,42,0.12)] transition group-hover:opacity-100">
                  {item.label}
                </span>
              </button>
            ))}
          </div>
        </Card>
      </div>

      <Card className="overflow-hidden p-0">
        <div className="flex items-center justify-between border-b border-zinc-200 px-4 py-3">
          <div>
            <div className="font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
              Rule Canvas
            </div>
            <div className="mt-1 text-sm text-zinc-600">
              React Flow visual editor. Drag nodes, connect branches, edit details on the right.
            </div>
          </div>
          <div className="flex items-center gap-3 text-xs text-zinc-500">
            {validation.globalIssues.length === 0 ? (
              <div className="flex items-center gap-2 text-emerald-700">
                <span className="h-2 w-2 rounded-full bg-emerald-500" />
                Valid
              </div>
            ) : (
              <div className="flex items-center gap-2 text-amber-700">
                <AlertTriangle className="h-3.5 w-3.5" />
                {validation.globalIssues.length} issue{validation.globalIssues.length > 1 ? "s" : ""}
              </div>
            )}
            <div>
              {graph.nodes.length} nodes · {graph.edges.length} edges
            </div>
          </div>
        </div>

        <div className="h-[720px] bg-[radial-gradient(circle_at_top,#fff_0%,#f5f5f4_48%,#f1f5f9_100%)]">
          <ReactFlow
            nodes={canvasNodes}
            edges={flowEdges}
            nodeTypes={nodeTypes}
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
                if (data.nodeType === "condition") return "#0f172a";
                if (data.nodeType === "end") return "#334155";
                return "#2563eb";
              }}
              maskColor="rgba(255,255,255,0.72)"
            />
            <Controls showInteractive={false} />
            <Background gap={20} size={1.2} color="rgba(15, 23, 42, 0.12)" />
            <div className="pointer-events-none absolute left-4 top-4 z-10 rounded-xl border border-zinc-200 bg-white/85 px-3 py-2 text-xs text-zinc-600 shadow-sm backdrop-blur">
              Drag icons from the left rail onto the canvas.
            </div>
          </ReactFlow>
        </div>
      </Card>

      <Card className="p-4">
        <div className="mb-3 font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
          Node Properties
        </div>
        {!selectedNode ? (
          <div className="text-sm text-zinc-500">Select a node to edit its properties.</div>
        ) : (
          <NodeInspector
            node={selectedNode}
            graph={graph}
            config={config}
            setConfig={setConfig}
            nodeOptions={nodeOptions}
            validationIssues={validation.nodeIssues[selectedNode.id] ?? []}
            onDelete={() => {
              const nextGraph = removeNodeFromGraph(graph, selectedNode.id);
              updateGraph(setConfig, nextGraph);
              setSelectedNodeId(nextGraph.start_node_id);
            }}
          />
        )}
      </Card>
    </div>
  );
}

const RuleCanvasNode = memo(function RuleCanvasNode({ data, selected }: NodeProps<RuleCanvasNodeData>) {
  const icon =
    data.nodeType === "condition" ? (
      <GitBranch className="h-4 w-4" />
    ) : data.nodeType === "start" || data.nodeType === "end" ? (
      <Grip className="h-4 w-4" />
    ) : (
      <Route className="h-4 w-4" />
    );

  const borderTone = data.unreachable
    ? "border-rose-300"
    : data.issueCount > 0
      ? "border-amber-300"
      : data.nodeType === "condition"
        ? "border-slate-700"
        : "border-zinc-200";

  const bgTone = selected
    ? "bg-zinc-900 text-white"
    : data.unreachable
      ? "bg-rose-50"
      : data.issueCount > 0
        ? "bg-amber-50"
        : "bg-white";

  return (
    <>
      <Handle type="target" position={Position.Left} className="!h-3 !w-3 !border-2 !border-white !bg-zinc-900" />
      {data.nodeType === "condition" ? (
        <>
          <Handle
            id="true"
            type="source"
            position={Position.Right}
            style={{ top: "34%" }}
            className="!h-3 !w-3 !border-2 !border-white !bg-emerald-500"
          />
          <Handle
            id="false"
            type="source"
            position={Position.Right}
            style={{ top: "68%" }}
            className="!h-3 !w-3 !border-2 !border-white !bg-rose-500"
          />
        </>
      ) : data.nodeType !== "end" ? (
        <Handle
          type="source"
          position={Position.Right}
          className="!h-3 !w-3 !border-2 !border-white !bg-blue-600"
        />
      ) : null}

      <div
        className={[
          "min-w-[220px] rounded-2xl border px-4 py-3 shadow-[0_16px_40px_rgba(15,23,42,0.08)] transition",
          borderTone,
          bgTone,
          selected ? "ring-2 ring-zinc-300" : "",
        ].join(" ")}
      >
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="font-mono text-[11px] uppercase tracking-[0.18em] opacity-60">
              {labelForType(data.nodeType)}
            </div>
            <div className="mt-1 text-sm font-semibold">{data.label}</div>
          </div>
          <div className={selected ? "text-zinc-300" : "text-zinc-500"}>{icon}</div>
        </div>
        <div className={["mt-2 text-xs", selected ? "text-zinc-300" : "text-zinc-600"].join(" ")}>
          {data.subtitle}
        </div>
        {data.nodeType === "condition" ? (
          <div className={["mt-3 flex justify-between text-[10px] uppercase tracking-[0.16em]", selected ? "text-zinc-400" : "text-zinc-500"].join(" ")}>
            <span>in</span>
            <span>true / false</span>
          </div>
        ) : null}
        {data.unreachable || data.issueCount > 0 ? (
          <div className={["mt-3 text-[11px] font-medium", selected ? "text-zinc-200" : "text-zinc-700"].join(" ")}>
            {data.unreachable
              ? "Unreachable from start"
              : `${data.issueCount} validation issue${data.issueCount > 1 ? "s" : ""}`}
          </div>
        ) : null}
      </div>
    </>
  );
});

const nodeTypes = {
  ruleNode: RuleCanvasNode,
};

function NodeInspector({
  node,
  graph,
  config,
  setConfig,
  nodeOptions,
  validationIssues,
  onDelete,
}: {
  node: RuleGraphNode;
  graph: RuleGraphConfig;
  config: GatewayConfig;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
  nodeOptions: Array<{ value: string; label: string }>;
  validationIssues: string[];
  onDelete: () => void;
}) {
  const [draft, setDraft] = useState<RuleGraphNode>(node);

  useEffect(() => {
    setDraft(node);
  }, [node]);

  const updateNode = (nextNode: RuleGraphNode) => {
    updateGraph(setConfig, replaceNode(graph, node.id, nextNode));
  };

  const commitDraft = (nextNode: RuleGraphNode = draft) => {
    setDraft(nextNode);
    updateNode(nextNode);
  };

  const updateTarget = (sourceHandle: string | null, targetId: string) => {
    updateGraph(setConfig, setEdgeTarget(graph, node.id, sourceHandle, targetId));
  };

  const nextTarget = getEdgeTarget(graph, node.id, null);
  const trueTarget = getEdgeTarget(graph, node.id, "true");
  const falseTarget = getEdgeTarget(graph, node.id, "false");

  return (
    <div className="space-y-4">
      {validationIssues.length > 0 && (
        <div className="space-y-2">
          {validationIssues.map((issue) => (
            <div
              key={issue}
              className="rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-800"
            >
              {issue}
            </div>
          ))}
        </div>
      )}

      <Field
        label="ID"
        value={draft.id}
        onChange={(value) => setDraft((current) => ({ ...current, id: value }))}
        onCommit={() => commitDraft()}
      />

      <SelectField
        label="Type"
        value={draft.type}
        disabled={draft.type === "start"}
        options={[
          { value: "start", label: "Start" },
          { value: "condition", label: "Condition" },
          { value: "route_provider", label: "Route Provider" },
          { value: "select_model", label: "Select Model" },
          { value: "rewrite_path", label: "Rewrite Path" },
          { value: "set_header", label: "Set Header" },
          { value: "remove_header", label: "Remove Header" },
          { value: "copy_header", label: "Copy Header" },
          { value: "set_header_if_absent", label: "Set If Absent" },
          { value: "end", label: "End" },
        ]}
        onChange={(value) => {
          const nextNode = resetNodeType(draft, value as RuleGraphNodeType);
          setDraft(nextNode);
          commitDraft(nextNode);
        }}
      />

      {draft.type === "condition" ? (
        <>
          <SelectField
            label="Mode"
            value={draft.condition?.mode ?? "expression"}
            options={[
              { value: "expression", label: "Expression" },
              { value: "builder", label: "Builder" },
            ]}
            onChange={(value) => {
              const nextNode = {
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
              };
              setDraft(nextNode);
              commitDraft(nextNode);
            }}
          />
          {draft.condition?.mode === "builder" ? (
            <>
              <SelectField
                label="Field"
                value={draft.condition?.builder?.field ?? "path"}
                options={CONDITION_FIELDS.map((item) => ({ value: item, label: item }))}
                onChange={(value) => {
                  const nextNode = {
                    ...draft,
                    condition: {
                      ...draft.condition!,
                      builder: {
                        field: value,
                        operator: draft.condition?.builder?.operator ?? "startsWith",
                        value: draft.condition?.builder?.value ?? "/v1/",
                      },
                    },
                  };
                  setDraft(nextNode);
                  commitDraft(nextNode);
                }}
              />
              <SelectField
                label="Operator"
                value={draft.condition?.builder?.operator ?? "startsWith"}
                options={CONDITION_OPERATORS.map((item) => ({ value: item, label: item }))}
                onChange={(value) => {
                  const nextNode = {
                    ...draft,
                    condition: {
                      ...draft.condition!,
                      builder: {
                        field: draft.condition?.builder?.field ?? "path",
                        operator: value,
                        value: draft.condition?.builder?.value ?? "/v1/",
                      },
                    },
                  };
                  setDraft(nextNode);
                  commitDraft(nextNode);
                }}
              />
              <Field
                label="Value"
                value={draft.condition?.builder?.value ?? ""}
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
                onCommit={() => commitDraft()}
              />
            </>
          ) : (
            <TextAreaField
              label="Expression"
              value={draft.condition?.expression ?? ""}
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
              onCommit={() => commitDraft()}
            />
          )}
          <SelectField
            label="True Target"
            value={trueTarget}
            options={nodeOptions.filter((item) => item.value !== node.id)}
            onChange={(value) => updateTarget("true", value)}
            placeholder="No target"
          />
          <SelectField
            label="False Target"
            value={falseTarget}
            options={nodeOptions.filter((item) => item.value !== node.id)}
            onChange={(value) => updateTarget("false", value)}
            placeholder="No target"
          />
        </>
      ) : null}

      {draft.type !== "condition" && draft.type !== "end" ? (
        <SelectField
          label="Next Target"
          value={nextTarget}
          options={nodeOptions.filter((item) => item.value !== node.id)}
          onChange={(value) => updateTarget(null, value)}
          placeholder="No target"
        />
      ) : null}

      {draft.type === "route_provider" ? (
        <SelectField
          label="Provider"
          value={draft.route_provider?.provider_id ?? ""}
          options={config.providers.map((provider) => ({ value: provider.id, label: provider.id }))}
          onChange={(value) => {
            const nextNode = {
              ...draft,
              route_provider: { provider_id: value },
            };
            setDraft(nextNode);
            commitDraft(nextNode);
          }}
          placeholder="Select provider"
        />
      ) : null}

      {draft.type === "select_model" ? (
        <SelectField
          label="Model"
          value={draft.select_model?.model_id ?? ""}
          options={config.models.map((model) => ({ value: model.id, label: model.id }))}
          onChange={(value) => {
            const nextNode = {
              ...draft,
              select_model: { model_id: value },
            };
            setDraft(nextNode);
            commitDraft(nextNode);
          }}
          placeholder="Select model"
        />
      ) : null}

      {draft.type === "rewrite_path" ? (
        <Field
          label="Path"
          value={draft.rewrite_path?.value ?? ""}
          onChange={(value) =>
            setDraft((current) => ({
              ...current,
              rewrite_path: { value },
            }))
          }
          onCommit={() => commitDraft()}
        />
      ) : null}

      {(draft.type === "set_header" || draft.type === "set_header_if_absent") && (
        <>
          <Field
            label="Header"
            value={
              draft.type === "set_header"
                ? draft.set_header?.name ?? ""
                : draft.set_header_if_absent?.name ?? ""
            }
            onChange={(value) =>
              setDraft((current) => ({
                ...current,
                [current.type]:
                  current.type === "set_header"
                    ? { name: value, value: current.set_header?.value ?? "" }
                    : { name: value, value: current.set_header_if_absent?.value ?? "" },
              }))
            }
            onCommit={() => commitDraft()}
          />
          <Field
            label="Value"
            value={
              draft.type === "set_header"
                ? draft.set_header?.value ?? ""
                : draft.set_header_if_absent?.value ?? ""
            }
            onChange={(value) =>
              setDraft((current) => ({
                ...current,
                [current.type]:
                  current.type === "set_header"
                    ? { name: current.set_header?.name ?? "", value }
                    : { name: current.set_header_if_absent?.name ?? "", value },
              }))
            }
            onCommit={() => commitDraft()}
          />
        </>
      )}

      {draft.type === "remove_header" ? (
        <Field
          label="Header"
          value={draft.remove_header?.name ?? ""}
          onChange={(value) =>
            setDraft((current) => ({
              ...current,
              remove_header: { name: value },
            }))
          }
          onCommit={() => commitDraft()}
        />
      ) : null}

      {draft.type === "copy_header" ? (
        <>
          <Field
            label="From"
            value={draft.copy_header?.from ?? ""}
            onChange={(value) =>
              setDraft((current) => ({
                ...current,
                copy_header: {
                  from: value,
                  to: current.copy_header?.to ?? "",
                },
              }))
            }
            onCommit={() => commitDraft()}
          />
          <Field
            label="To"
            value={draft.copy_header?.to ?? ""}
            onChange={(value) =>
              setDraft((current) => ({
                ...current,
                copy_header: {
                  from: current.copy_header?.from ?? "",
                  to: value,
                },
              }))
            }
            onCommit={() => commitDraft()}
          />
        </>
      ) : null}

      {draft.type !== "start" ? (
        <Button variant="outline" className="w-full justify-center gap-2" onClick={onDelete}>
          <Trash2 className="h-4 w-4" />
          Delete Node
        </Button>
      ) : null}
    </div>
  );
}

function replaceNode(graph: RuleGraphConfig, previousId: string, nextNode: RuleGraphNode): RuleGraphConfig {
  const nextGraph =
    previousId === nextNode.id
      ? {
          ...graph,
          nodes: graph.nodes.map((item) => (item.id === previousId ? nextNode : item)),
        }
      : renameNodeInGraph(graph, previousId, nextNode.id);

  return {
    ...nextGraph,
    nodes: nextGraph.nodes.map((item) => (item.id === nextNode.id ? nextNode : item)),
  };
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
      ) && edge.target !== sourceId,
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
    case "route_provider":
      return { ...base, route_provider: { provider_id: "" } };
    case "select_model":
      return { ...base, select_model: { model_id: "" } };
    case "rewrite_path":
      return { ...base, rewrite_path: { value: "/v1/chat/completions" } };
    case "set_header":
      return { ...base, set_header: { name: "X-Header", value: "" } };
    case "remove_header":
      return { ...base, remove_header: { name: "X-Header" } };
    case "copy_header":
      return { ...base, copy_header: { from: "Authorization", to: "X-Authorization" } };
    case "set_header_if_absent":
      return { ...base, set_header_if_absent: { name: "X-Header", value: "" } };
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

function seedNodePosition(type: RuleGraphNodeType, existingNodes: RuleGraphNode[]) {
  const similarNodes = existingNodes.filter((node) => node.type === type);
  const lane = Math.max(0, similarNodes.length);
  const lastNode = existingNodes.at(-1);

  return {
    x: lastNode ? lastNode.position.x + 260 : 320,
    y: 120 + lane * 120,
  };
}

function resetNodeType(node: RuleGraphNode, type: RuleGraphNodeType): RuleGraphNode {
  const next = createNode(type, 0, []);
  return { ...next, id: node.id, position: node.position };
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

    if (!reachable.has(node.id)) {
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

    if (node.type === "route_provider") {
      if (!node.route_provider?.provider_id) {
        issues.push("Provider is required.");
      } else if (
        !config.providers.some(
          (provider) => provider.id === (node.route_provider?.provider_id ?? ""),
        )
      ) {
        issues.push("Provider does not exist.");
      }
    }

    if (node.type === "select_model") {
      if (!node.select_model?.model_id) {
        issues.push("Model is required.");
      } else if (
        !config.models.some((model) => model.id === (node.select_model?.model_id ?? ""))
      ) {
        issues.push("Model does not exist.");
      }
    }

    if (node.type === "rewrite_path" && !node.rewrite_path?.value?.trim()) {
      issues.push("Path rewrite value is required.");
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

    if (issues.length > 0) {
      nodeIssues[node.id] = issues;
    }
  }

  return {
    globalIssues,
    nodeIssues,
    unreachableNodeIds: new Set(
      graph.nodes.filter((node) => !reachable.has(node.id)).map((node) => node.id),
    ),
  };
}

function subtitleForNode(node: RuleGraphNode) {
  switch (node.type) {
    case "condition":
      return node.condition?.mode === "builder"
        ? builderToExpression(node.condition?.builder)
        : node.condition?.expression || "No condition";
    case "route_provider":
      return node.route_provider?.provider_id || "No provider";
    case "select_model":
      return node.select_model?.model_id || "No model";
    case "rewrite_path":
      return node.rewrite_path?.value || "No path";
    case "set_header":
      return `${node.set_header?.name || "Header"} = ${node.set_header?.value || ""}`;
    case "remove_header":
      return node.remove_header?.name || "No header";
    case "copy_header":
      return `${node.copy_header?.from || "from"} -> ${node.copy_header?.to || "to"}`;
    case "set_header_if_absent":
      return `${node.set_header_if_absent?.name || "Header"} = ${node.set_header_if_absent?.value || ""}`;
    case "end":
      return "Terminal";
    default:
      return "Entry";
  }
}

function labelForType(type: RuleGraphNodeType) {
  switch (type) {
    case "start":
      return "Start";
    case "condition":
      return "Condition";
    case "route_provider":
      return "Route Provider";
    case "select_model":
      return "Select Model";
    case "rewrite_path":
      return "Rewrite Path";
    case "set_header":
      return "Set Header";
    case "remove_header":
      return "Remove Header";
    case "copy_header":
      return "Copy Header";
    case "set_header_if_absent":
      return "Set If Absent";
    case "end":
      return "End";
  }
}

function shortLabelForType(type: RuleGraphNodeType) {
  switch (type) {
    case "condition":
      return "If";
    case "route_provider":
      return "Route";
    case "select_model":
      return "Model";
    case "rewrite_path":
      return "Path";
    case "set_header":
      return "Set";
    case "remove_header":
      return "Drop";
    case "copy_header":
      return "Copy";
    case "set_header_if_absent":
      return "Guard";
    case "end":
      return "End";
    case "start":
      return "Start";
  }
}

function iconForLibraryNode(type: RuleGraphNodeType) {
  const iconClass = "h-4 w-4";
  switch (type) {
    case "condition":
      return <GitBranch className={iconClass} />;
    case "route_provider":
      return <Network className={iconClass} />;
    case "select_model":
      return <Split className={iconClass} />;
    case "rewrite_path":
      return <Route className={iconClass} />;
    case "set_header":
      return <Plus className={iconClass} />;
    case "remove_header":
      return <Minus className={iconClass} />;
    case "copy_header":
      return <CopyPlus className={iconClass} />;
    case "set_header_if_absent":
      return <ShieldPlus className={iconClass} />;
    case "end":
      return <Hand className={iconClass} />;
    case "start":
      return <ArrowRightLeft className={iconClass} />;
  }
}

function builderToExpression(
  builder?: { field: string; operator: string; value: string } | null,
) {
  if (!builder) return "Builder is incomplete";
  if (builder.operator === "startsWith") {
    return `${builder.field}.startsWith("${builder.value}")`;
  }
  if (builder.operator === "contains") {
    return `${builder.field}.contains("${builder.value}")`;
  }
  return `${builder.field} ${builder.operator} "${builder.value}"`;
}

function Field({
  label,
  value,
  onChange,
  onCommit,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  onCommit?: () => void;
}) {
  return (
    <div className="space-y-2">
      <Label>{label}</Label>
      <Input
        value={value}
        onChange={(event) => onChange(event.target.value)}
        onBlur={onCommit}
      />
    </div>
  );
}

function TextAreaField({
  label,
  value,
  onChange,
  onCommit,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  onCommit?: () => void;
}) {
  return (
    <div className="space-y-2">
      <Label>{label}</Label>
      <Textarea
        value={value}
        onChange={(event) => onChange(event.target.value)}
        onBlur={onCommit}
      />
    </div>
  );
}

function SelectField({
  label,
  value,
  options,
  onChange,
  placeholder,
  disabled,
}: {
  label: string;
  value: string;
  options: Array<{ value: string; label: string }>;
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
}) {
  return (
    <div className="space-y-2">
      <Label>{label}</Label>
      <select
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
        className="flex h-10 w-full rounded-md border border-oklch(0.923 0.003 48.717) bg-oklch(1 0 0) px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-oklch(0.709 0.01 56.259) disabled:cursor-not-allowed disabled:opacity-50"
      >
        <option value="">{placeholder ?? "Select..."}</option>
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}
