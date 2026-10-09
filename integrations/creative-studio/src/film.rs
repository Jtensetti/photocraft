//! Disposable FilmCraft edit view. Only Studio owns the persistent project/history.
use crate::model::Clip;
use filmcraft_edit::{Edge, EditCtx, TrimMode};
use filmcraft_project::{ClipId, Project, SequenceSettings, TrackItem};
use filmcraft_time::{FrameRate, Tick};
use serde_json::json;

pub fn trim(clip: &Clip, rate: FrameRate, start: u32, end: u32) -> Result<(u32, u32), String> {
    let mut view = Project::new("Creative Studio edit view");
    let sequence_id = view.new_sequence(
        "Edit",
        SequenceSettings {
            frame_rate: rate,
            ..Default::default()
        },
        1,
        0,
        None,
    );
    let sequence = view
        .sequence_mut(sequence_id)
        .ok_or("FilmCraft-sekvensen saknas")?;
    let item: TrackItem = serde_json::from_value(json!({
        "id": 100, "item": 200, "name": clip.id, "label": "Teal",
        "start": Tick::ZERO, "duration": rate.tick_of(i64::from(clip.frames)),
        "source_in": rate.tick_of(i64::from(clip.source_in)), "speed": clip.speed, "reverse":clip.reverse,
        "enabled": true, "effects": []
    }))
    .map_err(|e| e.to_string())?;
    let track = sequence
        .video_tracks
        .first_mut()
        .ok_or("FilmCraft-spåret saknas")?;
    track.items.push(item);
    let mut next_id = 1000;
    let mut ctx = EditCtx {
        next_id: &mut next_id,
        media_duration: &|_| None,
        media_start: &|_| Tick::ZERO,
        min_duration: rate.frame_duration(),
    };
    filmcraft_edit::trim(
        sequence,
        ClipId(100),
        Edge::Out,
        TrimMode::Ripple,
        rate.tick_of(i64::from(end) - i64::from(clip.frames)),
        &mut ctx,
    )
    .map_err(|e| e.to_string())?;
    filmcraft_edit::trim(
        sequence,
        ClipId(100),
        Edge::In,
        TrimMode::Ripple,
        rate.tick_of(i64::from(start)),
        &mut ctx,
    )
    .map_err(|e| e.to_string())?;
    let (_, result) = sequence
        .find_item(ClipId(100))
        .ok_or("FilmCraft-klippet saknas")?;
    let source = u32::try_from(rate.frame_at(result.source_in)).map_err(|_| "Ogiltig källtid")?;
    let frames =
        u32::try_from(rate.frame_at(result.duration)).map_err(|_| "Ogiltig varaktighet")?;
    let expected_source = clip.source_in
        + (f64::from(if clip.reverse {
            clip.frames - end
        } else {
            start
        }) * clip.speed.abs())
        .round() as u32;
    if source != expected_source || frames != end - start {
        return Err("FilmCraft kunde inte behålla den valda omfattningen".into());
    }
    Ok((source, frames))
}

