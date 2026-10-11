//! FilmCraft's software H.264 encoder is the browser fallback. Encoded frames are streamed out.
use filmcraft_h264enc::{Encoder, EncoderConfig, PacketFormat, Preset, RateControl, YuvFrame};
pub struct Video {
    encoder: Encoder,
    width: u32,
    height: u32,
    next: u32,
    y: Vec<u8>,
    u: Vec<u8>,
    v: Vec<u8>,
}
impl Video {
    pub fn new(p: &crate::model::Project) -> Result<Self, String> {
        let mut cfg = EncoderConfig::new(p.width, p.height, p.fps.num as u32, p.fps.den as u32);
        cfg.threads = 1;
        cfg.slices = 1;
        cfg.bframes = 0;
        cfg.format = PacketFormat::LengthPrefixed;
        cfg.preset = Preset::Speed;
        cfg.rate = RateControl::Vbr {
            target_kbps: 4000,
            max_kbps: 8000,
        };
        cfg.keyint = p.fps.as_f64().round().max(1.0) as u32 * 2;
        let encoder = Encoder::new(cfg).map_err(|e| e.to_string())?;
        Ok(Self {
            encoder,
            width: p.width,
            height: p.height,
            next: 0,
            y: vec![],
            u: vec![],
            v: vec![],
        })
    }
    pub fn config(&self) -> Vec<u8> {
        self.encoder.avcc()
    }
    pub fn frame(&mut self, index: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
        if index != self.next || rgba.len() != self.width as usize * self.height as usize * 4 {
            return Err("Fel bildruta eller buffert i videoexporten".into());
        }
        filmcraft_export::rgba_to_yuv420_8(
            rgba,
            self.width as usize,
            self.height as usize,
            &mut self.y,
            &mut self.u,
            &mut self.v,
        );
        let frame = YuvFrame {
            y: &self.y,
            u: &self.u,
            v: &self.v,
            y_stride: self.width as usize,
            uv_stride: self.width.div_ceil(2) as usize,
        };
        let packets = self
            .encoder
            .try_encode(&frame, i64::from(index))
            .map_err(|e| e.to_string())?;
        if packets.len() != 1 {
            return Err("Kodaren gav ett oväntat antal bildrutor".into());
        }
        let packet = packets
            .into_iter()
            .next()
            .ok_or("Den kodade bildrutan saknas")?;
        let mut bytes = Vec::with_capacity(packet.data.len() + 1);
        bytes.push(u8::from(packet.keyframe));
        bytes.extend(packet.data);
        self.next += 1;
        Ok(bytes)
    }
}
