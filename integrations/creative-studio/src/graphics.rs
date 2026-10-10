//! VectorCraft and DesignCraft are derived, temporal object layers of Studio.
//! Their real command engines and CPU renderers run here; no native project or
//! independent history survives a call. Journal replay keeps geometry editable.
use crate::model::{GraphicLayer, NativeCall, Project, Scope};
use serde_json::{Value, json};

fn supported(id: &str) -> bool {
    ![
        "file.",
        "prefs.",
        "window.",
        "help.",
        "plugins.",
        "interaction.",
        "history.",
        "script.",
        "recovery.",
        "application.",
    ]
    .iter()
    .any(|prefix| id.starts_with(prefix))
        && !matches!(id, "edit.undo" | "edit.redo")
}

pub fn catalog(engine: &str) -> Value {
    let mut result = vec![];
    if engine == "vector" {
        for c in vectorcraft_engine::command_specs() {
            result.push(json!({"id":c.id,"label":c.label,"menu":c.menu,"shortcut":c.shortcut,"params":c.params,"journal":c.journal,"connected":supported(c.id)}));
        }
    } else if engine == "design" {
        for c in designcraft_engine::command_specs() {
            result.push(json!({"id":c.id,"label":c.label,"menu":c.menu,"shortcut":c.shortcut,"params":c.params,"journal":c.undoable,"connected":supported(c.id)}));
        }
    }
    json!(result)
}

