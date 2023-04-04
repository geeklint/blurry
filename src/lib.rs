mod bisect;
mod edge;
mod math;
mod raster;

use std::collections::HashSet;

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
    TextureSize(u32, u32),
}

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub size: AssetSize,
    pub padding_ratio: f32,
    pub left_clamp_opt: bool,
}

pub fn build<'a, T, I>(settings: Settings, glyphs: I)
where
    I: 'a + Clone + Iterator<Item = (T, &'a Face<'a>, char)>,
{
}

type PackResult<'a, T> = Vec<crunch::PackedItem<Box<(T, &'a Face<'a>, char, RasteredSize)>>>;

fn find_font_packing<'a, T, I>(
    asset_width: u32,
    asset_height: u32,
    padding_ratio: f32,
    left_clamp_opt: bool,
    glyphs: &I,
) -> PackResult<'a, T>
where
    T: Clone,
    I: 'a + Clone + Iterator<Item = (T, &'a Face<'a>, char)>,
{
    let (font_size, pack) = bisect::bisect_font_size(
        asset_width,
        asset_height,
        padding_ratio,
        BisectArgs {
            lower_bound: 1.0,
            too_big: 8.0 * (asset_height as f32),
            attempts: 11,
        },
        glyphs,
        |_, _| false,
    );
    if !left_clamp_opt {
        return pack;
    }
    let clampable: HashSet<(*const Face<'a>, char)> = pack
        .into_iter()
        .filter_map(|packed_item| {
            let (_id, face, ch, rastered_size) = *packed_item.data;
            if raster::can_clamp_left(rastered_size, face, ch) {
                Some((face as *const Face<'_>, ch))
            } else {
                None
            }
        })
        .collect();
    bisect::bisect_font_size(
        asset_width,
        asset_height,
        padding_ratio,
        BisectArgs {
            lower_bound: font_size,
            too_big: 2.0 * font_size,
            attempts: 7,
        },
        glyphs,
        |face, ch| clampable.contains(&(face as *const Face<'_>, ch)),
    )
    .1
}
