extern crate bindgen;
#[macro_use]
extern crate failure;

use bindgen::{Bindings, RustTarget};
use failure::Error;
use std::path::Path;

pub fn generate_bindings(imtui_path: &Path, imgui_include_path: &Path) -> Result<Bindings, Error> {
    let imtui_include_path = imtui_path.join("include");
    let bindings = bindgen::builder()
        //.rust_target(RustTarget::Stable_1_33)
        .header("src/wrapper.hpp")
        .clang_arg("-xc++")
        .clang_arg("-std=c++14")
        .clang_arg(format!("-I{}", imtui_include_path.to_str().expect("No path")))
        .clang_arg(format!("-I{}", imgui_include_path.to_str().expect("No path")))
        .clang_arg("-fkeep-inline-functions")
        .enable_cxx_namespaces()
        .blacklist_type("ImDrawData") // comes from imgui
        .whitelist_type("ImTui::TScreen")
        .whitelist_function("ImTui_.*")
        .opaque_type("std::*")
        .raw_line("extern crate imgui_sys;")
        .module_raw_line("root", "use imgui_sys::ImDrawData;")
        .generate_inline_functions(true)
        .generate()
        .expect("Unable to generate bindings");
    Ok(bindings)
}
