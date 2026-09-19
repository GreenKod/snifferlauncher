#version 300 es
precision mediump float;
in vec2 v_screen_pos;
in vec2 v_local_pos;
in vec2 v_blur_uv;
out vec4 color;

uniform sampler2D u_blur_texture;
uniform float u_radius;
uniform vec2 u_rect_size;
uniform vec4 u_tint_color;
uniform float u_global_alpha;
uniform vec4 u_clip_rect;
uniform float u_clip_radius;
uniform mat3 u_clip_inv_transform;

float roundedBoxSDF(vec2 CenterPosition, vec2 Size, float Radius) {
    return length(max(abs(CenterPosition) - Size + Radius, 0.0)) - Radius;
}

void main() {
    float alpha = 1.0;
    if (u_radius > 0.0) {
        vec2 center = u_rect_size * 0.5;
        vec2 half_size = u_rect_size * 0.5;
        float d = roundedBoxSDF(v_local_pos - center, half_size, u_radius);
        alpha = 1.0 - smoothstep(-1.0, 0.5, d);
        if (alpha <= 0.0) {
            discard;
        }
    }

    vec4 blurred = texture(u_blur_texture, v_blur_uv);
    vec3 mixed_rgb = mix(blurred.rgb, u_tint_color.rgb, u_tint_color.a);
    float final_a = (blurred.a + u_tint_color.a * (1.0 - blurred.a)) * alpha * u_global_alpha;
    color = vec4(mixed_rgb, final_a);

    if (u_clip_radius >= 0.0) {
        vec2 clip_p = (u_clip_inv_transform * vec3(v_screen_pos, 1.0)).xy;
        vec2 clip_center = u_clip_rect.xy + u_clip_rect.zw * 0.5;
        vec2 clip_half = u_clip_rect.zw * 0.5;
        float clip_dist = roundedBoxSDF(clip_p - clip_center, clip_half, u_clip_radius);
        float clip_alpha = 1.0 - smoothstep(-0.5, 0.5, clip_dist);
        if (clip_alpha <= 0.0) {
            discard;
        }
        color = vec4(color.rgb, color.a * clip_alpha);
    }
}
