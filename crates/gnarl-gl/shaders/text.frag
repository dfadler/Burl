#version 150
// text.frag: coverage is the texture's alpha. The shadow is the same texture sampled one texel up
// and left, scaled by shadow_strength, composited under the glyph (data/shaders/text.frag).
uniform sampler2D tex;
uniform float shadow_strength;
uniform float texel_size;

in vec4 v_colour;
in vec2 v_uv;
out vec4 frag;

void main()
{
    float colour_alpha = texture(tex, v_uv).a;
    float shadow_alpha = texture(tex, v_uv - vec2(texel_size)).a * shadow_strength;

    float combined_alpha = 1.0 - (1.0 - shadow_alpha) * (1.0 - colour_alpha);

    if (combined_alpha > 0.0) colour_alpha /= combined_alpha;

    frag = v_colour * vec4(vec3(colour_alpha), combined_alpha);
}
