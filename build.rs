fn main() {
    println!("cargo:rerun-if-changed=src/hid.h");
    println!("cargo:rerun-if-changed=src/hid_win.cpp");

    let mut cfg = cc::Build::new();
    cfg.cpp(true);
    if cfg!(windows) {
        cfg.file("src/hid_win.cpp");
        cfg.cpp_link_stdlib("hid");
        cfg.cpp_link_stdlib("BluetoothApis");
    }

    cfg.compile("wiimote_hid");
}
