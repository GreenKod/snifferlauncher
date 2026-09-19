#version 330 core
in vec2 v_uv;
in vec2 v_local_pos;
in vec2 v_screen_pos;
in vec2 v_rect_size;
in float v_radius;
in float v_alpha;

out vec4 color;

uniform sampler2D u_texture;
uniform vec4 u_clip_rect;
uniform float u_clip_radius;
uniform mat3 u_clip_inv_transform;

// Backward-compatible uniform declarations
uniform float u_radius;
uniform vec2 u_rect_pos;
uniform vec2 u_rect_size;
uniform float u_global_alpha;

float roundedBoxSDF(vec2 CenterPosition, vec2 Size, float Radius) {
    return length(max(abs(CenterPosition) - Size + Radius, 0.0)) - Radius;
}

void main() {
    if (v_radius > 0.0) {
        vec2 center = v_rect_size * 0.5;
        vec2 half_size = v_rect_size * 0.5;
        float d = roundedBoxSDF(v_local_pos - center, half_size, v_radius);

        float alpha = 1.0 - smoothstep(-1.0, 0.5, d);
        if (alpha <= 0.0) {
            discard;
        }
        vec4 tex_color = texture(u_texture, v_uv);
        color = vec4(tex_color.rgb, tex_color.a * alpha * v_alpha);
    } else {
        vec4 tex_color = texture(u_texture, v_uv);
        color = vec4(tex_color.rgb, tex_color.a * v_alpha);
    }

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
