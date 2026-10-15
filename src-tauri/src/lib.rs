mod commands;
mod kline;
mod market;
mod models;
mod quotes;
mod state;
mod store;
mod ths;
mod ths_login;

use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 已有实例在跑：聚焦主窗口，避免第二实例抢 WebView2 数据目录出错
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            // 主窗口关闭 = 退出整个应用（防止登录窗口残留导致进程假死）
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                std::process::exit(0);
            }
        })
        .setup(|app| {
            let data_dir = app.path().app_data_dir().expect("无法获取应用数据目录");
            let state = Arc::new(state::AppState::new(data_dir.join("stocker.json")));
            app.manage(state.clone());
            state::spawn_quote_engine(app.handle().clone(), state.clone());
            state::spawn_auction_backfill(app.handle().clone(), state.clone());
            state::spawn_sync_scheduler(app.handle().clone(), state);
            // 清理上次运行残留的隐藏登录窗口（WebView2 会话恢复可能带出隐藏窗口）
            if let Some(w) = app.get_webview_window(ths_login::LOGIN_WINDOW_LABEL) {
                if !w.is_visible().unwrap_or(true) {
                    let _ = w.close();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::login_ths,
            commands::import_cookie,
            commands::logout_ths,
            commands::sync_now,
            commands::add_local_stock,
            commands::remove_local_stock,
            commands::get_kline,
            commands::set_settings,
            commands::search_stocks,
            ths_login::open_ths_login,
            ths_login::confirm_ths_login,
            ths_login::open_ths_forgot,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
