/* SPDX-License-Identifier: (Apache-2.0 OR MIT OR Zlib) */
/* Copyright © 2026 Violet Leonard */

use ttf_parser::Face;

use crate::{
    edge::{CubicCurve, EdgeBoundingBox, Line, QuadCurve, Segment},
    raster::{RasteredSize, Segments, Shape},
};

impl Shape<char> for Face<'_> {
    fn to_rastered_size(
        &self,
        ch: char,
        padding_ratio: f32,
        font_size: f32,
    ) -> Result<RasteredSize, crate::Error> {
        let face_height = f32::from(self.units_per_em());
        let padding = padding_ratio;
        let rel_from = |font_value: i16| f32::from(font_value) / face_height;
        let glyph_id = self.glyph_index(ch).ok_or(crate::Error::MissingGlyph(ch))?;
        let bbox = self
            .glyph_bounding_box(glyph_id)
            .ok_or(crate::Error::MissingGlyph(ch))?;
        let width = rel_from(bbox.width()) + (2.0 * padding);
        let height = rel_from(bbox.height()) + (2.0 * padding);
        let pixel_width = (width * font_size).round().clamp(0.0, u16::MAX.into()) as u16;
        let pixel_height = (height * font_size).round().clamp(0.0, u16::MAX.into()) as u16;
        let left = rel_from(bbox.x_min) - padding;
        let right = rel_from(bbox.x_max) + padding;
        let top = rel_from(bbox.y_max) + padding;
        let bottom = rel_from(bbox.y_min) - padding;
        Ok(RasteredSize {
            pixel_width,
            pixel_height,
            left,
            right,
            top,
            bottom,
        })
    }

    fn to_segments(&self, codepoint: char) -> Result<Segments, crate::Error> {
        let glyph_id = self
            .glyph_index(codepoint)
            .ok_or(crate::Error::MissingGlyph(codepoint))?;
        let mut builder = SegmentsBuilder::new(f32::from(self.units_per_em()));
        self.outline_glyph(glyph_id, &mut builder);
        Ok(builder.segments)
    }
}

pub struct SegmentsBuilder {
    face_height: f32,
    segments: Vec<(crate::edge::Segment, EdgeBoundingBox)>,
    curve_start: usize,
    cursor_x: f32,
    cursor_y: f32,
}

impl SegmentsBuilder {
    fn new(face_height: f32) -> Self {
        Self {
            face_height,
            segments: Vec::new(),
            curve_start: usize::MAX,
            cursor_x: 0.0,
            cursor_y: 0.0,
        }
    }
}

impl ttf_parser::OutlineBuilder for SegmentsBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.cursor_x = x / self.face_height;
        self.cursor_y = y / self.face_height;
        let segment = Segment::LoopPoint(0.0, 0.0);
        let bbox = EdgeBoundingBox {
            left: x,
            right: x,
            bottom: y,
            top: y,
        };
        self.curve_start = self.segments.len();
        self.segments.push((segment, bbox));
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

    fn close(&mut self) {
        let (end_dx, end_dy) = self.segments.last().unwrap().0.direction(1.0);
        let (start_dx, start_dy) = self.segments[self.curve_start + 1].0.direction(0.0);
        self.segments[self.curve_start].0 = Segment::LoopPoint(end_dx, end_dy);
        let end_segment = Segment::LoopPoint(start_dx, start_dy);
        let end_bbox = EdgeBoundingBox {
            left: self.cursor_x,
            right: self.cursor_x,
            top: self.cursor_y,
            bottom: self.cursor_y,
        };
        self.segments.push((end_segment, end_bbox));
    }
}
