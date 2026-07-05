#version 330 core
layout(location = 0) in vec2 position;
out vec2 v_uv;
out vec2 v_pos;

uniform vec2 u_resolution;
uniform vec2 u_rect_pos;
uniform vec2 u_rect_size;
uniform vec2 u_uv_scale;
uniform vec2 u_uv_offset;
uniform mat3 u_transform;

void main() {
    vec3 pixel_pos_3 = u_transform * vec3(position * u_rect_size + u_rect_pos, 1.0);
    vec2 pixel_pos = pixel_pos_3.xy;
    v_pos = pixel_pos;
    v_uv = position * u_uv_scale + u_uv_offset;
    
    vec2 zero_to_one = pixel_pos / u_resolution;
    vec2 zero_to_two = zero_to_one * 2.0;
    vec2 clip_space = zero_to_two - 1.0;
    gl_Position = vec4(clip_space.x, -clip_space.y, 0.0, 1.0);
}
