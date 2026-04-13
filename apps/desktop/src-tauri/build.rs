fn main() {
    #[cfg(windows)]
    {
        // embed_resource::compile("resources.rc", embed_resource::NONE);
    }
    tauri_build::build()
}
