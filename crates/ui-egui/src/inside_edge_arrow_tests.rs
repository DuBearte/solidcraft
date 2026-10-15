//! A fillet or chamfer on an inside (concave) edge adds material in the corner, so its value's
//! arrow points into the corner's open side; on an outside edge it points into the body (#1384).

use serde_json::json;
use solidcraft_engine::geom::Vec3;

use crate::scenario::Harness;

/// An L-shaped block 15 mm tall: its inside vertical edge is at (20, 10), an outside one at
/// (40, 0). The feature `name` rounds or cuts the edge through `edge`.
fn l_step(command: &str, edge: [f64; 3], name: &str) -> Harness {
    let mut h = Harness::new();
    h.app.run("sketch.create", json!({"plane": "XY"})).unwrap();
    h.app.run("sketch.line", json!({"points": [[0, 0], [40, 0], [40, 10], [20, 10], [20, 30], [0, 30]], "closed": true})).unwrap();
    h.app.run("solid.extrude", json!({"distance": 15, "body_name": "Step"})).unwrap();
    let value = if command == "solid.fillet" { json!({"radius": 2}) } else { json!({"distance": 2}) };
    let mut p = json!({"edges": [edge], "name": name});
    p.as_object_mut().unwrap().extend(value.as_object().unwrap().clone());
    h.app.run(command, p).unwrap();
    h.frames(3);
    h.step(&json!({"call": "ui.view", "params": {"view": "iso"}})).unwrap();
    h.step(&json!({"call": "ui.view", "params": {"view": "fit"}})).unwrap();
    h.step(&json!({"call": "ui.editFeature", "params": {"feature": name}})).unwrap();
    let began = std::time::Instant::now();
    while h.app.preview.busy && began.elapsed() < std::time::Duration::from_secs(60) {
        h.frame();
    }
    h.frames(4);
    h
}

fn arrow(h: &Harness) -> (Vec3, Vec3) {
    let d = h.app.dialog.as_ref().expect("the feature's dialog is open");
    let (at, dir) = super::anchor(&h.app, d).expect("the dialog has an anchor");
    (at, dir.expect("the value has an arrow"))
}

#[test]
fn the_arrow_points_into_the_open_corner_at_an_inside_edge() {
    let open = Vec3::new(1.0, 1.0, 0.0).normalized().unwrap();
    let into_body = Vec3::new(-1.0, 1.0, 0.0).normalized().unwrap();
    for command in ["solid.fillet", "solid.chamfer"] {
        // Inside edge: toward (+x, +y), the open side of the corner at (20, 10).
        let h = l_step(command, [20.0, 10.0, 7.5], "Inside");
        let (_, dir) = arrow(&h);
        assert!(dir.dot(open) > 0.99, "{command}, inside edge: {dir:?}");
        // Outside edge at (40, 0): into the body, toward (−x, +y), as before.
        let h = l_step(command, [40.0, 0.0, 7.5], "Outside");
        let (_, dir) = arrow(&h);
        assert!(dir.dot(into_body) > 0.99, "{command}, outside edge: {dir:?}");
    }
}

#[test]
fn dragging_along_the_inside_edge_arrow_increases_the_radius() {
    let mut h = l_step("solid.fillet", [20.0, 10.0, 7.5], "Inside");
    let (at, _) = arrow(&h);
    // Out into the open corner, where the fillet adds its material.
    let open = Vec3::new(1.0, 1.0, 0.0).normalized().unwrap();
    let handle = h.at(&json!({"handle": "arrow"})).expect("the arrow's handle is drawn");
    let further = h.call("ui.at", json!({"world": (at + open * 6.0).to_f32()}))["result"].clone();
    h.step(&json!({"drag": [handle, further]})).unwrap();
    h.frames(2);
    let crate::dialogs::Kind::Fillet { radius, .. } = &h.app.dialog.as_ref().unwrap().kind else { panic!("a fillet dialog") };
    let r: f64 = radius.trim().trim_end_matches("mm").trim().parse().unwrap_or(0.0);
    assert!(r > 2.0 + 1.0, "dragging outward grows the radius: {radius}");
}
