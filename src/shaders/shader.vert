#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec2 a_tex_coords;

layout(location=5) in vec3 pos_offset;
layout(location=6) in vec2 a_pos_scale;
layout(location=7) in vec2 a_tex_offset;
layout(location=8) in vec2 a_tex_scale;


layout(location=0) out vec2 v_tex_coords;

// NEW!
layout(set=1, binding=0) // 1.
uniform Uniforms {
    vec2 camera_pos; // 2.
};

void main() {
    v_tex_coords = (a_tex_coords * a_tex_scale) + a_tex_offset;
    gl_Position = vec4(a_position  * vec3(a_pos_scale, 1.0) + pos_offset - vec3(camera_pos,0), 1.0);
}