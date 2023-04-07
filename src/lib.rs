mod bisect;
mod edge;
mod math;
mod raster;

use std::{collections::HashSet, io::Write};

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
    pub left_clamp_opt: bool,
}

pub fn build<'a, T, I>(settings: Settings, glyphs: I) -> Vec<u8>
where
    T: Clone,
    I: 'a + Clone + Iterator<Item = (T, &'a Face<'a>, char)>,
{
    let (width, height, packing);
    match settings.size {
        AssetSize::FontSize(font_size) => {
            let (dim, packresult) = find_font_packing_by_font_size(
                font_size,
                settings.padding_ratio,
                settings.left_clamp_opt,
                &glyphs,
            );
            width = dim;
            height = dim;
            packing = packresult;
        }
        AssetSize::TextureSize(w, h) => {
            width = w;
            height = h;
            packing = find_font_packing_by_asset_size(
                width,
                height,
                settings.padding_ratio,
                settings.left_clamp_opt,
                &glyphs,
            );
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

fn find_font_packing_by_asset_size<'a, T, I>(
    asset_width: u16,
    asset_height: u16,
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
            raster::can_clamp_left(rastered_size, padding_ratio, face, ch)
                .then_some((face as *const Face<'_>, ch))
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

fn find_font_packing_by_font_size<'a, T, I>(
    font_size: f32,
    padding_ratio: f32,
    left_clamp_opt: bool,
    glyphs: &I,
) -> (u16, PackResult<'a, T>)
where
    T: Clone,
    I: 'a + Clone + Iterator<Item = (T, &'a Face<'a>, char)>,
{
    let clampable: HashSet<(*const Face<'a>, char)> = if left_clamp_opt {
        glyphs
            .clone()
            .filter_map(|(_id, face, ch)| {
                let rasterized_size =
                    raster::get_rastered_size(padding_ratio, false, font_size, face, ch);
                raster::can_clamp_left(rasterized_size, padding_ratio, face, ch)
                    .then_some((face as *const Face<'_>, ch))
            })
            .collect()
    } else {
        HashSet::new()
    };
    bisect::bisect_asset_size(font_size, padding_ratio, glyphs, |face, ch| {
        clampable.contains(&(face as *const Face<'_>, ch))
    })
}
