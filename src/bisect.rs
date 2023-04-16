/* SPDX-License-Identifier: (Apache-2.0 OR MIT OR Zlib) */
/* Copyright © 2023 Violet Leonard */

use crate::{GlyphRequest, PackResult};

pub struct BisectArgs<T> {
    pub lower_bound: T,
    pub too_big: T,
    pub attempts: u32,
}

pub fn bisect_font_size<'a, T, I>(
    asset_width: u16,
    asset_height: u16,
    padding_ratio: f32,
    allow_rotate: bool,
    args: BisectArgs<f32>,
    glyphs: &I,
) -> Result<(f32, PackResult<'a, T>), crate::Error>
where
    T: Clone,
    I: 'a + Clone + Iterator<Item = GlyphRequest<'a, T>>,
{
    let rot = if allow_rotate {
        crunch::Rotation::Allowed
    } else {
        crunch::Rotation::None
    };
    let mut attempts_remaining = args.attempts;
    let BisectArgs {
        mut lower_bound,
        mut too_big,
        ..
    } = args;
    loop {
        attempts_remaining = attempts_remaining.saturating_sub(1);

        let check_size = (lower_bound + too_big) / 2.0;
        let mut missing_glyph = Ok(());
        let rects = glyphs.clone().map_while(|req| {
            let rastered_size = match crate::raster::get_rastered_size(
                padding_ratio,
                check_size,
                req.face,
                req.codepoint,
            ) {
                Ok(sz) => sz,
                Err(ch) => {
                    missing_glyph = Err(crate::Error::MissingGlyph(ch));
                    return None;
                }
            };
            Some(crunch::Item {
                data: Box::new((req, rastered_size)),
                w: (rastered_size.pixel_width + 1).into(),
                h: (rastered_size.pixel_height + 1).into(),
                rot,
            })
        });
        let pack_width = (asset_width - 1).into();
        let pack_height = (asset_height - 1).into();
        match crunch::Packer::with_items(rects).pack(crunch::Rect {
            x: 1,
            y: 1,
            w: pack_width,
            h: pack_height,
        }) {
            Ok(result) => {
                missing_glyph?;
                lower_bound = check_size;
                if attempts_remaining == 0 {
                    return Ok((lower_bound, result));
                }
            }
            Err(_) => {
                missing_glyph?;
                too_big = check_size;
            }
        }
    }
}

pub fn bisect_asset_size<'a, T, I>(
    font_size: f32,
    padding_ratio: f32,
    allow_rotate: bool,
    glyphs: &I,
) -> Result<(u16, PackResult<'a, T>), crate::Error>
where
    T: Clone,
    I: 'a + Clone + Iterator<Item = GlyphRequest<'a, T>>,
{
    let rot = if allow_rotate {
        crunch::Rotation::Allowed
    } else {
        crunch::Rotation::None
    };
    let mut too_small = (font_size.floor().clamp(2.0, u16::MAX.into()) as u16) - 1;
    let missing_glyph = std::cell::Cell::new(Ok(()));
    let mut map_glyphs = |req: GlyphRequest<'a, T>| {
        let rastered_size = match crate::raster::get_rastered_size(
            padding_ratio,
            font_size,
            req.face,
            req.codepoint,
        ) {
            Ok(sz) => sz,
            Err(ch) => {
                missing_glyph.set(Err(crate::Error::MissingGlyph(ch)));
                return None;
            }
        };
        Some(crunch::Item {
            data: Box::new((req, rastered_size)),
            w: (rastered_size.pixel_width + 1).into(),
            h: (rastered_size.pixel_height + 1).into(),
            rot,
        })
    };
    let mut result = match crunch::Packer::with_items(glyphs.clone().map_while(&mut map_glyphs))
        .pack(crunch::Rect {
            x: 1,
            y: 1,
            w: u16::MAX.into(),
            h: u16::MAX.into(),
        }) {
        Ok(res) => {
            missing_glyph.get()?;
            res
        }
        Err(_) => {
            missing_glyph.get()?;
            return Err(crate::Error::PackingAtlasFailed);
        }
    };
    let mut upper_bound = u16::MAX;
    while (too_small + 1) < upper_bound {
        let check_size = too_small + ((upper_bound - too_small) / 2);
        match crunch::Packer::with_items(glyphs.clone().map_while(&mut map_glyphs)).pack(
            crunch::Rect {
                x: 1,
                y: 1,
                w: check_size.into(),
                h: check_size.into(),
            },
        ) {
            Ok(res) => {
                missing_glyph.get()?;
                result = res;
                upper_bound = check_size;
            }
            Err(_) => {
                missing_glyph.get()?;
                too_small = check_size;
            }
        }
    }
    Ok((upper_bound, result))
}
