mod bisect;
mod edge;
mod math;
mod raster;

use std::io::Write;

use ttf_parser::Face;

pub use crate::{
    bisect::BisectArgs,
    edge::{CubicCurve, Edge},
    raster::RasteredSize,
};

// settings:
// - font size / texture size
// - padding ratio
// - enable left-clamp
// take in iter of (id, Face, char)
//
// output:
// - id
// - xys left, right, top, bottom
// - uvs left, right, top, bottom
// - left clamp

#[derive(Clone, Copy, Debug)]
pub enum AssetSize {
    FontSize(f32),
    TextureSize(u16, u16),
}

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub size: AssetSize,
    pub padding_ratio: f32,
}

pub fn build<'a, T, I>(settings: Settings, glyphs: I) -> Vec<u8>
where
    T: Clone,
    I: 'a + Clone + Iterator<Item = (T, &'a Face<'a>, char)>,
{
    let (width, height, packing);
    match settings.size {
        AssetSize::FontSize(font_size) => {
            let (dim, packresult) =
                bisect::bisect_asset_size(font_size, settings.padding_ratio, &glyphs);
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
                settings.padding_ratio,
                BisectArgs {
                    lower_bound: 1.0,
                    too_big: 8.0 * (height as f32),
                    attempts: 11,
                },
                &glyphs,
            )
            .1;
        }
    }
    let buflen = usize::from(width) * usize::from(height);
    let mut buf = vec![0; buflen];
    for item in packing {
        eprint!("{}", item.data.2);
        raster::raster(
            raster::Buffer {
                data: &mut buf,
                width,
            },
            settings.padding_ratio,
            item,
        );
        let _ = std::io::stderr().flush();
    }
    eprintln!();
    buf
}

type PackResult<'a, T> = Vec<crunch::PackedItem<Box<(T, &'a Face<'a>, char, RasteredSize)>>>;
