/// Embed the shared PicPatcher Windows icon into `bin_name`.
///
/// `winres` correctly produces a `resource.o`, but on GNU targets that object
/// can be discarded when wrapped inside the static archive emitted by build
/// scripts. Linking `resource.o` directly keeps RT_ICON/RT_GROUP_ICON in the
/// final executable, which is required for Explorer file icons.
pub fn embed_icon_for_bin(bin_name: &str) {
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") {
        let _ = bin_name;
        return;
    }

    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let icon = manifest_dir.join("../../assets/icon.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    if !icon.exists() {
        panic!("missing Windows icon: {}", icon.display());
    }

    let mut res = winres::WindowsResource::new();
    res.set_icon(icon.to_str().expect("icon path must be valid UTF-8"));
    if target.contains("windows-gnu") && !cfg!(windows) {
        res.set_windres_path("x86_64-w64-mingw32-windres");
        res.set_ar_path("x86_64-w64-mingw32-ar");
    }
    res.compile().expect("failed to compile Windows resources");

    if target.contains("windows-gnu") {
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let resource_obj = out_dir.join("resource.o");
        println!(
            "cargo:rustc-link-arg-bin={bin_name}={}",
            resource_obj.display()
        );
    }
}
