fn main() {
    slint_build::compile("ui/app.slint").unwrap();
    println!("cargo:rerun-if-changed=ui/app.slint");

    let icon = std::path::Path::new("../../src/Diktator/Assets/app.ico");
    if icon.exists() {
        let mut res = winresource::WindowsResource::new();
        res.set_icon(icon.to_str().unwrap());
        let _ = res.compile();
    }
}
