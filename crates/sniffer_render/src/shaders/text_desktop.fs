#version 330 core
in vec2 v_uv;
in vec4 v_color;
flat in float v_is_color;
out vec4 FragColor;

uniform sampler2D u_font_texture;

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
}

