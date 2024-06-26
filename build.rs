#[cfg(target_os = "linux")]
fn main() {
    const HEADER_FILE: &str = "src/native/linux/bluetooth_linux.h";
    println!("cargo:rerun-if-changed={HEADER_FILE}");

    let bindings = bindgen::Builder::default()
        .header(HEADER_FILE)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Failed to generate bindings for libbluetooth");

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Failed to write bindings for libbluetooth");

    println!("cargo:rustc-link-lib=bluetooth");
}

#[cfg(not(target_os = "linux"))]
fn main() {}
