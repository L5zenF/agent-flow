wit_bindgen::generate!({
    world: "proxy-node-plugin",
});

use exports::proxy_tools::proxy_node_plugin::node_plugin::{
    ExecuteError, ExecuteInput, ExecuteOutput, Guest,
};

struct Component;

impl Guest for Component {
    fn execute(_input: ExecuteInput) -> Result<ExecuteOutput, ExecuteError> {
        Ok(ExecuteOutput {
            context_patch: None,
            header_ops: Vec::new(),
            path_rewrite: None,
            next_port: None,
            logs: Vec::new(),
        })
    }
}

export!(Component);
