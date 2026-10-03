fn main() {
    for asset in [
        "index.html",
        "style.css",
        "theme.css",
        "theme.js",
        "boot.js",
        "app.js",
        "view.js",
        "tauri.js",
    ] {
        println!("cargo:rerun-if-changed=../target/frontend-dist/{asset}");
    }
    tauri_build::build()
}
