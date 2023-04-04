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
