use filmcraft_time::{FrameRate, Tick};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const SCHEMA: u32 = 1;
pub const MAX_FRAMES: u32 = 2_000_000;

/// FilmCraft library data belongs to the shared project; it is not a second project.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FilmLibrary {
    pub settings: filmcraft_project::ProjectSettings,
    pub root: Option<filmcraft_project::Bin>,
    pub transcripts: std::collections::BTreeMap<
        filmcraft_project::ItemId,
        std::sync::Arc<filmcraft_project::Transcript>,
    >,
    pub search_bins: Vec<filmcraft_project::SearchBin>,
    pub source_graphics:
        std::collections::BTreeMap<filmcraft_project::ItemId, filmcraft_project::SourceGraphic>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub source_fps: Option<f64>,
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default)]
    pub sample_rate: Option<u32>,
    #[serde(default)]
    pub channels: Option<u16>,
    #[serde(default)]
    pub source_decoder: String,
    #[serde(default)]
    pub native_item: Option<filmcraft_project::ProjectItem>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    pub id: String,
    pub asset_id: String,
    pub frames: u32,
    pub source_in: u32,
    pub muted: bool,
    #[serde(default = "unity")]
    pub volume: f64,
    #[serde(default)]
    pub start: u32,
    #[serde(default = "crate::film::video_track")]
    pub track: u64,
    #[serde(default = "unity")]
    pub speed: f64,
    #[serde(default)]
    pub reverse: bool,
}
fn empty_look() -> Value {
    json!({})
}

