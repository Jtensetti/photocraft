use creative_studio::Studio;
use serde_json::{Value, json};
fn inspect(s: &Studio) -> Value {
    serde_json::from_str(&s.inspect().unwrap()).unwrap()
}
fn command(s: &mut Studio, id: &str, v: Value) {
    s.execute(id, &v.to_string()).unwrap();
}
fn setup() -> Studio {
    let mut s = Studio::new();
    command(&mut s, "project.new", json!({"width":32,"height":24}));
    command(
        &mut s,
        "asset.add",
        json!({"asset":{"id":"a","name":"Image","kind":"image","width":32,"height":24,"bytes":0,"source_fps":null},"frames":20}),
    );
    s
}
fn pixels() -> Vec<u8> {
    (0..32 * 24)
        .flat_map(|i| [if i % 32 < 16 { 40 } else { 120 }, 80, 90, 255])
        .collect()
}
fn native(s: &mut Studio, engine: &str, id: &str, v: Value) -> Value {
    let frame = inspect(s)["project"]["workspace"]["playhead"]
        .as_u64()
        .unwrap() as u32;
    serde_json::from_str(
        &s.execute_pixels(
            engine,
            id,
            &json!({"params":v}).to_string(),
            32,
            24,
            frame,
            &pixels(),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn native_selection_filter_shape_mask_transform_and_save_are_real_shared_operations() {
    let mut s = setup();
    let original = s.render(32, 24, 0, &pixels()).unwrap();
    native(
        &mut s,
        "photo",
        "select.lasso",
        json!({"points":[[0,0],[16,0],[16,24],[0,24]]}),
    );
    native(&mut s, "photo", "image.adjustments.invert", json!({}));
    let selected = s.render(32, 24, 0, &pixels()).unwrap();
    assert!(selected[0] > 150);
    assert_eq!(&selected[100..104], &original[100..104]);
    assert_eq!(s.render(32, 24, 1, &pixels()).unwrap(), original);
    native(&mut s, "photo", "select.deselect", json!({}));
    let state = native(
        &mut s,
        "photo",
        "shape.create",
        json!({"kind":"rect","rect":[10,5,10,10],"fill":"#ff0000"}),
    );
    assert!(
        state["project"]["workspace"]["native_target"]
            .as_str()
            .unwrap()
            .starts_with("photo-op-")
    );
    native(&mut s, "photo", "layer.layerMask.revealAll", json!({}));
    native(
        &mut s,
        "photo",
        "edit.transform",
        json!({"matrix":[1,0,0,1,3,0]}),
    );
    let painted = s.render(32, 24, 0, &pixels()).unwrap();
    assert!(painted[(8 * 32 + 16) * 4] > 200);
    let saved = s.save().unwrap();
    assert!(
        !serde_json::from_str::<Value>(&saved).unwrap()["project"]
            .as_object()
            .unwrap()
            .contains_key("clips")
    );
    let mut reopened = Studio::new();
    reopened.open(&saved).unwrap();
    assert_eq!(painted, reopened.render(32, 24, 0, &pixels()).unwrap());
    command(&mut reopened, "undo", json!({}));
    assert_ne!(painted, reopened.render(32, 24, 0, &pixels()).unwrap());
}
#[test]
fn full_lightcraft_masks_spots_geometry_and_presets_survive_save() {
    let mut s = setup();
    command(
        &mut s,
        "selection.set",
        json!({"clip_id":"clip-1","start":3,"end":10}),
    );
    command(&mut s, "seek", json!({"frame":5}));
    native(
        &mut s,
        "light",
        "mask.add",
        json!({"kind":"linear","start":[0.2,0.2],"end":[0.2,0.8]}),
    );
    native(
        &mut s,
        "light",
        "mask.adjust",
        json!({"values":{"exposure":1.5}}),
    );
    let changed = s.render(32, 24, 5, &pixels()).unwrap();
    let plain = s.render(32, 24, 10, &pixels()).unwrap();
    assert_ne!(plain, changed);
    assert_eq!(plain, s.render(32, 24, 2, &pixels()).unwrap());
    native(
        &mut s,
        "light",
        "spot.add",
        json!({"mode":"clone","points":[[0.65,0.5]],"size":0.2,"source":[-0.4,0.0]}),
    );
    native(
        &mut s,
        "light",
        "crop.set",
        json!({"rect":[0.1,0.1,0.8,0.8],"angle":0}),
    );
    native(
        &mut s,
        "light",
        "preset.create",
        json!({"name":"Shared","groups":["light"]}),
    );
    let result = s.render(32, 24, 5, &pixels()).unwrap();
    assert_eq!(result.len(), 32 * 24 * 4);
    let saved = s.save().unwrap();
    let mut reopened = Studio::new();
    reopened.open(&saved).unwrap();
    assert_eq!(result, reopened.render(32, 24, 5, &pixels()).unwrap());
    assert!(
        !inspect(&reopened)["project"]["light_presets"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
#[test]
fn native_film_tracks_effects_and_razor_share_scope_and_render() {
    let mut s = setup();
    command(
        &mut s,
        "asset.add",
        json!({"asset":{"id":"b","name":"Overlay","kind":"image","width":32,"height":24,"bytes":0,"source_fps":null},"frames":20}),
    );
    s.execute_film(
        "sequence.addTracks",
        &json!({"video":1,"audio":1}).to_string(),
    )
    .unwrap();
    s.execute_film(
        "timeline.move",
        &json!({"moves":[{"clip":2,"track":"V2","time":0}],"linked":false}).to_string(),
    )
    .unwrap();
    assert_eq!(inspect(&s)["total_frames"], 20);
    let plan: Value = serde_json::from_str(&s.frame_plan(5).unwrap()).unwrap();
    assert_eq!(plan.as_array().unwrap().len(), 2);
    let red: Vec<u8> = (0..32 * 24).flat_map(|_| [200, 0, 0, 255]).collect();
    let green: Vec<u8> = (0..32 * 24).flat_map(|_| [0, 200, 0, 255]).collect();
    let mut data = red.clone();
    data.extend(green.clone());
    let inputs: Vec<_> = plan
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut p = p.clone();
            p["offset"] = json!(i * 32 * 24 * 4);
            p["length"] = json!(32 * 24 * 4);
            p
        })
        .collect();
    let output = s
        .render_sequence(32, 24, 5, &json!(inputs).to_string(), &data)
        .unwrap();
    assert!(output[(12 * 32 + 16) * 4 + 1] > 180);
    assert!(output[(12 * 32 + 16) * 4] < 20);
    s.execute_film(
        "effects.setParam",
        &json!({"clip":2,"effect":"opacity","param":"opacity","value":50}).to_string(),
    )
    .unwrap();
    let mixed = s
        .render_sequence(32, 24, 5, &json!(inputs).to_string(), &data)
        .unwrap();
    assert!(mixed[(12 * 32 + 16) * 4] > 90);
    assert!(mixed[(12 * 32 + 16) * 4 + 1] > 90);
    command(&mut s, "seek", json!({"frame":5}));
    s.execute_film(
        "timeline.razor",
        &json!({"time":inspect(&s)["frame_ticks"].as_i64().unwrap()*5,"clip":2}).to_string(),
    )
    .unwrap();
    let saved = s.save().unwrap();
    let mut reopened = Studio::new();
    reopened.open(&saved).unwrap();
    assert_eq!(
        inspect(&s)["project"]["timeline"],
        inspect(&reopened)["project"]["timeline"]
    );
}
#[test]
fn native_command_failures_do_not_change_content_or_shared_history() {
    let mut s = setup();
    let before = s.save().unwrap();
    for (engine, id, params) in [
        ("photo", "shape.create", json!({"kind":"invented"})),
        ("photo", "filter.nonexistent", json!({})),
        (
            "light",
            "mask.adjust",
            json!({"id":999,"values":{"exposure":2}}),
        ),
        ("light", "crop.set", json!({"rect":[0,1]})),
    ] {
        assert!(
            s.execute_pixels(
                engine,
                id,
                &json!({"params":params}).to_string(),
                32,
                24,
                0,
                &pixels()
            )
            .is_err(),
            "{id}"
        );
        assert_eq!(before, s.save().unwrap());
    }
    assert!(
        s.execute_film(
            "timeline.move",
            &json!({"moves":[{"clip":1,"track":"V99","time":0}]}).to_string()
        )
        .is_err()
    );
    assert_eq!(before, s.save().unwrap());
}

#[test]
fn native_clipboard_and_brush_presets_cross_frames_and_reopen() {
    let mut s = setup();
    let original = s.render(32, 24, 0, &pixels()).unwrap();
    native(
        &mut s,
        "photo",
        "tools.setBrush",
        json!({"size":6,"hardness":1,"opacity":0.8,"color":[1,0,0,1]}),
    );
    native(
        &mut s,
        "photo",
        "brush.presets.save",
        json!({"name":"My brush"}),
    );
    native(&mut s, "photo", "tools.setBrush", json!({"size":1}));
    native(
        &mut s,
        "photo",
        "tools.setBrush",
        json!({"preset":"My brush"}),
    );
    assert_eq!(
        inspect(&s)["project"]["workspace"]["photo_tools"]["brush"]["size"].as_f64(),
        Some(6.0)
    );
    native(
        &mut s,
        "photo",
        "paint.pencil",
        json!({"points":[[8,12,1]]}),
    );
    let painted = s.render(32, 24, 0, &pixels()).unwrap();
    assert_ne!(painted, original);
    native(
        &mut s,
        "photo",
        "select.rect",
        json!({"x":4,"y":8,"width":8,"height":8}),
    );
    native(&mut s, "photo", "edit.copy", json!({}));
    let state = inspect(&s);
    let history = state["can_undo"].clone();
    command(&mut s, "seek", json!({"frame":1}));
    native(&mut s, "photo", "select.deselect", json!({}));
    native(&mut s, "photo", "edit.paste", json!({"inPlace":true}));
    let pasted = s.render(32, 24, 1, &pixels()).unwrap();
    assert_ne!(pasted, original);
    assert_eq!(s.render(32, 24, 0, &pixels()).unwrap(), painted);
    assert_eq!(history, json!(true));
    let saved = s.save().unwrap();
    assert!(!saved.contains("photo_clipboard"));
    let mut reopened = Studio::new();
    reopened.open(&saved).unwrap();
    assert_eq!(reopened.render(32, 24, 1, &pixels()).unwrap(), pasted);
    native(
        &mut reopened,
        "photo",
        "tools.setBrush",
        json!({"preset":"My brush"}),
    );
}

#[test]
fn canvas_tool_coordinates_follow_crop_orientation_and_film_motion() {
    let mut s = setup();
    native(
        &mut s,
        "light",
        "crop.set",
        json!({"rect":[0.25,0.25,0.75,0.75]}),
    );
    let coords: Vec<[f64; 2]> = serde_json::from_str(
        &s.canvas_coordinates("photo", "[[0.25,0.5],[0.5,0.5],[0.75,0.5]]", 32, 24)
            .unwrap(),
    )
    .unwrap();
    assert!((coords[0][0] - 12.0).abs() < 0.01);
    assert!((coords[1][0] - 16.0).abs() < 0.01);
    assert!((coords[2][0] - 20.0).abs() < 0.01);
    s.execute_film(
        "effects.setParam",
        &json!({"clip":1,"effect":"motion","param":"position","value":[20,12]}).to_string(),
    )
    .unwrap();
    let coord: Vec<[f64; 2]> = serde_json::from_str(
        &s.canvas_coordinates("photo", "[[0.625,0.5]]", 32, 24)
            .unwrap(),
    )
    .unwrap();
    assert!((coord[0][0] - 16.0).abs() < 0.01);
    native(
        &mut s,
        "photo",
        "paint.pencil",
        json!({"points":[[16,12,1]],"size":3,"color":"#ff0000"}),
    );
    let view: Value = serde_json::from_str(&s.native_view(32, 24, 0, &pixels()).unwrap()).unwrap();
    assert_eq!(view["width"], 32);
}

#[test]
fn reverse_and_razor_keep_source_scopes_and_layer_targets() {
    let mut s = setup();
    command(&mut s, "seek", json!({"frame":4}));
    command(
        &mut s,
        "layer.stroke",
        json!({"stroke":{"points":[[0.5,0.5,1]],"color":[1,0,0,1],"size":0.2,"erase":false}}),
    );
    let id = inspect(&s)["project"]["workspace"]["selected_layer"]
        .as_str()
        .unwrap()
        .to_owned();
    command(&mut s, "view.set", json!({"native_target":id}));
    native(&mut s, "photo", "image.adjustments.invert", json!({}));
    s.execute_film(
        "clip.speedDuration",
        &json!({"clips":[1],"speed":100,"reverse":true,"ripple":false}).to_string(),
    )
    .unwrap();
    let state = inspect(&s);
    assert_eq!(state["project"]["layers"][0]["scope"]["start"], 15);
    assert_eq!(
        state["project"]["photo_operations"][0]["scope"]["start"],
        15
    );
    command(&mut s, "seek", json!({"frame":10}));
    s.execute_film(
        "timeline.razor",
        &json!({"clip":1,"time":inspect(&s)["frame_ticks"].as_i64().unwrap()*10}).to_string(),
    )
    .unwrap();
    let state = inspect(&s);
    let scope = &state["project"]["photo_operations"][0]["scope"];
    assert_ne!(scope["clip_id"], "clip-1");
    assert_eq!(
        state["project"]["photo_operations"][0]["target"],
        state["project"]["layers"][0]["id"]
    );
    let at = state["project"]["clips"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == scope["clip_id"])
        .unwrap()["start"]
        .as_u64()
        .unwrap()
        + scope["start"].as_u64().unwrap();
    assert_eq!(
        s.render_target(
            scope["clip_id"].as_str().unwrap(),
            32,
            24,
            at as u32,
            &pixels()
        )
        .unwrap()
        .len(),
        32 * 24 * 4
    );
}

#[test]
fn native_audio_mixer_combines_tracks_gain_and_mute_without_audio_in_json() {
    let mut s = setup();
    command(
        &mut s,
        "asset.add",
        json!({"asset":{"id":"sound","name":"Tone.wav","kind":"audio","width":1,"height":1,"bytes":0,"source_fps":null,"sample_rate":48000,"channels":2,"duration":0.8},"frames":20}),
    );
    let plan: Value = serde_json::from_str(&s.audio_plan(0, 1024).unwrap()).unwrap();
    let req = plan[0].clone();
    let n = req["frames"].as_u64().unwrap() as usize;
    let data: Vec<f32> = (0..n * 2).map(|i| if i < n { 0.25 } else { 0.5 }).collect();
    let mut input = req.clone();
    input["offset"] = json!(0);
    input["channels"] = json!(2);
    let result = s
        .mix_audio(0, 1024, &json!([input.clone()]).to_string(), &data)
        .unwrap();
    assert_eq!(result.len(), 2048);
    assert!(result[400] > 0.2 && result[1424] > 0.4);
    let audio = inspect(&s)["project"]["timeline"]["audio_tracks"][0]["items"][0]["id"]
        .as_u64()
        .unwrap();
    s.execute_film(
        "clip.audioGain",
        &json!({"clips":[audio],"db":-6.020599913}).to_string(),
    )
    .unwrap();
    let quieter = s
        .mix_audio(0, 1024, &json!([input.clone()]).to_string(), &data)
        .unwrap();
    assert!((quieter[400] - result[400] / 2.0).abs() < 0.01);
    s.execute_film(
        "timeline.setTrack",
        &json!({"track":"A1","muted":true}).to_string(),
    )
    .unwrap();
    assert!(
        s.mix_audio(0, 1024, &json!([input]).to_string(), &data)
            .unwrap()
            .iter()
            .all(|x| x.abs() < 0.0001)
    );
}

fn film_command(s: &mut Studio, id: &str, params: Value) -> Value {
    serde_json::from_str::<Value>(&s.execute_film(id, &params.to_string()).unwrap()).unwrap()["native_result"].clone()
}
fn generated_frame(s: &Studio, frame: u32) -> Vec<u8> {
    let plan: Value = serde_json::from_str(&s.frame_plan(frame).unwrap()).unwrap();
    let mut packed = Vec::new();
    let mut inputs = Vec::new();
    for req in plan.as_array().unwrap() {
        let mut input = req.clone();
        let data = s
            .asset_clip_frame(
                req["asset_id"].as_str().unwrap(),
                req["clip"].as_str().unwrap(),
                32,
                24,
                req["seconds"].as_f64().unwrap(),
            )
            .unwrap();
        input["offset"] = json!(packed.len());
        input["length"] = json!(data.len());
        packed.extend(data);
        inputs.push(input);
    }
    s.render_sequence(32, 24, frame, &json!(inputs).to_string(), &packed)
        .unwrap()
}
#[test]
fn nesting_retains_frame_operations_and_actual_graphics_render() {
    let mut s = Studio::new();
    command(&mut s, "project.new", json!({"width":32,"height":24}));
    let item = film_command(
        &mut s,
        "file.newColorMatte",
        json!({"color":"#ff0000","seconds":0.8}),
    )["item"]
        .clone();
    film_command(
        &mut s,
        "timeline.place",
        json!({"item":item,"track":"V1","frame":0}),
    );
    let clip = inspect(&s)["project"]["clips"][0]["id"]
        .as_str()
        .unwrap()
        .trim_start_matches("clip-")
        .parse::<u64>()
        .unwrap();
    let before = generated_frame(&s, 0);
    assert!(before[0] > 240 && before[1] < 10);
    command(&mut s, "seek", json!({"frame":5}));
    native(&mut s, "photo", "image.adjustments.invert", json!({}));
    let adjusted = generated_frame(&s, 5);
    assert!(adjusted[0] < 10 && adjusted[1] > 240);
    film_command(&mut s, "timeline.select", json!({"clips":[clip]}));
    film_command(&mut s, "clip.nest", json!({"name":"One shared sequence"}));
    assert_eq!(generated_frame(&s, 0), before);
    assert_eq!(generated_frame(&s, 5), adjusted);
    let saved = s.save().unwrap();
    let mut reopened = Studio::new();
    reopened.open(&saved).unwrap();
    assert_eq!(generated_frame(&reopened, 5), adjusted);
    command(&mut reopened, "seek", json!({"frame":0}));
    film_command(
        &mut reopened,
        "graphics.newShape",
        json!({"shape":"rectangle","position":[8,6],"size":[10,10],"seconds":0.8}),
    );
    film_command(
        &mut reopened,
        "graphics.set",
        json!({"props":{"fill_color":"#00ff00"}}),
    );
    let graphic = generated_frame(&reopened, 0);
    assert!(
        graphic
            .as_chunks::<4>()
            .0
            .iter()
            .any(|px| px[1] > 200 && px[0] < 30),
        "Graphic must add actual green pixels; sample {:?}; max green {}",
        &graphic[0..4],
        graphic
            .as_chunks::<4>()
            .0
            .iter()
            .map(|px| px[1])
            .max()
            .unwrap()
    );
}
#[test]
fn native_transition_has_actual_intermediate_pixels_and_gap_trim_keeps_positions() {
    let mut s = Studio::new();
    command(&mut s, "project.new", json!({"width":32,"height":24}));
    for (color, frame) in [("#ff0000", 0), ("#0000ff", 20)] {
        let item = film_command(
            &mut s,
            "file.newColorMatte",
            json!({"color":color,"seconds":0.8}),
        )["item"]
            .clone();
        film_command(&mut s, "timeline.place", json!({"item":item,"frame":frame}));
    }
    let clip = inspect(&s)["project"]["clips"][1]["id"]
        .as_str()
        .unwrap()
        .trim_start_matches("clip-")
        .parse::<u64>()
        .unwrap();
    film_command(
        &mut s,
        "sequence.applyVideoTransition",
        json!({"clip":clip,"effect":"cross_dissolve","frames":8,"edge":"in"}),
    );
    let dissolve = generated_frame(&s, 20);
    assert!(dissolve[0] > 40 && dissolve[2] > 40, "{dissolve:?}");
    let move_time = inspect(&s)["frame_ticks"].as_i64().unwrap() * 30;
    film_command(
        &mut s,
        "timeline.move",
        json!({"moves":[{"clip":clip,"track":"V1","time":move_time}],"linked":false}),
    );
    command(
        &mut s,
        "clip.trim",
        json!({"id":format!("clip-{clip}"),"start":2,"end":15}),
    );
    assert_eq!(inspect(&s)["project"]["clips"][1]["start"], 30);
}

#[test]
fn native_project_adjustment_preserves_local_masks_and_software_encoder_streams() {
    let mut s = setup();
    native(
        &mut s,
        "light",
        "mask.add",
        json!({"kind":"linear","start":[0.2,0.2],"end":[0.2,0.8]}),
    );
    native(
        &mut s,
        "light",
        "mask.adjust",
        json!({"values":{"exposure":1}}),
    );
    command(&mut s, "view.set", json!({"scope":"project"}));
    native(
        &mut s,
        "light",
        "develop.set",
        json!({"values":{"light.contrast":20}}),
    );
    assert_eq!(
        inspect(&s)["look"]["settings"]["masks"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        inspect(&s)["project"]["project_look"]
            .get("settings_patch")
            .is_some()
    );
    let config = s.encode_video_begin().unwrap();
    assert_eq!(config[0], 1);
    for i in 0..3 {
        let bytes = s.encode_video_frame(i, &pixels()).unwrap();
        assert!(bytes.len() > 5);
        if i == 0 {
            assert_eq!(bytes[0], 1);
        }
    }
    assert!(s.encode_video_frame(6, &pixels()).is_err());
    s.encode_video_end();
    assert!(s.encode_video_frame(3, &pixels()).is_err());
}

#[test]
fn layer_deletion_removes_derived_dependencies_and_explicit_targets_obey_scope() {
    let mut s = setup();
    command(&mut s, "layer.new", json!({}));
    command(&mut s, "layer.new", json!({}));
    let mut saved: Value = serde_json::from_str(&s.save().unwrap()).unwrap();
    saved["project"]["layers"][0]["id"] = json!("layer-1");
    saved["project"]["layers"][1]["id"] = json!("layer-10");
    saved["project"]["workspace"]["selected_layer"] = json!("layer-1");
    s.open(&saved.to_string()).unwrap();
    native(
        &mut s,
        "photo",
        "shape.create",
        json!({"kind":"rect","rect":[4,4,10,10],"fill":"#ff0000"}),
    );
    native(
        &mut s,
        "photo",
        "edit.transform",
        json!({"matrix":[1,0,0,1,2,0]}),
    );
    command(&mut s, "view.set", json!({"native_target":"source"}));
    native(
        &mut s,
        "photo",
        "layer.duplicate",
        json!({"layer":"@layer-10"}),
    );
    command(
        &mut s,
        "view.set",
        json!({"native_target":"source","scope":"clip"}),
    );
    let before = s.save().unwrap();
    assert!(
        s.execute_pixels(
            "photo",
            "layer.duplicate",
            &json!({"params":{"layer":"@layer-10"}}).to_string(),
            32,
            24,
            0,
            &pixels(),
        )
        .is_err()
    );
    assert_eq!(s.save().unwrap(), before);
    command(&mut s, "view.set", json!({"scope":"frame"}));
    let painted = s.render(32, 24, 0, &pixels()).unwrap();
    command(&mut s, "layer.delete", json!({"id":"layer-1"}));
    assert_eq!(
        inspect(&s)["project"]["photo_operations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let remaining = s.render(32, 24, 0, &pixels()).unwrap();
    assert_ne!(painted, remaining);
    let mut reopened = Studio::new();
    reopened.open(&s.save().unwrap()).unwrap();
    assert_eq!(reopened.render(32, 24, 0, &pixels()).unwrap(), remaining);
    command(&mut s, "undo", json!({}));
    assert_eq!(s.render(32, 24, 0, &pixels()).unwrap(), painted);
}
