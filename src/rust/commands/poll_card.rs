use tauri::{command, AppHandle, Runtime};

use crate::error::Result;
use crate::models::{PollCardRequest, PollCardResponse};
use crate::FelicaExt;

#[command]
pub(crate) async fn poll_card<R: Runtime>(
    app: AppHandle<R>,
    payload: PollCardRequest,
) -> Result<PollCardResponse> {
    let felica = app.felica();
    let manager = felica.manager_arc();
    tauri::async_runtime::spawn_blocking(move || manager.poll_card(payload))
        .await
        .map_err(|e| crate::Error::CommunicationError(e.to_string()))?
}
