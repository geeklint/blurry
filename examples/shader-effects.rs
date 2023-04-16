use std::fmt::Write;

use glow::HasContext;

use blurry::{latin1, ttf_parser::Face, FontAssetBuilder, Glyph, GlyphRequest};

static DISPLAY_FONT_SIZE: f32 = 30.0;
const PADDING_RATIO: f32 = 0.3;

#[derive(Clone, Copy, Debug)]
struct AdvanceWidth(f32);

fn update_font(
    gl: &glow::Context,
    texture: glow::Texture,
    ttf_data: &[u8],
) -> Result<Vec<Glyph<AdvanceWidth>>, &'static str> {
    let face = Face::parse(ttf_data, 0).map_err(|_| "failed to parse font file")?;
    let height = f32::from(face.height());
    let mut asset = FontAssetBuilder::with_font_size(30.0)
        .with_padding_ratio(PADDING_RATIO)
        .build(latin1().map_while(|codepoint| {
            let advance_width: f32 = face
                .glyph_index(codepoint)
                .and_then(|glyph_id| face.glyph_hor_advance(glyph_id))
                .unwrap_or(0)
                .into();
            Some(GlyphRequest {
                user_data: AdvanceWidth(advance_width / height),
                face: &face,
                codepoint,
            })
        }))
        .map_err(|err| match err {
            blurry::Error::MissingGlyph(_) => "the font file didn't contain all the characters",
            blurry::Error::PackingAtlasFailed => {
                "we failed to pack the glyphs into a single texture"
            }
            _ => "an unspecified error occurred",
        })?;
    let space_width = face
        .glyph_index(' ')
        .and_then(|idx| face.glyph_hor_advance(idx))
        .map(|w| f32::from(w) / height)
        .unwrap_or(0.25);
    let mut space_glyph = asset.metadata[0];
    space_glyph.user_data = AdvanceWidth(space_width);
    space_glyph.codepoint = ' ';
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RED.try_into().unwrap(),
            asset.width.into(),
            asset.height.into(),
            0,
            glow::RED,
            glow::UNSIGNED_BYTE,
            Some(&asset.data),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as _,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as _,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as _,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as _,
        );
    }
    asset.metadata.push(space_glyph);
    Ok(asset.metadata)
}

static FIRST_FONT: &[u8] = include_bytes!("roboto/Roboto-Regular.ttf");

#[derive(Clone, Copy, Debug)]
struct Offset(f32, f32);

#[derive(Clone, Copy, Debug)]
struct Color(f32, f32, f32, f32);

#[derive(Clone, Copy, Debug)]
struct Config(f32, f32, f32);

#[derive(Clone, Copy, Debug)]
struct EffectPreset {
    name: &'static str,
    background_color: Color,
    effects: &'static [(Offset, Color, Config)],
}

static EFFECTS: &[EffectPreset] = &[
    EffectPreset {
        name: "white on grey",
        background_color: Color(0.314, 0.314, 0.314, 1.0),
        effects: &[(
            Offset(0.0, 0.0),
            Color(0.867, 0.867, 0.867, 1.0),
            Config(0.5, 2.0, 0.0),
        )],
    },
    EffectPreset {
        name: "basic drop shadow",
        background_color: Color(0.6, 0.5, 0.7, 1.0),
        effects: &[
            (
                Offset(0.06, -0.06),
                Color(0.0, 0.0, 0.0, 0.7),
                Config(0.5, 2.0, 0.075),
            ),
            (
                Offset(0.0, 0.0),
                Color(1.0, 1.0, 1.0, 1.0),
                Config(0.5, 2.0, 0.0),
            ),
        ],
    },
    EffectPreset {
        name: "clouds",
        background_color: Color(0.0, 0.656, 1.0, 1.0),
        effects: &[
            (
                Offset(0.0, 0.0),
                Color(1.0, 1.0, 1.0, 1.0),
                Config(0.1, 2.0, 0.0),
            ),
            (
                Offset(0.0, 0.0),
                Color(0.0, 0.656, 1.0, 1.0),
                Config(0.45, 2.0, 0.0),
            ),
        ],
    },
    EffectPreset {
        name: "over desktop",
        background_color: Color(0.0, 0.0, 0.0, 0.0),
        effects: &[
            (
                Offset(0.0, 0.0),
                Color(0.0, 0.0, 0.0, 1.0),
                Config(0.4, 2.0, 0.0),
            ),
            (
                Offset(0.0, 0.0),
                Color(1.0, 1.0, 1.0, 1.0),
                Config(0.5, 2.0, 0.0),
            ),
        ],
    },
];

