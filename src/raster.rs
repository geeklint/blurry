use ttf_parser::Face;

use crate::edge::{CubicCurve, Line, QuadCurve};

#[derive(Clone, Copy, Debug)]
pub struct RasteredSize {
    /// The width of the destination buffer
    pub pixel_width: u16,
    /// The height of the destination buffer
    pub pixel_height: u16,

    /// The left edge of the bounding box in percentage of font height
    pub left: f32,
    /// The right edge of the bounding box in percentage of font height
    pub right: f32,
    /// The top edge of the bounding box in percentage of font height
    pub top: f32,
    /// The bottom edge of the bounding box in percentage of font height
    pub bottom: f32,
    /// If this glyph uses the left-clamp trick, this is the clamped edge
    /// in percentage of font height
    pub left_clamped: f32,
    /// If this glyph uses the left-clamp trick, this is the clamped edge
    /// in percentage of font height with an additional pixel that is
    /// rendered outside of the clamped area for blending purposes
    pub left_clamped_internal_raster: f32,
}

pub fn get_rastered_size(
    padding_ratio: f32,
    clamp_left: bool,
    font_size: f32,
    face: &Face<'_>,
    ch: char,
) -> RasteredSize {
    let face_height = f32::from(face.height());
    let padding = padding_ratio;
    let left_padding = if clamp_left { 0.0 } else { padding };
    let rel_from = |font_value: i16| f32::from(font_value) / face_height;
    let Some(glyph_id) = face.glyph_index(ch) else {
        panic!("glyph '{ch:?}' not found in face");
    };
    let bbox = face.glyph_bounding_box(glyph_id).unwrap();
    // the width without a bonus pixel added for blending
    let edgeless_width = rel_from(bbox.width()) + padding + left_padding;
    let height = rel_from(bbox.height()) + (2.0 * padding);
    let pixel_width_f = (edgeless_width * font_size)
        .round()
        .clamp(0.0, u16::MAX.into());
    let pixel_height = (height * font_size).round().clamp(0.0, u16::MAX.into()) as u16;
    let left = rel_from(bbox.x_min) - padding;
    let right = rel_from(bbox.x_max) + padding;
    let top = rel_from(bbox.y_max) + padding;
    let bottom = rel_from(bbox.y_min) - padding;
    let (pixel_width, left_clamped, left_clamped_internal_raster);
    if clamp_left {
        // The width in font-units divided by the (rounded) number of pixels
        // gives the actual size of 1px in font units, which is slightly
        // distored vs `1.0 / font_size`; this is correct since we
        // can produce distorted glyphs which are undistorted by rendering.
        let one_pixel = edgeless_width / pixel_width_f;
        pixel_width = (pixel_width_f as u16) + 1;
        left_clamped = rel_from(bbox.x_min);
        left_clamped_internal_raster = left_clamped - one_pixel;
    } else {
        pixel_width = pixel_width_f as u16;
        left_clamped = left;
        left_clamped_internal_raster = left;
    }
    RasteredSize {
        pixel_width,
        pixel_height,
        left,
        right,
        top,
        bottom,
        left_clamped,
        left_clamped_internal_raster,
    }
}

pub fn can_clamp_left(unclamped: RasteredSize, padding: f32, face: &Face<'_>, ch: char) -> bool {
    let samples = unclamped.pixel_height;
    let height = unclamped.top - unclamped.bottom;
    let glyph_id = face.glyph_index(ch).unwrap();
    let mut segments = Segments::new(f32::from(face.height()));
    let Some(bbox) = face.outline_glyph(glyph_id, &mut segments) else {return false};
    let bottom = f32::from(bbox.y_min) / f32::from(face.height());
    let top = f32::from(bbox.y_max) / f32::from(face.height());
    let vertical_pixel = (unclamped.top - unclamped.bottom) / (unclamped.pixel_height as f32);
    let error_slope = vertical_pixel / padding;
    let x = unclamped.left;
    for sample in 0..samples {
        let y_percent = ((sample as f32) + 0.5) / (samples as f32);
        let y = unclamped.bottom + (y_percent * height);
        if y < bottom || y > top {
            continue;
        }
        let mut nearest_dist2 = f32::INFINITY;
        let mut nearest_point = (x, y);
        for segment in &segments.segments {
            let t = segment.nearest_t((unclamped.left, y));
            let (px, py) = segment.point(t);
            let dist2 = (px - x).powi(2) + (py - y).powi(2);
            if dist2 < nearest_dist2 {
                nearest_dist2 = dist2;
                nearest_point = (px, py);
            }
        }
        let slope = (nearest_point.1 - y).abs() / (nearest_point.0 - x).abs();
        if slope > error_slope {
            return false;
        }
    }
    true
}

pub struct Segments {
    face_height: f32,
    segments: Vec<crate::edge::Segment>,
    cursor_x: f32,
    cursor_y: f32,
}

