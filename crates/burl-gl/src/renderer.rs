//! The one `Renderer`: draws `burl_render` batches into an exact-size offscreen framebuffer, so
//! windowed and headless output share a path (#19 item 2).

use crate::{GlContext, Result};
use burl_render::{
    flip_rows, ortho_2d, quad_indices, Batch, Blend, Pipeline, Space, TextureId, Vertices,
};
use glam::Mat4;
use glow::HasContext;
use std::collections::HashMap;

const QUAD_VERT: &str = include_str!("../shaders/quad.vert");
const PLAIN_FRAG: &str = include_str!("../shaders/plain.frag");
const SHADOW_FRAG: &str = include_str!("../shaders/shadow.frag");
const TEXT_FRAG: &str = include_str!("../shaders/text.frag");
const BLOOM_VERT: &str = include_str!("../shaders/bloom.vert");
const BLOOM_FRAG: &str = include_str!("../shaders/bloom.frag");

/// Attribute widths in floats: pos, colour, uv (vec2 for quads, vec4 for bloom).
const QUAD_LAYOUT: [i32; 3] = [2, 4, 2];
const BLOOM_LAYOUT: [i32; 3] = [2, 4, 4];

// ponytail: GL objects are not deleted individually; they die with the context. Add Drop when a
// renderer outlives its context or textures are replaced per frame.
pub struct Renderer<C: GlContext> {
    ctx: C,
    plain: glow::Program,
    shadow: glow::Program,
    text: glow::Program,
    bloom: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ibo: glow::Buffer,
    /// 1x1 white: bound for batches without a texture (texture * colour = colour).
    white: glow::Texture,
    textures: HashMap<TextureId, glow::Texture>,
    fbo: glow::Framebuffer,
    size: (u32, u32),
}

fn program(gl: &glow::Context, vs: &str, fs: &str) -> Result<glow::Program> {
    // SAFETY: plain object creation on a current context; every handle is checked before use.
    unsafe {
        let p = gl.create_program()?;
        for (ty, src) in [(glow::VERTEX_SHADER, vs), (glow::FRAGMENT_SHADER, fs)] {
            let s = gl.create_shader(ty)?;
            gl.shader_source(s, src);
            gl.compile_shader(s);
            if !gl.get_shader_compile_status(s) {
                return Err(format!("shader: {}", gl.get_shader_info_log(s)));
            }
            gl.attach_shader(p, s);
            gl.delete_shader(s);
        }
        // The attribute order is the layout above; bind before linking.
        for (i, name) in ["pos", "colour", "uv"].iter().enumerate() {
            gl.bind_attrib_location(p, i as u32, name);
        }
        gl.link_program(p);
        if !gl.get_program_link_status(p) {
            return Err(format!("link: {}", gl.get_program_info_log(p)));
        }
        Ok(p)
    }
}

