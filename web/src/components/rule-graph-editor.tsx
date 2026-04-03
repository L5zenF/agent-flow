import { useMemo, useState } from "react";
import {
  addEdge,
  applyEdgeChanges,
  applyNodeChanges,
  Background,
  Controls,
  Handle,
  MarkerType,
  MiniMap,
  Position,
  ReactFlow,
  type Connection,
  type Edge,
  type EdgeChange,
  type Node,
  type NodeChange,
  type NodeProps,
} from "reactflow";
import { AlertTriangle, Plus } from "lucide-react";
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

type ValidationResult = {
  globalIssues: string[];
  nodeIssues: Record<string, string[]>;
  unreachableNodeIds: Set<string>;
};

type FlowNodeData = {
  title: string;
  subtitle: string;
  issues: string[];
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

const nodeTypes = {
  start: FlowStartNode,
  end: FlowEndNode,
  condition: FlowConditionNode,
  action: FlowActionNode,
};

export function RuleGraphEditor({ config, setConfig }: Props) {
  const graph = config.rule_graph ?? emptyConfig().rule_graph!;
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(graph.start_node_id);
  const validation = useMemo(() => validateGraph(graph, config), [graph, config]);

  const nodes = useMemo<Node<FlowNodeData>[]>(
    () =>
      graph.nodes.map((node) => {
        const issues = validation.nodeIssues[node.id] ?? [];
        const unreachable = validation.unreachableNodeIds.has(node.id);
        return {
          id: node.id,
          position: node.position,
          type: flowTypeForNode(node.type),
          data: {
            title: titleForNode(node),
            subtitle: subtitleForNode(node),
            issues,
            unreachable,
          },
          selected: node.id === selectedNodeId,
        };
      }),
    [graph.nodes, selectedNodeId, validation.nodeIssues, validation.unreachableNodeIds],
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
    <div className="grid gap-4 lg:grid-cols-[240px_minmax(0,1fr)_340px]">
      <div className="space-y-4">
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

        <Card className="h-fit">
          <div className="mb-3 flex items-center gap-2 font-mono text-xs uppercase tracking-[0.16em] text-zinc-500">
            <AlertTriangle className="h-4 w-4" />
            Validation
          </div>
          {validation.globalIssues.length === 0 ? (
            <div className="rounded-md border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-700">
              Graph structure looks valid.
            </div>
          ) : (
            <div className="space-y-2">
              {validation.globalIssues.map((issue) => (
                <div
                  key={issue}
                  className="rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-800"
                >
                  {issue}
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>

      <Card className="h-[720px] overflow-hidden p-0">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          fitView
          nodeTypes={nodeTypes}
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
            validationIssues={validation.nodeIssues[selectedNode.id] ?? []}
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
  validationIssues,
  onDelete,
}: {
  node: RuleGraphNode;
  config: GatewayConfig;
  setConfig: React.Dispatch<React.SetStateAction<GatewayConfig>>;
  validationIssues: string[];
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

      <div className="space-y-3">
        <Field label="ID" value={node.id} onChange={(value) => updateNode({ ...node, id: value })} />
        <SelectField
          label="Type"
          value={node.type}
          options={[
            "start",
            "condition",
            "route_provider",
            "select_model",
            "rewrite_path",
            "set_header",
            "remove_header",
            "copy_header",
            "set_header_if_absent",
            "end",
          ]}
          onChange={(value) => updateNode(resetNodeType(node, value as RuleGraphNodeType))}
        />
      </div>

      {node.type === "condition" && (
        <div className="space-y-3">
          <SelectField
            label="Mode"
            value={node.condition?.mode ?? "expression"}
            options={["expression", "builder"]}
            onChange={(value) =>
              updateNode({
                ...node,
                condition: {
                  mode: value as "builder" | "expression",
                  expression: node.condition?.expression ?? 'path.startsWith("/v1/")',
                  builder: node.condition?.builder ?? {
                    field: "path",
                    operator: "startsWith",
                    value: "/v1/",
                  },
                },
              })
            }
          />
          {node.condition?.mode === "builder" ? (
            <>
              <SelectField
                label="Field"
                value={node.condition?.builder?.field ?? "path"}
                options={CONDITION_FIELDS}
                onChange={(value) =>
                  updateNode({
                    ...node,
                    condition: {
                      ...node.condition!,
                      builder: {
                        field: value,
                        operator: node.condition?.builder?.operator ?? "startsWith",
                        value: node.condition?.builder?.value ?? "/v1/",
                      },
                    },
                  })
                }
              />
              <SelectField
                label="Operator"
                value={node.condition?.builder?.operator ?? "startsWith"}
                options={CONDITION_OPERATORS}
                onChange={(value) =>
                  updateNode({
                    ...node,
                    condition: {
                      ...node.condition!,
                      builder: {
                        field: node.condition?.builder?.field ?? "path",
                        operator: value,
                        value: node.condition?.builder?.value ?? "/v1/",
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
                        operator: node.condition?.builder?.operator ?? "startsWith",
                        value,
                      },
                    },
                  })
                }
              />
              <div className="rounded-md border border-zinc-200 bg-zinc-50 px-3 py-2 font-mono text-xs text-zinc-600">
                {builderToExpression(node.condition?.builder)}
              </div>
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
        <SelectField
          label="Provider"
          value={node.route_provider?.provider_id ?? ""}
          options={config.providers.map((provider) => provider.id)}
          placeholder="Select provider"
          onChange={(value) =>
            updateNode({
              ...node,
              route_provider: { provider_id: value },
            })
          }
        />
      )}

      {node.type === "select_model" && (
        <SelectField
          label="Model"
          value={node.select_model?.model_id ?? ""}
          options={config.models.map((model) => model.id)}
          placeholder="Select model"
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

function FlowStartNode({ data, selected }: NodeProps<FlowNodeData>) {
  return (
    <div className={nodeClassName(data, selected, true)}>
      <div className="font-medium">{data.title}</div>
      <div className="mt-1 text-[11px] text-zinc-500">{data.subtitle}</div>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}

function FlowEndNode({ data, selected }: NodeProps<FlowNodeData>) {
  return (
    <div className={nodeClassName(data, selected, true)}>
      <Handle type="target" position={Position.Left} />
      <div className="font-medium">{data.title}</div>
      <div className="mt-1 text-[11px] text-zinc-500">{data.subtitle}</div>
    </div>
  );
}

function FlowActionNode({ data, selected }: NodeProps<FlowNodeData>) {
  return (
    <div className={nodeClassName(data, selected, true)}>
      <Handle type="target" position={Position.Left} />
      <div className="font-medium">{data.title}</div>
      <div className="mt-1 text-[11px] text-zinc-500">{data.subtitle}</div>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}

function FlowConditionNode({ data, selected }: NodeProps<FlowNodeData>) {
  return (
    <div className={nodeClassName(data, selected, false)}>
      <Handle type="target" position={Position.Left} />
      <div className="font-medium">{data.title}</div>
      <div className="mt-1 text-[11px] text-zinc-500">{data.subtitle}</div>
      <div className="mt-3 flex justify-between text-[10px] uppercase tracking-[0.12em] text-zinc-500">
        <span>True</span>
        <span>False</span>
      </div>
      <Handle id="true" type="source" position={Position.Right} style={{ top: "38%" }} />
      <Handle id="false" type="source" position={Position.Bottom} style={{ left: "50%" }} />
    </div>
  );
}

function nodeClassName(data: FlowNodeData, selected: boolean, compact: boolean) {
  const issue = data.issues.length > 0;
  const unreachable = data.unreachable;
  return [
    "min-w-[170px] rounded-xl border bg-white px-3 py-2 text-left text-xs text-zinc-900 shadow-sm",
    compact ? "" : "min-w-[190px]",
    selected ? "border-zinc-900 ring-2 ring-zinc-900/10" : "border-zinc-200",
    issue ? "border-amber-400 bg-amber-50" : "",
    unreachable ? "border-rose-300 bg-rose-50" : "",
  ]
    .join(" ")
    .trim();
}

function titleForNode(node: RuleGraphNode) {
  switch (node.type) {
    case "route_provider":
      return node.route_provider?.provider_id || "Route Provider";
    case "select_model":
      return node.select_model?.model_id || "Select Model";
    case "rewrite_path":
      return "Rewrite Path";
    case "set_header":
      return `Set ${node.set_header?.name || "Header"}`;
    case "remove_header":
      return `Remove ${node.remove_header?.name || "Header"}`;
    case "copy_header":
      return `Copy ${node.copy_header?.from || "Header"}`;
    case "set_header_if_absent":
      return `Set If Absent ${node.set_header_if_absent?.name || "Header"}`;
    case "condition":
      return "Condition";
    case "end":
      return "End";
    default:
      return "Start";
  }
}

function subtitleForNode(node: RuleGraphNode) {
  switch (node.type) {
    case "condition":
      return node.condition?.mode === "builder"
        ? builderToExpression(node.condition?.builder)
        : node.condition?.expression || "No condition";
    case "rewrite_path":
      return node.rewrite_path?.value || "No path";
    case "set_header":
      return node.set_header?.value || "No value";
    case "remove_header":
      return "delete header";
    case "copy_header":
      return `${node.copy_header?.from || "from"} -> ${node.copy_header?.to || "to"}`;
    case "set_header_if_absent":
      return node.set_header_if_absent?.value || "No value";
    default:
      return node.type;
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

function flowTypeForNode(type: RuleGraphNodeType) {
  if (type === "start") return "start";
  if (type === "end") return "end";
  if (type === "condition") return "condition";
  return "action";
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
      } else if (!config.providers.some((provider) => provider.id === node.route_provider?.provider_id)) {
        issues.push("Provider does not exist.");
      }
    }

    if (node.type === "select_model") {
      if (!node.select_model?.model_id) {
        issues.push("Model is required.");
      } else if (!config.models.some((model) => model.id === node.select_model?.model_id)) {
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

function SelectField({
  label,
  value,
  options,
  onChange,
  placeholder,
}: {
  label: string;
  value: string;
  options: string[];
  onChange: (value: string) => void;
  placeholder?: string;
}) {
  return (
    <label>
      <Label>{label}</Label>
      <select
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="w-full rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-900 outline-none transition focus:border-zinc-900"
      >
        <option value="">{placeholder ?? "Select..."}</option>
        {options.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </select>
    </label>
  );
}
