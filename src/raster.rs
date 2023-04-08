use ttf_parser::Face;

use crate::edge::{CubicCurve, Line, QuadCurve, Segment};

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
}

pub fn get_rastered_size(
    padding_ratio: f32,
    font_size: f32,
    face: &Face<'_>,
    ch: char,
) -> RasteredSize {
    let face_height = f32::from(face.height());
    let padding = padding_ratio;
    let rel_from = |font_value: i16| f32::from(font_value) / face_height;
    let Some(glyph_id) = face.glyph_index(ch) else {
        panic!("glyph '{ch:?}' not found in face");
    };
    let bbox = face.glyph_bounding_box(glyph_id).unwrap();
    let width = rel_from(bbox.width()) + (2.0 * padding);
    let height = rel_from(bbox.height()) + (2.0 * padding);
    let pixel_width = (width * font_size).round().clamp(0.0, u16::MAX.into()) as u16;
    let pixel_height = (height * font_size).round().clamp(0.0, u16::MAX.into()) as u16;
    let left = rel_from(bbox.x_min) - padding;
    let right = rel_from(bbox.x_max) + padding;
    let top = rel_from(bbox.y_max) + padding;
    let bottom = rel_from(bbox.y_min) - padding;
    RasteredSize {
        pixel_width,
        pixel_height,
        left,
        right,
        top,
        bottom,
    }
}

pub struct Segments {
    face_height: f32,
    segments: Vec<(crate::edge::Segment, crate::edge::EdgeBoundingBox)>,
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
        let segment: Segment = Line::new((self.cursor_x, self.cursor_y), (x, y)).into();
        let bbox = segment.bbox();
        self.segments.push((segment, bbox));
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let x1 = x1 / self.face_height;
        let y1 = y1 / self.face_height;
        let x = x / self.face_height;
        let y = y / self.face_height;
        let segment: Segment =
            QuadCurve::new((self.cursor_x, self.cursor_y), (x1, y1), (x, y)).into();
        let bbox = segment.bbox();
        self.segments.push((segment, bbox));
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
        let segment: Segment =
            CubicCurve::new((self.cursor_x, self.cursor_y), (x1, y1), (x2, y2), (x, y)).into();
        let bbox = segment.bbox();
        self.segments.push((segment, bbox));
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
            let x = rastered_size.left + (x * (rastered_size.right - rastered_size.left));
            let y = rastered_size.bottom + (y * (rastered_size.top - rastered_size.bottom));
            let outside = (x - rastered_size.left) < padding
                || (rastered_size.right - x) < padding
                || (y - rastered_size.bottom) < padding
                || (rastered_size.top - y) < padding;
            let mut nearest = None;
            let mut nearest_dist2 = if outside {
                padding * padding
            } else {
                f32::INFINITY
            };
            // first pass, skip anything that requires newton's method
            for (i, (segment, seg_bbox)) in segments.segments.iter().enumerate() {
                if matches!(segment, Segment::Line(_)) {
                    // we can do nearest_t for lines
                    let t = segment.nearest_t((x, y));
                    let (px, py) = segment.point(t);
                    let dist2 = (px - x).powi(2) + (py - y).powi(2);
                    if dist2 < nearest_dist2 {
                        nearest_dist2 = dist2;
                        nearest = Some((i, t, px, py));
                    }
                } else {
                    let bbox_near_x = x.clamp(seg_bbox.left, seg_bbox.right);
                    let bbox_near_y = y.clamp(seg_bbox.bottom, seg_bbox.top);
                    let bbox_dist2 = (bbox_near_x - x).powi(2) + (bbox_near_y - y).powi(2);
                    if bbox_dist2 > nearest_dist2 {
                        continue;
                    }
                    // just check the end points for curves
                    let (px, py) = segment.point(0.0);
                    let dist2 = (px - x).powi(2) + (py - y).powi(2);
                    if dist2 < nearest_dist2 {
                        nearest_dist2 = dist2;
                        nearest = Some((i, 0.0, px, py));
                    }
                    let (px, py) = segment.point(1.0);
                    let dist2 = (px - x).powi(2) + (py - y).powi(2);
                    if dist2 < nearest_dist2 {
                        nearest_dist2 = dist2;
                        nearest = Some((i, 1.0, px, py));
                    }
                }
            }
            // second pass, skip anything farther than what the first pass found
            for (i, (segment, seg_bbox)) in segments.segments.iter().enumerate() {
                let bbox_near_x = x.clamp(seg_bbox.left, seg_bbox.right);
                let bbox_near_y = y.clamp(seg_bbox.bottom, seg_bbox.top);
                let bbox_dist2 = (bbox_near_x - x).powi(2) + (bbox_near_y - y).powi(2);
                if bbox_dist2 > nearest_dist2 {
                    continue;
                }
                let t = segment.nearest_t((x, y));
                let (px, py) = segment.point(t);
                let dist2 = (px - x).powi(2) + (py - y).powi(2);
                if dist2 < nearest_dist2 {
                    nearest_dist2 = dist2;
                    nearest = Some((i, t, px, py));
                }
            }
            if let Some((i, t, cx, cy)) = nearest {
                let (dx, dy) = segments.segments[i].0.direction(t);
                let (dx, dy) = if t == 0.0 {
                    let other_seg = if i == 0 {
                        segments.segments.len() - 1
                    } else {
                        i - 1
                    };
                    let (odx, ody) = segments.segments[other_seg].0.direction(1.0);
                    let dlen = (dx.powi(2) + dy.powi(2)).sqrt();
                    let odlen = (odx.powi(2) + ody.powi(2)).sqrt();
                    ((dx / dlen + odx / odlen), (dy / dlen + ody / odlen))
                } else if t == 1.0 {
                    let other_seg = (i + 1) % segments.segments.len();
                    let (odx, ody) = segments.segments[other_seg].0.direction(0.0);
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
