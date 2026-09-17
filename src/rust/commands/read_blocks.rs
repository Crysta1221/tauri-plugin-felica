use tauri::{command, AppHandle, Runtime};

use crate::error::Result;
use crate::models::{ReadBlocksRequest, ReadBlocksResultDto};
use crate::FelicaExt;

#[command]
pub(crate) async fn read_blocks<R: Runtime>(
    app: AppHandle<R>,
    payload: ReadBlocksRequest,
) -> Result<ReadBlocksResultDto> {
    let felica = app.felica();
    let manager = felica.manager_arc();
    tauri::async_runtime::spawn_blocking(move || manager.read_blocks(payload))
        .await
        .map_err(|e| crate::Error::CommunicationError(e.to_string()))?
}
