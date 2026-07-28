fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .as_library("Ruviz")
        .rust_module("slint_generated");
    slint_build::compile_with_config("ui/ruviz.slint", config)
        .expect("failed to compile the @Ruviz Slint component library");
}
