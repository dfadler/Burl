#version 150
// shadow.frag
uniform sampler2D tex;
uniform float shadow_strength;

in vec4 v_colour;
in vec2 v_uv;
out vec4 frag;

void main(void)
{
    vec4 colour = texture(tex, v_uv);

    frag = vec4(0.0, 0.0, 0.0, v_colour.w * colour.w * shadow_strength);
}
