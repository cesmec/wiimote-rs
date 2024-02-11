fn main() {
    println!("cargo:rerun-if-changed=src/cpp");

    let mut cfg = cc::Build::new();
    cfg.cpp(true);
    cfg.std("c++17");
    cfg.file("src/cpp/wiimote_api.cpp");

    if cfg!(windows) {
        cfg.define("WIN32_LEAN_AND_MEAN", None);
        cfg.file("src/cpp/wiimote_win.cpp");
        cfg.file("src/cpp/wiimote_scan_win.cpp");
        println!("cargo:rustc-link-lib=hid");
        println!("cargo:rustc-link-lib=BluetoothApis");
    }

    cfg.compile("wiimote_api");
}