// The native Sequence is the authoritative temporal component of the shared Project.
// `clips` is only a legacy-compatible browser view and is never saved alongside it.
use crate::model::Project as SharedProject;
use filmcraft_project::{
    ItemId, ItemKind, Label, MediaClip, ProjectItem, Sequence, TrackId, TrackKind,
};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};
const SEQUENCE_ID: u64 = 10_000_002;
pub fn video_track() -> u64 {
    10_000_000
}
pub fn next_native_id() -> u64 {
    50_000_000
}
fn clip_id(id: &str) -> Result<u64, String> {
    id.strip_prefix("clip-")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("Ogiltig klippidentifierare: {id}"))
}
fn media_id(index: usize) -> ItemId {
    ItemId(20_000_000 + index as u64)
}
fn asset_item(p: &SharedProject, index: usize) -> ItemId {
    p.assets
        .get(index)
        .and_then(|a| a.native_item.as_ref())
        .map(|i| i.id)
        .unwrap_or_else(|| media_id(index))
}
pub fn empty_sequence(w: u32, h: u32, fps: FrameRate) -> Sequence {
    let mut p = filmcraft_project::Project::new("Canvas");
    p.next_id = video_track();
    let id = p.new_sequence(
        "Canvas",
        SequenceSettings {
            width: w,
            height: h,
            frame_rate: fps,
            ..Default::default()
        },
        1,
        1,
        None,
    );
    p.sequence(id).cloned().unwrap_or_else(|| Sequence {
        settings: SequenceSettings::default(),
        video_tracks: vec![],
        audio_tracks: vec![],
        markers: vec![],
        mark_in: None,
        mark_out: None,
        work_area: None,
        start_timecode: 0,
        master_volume_db: 0.0,
        master_effects: vec![],
        master_mixer: Default::default(),
        submix_tracks: vec![],
        caption_tracks: vec![],
        multicam: None,
        merged: None,
        split: Default::default(),
    })
}
fn media(a: &crate::model::Asset, rate: FrameRate) -> Result<MediaClip, String> {
    serde_json::from_value(json!({"media":{"File":{"path":a.id}},"info":{"name":a.name,"kind":if a.kind=="video"{"Movie"}else if a.kind=="audio"{"AudioOnly"}else{"Still"},"duration":a.duration.map(Tick::from_seconds_f64).unwrap_or_else(||rate.tick_of(i64::from(crate::model::MAX_FRAMES))),"video":if a.kind=="audio"{Value::Null}else{json!({"width":a.width,"height":a.height,"frame_rate":a.source_fps.map(FrameRate::from_f64).unwrap_or(rate),"par":[1,1],"codec":"browser","pixel_format":"RGBA8","color":filmcraft_color::ColorInfo::SRGB_FULL,"has_alpha":true,"bitrate":null})},"audio":if matches!(a.kind.as_str(),"video"|"audio"){json!({"sample_rate":a.sample_rate.unwrap_or(48000),"channels":a.channels.unwrap_or(2),"codec":"browser","bits_per_sample":null})}else{Value::Null},"container":"Blob","start_timecode":null,"file_size":a.bytes},"interpret":filmcraft_project::Interpretation::default(),"mark_in":null,"mark_out":null,"markers":[],"offline":false,"proxy":null})).map_err(|e|e.to_string())
}
pub fn derived(p: &SharedProject) -> Result<filmcraft_project::Project, String> {
    let mut f = filmcraft_project::Project::new(&p.name);
    f.next_id = p.native_next_id;
    f.luts = p.film_luts.clone();
    f.settings = p.film_library.settings.clone();
    f.transcripts = p.film_library.transcripts.clone();
    f.search_bins = p.film_library.search_bins.clone();
    f.source_graphics = p.film_library.source_graphics.clone();
    for (i, a) in p.assets.iter().enumerate() {
        let id = asset_item(p, i);
        let item = if let Some(item) = &a.native_item {
            item.clone()
        } else {
            ProjectItem {
                id,
                name: a.name.clone(),
                label: Label::Teal,
                kind: ItemKind::Media(media(a, p.fps)?),
                metadata: Default::default(),
                created: id.0,
                split: Default::default(),
            }
        };
        f.items.insert(id, item);
    }
    for item in &p.resources {
        f.items.insert(item.id, item.clone());
    }
    let seq = p.timeline.clone().ok_or("Tidslinjen saknas")?;
    let id = ItemId(SEQUENCE_ID);
    f.items.insert(
        id,
        ProjectItem {
            id,
            name: p.name.clone(),
            label: Label::Teal,
            kind: ItemKind::Sequence(Box::new(seq)),
            metadata: Default::default(),
            created: id.0,
            split: Default::default(),
        },
    );
    if let Some(root) = &p.film_library.root {
        f.root = root.clone();
    }
    fn present(b: &filmcraft_project::Bin, ids: &mut Vec<ItemId>) {
        for e in &b.children {
            match e {
                filmcraft_project::BinEntry::Item(i) => ids.push(*i),
                filmcraft_project::BinEntry::Bin(child) => present(child, ids),
            }
        }
    }
    let mut placed = vec![];
    present(&f.root, &mut placed);
    f.root.children.extend(
        f.items
            .keys()
            .filter(|id| !placed.contains(id))
            .map(|id| filmcraft_project::BinEntry::Item(*id)),
    );
    Ok(f)
}
pub fn restore(p: &mut SharedProject) -> Result<(), String> {
    if p.timeline.is_none() {
        let mut at = 0;
        for c in &mut p.clips {
            c.start = at;
            at += c.frames;
        }
        p.timeline = Some(empty_sequence(p.width, p.height, p.fps));
        sync_timeline(p, "migrate")?;
    } else {
        sync_clips(p)?;
    }
    Ok(())
}
pub fn add_audio(p: &mut SharedProject, id: &str, frames: u32) -> Result<(), String> {
    let i = p.assets.len().checked_sub(1).ok_or("Ljudkällan saknas")?;
    let mut f = derived(p)?;
    let mut item = f
        .make_track_item(
            asset_item(p, i),
            TrackKind::Audio,
            p.fps.tick_of(i64::from(p.workspace.playhead)),
            filmcraft_time::TimeRange::new(Tick::ZERO, p.fps.tick_of(i64::from(frames))),
            p.fps,
        )
        .ok_or("Ljudklippet kan inte skapas")?;
    item.id = ClipId(clip_id(id)?);
    item.name = p.assets[i].name.clone();
    let seq = p.timeline.as_mut().ok_or("Tidslinjen saknas")?;
    if seq.audio_tracks.is_empty() {
        seq.audio_tracks.push(filmcraft_project::Track::new(
            TrackId(p.native_next_id),
            TrackKind::Audio,
            "A1".into(),
        ));
        p.native_next_id += 1;
    }
    seq.audio_tracks[0].items.push(item);
    Ok(())
}
pub fn sync_clips(p: &mut SharedProject) -> Result<(), String> {
    let seq = p.timeline.as_ref().ok_or("Tidslinjen saknas")?;
    let mut clips = vec![];
    for t in &seq.video_tracks {
        for c in &t.items {
            let asset = p
                .assets
                .iter()
                .enumerate()
                .find(|(i, _)| asset_item(p, *i) == c.item)
                .map(|(_, a)| a.id.clone())
                .unwrap_or_else(|| format!("resource-{}", c.item.0));
            let audio = c.link.and_then(|link| {
                seq.audio_tracks
                    .iter()
                    .flat_map(|t| &t.items)
                    .find(|a| a.link == Some(link))
            });
            clips.push(crate::model::Clip {
                id: format!("clip-{}", c.id.0),
                asset_id: asset,
                frames: p.fps.frame_at(c.duration).max(1) as u32,
                source_in: p.fps.frame_at(c.source_in).max(0) as u32,
                muted: audio.is_some_and(|a| !a.enabled),
                volume: audio
                    .map(|a| 10_f64.powf(a.gain_db / 20.0).clamp(0.0, 1.0))
                    .unwrap_or(1.0),
                start: p.fps.frame_at(c.start).max(0) as u32,
                track: t.id.0,
                speed: c.speed,
                reverse: c.reverse,
            });
        }
    }
    clips.sort_by_key(|c| (c.start, c.track));
    p.clips = clips;
    Ok(())
}
pub fn is_contiguous(p: &SharedProject) -> bool {
    let Some(seq) = &p.timeline else {
        return true;
    };
    let mut tracks = seq.video_tracks.iter().filter(|t| !t.items.is_empty());
    let Some(track) = tracks.next() else {
        return true;
    };
    if tracks.next().is_some() {
        return false;
    }
    let mut items: Vec<_> = track.items.iter().collect();
    items.sort_by_key(|c| c.start);
    let mut end = Tick::ZERO;
    for c in items {
        if c.start != end {
            return false;
        }
        end = c.end();
    }
    true
}
pub fn sync_timeline(p: &mut SharedProject, command: &str) -> Result<(), String> {
    let sequential = command == "migrate" || is_contiguous(p);
    let derived = derived(p)?;
    let mut seq = p.timeline.take().ok_or("Tidslinjen saknas")?;
    let ids = p
        .clips
        .iter()
        .map(|c| clip_id(&c.id))
        .collect::<Result<Vec<_>, _>>()?;
    for t in &mut seq.video_tracks {
        t.items.retain(|c| ids.contains(&c.id.0));
    }
    let mut at: BTreeMap<u64, u32> = BTreeMap::new();
    for c in &p.clips {
        let id = filmcraft_project::ClipId(clip_id(&c.id)?);
        let tid = TrackId(c.track);
        let item = p
            .assets
            .iter()
            .position(|a| a.id == c.asset_id)
            .map(|i| asset_item(p, i))
            .ok_or("Mediet saknas")?;
        let old = seq.find_item(id).map(|(_, i)| i.clone());
        let mut native = old
            .or_else(|| derived.make_track_item_readonly(item, TrackKind::Video, p.fps))
            .ok_or("Klippet kunde inte skapas")?;
        native.id = id;
        native.name = c.id.clone();
        native.item = item;
        native.duration = p.fps.tick_of(i64::from(c.frames));
        native.source_in = p.fps.tick_of(i64::from(c.source_in));
        native.speed = c.speed;
        native.reverse = c.reverse;
        let start = if matches!(
            command,
            "clip.move" | "clip.trim" | "clip.split" | "clip.duration" | "migrate"
        ) && sequential
        {
            *at.entry(c.track).or_default()
        } else {
            c.start
        };
        native.start = p.fps.tick_of(i64::from(start));
        at.insert(c.track, start + c.frames);
        if let Some((_, v)) = seq.find_item_mut(id) {
            *v = native.clone();
        } else {
            seq.track_mut(tid)
                .ok_or("Spåret saknas")?
                .items
                .push(native.clone());
        }
        if let Some(asset) = p.assets.iter().find(|a| a.id == c.asset_id)
            && asset.kind == "video"
        {
            let link = native.link.unwrap_or(id.0 + 100_000_000);
            if let Some((_, n)) = seq.find_item_mut(id) {
                n.link = Some(link);
            }
            let old_audio = seq
                .audio_tracks
                .iter_mut()
                .flat_map(|t| &mut t.items)
                .find(|a| a.link == Some(link));
            let gain = if c.volume == 0.0 {
                -96.0
            } else {
                20.0 * c.volume.log10()
            };
            if let Some(a) = old_audio {
                a.start = native.start;
                a.duration = native.duration;
                a.source_in = native.source_in;
                a.speed = native.speed;
                a.enabled = !c.muted;
                a.gain_db = gain;
            } else {
                let mut a = native.clone();
                a.id = filmcraft_project::ClipId(id.0 + 100_000_000);
                a.effects = vec![];
                a.link = Some(link);
                a.enabled = !c.muted;
                a.gain_db = gain;
                seq.audio_tracks
                    .first_mut()
                    .ok_or("Ljudspåret saknas")?
                    .items
                    .push(a);
            }
        }
    }
    let links: Vec<_> = seq
        .video_tracks
        .iter()
        .flat_map(|t| &t.items)
        .filter_map(|i| i.link)
        .collect();
    for t in &mut seq.audio_tracks {
        t.items
            .retain(|a| a.link.is_none_or(|l| links.contains(&l)));
        t.items.sort_by_key(|i| i.start);
    }
    for t in &mut seq.video_tracks {
        t.items.sort_by_key(|i| i.start);
    }
    p.timeline = Some(seq);
    sync_clips(p)
}
trait ReadonlyItem {
    fn make_track_item_readonly(
        &self,
        item: ItemId,
        kind: TrackKind,
        rate: FrameRate,
    ) -> Option<filmcraft_project::TrackItem>;
}
impl ReadonlyItem for filmcraft_project::Project {
    fn make_track_item_readonly(
        &self,
        item: ItemId,
        kind: TrackKind,
        rate: FrameRate,
    ) -> Option<filmcraft_project::TrackItem> {
        let mut f = self.clone();
        f.make_track_item(
            item,
            kind,
            Tick::ZERO,
            filmcraft_time::TimeRange::new(Tick::ZERO, rate.tick_of(1)),
            rate,
        )
    }
}
pub(crate) struct BrowserServices;
impl filmcraft_engine::Services for BrowserServices {
    fn read_file(&self, _: &str) -> std::io::Result<Vec<u8>> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Välj filen i webbläsarens filväljare",
        ))
    }
    fn write_file(&self, _: &str, _: &[u8]) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Använd Creative Studios spara eller exportera",
        ))
    }
}
pub fn connected(id: &str) -> bool {
    if matches!(
        id,
        "file.newBin"
            | "file.newBinFromSelection"
            | "file.newSearchBin"
            | "file.newOfflineFile"
            | "file.newAdjustmentLayer"
            | "file.newBarsAndTone"
            | "file.newBlackVideo"
            | "file.newColorMatte"
            | "file.newCountingLeader"
            | "file.newTransparentVideo"
    ) {
        return true;
    }
    let family = id.split('.').next().unwrap_or("");
    !matches!(
        family,
        "file"
            | "prefs"
            | "jobs"
            | "export"
            | "audio"
            | "mediaBrowser"
            | "shortcuts"
            | "help"
            | "events"
            | "mediaCache"
    ) && !matches!(id, "edit.undo" | "edit.redo")
}
pub fn execute(
    p: &mut SharedProject,
    command: &str,
    mut params: Value,
) -> Result<(Value, bool), String> {
    if !connected(command) {
        return Err(
            "Det här kommandot behöver webbläsarens fil-, export- eller enhetsadapter".into(),
        );
    }
    let old = p.clips.clone();
    // Graphic commands use `seconds` for duration; the upstream time parser also
    // reads that key as placement. Set an explicit time to keep creation at the playhead.
    if command.starts_with("graphics.new")
        && params.get("time").is_none()
        && params.get("frame").is_none()
    {
        params["time"] = json!(p.fps.tick_of(i64::from(p.workspace.playhead)).0);
    }
    let mut s = filmcraft_engine::Session::new(Arc::new(BrowserServices));
    s.project = Arc::new(derived(p)?);
    for (i, a) in p.assets.iter().enumerate().filter(|(_, a)| a.bytes > 0) {
        if let Ok(source) = crate::sources::get(&a.id, &a.name, a.bytes)
            && let Some(media) = s.project.item(asset_item(p, i)).and_then(|i| i.as_media())
        {
            s.media.insert_keyed(
                asset_item(p, i),
                filmcraft_engine::media_pool::media_key(media),
                source,
            );
        }
    }
    let before = serde_json::to_value(s.project.as_ref()).map_err(|e| e.to_string())?;
    s.state = p.workspace.film_state.clone();
    s.state.active_sequence = Some(ItemId(SEQUENCE_ID));
    s.set_playhead(p.fps.tick_of(i64::from(p.workspace.playhead)));
    if s.state.selection.is_empty()
        && let Some((c, _)) = p.at(p.workspace.playhead)
    {
        s.state.selection = vec![filmcraft_project::ClipId(clip_id(&c.id)?)];
    }
    let result = s.execute(command, params).map_err(|e| e.to_string())?;
    let changed = before != serde_json::to_value(s.project.as_ref()).map_err(|e| e.to_string())?;
    if !changed {
        p.workspace.playhead = p.fps.frame_at(s.playhead()).max(0) as u32;
        p.workspace.film_state = s.state;
        return Ok((result, false));
    }
    p.timeline = Some(
        s.project
            .sequence(ItemId(SEQUENCE_ID))
            .ok_or("Den gemensamma tidslinjen kan inte tas bort")?
            .clone(),
    );
    let sequence = p.timeline.as_ref().ok_or("Tidslinjen saknas")?;
    let old_fps = p.fps;
    p.width = sequence.settings.width;
    p.height = sequence.settings.height;
    p.fps = sequence.settings.frame_rate;
    if old_fps != p.fps
        && (!p.adjustments.is_empty() || !p.photo_operations.is_empty() || !p.layers.is_empty())
    {
        return Err(
            "Ändra projektets bildfrekvens innan du skapar bildrutespecifika redigeringar".into(),
        );
    }
    let asset_ids: Vec<_> = (0..p.assets.len()).map(|i| asset_item(p, i)).collect();
    for (i, a) in p.assets.iter_mut().enumerate() {
        if let Some(native) = s.project.item(asset_ids[i]) {
            a.native_item = Some(native.clone());
        }
    }
    p.assets = p
        .assets
        .iter()
        .enumerate()
        .filter(|(i, _)| s.project.item(asset_ids[*i]).is_some())
        .map(|(_, a)| a.clone())
        .collect();
    p.resources = s
        .project
        .items
        .values()
        .filter(|i| i.id.0 != SEQUENCE_ID && !asset_ids.contains(&i.id))
        .cloned()
        .collect();
    p.film_luts = s.project.luts.clone();
    p.film_library = crate::model::FilmLibrary {
        settings: s.project.settings.clone(),
        root: Some(s.project.root.clone()),
        transcripts: s.project.transcripts.clone(),
        search_bins: s.project.search_bins.clone(),
        source_graphics: s.project.source_graphics.clone(),
    };
    p.native_next_id = s.project.next_id;
    p.workspace.playhead = p.fps.frame_at(s.playhead()).max(0) as u32;
    p.workspace.film_state = s.state;
    // Synthetic/graphic resources are media-library entries in the same shared project.
    for item in &p.resources {
        let aid = format!("resource-{}", item.id.0);
        if !p.assets.iter().any(|a| a.id == aid) {
            p.assets.push(crate::model::Asset {
                id: aid,
                name: item.name.clone(),
                kind: "generator".into(),
                width: p.width,
                height: p.height,
                bytes: 0,
                source_fps: None,
                duration: None,
                sample_rate: None,
                channels: None,
                source_decoder: String::new(),
                native_item: Some(item.clone()),
            });
        }
    }
    sync_clips(p)?;
    remap_scopes(p, &old)?;
    p.workspace.playhead = p.workspace.playhead.min(p.frames().saturating_sub(1));
    Ok((result, changed))
}
fn remap_scopes(p: &mut SharedProject, old: &[crate::model::Clip]) -> Result<(), String> {
    // Preserve an operation's media interval after native razor/trim/slip/speed operations.
    let remap = |scope: &crate::model::Scope| -> Vec<crate::model::Scope> {
        let Some(before) = old.iter().find(|c| c.id == scope.clip_id) else {
            return vec![];
        };
        let source = |c: &crate::model::Clip, local: u32| {
            f64::from(c.source_in)
                + f64::from(if c.reverse { c.frames - local } else { local }) * c.speed.abs()
        };
        let a = source(before, scope.start);
        let b = source(before, scope.end);
        if !p.clips.iter().any(|c| c.id == before.id)
            && p.resources.iter().filter_map(|i| i.as_sequence()).any(|s| {
                s.find_item(ClipId(clip_id(&before.id).unwrap_or(0)))
                    .is_some()
            })
        {
            return vec![scope.clone()];
        }
        p.clips
            .iter()
            .filter(|c| {
                c.asset_id == before.asset_id
                    && (c.id == before.id || !old.iter().any(|o| o.id == c.id))
            })
            .filter_map(|c| {
                let position = |t: f64| {
                    let v = (t - f64::from(c.source_in)) / c.speed.abs().max(0.000001);
                    if c.reverse {
                        f64::from(c.frames) - v
                    } else {
                        v
                    }
                };
                let x = position(a).round().max(0.0).min(f64::from(c.frames)) as u32;
                let y = position(b).round().max(0.0).min(f64::from(c.frames)) as u32;
                (x != y).then(|| crate::model::Scope {
                    clip_id: c.id.clone(),
                    start: x.min(y),
                    end: x.max(y),
                })
            })
            .collect()
    };
    p.adjustments = p
        .adjustments
        .iter()
        .flat_map(|e| {
            remap(&e.scope)
                .into_iter()
                .map(|scope| crate::model::Adjustment { scope, ..e.clone() })
        })
        .collect();
    let old_layers = p.layers.clone();
    p.photo_operations = p
        .photo_operations
        .iter()
        .flat_map(|e| {
            remap(&e.scope).into_iter().map(|scope| {
                let mut op = crate::model::PhotoOperation {
                    scope: scope.clone(),
                    ..e.clone()
                };
                if scope.clip_id != e.scope.clip_id {
                    let aliases: BTreeMap<_, _> = old_layers
                        .iter()
                        .filter(|l| l.scope.clip_id == e.scope.clip_id)
                        .map(|l| (l.id.clone(), format!("{}-{}", l.id, scope.clip_id)))
                        .collect();
                    if let Some(id) = aliases.get(&op.target) {
                        op.target = id.clone();
                    }
                    fn map(v: &mut Value, aliases: &BTreeMap<String, String>) {
                        match v {
                            Value::String(s) => {
                                if let Some(id) = s.strip_prefix('@').and_then(|id| aliases.get(id))
                                {
                                    *s = format!("@{id}");
                                }
                            }
                            Value::Array(v) => v.iter_mut().for_each(|v| map(v, aliases)),
                            Value::Object(o) => o.values_mut().for_each(|v| map(v, aliases)),
                            _ => {}
                        }
                    }
                    map(&mut op.call.params, &aliases);
                    for c in op.selection.iter_mut().chain(op.context.iter_mut()) {
                        map(&mut c.params, &aliases);
                    }
                }
                op
            })
        })
        .collect();
    p.layers = p
        .layers
        .iter()
        .flat_map(|e| {
            remap(&e.scope)
                .into_iter()
                .map(|scope| crate::model::Layer {
                    scope: scope.clone(),
                    id: if scope.clip_id == e.scope.clip_id {
                        e.id.clone()
                    } else {
                        format!("{}-{}", e.id, scope.clip_id)
                    },
                    ..e.clone()
                })
        })
        .collect();
    p.workspace.selection = p
        .workspace
        .selection
        .as_ref()
        .and_then(|s| remap(s).into_iter().next());
    if p.workspace
        .selected_layer
        .as_ref()
        .is_some_and(|id| !p.layers.iter().any(|l| &l.id == id))
    {
        p.workspace.selected_layer = None;
    }
    Ok(())
}

