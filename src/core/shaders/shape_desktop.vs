#version 330 core
layout(location = 0) in vec2 position;
out vec2 v_local_pos;

uniform vec2 u_resolution;
uniform vec2 u_rect_pos;
uniform vec2 u_rect_size;

void main() {
    vec2 pixel_pos = position * u_rect_size + u_rect_pos;
    v_local_pos = position * u_rect_size;
    
    vec2 zero_to_one = pixel_pos / u_resolution;
    vec2 zero_to_two = zero_to_one * 2.0;
    vec2 clip_space = zero_to_two - 1.0;
    gl_Position = vec4(clip_space.x, -clip_space.y, 0.0, 1.0);
}
