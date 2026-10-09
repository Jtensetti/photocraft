use crate::model::Project;
use photocraft_doc::{
    ColorMode, Document, Layer, LayerContent, LayerMask, PixelFormat, SampleType,
};
use photocraft_geom::{Affine, Rect, Size};
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
                .map(|v| {
                    StrokePoint::new(
                        (v[0] + l.offset[0]) * f64::from(w),
                        (v[1] + l.offset[1]) * f64::from(h),
                        v[2] as f32,
                    )
                })
                .collect();
            photocraft_paint::render_stroke(&mut surface, &brush, &points, None, false, 1.0);
        }
        if let Some(text) = &l.text {
            let mut view = photocraft_doc::TextLayer {
                text: text.content.clone(),
                font_family: "Inter".into(),
                size_pt: text.size * w as f32,
                color: photocraft_doc::Color::rgba(
                    text.color[0],
                    text.color[1],
                    text.color[2],
                    text.color[3],
                ),
                transform: Affine::translate(
                    (text.position[0] + l.offset[0]) * f64::from(w),
                    (text.position[1] + l.offset[1]) * f64::from(h),
                ),
                ..Default::default()
            };
            static ENGINE: std::sync::OnceLock<std::sync::Mutex<photocraft_text::TextEngine>> =
                std::sync::OnceLock::new();
            let engine =
                ENGINE.get_or_init(|| std::sync::Mutex::new(photocraft_text::TextEngine::new()));
            engine
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .render_layer(&mut view, 72.0, PixelFormat::RGBA32F);
            surface = view.cache.ok_or("Textlagret kunde inte renderas")?;
        }
        let mut layer = Layer::new(&l.name, LayerContent::Raster(surface));
        layer.opacity = l.opacity;
        if let Some(r) = l.mask {
            let mut data = vec![0_u8; (w as usize) * (h as usize)];
            for y in 0..h {
                for x in 0..w {
                    let nx = f64::from(x) / f64::from(w);
                    let ny = f64::from(y) / f64::from(h);
                    if nx >= r[0]
                        && nx < r[2]
                        && ny >= r[1]
                        && ny < r[3]
                        && let Some(v) = data.get_mut((y as usize) * (w as usize) + x as usize)
                    {
                        *v = 255;
                    }
                }
            }
            let mut mask = LayerMask::hide_all();
            mask.surface = Surface::from_interleaved(PixelFormat::GRAY8, rect, &data);
            layer.mask = Some(mask);
        }
        doc.layers.push(layer);
    }
    Ok(photocraft_compose::render(&doc, rect).to_rgba8().pixels)
}
