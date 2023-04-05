use ttf_parser::Face;

use crate::PackResult;

pub struct BisectArgs<T> {
    pub lower_bound: T,
    pub too_big: T,
    pub attempts: u32,
}

pub fn bisect_font_size<'a, T, I, F>(
    asset_width: u32,
    asset_height: u32,
    padding_ratio: f32,
    args: BisectArgs<f32>,
    glyphs: &I,
    mut clamp: F,
) -> (f32, PackResult<'a, T>)
where
    T: Clone,
    I: 'a + Clone + Iterator<Item = (T, &'a Face<'a>, char)>,
    F: FnMut(&Face<'_>, char) -> bool,
{
    let mut attempts_remaining = args.attempts;
    let BisectArgs {
        mut lower_bound,
        mut too_big,
        ..
    } = args;
    loop {
        attempts_remaining = attempts_remaining.saturating_sub(1);

        let check_size = (lower_bound + too_big) / 2.0;
        let rects = glyphs.clone().map(|(id, face, ch)| {
            let rastered_size = crate::raster::get_rastered_size(
                padding_ratio,
                clamp(face, ch),
                check_size,
                face,
                ch,
            );
            crunch::Item {
                data: Box::new((id, face, ch, rastered_size)),
                w: (rastered_size.pixel_width + 1).try_into().unwrap(),
                h: (rastered_size.pixel_height + 1).try_into().unwrap(),
                rot: crunch::Rotation::Allowed,
            }
        });
        let pack_width = (asset_width - 1).try_into().unwrap();
        let pack_height = (asset_height - 1).try_into().unwrap();
        match crunch::Packer::with_items(rects).pack(crunch::Rect {
            x: 1,
            y: 1,
            w: pack_width,
            h: pack_height,
        }) {
            Ok(result) => {
                lower_bound = check_size;
                if attempts_remaining == 0 {
                    return (lower_bound, result);
                }
            }
            Err(_) => {
                too_big = check_size;
            }
        }
    }
}

pub fn bisect_asset_size<'a, T, I, F>(
    font_size: f32,
    padding_ratio: f32,
    glyphs: &I,
    mut clamp: F,
) -> (u16, PackResult<'a, T>)
where
    T: Clone,
    I: 'a + Clone + Iterator<Item = (T, &'a Face<'a>, char)>,
    F: FnMut(&Face<'_>, char) -> bool,
{
    let mut too_small = (font_size.floor().clamp(2.0, u16::MAX.into()) as u16) - 1;
    let mut map_glyphs = |(id, face, ch)| {
        let rastered_size =
            crate::raster::get_rastered_size(padding_ratio, clamp(face, ch), font_size, face, ch);
        crunch::Item {
            data: Box::new((id, face, ch, rastered_size)),
            w: (rastered_size.pixel_width + 1).try_into().unwrap(),
            h: (rastered_size.pixel_height + 1).try_into().unwrap(),
            rot: crunch::Rotation::Allowed,
        }
    };
    let mut result = match crunch::Packer::with_items(glyphs.clone().map(&mut map_glyphs)).pack(
        crunch::Rect {
            x: 1,
            y: 1,
            w: u16::MAX.into(),
            h: u16::MAX.into(),
        },
    ) {
        Ok(res) => res,
        Err(_) => {
            panic!("failed to pack glyphs at the given font size into a 2^16 by 2^16 square, the maximum we support");
        }
    };
    let mut upper_bound = u16::MAX;
    while (too_small + 1) < upper_bound {
        let check_size = too_small + ((upper_bound - too_small) / 2);
        match crunch::Packer::with_items(glyphs.clone().map(&mut map_glyphs)).pack(crunch::Rect {
            x: 1,
            y: 1,
            w: check_size.into(),
            h: check_size.into(),
        }) {
            Ok(res) => {
                result = res;
                upper_bound = check_size;
            }
            Err(_) => {
                too_small = check_size;
            }
        }
    }
    (upper_bound, result)
}