fn main() {
    unsafe {
        let event_loop = glutin::event_loop::EventLoop::new();
        let window_builder = glutin::window::WindowBuilder::new()
            .with_title("Hello triangle!")
            .with_transparent(true)
            .with_inner_size(glutin::dpi::LogicalSize::new(500.0, 500.0));
        let window = glutin::ContextBuilder::new()
            .with_vsync(true)
            .build_windowed(window_builder, &event_loop)
            .unwrap()
            .make_current()
            .unwrap();
        let gl = glow::Context::from_loader_function(|s| window.get_proc_address(s) as *const _);
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));
        let program = setup_shader_program(&gl);
        gl.use_program(Some(program));
        let vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        let texture = gl.create_texture().unwrap();
        let mut glyphs = update_font(&gl, texture, FIRST_FONT).unwrap();

        let default_text = "You can type to change this text. Click inside the window for the next style. Drag and drop a font file on the window to change the font.";
        let mut typed_text = String::new();
        let mut current_effect = 0;
        println!("switched to effect '{}'", EFFECTS[current_effect].name);

        let logical_size: glutin::dpi::LogicalSize<f32> = window
            .window()
            .inner_size()
            .to_logical(window.window().scale_factor());
        let mut font_mul_x = 2.0 * DISPLAY_FONT_SIZE / logical_size.width;
        let mut font_mul_y = 2.0 * DISPLAY_FONT_SIZE / logical_size.height;

        event_loop.run(move |event, _, control_flow| {
            use glutin::{
                event::{ElementState, Event, MouseButton, WindowEvent},
                event_loop::ControlFlow,
            };
            *control_flow = ControlFlow::Wait;
            match event {
                Event::RedrawRequested(_) => {
                    let effect_preset = EFFECTS[current_effect];
                    let Color(r, g, b, a) = effect_preset.background_color;
                    gl.clear_color(r, g, b, a);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                    gl.enable(glow::BLEND);
                    gl.blend_func_separate(
                        glow::ONE,
                        glow::ONE_MINUS_SRC_ALPHA,
                        glow::ONE,
                        glow::ONE,
                    );
                    let text = if typed_text.is_empty() {
                        default_text
                    } else {
                        &typed_text
                    };
                    let mut data = Vec::new();
                    let mut cursor_x = -1.0;
                    let mut cursor_y = 1.0 - font_mul_y;
                    for ch in text.chars() {
                        if let Some(glyph) = glyphs.iter().find(|glyph| glyph.codepoint == ch) {
                            let AdvanceWidth(advance) = glyph.user_data;
                            if !ch.is_whitespace() {
                                if (cursor_x + (advance * font_mul_x)) > 1.0 {
                                    cursor_x = -1.0;
                                    cursor_y -= font_mul_y;
                                }
                                push_glyph(
                                    &mut data, cursor_x, cursor_y, font_mul_x, font_mul_y, glyph,
                                );
                            }
                            cursor_x += advance * font_mul_x;
                        }
                    }
                    let item_size = std::mem::size_of::<[f32; 4]>();
                    let raw_buffer =
                        std::slice::from_raw_parts(data.as_ptr().cast(), data.len() * item_size);
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, raw_buffer, glow::DYNAMIC_DRAW);
                    gl.enable_vertex_attrib_array(0);
                    gl.vertex_attrib_pointer_f32(0, 4, glow::FLOAT, false, 0, 0);
                    let offset_uniform = gl.get_uniform_location(program, "offset");
                    let sdf_uniform = gl.get_uniform_location(program, "sdf");
                    let color_uniform = gl.get_uniform_location(program, "color");
                    let config_uniform = gl.get_uniform_location(program, "config");
                    gl.uniform_1_i32(sdf_uniform.as_ref(), 0);
                    gl.active_texture(glow::TEXTURE0);
                    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                    for (Offset(x, y), Color(r, g, b, a), Config(start, end, smoothing)) in
                        effect_preset.effects.iter().copied()
                    {
                        let smoothing = if smoothing == 0.0 {
                            let distance_range_in_px = DISPLAY_FONT_SIZE * (2.0 * PADDING_RATIO);
                            1.0 / distance_range_in_px
                        } else {
                            smoothing
                        };
                        gl.uniform_2_f32(offset_uniform.as_ref(), x * font_mul_x, y * font_mul_y);
                        gl.uniform_4_f32(color_uniform.as_ref(), r, g, b, a);
                        gl.uniform_3_f32(config_uniform.as_ref(), start, end, smoothing);
                        gl.draw_arrays(glow::TRIANGLES, 0, data.len() as _);
                    }
                    window.swap_buffers().unwrap();
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(size) => {
                        window.resize(size);
                        gl.viewport(0, 0, size.width as _, size.height as _);
                        let logical_size: glutin::dpi::LogicalSize<f32> = window
                            .window()
                            .inner_size()
                            .to_logical(window.window().scale_factor());
                        font_mul_x = 2.0 * DISPLAY_FONT_SIZE / logical_size.width;
                        font_mul_y = 2.0 * DISPLAY_FONT_SIZE / logical_size.height;
                        window.window().request_redraw();
                    }
                    WindowEvent::DroppedFile(path) => {
                        match std::fs::read(path) {
                            Ok(ttf_data) => match update_font(&gl, texture, &ttf_data) {
                                Ok(new_glyphs) => {
                                    glyphs = new_glyphs;
                                }
                                Err(err) => {
                                    eprintln!("{}", err);
                                    typed_text.replace_range(.., err);
                                }
                            },
                            Err(err) => {
                                typed_text.clear();
                                let _ = write!(typed_text, "couldn't read dropped file: {}", err);
                                eprintln!("{}", typed_text);
                            }
                        }
                        window.window().request_redraw();
                    }
                    WindowEvent::ReceivedCharacter(ch) => {
                        if ch == '\u{8}' {
                            typed_text.pop();
                        } else if ch == ' ' || latin1().any(|c| c == ch) {
                            typed_text.push(ch);
                        }
                        window.window().request_redraw();
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } => {
                        current_effect = (current_effect + 1) % EFFECTS.len();
                        println!("switched to effect '{}'", EFFECTS[current_effect].name);
                        window.window().request_redraw();
                    }
                    _ => {}
                },
                _ => {}
            }
        });
    }
}

