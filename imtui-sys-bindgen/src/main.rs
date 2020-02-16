extern crate imtui_sys_bindgen;

use imtui_sys_bindgen::generate_bindings;
use std::env;

fn main() {
    let cwd = env::current_dir().expect("Failed to read current directory");
    let imtui_path = cwd
        .join("../imtui-sys/third-party/imtui")
        .canonicalize()
        .expect("Failed to find imtui");
    let imgui_path = cwd
        .join("include")
        .canonicalize()
        .expect("Failed to find local include path");
    let bindings = generate_bindings(imtui_path.as_path(), imgui_path.as_path())
        .expect("Failed to generate bindings");
    let output_path = cwd.join("../imtui-sys/src/bindings.rs");
    bindings
        .write_to_file(&output_path)
        .expect("Failed to write bindings");
    println!("Wrote bindings to {}", output_path.to_string_lossy());
}
