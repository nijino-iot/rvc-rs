#[tauri::command]
fn invoke(name: &str) -> String {
    format!("Hello, {}! You've been invokeed from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![invoke,])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
