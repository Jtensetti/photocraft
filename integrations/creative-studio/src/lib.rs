#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]

mod film;
pub mod model;
mod render;
use model::*;
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

const PROJECT_BYTE_LIMIT: usize = 15_000_000;
const HISTORY_BYTE_LIMIT: usize = 15_000_000;

fn project_bytes(project: &Project) -> Result<usize, String> {
    serde_json::to_vec(project)
        .map(|v| v.len())
        .map_err(|e| e.to_string())
}

#[wasm_bindgen]
pub struct Studio {
    project: Project,
    undo: Vec<Project>,
    redo: Vec<Project>,
}
impl Default for Studio {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl Studio {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            project: Project::default(),
            undo: vec![],
            redo: vec![],
        }
    }
    pub fn execute(&mut self, command: &str, params: &str) -> Result<String, String> {
        if params.len() > 8_000_000 {
            return Err("Kommandot är för stort".into());
        }
        let v: Value = serde_json::from_str(params).map_err(|e| e.to_string())?;
        self.command(command, &v)?;
        self.inspect()
    }
    pub fn inspect(&self) -> Result<String, String> {
        let mut out = self.project.inspect();
        out["can_undo"] = json!(!self.undo.is_empty());
        out["can_redo"] = json!(!self.redo.is_empty());
        serde_json::to_string(&out).map_err(|e| e.to_string())
    }
    pub fn save(&self) -> Result<String, String> {
        serde_json::to_string(&json!({ "format": "creative-studio", "project": self.project, "undo": self.undo, "redo": self.redo }))
            .map_err(|e| e.to_string())
    }
    pub fn open(&mut self, text: &str) -> Result<String, String> {
        if text.len() > 32_000_000 {
            return Err("Projektfilen är för stor".into());
        }
        let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if value["format"] != "creative-studio" {
            return Err("Detta är ingen Creative Studio-fil".into());
        }
        let project: Project =
            serde_json::from_value(value["project"].clone()).map_err(|e| e.to_string())?;
        let undo: Vec<Project> =
            serde_json::from_value(value["undo"].clone()).map_err(|e| e.to_string())?;
        let redo: Vec<Project> =
            serde_json::from_value(value["redo"].clone()).map_err(|e| e.to_string())?;
        if undo.len() + redo.len() > 50 {
            return Err("För stor historik".into());
        }
        project.validate()?;
        if project_bytes(&project)? > PROJECT_BYTE_LIMIT {
            return Err("Projektets redigeringar är för stora".into());
        }
        let mut history_bytes = 0;
        for p in undo.iter().chain(&redo) {
            p.validate()?;
            history_bytes += project_bytes(p)?;
        }
        if history_bytes > HISTORY_BYTE_LIMIT {
            return Err("Projektets historik är för stor".into());
        }
        self.project = project;
        self.undo = undo;
        self.redo = redo;
        self.inspect()
    }
    /// Called in a worker. The pixels are one decoded frame, never a video file.
    pub fn render(
        &self,
        width: u32,
        height: u32,
        global_frame: u32,
        rgba: &[u8],
    ) -> Result<Vec<u8>, String> {
        render::frame(&self.project, width, height, global_frame, rgba)
    }
}
impl Studio {
    fn command(&mut self, cmd: &str, v: &Value) -> Result<(), String> {
        if cmd == "undo" || cmd == "redo" {
            let (from, to) = if cmd == "undo" {
                (&mut self.undo, &mut self.redo)
            } else {
                (&mut self.redo, &mut self.undo)
            };
            if let Some(p) = from.pop() {
                to.push(self.project.clone());
                let workspace = self.project.workspace.clone();
                self.project = p;
                self.project.workspace = workspace;
                self.repair_view();
            }
            self.trim_history()?;
            return Ok(());
        }
        let mut p = self.project.clone();
        let mut content = true;
        match cmd {
            "project.new" => {
                p = Project::default();
                if let Some(n) = v["name"].as_str() {
                    p.name = n.chars().take(200).collect();
                }
                if let Some(n) = v["width"].as_u64() {
                    p.width = u32::try_from(n).map_err(|_| "Ogiltig bredd")?;
                }
                if let Some(n) = v["height"].as_u64() {
                    p.height = u32::try_from(n).map_err(|_| "Ogiltig höjd")?;
                }
                if let Some(n) = v["fps"].as_f64() {
                    p.fps = filmcraft_time::FrameRate::from_f64(n);
                }
            }
            "project.rename" => {
                p.name = string(v, "name")?.chars().take(200).collect();
            }
            "asset.add" => {
                let mut asset: Asset =
                    serde_json::from_value(v["asset"].clone()).map_err(|e| e.to_string())?;
                let frames = integer(v, "frames")?;
                if p.assets.iter().any(|a| a.id == asset.id) {
                    return Err("Mediet finns redan".into());
                }
                if asset.id.is_empty() {
                    asset.id = p.id("asset");
                }
                let id = p.id("clip");
                p.workspace.playhead = p.frames();
                p.clips.push(Clip {
                    id,
                    asset_id: asset.id.clone(),
                    frames,
                    source_in: 0,
                    muted: false,
                    volume: 1.0,
                });
                p.assets.push(asset);
                p.workspace.selected_layer = None;
                p.workspace.selection = None;
            }
            "view.set" => {
                content = false;
                if let Some(s) = v["mode"].as_str() {
                    p.workspace.mode = s.into();
                }
                if let Some(s) = v["scope"].as_str() {
                    p.workspace.scope = s.into();
                }
                if let Some(b) = v["timeline_visible"].as_bool() {
                    p.workspace.timeline_visible = b;
                }
                if let Some(b) = v["left_visible"].as_bool() {
                    p.workspace.left_visible = b;
                }
                if let Some(b) = v["right_visible"].as_bool() {
                    p.workspace.right_visible = b;
                }
                if let Some(h) = v["timeline_height"].as_u64() {
                    p.workspace.timeline_height = h.clamp(120, 480) as u32;
                }
                if let Some(r) = v.get("pixel_selection") {
                    p.workspace.pixel_selection =
                        serde_json::from_value(r.clone()).map_err(|e| e.to_string())?;
                }
                if v.get("selected_layer").is_some_and(Value::is_null) {
                    p.workspace.selected_layer = None;
                }
                if let Some(id) = v["selected_layer"].as_str() {
                    p.workspace.selected_layer = Some(id.into());
                }
            }
            "seek" => {
                content = false;
                p.workspace.playhead = integer(v, "frame")?.min(p.frames().saturating_sub(1));
            }
            "selection.set" => {
                content = false;
                p.workspace.selection = Some(Scope {
                    clip_id: string(v, "clip_id")?.into(),
                    start: integer(v, "start")?,
                    end: integer(v, "end")?,
                });
                p.workspace.scope = "range".into();
            }
            "develop.set" => {
                Adjustments::default().merge(&v["values"])?;
                let patch = normalize_adjustment_values(&v["values"])?;
                if v["scope"].as_str() == Some("project")
                    || (v.get("scope").is_none() && p.workspace.scope == "project")
                {
                    p.project_look = normalize_adjustment_values(&p.project_look)?;
                    for (key, value) in patch.as_object().ok_or("Ogiltiga justeringar")? {
                        p.project_look[key] = value.clone();
                    }
                } else {
                    let scope = target_scope(&p, v)?;
                    // A gesture updates one shared operation. Move it last so overlap ordering is explicit.
                    let mut values = json!({});
                    if let Some(i) = p.adjustments.iter().position(|a| a.scope == scope) {
                        values = normalize_adjustment_values(&p.adjustments.remove(i).values)?;
                    }
                    for (k, n) in patch.as_object().ok_or("Ogiltiga justeringar")? {
                        values[k] = n.clone();
                    }
                    p.adjustments.push(Adjustment { scope, values });
                }
            }
            "develop.reset" => {
                if v["scope"].as_str() == Some("project")
                    || (v.get("scope").is_none() && p.workspace.scope == "project")
                {
                    p.project_look = json!({});
                } else {
                    let scope = target_scope(&p, v)?;
                    p.adjustments.retain(|a| a.scope != scope);
                }
            }
            "layer.new" => {
                let scope = target_scope(&p, v)?;
                let id = p.id("layer");
                p.layers.push(Layer {
                    id: id.clone(),
                    name: v["name"]
                        .as_str()
                        .unwrap_or("Pensellager")
                        .chars()
                        .take(200)
                        .collect(),
                    scope,
                    opacity: 1.0,
                    visible: true,
                    strokes: vec![],
                    mask: None,
                    text: None,
                    offset: [0.0, 0.0],
                    blend: photocraft_doc::BlendMode::Normal,
                    scale: 1.0,
                    rotation: 0.0,
                });
                p.workspace.selected_layer = Some(id);
            }
            "layer.text" => {
                let scope = target_scope(&p, v)?;
                let text: Text =
                    serde_json::from_value(v["text"].clone()).map_err(|e| e.to_string())?;
                let id = p.id("layer");
                p.layers.push(Layer {
                    id: id.clone(),
                    name: text.content.chars().take(80).collect(),
                    scope,
                    opacity: 1.0,
                    visible: true,
                    strokes: vec![],
                    mask: None,
                    offset: [0.0, 0.0],
                    blend: photocraft_doc::BlendMode::Normal,
                    scale: 1.0,
                    rotation: 0.0,
                    text: Some(text),
                });
                p.workspace.selected_layer = Some(id);
            }
            "layer.stroke" => {
                let stroke: Stroke =
                    serde_json::from_value(v["stroke"].clone()).map_err(|e| e.to_string())?;
                let target_scope = target_scope(&p, v)?;
                let selected = p.workspace.selected_layer.clone();
                let index = p.layers.iter().position(|l| {
                    Some(&l.id) == selected.as_ref() && l.scope == target_scope && l.text.is_none()
                });
                let index = match index {
                    Some(i) => i,
                    None => {
                        let id = p.id("layer");
                        p.workspace.selected_layer = Some(id.clone());
                        p.layers.push(Layer {
                            id,
                            name: "Pensellager".into(),
                            scope: target_scope,
                            opacity: 1.0,
                            visible: true,
                            strokes: vec![],
                            mask: None,
                            text: None,
                            offset: [0.0, 0.0],
                            blend: photocraft_doc::BlendMode::Normal,
                            scale: 1.0,
                            rotation: 0.0,
                        });
                        p.layers.len() - 1
                    }
                };
                let layer = p.layers.get_mut(index).ok_or("Lagret finns inte")?;
                layer.strokes.push(stroke);
            }
            "layer.set" => {
                let id = string(v, "id")?;
                let l = p
                    .layers
                    .iter_mut()
                    .find(|l| l.id == id)
                    .ok_or("Lagret finns inte")?;
                if let Some(b) = v["visible"].as_bool() {
                    l.visible = b;
                }
                if let Some(text) = v.get("text") {
                    l.text = Some(serde_json::from_value(text.clone()).map_err(|e| e.to_string())?);
                }
                if let Some(n) = v["name"].as_str() {
                    l.name = n.chars().take(200).collect();
                }
                if let Some(r) = v.get("mask") {
                    l.mask = serde_json::from_value(r.clone()).map_err(|e| e.to_string())?;
                }
                if let Some(r) = v.get("offset") {
                    l.offset = serde_json::from_value(r.clone()).map_err(|e| e.to_string())?;
                }
                if let Some(b) = v.get("blend") {
                    l.blend = serde_json::from_value(b.clone()).map_err(|e| e.to_string())?;
                }
                if let Some(n) = v["scale"].as_f64() {
                    l.scale = n;
                }
                if let Some(n) = v["rotation"].as_f64() {
                    l.rotation = n;
                }
                if let Some(n) = v["opacity"].as_f64() {
                    l.opacity = n as f32;
                }
            }
            "layer.duplicate" => {
                let id = string(v, "id")?;
                let mut copy = p
                    .layers
                    .iter()
                    .find(|l| l.id == id)
                    .ok_or("Lagret finns inte")?
                    .clone();
                copy.id = p.id("layer");
                copy.name = format!("{} kopia", copy.name);
                p.workspace.selected_layer = Some(copy.id.clone());
                p.layers.push(copy);
            }
            "layer.move" => {
                let id = string(v, "id")?;
                let old = p
                    .layers
                    .iter()
                    .position(|l| l.id == id)
                    .ok_or("Lagret finns inte")?;
                let index = integer(v, "index")? as usize;
                if index >= p.layers.len() {
                    return Err("Lagerpositionen finns inte".into());
                }
                let layer = p.layers.remove(old);
                p.layers.insert(index, layer);
            }
            "layer.delete" => {
                let id = string(v, "id")?;
                p.layers.retain(|l| l.id != id);
                p.workspace.selected_layer = None;
            }
            "layer.scope" => {
                let id = string(v, "id")?;
                let layer = p
                    .layers
                    .iter_mut()
                    .find(|l| l.id == id)
                    .ok_or("Lagret finns inte")?;
                layer.scope.start = integer(v, "start")?;
                layer.scope.end = integer(v, "end")?;
            }
            "clip.volume" => {
                let id = string(v, "id")?;
                let clip = p
                    .clips
                    .iter_mut()
                    .find(|c| c.id == id)
                    .ok_or("Klippet finns inte")?;
                clip.volume = v["volume"].as_f64().ok_or("Ogiltig volym")?;
            }
            "clip.move" => {
                let id = string(v, "id")?;
                let target = integer(v, "index")? as usize;
                if target >= p.clips.len() {
                    return Err("Ogiltig klipposition".into());
                }
                let index = p
                    .clips
                    .iter()
                    .position(|c| c.id == id)
                    .ok_or("Klippet finns inte")?;
                let active = p.at(p.workspace.playhead).map(|(c, f)| (c.id.clone(), f));
                let clip = p.clips.remove(index);
                p.clips.insert(target, clip);
                if let Some((active_id, local)) = active {
                    let offset: u32 = p
                        .clips
                        .iter()
                        .take_while(|c| c.id != active_id)
                        .map(|c| c.frames)
                        .sum();
                    p.workspace.playhead = offset + local;
                }
            }
            "clip.trim" => {
                let id = string(v, "id")?;
                let start = integer(v, "start")?;
                let end = integer(v, "end")?;
                let clip = p
                    .clips
                    .iter_mut()
                    .find(|c| c.id == id)
                    .ok_or("Klippet finns inte")?;
                if start >= end || end > clip.frames {
                    return Err("Trimningen måste behålla minst en bildruta inom klippet".into());
                }
                let (source_in, frames) = film::trim(clip, p.fps, start, end)?;
                clip.source_in = source_in;
                clip.frames = frames;
                p.layers.retain(|l| {
                    l.scope.clip_id != id || l.scope.start < end && l.scope.end > start
                });
                p.adjustments.retain(|l| {
                    l.scope.clip_id != id || l.scope.start < end && l.scope.end > start
                });
                for scope in p
                    .layers
                    .iter_mut()
                    .map(|l| &mut l.scope)
                    .chain(p.adjustments.iter_mut().map(|l| &mut l.scope))
                {
                    if scope.clip_id == id {
                        scope.start = scope.start.max(start) - start;
                        scope.end = scope.end.min(end) - start;
                    }
                }
                let offset: u32 = p
                    .clips
                    .iter()
                    .take_while(|c| c.id != id)
                    .map(|c| c.frames)
                    .sum();
                p.workspace.playhead = offset;
                p.workspace.selection = None;
                if !p
                    .layers
                    .iter()
                    .any(|l| Some(&l.id) == p.workspace.selected_layer.as_ref())
                {
                    p.workspace.selected_layer = None;
                }
            }
            "clip.mute" => {
                let id = string(v, "id")?;
                let c = p
                    .clips
                    .iter_mut()
                    .find(|c| c.id == id)
                    .ok_or("Klippet finns inte")?;
                c.muted = !c.muted;
            }
            "clip.duration" => {
                let id = string(v, "id")?;
                let frames = integer(v, "frames")?;
                let c = p
                    .clips
                    .iter_mut()
                    .find(|c| c.id == id)
                    .ok_or("Klippet finns inte")?;
                let a = p
                    .assets
                    .iter()
                    .find(|a| a.id == c.asset_id)
                    .ok_or("Mediet finns inte")?;
                if a.kind == "video" {
                    return Err(
                        "Använd dela klipp för video; varaktighet gäller stillbilder".into(),
                    );
                }
                c.frames = frames;
                p.layers
                    .retain(|l| l.scope.clip_id != id || l.scope.start < frames);
                p.adjustments
                    .retain(|l| l.scope.clip_id != id || l.scope.start < frames);
                for s in p
                    .layers
                    .iter_mut()
                    .map(|l| &mut l.scope)
                    .chain(p.adjustments.iter_mut().map(|l| &mut l.scope))
                {
                    if s.clip_id == id {
                        s.end = s.end.min(frames);
                    }
                }
                p.workspace.selection = None;
                p.workspace.playhead = p.workspace.playhead.min(p.frames().saturating_sub(1));
            }
            "clip.split" => {
                let (c, local) = p.at(p.workspace.playhead).ok_or("Inget aktivt klipp")?;
                if local == 0 {
                    return Err("Flytta till en bildruta inne i klippet först".into());
                }
                let old = c.id.clone();
                let new = p.id("clip");
                let index = p
                    .clips
                    .iter()
                    .position(|c| c.id == old)
                    .ok_or("Klippet saknas")?;
                let c = p.clips.get_mut(index).ok_or("Klippet saknas")?;
                let mut right = c.clone();
                right.id = new.clone();
                right.frames -= local;
                right.source_in += local;
                c.frames = local;
                p.clips.insert(index + 1, right);
                let mut new_layers = vec![];
                for l in &mut p.layers {
                    if l.scope.clip_id == old && l.scope.end > local {
                        let mut r = l.clone();
                        r.id = format!("{}-{}", l.id, new);
                        r.scope.clip_id = new.clone();
                        r.scope.start = r.scope.start.saturating_sub(local);
                        r.scope.end -= local;
                        new_layers.push(r);
                        l.scope.end = l.scope.end.min(local);
                    }
                }
                p.layers.retain(|l| l.scope.start < l.scope.end);
                p.layers.extend(new_layers);
                let mut new_edits = vec![];
                for e in &mut p.adjustments {
                    if e.scope.clip_id == old && e.scope.end > local {
                        let mut r = e.clone();
                        r.scope.clip_id = new.clone();
                        r.scope.start = r.scope.start.saturating_sub(local);
                        r.scope.end -= local;
                        new_edits.push(r);
                        e.scope.end = e.scope.end.min(local);
                    }
                }
                p.adjustments.retain(|e| e.scope.start < e.scope.end);
                p.adjustments.extend(new_edits);
                p.workspace.selection = None;
            }
            _ => return Err(format!("Kommandot stöds inte: {cmd}")),
        }
        p.validate()?;
        if content && project_bytes(&p)? > PROJECT_BYTE_LIMIT {
            return Err("Projektets redigeringar är för stora. Ta en säkerhetskopia och börja ett nytt projekt.".into());
        }
        if content {
            if cmd == "project.new" {
                self.undo.clear();
                self.redo.clear();
            } else {
                self.undo.push(self.project.clone());
                self.redo.clear();
                if self.undo.len() > 30 {
                    self.undo.remove(0);
                }
            }
        }
        self.project = p;
        if content {
            self.trim_history()?;
        }
        Ok(())
    }

