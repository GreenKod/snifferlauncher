#version 330 core
in vec2 v_local_pos;
in vec4 v_color;
in vec4 v_border_color;
in vec4 v_color_bottom;
in vec2 v_rect_size;
in vec2 v_shape_size;
in float v_radius;
in float v_border_width;
in float v_is_circle;
in float v_is_shadow;
in float v_shadow_blur;
in float v_is_gradient;
in vec2 v_screen_pos;

out vec4 FragColor;

uniform vec4 u_clip_rect;
uniform float u_clip_radius;
uniform mat3 u_clip_inv_transform;

float sdRoundedBox(in vec2 p, in vec2 b, in float r) {
    vec2 q = abs(p) - b + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

void main() {
    float dist = 0.0;
    if (v_is_circle > 0.5) {
        float radius = v_shape_size.x * 0.5;
        vec2 center = v_rect_size * 0.5;
        dist = length(v_local_pos - center) - radius;
    } else {
        vec2 half_size = v_rect_size * 0.5;
        vec2 shape_half = v_shape_size * 0.5;
        vec2 p = v_local_pos - half_size;
        dist = sdRoundedBox(p, shape_half, v_radius);
    }

    if (v_is_shadow > 0.5) {
        float alpha = 1.0 - smoothstep(-v_shadow_blur, v_shadow_blur, dist);
        FragColor = vec4(v_color.rgb, v_color.a * alpha);
    } else {
        float alpha = 1.0 - smoothstep(-1.0, 1.0, dist);
        vec4 col = v_color;
        if (v_is_gradient > 0.5) {
            float gradient_factor = v_local_pos.y / max(v_rect_size.y, 1.0);
            col = mix(v_color, v_color_bottom, gradient_factor);
        }
        if (v_border_width > 0.0) {
            float border_dist = dist + v_border_width;
            float border_alpha = smoothstep(-1.0, 1.0, border_dist);
            col = mix(col, v_border_color, border_alpha);
        }
        FragColor = vec4(col.rgb, col.a * alpha);
    }

    if (u_clip_radius >= 0.0) {
        vec2 clip_p = (u_clip_inv_transform * vec3(v_screen_pos, 1.0)).xy;
        vec2 clip_center = u_clip_rect.xy + u_clip_rect.zw * 0.5;
        vec2 clip_half = u_clip_rect.zw * 0.5;
        float clip_dist = sdRoundedBox(clip_p - clip_center, clip_half, u_clip_radius);
        float clip_alpha = 1.0 - smoothstep(-0.5, 0.5, clip_dist);
        if (clip_alpha <= 0.0) {
            discard;
        }
        FragColor = vec4(FragColor.rgb, FragColor.a * clip_alpha);
    }
}
