static DATA: &[u8] = include_bytes!("NotoSans-Regular.ttf");

fn main() {
    for i in 0..10 {
        let face = ttf_parser::Face::parse(DATA, 0).unwrap();
        let data = blurry::build(
            blurry::Settings {
                size: blurry::AssetSize::TextureSize(255, 255),
                padding_ratio: 0.2,
            },
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
                .chars()
                .map(|ch| ((), &face, ch)),
        );
        std::fs::write(format!("test-{i}.data"), data).unwrap();
    }
}
