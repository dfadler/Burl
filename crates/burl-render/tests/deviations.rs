//! Burl-only tests pinning the phase 5 rows of docs/rust-rewrite/DEVIATIONS.md (D10, D11, G5).

use burl_render::*;
use glam::{Vec2, Vec4};

#[test]
fn deviation_d10_bloom_vertex_has_no_padding() {
    // C++ `bloom_vertex` is 64 bytes: vec2 + vec4 + vec4 (40) plus `char padding[24]`.
    assert_eq!(std::mem::size_of::<BloomVertex>(), 40);
}

/// GLSL `smoothstep` with edges in either order, as the C++ shader's reversed call evaluates it.
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[test]
fn deviation_d11_bloom_ordered_edge_smoothstep() {
    // The shader keeps the ordered form...
    let frag = include_str!("../../burl-gl/shaders/bloom.frag");
    assert!(frag.contains("1.0 - smoothstep(0.67 + r * 0.33, 1.0, 1.0 - intensity)"));
    // ...which is the same curve as the C++ reversed-edge `smoothstep(1.0, 0.67 + r*0.33, x)`.
    // r = 1 makes both edges 1.0, where either form is undefined; r < 1 in practice.
    for r in [0.0f32, 0.25, 0.5, 0.99] {
        let a = 0.67 + r * 0.33;
        for i in 0..=100 {
            let x = i as f32 / 100.0;
            let (cpp, burl) = (smoothstep(1.0, a, x), 1.0 - smoothstep(a, 1.0, x));
            assert!((cpp - burl).abs() < 1e-5, "r={r} x={x}: {cpp} vs {burl}");
        }
    }
}

#[test]
fn deviation_g5_quadbuf_skips_vertices_before_first_run() {
    let quad = |buf: &mut QuadBuf, tex| {
        buf.add(tex, Vec2::ZERO, Vec2::ONE, Vec4::ONE, FULL_TEXCOORD);
    };
    let edge_vertices = |buf: QuadBuf| -> usize {
        let frame = Frame {
            edges: buf,
            ..Frame::default()
        };
        frame
            .batches()
            .iter()
            .filter(|b| b.pass == Pass::Edges)
            .map(|b| match &b.vertices {
                Vertices::Quads(v) => v.len(),
                Vertices::Bloom(v) => v.len(),
            })
            .sum()
    };
    // No runs: the whole buffer is drawn untextured.
    let mut b = QuadBuf::default();
    quad(&mut b, None);
    assert_eq!(edge_vertices(b), 4);
    // An untextured quad before the first texture run is not drawn (as in C++).
    let mut b = QuadBuf::default();
    quad(&mut b, None);
    quad(&mut b, Some(TextureId(1)));
    assert_eq!(edge_vertices(b), 4);
}
