//! Canvas tools invert the same LightCraft geometry and FilmCraft placement used by rendering.
use crate::model::Project;
use lightcraft_geom::{Affine, Point};
use lightcraft_pipeline::geometry::Frame;

pub struct Mapping {
    width: f64,
    height: f64,
    frame: Frame,
    output: (usize, usize),
    motion: Affine,
}
impl Mapping {
    pub fn new(p: &Project, global: u32) -> Result<Self, String> {
        let (c, f) = p.at(global).ok_or("Ingen bildruta")?;
        let settings = crate::render::settings(&p.look(&c.id, f))?;
        let frame = Frame::new(
            p.width as usize,
            p.height as usize,
            &settings,
            p.develop_canvas,
        );
        let output = frame.fit(p.width as usize, p.height as usize);
        Ok(Self {
            width: f64::from(p.width),
            height: f64::from(p.height),
            frame,
            output,
            motion: Affine(crate::film::canvas_motion(p, global)?),
        })
    }
    pub fn to_source(
        &self,
        point: [f64; 2],
        kind: &str,
        doc: (u32, u32),
    ) -> Result<[f64; 2], String> {
        let inverse = self
            .motion
            .inverse()
            .ok_or("Klippets transformation kan inte inverteras")?;
        let canvas = inverse.apply(Point::new(point[0] * self.width, point[1] * self.height));
        let (rw, rh) = self.output;
        let fitted = Point::new(
            canvas.x - ((self.width - rw as f64) / 2.0).floor(),
            canvas.y - ((self.height - rh as f64) / 2.0).floor(),
        );
        let oriented = self.frame.out_to_oriented(rw, rh).apply(fitted);
        if kind == "light" {
            return Ok([oriented.x / self.frame.ow, oriented.y / self.frame.oh]);
        }
        let src = self
            .frame
            .warp
            .as_ref()
            .map_or(oriented, |w| w.to_source(oriented, 1));
        let (x, y) = self
            .frame
            .orient
            .inverse()
            .map(src.x, src.y, self.frame.ow, self.frame.oh);
        if kind == "source" {
            return Ok([x / self.width, y / self.height]);
        }
        let (scale, ox, oy) = self.doc_fit(doc);
        Ok([(x - ox) / scale, (y - oy) / scale])
    }
    fn doc_fit(&self, doc: (u32, u32)) -> (f64, f64, f64) {
        let scale =
            (self.width / f64::from(doc.0.max(1))).min(self.height / f64::from(doc.1.max(1)));
        let w = (f64::from(doc.0) * scale).round();
        let h = (f64::from(doc.1) * scale).round();
        (
            scale,
            ((self.width - w) / 2.0).floor(),
            ((self.height - h) / 2.0).floor(),
        )
    }
    pub fn to_canvas(&self, point: [f64; 2], doc: (u32, u32)) -> [f64; 2] {
        let (scale, ox, oy) = self.doc_fit(doc);
        let (x, y) = self.frame.orient.map(
            point[0] * scale + ox,
            point[1] * scale + oy,
            self.width,
            self.height,
        );
        let source = Point::new(x, y);
        let mut transformed = source;
        if let Some(warp) = &self.frame.warp {
            // Invert the actual lens/perspective map with a bounded Newton solve.
            for _ in 0..12 {
                let q = warp.to_source(transformed, 1);
                let dx = source.x - q.x;
                let dy = source.y - q.y;
                if dx.abs() + dy.abs() < 0.0001 {
                    break;
                }
                let a = warp.to_source(Point::new(transformed.x + 0.01, transformed.y), 1);
                let b = warp.to_source(Point::new(transformed.x, transformed.y + 0.01), 1);
                let (aa, bb, cc, dd) = (
                    (a.x - q.x) * 100.0,
                    (a.y - q.y) * 100.0,
                    (b.x - q.x) * 100.0,
                    (b.y - q.y) * 100.0,
                );
                let det = aa * dd - bb * cc;
                if det.abs() < 1e-9 {
                    break;
                }
                transformed.x += (dd * dx - cc * dy) / det;
                transformed.y += (-bb * dx + aa * dy) / det;
            }
        }
        let (rw, rh) = self.output;
        let output = self.frame.norm_to_out(rw, rh).apply(Point::new(
            transformed.x / self.frame.ow,
            transformed.y / self.frame.oh,
        ));
        let result = self.motion.apply(Point::new(
            output.x + ((self.width - rw as f64) / 2.0).floor(),
            output.y + ((self.height - rh as f64) / 2.0).floor(),
        ));
        [result.x / self.width, result.y / self.height]
    }
}
