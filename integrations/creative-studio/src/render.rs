use crate::model::Project;
use photocraft_doc::{ColorMode, Document, Layer, LayerContent, PixelFormat, SampleType};
use photocraft_geom::{Rect, Size};
use photocraft_paint::{BrushSettings, StrokePoint};
use photocraft_raster::Surface;

pub fn frame(p: &Project, w: u32, h: u32, global: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let bytes = u64::from(w) * u64::from(h) * 4;
    if w == 0 || h == 0 || bytes > 67_108_864 || bytes != rgba.len() as u64 {
        return Err("Ogiltig bildrutebuffert".into());
    }
    let (clip, local) = p.at(global).ok_or("Bildrutan ligger utanför projektet")?;
    let a = p.look(&clip.id, local);
    let rect = Rect::from_size(Size::new(w, h));
    let mut base = rgba.to_vec();
    if a.exposure != 0.0
        || a.contrast != 0.0
        || a.highlights != 0.0
        || a.shadows != 0.0
        || a.temperature != 0.0
        || a.tint != 0.0
        || a.saturation != 0.0
    {
        let matrix = lightcraft_color::SRGB.to_space(&lightcraft_color::REC2020);
        let source = lightcraft_raster::Rgb32f {
            width: w as usize,
            height: h as usize,
            data: rgba
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
        };
        let mut settings = lightcraft_develop::DevelopSettings::default();
        settings.light.exposure = a.exposure;
        settings.light.contrast = a.contrast;
        settings.light.highlights = a.highlights;
        settings.light.shadows = a.shadows;
        settings.wb.temp = 6500.0 + a.temperature * 30.0;
        settings.wb.tint = a.tint;
        settings.wb.mode = lightcraft_develop::WbMode::Custom;
        settings.color.saturation = a.saturation;
        let result = lightcraft_pipeline::render(
            &source,
            &lightcraft_pipeline::SourceInfo::default(),
            &settings,
            &lightcraft_pipeline::RenderRequest::fit(w as usize, h as usize),
        );
        for (dst, src) in base
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(&result.image.data)
        {
            dst[..3].copy_from_slice(&src[..3]);
        }
    }
    let mut doc = Document::new(
        "Frame view",
        Size::new(w, h),
        ColorMode::Rgb,
        SampleType::F32,
    );
    doc.layers.push(Layer::new(
        "Source",
        LayerContent::Raster(Surface::from_interleaved(PixelFormat::RGBA8, rect, &base)),
    ));
    for l in p
        .layers
        .iter()
        .filter(|l| l.visible && l.scope.contains(&clip.id, local))
    {
        let mut surface = Surface::new(PixelFormat::RGBA32F);
        for s in &l.strokes {
            let brush = BrushSettings {
                size: s.size * w as f32,
                color: s.color,
                erase: s.erase,
                pressure_size: false,
                hardness: 0.85,
                ..Default::default()
            };
            let points: Vec<_> = s
                .points
                .iter()
                .map(|v| StrokePoint::new(v[0] * f64::from(w), v[1] * f64::from(h), v[2] as f32))
                .collect();
            photocraft_paint::render_stroke(&mut surface, &brush, &points, None, false, 1.0);
        }
        let mut layer = Layer::new(&l.name, LayerContent::Raster(surface));
        layer.opacity = l.opacity;
        doc.layers.push(layer);
    }
    Ok(photocraft_compose::render(&doc, rect).to_rgba8().pixels)
}
