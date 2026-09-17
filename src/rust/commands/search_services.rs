use tauri::{command, AppHandle, Runtime};

use crate::error::Result;
use crate::models::{SearchServicesRequest, SearchServicesResponse};
use crate::FelicaExt;

#[command]
pub(crate) async fn search_services<R: Runtime>(
    app: AppHandle<R>,
    payload: SearchServicesRequest,
) -> Result<SearchServicesResponse> {
    let felica = app.felica();
    let manager = felica.manager_arc();
    tauri::async_runtime::spawn_blocking(move || manager.search_services(payload))
        .await
        .map_err(|e| crate::Error::CommunicationError(e.to_string()))?
}
