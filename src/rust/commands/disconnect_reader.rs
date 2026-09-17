use tauri::{AppHandle, Runtime, command};
use crate::error::Result;
use crate::models::reader::DisconnectReaderRequest;
use crate::FelicaExt;

#[command]
pub(crate) async fn disconnect_reader<R: Runtime>(
    app: AppHandle<R>,
    payload: DisconnectReaderRequest,
) -> Result<()> {
    app.felica().manager().disconnect_reader(&payload.session_id)
}
