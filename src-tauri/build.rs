fn main() {
    let windows_gnu = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu");
    if windows_gnu {
        embed_windows_test_manifest();
        let attributes = tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        tauri_build::try_build(attributes).expect("failed to prepare the Tauri build");
    } else {
        tauri_build::build();
    }
}

fn embed_windows_test_manifest() {
    let output = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").expect("Cargo did not provide OUT_DIR"),
    )
    .join("loopcode-test-manifest.o");
    let status = std::process::Command::new("windres")
        .current_dir("windows")
        .args(["--input-format=rc", "--output-format=coff"])
        .arg("test-manifest.rc")
        .arg(&output)
        .status()
        .expect("failed to invoke windres for the Windows test manifest");
    assert!(
        status.success(),
        "windres could not compile the Windows test manifest"
    );

    println!("cargo:rerun-if-changed=windows/test-manifest.rc");
    println!("cargo:rerun-if-changed=windows/test-manifest.xml");
    println!("cargo:rustc-link-arg={}", output.display());
}
