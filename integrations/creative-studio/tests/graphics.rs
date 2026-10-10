use creative_studio::Studio;
use serde_json::{Value, json};
fn view(s: &Studio) -> Value {
    serde_json::from_str(&s.inspect().unwrap()).unwrap()
}
fn cmd(s: &mut Studio, id: &str, p: Value) {
    s.execute(id, &p.to_string()).unwrap();
}
fn setup() -> Studio {
    let mut s = Studio::new();
    cmd(&mut s, "project.new", json!({"width":64,"height":48}));
    cmd(
        &mut s,
        "asset.add",
        json!({"asset":{"id":"a","name":"Media","kind":"image","width":64,"height":48,"bytes":0,"source_fps":null},"frames":20}),
    );
    s
}
fn pixels(s: &Studio, f: u32) -> Vec<u8> {
    s.render(64, 48, f, &[255, 255, 255, 255].repeat(64 * 48))
        .unwrap()
}
fn native(s: &mut Studio, e: &str, c: &str, p: Value) -> Value {
    serde_json::from_str(
        &s.execute_graphics(e, c, &json!({"params":p}).to_string())
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn vector_and_layout_are_rendered_on_the_same_temporal_canvas_and_history() {
    let mut s = setup();
    let original = pixels(&s, 0);
    cmd(
        &mut s,
        "selection.set",
        json!({"clip_id":"clip-1","start":3,"end":10}),
    );
    cmd(
        &mut s,
        "view.set",
        json!({"scope":"range","playhead":5,"mode":"vector"}),
    );
    native(
        &mut s,
        "vector",
        "paint.setFill",
        json!({"color":"#ff0000"}),
    );
    let v = native(
        &mut s,
        "vector",
        "shape.rectangle",
        json!({"x":4,"y":4,"width":20,"height":20}),
    );
    let object = v["native_result"]["id"].as_u64().unwrap();
    let red = pixels(&s, 5);
    assert!(red[(8 * 64 + 8) * 4] > 240 && red[(8 * 64 + 8) * 4 + 1] < 20);
    assert_eq!(pixels(&s, 2), original);
    assert_eq!(pixels(&s, 10), original);
    native(&mut s, "vector", "select.set", json!({"ids":[object]}));
    native(
        &mut s,
        "vector",
        "object.transform",
        json!({"matrix":[1,0,0,1,8,0]}),
    );
    let moved = pixels(&s, 5);
    assert_ne!(moved, red);
    assert_eq!(pixels(&s, 8), moved);
    cmd(
        &mut s,
        "view.set",
        json!({"mode":"design","graphic_target":null}),
    );
    let frame = native(
        &mut s,
        "design",
        "frame.create",
        json!({"rect":[34,4,54,24],"shape":"rectangle","content":"unassigned"}),
    );
    let inspect: Value = serde_json::from_str(&s.graphics_view("design").unwrap()).unwrap();
    let id = inspect["spreads"][0]["items"][0]["id"].as_u64().unwrap();
    assert!(frame["content_changed"].as_bool().unwrap());
    native(&mut s, "design", "selection.set", json!({"ids":[id]}));
    native(&mut s, "design", "object.fill", json!({"swatch":"[Black]"}));
    let both = pixels(&s, 5);
    assert_ne!(both, moved);
    assert!(both[(8 * 64 + 40) * 4] < 20);
    assert!(both[(8 * 64 + 18) * 4] > 240 && both[(8 * 64 + 18) * 4 + 1] < 20);
    let saved = s.save().unwrap();
    let mut reopened = Studio::new();
    reopened.open(&saved).unwrap();
    assert_eq!(pixels(&reopened, 5), both);
    for mode in ["photo", "light", "film", "vector", "design"] {
        let before = view(&reopened);
        cmd(&mut reopened, "view.set", json!({"mode":mode}));
        assert_eq!(
            view(&reopened)["project"]["workspace"]["selection"],
            before["project"]["workspace"]["selection"]
        );
        assert_eq!(pixels(&reopened, 5), both);
    }
    cmd(&mut reopened, "undo", json!({}));
    assert_eq!(pixels(&reopened, 5), moved);
    cmd(&mut reopened, "redo", json!({}));
    assert_eq!(pixels(&reopened, 5), both);
}
#[test]
fn scopes_never_expand_and_survive_native_razor_and_legacy_trim() {
    let mut s = setup();
    cmd(
        &mut s,
        "selection.set",
        json!({"clip_id":"clip-1","start":3,"end":15}),
    );
    cmd(&mut s, "view.set", json!({"scope":"range","playhead":5}));
    native(
        &mut s,
        "vector",
        "shape.rectangle",
        json!({"x":5,"y":5,"width":20,"height":20}),
    );
    let rendered = pixels(&s, 5);
    cmd(&mut s, "view.set", json!({"scope":"frame"}));
    let before = s.save().unwrap();
    assert!(
        s.execute_graphics(
            "vector",
            "object.transform",
            &json!({"params":{"matrix":[1,0,0,1,2,0]}}).to_string()
        )
        .is_err()
    );
    assert_eq!(s.save().unwrap(), before);
    cmd(&mut s, "seek", json!({"frame":10}));
    cmd(&mut s, "clip.split", json!({}));
    assert_eq!(
        view(&s)["project"]["graphic_layers"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(pixels(&s, 5), rendered);
    assert_eq!(pixels(&s, 12), rendered);
    assert_ne!(pixels(&s, 15), rendered);
    let second = view(&s)["project"]["clips"][1]["id"]
        .as_str()
        .unwrap()
        .to_string();
    cmd(&mut s, "clip.trim", json!({"id":second,"start":1,"end":7}));
    assert_eq!(pixels(&s, 10), rendered);
    assert_ne!(pixels(&s, 14), rendered);
    let saved = s.save().unwrap();
    let mut reopened = Studio::new();
    reopened.open(&saved).unwrap();
    assert_eq!(pixels(&reopened, 10), rendered);
}
#[test]
fn bad_graphics_operations_are_atomic_and_older_projects_still_open() {
    let mut s = setup();
    let original = s.save().unwrap();
    for (engine, id) in [
        ("vector", "file.new"),
        ("design", "file.new"),
        ("vector", "no.such.command"),
    ] {
        assert!(s.execute_graphics(engine, id, "{\"params\":{}}").is_err());
        assert_eq!(s.save().unwrap(), original);
    }
    let mut old: Value = serde_json::from_str(&original).unwrap();
    old["project"]
        .as_object_mut()
        .unwrap()
        .remove("graphic_layers");
    old["project"]["workspace"]
        .as_object_mut()
        .unwrap()
        .remove("graphic_target");
    s.open(&old.to_string()).unwrap();
    assert!(
        view(&s)["project"]["graphic_layers"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
