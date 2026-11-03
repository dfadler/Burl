#version 150
// bloom.vert: the node centre rides in texcoord.yzw, the radius scale in texcoord.x.
uniform mat4 mvp;
uniform vec2 offset;

in vec2 pos;
in vec4 colour;
in vec4 uv;

out vec3 v_pos;
out vec4 v_tc;
out vec4 v_colour;

void main()
{
    v_pos = vec3(pos, 0.0) - uv.yzw;
    v_tc = uv;
    v_colour = colour;
    gl_Position = mvp * vec4(pos + offset, 0.0, 1.0);
}
