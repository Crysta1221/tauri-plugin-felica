use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

mod cards;
mod commands;
mod desktop;
mod error;
pub mod hardware;
pub mod models;
pub mod protocol;

pub use error::{Error, FelicaErrorCode, Result};

use desktop::Felica;
pub use desktop::FelicaManager;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the felica APIs.
pub trait FelicaExt<R: Runtime> {
    fn felica(&self) -> &Felica<R>;
}

impl<R: Runtime, T: Manager<R>> crate::FelicaExt<R> for T {
    fn felica(&self) -> &Felica<R> {
        self.state::<Felica<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("felica")
        .invoke_handler(tauri::generate_handler![
            commands::list_readers::list_readers,
            commands::connect_reader::connect_reader,
            commands::scan_card::scan_card,
            commands::cancel_scan::cancel_scan,
            commands::disconnect_reader::disconnect_reader,
            commands::read_blocks::read_blocks,
            commands::poll_card::poll_card,
            commands::release_card::release_card,
            commands::request_service::request_service,
            commands::search_services::search_services,
            commands::select_system::select_system,
        ])
        .setup(|app, api| {
            let felica = desktop::init(app, api)?;
            app.manage(felica);
            Ok(())
        })
        .build()
}
