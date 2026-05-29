use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let mut library = HashMap::new();
    library.insert("lucide".to_string(), PathBuf::from(lucide_slint::lib()));

    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(library);

    slint_build::compile_with_config("src/app.slint", config).expect("Slint build failed");
}
