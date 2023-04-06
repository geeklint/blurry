use std::fs::OpenOptions;

use blurry::CubicCurve;

struct PrintsOutline {}

impl ttf_parser::OutlineBuilder for PrintsOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        println!("move_to({x}, {y})");
    }

    fn line_to(&mut self, x: f32, y: f32) {
        println!("line_to({x}, {y})");
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        println!("quad_to({x1}, {y1}, {x}, {y})");
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        println!("curve_to({x1}, {y1}, {x2}, {y2}, {x}, {y})");
    }

    fn close(&mut self) {
        println!("close()");
    }
}

static DATA: &[u8] = include_bytes!("/usr/share/fonts/noto/NotoSans-Regular.ttf");

fn main() {
    let face = ttf_parser::Face::parse(DATA, 0).unwrap();
    blurry::build(
        blurry::Settings {
            size: blurry::AssetSize::TextureSize(255, 255),
            padding_ratio: 0.2,
            left_clamp_opt: true,
        },
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
            .chars()
            .map(|ch| ((), &face, ch)),
    );
    /*
    dbg!(face.height());
    let glyph_id = face.glyph_index('A').unwrap();
    face.outline_glyph(glyph_id, &mut PrintsOutline {});

    let mut svg = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("test.html")
        .unwrap();
    use blurry::Edge;
    use std::io::Write;
    let curve = CubicCurve::new((0.0, 0.0), (0.85, 0.46), (0.01, 1.52), (1.0, 1.0));
    let argv: Vec<_> = std::env::args().collect();
    let (px, py) = if let [pxarg, pyarg, ..] = &argv[1..] {
        (pxarg.parse().unwrap(), pyarg.parse().unwrap())
    } else {
        (0.66, 0.5)
    };
    let t = dbg!(curve.nearest_t((px, py)));
    let (x, y) = dbg!(curve.point(t));
    let (dx, dy) = curve.direction(t);
    let curve_side = (dx * (py - y) - dy * (px - x)).signum();
    dbg!(curve_side);
    let dmag = (dx.powi(2) + dy.powi(2)).sqrt();
    let dx = dx / dmag + x;
    let dy = dy / dmag + y;
    let [x, y, px, py, dx, dy] = [x, y, px, py, dx, dy].map(|t| t * 100.0);
    write!(svg, "<html><body><svg width=\"1000\" height=\"1000\">").unwrap();
    write!(
        svg,
        "<path d=\"M0,0 C85,46 1,152 100,100\" stroke=\"magenta\" fill=\"transparent\" />"
    )
    .unwrap();
    write!(
        svg,
        "<path d=\"M{x},{y} L{dx},{dy}\" stroke=\"cyan\" fill=\"transparent\" />"
    )
    .unwrap();
    write!(svg, "<circle cx=\"{px}\" cy=\"{py}\" r=\"5\" />").unwrap();
    write!(
        svg,
        "<circle cx=\"{x}\" cy=\"{y}\" r=\"5\" stroke=\"magenta\" fill=\"transparent\" />"
    )
    .unwrap();
    write!(svg, "</svg></body></html>").unwrap();
    */
}
