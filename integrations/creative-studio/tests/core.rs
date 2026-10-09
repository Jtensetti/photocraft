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

#[test]
fn trim_rebases_operations_onto_the_same_source_frames() {
    let mut s = setup();
    let st = cmd(
        &mut s,
        "selection.set",
        json!({"clip_id":"clip-1","start":150,"end":250}),
    );
    let id = st["active_clip"]["id"].as_str().unwrap().to_owned();
    cmd(&mut s, "develop.set", json!({"values":{"exposure":1}}));
    cmd(&mut s, "seek", json!({"frame":137}));
    cmd(&mut s, "view.set", json!({"scope":"frame"}));
    cmd(
        &mut s,
        "layer.stroke",
        json!({"stroke":{"color":[1,0,0,1],"size":0.3,"erase":false,"points":[[0.5,0.5,1]]}}),
    );
    let rgba = vec![80; 32 * 24 * 4];
    let before = s.render(32, 24, 137, &rgba).unwrap();
    let st = cmd(&mut s, "clip.trim", json!({"id":id,"start":100,"end":200}));
    assert_eq!(st["source_seconds"], 4.0);
    assert_eq!(st["total_frames"], 100);
    assert_eq!(st["project"]["layers"][0]["scope"]["start"], 37);
    assert_eq!(st["project"]["adjustments"][0]["scope"]["start"], 50);
    assert_eq!(st["project"]["adjustments"][0]["scope"]["end"], 100);
    assert_eq!(before, s.render(32, 24, 37, &rgba).unwrap());
    cmd(&mut s, "undo", json!({}));
    assert_eq!(before, s.render(32, 24, 137, &rgba).unwrap());
    cmd(&mut s, "redo", json!({}));
    assert_eq!(before, s.render(32, 24, 37, &rgba).unwrap());
}

#[test]
fn reorder_keeps_the_active_frame_and_scopes_attached_to_the_clip() {
    let mut s = setup();
    let st = cmd(
        &mut s,
        "asset.add",
        json!({"asset":{"id":"second","name":"Second","kind":"blank","width":32,"height":24,"bytes":0,"source_fps":null},"frames":20}),
    );
    let id = st["active_clip"]["id"].as_str().unwrap().to_owned();
    cmd(&mut s, "seek", json!({"frame":405}));
    cmd(
        &mut s,
        "selection.set",
        json!({"clip_id":id,"start":2,"end":10}),
    );
    cmd(&mut s, "develop.set", json!({"values":{"contrast":20}}));
    let st = cmd(&mut s, "clip.move", json!({"id":id,"index":0}));
    assert_eq!(st["project"]["workspace"]["playhead"], 5);
    assert_eq!(st["local_frame"], 5);
    assert_eq!(st["look"]["contrast"], 20.0);
    assert_eq!(st["project"]["workspace"]["selection"]["clip_id"], id);
}

#[test]
fn undo_preserves_the_presentation_and_repairs_deleted_layer_selection() {
    let mut s = setup();
    cmd(&mut s, "layer.new", json!({}));
    cmd(
        &mut s,
        "view.set",
        json!({"mode":"light","timeline_visible":false,"timeline_height":360,"right_visible":false}),
    );
    let st = cmd(&mut s, "undo", json!({}));
    assert_eq!(st["project"]["workspace"]["mode"], "light");
    assert_eq!(st["project"]["workspace"]["timeline_visible"], false);
    assert_eq!(st["project"]["workspace"]["timeline_height"], 360);
    assert_eq!(st["project"]["workspace"]["selected_layer"], Value::Null);
}

#[test]
fn project_scope_affects_all_frames_and_rejects_unsupported_paint_atomically() {
    let mut s = setup();
    cmd(&mut s, "view.set", json!({"scope":"project"}));
    cmd(&mut s, "develop.set", json!({"values":{"exposure":1}}));
    let rgba = vec![80; 32 * 24 * 4];
    assert_eq!(
        s.render(32, 24, 0, &rgba).unwrap(),
        s.render(32, 24, 399, &rgba).unwrap()
    );
    let saved = s.save().unwrap();
    assert!(s.execute("layer.new", "{}").is_err());
    assert_eq!(s.save().unwrap(), saved);
    assert_eq!(
        cmd(&mut s, "seek", json!({"frame":399}))["look"]["exposure"],
        1.0
    );
}

