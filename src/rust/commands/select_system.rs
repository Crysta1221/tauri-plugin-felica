use tauri::{command, AppHandle, Runtime};

use crate::error::Result;
use crate::models::{SelectSystemRequest, SelectSystemResponse};
use crate::FelicaExt;

#[command]
pub(crate) async fn select_system<R: Runtime>(
    app: AppHandle<R>,
    payload: SelectSystemRequest,
) -> Result<SelectSystemResponse> {
    let felica = app.felica();
    let manager = felica.manager_arc();
    tauri::async_runtime::spawn_blocking(move || manager.select_system(payload))
        .await
        .map_err(|e| crate::Error::CommunicationError(e.to_string()))?
}
