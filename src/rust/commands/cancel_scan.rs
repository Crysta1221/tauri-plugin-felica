use tauri::{AppHandle, Runtime, command};
use crate::error::Result;
use crate::models::scan::CancelScanRequest;
use crate::FelicaExt;

#[command]
pub(crate) async fn cancel_scan<R: Runtime>(
    app: AppHandle<R>,
    payload: CancelScanRequest,
) -> Result<()> {
    app.felica().manager().cancel_scan(&payload.session_id)
}
