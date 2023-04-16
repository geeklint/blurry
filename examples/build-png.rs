use std::{fs::File, path::Path};

use blurry::{FontAssetBuilder, Glyph, GlyphRequest};

static FONT_DATA: &[u8] = include_bytes!("roboto/Roboto-Regular.ttf");

fn main() {
    let face = ttf_parser::Face::parse(FONT_DATA, 0).unwrap();
    let asset = FontAssetBuilder::with_texture_size(255, 255)
        .build(blurry::latin1().map(|codepoint| GlyphRequest {
            user_data: (),
            face: &face,
            codepoint,
        }))
        .unwrap();
    let mut output_path = Path::new(file!()).parent().unwrap().to_path_buf();
    output_path.push("demo-sdf.png");
    let file = File::create(&output_path).unwrap();
    let mut encoder = png::Encoder::new(file, 255, 255);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&asset.data)
        .unwrap();
    for Glyph {
        codepoint,
        tex_left,
        tex_bottom,
        ..
    } in asset.metadata
    {
        println!("glyph '{codepoint}' @ {tex_left} , {tex_bottom}");
    }
    println!("checkout the image at '{}'", output_path.to_string_lossy());
}
