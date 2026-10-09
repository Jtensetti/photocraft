use crate::model::Project;
use photocraft_doc::{
    ColorMode, Document, Layer, LayerContent, LayerMask, PixelFormat, SampleType,
};
use photocraft_geom::{Affine, Point, Rect, Size};
use photocraft_paint::{BrushSettings, StrokePoint};
use photocraft_raster::Surface;

pub fn document(
    p: &Project,
    w: u32,
    h: u32,
    global: u32,
    rgba: &[u8],
) -> Result<
    (
        Document,
        std::collections::BTreeMap<String, photocraft_doc::LayerId>,
    ),
    String,
> {
    let bytes = u64::from(w) * u64::from(h) * 4;
    if w == 0 || h == 0 || bytes > 67_108_864 || bytes != rgba.len() as u64 {
        return Err("Ogiltig bildrutebuffert".into());
    }
    let (clip, local) = p.at(global).ok_or("Bildrutan ligger utanför projektet")?;
    let a = p.look(&clip.id, local);
    let rect = Rect::from_size(Size::new(w, h));
    let mut base = rgba.to_vec();
    if !p.develop_canvas {
        base = develop(&a, w, h, &base)?;
    }
    let mut aliases = std::collections::BTreeMap::new();
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
    if p.develop_canvas
        && let Some(asset) = p.assets.iter().find(|a| a.id == clip.asset_id)
        && let Some(original) = crate::sources::photo_document(asset, w, h)?
    {
        doc = original;
        fn aliases_for(
            layers: &[Layer],
            aliases: &mut std::collections::BTreeMap<String, photocraft_doc::LayerId>,
            count: &mut usize,
        ) {
            for l in layers {
                aliases.insert(format!("original:{}", *count), l.id);
                *count += 1;
                if let Some(child) = l.children() {
                    aliases_for(child, aliases, count);
                }
            }
        }
        aliases_for(&doc.layers, &mut aliases, &mut 0);
    }
    let source = doc.layers.first().ok_or("Originalet saknar lager")?.id;
    aliases.insert("source".into(), source);
    for l in p
        .layers
        .iter()
        .filter(|l| l.visible && l.scope.contains(&clip.id, local))
    {
        let mut surface = Surface::new(PixelFormat::RGBA32F);
        let transform = Affine::translate(-f64::from(w) / 2.0, -f64::from(h) / 2.0)
            .then(&Affine::scale(l.scale))
            .then(&Affine::rotate(l.rotation.to_radians()))
            .then(&Affine::translate(
                f64::from(w) / 2.0 + l.offset[0] * f64::from(w),
                f64::from(h) / 2.0 + l.offset[1] * f64::from(h),
            ));
        for s in &l.strokes {
            let brush = BrushSettings {
                size: (f64::from(s.size) * f64::from(w) * l.scale).min(4096.0) as f32,
                color: s.color,
                erase: s.erase,
                pressure_size: s.pressure_size,
                hardness: s.hardness,
                opacity: s.opacity,
                flow: s.flow,
                ..Default::default()
            };
            let points: Vec<_> = s
                .points
                .iter()
                .map(|v| {
                    let point =
                        transform.apply(Point::new(v[0] * f64::from(w), v[1] * f64::from(h)));
                    StrokePoint::new(point.x, point.y, v[2] as f32)
                })
                .collect();
            let selection = s.selection.map(|r| selection_surface(w, h, r));
            photocraft_paint::render_stroke(
                &mut surface,
                &brush,
                &points,
                selection.as_ref(),
                false,
                1.0,
            );
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
                    text.position[0] * f64::from(w),
                    text.position[1] * f64::from(h),
                )
                .then(&transform),
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
        layer.blend = l.blend;
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
        aliases.insert(l.id.clone(), layer.id);
        doc.layers.push(layer);
    }
    Ok((doc, aliases))
}

pub fn frame(p: &Project, w: u32, h: u32, global: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let (session, _) = crate::native::photo_session(p, w, h, global, rgba)?;
    let pixels = composite(&session, w, h)?;
    let (clip, local) = p.at(global).ok_or("Ingen bildruta")?;
    if p.develop_canvas {
        develop(&p.look(&clip.id, local), w, h, &pixels)
    } else {
        Ok(pixels)
    }
}

pub fn composite(session: &photocraft_engine::Session, w: u32, h: u32) -> Result<Vec<u8>, String> {
    let active = session.active().ok_or("Ingen canvas")?;
    let floating = photocraft_engine::float_cmds::displayed(active, (0, 0));
    let doc = floating.as_ref().unwrap_or(&active.doc);
    let mut fit = photocraft_engine::Session::new();
    fit.add_document(doc.clone(), None);
    if doc.size != Size::new(w, h) {
        let scale = (f64::from(w) / f64::from(doc.size.width.max(1)))
            .min(f64::from(h) / f64::from(doc.size.height.max(1)));
        let width = (f64::from(doc.size.width) * scale).round().max(1.0) as u32;
        let height = (f64::from(doc.size.height) * scale).round().max(1.0) as u32;
        fit.execute(
            "image.imageSize",
            serde_json::json!({"width":width,"height":height,"resample":true}),
        )
        .map_err(|e| e.to_string())?;
        fit.execute(
            "image.canvasSize",
            serde_json::json!({"width":w,"height":h,"anchor":"center","color":[0,0,0,0]}),
        )
        .map_err(|e| e.to_string())?;
    }
    let shown = &fit.active().ok_or("Ingen canvas")?.doc;
    Ok(
        photocraft_compose::render(shown, Rect::from_size(Size::new(w, h)))
            .to_rgba8()
            .pixels,
    )
}