unsafe fn setup_shader_program(gl: &glow::Context) -> glow::Program {
    let program = gl.create_program().unwrap();
    let vertex_shader = gl.create_shader(glow::VERTEX_SHADER).unwrap();
    let fragment_shader = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
    gl.shader_source(
        vertex_shader,
        "#version 410
        uniform vec2 offset;
        layout(location = 0) in vec4 vert_input;
        out vec2 uv;
        void main() {
            uv = vert_input.zw;
            gl_Position = vec4(offset + vert_input.xy, 0.0, 1.0);
        }
        ",
    );
    gl.shader_source(
        fragment_shader,
        "#version 410
        uniform sampler2D sdf;
        uniform vec4 color;
        uniform vec3 config;
        in vec2 uv;
        out vec4 out_color;
        void main() {
            float dist = texture2D(sdf, uv.xy).r;
            float start = 0.5 + (dist - config.x) / config.z;
            float end = 0.5 + (config.y - dist) / config.z;
            float inside = clamp(min(start, end), 0.0, 1.0);
            out_color = color * inside;
        }
        ",
    );
    gl.compile_shader(vertex_shader);
    if !gl.get_shader_compile_status(vertex_shader) {
        panic!("{}", gl.get_shader_info_log(vertex_shader));
    }
    gl.attach_shader(program, vertex_shader);
    gl.compile_shader(fragment_shader);
    if !gl.get_shader_compile_status(fragment_shader) {
        panic!("{}", gl.get_shader_info_log(fragment_shader));
    }
    gl.attach_shader(program, fragment_shader);
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        panic!("{}", gl.get_program_info_log(program));
    }
    gl.detach_shader(program, vertex_shader);
    gl.detach_shader(program, fragment_shader);
    gl.delete_shader(vertex_shader);
    gl.delete_shader(fragment_shader);
    program
}

fn push_glyph(
    data: &mut Vec<[f32; 4]>,
    offset_x: f32,
    offset_y: f32,
    font_mul_x: f32,
    font_mul_y: f32,
    glyph: &Glyph<AdvanceWidth>,
) {
    // first triangle
    data.push([
        offset_x + glyph.left * font_mul_x,
        offset_y + glyph.bottom * font_mul_y,
        glyph.tex_left,
        glyph.tex_bottom,
    ]);
    data.push([
        offset_x + glyph.right * font_mul_x,
        offset_y + glyph.bottom * font_mul_y,
        glyph.tex_right,
        glyph.tex_bottom,
    ]);
    data.push([
        offset_x + glyph.left * font_mul_x,
        offset_y + glyph.top * font_mul_y,
        glyph.tex_left,
        glyph.tex_top,
    ]);
    // second triangle
    data.push([
        offset_x + glyph.right * font_mul_x,
        offset_y + glyph.bottom * font_mul_y,
        glyph.tex_right,
        glyph.tex_bottom,
    ]);
    data.push([
        offset_x + glyph.right * font_mul_x,
        offset_y + glyph.top * font_mul_y,
        glyph.tex_right,
        glyph.tex_top,
    ]);
    data.push([
        offset_x + glyph.left * font_mul_x,
        offset_y + glyph.top * font_mul_y,
        glyph.tex_left,
        glyph.tex_top,
    ]);
}
