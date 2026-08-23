#[derive(Clone)]
pub struct ShapeUniforms {
    pub u_resolution: Option<glow::UniformLocation>,
    pub u_rect_pos: Option<glow::UniformLocation>,
    pub u_rect_size: Option<glow::UniformLocation>,
    pub u_color: Option<glow::UniformLocation>,
    pub u_radius: Option<glow::UniformLocation>,
    pub u_border_width: Option<glow::UniformLocation>,
    pub u_border_color: Option<glow::UniformLocation>,
    pub u_is_circle: Option<glow::UniformLocation>,
    pub u_is_shadow: Option<glow::UniformLocation>,
    pub u_shadow_blur: Option<glow::UniformLocation>,
    pub u_is_gradient: Option<glow::UniformLocation>,
    pub u_color_bottom: Option<glow::UniformLocation>,
    pub u_shape_size: Option<glow::UniformLocation>,
    pub u_transform: Option<glow::UniformLocation>,
}

#[derive(Clone)]
pub struct ImageUniforms {
    pub u_resolution: Option<glow::UniformLocation>,
    pub u_rect_pos: Option<glow::UniformLocation>,
    pub u_rect_size: Option<glow::UniformLocation>,
    pub u_uv_scale: Option<glow::UniformLocation>,
    pub u_uv_offset: Option<glow::UniformLocation>,
    pub u_radius: Option<glow::UniformLocation>,
    pub u_global_alpha: Option<glow::UniformLocation>,
    pub u_transform: Option<glow::UniformLocation>,
}

#[derive(Clone)]
pub struct TextUniforms {
    pub u_resolution: Option<glow::UniformLocation>,
    pub u_rect_pos: Option<glow::UniformLocation>,
    pub u_rect_size: Option<glow::UniformLocation>,
    pub u_color: Option<glow::UniformLocation>,
    pub u_uv_start: Option<glow::UniformLocation>,
    pub u_uv_end: Option<glow::UniformLocation>,
    pub u_transform: Option<glow::UniformLocation>,
    pub u_is_color: Option<glow::UniformLocation>,
}
