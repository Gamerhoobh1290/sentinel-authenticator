fn main() {
    // tauri-build generates the capability schema and the icon set
    // referenced by `tauri.conf.json`. It must run before `cargo build`.
    tauri_build::build()
}
