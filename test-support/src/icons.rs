//! Rasterizer for the approved, deliberately restricted SVG mark.
use crate::*;
use std::{collections::HashMap, io::Write};
#[derive(Clone)]
struct Shape {
    fill: Option<[u8; 3]>,
    stroke: Option<[u8; 3]>,
    geometry: Geometry,
}
#[derive(Clone)]
enum Geometry {
    Rect {
        left: f64,
        top: f64,
        width: f64,
        height: f64,
        radius: f64,
    },
    Circle {
        cx: f64,
        cy: f64,
        radius: f64,
        half_stroke: f64,
    },
    Line {
        x1: f64,
        y1: f64,
        dx: f64,
        dy: f64,
        length_squared: f64,
        half_stroke: f64,
    },
    Polygon(Vec<(f64, f64)>),
}
fn color(value: Option<&str>) -> Result<Option<[u8; 3]>> {
    match value {
        None | Some("none") => Ok(None),
        Some(v) if v.len() == 7 && v.starts_with('#') => Ok(Some([
            u8::from_str_radix(&v[1..3], 16)?,
            u8::from_str_radix(&v[3..5], 16)?,
            u8::from_str_radix(&v[5..7], 16)?,
        ])),
        _ => Err("icon color must be #RRGGBB or none".into()),
    }
}
fn parse(source: &str) -> Result<Vec<Shape>> {
    let doc = roxmltree::Document::parse(source)?;
    let root = doc.root_element();
    if root.attribute("viewBox") != Some("0 0 128 128") {
        return Err("icon viewBox must be 0 0 128 128".into());
    }
    let mut shapes = vec![];
    for element in root.children().filter(|n| n.is_element()) {
        let kind = element.tag_name().name();
        if kind == "title" {
            continue;
        }
        if !["rect", "circle", "line", "polygon"].contains(&kind) {
            return Err(format!("unsupported icon element: {kind}").into());
        }
        let fill = color(element.attribute("fill"))?;
        let stroke = color(element.attribute("stroke"))?;
        let mut numbers: HashMap<String, f64> = HashMap::new();
        let mut points = vec![];
        for attribute in element.attributes() {
            match attribute.name() {
                "fill" | "stroke" => {}
                "points" => {
                    for p in attribute.value().split_whitespace() {
                        let (x, y) = p.split_once(',').ok_or("invalid polygon point")?;
                        points.push((x.parse()?, y.parse()?));
                    }
                }
                "stroke-linecap" => {
                    if attribute.value() != "round" {
                        return Err("only round caps are supported".into());
                    }
                }
                key => {
                    numbers.insert(key.into(), attribute.value().parse()?);
                }
            }
        }
        if kind == "line" && element.attribute("stroke-linecap") != Some("round") {
            return Err("icon lines require round caps".into());
        }
        let required: &[&str] = match kind {
            "rect" => &["x", "y", "width", "height"],
            "circle" => &["cx", "cy", "r"],
            "line" => &["x1", "y1", "x2", "y2"],
            _ => &[],
        };
        if required.iter().any(|key| !numbers.contains_key(*key))
            || (kind == "polygon" && points.len() < 3)
        {
            return Err("icon shape is incomplete".into());
        }
        if kind == "line" && numbers["x1"] == numbers["x2"] && numbers["y1"] == numbers["y2"] {
            return Err("zero-length icon line".into());
        }
        // Resolve attributes and shape-only arithmetic once, outside pixel sampling.
        let n = |key: &str, default: f64| *numbers.get(key).unwrap_or(&default);
        let geometry = match kind {
            "rect" => {
                let width = n("width", 0.);
                let height = n("height", 0.);
                Geometry::Rect {
                    left: n("x", 0.),
                    top: n("y", 0.),
                    width,
                    height,
                    radius: n("rx", 0.).min(width / 2.).min(height / 2.),
                }
            }
            "circle" => Geometry::Circle {
                cx: n("cx", 0.),
                cy: n("cy", 0.),
                radius: n("r", 0.),
                half_stroke: n("stroke-width", 1.) / 2.,
            },
            "line" => {
                let x1 = n("x1", 0.);
                let y1 = n("y1", 0.);
                let dx = n("x2", 0.) - x1;
                let dy = n("y2", 0.) - y1;
                Geometry::Line {
                    x1,
                    y1,
                    dx,
                    dy,
                    length_squared: dx * dx + dy * dy,
                    half_stroke: n("stroke-width", 1.) / 2.,
                }
            }
            _ => Geometry::Polygon(points),
        };
        shapes.push(Shape {
            fill,
            stroke,
            geometry,
        });
    }
    Ok(shapes)
}
impl Shape {
    fn paint(&self, x: f64, y: f64) -> Option<[u8; 3]> {
        match &self.geometry {
            Geometry::Rect {
                left,
                top,
                width,
                height,
                radius,
            } => {
                let (left, top, width, height, r) = (*left, *top, *width, *height, *radius);
                if !(left <= x && x <= left + width && top <= y && y <= top + height) {
                    return None;
                }
                let cx = x.clamp(left + r, left + width - r);
                let cy = y.clamp(top + r, top + height - r);
                if (x - cx).powi(2) + (y - cy).powi(2) <= r * r {
                    self.fill
                } else {
                    None
                }
            }
            Geometry::Circle {
                cx,
                cy,
                radius,
                half_stroke,
            } => {
                let d = (x - cx).hypot(y - cy);
                if self.stroke.is_some() && (d - radius).abs() <= *half_stroke {
                    self.stroke
                } else if d <= *radius {
                    self.fill
                } else {
                    None
                }
            }
            Geometry::Line {
                x1,
                y1,
                dx,
                dy,
                length_squared,
                half_stroke,
            } => {
                let f = (((x - x1) * dx + (y - y1) * dy) / length_squared).clamp(0., 1.);
                if (x - x1 - f * dx).hypot(y - y1 - f * dy) <= *half_stroke {
                    self.stroke
                } else {
                    None
                }
            }
            Geometry::Polygon(points) => {
                let mut inside = false;
                let mut previous = *points.last().unwrap();
                for &(x2, y2) in points {
                    let (x1, y1) = previous;
                    if (y1 > y) != (y2 > y) && x < (x2 - x1) * (y - y1) / (y2 - y1) + x1 {
                        inside = !inside;
                    }
                    previous = (x2, y2);
                }
                if inside {
                    self.fill
                } else {
                    None
                }
            }
        }
    }
}
pub fn rasterize(source: &str, size: usize) -> Result<Vec<u8>> {
    let shapes = parse(source)?;
    Ok(rasterize_shapes(&shapes, size))
}
fn rasterize_shapes(shapes: &[Shape], size: usize) -> Vec<u8> {
    let mut pixels = vec![];
    let scale = 128. / size as f64;
    for y in 0..size {
        for x in 0..size {
            let mut totals = [0u32; 3];
            let mut covered = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = (x as f64 + (sx as f64 + 0.5) / 4.) * scale;
                    let py = (y as f64 + (sy as f64 + 0.5) / 4.) * scale;
                    // The last painted shape wins; transparent shapes do not cover it.
                    let pixel = shapes.iter().rev().find_map(|s| s.paint(px, py));
                    if let Some(pixel) = pixel {
                        covered += 1;
                        for i in 0..3 {
                            totals[i] += u32::from(pixel[i]);
                        }
                    }
                }
            }
            if covered > 0 {
                for total in totals {
                    pixels.push((total as f64 / covered as f64).round_ties_even() as u8);
                }
                pixels.push((255. * covered as f64 / 16.).round_ties_even() as u8);
            } else {
                pixels.extend([0; 4]);
            }
        }
    }
    pixels
}
fn bitmap(size: usize, rgba: &[u8]) -> Vec<u8> {
    let mut pixels = vec![];
    let mut mask = vec![];
    let stride = size.div_ceil(32) * 4;
    for y in (0..size).rev() {
        let mut row = vec![0u8; stride];
        for x in 0..size {
            let o = (y * size + x) * 4;
            pixels.extend([rgba[o + 2], rgba[o + 1], rgba[o], rgba[o + 3]]);
            if rgba[o + 3] == 0 {
                row[x / 8] |= 1 << (7 - x % 8);
            }
        }
        mask.extend(row);
    }
    let mut out = vec![];
    for v in [40, size as u32, (size * 2) as u32] {
        out.extend(v.to_le_bytes());
    }
    out.extend(1u16.to_le_bytes());
    out.extend(32u16.to_le_bytes());
    for v in [0, pixels.len() as u32, 0, 0, 0, 0] {
        out.extend(v.to_le_bytes());
    }
    out.extend(pixels);
    out.extend(mask);
    out
}
fn png(size: usize, rgba: &[u8]) -> Result<Vec<u8>> {
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], bytes: &[u8]) {
        out.extend((bytes.len() as u32).to_be_bytes());
        out.extend(kind);
        out.extend(bytes);
        let mut crc = crc32fast::Hasher::new();
        crc.update(kind);
        crc.update(bytes);
        out.extend(crc.finalize().to_be_bytes());
    }
    let mut rows = vec![];
    for row in rgba.chunks_exact(size * 4) {
        rows.push(0);
        rows.extend(row);
    }
    let mut compressor = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::default());
    compressor.write_all(&rows)?;
    let compressed = compressor.finish()?;
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = vec![];
    header.extend((size as u32).to_be_bytes());
    header.extend((size as u32).to_be_bytes());
    header.extend([8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &compressed);
    chunk(&mut out, b"IEND", &[]);
    Ok(out)
}
pub fn generate(directory: &Path) -> Result<()> {
    let source = fs::read_to_string(directory.join("icon.svg"))?;
    let shapes = parse(&source)?;
    let sizes = [16, 20, 24, 32, 48, 64, 128, 256];
    let mut header = vec![];
    for v in [0u16, 1, 8] {
        header.extend(v.to_le_bytes());
    }
    let mut images = vec![];
    let mut offset = 6 + 16 * sizes.len();
    let mut large = None;
    for size in sizes {
        let rgba = rasterize_shapes(&shapes, size);
        let data = bitmap(size, &rgba);
        let dimension = if size == 256 { 0 } else { size as u8 };
        header.extend([dimension, dimension, 0, 0]);
        header.extend(1u16.to_le_bytes());
        header.extend(32u16.to_le_bytes());
        header.extend((data.len() as u32).to_le_bytes());
        header.extend((offset as u32).to_le_bytes());
        offset += data.len();
        images.extend(data);
        if size == 256 {
            large = Some(png(size, &rgba)?);
        }
    }
    header.extend(images);
    write(&directory.join("icon.ico"), header)?;
    write(&directory.join("icon.png"), large.unwrap())
}
