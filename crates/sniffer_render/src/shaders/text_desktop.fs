#version 330 core
in vec2 v_uv;
in vec4 v_color;
flat in float v_is_color;
in vec2 v_screen_pos;
out vec4 FragColor;

uniform sampler2D u_font_texture;
uniform vec4 u_clip_rect;
uniform float u_clip_radius;
uniform mat3 u_clip_inv_transform;

float sdRoundedBox(in vec2 p, in vec2 b, in float r) {
    vec2 q = abs(p) - b + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

void main() {
    if (v_is_color > 0.5) {
        vec4 tex_color = texture(u_font_texture, v_uv);
        if (tex_color.a < 0.01) {
            discard;
        }
        FragColor = vec4(tex_color.rgb, tex_color.a * v_color.a);
    } else {
        float dist = texture(u_font_texture, v_uv).r;
        float smoothing = fwidth(dist);
        float alpha = smoothstep(0.5 - smoothing, 0.5 + smoothing, dist);
        
        if (alpha < 0.01) {
            discard;
        }
        FragColor = vec4(v_color.rgb, v_color.a * alpha);
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

