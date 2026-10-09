//! Native engines are derived views, never independently saved projects or histories.
use crate::model::{NativeCall, PhotoClipboard, PhotoOperation, PhotoTools, Project};
use photocraft_doc::{Layer, LayerId};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

pub type Aliases = BTreeMap<String, LayerId>;

fn references(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) if s.starts_with('@') => out.push(s[1..].into()),
        Value::Array(a) => a.iter().for_each(|v| references(v, out)),
        Value::Object(o) => o.values().for_each(|v| references(v, out)),
        _ => {}
    }
}

// Removing a base layer also removes operations that need its derived layers.
// Exact aliases avoid confusing layer-1 with layer-10.
pub fn remove_target(p: &mut Project, target: &str) {
    let mut creators = std::collections::BTreeSet::<String>::new();
    let missing = |alias: &str, creators: &std::collections::BTreeSet<String>| {
        alias == target
            || alias
                .split_once(':')
                .is_some_and(|(id, _)| creators.contains(id))
    };
    p.photo_operations.retain(|op| {
        let mut refs = vec![op.target.clone()];
        references(&op.call.params, &mut refs);
        for call in op.context.iter().chain(&op.selection) {
            references(&call.params, &mut refs);
        }
        if refs.iter().any(|r| missing(r, &creators)) {
            creators.insert(op.id.clone());
            false
        } else {
            true
        }
    });
    for calls in [
        &mut p.workspace.photo_context,
        &mut p.workspace.photo_selection,
    ] {
        calls.retain(|call| {
            let mut refs = vec![];
            references(&call.params, &mut refs);
            !refs.iter().any(|r| missing(r, &creators))
        });
    }
    if p.workspace
        .native_target
        .as_deref()
        .is_some_and(|r| missing(r, &creators))
    {
        p.workspace.native_target = None;
    }
}