fn selection_surface(w: u32, h: u32, r: [f64; 4]) -> Surface {
    let mut data = vec![0_u8; (w as usize) * (h as usize)];
    for y in 0..h {
        for x in 0..w {
            let nx = f64::from(x) / f64::from(w);
            let ny = f64::from(y) / f64::from(h);
            if nx >= r[0]
                && nx < r[2]
                && ny >= r[1]
                && ny < r[3]
                && let Some(value) = data.get_mut((y as usize) * (w as usize) + x as usize)
            {
                *value = 255;
            }
        }
    }
    Surface::from_interleaved(PixelFormat::GRAY8, Rect::from_size(Size::new(w, h)), &data)
}

pub fn develop(
    a: &crate::model::Adjustments,
    w: u32,
    h: u32,
    rgba: &[u8],
) -> Result<Vec<u8>, String> {
    let mut base = rgba.to_vec();
    if a.exposure != 0.0
        || a.contrast != 0.0
        || a.highlights != 0.0
        || a.shadows != 0.0
        || a.temperature != 0.0
        || a.tint != 0.0
        || a.saturation != 0.0
        || !a.advanced.is_empty()
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
        let settings = settings(a)?;
        let result = lightcraft_pipeline::render(
            &source,
            &lightcraft_pipeline::SourceInfo::default(),
            &settings,
            &lightcraft_pipeline::RenderRequest::fit(w as usize, h as usize),
        );
        let rw = result.image.width;
        let rh = result.image.height;
        let alpha_src = lightcraft_raster::Rgb32f {
            width: w as usize,
            height: h as usize,
            data: rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| [f32::from(c[3]) / 255.0; 3])
                .collect(),
        };
        let geometry =
            lightcraft_pipeline::frame_for(&source, &Default::default(), &settings, true);
        let alpha = geometry.sample(&alpha_src, rw, rh);
        let mapping = geometry.out_to_oriented(rw, rh);
        base.fill(0);
        let ox = (w as usize - rw) / 2;
        let oy = (h as usize - rh) / 2;
        for y in 0..rh {
            for x in 0..rw {
                let i = y * rw + x;
                let j = ((y + oy) * w as usize + x + ox) * 4;
                base[j..j + 3].copy_from_slice(&result.image.data[i][..3]);
                let point =
                    mapping.apply(lightcraft_geom::Point::new(x as f64 + 0.5, y as f64 + 0.5));
                let point = geometry
                    .warp
                    .as_ref()
                    .map_or(point, |warp| warp.to_source(point, 1));
                base[j + 3] = if point.x < 0.0
                    || point.y < 0.0
                    || point.x >= geometry.ow
                    || point.y >= geometry.oh
                {
                    0
                } else {
                    (alpha.data[i][0].clamp(0.0, 1.0) * 255.0).round() as u8
                };
            }
        }
    }
    Ok(base)
}

pub fn settings(
    a: &crate::model::Adjustments,
) -> Result<lightcraft_develop::DevelopSettings, String> {
    let mut settings = match a.advanced.get("settings") {
        Some(v) => serde_json::from_value(v.clone()).map_err(|e| e.to_string())?,
        None => lightcraft_develop::DevelopSettings::default(),
    };
    settings.light.exposure = a.exposure;
    settings.light.contrast = a.contrast;
    settings.light.highlights = a.highlights;
    settings.light.shadows = a.shadows;
    let temperature = 6500.0 + a.temperature * 30.0;
    if !a.advanced.contains_key("settings")
        || (settings.wb.temp - temperature).abs() > 0.00001
        || settings.wb.tint != a.tint
    {
        settings.wb.mode = lightcraft_develop::WbMode::Custom;
    }
    settings.wb.temp = temperature;
    settings.wb.tint = a.tint;
    settings.color.saturation = a.saturation;
    for (key, value) in &a.advanced {
        match key.as_str() {
            "settings" => {}
            "treatment" => {
                settings.treatment =
                    serde_json::from_value(value.clone()).map_err(|e| e.to_string())?
            }
            "curve.master" => {
                settings.curve.master =
                    serde_json::from_value(value.clone()).map_err(|e| e.to_string())?
            }
            "curve.red" => {
                settings.curve.red =
                    serde_json::from_value(value.clone()).map_err(|e| e.to_string())?
            }
            "curve.green" => {
                settings.curve.green =
                    serde_json::from_value(value.clone()).map_err(|e| e.to_string())?
            }
            "curve.blue" => {
                settings.curve.blue =
                    serde_json::from_value(value.clone()).map_err(|e| e.to_string())?
            }
            _ => {
                if let Some(n) = value.as_f64() {
                    lightcraft_develop::controls::set(&mut settings, key, n);
                }
            }
        }
    }
    Ok(settings)
}
