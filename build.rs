use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out_path = PathBuf::from("src/native/linux/bindings.rs");

    let mut bindings_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(out_path)
        .unwrap();

    if cfg!(target_os = "linux") {
        const HEADER_FILE: &str = "src/native/linux/bluetooth_linux.h";
        println!("cargo:rerun-if-changed={HEADER_FILE}");

        let bindings = bindgen::Builder::default()
            .header(HEADER_FILE)
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .generate()
            .expect("Failed to generate bindings for libbluetooth");

        bindings_file.write_all(b"#![allow(warnings)]\n\n").unwrap();

        bindings
            .write(Box::new(bindings_file))
            .expect("Failed to write bindings for libbluetooth");

        println!("cargo:rustc-link-lib=bluetooth");
    }
}
