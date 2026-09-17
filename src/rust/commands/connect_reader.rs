use tauri::{AppHandle, Runtime, command};
use crate::error::Result;
use crate::models::reader::{ConnectReaderRequest, ConnectReaderResponse};
use crate::FelicaExt;

#[command]
pub(crate) async fn connect_reader<R: Runtime>(
    app: AppHandle<R>,
    payload: ConnectReaderRequest,
) -> Result<ConnectReaderResponse> {
    app.felica().manager().connect_reader(payload)
}
