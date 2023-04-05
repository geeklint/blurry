use ttf_parser::Face;

use crate::edge::{CubicCurve, Line, QuadCurve};

/// the largest f32 which is less than u32::MAX
///
/// u32::MAX rounds up when converted to float, so do this manually
const MAX_F32_IN_U32: f32 = 4294967040.0;

#[derive(Clone, Copy, Debug)]
pub struct RasteredSize {
    /// The width of the destination buffer
    pub pixel_width: u32,
    /// The height of the destination buffer
    pub pixel_height: u32,

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
        .clamp(0.0, MAX_F32_IN_U32);
    let pixel_height = (height * font_size).round().clamp(0.0, MAX_F32_IN_U32) as u32;
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
        pixel_width = (pixel_width_f as u32) + 1;
        left_clamped = rel_from(bbox.x_min);
        left_clamped_internal_raster = left_clamped - one_pixel;
    } else {
        pixel_width = pixel_width_f as u32;
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
    let mut segments = Segments::default();
    face.outline_glyph(glyph_id, &mut segments);
    let vertical_pixel = (unclamped.top - unclamped.bottom) / (unclamped.pixel_height as f32);
    let error_slope = vertical_pixel / padding;
    let x = unclamped.left;
    for sample in 0..samples {
        let y_percent = ((sample as f32) + 0.5) / (samples as f32);
        let y = unclamped.bottom + (y_percent * height);
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

#[derive(Default)]
pub struct Segments {
    segments: Vec<crate::edge::Segment>,
    cursor_x: f32,
    cursor_y: f32,
}

impl ttf_parser::OutlineBuilder for Segments {
    fn move_to(&mut self, x: f32, y: f32) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.segments
            .push(Line::new((self.cursor_x, self.cursor_y), (x, y)).into());
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.segments
            .push(QuadCurve::new((self.cursor_x, self.cursor_y), (x1, y1), (x, y)).into());
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.segments.push(
            CubicCurve::new((self.cursor_x, self.cursor_y), (x1, y1), (x2, y2), (x, y)).into(),
        );
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn close(&mut self) {}
}
