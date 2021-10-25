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
pub struct Instance {
    pub left_top: VertexLeftTop,
    pub right_bottom: VertexRightBottom,
    pub tex_left_top: TextureLeftTop,
    pub tex_right_bottom: TextureRightBottom,
    pub color: TextColor,
}
