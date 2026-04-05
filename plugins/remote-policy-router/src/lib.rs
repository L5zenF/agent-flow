#[allow(warnings)]
mod bindings;

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;

use serde::Deserialize;

use bindings::exports::proxy_tools::proxy_node_plugin::node_plugin::{
    ContextEntry, ContextPatch, ContextPatchOp, ExecuteError, ExecuteInput, ExecuteOutput, Guest,
    HeaderOp, JsonDocument, LogEntry, LogLevel, NextPort, RequestHeader,
};

struct Component;

#[derive(Debug, Default, Deserialize)]
struct RouterConfig {
    policy_file: Option<String>,
    policy_url: Option<String>,
    match_header: Option<String>,
    fallback_port: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct PolicyDocument {
    #[serde(default)]
    header_routes: BTreeMap<String, String>,
    #[serde(default)]
    path_prefix_routes: Vec<PathPrefixRoute>,
    #[serde(default)]
    default_port: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PathPrefixRoute {
    prefix: String,
    port: String,
}

#[derive(Debug)]
struct SimpleHttpUrl {
    host: String,
    port: u16,
    path: String,
}

#[derive(Debug)]
struct Decision {
    port: String,
    route_key: String,
    policy_source: String,
}

impl Guest for Component {
    fn execute(input: ExecuteInput) -> Result<ExecuteOutput, ExecuteError> {
        let config = parse_config(input.node_config.config.as_ref())?;
        let fallback_port = config
            .fallback_port
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let match_header = config
            .match_header
            .clone()
            .unwrap_or_else(|| "x-tenant".to_string());

        let mut logs = Vec::new();
        let local_policy = load_local_policy(&config, &mut logs);
        let remote_policy = load_remote_policy(&config, &mut logs);

        let decision = choose_port(
            &input,
            &match_header,
            local_policy.as_ref(),
            remote_policy.as_ref(),
            &fallback_port,
        );

        logs.push(LogEntry {
            level: LogLevel::Info,
            message: format!(
                "remote-policy-router chose port '{}' using route key '{}' from {}",
                decision.port, decision.route_key, decision.policy_source
            ),
        });

        Ok(ExecuteOutput {
            context_patch: Some(ContextPatch {
                ops: vec![
                    ContextPatchOp::Set(ContextEntry {
                        key: "route_key".to_string(),
                        value: decision.route_key.clone(),
                    }),
                    ContextPatchOp::Set(ContextEntry {
                        key: "policy_source".to_string(),
                        value: decision.policy_source.clone(),
                    }),
                ],
            }),
            header_ops: vec![
                HeaderOp::Set(RequestHeader {
                    name: "x-policy-source".to_string(),
                    value: decision.policy_source.clone(),
                }),
                HeaderOp::Set(RequestHeader {
                    name: "x-route-key".to_string(),
                    value: decision.route_key.clone(),
                }),
            ],
            path_rewrite: None,
            next_port: Some(NextPort {
                port: decision.port,
            }),
            logs,
        })
    }
}

fn parse_config(config: Option<&JsonDocument>) -> Result<RouterConfig, ExecuteError> {
    match config {
        Some(document) => serde_json::from_str::<RouterConfig>(&document.json).map_err(|error| {
            ExecuteError::InvalidInput(format!("invalid node config JSON: {error}"))
        }),
        None => Ok(RouterConfig::default()),
    }
}

fn load_local_policy(config: &RouterConfig, logs: &mut Vec<LogEntry>) -> Option<PolicyDocument> {
    let path = config.policy_file.as_deref()?;
    match fs::read_to_string(path) {
        Ok(raw) => match serde_json::from_str::<PolicyDocument>(&raw) {
            Ok(policy) => {
                logs.push(LogEntry {
                    level: LogLevel::Info,
                    message: format!("loaded local policy from '{path}'"),
                });
                Some(policy)
            }
            Err(error) => {
                logs.push(LogEntry {
                    level: LogLevel::Warn,
                    message: format!("failed to parse local policy '{path}': {error}"),
                });
                None
            }
        },
        Err(error) => {
            logs.push(LogEntry {
                level: LogLevel::Warn,
                message: format!("failed to read local policy '{path}': {error}"),
            });
            None
        }
    }
}

fn load_remote_policy(config: &RouterConfig, logs: &mut Vec<LogEntry>) -> Option<PolicyDocument> {
    let url = config.policy_url.as_deref()?;
    match fetch_http_json(url) {
        Ok(raw) => match serde_json::from_str::<PolicyDocument>(&raw) {
            Ok(policy) => {
                logs.push(LogEntry {
                    level: LogLevel::Info,
                    message: format!("loaded remote policy from '{url}'"),
                });
                Some(policy)
            }
            Err(error) => {
                logs.push(LogEntry {
                    level: LogLevel::Warn,
                    message: format!("failed to parse remote policy '{url}': {error}"),
                });
                None
            }
        },
        Err(error) => {
            logs.push(LogEntry {
                level: LogLevel::Warn,
                message: format!("failed to fetch remote policy '{url}': {error}"),
            });
            None
        }
    }
}

fn choose_port(
    input: &ExecuteInput,
    match_header: &str,
    local_policy: Option<&PolicyDocument>,
    remote_policy: Option<&PolicyDocument>,
    fallback_port: &str,
) -> Decision {
    let header_value = input
        .request_headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(match_header))
        .map(|header| header.value.as_str());

    if let Some(route_key) = header_value {
        if let Some(port) = remote_policy
            .and_then(|policy| policy.header_routes.get(route_key))
            .cloned()
        {
            return Decision {
                port,
                route_key: route_key.to_string(),
                policy_source: "remote".to_string(),
            };
        }
        if let Some(port) = local_policy
            .and_then(|policy| policy.header_routes.get(route_key))
            .cloned()
        {
            return Decision {
                port,
                route_key: route_key.to_string(),
                policy_source: "local".to_string(),
            };
        }
    }

    if let Some(route) = remote_policy
        .and_then(|policy| match_path_prefix(policy, &input.current_path))
        .cloned()
    {
        return Decision {
            port: route.port,
            route_key: route.prefix,
            policy_source: "remote".to_string(),
        };
    }

    if let Some(route) = local_policy
        .and_then(|policy| match_path_prefix(policy, &input.current_path))
        .cloned()
    {
        return Decision {
            port: route.port,
            route_key: route.prefix,
            policy_source: "local".to_string(),
        };
    }

    if let Some(port) = remote_policy.and_then(|policy| policy.default_port.clone()) {
        return Decision {
            port,
            route_key: "default".to_string(),
            policy_source: "remote".to_string(),
        };
    }

    if let Some(port) = local_policy.and_then(|policy| policy.default_port.clone()) {
        return Decision {
            port,
            route_key: "default".to_string(),
            policy_source: "local".to_string(),
        };
    }

    Decision {
        port: fallback_port.to_string(),
        route_key: "fallback".to_string(),
        policy_source: "fallback".to_string(),
    }
}

fn match_path_prefix<'a>(policy: &'a PolicyDocument, path: &str) -> Option<&'a PathPrefixRoute> {
    policy
        .path_prefix_routes
        .iter()
        .find(|route| path.starts_with(&route.prefix))
}

