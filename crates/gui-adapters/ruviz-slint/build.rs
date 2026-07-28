fn main() -> Result<(), slint_build::CompileError> {
    let config = slint_build::CompilerConfiguration::new()
        .as_library("Ruviz")
        .rust_module("slint_generated");
    slint_build::compile_with_config("ui/ruviz.slint", config)
}