// The upstream Session's desktop download destructor asks std::Instant for a
// clock even with zero downloads. WASM has no such clock. Reuse one derived
// command view, rebuilding its catalog and discarding source/history each call.
// It never saves or owns authoritative state; only the Studio project does.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static LIGHT_VIEW: std::cell::RefCell<lightcraft_engine::Session> = std::cell::RefCell::new(lightcraft_engine::Session::new());
}
fn with_light_view<T>(f: impl FnOnce(&mut lightcraft_engine::Session) -> T) -> T {
    #[cfg(target_arch = "wasm32")]
    {
        LIGHT_VIEW.with(|view| {
            let mut s = view.borrow_mut();
            s.catalog = lightcraft_engine::catalog::Catalog::new();
            s.undo.clear();
            s.redo.clear();
            s.journal.clear();
            s.drain_log();
            s.selection = Default::default();
            s.before.clear();
            s.media.clear_sources();
            s.active_mask = None;
            s.active_spot = None;
            s.clipboard = None;
            s.auto_sync = false;
            s.presets = lightcraft_engine::presets::builtin();
            let result = f(&mut s);
            s.catalog = lightcraft_engine::catalog::Catalog::new();
            s.undo.clear();
            s.redo.clear();
            s.journal.clear();
            s.drain_log();
            s.media.clear_sources();
            s.before.clear();
            result
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        f(&mut lightcraft_engine::Session::new())
    }
}

pub fn supported(engine: &str, command: &str) -> bool {
    let family = command.split('.').next().unwrap_or("");
    match engine {
        "photo" => {
            !matches!(
                family,
                "file"
                    | "prefs"
                    | "plugins"
                    | "plugin"
                    | "video"
                    | "window"
                    | "help"
                    | "actions"
                    | "batch"
                    | "device"
                    | "scanner"
            ) && !matches!(
                command,
                "edit.undo" | "edit.redo" | "image.imageSize" | "image.canvasSize"
            ) && !command.contains("generative")
        }
        "light" => {
            matches!(
                family,
                "develop"
                    | "crop"
                    | "curve"
                    | "mask"
                    | "spot"
                    | "redeye"
                    | "geometry"
                    | "profile"
                    | "pointColor"
                    | "preset"
            ) && !matches!(
                command,
                "develop.beginInteraction" | "develop.endInteraction"
            ) && !command.contains("export")
                && !command.contains("import")
        }
        _ => false,
    }
}

pub fn catalog() -> Value {
    let photo:Vec<_> = photocraft_engine::command_specs().iter().map(|c| json!({"id":c.id,"label":c.label,"menu":c.menu,"shortcut":c.shortcut,"params":c.params,"journal":c.journal,"connected":supported("photo",c.id)})).collect();
    let light:Vec<_> = lightcraft_engine::command_specs().iter().map(|c| json!({"id":c.id,"label":c.label,"menu":c.menu,"shortcut":c.shortcut,"params":c.params,"journal":c.journal,"connected":supported("light",c.id)})).collect();
    let film:Vec<_> = filmcraft_engine::command_specs().iter().map(|c| json!({"id":c.id,"label":c.label,"menu":c.menu,"shortcut":c.shortcut,"params":c.params,"journal":c.journal,"connected":crate::film::connected(c.id)})).collect();
    let effects = filmcraft_engine::Session::new(Arc::new(crate::film::BrowserServices))
        .execute("effects.list", json!({"detail":true}))
        .unwrap_or_else(|_| json!([]));
    json!({"photo":photo,"light":light,"film":film,"film_effects":effects})
}

fn ids(layers: &[Layer], out: &mut Vec<LayerId>) {
    for l in layers {
        out.push(l.id);
        if let Some(c) = l.children() {
            ids(c, out);
        }
    }
}
fn resolve(value: &Value, aliases: &Aliases) -> Result<Value, String> {
    match value {
        Value::String(s) if s.starts_with('@') => aliases
            .get(&s[1..])
            .map(|id| json!(id.0))
            .ok_or_else(|| format!("Lagret finns inte på den här bildrutan: {s}")),
        Value::Array(a) => a
            .iter()
            .map(|v| resolve(v, aliases))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(o) => o
            .iter()
            .map(|(k, v)| Ok((k.clone(), resolve(v, aliases)?)))
            .collect::<Result<serde_json::Map<_, _>, String>>()
            .map(Value::Object),
        _ => Ok(value.clone()),
    }
}
fn run(
    s: &mut photocraft_engine::Session,
    call: &NativeCall,
    aliases: &Aliases,
) -> Result<Value, String> {
    s.execute(&call.command, resolve(&call.params, aliases)?)
        .map_err(|e| e.to_string())
}
fn select(
    s: &mut photocraft_engine::Session,
    target: &str,
    aliases: &Aliases,
) -> Result<(), String> {
    let id = *aliases
        .get(target)
        .ok_or_else(|| format!("Mållagret finns inte på bildrutan: {target}"))?;
    let d = s.active_mut().ok_or("Ingen canvas")?;
    if d.doc.layer(id).is_none() {
        return Err("Mållagret har tagits bort".into());
    }
    d.active_layer = Some(id);
    d.selected_layers = vec![id];
    Ok(())
}
fn selection(
    s: &mut photocraft_engine::Session,
    calls: &[NativeCall],
    aliases: &Aliases,
) -> Result<(), String> {
    let d = s.active_mut().ok_or("Ingen canvas")?;
    Arc::make_mut(&mut d.doc).selection = None;
    for call in calls {
        run(s, call, aliases)?;
    }
    Ok(())
}
pub fn context_command(command: &str) -> bool {
    command.starts_with("tools.")
        || command.starts_with("brush.")
        || command.starts_with("mixer.")
        || command.starts_with("swatches.")
        || command.starts_with("presets.")
        || matches!(command, "channel.target" | "type.setDefaults")
}
fn tool_snapshot(s: &photocraft_engine::Session) -> PhotoTools {
    PhotoTools {
        foreground: s.tools.foreground,
        background: s.tools.background,
        brush: s.tools.brush.clone(),
        presets: s.tools.presets.clone(),
        mixer: s.tools.mixer.clone(),
    }
}
fn restore_tools(s: &mut photocraft_engine::Session, t: &PhotoTools) {
    s.tools.foreground = t.foreground;
    s.tools.background = t.background;
    s.tools.brush = t.brush.clone();
    s.tools.presets = t.presets.clone();
    s.tools.mixer = t.mixer.clone();
}
fn context(
    s: &mut photocraft_engine::Session,
    calls: &[NativeCall],
    aliases: &Aliases,
) -> Result<(), String> {
    for call in calls {
        run(s, call, aliases)?;
    }
    Ok(())
}
fn restore_clipboard(s: &mut photocraft_engine::Session, c: &PhotoClipboard) -> Result<(), String> {
    if let (Some(data), Some(bounds)) = (&c.pixels, c.bounds) {
        if data.len() > 16_000_000 {
            return Err("Urklippet är för stort".into());
        }
        let d = photocraft_format::load_from_bytes_with(
            data,
            &photocraft_format::LoadOptions {
                max_manifest_bytes: 4_000_000,
                max_blob_bytes: 67_108_864,
                max_total_bytes: 268_435_456,
                preserve_ids: false,
            },
        )
        .map_err(|e| e.to_string())?;
        let surface = d
            .layers
            .first()
            .and_then(|l| l.surface())
            .ok_or("Urklippet saknar pixlar")?;
        s.clipboard = Some(photocraft_engine::edit_cmds::Clip {
            surface: surface.clone(),
            bounds: photocraft_geom::Rect::new(bounds[0], bounds[1], bounds[2], bounds[3]),
        });
    }
    if let Some(bytes) = &c.style {
        let d = photocraft_format::load_from_bytes_with(
            bytes,
            &photocraft_format::LoadOptions {
                max_manifest_bytes: 4_000_000,
                max_blob_bytes: 67_108_864,
                max_total_bytes: 268_435_456,
                preserve_ids: false,
            },
        )
        .map_err(|e| e.to_string())?;
        let l = d.layers.first().ok_or("Urklippet saknar lagerstil")?;
        s.style_clipboard = Some((l.effects.clone(), l.blend, l.opacity, l.advanced));
    }
    s.path_fill_clipboard = c.fill.clone();
    s.path_stroke_clipboard = c.stroke.clone();
    Ok(())
}
fn capture_clipboard(s: &photocraft_engine::Session) -> Result<PhotoClipboard, String> {
    let (pixels, bounds) = if let Some(c) = &s.clipboard {
        let b = c.surface.content_bounds();
        if u64::from(b.width()) * u64::from(b.height()) > 16_777_216 {
            return Err("Urklippet är för stort".into());
        }
        let mut d = photocraft_doc::Document::new(
            "Urklipp",
            photocraft_geom::Size::new(b.width().max(1), b.height().max(1)),
            c.surface.format().mode,
            c.surface.format().sample,
        );
        d.layers.push(photocraft_doc::Layer::new(
            "Urklipp",
            photocraft_doc::LayerContent::Raster(c.surface.clone()),
        ));
        let bytes =
            photocraft_format::save_to_bytes(&d, &Default::default()).map_err(|e| e.to_string())?;
        if bytes.len() > 16_000_000 {
            return Err("Urklippet är för stort".into());
        }
        (
            Some(Arc::new(bytes)),
            Some([c.bounds.x0, c.bounds.y0, c.bounds.x1, c.bounds.y1]),
        )
    } else {
        (None, None)
    };
    let style = if let Some((effects, blend, opacity, advanced)) = &s.style_clipboard {
        let mut d = photocraft_doc::Document::new(
            "Lagerstil",
            photocraft_geom::Size::new(1, 1),
            photocraft_doc::ColorMode::Rgb,
            photocraft_doc::SampleType::F32,
        );
        let mut l = photocraft_doc::Layer::new(
            "Stil",
            photocraft_doc::LayerContent::Raster(photocraft_raster::Surface::new(
                photocraft_doc::PixelFormat::RGBA32F,
            )),
        );
        l.effects = effects.clone();
        l.blend = *blend;
        l.opacity = *opacity;
        l.advanced = *advanced;
        d.layers.push(l);
        Some(Arc::new(
            photocraft_format::save_to_bytes(&d, &Default::default()).map_err(|e| e.to_string())?,
        ))
    } else {
        None
    };
    Ok(PhotoClipboard {
        pixels,
        bounds,
        style,
        fill: s.path_fill_clipboard.clone(),
        stroke: s.path_stroke_clipboard.clone(),
    })
}
fn apply(
    s: &mut photocraft_engine::Session,
    aliases: &mut Aliases,
    op: &PhotoOperation,
) -> Result<Value, String> {
    if let Some(t) = &op.tools {
        restore_tools(s, t);
    }
    context(s, &op.context, aliases)?;
    if let Some(c) = &op.clipboard {
        restore_clipboard(s, c)?;
    }
    select(s, &op.target, aliases)?;
    selection(s, &op.selection, aliases)?;
    let mut before = vec![];
    ids(&s.active().ok_or("Ingen canvas")?.doc.layers, &mut before);
    let result = run(s, &op.call, aliases)?;
    let size = s.active().ok_or("Ingen canvas")?.doc.size;
    if size.width == 0
        || size.height == 0
        || u64::from(size.width) * u64::from(size.height) > 16_777_216
    {
        return Err("Bildverktyget överskrider canvasens pixelgräns".into());
    }
    let mut after = vec![];
    ids(&s.active().ok_or("Ingen canvas")?.doc.layers, &mut after);
    for (i, id) in after
        .into_iter()
        .filter(|id| !before.contains(id))
        .enumerate()
    {
        aliases.insert(format!("{}:{i}", op.id), id);
    }
    // Native history is discarded with this view. The shared Studio owns undo/redo.
    if let Some(d) = s.active_mut() {
        d.history.max_states = 2;
        d.history.max_bytes = 134_217_728;
    }
    Ok(result)
}

pub fn photo_session(
    p: &Project,
    w: u32,
    h: u32,
    global: u32,
    rgba: &[u8],
) -> Result<(photocraft_engine::Session, Aliases), String> {
    let (doc, mut aliases) = crate::render::document(p, w, h, global, rgba)?;
    let mut s = photocraft_engine::Session::new();
    s.add_document(doc, None);
    if let Some(d) = s.active_mut() {
        d.history.max_states = 2;
        d.history.max_bytes = 134_217_728;
    }
    let (c, f) = p.at(global).ok_or("Ingen bildruta")?;
    for op in &p.photo_operations {
        if op.scope.contains(&c.id, f) {
            apply(&mut s, &mut aliases, op).map_err(|e| format!("{}: {e}", op.call.command))?;
        }
    }
    if let Some(t) = &p.workspace.photo_tools {
        restore_tools(&mut s, t);
    }
    context(&mut s, &p.workspace.photo_context, &aliases)?;
    restore_clipboard(&mut s, &p.workspace.photo_clipboard)?;
    Ok((s, aliases))
}
fn alias_view(mut v: Value, aliases: &Aliases) -> Value {
    fn map(v: &mut Value, aliases: &Aliases) {
        match v {
            Value::Array(a) => {
                for x in a {
                    map(x, aliases);
                }
            }
            Value::Object(o) => {
                for (k, x) in o {
                    if matches!(k.as_str(), "id" | "activeLayer")
                        && let Some(n) = x.as_u64()
                        && let Some((a, _)) = aliases.iter().find(|(_, id)| id.0 == n)
                    {
                        *x = json!(a);
                    } else {
                        map(x, aliases);
                    }
                }
            }
            _ => {}
        }
    }
    map(&mut v, aliases);
    v
}
pub fn photo_view(p: &Project, w: u32, h: u32, global: u32, rgba: &[u8]) -> Result<Value, String> {
    let (mut s, aliases) = photo_session(p, w, h, global, rgba)?;
    let target = p
        .workspace
        .native_target
        .as_deref()
        .or(p.workspace.selected_layer.as_deref())
        .unwrap_or("source");
    if aliases.contains_key(target) {
        select(&mut s, target, &aliases)?;
    }
    selection(&mut s, &p.workspace.photo_selection, &aliases)?;
    let d = s.active().ok_or("Ingen canvas")?;
    let mut view = alias_view(photocraft_engine::inspect::document(d), &aliases);
    if let Some(mask) = &d.doc.selection {
        let size = d.doc.size;
        let mapping = crate::coordinates::Mapping::new(p, global)?;
        let step = (size.width.max(size.height).div_ceil(512)).max(1) as i32;
        let mut edges = Vec::new();
        let selected = |x: i32, y: i32| {
            x >= 0
                && y >= 0
                && x < size.width as i32
                && y < size.height as i32
                && mask.sample_channel(x, y, 0) > 0.5
        };
        'rows: for y in (0..size.height as i32).step_by(step as usize) {
            for x in (0..size.width as i32).step_by(step as usize) {
                if !selected(x, y) {
                    continue;
                }
                for (dx, dy, a, b) in [
                    (-step, 0, [x, y], [x, y + step]),
                    (step, 0, [x + step, y], [x + step, y + step]),
                    (0, -step, [x, y], [x + step, y]),
                    (0, step, [x, y + step], [x + step, y + step]),
                ] {
                    if !selected(x + dx, y + dy) {
                        edges.push([
                            mapping.to_canvas(
                                [f64::from(a[0]), f64::from(a[1])],
                                (size.width, size.height),
                            ),
                            mapping.to_canvas(
                                [f64::from(b[0]), f64::from(b[1])],
                                (size.width, size.height),
                            ),
                        ]);
                        if edges.len() >= 16_384 {
                            break 'rows;
                        }
                    }
                }
            }
        }
        view["selectionOutline"] = json!(edges);
    }
    Ok(view)
}

#[allow(clippy::too_many_arguments)] // One frame and its explicit edit scope cross the worker ABI.
pub fn photo(
    p: &mut Project,
    command: &str,
    params: Value,
    scope: crate::model::Scope,
    w: u32,
    h: u32,
    global: u32,
    rgba: &[u8],
) -> Result<(Value, bool), String> {
    if !supported("photo", command) {
        return Err(
            "Det här kommandot kräver en särskild webbadapter eller den lokala skrivbordsversionen"
                .into(),
        );
    }
    let (mut s, mut aliases) = photo_session(p, w, h, global, rgba)?;
    let target = p
        .workspace
        .native_target
        .clone()
        .or_else(|| p.workspace.selected_layer.clone())
        .unwrap_or_else(|| "source".into());
    let covers = |s: &crate::model::Scope| {
        s.clip_id == scope.clip_id && s.start <= scope.start && s.end >= scope.end
    };
    let mut targets = vec![target.clone()];
    references(&params, &mut targets);
    if targets.iter().any(|target| {
        p.layers
            .iter()
            .find(|l| l.id == *target)
            .is_some_and(|l| !covers(&l.scope))
            || p.photo_operations
                .iter()
                .any(|o| target.starts_with(&format!("{}:", o.id)))
                && !p
                    .photo_operations
                    .iter()
                    .any(|o| target.starts_with(&format!("{}:", o.id)) && covers(&o.scope))
    }) {
        return Err("Mållagrets tidsintervall täcker inte hela den valda omfattningen. Välj ett intervall inom lagret.".into());
    }
    select(&mut s, &target, &aliases)?;
    selection(&mut s, &p.workspace.photo_selection, &aliases)?;
    let call = NativeCall {
        command: command.into(),
        params,
    };
    if command.starts_with("select.") && !matches!(command, "select.float" | "select.drop") {
        let r = run(&mut s, &call, &aliases)?;
        if command == "select.deselect" {
            p.workspace.photo_selection.clear();
            p.workspace.pixel_selection = None;
        } else {
            p.workspace.photo_selection.push(call);
        }
        return Ok((r, false));
    }
    if context_command(command) {
        let r = run(&mut s, &call, &aliases)?;
        if command.starts_with("tools.")
            || command.starts_with("brush.")
            || command.starts_with("mixer.")
        {
            p.workspace.photo_tools = Some(tool_snapshot(&s));
        } else {
            p.workspace.photo_context.push(call);
        }
        return Ok((r, false));
    }
    let spec = photocraft_engine::commands::find(command).ok_or("Okänt originalverktyg")?;
    if !spec.journal
        || matches!(command, "edit.copy" | "edit.copyMerged")
        || command.to_lowercase().contains("copy") && !matches!(command, "layer.viaCopy")
    {
        let r = run(&mut s, &call, &aliases)?;
        if command.to_lowercase().contains("copy") {
            p.workspace.photo_clipboard = capture_clipboard(&s)?;
        }
        if command.starts_with("layer.select")
            && let Some(id) = s.active().and_then(|d| d.active_layer)
            && let Some((key, _)) = aliases.iter().find(|(_, v)| **v == id)
        {
            p.workspace.native_target = Some(key.clone());
        }
        return Ok((r, false));
    }
    let mut spatial = p.workspace.photo_selection.clone();
    if spatial.is_empty()
        && let Some(r) = p.workspace.pixel_selection
    {
        spatial.push(NativeCall{command:"select.rect".into(),params:json!({"x":r[0]*f64::from(w),"y":r[1]*f64::from(h),"width":(r[2]-r[0])*f64::from(w),"height":(r[3]-r[1])*f64::from(h)})});
    }
    let op = PhotoOperation {
        id: p.id("photo-op"),
        scope,
        call,
        target,
        selection: spatial,
        context: p.workspace.photo_context.clone(),
        clipboard: command
            .to_lowercase()
            .contains("paste")
            .then(|| p.workspace.photo_clipboard.clone()),
        tools: p.workspace.photo_tools.clone(),
    };
    let r = apply(&mut s, &mut aliases, &op)?;
    if command == "edit.cut" {
        p.workspace.photo_clipboard = capture_clipboard(&s)?;
    }
    if command == "paint.mixerBrush" {
        p.workspace.photo_tools = Some(tool_snapshot(&s));
    }
    if let Some(id) = s.active().and_then(|d| d.active_layer)
        && let Some((key, _)) = aliases.iter().find(|(_, v)| **v == id)
    {
        p.workspace.native_target = Some(key.clone());
    }
    p.photo_operations.push(op);
    Ok((alias_view(r, &aliases), true))
}

#[allow(clippy::too_many_arguments)] // Same frame ABI as PhotoCraft.
pub fn light(
    p: &mut Project,
    command: &str,
    params: &Value,
    scope: Option<crate::model::Scope>,
    w: u32,
    h: u32,
    global: u32,
    rgba: &[u8],
) -> Result<(Value, bool), String> {
    if !supported("light", command) {
        return Err(
            "Det här kommandot kräver en särskild webbadapter eller den lokala skrivbordsversionen"
                .into(),
        );
    }
    use lightcraft_engine::catalog::{Op, Photo, PhotoId, Source};
    let id = PhotoId(1);
    if scope.is_none()
        && matches!(
            command.split('.').next(),
            Some("mask" | "spot" | "redeye" | "crop" | "geometry")
        )
    {
        return Err("Välj bildruta, intervall eller klipp för lokala masker och geometri".into());
    }
    let (clip, local) = p.at(global).ok_or("Ingen bildruta")?;
    let settings = if scope.is_none() {
        let mut a = crate::model::Adjustments::default();
        a.merge(&p.project_look)?;
        crate::render::settings(&a)?
    } else {
        crate::render::settings(&p.look(&clip.id, local))?
    };
    let mut photo = Photo::new(
        id,
        Source::File {
            path: "canvas".into(),
        },
        "Gemensam canvas",
        "RGBA",
        w,
        h,
        "",
    );
    photo.develop = Arc::new(settings.clone());
    with_light_view(|s| {
        s.catalog
            .apply(Op::AddPhoto {
                photo: Box::new(photo),
            })
            .map_err(|e| e.to_string())?;
        s.execute("library.select", &json!({"ids":[1],"active":1}))
            .map_err(|e| e.to_string())?;
        if !p.light_presets.is_empty() {
            s.presets = p.light_presets.clone();
        }
        s.clipboard = p.develop_clipboard.clone();
        s.active_mask = p.workspace.active_mask;
        s.active_spot = p.workspace.active_spot;
        let (ps, _) = photo_session(p, w, h, global, rgba)?;
        let pixels = crate::render::composite(&ps, w, h)?;
        let matrix = lightcraft_color::SRGB.to_space(&lightcraft_color::REC2020);
        let source = Arc::new(lightcraft_raster::Rgb32f {
            width: w as usize,
            height: h as usize,
            data: pixels
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| {
                    matrix.apply_f32([
                        lightcraft_color::transfer::decode_srgb8(c[0]),
                        lightcraft_color::transfer::decode_srgb8(c[1]),
                        lightcraft_color::transfer::decode_srgb8(c[2]),
                    ])
                })
                .collect(),
        });
        s.media
            .insert(id, lightcraft_engine::SourceLevel::Thumb, source.clone());
        s.media
            .insert(id, lightcraft_engine::SourceLevel::Preview, source.clone());
        s.media
            .insert(id, lightcraft_engine::SourceLevel::Full, source);
        let result = s.execute(command, params).map_err(|e| e.to_string())?;
        p.workspace.active_mask = s.active_mask;
        p.workspace.active_spot = s.active_spot;
        let after = s.develop_of(id).ok_or("Ingen framkallning")?;
        let changed = after.to_json() != settings.to_json();
        let presets_changed = json!(s.presets) != json!(p.light_presets)
            && command.starts_with("preset.")
            && !command.ends_with("apply");
        if presets_changed {
            p.light_presets = s.presets.clone();
        }
        p.develop_clipboard = s.clipboard.take();
        if changed {
            let values = json!({"settings":after});
            if let Some(scope) = scope {
                p.adjustments.retain(|a| a.scope != scope);
                p.adjustments
                    .push(crate::model::Adjustment { scope, values });
            } else {
                fn difference(base: &Value, after: &Value) -> Option<Value> {
                    if base == after {
                        return None;
                    }
                    if let (Some(b), Some(a)) = (base.as_object(), after.as_object()) {
                        let diff: serde_json::Map<_, _> = a
                            .iter()
                            .filter_map(|(key, val)| {
                                difference(b.get(key).unwrap_or(&Value::Null), val)
                                    .map(|v| (key.clone(), v))
                            })
                            .collect();
                        Some(Value::Object(diff))
                    } else {
                        Some(after.clone())
                    }
                }
                let base =
                    crate::render::settings(&crate::model::Adjustments::default())?.to_json();
                p.project_look = json!({"settings_patch":difference(&base,&after.to_json()).unwrap_or_else(||json!({}))});
            }
        }
        Ok((result, changed || presets_changed))
    })
}
