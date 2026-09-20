#version 300 es
precision mediump float;
layout(location = 0) in vec2 position;
layout(location = 1) in vec4 a_bounds;    // rect_pos.xy, rect_size.zw
layout(location = 2) in vec4 a_uv_bounds; // uv_min.xy, uv_max.zw
layout(location = 3) in vec4 a_params;    // radius, alpha, _pad1, _pad2

out vec2 v_uv;
out vec2 v_local_pos;
out vec2 v_screen_pos;
out vec2 v_rect_size;
out float v_radius;
out float v_alpha;

uniform vec2 u_resolution;
uniform mat3 u_transform;

// Backward-compatible fallback uniforms (used when u_instanced == 0)
uniform int u_instanced;
uniform vec2 u_rect_pos;
uniform vec2 u_rect_size;
uniform vec2 u_uv_scale;
uniform vec2 u_uv_offset;
uniform float u_radius;
uniform float u_global_alpha;

void main() {
    vec2 rect_pos = (u_instanced == 1) ? a_bounds.xy : u_rect_pos;
    vec2 rect_size = (u_instanced == 1) ? a_bounds.zw : u_rect_size;

    vec3 pixel_pos_3 = u_transform * vec3(position * rect_size + rect_pos, 1.0);
    vec2 pixel_pos = pixel_pos_3.xy;
    v_screen_pos = pixel_pos;
    v_local_pos = position * rect_size;
    v_rect_size = rect_size;

    if (u_instanced == 1) {
        v_uv = mix(a_uv_bounds.xy, a_uv_bounds.zw, position);
        v_radius = a_params.x;
        v_alpha = a_params.y;
    } else {
        v_uv = position * u_uv_scale + u_uv_offset;
        v_radius = u_radius;
        v_alpha = u_global_alpha;
    }

    vec2 zero_to_one = pixel_pos / u_resolution;
    vec2 zero_to_two = zero_to_one * 2.0;
    vec2 clip_space = zero_to_two - 1.0;
    gl_Position = vec4(clip_space.x, -clip_space.y, 0.0, 1.0);
}
