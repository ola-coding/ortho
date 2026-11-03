//! ELK places a small graph where elkjs, which the TypeScript version used, did, on
//! every system ortho ships for. Each fixture holds a graph of the kind ortho gives
//! ELK, and elkjs 0.12.0's result.
use ortho::layout::elk::layout;
use serde_json::{Value, json};

/// Every box and port below `node`: its id, position and size.
fn places(node: &Value, out: &mut Vec<Value>) {
    let place = |n: &Value| json!([n["id"], n["x"], n["y"], n["width"], n["height"]]);
    for child in node["children"].as_array().into_iter().flatten() {
        out.push(place(child));
        out.extend(child["ports"].as_array().into_iter().flatten().map(place));
        places(child, out);
    }
}

/// Every wire: its id, and the coordinates of its route.
fn wires(graph: &Value) -> Vec<Value> {
    let route = |edge: &Value| {
        let section = &edge["sections"][0];
        let mut points = vec![&section["startPoint"]];
        points.extend(section["bendPoints"].as_array().into_iter().flatten());
        points.push(&section["endPoint"]);
        let coordinates: Vec<&Value> = points.into_iter().flat_map(|p| [&p["x"], &p["y"]]).collect();
        json!([edge["id"], coordinates])
    };
    graph["edges"].as_array().into_iter().flatten().map(route).collect()
}

fn assert_close(actual: &Value, expected: &Value, path: &str) {
    match (actual, expected) {
        (Value::Number(a), Value::Number(e)) => {
            let (a, e) = (a.as_f64().unwrap(), e.as_f64().unwrap());
            assert!((a - e).abs() < 1e-6, "{path}: {a}, expected {e}");
        }
        (Value::Array(a), Value::Array(e)) => {
            assert_eq!(a.len(), e.len(), "{path}");
            for (i, (a, e)) in a.iter().zip(e).enumerate() {
                assert_close(a, e, &format!("{path}[{i}]"));
            }
        }
        _ => assert_eq!(actual, expected, "{path}"),
    }
}

fn check(fixture: &str) {
    let fixture: Value = serde_json::from_str(fixture).unwrap();
    let laid_out = layout(fixture["graph"].clone()).unwrap();
    let mut placed = Vec::new();
    places(&laid_out, &mut placed);
    assert_close(&Value::Array(placed), &fixture["places"], "places");
    assert_close(&Value::Array(wires(&laid_out)), &fixture["wires"], "wires");
}

#[test]
fn lays_out_nested_boxes_with_wired_ports_as_elkjs_does() {
    check(include_str!("fixtures/elk-layered.json"));
}

#[test]
fn packs_boxes_as_elkjs_does() {
    check(include_str!("fixtures/elk-packed.json"));
}
