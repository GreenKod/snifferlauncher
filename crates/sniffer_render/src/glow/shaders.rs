use glow::HasContext;

pub(crate) unsafe fn compile_program(
    gl: &glow::Context,
    vs_src: &str,
    fs_src: &str,
) -> Result<glow::Program, String> {
    unsafe {
        let vs = gl.create_shader(glow::VERTEX_SHADER)?;
        gl.shader_source(vs, vs_src);
        gl.compile_shader(vs);
        if !gl.get_shader_compile_status(vs) {
            return Err(format!("VS Compile Error: {}", gl.get_shader_info_log(vs)));
        }

        let fs = gl.create_shader(glow::FRAGMENT_SHADER)?;
        gl.shader_source(fs, fs_src);
        gl.compile_shader(fs);
        if !gl.get_shader_compile_status(fs) {
            return Err(format!("FS Compile Error: {}", gl.get_shader_info_log(fs)));
        }

        let program = gl.create_program()?;
        gl.attach_shader(program, vs);
        gl.attach_shader(program, fs);
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(format!(
                "Shader Link Error: {}",
                gl.get_program_info_log(program)
            ));
        }

        gl.delete_shader(vs);
        gl.delete_shader(fs);

        Ok(program)
    }
}
