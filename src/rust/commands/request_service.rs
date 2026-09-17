use tauri::{command, AppHandle, Runtime};

use crate::error::Result;
use crate::models::{RequestServiceCommand, RequestServiceResponseDto};
use crate::FelicaExt;

#[command]
pub(crate) async fn request_service<R: Runtime>(
    app: AppHandle<R>,
    payload: RequestServiceCommand,
) -> Result<RequestServiceResponseDto> {
    let felica = app.felica();
    let manager = felica.manager_arc();
    tauri::async_runtime::spawn_blocking(move || manager.request_service(payload))
        .await
        .map_err(|e| crate::Error::CommunicationError(e.to_string()))?
}
