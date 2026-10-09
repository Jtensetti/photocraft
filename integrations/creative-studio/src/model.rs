use filmcraft_time::{FrameRate, Tick};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const SCHEMA: u32 = 1;
pub const MAX_FRAMES: u32 = 2_000_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub source_fps: Option<f64>,
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
}
impl Adjustments {
    pub fn merge(&mut self, v: &Value) -> Result<(), String> {
        let o = v.as_object().ok_or("Justeringar måste vara ett objekt")?;
        for (key, val) in o {
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Adjustment {
    pub scope: Scope,
    pub values: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stroke {
    pub color: [f32; 4],
    /// Diameter relative to project width.
    pub size: f32,
    /// Normalized x, y, pressure.
    pub points: Vec<[f64; 3]>,
    pub erase: bool,
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
    pub clips: Vec<Clip>,
    pub adjustments: Vec<Adjustment>,
    #[serde(default = "empty_look")]
    pub project_look: Value,
    pub layers: Vec<Layer>,
    pub workspace: Workspace,
    pub next_id: u64,
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
            adjustments: vec![],
            project_look: json!({}),
            layers: vec![],
            workspace: Workspace::default(),
            next_id: 1,
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
        self.clips.iter().map(|c| c.frames).sum()
    }
    pub fn at(&self, global: u32) -> Option<(&Clip, u32)> {
        let mut offset = 0;
        for c in &self.clips {
            if global >= offset && global < offset + c.frames {
                return Some((c, global - offset));
            }
            offset += c.frames;
        }
        None
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
        {
            return Err("Projektet har för många objekt".into());
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
        let mut frames = 0_u32;
        for c in &self.clips {
            if !self.assets.iter().any(|a| a.id == c.asset_id)
                || c.frames == 0
                || !c.volume.is_finite()
                || !(0.0..=1.0).contains(&c.volume)
                || c.source_in > MAX_FRAMES
            {
                return Err("Klippet har ogiltig media eller varaktighet".into());
            }
            frames = frames
                .checked_add(c.frames)
                .filter(|&f| f <= MAX_FRAMES)
                .ok_or("Tidslinjen är för lång")?;
        }
        for s in self
            .adjustments
            .iter()
            .map(|e| &e.scope)
            .chain(self.layers.iter().map(|l| &l.scope))
            .chain(self.workspace.selection.iter())
        {
            if !self
                .clips
                .iter()
                .any(|c| c.id == s.clip_id && s.start < s.end && s.end <= c.frames)
            {
                return Err("Redigeringens intervall ligger utanför klippet".into());
            }
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
            if !["image", "video", "blank"].contains(&a.kind.as_str())
                || a.width == 0
                || a.height == 0
                || a.width > 32768
                || a.height > 32768
                || u64::from(a.width) * u64::from(a.height) > 16_777_216
            {
                return Err("Ogiltig medietillgång".into());
            }
        }
        Adjustments::default().merge(&self.project_look)?;
        for e in &self.adjustments {
            Adjustments::default().merge(&e.values)?;
        }
        let mut points = 0_usize;
        for l in &self.layers {
            if l.text.as_ref().is_some_and(|text| {
                text.content.chars().count() > 1000
                    || (text.content.chars().count() as f64)
                        * (f64::from(text.size) * f64::from(self.width)).powi(2)
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
        json!({ "project": self, "total_frames": self.frames(), "active_clip": clip, "local_frame": local,
            "seconds": self.fps.tick_of(i64::from(self.workspace.playhead)).seconds(),
            "source_seconds": clip.map(|c| self.fps.tick_of(i64::from(c.source_in + local)).seconds()),
            "look": clip.map(|c| self.look(&c.id, local)), "ticks_per_second": filmcraft_time::TICKS_PER_SECOND,
            "frame_ticks": self.fps.frame_duration(), "duration_ticks": self.fps.tick_of(i64::from(self.frames())),
            "render_backend": "PhotoCraft compose/paint/text + LightCraft pipeline + FilmCraft edit/time" })
    }
    pub fn tick_at(&self, frame: u32) -> Tick {
        self.fps.tick_of(i64::from(frame))
    }
}
