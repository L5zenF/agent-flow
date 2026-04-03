import { useMemo, useState } from "react";
import {
  addEdge,
  applyEdgeChanges,
  applyNodeChanges,
  Background,
  Controls,
  MarkerType,
  MiniMap,
  Position,
  ReactFlow,
  type Connection,
  type Edge,
  type EdgeChange,
  type Node,
  type NodeChange,
} from "reactflow";
import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
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

export function RuleGraphEditor({ config, setConfig }: Props) {
  const graph = config.rule_graph ?? emptyConfig().rule_graph!;
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(graph.start_node_id);

  const nodes = useMemo<Node[]>(
    () =>
      graph.nodes.map((node) => ({
        id: node.id,
        position: node.position,
        data: { label: labelForNode(node) },
        type: "default",
        sourcePosition: Position.Right,
        targetPosition: Position.Left,
        style: node.id === selectedNodeId ? selectedStyle : defaultStyle,
      })),
    [graph.nodes, selectedNodeId],
  );

  const edges = useMemo<Edge[]>(
    () =>
      graph.edges.map((edge) => ({
        id: edge.id,
        source: edge.source,
        target: edge.target,
        sourceHandle: edge.source_handle ?? undefined,
        markerEnd: { type: MarkerType.ArrowClosed },
        label: edge.source_handle ?? undefined,
      })),
    [graph.edges],
  );

  const selectedNode = graph.nodes.find((node) => node.id === selectedNodeId) ?? null;

  return (
    <div className="grid gap-4 lg:grid-cols-[220px_minmax(0,1fr)_320px]">
      <Card className="h-fit">
        <div className="mb-3 font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
          Node Library
        </div>
        <div className="space-y-2">
          {NODE_LIBRARY.map((item) => (
            <button
              key={item.type}
              type="button"
              onClick={() => {
                const next = createNode(item.type, graph.nodes.length);
                updateGraph(setConfig, {
                  ...graph,
                  nodes: [...graph.nodes, next],
                });
                setSelectedNodeId(next.id);
              }}
              className="flex w-full items-center justify-between rounded-md border border-zinc-200 bg-zinc-50 px-3 py-2 text-sm text-zinc-700 transition hover:border-zinc-300 hover:bg-white"
            >
              {item.label}
              <Plus className="h-4 w-4" />
            </button>
          ))}
        </div>
      </Card>

      <Card className="h-[720px] overflow-hidden p-0">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          fitView
          onNodesChange={(changes) => {
            const nextNodes = applyVisualNodeChanges(graph.nodes, changes);
            updateGraph(setConfig, { ...graph, nodes: nextNodes });
          }}
          onEdgesChange={(changes) => {
            const nextEdges = applyVisualEdgeChanges(graph.edges, changes);
            updateGraph(setConfig, { ...graph, edges: nextEdges });
          }}
          onConnect={(connection) => {
            const nextEdges = addGraphEdge(graph.edges, connection);
            updateGraph(setConfig, { ...graph, edges: nextEdges });
          }}
          onNodeClick={(_, node) => setSelectedNodeId(node.id)}
        >
          <Background gap={20} size={1} color="#e4e4e7" />
          <MiniMap pannable zoomable />
          <Controls />
        </ReactFlow>
      </Card>

      <Card className="h-fit">
        <div className="mb-3 font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
          Node Properties
        </div>
        {!selectedNode ? (
          <div className="text-sm text-zinc-500">Select a node to edit its properties.</div>
        ) : (
          <NodeInspector
            node={selectedNode}
            config={config}
            setConfig={setConfig}
            onDelete={() => {
              updateGraph(setConfig, {
                ...graph,
                nodes: graph.nodes.filter((item) => item.id !== selectedNode.id),
                edges: graph.edges.filter(
                  (edge) => edge.source !== selectedNode.id && edge.target !== selectedNode.id,
                ),
              });
              setSelectedNodeId(null);
            }}
          />
        )}
      </Card>
    </div>
  );
}

