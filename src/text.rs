use glyph_brush::{BrushAction, BrushError, GlyphBrush};
use luminance::{
    blending::{Blending, Equation, Factor},
    pipeline::{PipelineError, TextureBinding},
    pixel::{NormR8UI, NormUnsigned},
    render_state::RenderState,
    shader::Uniform,
    tess::Mode,
    texture::{Dim2, GenMipmaps, Sampler},
};
use luminance_derive::UniformInterface;
use luminance_front::{
    context::GraphicsContext, pipeline::Pipeline, shader::Program, shading_gate::ShadingGate,
    tess::Tess, texture::Texture,
};
use luminance_glfw::GL33Context;

use crate::{vertex::*, HEIGHT, WIDTH};

#[derive(UniformInterface)]
pub struct TextShaderInterface {
    pub tex: Uniform<TextureBinding<Dim2, NormUnsigned>>,
}

const VS_FONT_STR: &str = include_str!("text.vert.glsl");
const FS_FONT_STR: &str = include_str!("text.frag.glsl");

pub struct TextRenderer {
    program: Program<VertexSemantics, (), TextShaderInterface>,
    render_state: RenderState,
    texture: Texture<Dim2, NormR8UI>,
    pub tess: Option<Tess<(), (), TextInstance>>,
}

impl TextRenderer {
    pub fn new(ctxt: &mut GL33Context, glyph_brush: &mut GlyphBrush<TextInstance>) -> Self {
        let program = ctxt
            .new_shader_program::<VertexSemantics, (), TextShaderInterface>()
            .from_strings(VS_FONT_STR, None, None, FS_FONT_STR)
            .expect("Program creation")
            .ignore_warnings();

        let render_state = RenderState::default()
            .set_blending(Blending {
                equation: Equation::Additive,
                src: Factor::SrcAlpha,
                dst: Factor::Zero,
            })
            .set_depth_test(None);

        let texture = Texture::new(
            ctxt,
            [
                glyph_brush.texture_dimensions().0,
                glyph_brush.texture_dimensions().1,
            ],
            0,
            Sampler::default(),
            GenMipmaps::No,
            &vec![
                0u8;
                glyph_brush.texture_dimensions().0 as usize
                    * glyph_brush.texture_dimensions().1 as usize
            ],
        )
        .expect("luminance texture creation");

        Self {
            program,
            render_state,
            texture,
            tess: None,
        }
    }

    pub fn process_queued(
        &mut self,
        ctxt: &mut GL33Context,
        glyph_brush: &mut GlyphBrush<TextInstance>,
    ) {
        let action = glyph_brush.process_queued(
            |rect, tex_data| {
                self.texture
                    .upload_part_raw(
                        GenMipmaps::No,
                        [rect.min[0] as u32, rect.min[1] as u32],
                        [rect.width() as u32, rect.height() as u32],
                        tex_data,
                    )
                    .expect("Cannot upload part of texture");
            },
            |vertex_data| to_vertex(WIDTH as f32 * 2., HEIGHT as f32 * 2., vertex_data),
        );

        if let Err(e) = action {
            let BrushError::TextureTooSmall { suggested } = e;
            glyph_brush.resize_texture(suggested.0, suggested.1);
            return;
        }
        let action = action.unwrap();
        match action {
            BrushAction::Draw(v) => {
                let tess = ctxt
                    .new_tess()
                    .set_render_vertex_nb(4)
                    .set_instances(v)
                    .set_mode(Mode::TriangleStrip)
                    .build()
                    .unwrap();
                self.tess = Some(tess);
            }
            BrushAction::ReDraw => (),
        };
    }

    pub fn render(
        &mut self,
        pipeline: &Pipeline,
        shd_gate: &mut ShadingGate,
    ) -> Result<(), PipelineError> {
        if let Some(tess) = self.tess.as_ref() {
            let program = &mut self.program;
            let texture = &mut self.texture;
            let render_state = &self.render_state;
            shd_gate.shade(program, |mut iface, uni, mut rdr_gate| {
                let bound_tex = pipeline.bind_texture(texture)?;
                iface.set(&uni.tex, bound_tex.binding());
                rdr_gate.render(render_state, |mut tess_gate| tess_gate.render(tess))
            })?;
        }
        Ok(())
    }
}
