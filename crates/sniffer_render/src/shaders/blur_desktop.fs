#version 330 core
in vec2 v_uv;
out vec4 fragColor;

uniform sampler2D u_texture;
uniform vec2 u_offset;

void main() {
    vec4 col = texture(u_texture, v_uv + vec2(-u_offset.x, -u_offset.y));
    col += texture(u_texture, v_uv + vec2(u_offset.x, -u_offset.y));
    col += texture(u_texture, v_uv + vec2(-u_offset.x, u_offset.y));
    col += texture(u_texture, v_uv + vec2(u_offset.x, u_offset.y));
    fragColor = col * 0.25;
}
