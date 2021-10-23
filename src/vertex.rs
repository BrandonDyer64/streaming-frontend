use glyph_brush::ab_glyph::{point, Rect};
use luminance::{pixel::NormRGB8UI, texture::Dim2};
use luminance_derive::{Semantics, Vertex};
use luminance_front::texture::Texture;

pub type RGBTexture = Texture<Dim2, NormRGB8UI>;

#[derive(Copy, Clone, Debug, Semantics)]
pub enum VertexSemantics {
    #[sem(name = "co", repr = "[f32; 2]", wrapper = "VertexCo")]
    Co,
    #[sem(name = "color", repr = "[u8; 3]", wrapper = "VertexRGB")]
    Color,
    #[sem(name = "left_top", repr = "[f32; 3]", wrapper = "VertexLeftTop")]
    LeftTop,

    #[sem(
        name = "right_bottom",
        repr = "[f32; 2]",
        wrapper = "VertexRightBottom"
    )]
    RightBottom,

    #[sem(name = "tex_left_top", repr = "[f32; 2]", wrapper = "TextureLeftTop")]
    TexLeftTop,

    #[sem(
        name = "tex_right_bottom",
        repr = "[f32; 2]",
        wrapper = "TextureRightBottom"
    )]
    TexRightBottom,

    #[sem(name = "color", repr = "[f32; 4]", wrapper = "TextColor")]
    TextColor,
}

#[derive(Clone, Copy, Debug, Vertex)]
#[vertex(sem = "VertexSemantics")]
pub struct Vertex {
    #[allow(dead_code)]
    position: VertexCo,

    #[allow(dead_code)]
    #[vertex(normalized = "true")]
    color: VertexRGB,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Vertex, Copy, Debug, Clone)]
#[vertex(sem = "VertexSemantics", instanced = "true")]
pub struct TextInstance {
    pub left_top: VertexLeftTop,
    pub right_bottom: VertexRightBottom,
    pub tex_left_top: TextureLeftTop,
    pub tex_right_bottom: TextureRightBottom,
    pub color: TextColor,
}

#[inline]
pub fn to_vertex(
    width: f32,
    height: f32,
    glyph_brush::GlyphVertex {
        mut tex_coords,
        pixel_coords,
        bounds,
        ..
    }: glyph_brush::GlyphVertex,
) -> TextInstance {
    let gl_bounds = bounds;

    let mut gl_rect = Rect {
        min: point(pixel_coords.min.x as f32, pixel_coords.min.y as f32),
        max: point(pixel_coords.max.x as f32, pixel_coords.max.y as f32),
    };
    // println!("GL_RECT = {:?}", gl_rect);

    // handle overlapping bounds, modify uv_rect to preserve texture aspect
    if gl_rect.max.x > gl_bounds.max.x {
        let old_width = gl_rect.width();
        gl_rect.max.x = gl_bounds.max.x;
        tex_coords.max.x = tex_coords.min.x + tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.min.x < gl_bounds.min.x {
        let old_width = gl_rect.width();
        gl_rect.min.x = gl_bounds.min.x;
        tex_coords.min.x = tex_coords.max.x - tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.max.y > gl_bounds.max.y {
        let old_height = gl_rect.height();
        gl_rect.max.y = gl_bounds.max.y;
        tex_coords.max.y = tex_coords.min.y + tex_coords.height() * gl_rect.height() / old_height;
    }
    if gl_rect.min.y < gl_bounds.min.y {
        let old_height = gl_rect.height();
        gl_rect.min.y = gl_bounds.min.y;
        tex_coords.min.y = tex_coords.max.y - tex_coords.height() * gl_rect.height() / old_height;
    }

    let to_view_space = |x: f32, y: f32| -> [f32; 2] {
        let pos_x = (x / width) * 2.0 - 1.0;
        let pos_y = (1.0 - y / height) * 2.0 - 1.0;
        [pos_x, pos_y]
    };

    let left_top = to_view_space(gl_rect.min.x, gl_rect.max.y);

    let v = TextInstance {
        left_top: VertexLeftTop::new([left_top[0], left_top[1], 0.]),
        right_bottom: VertexRightBottom::new(to_view_space(gl_rect.max.x, gl_rect.min.y)),
        tex_left_top: TextureLeftTop::new([tex_coords.min.x, tex_coords.max.y]),
        tex_right_bottom: TextureRightBottom::new([tex_coords.max.x, tex_coords.min.y]),
        color: TextColor::new([1., 1., 1., 1.]),
    };

    // println!("vertex -> {:?}", v);
    v
}
