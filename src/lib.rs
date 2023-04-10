/* SPDX-License-Identifier: (Apache-2.0 OR MIT) */
/* Copyright © 2023 Violet Leonard */

//! You can use this crate to generate an image atlas containing a signed
//! distance field of glyphs from a font.

#![warn(missing_docs)]

pub extern crate ttf_parser;

mod bisect;
mod edge;
mod math;
mod raster;

use ttf_parser::Face;

use crate::{bisect::BisectArgs, raster::RasteredSize};

/// Knobs and dials for asset generation
#[derive(Clone, Copy, Debug)]
pub struct FontAssetBuilder {
    size: AssetSize,
    padding: f32,
    allow_rotate: bool,
}

/// The result of asset generation
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SdfFontAsset<T> {
    /// The width of the resulting image in pixels
    pub width: u16,

    /// The height of the resulting image in pixels
    pub height: u16,

    /// The raw image data
    pub data: Vec<u8>,

    /// A list of metadata for the rendered glyphs
    pub metadata: Vec<Glyph<T>>,
}

/// Possible errors that can happen while generating the image
#[derive(Clone, Copy, Debug)]
pub enum Error {
    /// This error occurs if the library could not get
    /// all the information it needs to render a glyph
    /// from the font file.
    MissingGlyph(char),

    /// This error occurs if too large a font size
    /// is specified to neatly pack the requested glyphs
    /// in a single texture
    PackingAtlasFailed,
}

impl FontAssetBuilder {
    /// Define the size of the resulting asset by specifying the image
    /// dimensions.  The size of glyphs will be adjusted to fit inside.
    pub fn with_texture_size(width: u16, height: u16) -> Self {
        assert!(width >= 2 && height >= 2);
        Self {
            size: AssetSize::TextureSize(width, height),
            padding: 0.1,
            allow_rotate: false,
        }
    }

    /// Define the size of the resulting asset by specifying the desired final
    /// font size.  The dimensions of the image will be chosen to fit all glyphs
    /// at the provided size.
    pub fn with_font_size(font_size: f32) -> Self {
        assert!(font_size > 0.0);
        Self {
            size: AssetSize::FontSize(font_size),
            padding: 0.1,
            allow_rotate: false,
        }
    }

    /// Define the ratio of the distance field to the size of the glyph.  For
    /// example, a 16px glyph with a padding ratio of 0.25 render such that the
    /// signed distance field measures -4 to +4 pixels.
    pub fn with_padding_ratio(self, padding: f32) -> Self {
        Self { padding, ..self }
    }

    /// Use this to allow rotating glyphs, which may make the atlas packing more
    /// optimal but requires more attention when decoding the resulting texture
    /// coordinates.
    pub fn allow_rotating_glyphs(self) -> Self {
        Self {
            allow_rotate: true,
            ..self
        }
    }

    /// Build a SDF font asset given a set of glyphs to include.
    pub fn build<'a, T, I>(self, glyphs: I) -> Result<SdfFontAsset<T>, Error>
    where
        T: Clone,
        I: 'a + Clone + Iterator<Item = GlyphRequest<'a, T>>,
    {
        let (width, height, packing);
        match self.size {
            AssetSize::FontSize(font_size) => {
                let (dim, packresult) =
                    bisect::bisect_asset_size(font_size, self.padding, self.allow_rotate, &glyphs)?;
                width = dim;
                height = dim;
                packing = packresult;
            }
            AssetSize::TextureSize(w, h) => {
                width = w;
                height = h;
                packing = bisect::bisect_font_size(
                    width,
                    height,
                    self.padding,
                    self.allow_rotate,
                    BisectArgs {
                        lower_bound: 1.0,
                        too_big: 8.0 * (height as f32),
                        attempts: 11,
                    },
                    &glyphs,
                )?
                .1;
            }
        }
        let buflen = usize::from(width) * usize::from(height);
        let mut buf = vec![0; buflen];
        let mut meta = Vec::with_capacity(packing.len());
        for item in packing {
            raster::raster(
                raster::Buffer {
                    data: &mut buf,
                    width,
                },
                self.padding,
                &item,
            )?;
            // calculate metadata
            let (request, rastered_size) = *item.data;
            let rotated = (item.rect.w - 1) != rastered_size.pixel_width.into();
            let RasteredSize {
                left,
                right,
                top,
                bottom,
                ..
            } = rastered_size;
            let tex_left = (item.rect.x as f32) / f32::from(width);
            let tex_right =
                (item.rect.x as f32 + f32::from(rastered_size.pixel_width)) / f32::from(width);
            let tex_bottom = (item.rect.y as f32) / f32::from(height);
            let tex_top =
                (item.rect.y as f32 + f32::from(rastered_size.pixel_height)) / f32::from(height);
            meta.push(Glyph {
                id: request.id,
                codepoint: request.codepoint,
                rotated,
                left,
                right,
                top,
                bottom,
                tex_left,
                tex_right,
                tex_bottom,
                tex_top,
            });
        }
        Ok(SdfFontAsset {
            width,
            height,
            data: buf,
            metadata: meta,
        })
    }
}

