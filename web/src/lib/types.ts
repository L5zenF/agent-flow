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

export type GatewayConfig = {
  listen: string;
  admin_listen: string;
  default_secret_env?: string | null;
  providers: ProviderConfig[];
  models: ModelConfig[];
  routes: RouteConfig[];
  header_rules: HeaderRuleConfig[];
};

export const emptyConfig = (): GatewayConfig => ({
  listen: "127.0.0.1:9001",
  admin_listen: "127.0.0.1:9002",
  default_secret_env: "PROXY_SECRET",
  providers: [],
  models: [],
  routes: [],
  header_rules: [],
});
