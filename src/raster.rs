/* SPDX-License-Identifier: (Apache-2.0 OR MIT OR Zlib) */
/* Copyright © 2023 Violet Leonard */

use std::fmt::Debug;

use crate::{
    edge::{EdgeBoundingBox, Segment},
    ShapeRequest,
};

pub type Segments = Vec<(crate::edge::Segment, EdgeBoundingBox)>;

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

pub(crate) trait Shape<ID>: Debug {
    fn to_rastered_size(
        &self,
        id: ID,
        padding_ratio: f32,
        scale: f32,
    ) -> Result<RasteredSize, crate::Error>;
    fn to_segments(&self, id: ID) -> Result<Segments, crate::Error>;
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
    item: &crunch::PackedItem<Box<(ShapeRequest<'_, char, T>, RasteredSize)>>,
) -> Result<(), crate::Error> {
    let (request, rastered_size) = &*item.data;
    let rotate = (item.rect.w - 1) != rastered_size.pixel_width.into();
    let segments = request.shape.to_segments(request.id)?;
    // glyphs must be separated by a pixel of zero, so the area we mutate is
    // reduced by 1 in each dimension
    let positive_width = item.rect.w - 1;
    let positive_height = item.rect.h - 1;
    for dest_y in 0..positive_height {
        // this math is subtle, the edge of the glyph is in the middle of the
        // zero (spacing) texels (the glyph rect expands outside the rect
        // defined by positive_width/height by half a texel in all cardinal
        // directions), so the total size is increased by 1, and the middle of
        // the texels we are mutating are at integer offsets. But the first
        // texel we are mutating isn't position zero, that's the spacing texel;
        // the first one we are mutating is actually at offset 1.
        let y = (dest_y as f32 + 1.0) / (positive_height as f32 + 1.0);
        let dest_y = dest_y + item.rect.y;
        for dest_x in 0..positive_width {
            let x = (dest_x as f32 + 1.0) / (positive_width as f32 + 1.0);
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
            for (i, (segment, seg_bbox)) in segments.iter().enumerate() {
                match segment {
                    Segment::LoopPoint(_, _) => continue,
                    Segment::Line(_) => {
                        // we can do nearest_t for lines
                        let t = segment.nearest_t((x, y));
                        let (px, py) = segment.point(t);
                        let dist2 = (px - x).powi(2) + (py - y).powi(2);
                        if dist2 < nearest_dist2 {
                            nearest_dist2 = dist2;
                            nearest = Some((i, t, px, py));
                        }
                    }
                    _ => {
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
            }
            // second pass, skip anything farther than what the first pass found
            for (i, (segment, seg_bbox)) in segments.iter().enumerate() {
                if matches!(segment, Segment::LoopPoint(_, _)) {
                    continue;
                }
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
                let (dx, dy) = segments[i].0.direction(t);
                let (dx, dy) = if t == 0.0 {
                    let other_seg = if i == 0 { segments.len() - 1 } else { i - 1 };
                    let (odx, ody) = segments[other_seg].0.direction(1.0);
                    let dlen = (dx.powi(2) + dy.powi(2)).sqrt();
                    let odlen = (odx.powi(2) + ody.powi(2)).sqrt();
                    ((dx / dlen + odx / odlen), (dy / dlen + ody / odlen))
                } else if t == 1.0 {
                    let other_seg = (i + 1) % segments.len();
                    let (odx, ody) = segments[other_seg].0.direction(0.0);
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
    Ok(())
}