    fn repair_view(&mut self) {
        let p = &mut self.project;
        p.workspace.playhead = p.workspace.playhead.min(p.frames().saturating_sub(1));
        if p.workspace.selection.as_ref().is_some_and(|s| {
            !p.clips
                .iter()
                .any(|c| c.id == s.clip_id && s.end <= c.frames)
        }) {
            p.workspace.selection = None;
            if p.workspace.scope == "range" {
                p.workspace.scope = "frame".into();
            }
        }
        if !p
            .layers
            .iter()
            .any(|l| Some(&l.id) == p.workspace.selected_layer.as_ref())
        {
            p.workspace.selected_layer = None;
        }
    }

    fn trim_history(&mut self) -> Result<(), String> {
        let undo_sizes: Vec<_> = self
            .undo
            .iter()
            .map(project_bytes)
            .collect::<Result<_, _>>()?;
        let redo_sizes: Vec<_> = self
            .redo
            .iter()
            .map(project_bytes)
            .collect::<Result<_, _>>()?;
        let mut total: usize = undo_sizes.iter().chain(&redo_sizes).sum();
        let (mut u, mut r) = (0, 0);
        // Discard the oldest snapshots first. The current project and the
        // closest undo/redo states stay intact whenever they fit the budget.
        while total > HISTORY_BYTE_LIMIT {
            if let Some(size) = undo_sizes.get(u) {
                total -= size;
                u += 1;
            } else if let Some(size) = redo_sizes.get(r) {
                total -= size;
                r += 1;
            } else {
                break;
            }
        }
        self.undo.drain(..u);
        self.redo.drain(..r);
        Ok(())
    }
}
fn string<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str().ok_or_else(|| format!("{k} saknas"))
}
fn integer(v: &Value, k: &str) -> Result<u32, String> {
    v[k].as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| format!("{k} måste vara ett positivt heltal"))
}

fn target_scope(p: &Project, v: &Value) -> Result<Scope, String> {
    if let Some(scope) = v.get("scope") {
        serde_json::from_value(scope.clone()).map_err(|_| "Ogiltig redigeringsomfattning".into())
    } else {
        p.scope()
    }
}
