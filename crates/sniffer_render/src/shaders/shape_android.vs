#version 300 es
precision mediump float;
layout(location = 0) in vec2 position;
layout(location = 1) in vec4 a_bounds;       // rect_pos.xy, rect_size.zw
layout(location = 2) in vec4 a_color;        // top / fill color
layout(location = 3) in vec4 a_border_color; // border color
layout(location = 4) in vec4 a_color_bottom; // bottom gradient color
layout(location = 5) in vec4 a_shape_info;   // shape_size.xy, is_circle, is_shadow
layout(location = 6) in vec4 a_params;       // radius, border_width, shadow_blur, is_gradient
layout(location = 7) in vec4 a_border_color_bottom; // bottom border color

out vec2 v_local_pos;
out vec4 v_color;
out vec4 v_border_color;
out vec4 v_border_color_bottom;
out vec4 v_color_bottom;
out vec2 v_rect_size;
out vec2 v_shape_size;
out float v_radius;
out float v_border_width;
out float v_is_circle;
out float v_is_shadow;
out float v_shadow_blur;
out float v_is_gradient;
out vec2 v_screen_pos;

uniform vec2 u_resolution;
uniform mat3 u_transform;

// Backward-compatible fallback uniforms (used when u_instanced == 0)
uniform int u_instanced;
uniform vec2 u_rect_pos;
uniform vec2 u_rect_size;
uniform vec2 u_shape_size;
uniform vec4 u_color;
uniform float u_radius;
uniform float u_border_width;
uniform vec4 u_border_color;
uniform vec4 u_border_color_bottom;
uniform float u_is_circle;
uniform float u_is_shadow;
uniform float u_shadow_blur;
uniform float u_is_gradient;
uniform vec4 u_color_bottom;

void main() {
    vec2 rect_pos = (u_instanced == 1) ? a_bounds.xy : u_rect_pos;
    vec2 rect_size = (u_instanced == 1) ? a_bounds.zw : u_rect_size;
    
    vec3 pixel_pos_3 = u_transform * vec3(position * rect_size + rect_pos, 1.0);
    vec2 pixel_pos = pixel_pos_3.xy;
    v_screen_pos = pixel_pos;
    v_local_pos = position * rect_size;
    
    v_color = (u_instanced == 1) ? a_color : u_color;
    v_border_color = (u_instanced == 1) ? a_border_color : u_border_color;
    v_border_color_bottom = (u_instanced == 1) ? a_border_color_bottom : u_border_color_bottom;
    v_color_bottom = (u_instanced == 1) ? a_color_bottom : u_color_bottom;
    v_rect_size = rect_size;
    v_shape_size = (u_instanced == 1) ? a_shape_info.xy : u_shape_size;
    v_is_circle = (u_instanced == 1) ? a_shape_info.z : u_is_circle;
    v_is_shadow = (u_instanced == 1) ? a_shape_info.w : u_is_shadow;
    v_radius = (u_instanced == 1) ? a_params.x : u_radius;
    v_border_width = (u_instanced == 1) ? a_params.y : u_border_width;
    v_shadow_blur = (u_instanced == 1) ? a_params.z : u_shadow_blur;
    v_is_gradient = (u_instanced == 1) ? a_params.w : u_is_gradient;
    
    vec2 zero_to_one = pixel_pos / u_resolution;
    vec2 zero_to_two = zero_to_one * 2.0;
    vec2 clip_space = zero_to_two - 1.0;
    gl_Position = vec4(clip_space.x, -clip_space.y, 0.0, 1.0);
}