fn fetch_http_json(url: &str) -> Result<String, String> {
    let url = parse_http_url(url)?;
    let mut stream = TcpStream::connect((url.host.as_str(), url.port))
        .map_err(|error| format!("connect {}:{} failed: {error}", url.host, url.port))?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept: application/json\r\n\r\n",
        url.path, url.host
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("request write failed: {error}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("response read failed: {error}"))?;

    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "response missing header/body separator".to_string())?;
    let status_line = head
        .lines()
        .next()
        .ok_or_else(|| "response missing status line".to_string())?;

    if !status_line.contains(" 200 ") {
        return Err(format!("unexpected response status: {status_line}"));
    }

    Ok(body.to_string())
}

fn parse_http_url(raw: &str) -> Result<SimpleHttpUrl, String> {
    let without_scheme = raw
        .strip_prefix("http://")
        .ok_or_else(|| "only http:// URLs are supported".to_string())?;
    let (authority, path) = match without_scheme.split_once('/') {
        Some((authority, rest)) => (authority, format!("/{rest}")),
        None => (without_scheme, "/".to_string()),
    };
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => {
            let parsed_port = port
                .parse::<u16>()
                .map_err(|error| format!("invalid URL port '{port}': {error}"))?;
            (host.to_string(), parsed_port)
        }
        None => (authority.to_string(), 80),
    };

    if host.is_empty() {
        return Err("URL host is required".to_string());
    }

    Ok(SimpleHttpUrl { host, port, path })
}

bindings::export!(Component with_types_in bindings);