pub fn frame_plan(p: &SharedProject, frame: u32) -> Result<Value, String> {
    let seq = p.timeline.as_ref().ok_or("Tidslinjen saknas")?;
    let t = p.fps.tick_of(i64::from(frame));
    let mut requests = vec![];
    for tr in seq.video_tracks.iter().filter(|t| t.enabled) {
        for c in &tr.items {
            if !c.enabled {
                continue;
            }
            let transition = tr
                .transitions
                .iter()
                .any(|x| x.range().contains(t) && (x.from == Some(c.id) || x.to == Some(c.id)));
            if !c.range().contains(t) && !transition {
                continue;
            }
            if p.resources
                .iter()
                .find(|i| i.id == c.item)
                .is_some_and(|i| matches!(i.kind, ItemKind::AdjustmentLayer { .. }))
            {
                continue;
            }
            let clip = format!("clip-{}", c.id.0);
            let Some(view) = p.clips.iter().find(|i| i.id == clip) else {
                continue;
            };
            requests.extend(source_requests(p, c, view, t));
        }
    }
    if requests.len() > 32 {
        return Err("För många samtidiga videokällor".into());
    }
    Ok(Value::Array(requests))
}
fn source_requests(
    p: &SharedProject,
    c: &filmcraft_project::TrackItem,
    view: &crate::model::Clip,
    t: Tick,
) -> Vec<Value> {
    let asset = p.assets.iter().find(|a| a.id == view.asset_id);
    let rate = asset
        .and_then(|a| a.source_fps)
        .map(FrameRate::from_f64)
        .unwrap_or(p.fps);
    let mt = c.source_time_at(t);
    let time = rate.tick_of(rate.frame_at(mt)).seconds().max(0.0);
    let local = p
        .fps
        .frame_at(t - c.start)
        .clamp(0, i64::from(view.frames) - 1) as u32;
    let mut requests = vec![
        json!({"item":c.item.0,"clip":view.id,"asset_id":view.asset_id,"seconds":time,"local":local}),
    ];
    if let Some((next, _)) = filmcraft_engine::render::interpolation_blend(c, t, rate) {
        requests.push(json!({"item":c.item.0,"clip":view.id,"asset_id":view.asset_id,"seconds":next.seconds().max(0.0),"local":local}));
    }
    requests
}
pub fn clip_plan(p: &SharedProject, id: &str, frame: u32) -> Result<Value, String> {
    let seq = p.timeline.as_ref().ok_or("Tidslinjen saknas")?;
    let view = p
        .clips
        .iter()
        .find(|c| c.id == id && frame >= c.start && frame < c.start + c.frames)
        .ok_or("Bildrutan ligger utanför klippet")?;
    let (_, clip) = seq
        .find_item(ClipId(clip_id(id)?))
        .ok_or("Klippet saknas")?;
    Ok(json!(source_requests(
        p,
        clip,
        view,
        p.fps.tick_of(i64::from(frame))
    )))
}
fn prepare_placement(p: &SharedProject, c: &mut filmcraft_project::TrackItem, source: (u32, u32)) {
    let fit =
        (p.width as f64 / source.0.max(1) as f64).min(p.height as f64 / source.1.max(1) as f64);
    if let Some(m) = c.effect_mut("motion")
        && let Some(a) = m.param_mut("anchor")
        && let filmcraft_project::ParamValue::Vec2(point) = &mut a.value
    {
        point.x = point.x * fit + (p.width as f64 - source.0 as f64 * fit) / 2.0;
        point.y = point.y * fit + (p.height as f64 - source.1 as f64 * fit) / 2.0;
    }
    // The shared input has already been fitted on the project canvas.
    c.scale_to_frame = false;
}
pub fn canvas_motion(p: &SharedProject, global: u32) -> Result<[f64; 6], String> {
    let (view, _) = p.at(global).ok_or("Ingen bildruta")?;
    let seq = p.timeline.as_ref().ok_or("Tidslinjen saknas")?;
    let (_, c) = seq
        .find_item(ClipId(clip_id(&view.id)?))
        .ok_or("Klippet saknas")?;
    let source = p
        .assets
        .iter()
        .find(|a| a.id == view.asset_id)
        .map(|a| (a.width, a.height))
        .unwrap_or((p.width, p.height));
    let mut c = c.clone();
    prepare_placement(p, &mut c, source);
    let m = filmcraft_engine::render::motion_matrix(
        seq,
        &c,
        (p.width, p.height),
        c.effect_time_at(p.fps.tick_of(i64::from(global))),
    );
    Ok([m.a, m.b, m.c, m.d, m.e, m.f])
}
struct PreparedFrames {
    info: filmcraft_media::MediaInfo,
    frames: Vec<(Tick, Arc<filmcraft_frame::VideoFrame>)>,
}
impl filmcraft_media::MediaSource for PreparedFrames {
    fn info(&self) -> &filmcraft_media::MediaInfo {
        &self.info
    }
    fn video_frame(
        &self,
        req: filmcraft_media::FrameRequest,
    ) -> filmcraft_media::Result<Arc<filmcraft_frame::VideoFrame>> {
        self.frames
            .iter()
            .filter(|(t, _)| *t <= req.time + Tick(1))
            .max_by_key(|(t, _)| *t)
            .or_else(|| self.frames.first())
            .map(|(_, f)| f.clone())
            .ok_or(filmcraft_media::MediaError::NoStream("video"))
    }
    fn audio(
        &self,
        _: i64,
        _: usize,
        _: u32,
    ) -> filmcraft_media::Result<filmcraft_frame::AudioBuffer> {
        Err(filmcraft_media::MediaError::NoStream("audio"))
    }
}
#[derive(serde::Deserialize)]
struct Input {
    item: u64,
    clip: String,
    seconds: f64,
    local: u32,
    offset: usize,
    length: usize,
}
pub fn render(
    p: &SharedProject,
    w: u32,
    h: u32,
    frame: u32,
    inputs: &str,
    pixels: &[u8],
) -> Result<Vec<u8>, String> {
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > 16_777_216 || inputs.len() > 100_000 {
        return Err("Ogiltig sekvensbuffert".into());
    }
    let inputs: Vec<Input> = serde_json::from_str(inputs).map_err(|e| e.to_string())?;
    if inputs.len() > 32 {
        return Err("För många källor".into());
    }
    let mut native = derived(p)?;
    let mut prepared: BTreeMap<ItemId, PreparedFrames> = BTreeMap::new();
    for input in inputs {
        let end = input
            .offset
            .checked_add(input.length)
            .ok_or("Ogiltig buffert")?;
        let data = pixels
            .get(input.offset..end)
            .ok_or("Ofullständig bildruta")?;
        if !input.seconds.is_finite() {
            return Err("Ogiltig källtid".into());
        }
        let mut view = p.clone();
        view.workspace.active_clip_id = Some(input.clip.clone());
        let c = view
            .clips
            .iter()
            .find(|c| c.id == input.clip)
            .ok_or("Klippet saknas")?;
        let global = c.start + input.local;
        let rgba = crate::render::frame(&view, w, h, global, data)?;
        let uid = ItemId(200_000_000 + clip_id(&input.clip)?);
        let mut item = native
            .item(ItemId(input.item))
            .ok_or("Källan saknas")?
            .clone();
        item.id = uid;
        if !matches!(item.kind, ItemKind::Media(_)) {
            let asset = view
                .assets
                .iter()
                .find(|a| a.id == c.asset_id)
                .ok_or("Källan saknas")?;
            item.kind = ItemKind::Media(media(asset, p.fps)?);
            if let Some((_, clip)) = native
                .sequence_mut(ItemId(SEQUENCE_ID))
                .and_then(|s| s.find_item_mut(ClipId(clip_id(&input.clip).ok()?)))
            {
                clip.effects
                    .retain(|e| !filmcraft_project::graphic::is_layer_id(&e.effect));
            }
        }
        let source = native
            .source_size(ItemId(input.item))
            .unwrap_or((p.width, p.height));
        if let ItemKind::Media(m) = &mut item.kind {
            m.interpret.color_space = None;
            if let Some(v) = &mut m.info.video {
                v.width = p.width;
                v.height = p.height;
                v.color = filmcraft_color::ColorInfo::SRGB_FULL;
            }
        }
        let info = filmcraft_media::MediaInfo {
            name: "canvas".into(),
            kind: filmcraft_media::MediaKind::Still,
            duration: p.fps.tick_of(i64::from(crate::model::MAX_FRAMES)),
            video: Some(filmcraft_media::VideoStreamInfo {
                width: w,
                height: h,
                frame_rate: p.fps,
                par: (1, 1),
                codec: "RGBA".into(),
                pixel_format: "RGBA8".into(),
                color: filmcraft_color::ColorInfo::SRGB_FULL,
                has_alpha: true,
                bitrate: None,
                hdr: None,
            }),
            audio: None,
            container: "Canvas".into(),
            start_timecode: None,
            file_size: None,
        };
        native.items.insert(uid, item);
        if let Some((_, c)) = native
            .sequence_mut(ItemId(SEQUENCE_ID))
            .and_then(|s| s.find_item_mut(filmcraft_project::ClipId(clip_id(&input.clip).ok()?)))
        {
            if !prepared.contains_key(&uid) {
                prepare_placement(p, c, source);
            }
            c.item = uid;
        }
        let data = prepared.entry(uid).or_insert_with(|| PreparedFrames {
            info,
            frames: vec![],
        });
        if let Some(v) = &mut data.info.video {
            v.frame_rate = p
                .assets
                .iter()
                .find(|a| a.id == view.at(global).map_or("", |(c, _)| c.asset_id.as_str()))
                .and_then(|a| a.source_fps)
                .map(FrameRate::from_f64)
                .unwrap_or(p.fps);
        }
        data.frames.push((
            Tick::from_seconds_f64(input.seconds),
            Arc::new(filmcraft_frame::VideoFrame::rgba8(w, h, rgba)),
        ));
    }
    let sources: BTreeMap<ItemId, filmcraft_media::SharedSource> = prepared
        .into_iter()
        .map(|(id, source)| (id, Arc::new(source) as filmcraft_media::SharedSource))
        .collect();
    let generated = generators(&native);
    let provider = |id| {
        sources
            .get(&id)
            .cloned()
            .or_else(|| generated.get(&id).cloned())
    };
    let image = filmcraft_engine::render::render_sequence(
        &native,
        ItemId(SEQUENCE_ID),
        p.fps.tick_of(i64::from(frame)),
        filmcraft_engine::render::RenderOptions {
            scale: w as f32 / p.width as f32,
            captions: true,
            ..Default::default()
        },
        &provider,
    );
    if image.w != w as usize || image.h != h as usize {
        return Err("Sekvensens bildformat måste matcha canvasen".into());
    }
    Ok(image.to_rgba8())
}

