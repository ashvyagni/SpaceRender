// Embeds the application icon into Windows executables. No-op on other targets.
fn main() {
    println!("cargo:rerun-if-changed=../../packaging/icons/cosmogon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let icon = "../../packaging/icons/cosmogon.ico";
        if std::path::Path::new(icon).exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon(icon);
            res.set("ProductName", "Cosmogon");
            res.set("FileDescription", "Cosmogon — a living-universe simulator");
            if let Err(e) = res.compile() {
                println!("cargo:warning=could not embed icon: {e}");
            }
        }
    }
}
