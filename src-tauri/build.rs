fn main() {
    let slint_config = slint_build::CompilerConfiguration::default().with_debug_info(true);
    slint_build::compile_with_config("ui/main.slint", slint_config)
        .expect("Slint compilation failed");
    tauri_build::build()
}
