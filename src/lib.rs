pub extern crate ttf_parser;

mod bisect;
mod edge;
mod math;
mod raster;

use ttf_parser::Face;

use crate::{bisect::BisectArgs, raster::RasteredSize};

#[derive(Clone, Copy, Debug)]
pub struct FontAssetBuilder {
    size: AssetSize,
    padding: f32,
    allow_rotate: bool,
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SdfFontAsset<T> {
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
    pub metadata: Vec<Glyph<T>>,
}

#[derive(Clone, Copy, Debug)]
pub enum Error {
    MissingGlyph(char),
    PackingAtlasFailed,
}

impl FontAssetBuilder {
    pub fn with_texture_size(width: u16, height: u16) -> Self {
        assert!(width >= 2 && height >= 2);
        Self {
            size: AssetSize::TextureSize(width, height),
            padding: 0.1,
            allow_rotate: false,
        }
    }

    pub fn with_font_size(font_size: f32) -> Self {
        assert!(font_size > 0.0);
        Self {
            size: AssetSize::FontSize(font_size),
            padding: 0.1,
            allow_rotate: false,
        }
    }

    pub fn with_padding_ratio(self, padding: f32) -> Self {
        Self { padding, ..self }
    }

    pub fn allow_rotating_glyphs(self) -> Self {
        Self {
            allow_rotate: true,
            ..self
        }
    }

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

#[derive(Clone, Copy, Debug)]
pub struct GlyphRequest<'a, T> {
    pub id: T,
    pub face: &'a Face<'a>,
    pub codepoint: char,
}

#[derive(Clone, Copy, Debug)]
enum AssetSize {
    FontSize(f32),
    TextureSize(u16, u16),
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Glyph<T> {
    pub id: T,
    pub codepoint: char,
    pub rotated: bool,
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub tex_left: f32,
    pub tex_right: f32,
    pub tex_top: f32,
    pub tex_bottom: f32,
}

pub fn hexdigits() -> impl Clone + Iterator<Item = char> {
    b"0123456789abcdefABCDEFxX".iter().copied().map(char::from)
}

pub fn ascii() -> impl Clone + Iterator<Item = char> {
    (b'!'..=b'~').map(char::from)
}

pub fn latin1() -> impl Clone + Iterator<Item = char> {
    ascii().chain((0xa1..=0xff).map(char::from))
}

pub fn latin1_french() -> impl Clone + Iterator<Item = char> {
    latin1().chain(['\u{0152}', '\u{0153}', '\u{0178}'])
}

type PackResult<'a, T> = Vec<crunch::PackedItem<Box<(GlyphRequest<'a, T>, RasteredSize)>>>;
