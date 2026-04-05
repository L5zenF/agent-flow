export type HeaderValue =
  | { value: string }
  | { value: string; encrypted: boolean; secret_env?: string | null };

export type HeaderConfig = {
  name: string;
  value: HeaderValue;
};

export type ProviderConfig = {
  id: string;
  name: string;
  base_url: string;
  default_headers: HeaderConfig[];
};

export type ModelConfig = {
  id: string;
  name: string;
  provider_id: string;
  description?: string | null;
};

export type RouteConfig = {
  id: string;
  priority: number;
  enabled: boolean;
  matcher: string;
  provider_id: string;
  model_id?: string | null;
  path_rewrite?: string | null;
};

export type HeaderAction =
  | { type: "set"; name: string; value: string }
  | { type: "remove"; name: string }
  | { type: "copy"; from: string; to: string }
  | { type: "set_if_absent"; name: string; value: string };

export type RuleScope = "global" | "provider" | "model" | "route";

export type HeaderRuleConfig = {
  id: string;
  enabled: boolean;
  scope: RuleScope;
  target_id?: string | null;
  when?: string | null;
  actions: HeaderAction[];
};

export type GraphPosition = {
  x: number;
  y: number;
};

export type WasmCapability = "log" | "fs" | "network";

export type WasmPluginManifestSummary = {
  id: string;
  name: string;
  version: string;
  description: string;
  supported_output_ports: string[];
  capabilities: WasmCapability[];
  default_config_schema_hints?: unknown | null;
};

export type RuleGraphNodeType =
  | "start"
  | "condition"
  | "select_model"
  | "rewrite_path"
  | "set_context"
  | "router"
  | "log"
  | "set_header"
  | "remove_header"
  | "copy_header"
  | "set_header_if_absent"
  | "wasm_plugin"
  | "note"
  | "end";

export type ConditionMode = "builder" | "expression";

export type ConditionBuilderConfig = {
  field: string;
  operator: string;
  value: string;
};

export type RuleGraphNode = {
  id: string;
  type: RuleGraphNodeType;
  position: GraphPosition;
  note?: string | null;
  condition?: {
    mode: ConditionMode;
    expression?: string | null;
    builder?: ConditionBuilderConfig | null;
  } | null;
  select_model?: {
    provider_id: string;
    model_id: string;
  } | null;
  rewrite_path?: {
    value: string;
  } | null;
  set_context?: {
    key: string;
    value_template: string;
  } | null;
  router?: {
    rules: Array<{
      id: string;
      clauses: Array<{
        source: string;
        operator: string;
        value: string;
      }>;
      target_node_id: string;
    }>;
    fallback_node_id?: string | null;
  } | null;
  log?: {
    message: string;
  } | null;
  set_header?: {
    name: string;
    value: string;
  } | null;
  remove_header?: {
    name: string;
  } | null;
  copy_header?: {
    from: string;
    to: string;
  } | null;
  set_header_if_absent?: {
    name: string;
    value: string;
  } | null;
  wasm_plugin?: {
    plugin_id: string;
    timeout_ms: number;
    fuel?: number | null;
    max_memory_bytes: number;
    granted_capabilities: WasmCapability[];
    read_dirs: string[];
    write_dirs: string[];
    allowed_hosts: string[];
    config: Record<string, unknown>;
  } | null;
  note_node?: {
    text: string;
  } | null;
};

export type RuleGraphEdge = {
  id: string;
  source: string;
  target: string;
  source_handle?: string | null;
};

export type RuleGraphConfig = {
  version: number;
  start_node_id: string;
  nodes: RuleGraphNode[];
  edges: RuleGraphEdge[];
};

export type GatewayConfig = {
  listen: string;
  admin_listen: string;
  default_secret_env?: string | null;
  providers: ProviderConfig[];
  models: ModelConfig[];
  routes: RouteConfig[];
  header_rules: HeaderRuleConfig[];
  rule_graph?: RuleGraphConfig | null;
};

export type GatewayConfigWire = Omit<GatewayConfig, "providers"> & {
  providers: Array<
    Omit<ProviderConfig, "default_headers"> & {
      default_headers: Array<{
        name: string;
        value: string;
        encrypted?: boolean;
        secret_env?: string | null;
      }>;
    }
  >;
};

export const emptyConfig = (): GatewayConfig => ({
  listen: "127.0.0.1:9001",
  admin_listen: "127.0.0.1:9002",
  default_secret_env: "PROXY_SECRET",
  providers: [],
  models: [],
  routes: [],
  header_rules: [],
  rule_graph: {
    version: 1,
    start_node_id: "start",
    nodes: [
      {
        id: "start",
        type: "start",
        position: { x: 80, y: 160 },
      },
      {
        id: "end",
        type: "end",
        position: { x: 880, y: 160 },
      },
    ],
    edges: [],
  },
});

export function normalizeConfig(input: GatewayConfigWire): GatewayConfig {
  return {
    ...input,
    providers: input.providers.map((provider) => ({
      ...provider,
      default_headers: provider.default_headers.map((header) => ({
        name: header.name,
        value:
          header.encrypted
            ? {
                value: header.value,
                encrypted: true,
                secret_env: header.secret_env ?? null,
              }
            : {
                value: header.value,
              },
      })),
    })),
  };
}

export function toWireConfig(input: GatewayConfig): GatewayConfigWire {
  return {
    ...input,
    providers: input.providers.map((provider) => ({
      ...provider,
      default_headers: provider.default_headers.map((header) => ({
        name: header.name,
        value: header.value.value,
        ...("encrypted" in header.value && header.value.encrypted
          ? {
              encrypted: true,
              secret_env: header.value.secret_env ?? null,
            }
          : {}),
      })),
    })),
  };
}