fn vector(
    p: &Project,
    layer: Option<&GraphicLayer>,
) -> Result<vectorcraft_engine::Session, String> {
    let mut s = vectorcraft_engine::Session::new();
    s.add_document(
        vectorcraft_engine::doc::Document::new(f64::from(p.width), f64::from(p.height)),
        None,
    );
    if let Some(layer) = layer {
        for call in &layer.calls {
            s.execute(&call.command, &call.params)
                .map_err(|e| format!("VectorCraft · {}: {e}", call.command))?;
        }
    }
    Ok(s)
}
fn design(
    p: &Project,
    layer: Option<&GraphicLayer>,
) -> Result<designcraft_engine::Session, String> {
    let mut s = designcraft_engine::Session::new();
    s.execute(
        "file.new",
        &json!({"width":p.width,"height":p.height,"pages":1,"facingPages":false,"margins":0}),
    )
    .map_err(|e| e.to_string())?;
    if let Some(layer) = layer {
        for call in &layer.calls {
            s.execute(&call.command, &call.params)
                .map_err(|e| format!("DesignCraft · {}: {e}", call.command))?;
        }
    }
    Ok(s)
}
pub fn validate(p: &Project) -> Result<(), String> {
    if p.graphic_layers.len() > 100 {
        return Err("För många grafiklager".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for l in &p.graphic_layers {
        if l.id.is_empty() || !ids.insert(&l.id) {
            return Err("Grafiklagrets ID är ogiltigt eller duplicerat".into());
        }
        let clip = p
            .clips
            .iter()
            .find(|c| c.id == l.scope.clip_id)
            .ok_or("Grafiklagrets klipp saknas")?;
        if !matches!(l.engine.as_str(), "vector" | "design")
            || l.scope.start >= l.scope.end
            || l.scope.end > clip.frames
            || l.calls.len() > 1000
        {
            return Err("Ogiltigt grafiklager eller tidsintervall".into());
        }
        for call in &l.calls {
            if !supported(&call.command)
                || call.command.len() > 200
                || serde_json::to_vec(&call.params)
                    .map_err(|e| e.to_string())?
                    .len()
                    > 2_000_000
            {
                return Err("Grafikoperationen stöds inte".into());
            }
            let exists = if l.engine == "vector" {
                vectorcraft_engine::find_command(&call.command).is_some()
            } else {
                designcraft_engine::find_command(&call.command).is_some()
            };
            if !exists {
                return Err("Grafikoperationen finns inte i originalmotorn".into());
            }
        }
    }
    Ok(())
}
pub fn execute(
    p: &mut Project,
    engine: &str,
    command: &str,
    params: Value,
    scope: Scope,
) -> Result<(Value, bool), String> {
    if !matches!(engine, "vector" | "design") || !supported(command) {
        return Err("Verktyget behöver en separat fil-, enhets- eller desktopadapter".into());
    }
    let creates = command.starts_with("shape.")
        || matches!(
            command,
            "path.create" | "text.create" | "frame.create" | "line.create"
        );
    let target = p.workspace.graphic_target.as_ref().and_then(|id| {
        p.graphic_layers
            .iter()
            .position(|l| &l.id == id && l.engine == engine)
    });
    let target = if creates
        && target.is_some_and(|i| p.graphic_layers.get(i).is_some_and(|l| l.scope != scope))
    {
        None
    } else {
        target
    };
    let layer = target.and_then(|i| p.graphic_layers.get(i));
    if layer.is_some_and(|l| l.scope != scope) {
        return Err("Grafiklagrets omfattning skiljer sig från ditt val. Välj samma intervall eller skapa ett nytt grafiklager.".into());
    }
    let (result, content) = if engine == "vector" {
        let spec = vectorcraft_engine::find_command(command).ok_or("Okänt VectorCraft-verktyg")?;
        let mut s = vector(p, layer)?;
        let before = s.active().map(|d| d.doc.clone());
        let result = s.execute(command, &params).map_err(|e| e.to_string())?;
        let changed = before
            .zip(s.active())
            .is_some_and(|(before, after)| !std::sync::Arc::ptr_eq(&before, &after.doc));
        (result, spec.journal || changed)
    } else {
        let spec = designcraft_engine::find_command(command).ok_or("Okänt DesignCraft-verktyg")?;
        let mut s = design(p, layer)?;
        let before = s.active().map(|d| d.doc.clone());
        let result = s.execute(command, &params).map_err(|e| e.to_string())?;
        let changed = before
            .zip(s.active())
            .is_some_and(|(before, after)| !std::sync::Arc::ptr_eq(&before, &after.doc));
        (result, spec.undoable || changed)
    };
    // Queries don't grow the replay log. Selection and paint defaults must replay
    // before the next edit but are presentation/context, not an undo step.
    let context = command.starts_with("select.")
        || command.starts_with("selection.")
        || command.starts_with("paint.")
        || command.starts_with("tools.")
        || command.starts_with("text.select")
        || matches!(
            command,
            "edit.copy" | "edit.copyMerged" | "edit.copyInPlace"
        );
    if content || context {
        let index = if let Some(i) = target {
            i
        } else {
            let id = p.id("graphic");
            p.graphic_layers.push(GraphicLayer {
                id: id.clone(),
                engine: engine.into(),
                scope,
                calls: vec![],
            });
            p.workspace.graphic_target = Some(id);
            p.graphic_layers.len().saturating_sub(1)
        };
        let l = p
            .graphic_layers
            .get_mut(index)
            .ok_or("Grafiklagret saknas")?;
        l.calls.push(NativeCall {
            command: command.into(),
            params,
        });
    }
    Ok((result, content))
}
pub fn view(p: &Project, engine: &str) -> Result<Value, String> {
    let layer = p.workspace.graphic_target.as_ref().and_then(|id| {
        p.graphic_layers
            .iter()
            .find(|l| &l.id == id && l.engine == engine)
    });
    let mut result = match engine {
        "vector" => vector(p, layer)?
            .execute("document.inspect", &json!({}))
            .map_err(|e| e.to_string())?,
        "design" => design(p, layer)?
            .execute("document.inspect", &json!({}))
            .map_err(|e| e.to_string())?,
        _ => return Err("Okänd grafikmotor".into()),
    };
    result["studio_layer"] = json!(layer.map(|l| &l.id));
    result["studio_scope"] = json!(layer.map(|l| &l.scope));
    Ok(result)
}
thread_local! {
    static RASTERS: std::cell::RefCell<std::collections::VecDeque<(String,Vec<u8>)>> = const {std::cell::RefCell::new(std::collections::VecDeque::new())};
}
pub fn pixels(p: &Project, l: &GraphicLayer, w: u32, h: u32) -> Result<Vec<u8>, String> {
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > 16_777_216 {
        return Err("Grafikbilden är för stor".into());
    }
    // Many selected frames share one immutable object journal. Cache its raster,
    // not copies per frame. Exact keys avoid hash collisions changing output.
    let key = serde_json::to_string(&(&l.engine, &l.calls, p.width, p.height, w, h))
        .map_err(|e| e.to_string())?;
    if let Some(pixels) = RASTERS.with(|cache| {
        cache
            .borrow()
            .iter()
            .find(|(k, _)| k == &key)
            .map(|(_, v)| v.clone())
    }) {
        return Ok(pixels);
    }
    let pixels = render_pixels(p, l, w, h)?;
    let cost = pixels.len() + key.len();
    if cost <= 32_000_000 {
        RASTERS.with(|cache| {
            let mut cache = cache.borrow_mut();
            while cache.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>() + cost > 32_000_000 {
                cache.pop_front();
            }
            cache.push_back((key, pixels.clone()));
        });
    }
    Ok(pixels)
}
fn render_pixels(p: &Project, l: &GraphicLayer, w: u32, h: u32) -> Result<Vec<u8>, String> {
    if l.engine == "vector" {
        let s = vector(p, Some(l))?;
        let doc = &s.active().ok_or("Vektorlagret saknas")?.doc;
        let mut r = vectorcraft_render::Renderer::new();
        let opts = vectorcraft_render::RenderOptions {
            artboards: false,
            background: None,
            ..Default::default()
        };
        Ok(r.render(
            doc,
            w,
            h,
            vectorcraft_geom::Affine::scale_non_uniform(
                f64::from(w) / f64::from(p.width),
                f64::from(h) / f64::from(p.height),
            ),
            &opts,
        )
        .to_straight())
    } else {
        let s = design(p, Some(l))?;
        let doc = &s.active().ok_or("Layoutlagret saknas")?.doc;
        let mut r = designcraft_render::Renderer::new();
        r.threads = 0;
        let opts = designcraft_render::RenderOptions {
            paper: false,
            background: None,
            ..Default::default()
        };
        Ok(r.render(
            doc,
            &s.cache,
            &[designcraft_render::Placed {
                spread: designcraft_engine::doc::SpreadRef::Doc(0),
                xf: designcraft_geom::Affine::IDENTITY,
            }],
            w,
            h,
            designcraft_geom::Affine::scale_non_uniform(
                f64::from(w) / f64::from(p.width),
                f64::from(h) / f64::from(p.height),
            ),
            &opts,
        )
        .to_straight())
    }
}
