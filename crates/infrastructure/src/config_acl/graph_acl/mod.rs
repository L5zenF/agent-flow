mod graph_structure;
mod node_acl;
mod wasm_plugin_acl;

use super::{InfrastructureAclError, RawGatewayConfig, RawRuleGraphConfig};

use self::graph_structure::RuleGraphStructure;
use self::node_acl::RuleGraphNodeAcl;

pub(super) fn validate_rule_graph_subset(
    graph: &RawRuleGraphConfig,
    raw: &RawGatewayConfig,
) -> Result<(), InfrastructureAclError> {
    let structure = RuleGraphStructure::new(graph)?;
    structure.validate()?;
    RuleGraphNodeAcl::new(graph, raw, &structure).validate()
}
