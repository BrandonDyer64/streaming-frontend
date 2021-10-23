use luminance::{pixel::NormRGB8UI, texture::Dim2};
use luminance_derive::{Semantics, Vertex};
use luminance_front::texture::Texture as VTexture;

pub type RGBTexture = VTexture<Dim2, NormRGB8UI>;

#[derive(Copy, Clone, Debug, Semantics)]
pub enum VertexSemantics {
    #[sem(name = "co", repr = "[f32; 2]", wrapper = "VertexPosition")]
    Position,
    #[sem(name = "color", repr = "[u8; 3]", wrapper = "VertexRGB")]
    Color,
    #[sem(
        name = "position",
        repr = "[f32; 2]",
        wrapper = "VertexInstancePosition"
    )]
    InstancePosition,
    #[sem(name = "weight", repr = "f32", wrapper = "VertexWeight")]
    Weight,
}

#[derive(Clone, Copy, Debug, Vertex)]
#[vertex(sem = "VertexSemantics")]
pub struct Vertex {
    #[allow(dead_code)]
    position: VertexPosition,

    #[allow(dead_code)]
    #[vertex(normalized = "true")]
    color: VertexRGB,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Vertex)]
#[vertex(sem = "VertexSemantics", instanced = "true")]
pub struct Instance {
    pub pos: VertexInstancePosition,
    pub w: VertexWeight,
}
