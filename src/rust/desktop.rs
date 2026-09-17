use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::cards::{
    poll_card, read_services, request_service_codes, scan_card, search_services, select_system,
    ScanParams,
};
use crate::error::{Error, Result};
use crate::hardware::{list_pasori_devices, open_reader, ReaderDevice};
use crate::models::blocks::ReadBlocksResultDto;
use crate::models::low_level::{
    PollCardRequest, PollCardResponse, ReadBlocksRequest, RequestServiceCommand,
    RequestServiceResponseDto, SearchServicesRequest, SearchServicesResponse, SelectSystemRequest,
    SelectSystemResponse, ServiceStatusDto,
};
use crate::models::reader::{ConnectReaderRequest, ConnectReaderResponse, ReaderInfoDto};
use crate::models::scan::{ScanCardRequest, ScanCardResponse};

struct HeldCard {
    idm: [u8; 8],
    pmm: [u8; 8],
    current_system: Option<u16>,
    system_codes: Vec<u16>,
}

struct DeviceSession {
    device: Box<dyn ReaderDevice>,
    held: Option<HeldCard>,
}

#[derive(Clone)]
struct SessionHandles {
    inner: Arc<std::sync::Mutex<DeviceSession>>,
    cancel_flag: Arc<AtomicBool>,
    scanning: Arc<AtomicBool>,
}

/// Core state manager for FeliCa readers on desktop platforms.
pub struct FelicaManager {
    sessions: std::sync::Mutex<HashMap<String, SessionHandles>>,
}

impl FelicaManager {
    pub fn new() -> Self {
        Self {
            sessions: std::sync::Mutex::new(HashMap::new()),
        }
    }

    pub fn list_readers(&self) -> Result<Vec<ReaderInfoDto>> {
        list_pasori_devices()
    }

    pub fn connect_reader(&self, req: ConnectReaderRequest) -> Result<ConnectReaderResponse> {
        let device = open_reader(req.id.as_deref(), req.preference)?;
        let info = device.info().clone();
        let session_id = format!(
            "session-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        let handles = SessionHandles {
            inner: Arc::new(std::sync::Mutex::new(DeviceSession {
                device,
                held: None,
            })),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            scanning: Arc::new(AtomicBool::new(false)),
        };

        let mut map = self.sessions.lock().unwrap();
        map.insert(session_id.clone(), handles);

        Ok(ConnectReaderResponse {
            session_id,
            reader: info,
        })
    }

    pub fn cancel_scan(&self, session_id: &str) -> Result<()> {
        let map = self.sessions.lock().unwrap();
        if let Some(handles) = map.get(session_id) {
            handles.cancel_flag.store(true, Ordering::SeqCst);
            Ok(())
        } else {
            Err(Error::SessionClosed(session_id.to_string()))
        }
    }

    pub fn disconnect_reader(&self, session_id: &str) -> Result<()> {
        let handles = {
            let mut map = self.sessions.lock().unwrap();
            map.remove(session_id)
        };

        if let Some(handles) = handles {
            handles.cancel_flag.store(true, Ordering::SeqCst);
            if let Ok(mut session) = handles.inner.lock() {
                session.held = None;
                let _ = session.device.close();
            }
            Ok(())
        } else {
            Err(Error::SessionClosed(session_id.to_string()))
        }
    }

    fn handles(&self, session_id: &str) -> Result<SessionHandles> {
        let map = self.sessions.lock().unwrap();
        map.get(session_id)
            .cloned()
            .ok_or_else(|| Error::SessionClosed(session_id.to_string()))
    }

