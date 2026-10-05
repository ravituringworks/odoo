//! Desktop shell: the Rust core runs in-process; the UI talks to it through one `rpc` command
//! (same `odoo_app::App::dispatch` the HTTP server uses), with SQLite stored in the OS app-data dir.
use odoo_app::{App, Config, Request};
use std::sync::Arc;
use tauri::Manager;

#[tauri::command]
async fn rpc(app: tauri::State<'_, Arc<App>>, req: Request) -> Result<serde_json::Value, String> {
    let a = app.inner().clone();
    tauri::async_runtime::spawn_blocking(move || a.dispatch(req)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let res = app.path().resource_dir()?;
            let cfg = Config {
                backend: "sqlite".into(), url: dir.join("odoo.sqlite").to_string_lossy().into(),
                modules: ["sale_management", "account", "stock", "purchase", "crm", "contacts", "hr", "project"].map(String::from).to_vec(),
                schema_dir: res.join("schema"), data_dir: res.join("data"), views_dir: res.join("views"), i18n_dir: res.join("i18n"), seed: true, require_auth: false, trial: None, is_trial: false,
            };
            app.manage(Arc::new(App::open(&cfg).map_err(|e| e.to_string())?));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![rpc])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
