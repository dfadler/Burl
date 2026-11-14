#version 150
// The C++ draws these passes with program 0: fixed-function GL_MODULATE, texture * vertex colour.
uniform sampler2D tex;

in vec4 v_colour;
in vec2 v_uv;
out vec4 frag;

void main()
{
    frag = v_colour * texture(tex, v_uv);
}
