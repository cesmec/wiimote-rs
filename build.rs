fn main() {
    println!("cargo:rerun-if-changed=src/cpp");

    let mut cfg = cc::Build::new();
    cfg.cpp(true);
    cfg.std("c++20");
    cfg.file("src/cpp/wiimote_api.cpp");

    if cfg!(windows) {
        cfg.define("WIN32_LEAN_AND_MEAN", None);
        cfg.define("NOMINMAX", None);
        cfg.file("src/cpp/wiimote_win.cpp");
        cfg.file("src/cpp/wiimote_scan_win.cpp");
        println!("cargo:rustc-link-lib=hid");
        println!("cargo:rustc-link-lib=BluetoothApis");
        println!("cargo:rustc-link-lib=Cfgmgr32");
    } else {
        cfg.file("src/cpp/wiimote_linux.cpp");
        cfg.file("src/cpp/wiimote_scan_linux.cpp");
        println!("cargo:rustc-link-lib=bluetooth");
    }

    cfg.compile("wiimote_api");
}
