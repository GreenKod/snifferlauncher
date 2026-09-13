#version 330 core
layout(location = 0) in vec2 position;
layout(location = 1) in vec2 uv;
out vec2 v_uv;

uniform vec2 u_resolution;
uniform mat3 u_transform;

void main() {
    vec3 pixel_pos_3 = u_transform * vec3(position, 1.0);
    vec2 pixel_pos = pixel_pos_3.xy;
    v_uv = uv;

    vec2 zero_to_one = pixel_pos / u_resolution;
    vec2 zero_to_two = zero_to_one * 2.0;
    vec2 clip_space = zero_to_two - 1.0;
    gl_Position = vec4(clip_space.x, -clip_space.y, 0.0, 1.0);
}