impl Segments {
    fn new(face_height: f32) -> Self {
        Self {
            face_height,
            segments: Vec::new(),
            cursor_x: 0.0,
            cursor_y: 0.0,
        }
    }
}

impl ttf_parser::OutlineBuilder for Segments {
    fn move_to(&mut self, x: f32, y: f32) {
        self.cursor_x = x / self.face_height;
        self.cursor_y = y / self.face_height;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let x = x / self.face_height;
        let y = y / self.face_height;
        self.segments
            .push(Line::new((self.cursor_x, self.cursor_y), (x, y)).into());
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let x1 = x1 / self.face_height;
        let y1 = y1 / self.face_height;
        let x = x / self.face_height;
        let y = y / self.face_height;
        self.segments
            .push(QuadCurve::new((self.cursor_x, self.cursor_y), (x1, y1), (x, y)).into());
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let x1 = x1 / self.face_height;
        let y1 = y1 / self.face_height;
        let x2 = x2 / self.face_height;
        let y2 = y2 / self.face_height;
        let x = x / self.face_height;
        let y = y / self.face_height;
        self.segments.push(
            CubicCurve::new((self.cursor_x, self.cursor_y), (x1, y1), (x2, y2), (x, y)).into(),
        );
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn close(&mut self) {}
}

pub struct Buffer<'a> {
    pub data: &'a mut [u8],
    pub width: u16,
}

impl<'a> Buffer<'a> {
    fn set_pixel(&mut self, (x, y): (usize, usize), value: u8) {
        let width = usize::from(self.width);
        self.data[y * width + x] = value;
    }
}

pub fn raster<T>(
    mut buffer: Buffer<'_>,
    padding: f32,
    item: crunch::PackedItem<Box<(T, &Face<'_>, char, RasteredSize)>>,
) {
    let (_id, face, ch, rastered_size) = *item.data;
    let rotate = (item.rect.w - 1) != rastered_size.pixel_width.into();
    let glyph_id = face.glyph_index(ch).unwrap();
    let mut segments = Segments::new(f32::from(face.height()));
    face.outline_glyph(glyph_id, &mut segments);
    for dest_y in 0..(item.rect.h - 1) {
        let y = (dest_y as f32 + 0.5) / ((item.rect.h - 1) as f32);
        let dest_y = dest_y + item.rect.y;
        for dest_x in 0..(item.rect.w - 1) {
            let x = (dest_x as f32 + 0.5) / ((item.rect.w - 1) as f32);
            let dest_x = dest_x + item.rect.x;
            let (x, y) = if rotate { (y, x) } else { (x, y) };
            let x = rastered_size.left_clamped_internal_raster
                + (x * (rastered_size.right - rastered_size.left_clamped_internal_raster));
            let y = rastered_size.bottom + (y * (rastered_size.top - rastered_size.bottom));
            let mut nearest = None;
            let mut nearest_dist2 = f32::INFINITY;
            for (i, segment) in segments.segments.iter().enumerate() {
                let t = segment.nearest_t((x, y));
                let (px, py) = segment.point(t);
                let dist2 = (px - x).powi(2) + (py - y).powi(2);
                if dist2 < nearest_dist2 {
                    nearest_dist2 = dist2;
                    nearest = Some((i, t));
                }
            }
            if let Some((i, t)) = nearest {
                let (cx, cy) = segments.segments[i].point(t);
                let (dx, dy) = segments.segments[i].direction(t);
                let (dx, dy) = if t == 0.0 {
                    let other_seg = if i == 0 {
                        segments.segments.len() - 1
                    } else {
                        i - 1
                    };
                    let (odx, ody) = segments.segments[other_seg].direction(1.0);
                    let dlen = (dx.powi(2) + dy.powi(2)).sqrt();
                    let odlen = (odx.powi(2) + ody.powi(2)).sqrt();
                    ((dx / dlen + odx / odlen), (dy / dlen + ody / odlen))
                } else if t == 1.0 {
                    let other_seg = (i + 1) % segments.segments.len();
                    let (odx, ody) = segments.segments[other_seg].direction(0.0);
                    let dlen = (dx.powi(2) + dy.powi(2)).sqrt();
                    let odlen = (odx.powi(2) + ody.powi(2)).sqrt();
                    ((dx / dlen + odx / odlen), (dy / dlen + ody / odlen))
                } else {
                    (dx, dy)
                };
                let curve_side = (dx * (y - cy) - dy * (x - cx)).signum();
                //let inside = curve_side < 0.0;
                let dist = nearest_dist2.sqrt() / padding;
                let signed_dist = 0.5 - curve_side * (dist * 0.5);
                let value = (f32::from(u8::MAX) * signed_dist.clamp(0.0, 1.0)) as u8;
                buffer.set_pixel((dest_x, dest_y), value)
            }
        }
    }
}