#[test]
fn photocraft_mask_and_layer_translation_reach_rendered_pixels() {
    let mut s = setup();
    let st = cmd(
        &mut s,
        "layer.stroke",
        json!({"stroke":{"color":[1,0,0,1],"size":0.5,"erase":false,"points":[[0.5,0.5,1]]}}),
    );
    let id = st["project"]["layers"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let rgba: Vec<_> = (0..32 * 24).flat_map(|_| [0, 0, 0, 255]).collect();
    let original = s.render(32, 24, 0, &rgba).unwrap();
    cmd(&mut s, "layer.set", json!({"id":id,"mask":[0,0,0.5,1]}));
    let masked = s.render(32, 24, 0, &rgba).unwrap();
    assert!(original[(12 * 32 + 17) * 4] > 100);
    assert_eq!(masked[(12 * 32 + 17) * 4], 0);
    assert!(masked[(12 * 32 + 14) * 4] > 100);
    cmd(
        &mut s,
        "layer.set",
        json!({"id":id,"mask":null,"offset":[0.4,0]}),
    );
    let moved = s.render(32, 24, 0, &rgba).unwrap();
    assert_eq!(moved[(12 * 32 + 14) * 4], 0);
    assert!(moved[(12 * 32 + 28) * 4] > 100);
}

#[test]
fn older_projects_migrate_defaults_and_invalid_new_commands_are_atomic() {
    let s = setup();
    let mut saved: Value = serde_json::from_str(&s.save().unwrap()).unwrap();
    saved["project"]
        .as_object_mut()
        .unwrap()
        .remove("project_look");
    saved["project"]["clips"][0]
        .as_object_mut()
        .unwrap()
        .remove("volume");
    let mut migrated = Studio::new();
    migrated.open(&saved.to_string()).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&migrated.inspect().unwrap()).unwrap()["active_clip"]["volume"],
        1.0
    );
    let before = migrated.save().unwrap();
    for (c, params) in [
        ("clip.trim", json!({"id":"clip-1","start":10,"end":10})),
        ("clip.trim", json!({"id":"clip-1","start":0,"end":401})),
        ("clip.move", json!({"id":"clip-1","index":999})),
        ("clip.volume", json!({"id":"clip-1","volume":2})),
    ] {
        assert!(migrated.execute(c, &params.to_string()).is_err());
        assert_eq!(migrated.save().unwrap(), before);
    }
}

#[test]
fn text_uses_the_upstream_engine_and_keeps_its_temporal_scope() {
    let mut s = setup();
    cmd(
        &mut s,
        "selection.set",
        json!({"clip_id":"clip-1","start":137,"end":200}),
    );
    cmd(
        &mut s,
        "layer.text",
        json!({"text":{"content":"AI","size":0.25,"color":[1,0,0,1],"position":[0.1,0.7]}}),
    );
    let rgba: Vec<_> = (0..32 * 24).flat_map(|_| [0, 0, 0, 255]).collect();
    let before = s.render(32, 24, 136, &rgba).unwrap();
    let inside = s.render(32, 24, 137, &rgba).unwrap();
    assert_ne!(before, inside);
    assert_eq!(inside, s.render(32, 24, 199, &rgba).unwrap());
    assert_eq!(before, s.render(32, 24, 200, &rgba).unwrap());
    let mut reopened = Studio::new();
    reopened.open(&s.save().unwrap()).unwrap();
    assert_eq!(inside, reopened.render(32, 24, 150, &rgba).unwrap());
}

#[test]
fn explicit_edit_scope_survives_a_changed_workspace_selection() {
    let mut s = setup();
    let original = json!({"clip_id":"clip-1","start":2,"end":7});
    cmd(&mut s, "seek", json!({"frame":90}));
    let st = cmd(
        &mut s,
        "develop.set",
        json!({"scope":original,"values":{"exposure":1}}),
    );
    assert_eq!(st["look"]["exposure"], 0.0);
    assert_eq!(st["project"]["adjustments"][0]["scope"]["end"], 7);
    let saved = s.save().unwrap();
    assert!(
        s.execute(
            "develop.set",
            &json!({"scope":{"clip_id":"clip-1","start":390,"end":500},"values":{"exposure":1}})
                .to_string()
        )
        .is_err()
    );
    assert_eq!(s.save().unwrap(), saved);
}