function NodeInspector({
  node,
  config,
  setConfig,
  onDelete,
}: {
  node: RuleGraphNode;
  config: GatewayConfig;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
  onDelete: () => void;
}) {
  const graph = config.rule_graph ?? emptyConfig().rule_graph!;
  const nodeIndex = graph.nodes.findIndex((item) => item.id === node.id);

  const updateNode = (nextNode: RuleGraphNode) => {
    const nextNodes = [...graph.nodes];
    nextNodes[nodeIndex] = nextNode;
    updateGraph(setConfig, { ...graph, nodes: nextNodes });
  };

  return (
    <div className="space-y-4">
      <div className="space-y-3">
        <Field label="ID" value={node.id} onChange={(value) => updateNode({ ...node, id: value })} />
        <Field
          label="Type"
          value={node.type}
          onChange={(value) => updateNode(resetNodeType(node, value as RuleGraphNodeType))}
        />
      </div>

      {node.type === "condition" && (
        <div className="space-y-3">
          <Field
            label="Mode"
            value={node.condition?.mode ?? "expression"}
            onChange={(value) =>
              updateNode({
                ...node,
                condition: {
                  mode: value as "builder" | "expression",
                  expression: node.condition?.expression ?? "",
                  builder: node.condition?.builder ?? {
                    field: "path",
                    operator: "==",
                    value: "/v1/chat/completions",
                  },
                },
              })
            }
          />
          {node.condition?.mode === "builder" ? (
            <>
              <Field
                label="Field"
                value={node.condition?.builder?.field ?? ""}
                onChange={(value) =>
                  updateNode({
                    ...node,
                    condition: {
                      ...node.condition!,
                      builder: {
                        field: value,
                        operator: node.condition?.builder?.operator ?? "==",
                        value: node.condition?.builder?.value ?? "",
                      },
                    },
                  })
                }
              />
              <Field
                label="Operator"
                value={node.condition?.builder?.operator ?? ""}
                onChange={(value) =>
                  updateNode({
                    ...node,
                    condition: {
                      ...node.condition!,
                      builder: {
                        field: node.condition?.builder?.field ?? "path",
                        operator: value,
                        value: node.condition?.builder?.value ?? "",
                      },
                    },
                  })
                }
              />
              <Field
                label="Value"
                value={node.condition?.builder?.value ?? ""}
                onChange={(value) =>
                  updateNode({
                    ...node,
                    condition: {
                      ...node.condition!,
                      builder: {
                        field: node.condition?.builder?.field ?? "path",
                        operator: node.condition?.builder?.operator ?? "==",
                        value,
                      },
                    },
                  })
                }
              />
            </>
          ) : (
            <div>
              <Label>Expression</Label>
              <Textarea
                value={node.condition?.expression ?? ""}
                onChange={(event) =>
                  updateNode({
                    ...node,
                    condition: {
                      mode: "expression",
                      expression: event.target.value,
                      builder: node.condition?.builder ?? null,
                    },
                  })
                }
              />
            </div>
          )}
        </div>
      )}

      {node.type === "route_provider" && (
        <Field
          label="Provider ID"
          value={node.route_provider?.provider_id ?? ""}
          onChange={(value) =>
            updateNode({
              ...node,
              route_provider: { provider_id: value },
            })
          }
        />
      )}

      {node.type === "select_model" && (
        <Field
          label="Model ID"
          value={node.select_model?.model_id ?? ""}
          onChange={(value) =>
            updateNode({
              ...node,
              select_model: { model_id: value },
            })
          }
        />
      )}

      {node.type === "rewrite_path" && (
        <Field
          label="Path"
          value={node.rewrite_path?.value ?? ""}
          onChange={(value) =>
            updateNode({
              ...node,
              rewrite_path: { value },
            })
          }
        />
      )}

      {(node.type === "set_header" || node.type === "set_header_if_absent") && (
        <>
          <Field
            label="Header"
            value={
              node.type === "set_header"
                ? node.set_header?.name ?? ""
                : node.set_header_if_absent?.name ?? ""
            }
            onChange={(value) =>
              updateNode({
                ...node,
                [node.type]:
                  node.type === "set_header"
                    ? { name: value, value: node.set_header?.value ?? "" }
                    : { name: value, value: node.set_header_if_absent?.value ?? "" },
              })
            }
          />
          <Field
            label="Value"
            value={
              node.type === "set_header"
                ? node.set_header?.value ?? ""
                : node.set_header_if_absent?.value ?? ""
            }
            onChange={(value) =>
              updateNode({
                ...node,
                [node.type]:
                  node.type === "set_header"
                    ? { name: node.set_header?.name ?? "", value }
                    : { name: node.set_header_if_absent?.name ?? "", value },
              })
            }
          />
        </>
      )}

      {node.type === "remove_header" && (
        <Field
          label="Header"
          value={node.remove_header?.name ?? ""}
          onChange={(value) =>
            updateNode({
              ...node,
              remove_header: { name: value },
            })
          }
        />
      )}

      {node.type === "copy_header" && (
        <>
          <Field
            label="From"
            value={node.copy_header?.from ?? ""}
            onChange={(value) =>
              updateNode({
                ...node,
                copy_header: {
                  from: value,
                  to: node.copy_header?.to ?? "",
                },
              })
            }
          />
          <Field
            label="To"
            value={node.copy_header?.to ?? ""}
            onChange={(value) =>
              updateNode({
                ...node,
                copy_header: {
                  from: node.copy_header?.from ?? "",
                  to: value,
                },
              })
            }
          />
        </>
      )}

      <Button onClick={onDelete} className="w-full bg-white text-zinc-900">
        Delete Node
      </Button>
    </div>
  );
}

