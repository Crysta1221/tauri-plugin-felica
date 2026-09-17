use tauri::{command, AppHandle, Runtime};

use crate::error::Result;
use crate::models::SessionOnlyRequest;
use crate::FelicaExt;

#[command]
pub(crate) async fn release_card<R: Runtime>(
    app: AppHandle<R>,
    payload: SessionOnlyRequest,
) -> Result<()> {
    app.felica().manager().release_card(&payload.session_id)
}
