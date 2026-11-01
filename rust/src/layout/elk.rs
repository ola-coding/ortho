//! The Eclipse Layout Kernel, given and giving back the JSON elkjs takes.
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api::layout_value;
use serde_json::Value;

/// `graph` with its nodes, ports, edges and labels placed.
pub fn layout(graph: Value) -> Result<Value, String> {
    layout_value(graph, "{}")
}
