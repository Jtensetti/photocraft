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
        "source_in": rate.tick_of(i64::from(clip.source_in)), "speed": 1.0,
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
    if source != clip.source_in + start || frames != end - start {
        return Err("FilmCraft kunde inte behålla den valda omfattningen".into());
    }
    Ok((source, frames))
}
