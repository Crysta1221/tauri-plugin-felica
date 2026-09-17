const COMMANDS: &[&str] = &[
    "list_readers",
    "connect_reader",
    "scan_card",
    "cancel_scan",
    "disconnect_reader",
    "read_blocks",
    "poll_card",
    "release_card",
    "request_service",
    "search_services",
    "select_system",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
