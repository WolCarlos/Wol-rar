fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=advapi32");
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("../assets/wol-rar.ico");
        resource.set("CompanyName", "WolCarlos");
        resource.set("FileDescription", "Wol-rar - Archiviatore privato in Rust");
        resource.set("FileVersion", "0.4.0");
        resource.set("InternalName", "Wol-rar");
        resource.set("LegalCopyright", "WolCarlos");
        resource.set("OriginalFilename", "wol-rar.exe");
        resource.set("ProductName", "Wol-rar");
        resource.set("ProductVersion", "0.4.0");
        resource.compile().expect("impossibile compilare le risorse Windows");
    }
}
