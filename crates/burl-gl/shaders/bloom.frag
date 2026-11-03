#version 150
// bloom.frag, expression for expression.
in vec3 v_pos;
in vec4 v_tc;
in vec4 v_colour;
out vec4 frag;

void main()
{
    float r = fract(sin(dot(v_pos.xy, vec2(11.3713, 67.3219))) * 2351.3718);

    float offset = (0.5 - r) * v_tc.x * 0.045;

    float intensity = min(1.0, cos((length(v_pos * 2.0) + offset) / v_tc.x));
    float gradient  = intensity * smoothstep(0.0, 2.0, intensity);

    gradient *= smoothstep(1.0, 0.67 + r * 0.33, 1.0 - intensity);

    frag = v_colour * gradient;
}
