use creative_studio::Studio;
use serde_json::{Value, json};

fn cmd(s: &mut Studio, c: &str, p: Value) -> Value {
    serde_json::from_str(&s.execute(c, &p.to_string()).unwrap()).unwrap()
}
fn setup() -> Studio {
    let mut s = Studio::new();
    cmd(&mut s, "project.new", json!({"width": 32, "height": 24}));
    cmd(
        &mut s,
        "asset.add",
        json!({"asset":{"id":"source","name":"source.mp4","kind":"video","width":32,"height":24,"bytes":100,"source_fps":25},"frames":400}),
    );
    s
}
#[test]
fn interval_is_one_operation_and_half_open() {
    let mut s = setup();
    let st = cmd(&mut s, "seek", json!({"frame":200}));
    let id = &st["active_clip"]["id"];
    cmd(
        &mut s,
        "selection.set",
        json!({"clip_id":id,"start":200,"end":300}),
    );
    cmd(&mut s, "develop.set", json!({"values":{"exposure":1}}));
    let st = cmd(&mut s, "view.set", json!({"mode":"light"}));
    assert_eq!(st["project"]["adjustments"].as_array().unwrap().len(), 1);
    for (f, ev) in [(199, 0.0), (200, 1.0), (299, 1.0), (300, 0.0)] {
        let st = cmd(&mut s, "seek", json!({"frame":f}));
        assert_eq!(st["look"]["exposure"].as_f64(), Some(ev));
    }
    let st = cmd(
        &mut s,
        "view.set",
        json!({"mode":"film","timeline_visible":false}),
    );
    assert_eq!(st["project"]["workspace"]["selection"]["end"], 300);
}
#[test]
fn real_render_is_local_and_undoable_and_survives_reopen() {
    let mut s = setup();
    cmd(&mut s, "seek", json!({"frame":137}));
    cmd(
        &mut s,
        "layer.stroke",
        json!({"stroke":{"color":[1,0,0,1],"size":0.3,"erase":false,"points":[[0.5,0.5,1]]}}),
    );
    let rgba = vec![80; 32 * 24 * 4];
    let original = s.render(32, 24, 136, &rgba).unwrap();
    let changed = s.render(32, 24, 137, &rgba).unwrap();
    assert_ne!(original, changed);
    assert_eq!(original, s.render(32, 24, 138, &rgba).unwrap());
    let saved = s.save().unwrap();
    let mut reopened = Studio::new();
    reopened.open(&saved).unwrap();
    assert_eq!(changed, reopened.render(32, 24, 137, &rgba).unwrap());
    cmd(&mut reopened, "undo", json!({}));
    assert_eq!(original, reopened.render(32, 24, 137, &rgba).unwrap());
    cmd(&mut reopened, "redo", json!({}));
    assert_eq!(changed, reopened.render(32, 24, 137, &rgba).unwrap());
}
#[test]
fn lightcraft_pipeline_changes_pixels_only_inside_range() {
    let mut s = setup();
    let st = cmd(&mut s, "seek", json!({"frame":10}));
    let id = &st["active_clip"]["id"];
    cmd(
        &mut s,
        "selection.set",
        json!({"clip_id":id,"start":10,"end":20}),
    );
    cmd(
        &mut s,
        "develop.set",
        json!({"values":{"exposure":1,"temperature":15}}),
    );
    let rgba: Vec<_> = (0..32 * 24).flat_map(|_| [70, 90, 110, 255]).collect();
    let before = s.render(32, 24, 9, &rgba).unwrap();
    let in_range = s.render(32, 24, 10, &rgba).unwrap();
    assert_ne!(before, in_range);
    assert_eq!(before, s.render(32, 24, 20, &rgba).unwrap());
    assert_eq!(in_range, s.render(32, 24, 19, &rgba).unwrap());
}
#[test]
fn split_preserves_scope_and_source_time() {
    let mut s = setup();
    let st = cmd(&mut s, "view.set", json!({"scope":"clip"}));
    let old = &st["active_clip"]["id"];
    cmd(&mut s, "develop.set", json!({"values":{"exposure":0.5}}));
    cmd(&mut s, "seek", json!({"frame":100}));
    let st = cmd(&mut s, "clip.split", json!({}));
    assert_eq!(st["project"]["clips"].as_array().unwrap().len(), 2);
    assert_ne!(st["active_clip"]["id"], *old);
    assert_eq!(st["source_seconds"], 4.0);
    assert_eq!(st["look"]["exposure"], 0.5);
    assert_eq!(st["total_frames"], 400);
}
#[test]
fn invalid_commands_and_files_are_atomic() {
    let mut s = setup();
    let saved = s.save().unwrap();
    assert!(
        s.execute("develop.set", "{\"values\":{\"exposure\":999}} ")
            .is_err()
    );
    assert_eq!(s.save().unwrap(), saved);
    let mut v: Value = serde_json::from_str(&saved).unwrap();
    v["project"]["fps"]["den"] = json!(0);
    assert!(s.open(&v.to_string()).is_err());
    assert_eq!(s.save().unwrap(), saved);
    assert!(s.render(0, 0, 0, &[]).is_err());
    assert!(s.render(10000, 10000, 0, &[]).is_err());
}
#[test]
fn fractional_frame_rate_has_no_timeline_drift() {
    let mut s = Studio::new();
    let st = cmd(&mut s, "project.new", json!({"fps":29.97002997002997}));
    assert_eq!(st["project"]["fps"]["num"], 30000);
    assert_eq!(st["project"]["fps"]["den"], 1001);
    assert_eq!(
        st["frame_ticks"].as_i64().unwrap() * 30000,
        254_016_000_000_i64 * 1001
    );
}

#[test]
fn white_balance_reaches_the_real_pipeline_and_preserves_alpha() {
    let mut s = setup();
    let rgba: Vec<_> = (0..32 * 24).flat_map(|_| [90, 90, 90, 123]).collect();
    let original = s.render(32, 24, 0, &rgba).unwrap();
    cmd(&mut s, "develop.set", json!({"values":{"temperature":60}}));
    let warm = s.render(32, 24, 0, &rgba).unwrap();
    assert_ne!(original, warm);
    assert!(warm[0] > warm[2]);
    assert_eq!(warm[3], 123);
}

#[test]
fn large_brush_history_stays_portable_and_reopenable() {
    let mut studio = setup();
    let points = vec![[0.1, 0.2, 1.0]; 12_000];
    for _ in 0..30 {
        cmd(
            &mut studio,
            "layer.stroke",
            json!({"stroke":{"color":[1,0,0,1],"size":0.01,"erase":false,"points":points}}),
        );
    }
    let saved = studio.save().unwrap();
    assert!(saved.len() < 32_000_000);
    let mut restored = Studio::new();
    restored.open(&saved).unwrap();
    assert_eq!(restored.inspect().unwrap(), studio.inspect().unwrap());
    assert!(
        serde_json::from_str::<Value>(&saved).unwrap()["undo"]
            .as_array()
            .unwrap()
            .len()
            < 30
    );
    cmd(&mut restored, "undo", json!({}));
    cmd(&mut restored, "redo", json!({}));
    assert_eq!(restored.inspect().unwrap(), studio.inspect().unwrap());
}