function labelForNode(node: RuleGraphNode) {
  switch (node.type) {
    case "condition":
      return "Condition";
    case "route_provider":
      return `Provider: ${node.route_provider?.provider_id || "unset"}`;
    case "select_model":
      return `Model: ${node.select_model?.model_id || "unset"}`;
    case "rewrite_path":
      return `Rewrite: ${node.rewrite_path?.value || "unset"}`;
    case "set_header":
      return `Set: ${node.set_header?.name || "header"}`;
    case "remove_header":
      return `Remove: ${node.remove_header?.name || "header"}`;
    case "copy_header":
      return `Copy: ${node.copy_header?.from || "from"}`;
    case "set_header_if_absent":
      return `Set If Absent: ${node.set_header_if_absent?.name || "header"}`;
    case "end":
      return "End";
    default:
      return "Start";
  }
}

function createNode(type: RuleGraphNodeType, index: number): RuleGraphNode {
  const base: RuleGraphNode = {
    id: `${type}-${index + 1}`,
    type,
    position: { x: 160 + index * 40, y: 120 + index * 24 },
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

function resetNodeType(node: RuleGraphNode, type: RuleGraphNodeType): RuleGraphNode {
  const next = createNode(type, 0);
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

function applyVisualNodeChanges(nodes: RuleGraphNode[], changes: NodeChange[]) {
  const rfNodes = nodes.map((node) => ({
    id: node.id,
    position: node.position,
    data: {},
  }));
  const next = applyNodeChanges(changes, rfNodes);
  return nodes
    .filter((node) => next.some((rfNode) => rfNode.id === node.id))
    .map((node) => {
      const nextNode = next.find((rfNode) => rfNode.id === node.id);
      return nextNode ? { ...node, position: nextNode.position } : node;
    });
}

function applyVisualEdgeChanges(edges: RuleGraphConfig["edges"], changes: EdgeChange[]) {
  const rfEdges = edges.map((edge) => ({
    id: edge.id,
    source: edge.source,
    target: edge.target,
    sourceHandle: edge.source_handle ?? undefined,
  }));
  const next = applyEdgeChanges(changes, rfEdges);
  return next.map((edge) => ({
    id: edge.id,
    source: edge.source,
    target: edge.target,
    source_handle: edge.sourceHandle ?? null,
  }));
}

function addGraphEdge(edges: RuleGraphConfig["edges"], connection: Connection) {
  const next = addEdge(
    {
      ...connection,
      id: `edge-${edges.length + 1}`,
    },
    edges.map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      sourceHandle: edge.source_handle ?? undefined,
    })),
  );

  return next.map((edge) => ({
    id: edge.id,
    source: edge.source,
    target: edge.target,
    source_handle: edge.sourceHandle ?? null,
  }));
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

const defaultStyle = {
  borderRadius: 14,
  border: "1px solid #d4d4d8",
  background: "#ffffff",
  color: "#18181b",
  fontSize: 12,
  padding: 10,
};

const selectedStyle = {
  ...defaultStyle,
  border: "1px solid #18181b",
  boxShadow: "0 0 0 2px rgba(24,24,27,0.08)",
};
