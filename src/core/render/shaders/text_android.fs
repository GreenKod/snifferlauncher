#version 300 es
precision mediump float;
in vec2 v_uv;
out vec4 FragColor;

uniform sampler2D u_font_texture;
uniform vec4 u_color;

void main() {
    float val = texture(u_font_texture, v_uv).r;
    if (val < 0.1) {
        discard;
    }
    FragColor = vec4(u_color.rgb, u_color.a * val);
}