fn unity() -> f64 {
    1.0
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub clip_id: String,
    pub start: u32,
    pub end: u32,
}
impl Scope {
    pub fn contains(&self, clip: &str, frame: u32) -> bool {
        self.clip_id == clip && frame >= self.start && frame < self.end
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Adjustments {
    pub exposure: f64,
    pub contrast: f64,
    pub highlights: f64,
    pub shadows: f64,
    pub temperature: f64,
    pub tint: f64,
    pub saturation: f64,
    #[serde(flatten)]
    pub advanced: std::collections::BTreeMap<String, Value>,
}
impl Adjustments {
    pub fn merge(&mut self, v: &Value) -> Result<(), String> {
        let o = v.as_object().ok_or("Justeringar måste vara ett objekt")?;
        if let Some(patch) = o.get("settings_patch") {
            fn overlay(base: &mut Value, patch: &Value) {
                if let (Some(b), Some(p)) = (base.as_object_mut(), patch.as_object()) {
                    for (key, value) in p {
                        overlay(b.entry(key).or_insert(Value::Null), value);
                    }
                } else {
                    *base = patch.clone();
                }
            }
            let mut settings =
                serde_json::to_value(crate::render::settings(self)?).map_err(|e| e.to_string())?;
            overlay(&mut settings, patch);
            self.merge(&json!({"settings":settings}))?;
        }
        if let Some(v) = o.get("settings") {
            let d: lightcraft_develop::DevelopSettings =
                serde_json::from_value(v.clone()).map_err(|e| e.to_string())?;
            self.exposure = d.light.exposure;
            self.contrast = d.light.contrast;
            self.highlights = d.light.highlights;
            self.shadows = d.light.shadows;
            self.temperature = (d.wb.temp - 6500.0) / 30.0;
            self.tint = d.wb.tint;
            self.saturation = d.color.saturation;
            self.advanced.clear();
            self.advanced.insert(
                "settings".into(),
                serde_json::to_value(d).map_err(|e| e.to_string())?,
            );
        }
        for (input_key, val) in o {
            if input_key == "settings" || input_key == "settings_patch" {
                continue;
            }
            let key = canonical_control_key(input_key).to_owned();
            if key == "treatment" {
                if !matches!(val.as_str(), Some("color" | "bw")) {
                    return Err("Ogiltig färgbehandling".into());
                }
                self.advanced.insert(key.clone(), val.clone());
                continue;
            }
            if ["curve.master", "curve.red", "curve.green", "curve.blue"].contains(&key.as_str()) {
                let points = val.as_array().ok_or("Kurvan måste innehålla punkter")?;
                if points.len() < 2 || points.len() > 32 {
                    return Err("Kurvan måste ha 2–32 punkter".into());
                }
                let mut last = -1.0;
                for point in points {
                    let x = point["x"]
                        .as_f64()
                        .filter(|n| n.is_finite() && (0.0..=1.0).contains(n))
                        .ok_or("Ogiltig kurvpunkt")?;
                    let _y = point["y"]
                        .as_f64()
                        .filter(|n| n.is_finite() && (0.0..=1.0).contains(n))
                        .ok_or("Ogiltig kurvpunkt")?;
                    if x <= last {
                        return Err("Kurvpunkter måste ha stigande indata".into());
                    }
                    last = x;
                }
                self.advanced.insert(key.clone(), val.clone());
                continue;
            }
            if let Some(spec) = lightcraft_develop::controls::find(&key)
                && supported_control(spec)
            {
                let n = val
                    .as_f64()
                    .filter(|n| n.is_finite() && (spec.min..=spec.max).contains(n))
                    .ok_or("Ogiltigt framkallningsvärde")?;
                self.advanced.insert(key.clone(), json!(n));
                continue;
            }
            let n = val
                .as_f64()
                .filter(|v| v.is_finite())
                .ok_or("Ogiltigt reglagevärde")?;
            let (slot, lo, hi) = match key.as_str() {
                "exposure" => (&mut self.exposure, -5.0, 5.0),
                "contrast" => (&mut self.contrast, -100.0, 100.0),
                "highlights" => (&mut self.highlights, -100.0, 100.0),
                "shadows" => (&mut self.shadows, -100.0, 100.0),
                "temperature" => (&mut self.temperature, -100.0, 100.0),
                "tint" => (&mut self.tint, -100.0, 100.0),
                "saturation" => (&mut self.saturation, -100.0, 100.0),
                _ => return Err(format!("Okänt reglage: {key}")),
            };
            if !(lo..=hi).contains(&n) {
                return Err("Reglaget ligger utanför tillåtet intervall".into());
            }
            *slot = n;
        }
        Ok(())
    }
}
pub fn supported_control(spec: &lightcraft_develop::ControlSpec) -> bool {
    use lightcraft_develop::Section::*;
    matches!(
        spec.section,
        Light
            | Curve
            | Color
            | Mixer
            | BwMix
            | Grading
            | Effects
            | Vignette
            | Grain
            | Detail
            | Calibration
            | Optics
            | Geometry
            | Profile
    )
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Adjustment {
    pub scope: Scope,
    pub values: Value,
}
pub fn canonical_control_key(key: &str) -> &str {
    match key {
        "light.exposure" => "exposure",
        "light.contrast" => "contrast",
        "light.highlights" => "highlights",
        "light.shadows" => "shadows",
        "color.saturation" => "saturation",
        _ => key,
    }
}

/// Retain sparse settings while ensuring both UI/API spellings update one value.
pub fn normalize_adjustment_values(values: &Value) -> Result<Value, String> {
    let mut normalized = json!({});
    for (key, value) in values.as_object().ok_or("Ogiltiga justeringar")? {
        normalized[canonical_control_key(key)] = value.clone();
    }
    Ok(normalized)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stroke {
    pub color: [f32; 4],
    /// Diameter relative to project width.
    pub size: f32,
    /// Normalized x, y, pressure.
    pub points: Vec<[f64; 3]>,
    pub erase: bool,
    #[serde(default = "default_hardness")]
    pub hardness: f32,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default = "default_opacity")]
    pub flow: f32,
    #[serde(default)]
    pub pressure_size: bool,
    #[serde(default)]
    pub selection: Option<[f64; 4]>,
}
fn default_hardness() -> f32 {
    0.85
}
fn default_opacity() -> f32 {
    1.0
}
fn normal_blend() -> photocraft_doc::BlendMode {
    photocraft_doc::BlendMode::Normal
}
fn default_scale() -> f64 {
    1.0
}
fn valid_rect(r: [f64; 4]) -> bool {
    r.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)) && r[0] < r[2] && r[1] < r[3]
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Text {
    pub content: String,
    pub size: f32,
    pub color: [f32; 4],
    pub position: [f64; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub scope: Scope,
    pub opacity: f32,
    pub visible: bool,
    pub strokes: Vec<Stroke>,
    #[serde(default)]
    pub mask: Option<[f64; 4]>,
    #[serde(default)]
    pub text: Option<Text>,
    #[serde(default)]
    pub offset: [f64; 2],
    #[serde(default = "normal_blend")]
    pub blend: photocraft_doc::BlendMode,
    #[serde(default = "default_scale")]
    pub scale: f64,
    #[serde(default)]
    pub rotation: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NativeCall {
    pub command: String,
    pub params: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhotoTools {
    pub foreground: [f32; 4],
    pub background: [f32; 4],
    pub brush: photocraft_paint::BrushSettings,
    pub presets: Vec<photocraft_paint::BrushPreset>,
    pub mixer: photocraft_paint::mixer::MixerState,
}
/// A copied payload is stored only with operations that consume it; the live clipboard is UI state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PhotoClipboard {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pixels: Option<std::sync::Arc<Vec<u8>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[i32; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<std::sync::Arc<Vec<u8>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<photocraft_doc::Fill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<photocraft_doc::ShapeStroke>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhotoOperation {
    pub id: String,
    pub scope: Scope,
    pub call: NativeCall,
    pub target: String,
    pub selection: Vec<NativeCall>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<NativeCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clipboard: Option<PhotoClipboard>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<PhotoTools>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub mode: String,
    pub scope: String,
    pub playhead: u32,
    pub selection: Option<Scope>,
    pub selected_layer: Option<String>,
    pub timeline_visible: bool,
    pub timeline_height: u32,
    pub left_visible: bool,
    pub right_visible: bool,
    #[serde(default)]
    pub pixel_selection: Option<[f64; 4]>,
    #[serde(default)]
    pub photo_selection: Vec<NativeCall>,
    #[serde(default)]
    pub photo_context: Vec<NativeCall>,
    #[serde(default)]
    pub photo_tools: Option<PhotoTools>,
    #[serde(skip)]
    pub photo_clipboard: PhotoClipboard,
    #[serde(default)]
    pub native_target: Option<String>,
    #[serde(default)]
    pub active_mask: Option<u32>,
    #[serde(default)]
    pub active_spot: Option<usize>,
    #[serde(default)]
    pub active_clip_id: Option<String>,
    #[serde(default)]
    pub film_state: filmcraft_engine::EditorState,
}
impl Default for Workspace {
    fn default() -> Self {
        Self {
            mode: "photo".into(),
            scope: "frame".into(),
            playhead: 0,
            selection: None,
            selected_layer: None,
            timeline_visible: true,
            timeline_height: 210,
            left_visible: true,
            right_visible: true,
            pixel_selection: None,
            photo_selection: vec![],
            photo_context: vec![],
            photo_tools: None,
            photo_clipboard: Default::default(),
            native_target: None,
            active_mask: None,
            active_spot: None,
            active_clip_id: None,
            film_state: Default::default(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps: FrameRate,
    pub assets: Vec<Asset>,
    #[serde(default, skip_serializing)]
    pub clips: Vec<Clip>,
    #[serde(default)]
    pub timeline: Option<filmcraft_project::Sequence>,
    #[serde(default)]
    pub resources: Vec<filmcraft_project::ProjectItem>,
    #[serde(default = "crate::film::next_native_id")]
    pub native_next_id: u64,
    #[serde(default)]
    pub film_luts: Vec<filmcraft_project::ProjectLut>,
    #[serde(default)]
    pub film_library: FilmLibrary,
    #[serde(default)]
    pub light_presets: Vec<lightcraft_develop::Preset>,
    #[serde(default)]
    pub develop_clipboard: Option<Value>,
    pub adjustments: Vec<Adjustment>,
    #[serde(default = "empty_look")]
    pub project_look: Value,
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub photo_operations: Vec<PhotoOperation>,
    pub workspace: Workspace,
    pub next_id: u64,
    /// Legacy projects retain source-only grading; new projects grade the shared canvas.
    #[serde(default)]
    pub develop_canvas: bool,
}
impl Default for Project {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA,
            id: "project-1".into(),
            name: "Mitt projekt".into(),
            width: 1280,
            height: 720,
            fps: FrameRate::FPS_25,
            assets: vec![],
            clips: vec![],
            timeline: Some(crate::film::empty_sequence(1280, 720, FrameRate::FPS_25)),
            resources: vec![],
            native_next_id: crate::film::next_native_id(),
            film_luts: vec![],
            film_library: FilmLibrary::default(),
            light_presets: vec![],
            develop_clipboard: None,
            adjustments: vec![],
            project_look: json!({}),
            layers: vec![],
            photo_operations: vec![],
            workspace: Workspace::default(),
            next_id: 1,
            develop_canvas: true,
        }
    }
}
impl Project {
    pub fn id(&mut self, prefix: &str) -> String {
        let id = format!("{prefix}-{}", self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }
    pub fn frames(&self) -> u32 {
        self.timeline
            .as_ref()
            .map(|s| self.fps.frame_at(s.duration()).max(0) as u32)
            .unwrap_or_else(|| self.clips.iter().map(|c| c.frames).sum())
    }
    pub fn at(&self, global: u32) -> Option<(&Clip, u32)> {
        let covers = |c: &&Clip| {
            global >= c.start
                && global < c.start + c.frames
                && !self
                    .assets
                    .iter()
                    .find(|a| a.id == c.asset_id)
                    .and_then(|a| a.native_item.as_ref())
                    .is_some_and(|i| {
                        matches!(i.kind, filmcraft_project::ItemKind::AdjustmentLayer { .. })
                    })
        };
        if let Some(id) = &self.workspace.active_clip_id
            && let Some(c) = self.clips.iter().filter(covers).find(|c| &c.id == id)
        {
            return Some((c, global - c.start));
        }
        self.clips
            .iter()
            .filter(covers)
            .max_by_key(|c| {
                self.timeline
                    .as_ref()
                    .and_then(|s| s.video_tracks.iter().position(|t| t.id.0 == c.track))
                    .unwrap_or(0)
            })
            .map(|c| (c, global - c.start))
    }
    pub fn scope(&self) -> Result<Scope, String> {
        let (c, f) = self
            .at(self.workspace.playhead)
            .ok_or("Importera eller skapa en bild först")?;
        match self.workspace.scope.as_str() {
            "frame" => Ok(Scope {
                clip_id: c.id.clone(),
                start: f,
                end: f + 1,
            }),
            "clip" => Ok(Scope {
                clip_id: c.id.clone(),
                start: 0,
                end: c.frames,
            }),
            "range" => self
                .workspace
                .selection
                .clone()
                .ok_or("Markera ett intervall först".into()),
            "project" => Err("Projektomfattning stöds av framkallning. Välj bildruta, intervall eller klipp för lager.".into()),
            _ => Err("Ogiltig omfattning".into()),
        }
    }
    pub fn look(&self, clip: &str, frame: u32) -> Adjustments {
        let mut a = Adjustments::default();
        for edit in &self.adjustments {
            if edit.scope.contains(clip, frame) {
                let _ = a.merge(&edit.values);
            }
        }
        let _ = a.merge(&self.project_look);
        a
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA {
            return Err("Projektversionen stöds inte".into());
        }
        if self.width == 0
            || self.height == 0
            || self.width > 16384
            || self.height > 16384
            || u64::from(self.width) * u64::from(self.height) > 16_777_216
        {
            return Err("Bilden måste vara mellan 1 och 16 miljoner pixlar".into());
        }
        if self.fps.num <= 0 || self.fps.den <= 0 || !(1.0..=120.0).contains(&self.fps.as_f64()) {
            return Err("Ogiltig bildfrekvens".into());
        }
        if self.assets.len() > 500
            || self.clips.len() > 1000
            || self.layers.len() > 500
            || self.adjustments.len() > 5000
            || self.photo_operations.len() > 2000
            || self.workspace.photo_selection.len() > 200
            || self.workspace.photo_context.len() > 500
        {
            return Err("Projektet har för många objekt".into());
        }
        if self
            .workspace
            .photo_context
            .iter()
            .any(|c| !crate::native::context_command(&c.command))
        {
            return Err("Ogiltigt verktygstillstånd".into());
        }
        for t in self.workspace.photo_tools.iter().chain(
            self.photo_operations
                .iter()
                .filter_map(|o| o.tools.as_ref()),
        ) {
            if t.presets.len() > 1024
                || !t.brush.size.is_finite()
                || !(0.01..=4096.0).contains(&t.brush.size)
                || !t.brush.spacing.is_finite()
                || !(0.001..=10.0).contains(&t.brush.spacing)
                || [t.brush.hardness, t.brush.opacity, t.brush.flow]
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            {
                return Err("Ogiltiga penselinställningar".into());
            }
        }
        let mut ids = std::collections::HashSet::new();
        for id in self
            .assets
            .iter()
            .map(|a| &a.id)
            .chain(self.clips.iter().map(|c| &c.id))
            .chain(self.layers.iter().map(|l| &l.id))
        {
            if id.is_empty() || id.len() > 160 || !ids.insert(id) {
                return Err("Projektet har dubbla eller tomma identifierare".into());
            }
        }
        for c in &self.clips {
            if !self.assets.iter().any(|a| a.id == c.asset_id)
                || c.frames == 0
                || !c.volume.is_finite()
                || !(0.0..=1.0).contains(&c.volume)
                || c.source_in > MAX_FRAMES
            {
                return Err("Klippet har ogiltig media eller varaktighet".into());
            }
            let _end = c
                .start
                .checked_add(c.frames)
                .filter(|&f| f <= MAX_FRAMES)
                .ok_or("Tidslinjen är för lång")?;
        }
        for s in self
            .adjustments
            .iter()
            .map(|e| &e.scope)
            .chain(self.layers.iter().map(|l| &l.scope))
            .chain(self.photo_operations.iter().map(|l| &l.scope))
            .chain(self.workspace.selection.iter())
        {
            if !self
                .clips
                .iter()
                .any(|c| c.id == s.clip_id && s.start < s.end && s.end <= c.frames)
                && !self
                    .resources
                    .iter()
                    .filter_map(|i| i.as_sequence())
                    .any(|seq| {
                        seq.video_tracks.iter().flat_map(|t| &t.items).any(|c| {
                            format!("clip-{}", c.id.0) == s.clip_id
                                && s.start < s.end
                                && i64::from(s.end) <= seq.settings.frame_rate.frame_at(c.duration)
                        })
                    })
            {
                return Err("Redigeringens intervall ligger utanför klippet".into());
            }
        }
        let frames = self.frames();
        if frames > MAX_FRAMES {
            return Err("Tidslinjen är för lång".into());
        }
        if frames > 0 && self.workspace.playhead >= frames {
            return Err("Tidspositionen ligger utanför projektet".into());
        }
        if !["photo", "light", "film"].contains(&self.workspace.mode.as_str())
            || !["frame", "range", "clip", "project"].contains(&self.workspace.scope.as_str())
        {
            return Err("Ogiltigt verktygsläge eller omfattning".into());
        }
        if self
            .workspace
            .selected_layer
            .as_ref()
            .is_some_and(|id| !self.layers.iter().any(|l| &l.id == id))
        {
            return Err("Det valda lagret finns inte".into());
        }
        for a in &self.assets {
            if !["image", "video", "blank", "audio", "generator"].contains(&a.kind.as_str())
                || a.width == 0
                || a.height == 0
                || a.width > 32768
                || a.height > 32768
                || a.source_fps
                    .is_some_and(|f| !f.is_finite() || !(1.0..=240.0).contains(&f))
                || a.duration.is_some_and(|f| !f.is_finite() || f <= 0.0)
                || a.sample_rate.is_some_and(|f| !(8000..=192000).contains(&f))
                || a.channels.is_some_and(|c| c == 0 || c > 6)
                || u64::from(a.width) * u64::from(a.height) > 16_777_216
            {
                return Err("Ogiltig medietillgång".into());
            }
        }
        if let Some(t) = &self.timeline {
            t.settings.validate()?;
            if t.settings.frame_rate != self.fps
                || !(8000..=192000).contains(&t.settings.sample_rate)
            {
                return Err("Ogiltigt tidsformat".into());
            }
            if t.settings.width != self.width
                || t.settings.height != self.height
                || t.all_tracks().count() > 64
                || t.all_tracks().any(|tr| {
                    tr.items.len() > 1000
                        || tr.transitions.len() > 1000
                        || tr.items.iter().any(|i| {
                            i.start.0 < 0
                                || i.duration.0 <= 0
                                || !i.speed.is_finite()
                                || i.speed.abs() > 100.0
                        })
                })
            {
                return Err("Ogiltig tidslinje".into());
            }
        }
        for op in &self.photo_operations {
            if !crate::native::supported("photo", &op.call.command)
                || op.context.len() > 500
                || op
                    .context
                    .iter()
                    .any(|c| !crate::native::context_command(&c.command))
                || op
                    .clipboard
                    .as_ref()
                    .is_some_and(|c| c.pixels.as_ref().is_some_and(|v| v.len() > 16_000_000))
                || op
                    .selection
                    .iter()
                    .any(|c| !c.command.starts_with("select."))
            {
                return Err("Ogiltig bildoperation".into());
            }
        }
        Adjustments::default().merge(&self.project_look)?;
        for e in &self.adjustments {
            Adjustments::default().merge(&e.values)?;
        }
        if self
            .workspace
            .pixel_selection
            .is_some_and(|r| !valid_rect(r))
        {
            return Err("Ogiltig bildmarkering".into());
        }
        let mut points = 0_usize;
        for l in &self.layers {
            if !l.scale.is_finite()
                || !(0.05..=4.0).contains(&l.scale)
                || !l.rotation.is_finite()
                || !(-180.0..=180.0).contains(&l.rotation)
            {
                return Err("Ogiltig lagertransformation".into());
            }
            if l.text.as_ref().is_some_and(|text| {
                text.content.chars().count() > 1000
                    || (text.content.chars().count() as f64)
                        * (f64::from(text.size) * f64::from(self.width) * l.scale).powi(2)
                        > 16_777_216.0
                    || text.content.is_empty()
                    || !text.size.is_finite()
                    || !(0.0001..=0.5).contains(&text.size)
                    || text
                        .position
                        .iter()
                        .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                    || text
                        .color
                        .iter()
                        .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            }) {
                return Err("Ogiltigt textlager".into());
            }
            if l.offset
                .iter()
                .any(|v| !v.is_finite() || !(-1.0..=1.0).contains(v))
                || l.mask.is_some_and(|r| {
                    r.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                        || r[0] >= r[2]
                        || r[1] >= r[3]
                })
            {
                return Err("Ogiltig lagermask eller förflyttning".into());
            }
            if !l.opacity.is_finite() || !(0.0..=1.0).contains(&l.opacity) {
                return Err("Ogiltig lageropacitet".into());
            }
            for s in &l.strokes {
                if [s.hardness, s.opacity, s.flow]
                    .iter()
                    .any(|n| !n.is_finite() || !(0.0..=1.0).contains(n))
                    || s.selection.is_some_and(|r| !valid_rect(r))
                {
                    return Err("Ogiltiga penselinställningar".into());
                }
                points = points
                    .checked_add(s.points.len())
                    .filter(|&n| n <= 500_000)
                    .ok_or("För många penselpunkter")?;
                if !s.size.is_finite()
                    || !(0.0001..=0.5).contains(&s.size)
                    || s.points.is_empty()
                    || s.points.len() > 20_000
                    || s.color
                        .iter()
                        .any(|n| !n.is_finite() || !(0.0..=1.0).contains(n))
                    || s.points
                        .iter()
                        .flatten()
                        .any(|n| !n.is_finite() || !(0.0..=1.0).contains(n))
                {
                    return Err("Ogiltigt penseldrag".into());
                }
            }
        }
        Ok(())
    }
    pub fn inspect(&self) -> Value {
        let (clip, local) = self
            .at(self.workspace.playhead)
            .map(|(c, f)| (Some(c), f))
            .unwrap_or((None, 0));
        let mut project = serde_json::to_value(self).unwrap_or(Value::Null);
        project["clips"] = json!(self.clips);
        let mut look = clip
            .map(|c| serde_json::to_value(self.look(&c.id, local)).unwrap_or(Value::Null))
            .unwrap_or_else(|| json!({}));
        if let Some(c) = clip
            && let Ok(settings) = crate::render::settings(&self.look(&c.id, local))
        {
            for spec in lightcraft_develop::CONTROLS
                .iter()
                .filter(|c| supported_control(c))
            {
                if let Some(n) = lightcraft_develop::controls::get(&settings, spec.id) {
                    look[canonical_control_key(spec.id)] = json!(n);
                }
            }
            look["treatment"] = json!(settings.treatment);
            for (k, points) in [
                ("master", &settings.curve.master),
                ("red", &settings.curve.red),
                ("green", &settings.curve.green),
                ("blue", &settings.curve.blue),
            ] {
                look[format!("curve.{k}")] = json!(points);
            }
        }
        json!({ "project": project, "total_frames": self.frames(), "active_clip": clip, "local_frame": local,
            "seconds": self.fps.tick_of(i64::from(self.workspace.playhead)).seconds(),
            "source_seconds": clip.map(|c| self.fps.tick_of(i64::from(c.source_in)).seconds() + f64::from(if c.reverse { c.frames.saturating_sub(1 + local) } else { local }) / self.fps.as_f64() * c.speed.abs()),
            "look": look, "ticks_per_second": filmcraft_time::TICKS_PER_SECOND,
            "frame_ticks": self.fps.frame_duration(), "duration_ticks": self.fps.tick_of(i64::from(self.frames())),
            "develop_controls": lightcraft_develop::CONTROLS.iter().filter(|c| supported_control(c)).collect::<Vec<_>>(),
            "blend_modes": photocraft_doc::BlendMode::LAYER_MODES.iter().map(|b| json!({"id":b,"label":b.label()})).collect::<Vec<_>>(),
            "render_backend": "PhotoCraft engine/compose + LightCraft engine/pipeline + FilmCraft engine/render/audio" })
    }
    pub fn tick_at(&self, frame: u32) -> Tick {
        self.fps.tick_of(i64::from(frame))
    }
}
