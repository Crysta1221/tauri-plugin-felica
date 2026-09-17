use tauri::{AppHandle, Runtime, command};
use crate::error::Result;
use crate::models::reader::ReaderInfoDto;
use crate::FelicaExt;

#[command]
pub(crate) async fn list_readers<R: Runtime>(app: AppHandle<R>) -> Result<Vec<ReaderInfoDto>> {
    app.felica().manager().list_readers()
}
