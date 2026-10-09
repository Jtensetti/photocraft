//! Original Blob adapters and bounded derived decoder caches. No media is copied into project JSON.
use filmcraft_media::{
    SharedSource,
    reader::{ByteReader, SharedReader},
};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::VecDeque, io, sync::Arc};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = r#"
export function read_studio_blob(id, offset, length) {
  const blob = globalThis.__studioBlobs?.get(id);
  if (!blob) throw new Error('Originalet saknas. Återlänka filen.');
  if (length > 67108864) throw new Error('Avkodaren begärde en för stor databuffert.');
  return new Uint8Array(new FileReaderSync().readAsArrayBuffer(blob.slice(offset, offset + length)));
}
export function report_studio_panic(message) {
  globalThis.__studioPanic = message;
  console.error(message);
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    fn read_studio_blob(id: &str, offset: f64, length: u32) -> Result<Vec<u8>, JsValue>;
    fn report_studio_panic(message: &str);
}
pub fn install_error_hook() {
    #[cfg(target_arch = "wasm32")]
    std::panic::set_hook(Box::new(|info| report_studio_panic(&info.to_string())));
}
struct BlobReader {
    id: String,
    length: u64,
}
impl ByteReader for BlobReader {
    fn len(&self) -> u64 {
        self.length
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        if buf.len() > 67_108_864
            || offset
                .checked_add(buf.len() as u64)
                .is_none_or(|n| n > self.length)
        {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Ogiltigt medieintervall",
            ));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let bytes =
                read_studio_blob(&self.id, offset as f64, buf.len() as u32).map_err(|e| {
                    io::Error::other(
                        e.as_string()
                            .unwrap_or_else(|| "Originalet kan inte läsas".into()),
                    )
                })?;
            if bytes.len() != buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Ofullständigt original",
                ));
            }
            buf.copy_from_slice(&bytes);
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = &self.id;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Blob-avkodning körs i webbarbetaren",
            ))
        }
    }
}
#[derive(Clone)]
struct Cached {
    id: String,
    source: SharedSource,
    photo: Option<Arc<photocraft_doc::Document>>,
}
thread_local! {static SOURCES:RefCell<VecDeque<Cached>>=const {RefCell::new(VecDeque::new())};}
fn remember(entry: Cached) {
    SOURCES.with(|s| {
        let mut s = s.borrow_mut();
        s.retain(|c| c.id != entry.id);
        s.push_back(entry);
        while s.len() > 2 {
            s.pop_front();
        }
    });
}
pub fn clear() {
    SOURCES.with(|s| s.borrow_mut().clear());
}
pub fn forget(id: &str) {
    SOURCES.with(|s| s.borrow_mut().retain(|c| c.id != id));
}
pub fn get(id: &str, name: &str, length: u64) -> Result<SharedSource, String> {
    if let Some(c) = SOURCES.with(|s| s.borrow().iter().find(|c| c.id == id).cloned()) {
        remember(c.clone());
        return Ok(c.source);
    }
    let reader: SharedReader = Arc::new(BlobReader {
        id: id.into(),
        length,
    });
    let photo = name.rsplit('.').next().is_some_and(|ext| {
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "webp"
                | "avif"
                | "psd"
                | "psb"
                | "pcraft"
                | "tif"
                | "tiff"
                | "dng"
                | "cr2"
                | "nef"
                | "arw"
                | "raf"
                | "rw2"
                | "orf"
                | "bmp"
                | "svg"
                | "exr"
                | "hdr"
        )
    });
    let (source, doc) = if photo {
        if length > 67_108_864 {
            return Err("Bilden är för stor för webbimport (högst 64 MB)".into());
        }
        let bytes = filmcraft_media::reader::read_range(reader.as_ref(), 0, length as usize)
            .map_err(|e| e.to_string())?;
        let imported = photocraft_io::import(name, &bytes).map_err(|e| e.to_string())?;
        let d = imported.document;
        let (w, h) = (d.size.width, d.size.height);
        if u64::from(w) * u64::from(h) > 16_777_216 {
            return Err("Bilden överskrider 16 miljoner pixlar".into());
        }
        let rgba = photocraft_compose::render(&d, photocraft_geom::Rect::from_size(d.size))
            .to_rgba8()
            .pixels;
        (
            Arc::new(filmcraft_media::still::StillSource::from_rgba(
                name, w, h, rgba, true, length,
            )) as SharedSource,
            Some(Arc::new(d)),
        )
    } else if name.rsplit('.').next().is_some_and(|ext| {
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "mp3" | "flac" | "aif" | "aiff" | "wav"
        )
    }) {
        (
            Arc::new(crate::stream_audio::StreamAudio::open(name, reader)?) as SharedSource,
            None,
        )
    } else {
        // Containers are read by index and GOP/sample. A movie is never read whole.
        let source = filmcraft_media::reader::open_reader_within(
            name,
            reader,
            &filmcraft_codecs::reader_openers(),
            &filmcraft_codecs::openers(),
            0,
        )
        .map_err(|e| e.to_string())?;
        (source, None)
    };
    if source
        .info()
        .video
        .as_ref()
        .is_some_and(|v| u64::from(v.width) * u64::from(v.height) > 16_777_216)
    {
        return Err("Mediet överskrider 16 miljoner pixlar".into());
    }
    remember(Cached {
        id: id.into(),
        source: source.clone(),
        photo: doc,
    });
    Ok(source)
}
pub fn probe(id: &str, name: &str, length: u64) -> Result<Value, String> {
    let s = get(id, name, length)?;
    let info = s.info();
    let v = info.video.as_ref();
    let a = info.audio.as_ref();
    let photo = SOURCES.with(|s| {
        s.borrow()
            .iter()
            .find(|c| c.id == id)
            .is_some_and(|c| c.photo.is_some())
    });
    Ok(
        json!({"kind":if v.is_none(){"audio"}else if photo{"image"}else{"video"},"width":v.map_or(1,|v|v.width),"height":v.map_or(1,|v|v.height),"duration":info.duration.seconds(),"fps":v.map(|v|v.frame_rate.as_f64()),"sample_rate":a.map(|a|a.sample_rate),"channels":a.map(|a|a.channels),"source_decoder":if photo{"photo"}else{"native"},"info":info}),
    )
}
pub fn frame(asset: &crate::model::Asset, seconds: f64, scale: f32) -> Result<Vec<u8>, String> {
    if !seconds.is_finite() || !(0.001..=1.0).contains(&scale) {
        return Err("Ogiltig källbildruta".into());
    }
    let source = get(&asset.id, &asset.name, asset.bytes)?;
    let f = source
        .video_frame(filmcraft_media::FrameRequest {
            time: filmcraft_time::Tick::from_seconds_f64(seconds),
            scale,
        })
        .map_err(|e| e.to_string())?;
    let bytes = f.to_rgba8();
    if bytes.len() > 67_108_864 {
        return Err("Bildrutan är för stor".into());
    }
    let mut out = Vec::with_capacity(bytes.len() + 8);
    out.extend(f.width.to_le_bytes());
    out.extend(f.height.to_le_bytes());
    out.extend(bytes);
    Ok(out)
}
pub fn audio(
    asset: &crate::model::Asset,
    start: i64,
    frames: usize,
    rate: u32,
) -> Result<Vec<f32>, String> {
    if start < 0 || frames > 1_600_000 || !(8000..=192000).contains(&rate) {
        return Err("Ogiltig ljudbegäran".into());
    }
    let source = get(&asset.id, &asset.name, asset.bytes)?;
    let audio = source
        .audio(start, frames, rate)
        .map_err(|e| e.to_string())?;
    if audio.channels.is_empty()
        || audio.channels.len() > 6
        || audio.channels.iter().any(|c| c.len() != frames)
    {
        return Err("Ogiltigt antal ljudkanaler".into());
    }
    let mut out = Vec::with_capacity(1 + audio.channels.len() * frames);
    out.push(audio.channels.len() as f32);
    for ch in audio.channels {
        out.extend(ch);
    }
    Ok(out)
}
pub fn photo_document(
    asset: &crate::model::Asset,
    w: u32,
    h: u32,
) -> Result<Option<photocraft_doc::Document>, String> {
    if asset.source_decoder != "photo" {
        return Ok(None);
    }
    let _ = get(&asset.id, &asset.name, asset.bytes)?;
    let doc = SOURCES.with(|s| {
        s.borrow()
            .iter()
            .find(|c| c.id == asset.id)
            .and_then(|c| c.photo.clone())
    });
    let Some(doc) = doc else { return Ok(None) };
    let mut s = photocraft_engine::Session::new();
    s.add_document((*doc).clone(), None);
    let ratio = (w as f64 / doc.size.width as f64).min(h as f64 / doc.size.height as f64);
    let rw = (doc.size.width as f64 * ratio).round().max(1.0) as u32;
    let rh = (doc.size.height as f64 * ratio).round().max(1.0) as u32;
    if rw != doc.size.width || rh != doc.size.height {
        s.execute("image.imageSize", json!({"width":rw,"height":rh}))
            .map_err(|e| e.to_string())?;
    }
    if rw != w || rh != h {
        s.execute(
            "image.canvasSize",
            json!({"width":w,"height":h,"anchor":"center","extensionColor":"transparent"}),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(s.active().map(|d| (*d.doc).clone()))
}
