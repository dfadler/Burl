#version 150
// shadow.vert / text.vert: pass the colour and texcoord through (the C++ uses ftransform();
// here the projection*view matrix is the `mvp` uniform and `offset` is the shadow glTranslatef).
uniform mat4 mvp;
uniform vec2 offset;

in vec2 pos;
in vec4 colour;
in vec4 uv;

out vec4 v_colour;
out vec2 v_uv;

void main()
{
    v_uv = uv.xy;
    v_colour = colour;
    gl_Position = mvp * vec4(pos + offset, 0.0, 1.0);
}