impl<C: GlContext> Renderer<C> {
    pub fn new(ctx: C) -> Result<Self> {
        ctx.make_current()?;
        let gl = ctx.gl();
        let size = ctx.size();
        let (w, h) = (size.0 as i32, size.1 as i32);
        let plain = program(gl, QUAD_VERT, PLAIN_FRAG)?;
        let shadow = program(gl, QUAD_VERT, SHADOW_FRAG)?;
        let text = program(gl, QUAD_VERT, TEXT_FRAG)?;
        let bloom = program(gl, BLOOM_VERT, BLOOM_FRAG)?;
        // SAFETY: objects are created on the current context and only used with it; the
        // framebuffer completeness is checked before returning.
        unsafe {
            let vao = gl.create_vertex_array()?;
            let vbo = gl.create_buffer()?;
            let ibo = gl.create_buffer()?;
            let white = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(white));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                1,
                1,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&[255; 4])),
            );
            let fbo = gl.create_framebuffer()?;
            let rb = gl.create_renderbuffer()?;
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(rb));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::RGBA8, w, h);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::RENDERBUFFER,
                Some(rb),
            );
            if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                return Err(format!("framebuffer incomplete at {w}x{h}"));
            }
            // The C++ never enables depth testing, so there is no depth attachment.
            Ok(Self {
                ctx,
                plain,
                shadow,
                text,
                bloom,
                vao,
                vbo,
                ibo,
                white,
                textures: HashMap::new(),
                fbo,
                size,
            })
        }
    }

    pub fn context(&self) -> &C {
        &self.ctx
    }

    /// Upload RGBA8 pixels (top row first) as texture `id`: linear filtering, clamp to edge.
    pub fn upload_texture(
        &mut self,
        id: TextureId,
        width: u32,
        height: u32,
        rgba: &[u8],
        mipmaps: bool,
    ) -> Result<()> {
        if rgba.len() != width as usize * height as usize * 4 {
            return Err(format!(
                "texture {}: {} bytes for {width}x{height} RGBA",
                id.0,
                rgba.len()
            ));
        }
        self.ctx.make_current()?;
        let gl = self.ctx.gl();
        // SAFETY: the slice length was checked against the dimensions above.
        unsafe {
            let t = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(t));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(rgba)),
            );
            let min = if mipmaps {
                glow::LINEAR_MIPMAP_LINEAR
            } else {
                glow::LINEAR
            };
            if mipmaps {
                gl.generate_mipmap(glow::TEXTURE_2D);
            }
            let err = gl.get_error();
            if err != glow::NO_ERROR {
                gl.delete_texture(t);
                return Err(format!(
                    "texture {}: GL error 0x{err:x} during upload",
                    id.0
                ));
            }
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, min as i32);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            if let Some(old) = self.textures.insert(id, t) {
                gl.delete_texture(old);
            }
        }
        Ok(())
    }

    /// Clear to `clear` (RGBA, `Display::clear`) and draw `batches` in order. `world` is the
    /// camera's projection*view for `Space::World` batches; `Space::Screen` batches use
    /// `ortho_2d` of the output size.
    pub fn render(&mut self, clear: [f32; 4], batches: &[Batch], world: Mat4) -> Result<()> {
        self.ctx.make_current()?;
        let gl = self.ctx.gl();
        let (w, h) = (self.size.0 as i32, self.size.1 as i32);
        let screen = ortho_2d(w as f32, h as f32);
        // SAFETY: all handles belong to this context; buffer uploads are sized from the slices
        // they come from, and the vertex pointers match the interleaved layouts declared above.
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
            gl.viewport(0, 0, w, h);
            gl.clear_color(clear[0], clear[1], clear[2], clear[3]);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.enable(glow::BLEND);
            gl.bind_vertex_array(Some(self.vao));
            gl.active_texture(glow::TEXTURE0);
            for b in batches {
                let (floats, layout, quads): (Vec<f32>, _, _) = match &b.vertices {
                    Vertices::Quads(v) => (
                        v.iter()
                            .flat_map(|v| {
                                [v.pos[0], v.pos[1]].into_iter().chain(v.colour).chain(v.uv)
                            })
                            .collect(),
                        QUAD_LAYOUT,
                        v.len() / 4,
                    ),
                    Vertices::Bloom(v) => (
                        v.iter()
                            .flat_map(|v| {
                                [v.pos[0], v.pos[1]].into_iter().chain(v.colour).chain(v.uv)
                            })
                            .collect(),
                        BLOOM_LAYOUT,
                        v.len() / 4,
                    ),
                };
                match b.blend {
                    Blend::Alpha => gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA),
                    Blend::Additive => gl.blend_func(glow::ONE, glow::ONE),
                }
                let prog = match b.pipeline {
                    Pipeline::Plain => self.plain,
                    Pipeline::Shadow => self.shadow,
                    Pipeline::Bloom => self.bloom,
                    Pipeline::Text => self.text,
                };
                gl.use_program(Some(prog));
                let mvp = match b.space {
                    Space::Screen => screen,
                    Space::World => world,
                };
                gl.uniform_matrix_4_f32_slice(
                    gl.get_uniform_location(prog, "mvp").as_ref(),
                    false,
                    &mvp.to_cols_array(),
                );
                gl.uniform_2_f32(
                    gl.get_uniform_location(prog, "offset").as_ref(),
                    b.offset[0],
                    b.offset[1],
                );
                gl.uniform_1_i32(gl.get_uniform_location(prog, "tex").as_ref(), 0);
                gl.uniform_1_f32(
                    gl.get_uniform_location(prog, "shadow_strength").as_ref(),
                    match b.pipeline {
                        Pipeline::Text => burl_render::TEXT_SHADOW_STRENGTH,
                        _ => burl_render::SHADOW_STRENGTH,
                    },
                );
                gl.uniform_1_f32(
                    gl.get_uniform_location(prog, "texel_size").as_ref(),
                    1.0 / burl_render::PAGE_SIZE as f32,
                );
                let tex = match b.texture {
                    Some(t) => *self
                        .textures
                        .get(&t)
                        .ok_or_else(|| format!("texture {} not uploaded", t.0))?,
                    None => self.white,
                };
                gl.bind_texture(glow::TEXTURE_2D, Some(tex));

                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, f32_bytes(&floats), glow::DYNAMIC_DRAW);
                let stride = layout.iter().sum::<i32>() * 4;
                let mut offset = 0;
                for (i, n) in layout.iter().enumerate() {
                    gl.enable_vertex_attrib_array(i as u32);
                    gl.vertex_attrib_pointer_f32(i as u32, *n, glow::FLOAT, false, stride, offset);
                    offset += n * 4;
                }
                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ibo));
                let idx = quad_indices(quads);
                gl.buffer_data_u8_slice(
                    glow::ELEMENT_ARRAY_BUFFER,
                    u32_bytes(&idx),
                    glow::DYNAMIC_DRAW,
                );
                gl.draw_elements(glow::TRIANGLES, idx.len() as i32, glow::UNSIGNED_INT, 0);
            }
            match gl.get_error() {
                glow::NO_ERROR => Ok(()),
                e => Err(format!("GL error 0x{e:x}")),
            }
        }
    }

    /// The last rendered frame as tightly packed RGBA8, top row first.
    pub fn read_frame(&self) -> Result<Vec<u8>> {
        self.ctx.make_current()?;
        let gl = self.ctx.gl();
        let (w, h) = (self.size.0 as usize, self.size.1 as usize);
        let mut px = vec![0u8; w * h * 4];
        // SAFETY: `px` is exactly w*h*4 bytes, the size glReadPixels writes at PACK_ALIGNMENT 1.
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
            gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            gl.read_pixels(
                0,
                0,
                w as i32,
                h as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut px)),
            );
            match gl.get_error() {
                glow::NO_ERROR => {}
                e => return Err(format!("GL error 0x{e:x} reading pixels")),
            }
        }
        Ok(flip_rows(&px, w, h, 4))
    }

    pub fn gl_info(&self) -> Result<String> {
        self.ctx.make_current()?;
        let gl = self.ctx.gl();
        // SAFETY: parameter queries on the current context.
        unsafe {
            Ok(format!(
                "{} | {}",
                gl.get_parameter_string(glow::VERSION),
                gl.get_parameter_string(glow::RENDERER)
            ))
        }
    }
}

fn f32_bytes(v: &[f32]) -> &[u8] {
    // SAFETY: f32 has no padding or invalid bit patterns as bytes; the length is scaled to match.
    unsafe { std::slice::from_raw_parts(v.as_ptr().cast(), std::mem::size_of_val(v)) }
}

fn u32_bytes(v: &[u32]) -> &[u8] {
    // SAFETY: as `f32_bytes`.
    unsafe { std::slice::from_raw_parts(v.as_ptr().cast(), std::mem::size_of_val(v)) }
}
