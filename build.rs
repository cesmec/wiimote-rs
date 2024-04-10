fn main() {
    println!("cargo:rerun-if-changed=src/cpp");

    if !cfg!(windows) {
        let mut cfg = cc::Build::new();
        cfg.cpp(true);
        cfg.std("c++20");
        cfg.file("src/cpp/wiimote_api.cpp");
        cfg.file("src/cpp/wiimote_linux.cpp");
        cfg.file("src/cpp/wiimote_scan_linux.cpp");
        println!("cargo:rustc-link-lib=bluetooth");
        cfg.compile("wiimote_api");
    }
}
