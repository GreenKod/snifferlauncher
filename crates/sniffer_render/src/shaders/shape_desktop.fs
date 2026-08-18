#version 330 core
in vec2 v_local_pos;
out vec4 FragColor;

uniform vec2 u_rect_size;
uniform vec2 u_shape_size;
uniform vec4 u_color;
uniform float u_radius;
uniform float u_border_width;
uniform vec4 u_border_color;
uniform float u_is_circle;
uniform float u_is_shadow;
uniform float u_shadow_blur;
uniform float u_is_gradient;
uniform vec4 u_color_bottom;

float sdRoundedBox(in vec2 p, in vec2 b, in float r) {
    vec2 q = abs(p) - b + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

void main() {
    float dist = 0.0;
    if (u_is_circle > 0.5) {
        float radius = u_shape_size.x * 0.5;
        vec2 center = u_rect_size * 0.5;
        dist = length(v_local_pos - center) - radius;
    } else {
        vec2 half_size = u_rect_size * 0.5;
        vec2 shape_half = u_shape_size * 0.5;
        vec2 p = v_local_pos - half_size;
        dist = sdRoundedBox(p, shape_half, u_radius);
    }

    if (u_is_shadow > 0.5) {
        float alpha = 1.0 - smoothstep(-u_shadow_blur, u_shadow_blur, dist);
        FragColor = vec4(u_color.rgb, u_color.a * alpha);
    } else {
        float alpha = 1.0 - smoothstep(-1.0, 1.0, dist);
        vec4 col = u_color;
        if (u_is_gradient > 0.5) {
            // v_local_pos.y goes from 0 to u_rect_size.y
            float gradient_factor = v_local_pos.y / max(u_rect_size.y, 1.0);
            col = mix(u_color, u_color_bottom, gradient_factor);
        }
        if (u_border_width > 0.0) {
            float border_dist = dist + u_border_width;
            float border_alpha = smoothstep(-1.0, 1.0, border_dist);
            col = mix(col, u_border_color, border_alpha);
        }
        FragColor = vec4(col.rgb, col.a * alpha);
    }
}