pub fn render_clip(
    p: &SharedProject,
    clip: &str,
    w: u32,
    h: u32,
    frame: u32,
    inputs: &str,
    pixels: &[u8],
) -> Result<Vec<u8>, String> {
    let mut view = p.clone();
    view.workspace.active_clip_id = Some(clip.into());
    let id = ClipId(clip_id(clip)?);
    let seq = view.timeline.as_mut().ok_or("Tidslinjen saknas")?;
    for track in &mut seq.video_tracks {
        for c in &mut track.items {
            c.enabled = c.id == id;
        }
        track.transitions.clear();
    }
    render(&view, w, h, frame, inputs, pixels)
}
fn generators(p: &filmcraft_project::Project) -> BTreeMap<ItemId, filmcraft_media::SharedSource> {
    p.items
        .iter()
        .filter_map(|(id, item)| {
            let media = item.as_media()?;
            let filmcraft_project::MediaRef::Generator(g) = &media.media else {
                return None;
            };
            let v = media.info.video.as_ref();
            Some((
                *id,
                Arc::new(filmcraft_media::generators::GeneratorSource::new(
                    g.clone(),
                    v.map_or(1280, |v| v.width),
                    v.map_or(720, |v| v.height),
                    media.info.frame_rate(),
                    media.info.duration,
                )) as filmcraft_media::SharedSource,
            ))
        })
        .collect()
}
pub fn asset_frame(
    p: &SharedProject,
    asset: &str,
    w: u32,
    h: u32,
    seconds: f64,
    clip: Option<&str>,
) -> Result<Vec<u8>, String> {
    asset_frame_inner(p, asset, w, h, seconds, clip, 0)
}
fn asset_frame_inner(
    p: &SharedProject,
    asset: &str,
    w: u32,
    h: u32,
    seconds: f64,
    clip: Option<&str>,
    depth: u8,
) -> Result<Vec<u8>, String> {
    if !seconds.is_finite()
        || w == 0
        || h == 0
        || u64::from(w) * u64::from(h) > 16_777_216
        || depth > 8
    {
        return Err("Ogiltig eller för djup mediecanvas".into());
    }
    let i = p
        .assets
        .iter()
        .position(|a| a.id == asset)
        .ok_or("Källan saknas")?;
    let f = derived(p)?;
    let item = asset_item(p, i);
    let pi = f.item(item).ok_or("Källan saknas")?;
    if let ItemKind::Sequence(seq) = &pi.kind {
        let mut child = p.clone();
        child.timeline = Some((**seq).clone());
        child.width = seq.settings.width;
        child.height = seq.settings.height;
        child.fps = seq.settings.frame_rate;
        child.project_look = json!({});
        sync_clips(&mut child)?;
        let frame = child.fps.frame_at(Tick::from_seconds_f64(seconds)).max(0) as u32;
        let requests: Vec<Value> =
            serde_json::from_value(frame_plan(&child, frame)?).map_err(|e| e.to_string())?;
        let mut inputs = vec![];
        let mut data = vec![];
        for mut req in requests {
            let id = req["asset_id"].as_str().ok_or("Källan saknas")?;
            let asset = child
                .assets
                .iter()
                .find(|a| a.id == id)
                .ok_or("Källan saknas")?;
            let seconds = req["seconds"].as_f64().ok_or("Källtiden saknas")?;
            let pixels = if asset.kind == "generator" {
                asset_frame_inner(&child, id, w, h, seconds, req["clip"].as_str(), depth + 1)?
            } else if asset.kind == "blank" {
                vec![0; w as usize * h as usize * 4]
            } else {
                let packed = crate::sources::frame(
                    asset,
                    seconds,
                    (w as f32 / asset.width as f32)
                        .min(h as f32 / asset.height as f32)
                        .min(1.0),
                )?;
                let header = packed.get(..8).ok_or("Ofullständig bildruta")?;
                let (sw, sh) = (
                    u32::from_le_bytes(header[..4].try_into().map_err(|_| "Bildbredd saknas")?),
                    u32::from_le_bytes(header[4..].try_into().map_err(|_| "Bildhöjd saknas")?),
                );
                let mut doc = photocraft_doc::Document::new(
                    "Källvy",
                    photocraft_geom::Size::new(sw, sh),
                    photocraft_doc::ColorMode::Rgb,
                    photocraft_doc::SampleType::U8,
                );
                doc.layers.push(photocraft_doc::Layer::new(
                    "Källa",
                    photocraft_doc::LayerContent::Raster(
                        photocraft_raster::Surface::from_interleaved(
                            photocraft_doc::PixelFormat::RGBA8,
                            photocraft_geom::Rect::from_size(doc.size),
                            &packed[8..],
                        ),
                    ),
                ));
                let mut session = photocraft_engine::Session::new();
                session.add_document(doc, None);
                crate::render::composite(&session, w, h)?
            };
            req["offset"] = json!(data.len());
            req["length"] = json!(pixels.len());
            data.extend(pixels);
            inputs.push(req);
        }
        return render(&child, w, h, frame, &json!(inputs).to_string(), &data);
    }
    let sources = generators(&f);
    let provider = |id| {
        sources.get(&id).cloned().or_else(|| {
            p.assets
                .iter()
                .enumerate()
                .find(|(i, _)| asset_item(p, *i) == id)
                .and_then(|(_, a)| crate::sources::get(&a.id, &a.name, a.bytes).ok())
        })
    };
    let image = if matches!(pi.kind, ItemKind::Graphic { .. }) {
        let (_, c) = p
            .timeline
            .as_ref()
            .and_then(|s| {
                clip.and_then(|c| clip_id(c).ok())
                    .and_then(|c| s.find_item(ClipId(c)))
                    .or_else(|| {
                        s.video_tracks
                            .iter()
                            .flat_map(|t| &t.items)
                            .find(|c| c.item == item)
                            .and_then(|c| s.find_item(c.id))
                    })
            })
            .ok_or("Grafikklippet saknas")?;
        let mut image = filmcraft_engine::render::Image::new(w as usize, h as usize);
        filmcraft_engine::render::graphic_clip::render_graphic(
            c,
            Tick::from_seconds_f64(seconds),
            (p.width, p.height),
            &filmcraft_geom::Affine::scale(w as f64 / p.width as f64, h as f64 / p.height as f64),
            &mut image,
        );
        image
    } else {
        filmcraft_engine::render::render_item(
            &f,
            item,
            Tick::from_seconds_f64(seconds),
            w as f32 / p.width as f32,
            &provider,
        )
        .ok_or("Källan kan inte renderas utan originalet")?
    };
    if image.w != w as usize || image.h != h as usize {
        return Err("Källans bildformat matchar inte projektet".into());
    }
    Ok(image.to_rgba8())
}
pub fn audio_plan(p: &SharedProject, start: i64, frames: usize) -> Result<Value, String> {
    if start < 0 || frames == 0 || frames > 48_000 {
        return Err("Ogiltigt ljudintervall".into());
    }
    let seq = p.timeline.as_ref().ok_or("Tidslinjen saknas")?;
    let sr = i64::from(seq.settings.sample_rate);
    let begin = Tick::from_units((start - 8 * sr).max(0), sr);
    let end = Tick::from_units(start + frames as i64 + sr, sr);
    let range = filmcraft_time::TimeRange::from_bounds(begin, end);
    let mut requests: BTreeMap<ItemId, (i64, i64)> = BTreeMap::new();
    for tr in seq.audio_tracks.iter().filter(|t| t.enabled && !t.muted) {
        for c in &tr.items {
            if !c.enabled || !c.range().overlaps(&range) {
                continue;
            }
            let a = c.source_time_at(begin.max(c.start)).to_units_floor(sr);
            let b = c.source_time_at(end.min(c.end())).to_units_floor(sr);
            let lo = a.min(b).max(0);
            let hi = a.max(b) + 2;
            requests
                .entry(c.item)
                .and_modify(|v| {
                    v.0 = v.0.min(lo);
                    v.1 = v.1.max(hi);
                })
                .or_insert((lo, hi));
        }
    }
    let mut out = vec![];
    for (item, (start, end)) in requests {
        let Some((i, a)) = p
            .assets
            .iter()
            .enumerate()
            .find(|(i, _)| asset_item(p, *i) == item)
        else {
            continue;
        };
        let _ = i;
        if end - start > 32 * sr {
            return Err("För stor ljudavkodningsbuffert. Minska uppspelningshastigheten eller dela ljudklippet".into());
        }
        out.push(json!({"item":item.0,"asset_id":a.id,"start":start,"frames":end-start,"sample_rate":sr}));
    }
    Ok(Value::Array(out))
}
#[derive(serde::Deserialize)]
struct AudioInput {
    item: u64,
    start: i64,
    frames: usize,
    channels: usize,
    offset: usize,
}
struct ChunkAudio {
    info: filmcraft_media::MediaInfo,
    start: i64,
    data: filmcraft_frame::AudioBuffer,
}
impl filmcraft_media::MediaSource for ChunkAudio {
    fn info(&self) -> &filmcraft_media::MediaInfo {
        &self.info
    }
    fn video_frame(
        &self,
        _: filmcraft_media::FrameRequest,
    ) -> filmcraft_media::Result<Arc<filmcraft_frame::VideoFrame>> {
        Err(filmcraft_media::MediaError::NoStream("video"))
    }
    fn audio(
        &self,
        start: i64,
        frames: usize,
        sample_rate: u32,
    ) -> filmcraft_media::Result<filmcraft_frame::AudioBuffer> {
        if sample_rate != self.data.sample_rate || frames > 4_000_000 {
            return Err(filmcraft_media::MediaError::Unsupported(
                "Ljudformatet matchar inte sekvensen".into(),
            ));
        }
        let mut out =
            filmcraft_frame::AudioBuffer::silence(sample_rate, self.data.channels.len(), frames);
        for (dest, src) in out.channels.iter_mut().zip(&self.data.channels) {
            for (i, d) in dest.iter_mut().enumerate() {
                if let Ok(j) = usize::try_from(start + i as i64 - self.start)
                    && let Some(s) = src.get(j)
                {
                    *d = *s;
                }
            }
        }
        Ok(out)
    }
}
pub fn mix_audio(
    p: &SharedProject,
    start: i64,
    frames: usize,
    inputs: &str,
    samples: &[f32],
) -> Result<Vec<f32>, String> {
    if start < 0
        || frames == 0
        || frames > 48_000
        || samples.len() > 32_000_000
        || inputs.len() > 100_000
    {
        return Err("Ogiltig ljudbuffert".into());
    }
    let inputs: Vec<AudioInput> = serde_json::from_str(inputs).map_err(|e| e.to_string())?;
    if inputs.len() > 64 {
        return Err("För många ljudkällor".into());
    }
    let f = derived(p)?;
    let seq = f.sequence(ItemId(SEQUENCE_ID)).ok_or("Tidslinjen saknas")?;
    let mut sources = generators(&f);
    for input in inputs {
        if input.channels == 0 || input.channels > 6 || input.frames > 4_000_000 {
            return Err("Ogiltig ljudkälla".into());
        }
        let item = ItemId(input.item);
        let media = f
            .item(item)
            .and_then(|i| i.as_media())
            .ok_or("Ljudkällan saknas")?;
        let mut channels = vec![];
        for c in 0..input.channels {
            let lo = c
                .checked_mul(input.frames)
                .and_then(|n| input.offset.checked_add(n))
                .ok_or("Ogiltig ljudbuffert")?;
            let hi = lo.checked_add(input.frames).ok_or("Ogiltig ljudbuffert")?;
            let src = samples.get(lo..hi).ok_or("Ofullständigt ljud")?;
            if src.iter().any(|f| !f.is_finite()) {
                return Err("Ogiltiga ljudvärden".into());
            }
            channels.push(src.to_vec());
        }
        sources.insert(
            item,
            Arc::new(ChunkAudio {
                info: media.info.clone(),
                start: input.start,
                data: filmcraft_frame::AudioBuffer {
                    sample_rate: seq.settings.sample_rate,
                    channels,
                },
            }),
        );
    }
    let provider = |id| {
        sources.get(&id).cloned().or_else(|| {
            p.assets
                .iter()
                .enumerate()
                .find(|(i, _)| asset_item(p, *i) == id)
                .and_then(|(_, a)| crate::sources::get(&a.id, &a.name, a.bytes).ok())
        })
    };
    let mix = filmcraft_engine::render::audio::mix_sequence(&f, seq, start, frames, &provider);
    Ok(mix.channels.into_iter().flatten().collect())
}
