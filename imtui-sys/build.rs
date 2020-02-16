use std::io;

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");

    let mut build = cc::Build::new();
    let files = vec![
        "third-party/imtui/src/imtui-impl-text.cpp",
        "third-party/imtui/src/imtui-impl-ncurses.cpp"
    ];
    build
        .cpp(true)
        .include("third-party/imtui/include")
        .include("../imgui-rs/imgui-sys/third-party/cimgui")
        .flag("-std=c++17")
        .files(files.iter())
        .compile("libimtui.a");

    println!("cargo:rustc-link-lib=ncurses");
    Ok(())
}
