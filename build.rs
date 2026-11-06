// Embeds icon.ico into folio.exe so Windows Explorer, the taskbar and
// shortcuts show the app icon. Does nothing on Linux/macOS.
fn main() {
    println!("cargo:rerun-if-changed=icon.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icon.ico");
        res.set("ProductName", "Folio");
        res.set("FileDescription", "Folio — e-book reader");
        res.compile().expect("failed to embed Windows icon");
    }
}