/// A request for a glyph to be rendered.
#[derive(Clone, Copy, Debug)]
pub struct GlyphRequest<'a, T> {
    /// An id you can use to relate GlyphRequests to rendered Glyphs.
    pub id: T,

    /// The font face to render the glyph from.
    pub face: &'a Face<'a>,

    /// The codepoint of the glyph.
    pub codepoint: char,
}

#[derive(Clone, Copy, Debug)]
enum AssetSize {
    FontSize(f32),
    TextureSize(u16, u16),
}

/// Metadata for a glyph that was rendered in an asset.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Glyph<T> {
    /// The id from the GlyphRequest.
    pub id: T,

    /// The codepoint that was rendered.
    pub codepoint: char,

    /// Whether rotation was applied when this glyph was packed.
    pub rotated: bool,

    /// The relative left edge of a bounding box from the glyph's 0 position
    /// that will position the resulting SDF so that the middle distance
    /// describes a character as specified by the font.
    pub left: f32,

    /// The relative right edge of a bounding box from the glyph's 0 position
    /// that will position the resulting SDF so that the middle distance
    /// describes a character as specified by the font.
    pub right: f32,

    /// The relative bottom edge of a bounding box from the glyph's 0 position
    /// that will position the resulting SDF so that the middle distance
    /// describes a character as specified by the font.
    pub bottom: f32,

    /// The relative top edge of a bounding box from the glyph's 0 position
    /// that will position the resulting SDF so that the middle distance
    /// describes a character as specified by the font.
    pub top: f32,

    /// The left edge of the rendered glyph as a texture coordinate
    pub tex_left: f32,

    /// The right edge of the rendered glyph as a texture coordinate
    pub tex_right: f32,

    /// The top edge of the rendered glyph as a texture coordinate
    pub tex_top: f32,

    /// The bottom edge of the rendered glyph as a texture coordinate
    pub tex_bottom: f32,
}

/// Returns an iterator of the chars you would want to pass to
/// [`build`](FontAssetBuilder::build) if you will be using the rendered font to
/// display hexadecimal values.
pub fn hexdigits() -> impl Clone + Iterator<Item = char> {
    b"0123456789abcdefABCDEFxX".iter().copied().map(char::from)
}

/// Returns an iterator of the chars you would want to pass to
/// [`build`](FontAssetBuilder::build) if you will be using the rendered font to
/// display ascii text.
pub fn ascii() -> impl Clone + Iterator<Item = char> {
    (b'!'..=b'~').map(char::from)
}

/// Returns an iterator of the chars you would want to pass to
/// [`build`](FontAssetBuilder::build) if you will be using the rendered font to
/// display ISO-8859-1 ("Latin 1") text.
pub fn latin1() -> impl Clone + Iterator<Item = char> {
    ascii().chain((0xa1..=0xff).map(char::from))
}

/// Returns an iterator of the chars you would want to pass to
/// [`build`](FontAssetBuilder::build) if you will be using the rendered font to
/// display ISO-8859-1 ("Latin 1") text with French support.
pub fn latin1_french() -> impl Clone + Iterator<Item = char> {
    latin1().chain(['\u{0152}', '\u{0153}', '\u{0178}'])
}

type PackResult<'a, T> = Vec<crunch::PackedItem<Box<(GlyphRequest<'a, T>, RasteredSize)>>>;