    fn begin_exclusive(&self, session_id: &str) -> Result<SessionHandles> {
        let handles = self.handles(session_id)?;
        if handles
            .scanning
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(Error::DeviceBusy);
        }
        {
            let session = handles.inner.lock().map_err(|_| Error::DeviceBusy)?;
            if session.held.is_some() {
                handles.scanning.store(false, Ordering::SeqCst);
                return Err(Error::DeviceBusy);
            }
        }
        handles.cancel_flag.store(false, Ordering::SeqCst);
        Ok(handles)
    }

    pub fn scan_card(&self, req: ScanCardRequest) -> Result<ScanCardResponse> {
        let handles = self.begin_exclusive(&req.session_id)?;
        let result = (|| {
            let mut session = handles.inner.try_lock().map_err(|_| Error::DeviceBusy)?;
            let timeout = req.timeout_ms.filter(|&ms| ms > 0).map(Duration::from_millis);
            scan_card(
                &mut *session.device,
                ScanParams {
                    timeout,
                    targets: req.targets,
                    require: req.require,
                    detail: req.detail,
                    cancel: &handles.cancel_flag,
                },
            )
        })();
        handles.scanning.store(false, Ordering::SeqCst);
        match result {
            Err(Error::DeviceDisconnected) => {
                let _ = self.disconnect_reader(&req.session_id);
                Err(Error::DeviceDisconnected)
            }
            other => other,
        }
    }

    pub fn read_blocks(&self, req: ReadBlocksRequest) -> Result<ReadBlocksResultDto> {
        let handles = self.handles(&req.session_id)?;
        if handles.scanning.load(Ordering::SeqCst) {
            return Err(Error::DeviceBusy);
        }
        let mut session = handles.inner.try_lock().map_err(|_| Error::DeviceBusy)?;
        if session.held.is_some() {
            return read_services(&mut *session.device, &req.services);
        }
        drop(session);
        let handles = self.begin_exclusive(&req.session_id)?;
        let result = (|| {
            let mut session = handles.inner.try_lock().map_err(|_| Error::DeviceBusy)?;
            read_services(&mut *session.device, &req.services)
        })();
        handles.scanning.store(false, Ordering::SeqCst);
        result
    }

    pub fn poll_card(&self, req: PollCardRequest) -> Result<PollCardResponse> {
        let handles = self.begin_exclusive(&req.session_id)?;
        let result = (|| {
            let mut session = handles.inner.try_lock().map_err(|_| Error::DeviceBusy)?;
            let timeout = req.timeout_ms.filter(|&ms| ms > 0).map(Duration::from_millis);
            let parsed = poll_card(
                &mut *session.device,
                req.system_code,
                timeout,
                &handles.cancel_flag,
            )?;
            let system_codes =
                crate::protocol::command::build_request_system_code(&parsed.idm);
            let system_codes = match session.device.transceive(&system_codes, 100) {
                Ok(resp) => crate::protocol::response::parse_system_codes_response_for(
                    &resp,
                    Some(&parsed.idm),
                )
                .unwrap_or_default(),
                Err(_) => parsed.system_code.into_iter().collect(),
            };
            session.held = Some(HeldCard {
                idm: parsed.idm,
                pmm: parsed.pmm,
                current_system: req.system_code.or(parsed.system_code),
                system_codes: system_codes.clone(),
            });
            Ok(PollCardResponse {
                idm: hex::encode_upper(parsed.idm),
                pmm: hex::encode_upper(parsed.pmm),
                system_codes,
            })
        })();
        handles.scanning.store(false, Ordering::SeqCst);
        result
    }

    pub fn release_card(&self, session_id: &str) -> Result<()> {
        let handles = self.handles(session_id)?;
        let mut session = handles.inner.lock().map_err(|_| Error::DeviceBusy)?;
        session.held = None;
        Ok(())
    }

    pub fn request_service(&self, req: RequestServiceCommand) -> Result<RequestServiceResponseDto> {
        let handles = self.require_idle(&req.session_id)?;
        let mut session = handles.inner.lock().map_err(|_| Error::DeviceBusy)?;
        let held = Self::held_of(&session, &req.session_id)?;
        let pairs = request_service_codes(&mut *session.device, &held.idm, &req.service_codes)?;
        Ok(RequestServiceResponseDto {
            results: pairs
                .into_iter()
                .map(|(service_code, key_version)| ServiceStatusDto {
                    service_code,
                    key_version,
                    exists: key_version != 0xFFFF,
                })
                .collect(),
        })
    }

    pub fn search_services(&self, req: SearchServicesRequest) -> Result<SearchServicesResponse> {
        let handles = self.require_idle(&req.session_id)?;
        let mut session = handles.inner.lock().map_err(|_| Error::DeviceBusy)?;
        let held = Self::held_of(&session, &req.session_id)?;
        let nodes = search_services(
            &mut *session.device,
            &held.idm,
            req.start_index.unwrap_or(0),
            req.max_nodes.unwrap_or(32),
        )?;
        Ok(SearchServicesResponse { nodes })
    }

    pub fn select_system(&self, req: SelectSystemRequest) -> Result<SelectSystemResponse> {
        let handles = self.require_idle(&req.session_id)?;
        let mut session = handles.inner.lock().map_err(|_| Error::DeviceBusy)?;
        let held = Self::held_of(&session, &req.session_id)?;
        let parsed = select_system(&mut *session.device, req.system_code)?;
        session.held = Some(HeldCard {
            idm: parsed.idm,
            pmm: parsed.pmm,
            current_system: Some(req.system_code),
            system_codes: held.system_codes,
        });
        Ok(SelectSystemResponse {
            idm: hex::encode_upper(parsed.idm),
            pmm: hex::encode_upper(parsed.pmm),
        })
    }

    fn require_idle(&self, session_id: &str) -> Result<SessionHandles> {
        let handles = self.handles(session_id)?;
        if handles.scanning.load(Ordering::SeqCst) {
            return Err(Error::DeviceBusy);
        }
        Ok(handles)
    }

    fn held_of(session: &DeviceSession, session_id: &str) -> Result<HeldCard> {
        session
            .held
            .clone()
            .ok_or_else(|| Error::SessionClosed(session_id.to_string()))
    }
}

impl Clone for HeldCard {
    fn clone(&self) -> Self {
        Self {
            idm: self.idm,
            pmm: self.pmm,
            current_system: self.current_system,
            system_codes: self.system_codes.clone(),
        }
    }
}

/// Access to the felica plugin desktop APIs.
pub struct Felica<R: Runtime> {
    app: AppHandle<R>,
    pub manager: Arc<FelicaManager>,
}

impl<R: Runtime> Felica<R> {
    pub fn manager(&self) -> &FelicaManager {
        &self.manager
    }

    pub fn manager_arc(&self) -> Arc<FelicaManager> {
        Arc::clone(&self.manager)
    }

    pub fn app(&self) -> &AppHandle<R> {
        &self.app
    }
}

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<Felica<R>> {
    Ok(Felica {
        app: app.clone(),
        manager: Arc::new(FelicaManager::new()),
    })
}
