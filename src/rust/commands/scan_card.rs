use tauri::{AppHandle, Runtime, command};
use crate::error::Result;
use crate::models::scan::{ScanCardRequest, ScanCardResponse};
use crate::FelicaExt;

#[command]
pub(crate) async fn scan_card<R: Runtime>(
    app: AppHandle<R>,
    payload: ScanCardRequest,
) -> Result<ScanCardResponse> {
    let felica = app.felica();
    let manager = felica.manager_arc();

    tauri::async_runtime::spawn_blocking(move || manager.scan_card(payload))
        .await
        .map_err(|e| crate::Error::CommunicationError(e.to_string()))?
}
