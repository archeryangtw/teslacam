mod commands;
mod db;
mod scanner;

use db::Database;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // 初始化資料庫
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("無法取得應用程式資料目錄");

            let database = Database::new(app_dir).expect("無法初始化資料庫");
            app.manage(database);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_directory,
            commands::get_events,
            commands::delete_event,
            commands::backup_event,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
