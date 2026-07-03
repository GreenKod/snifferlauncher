#version 300 es
precision mediump float;
in vec2 v_uv;
in vec2 v_pos;
out vec4 color;

uniform sampler2D u_texture;
uniform float u_radius;
uniform vec2 u_rect_pos;
uniform vec2 u_rect_size;
uniform float u_global_alpha;

float roundedBoxSDF(vec2 CenterPosition, vec2 Size, float Radius) {
    return length(max(abs(CenterPosition) - Size + Radius, 0.0)) - Radius;
}

void main() {
    if (u_radius > 0.0) {
        vec2 center = u_rect_pos + u_rect_size * 0.5;
        vec2 half_size = u_rect_size * 0.5;
        float d = roundedBoxSDF(v_pos - center, half_size, u_radius);
        
        float alpha = 1.0 - smoothstep(-1.0, 0.5, d);
        if (alpha <= 0.0) {
            discard;
        }
        vec4 tex_color = texture(u_texture, v_uv);
        color = vec4(tex_color.rgb, tex_color.a * alpha * u_global_alpha);
    } else {
        vec4 tex_color = texture(u_texture, v_uv);
        color = vec4(tex_color.rgb, tex_color.a * u_global_alpha);
    }
}
