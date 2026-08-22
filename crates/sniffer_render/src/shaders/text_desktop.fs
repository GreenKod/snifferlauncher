#version 330 core
in vec2 v_uv;
out vec4 FragColor;

uniform sampler2D u_font_texture;
uniform vec4 u_color;
uniform int u_is_color;

void main() {
    if (u_is_color == 1) {
        vec4 tex_color = texture(u_font_texture, v_uv);
        if (tex_color.a < 0.01) {
            discard;
        }
        FragColor = vec4(tex_color.rgb, tex_color.a * u_color.a);
    } else {
        float dist = texture(u_font_texture, v_uv).r;
        float smoothing = fwidth(dist);
        float alpha = smoothstep(0.5 - smoothing, 0.5 + smoothing, dist);
        
        if (alpha < 0.01) {
            discard;
        }
        FragColor = vec4(u_color.rgb, u_color.a * alpha);
    }
}

