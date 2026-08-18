#version 300 es
precision mediump float;
in vec2 position;
out vec2 v_uv;

uniform vec2 u_resolution;
uniform vec2 u_rect_pos;
uniform vec2 u_rect_size;
uniform vec2 u_uv_start;
uniform vec2 u_uv_end;
uniform mat3 u_transform;

void main() {
    vec3 pixel_pos_3 = u_transform * vec3(position * u_rect_size + u_rect_pos, 1.0);
    vec2 pixel_pos = pixel_pos_3.xy;
    
    float u = u_uv_start.x + position.x * (u_uv_end.x - u_uv_start.x);
    float v = u_uv_start.y + position.y * (u_uv_end.y - u_uv_start.y);
    v_uv = vec2(u, v);

    vec2 zero_to_one = pixel_pos / u_resolution;
    vec2 zero_to_two = zero_to_one * 2.0;
    vec2 clip_space = zero_to_two - 1.0;
    gl_Position = vec4(clip_space.x, -clip_space.y, 0.0, 1.0);
}
