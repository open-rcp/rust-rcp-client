use crate::ui::models::ConnectionEntry;
use log::error;
use std::fs;
use std::path::PathBuf;

/// Load connection history from disk
pub fn load_connection_history() -> Vec<ConnectionEntry> {
    // Get history file path
    let history_path = get_history_file_path();

    // If file doesn't exist, return empty vector
    if !history_path.exists() {
        return Vec::new();
    }

    // Attempt to read and deserialize history file
    match fs::read_to_string(&history_path) {
        Ok(content) => match serde_json::from_str::<Vec<ConnectionEntry>>(&content) {
            Ok(history) => history,
            Err(e) => {
                error!("Failed to parse connection history: {}", e);
                Vec::new()
            }
        },
        Err(e) => {
            error!("Failed to read connection history: {}", e);
            Vec::new()
        }
    }
}

/// Save connection history to config
pub fn save_connection_history(history: &[ConnectionEntry]) {
    // Get history file path
    let history_path = get_history_file_path();

    // Ensure parent directory exists
    if let Some(parent) = history_path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                error!("Failed to create history directory: {}", e);
                return;
            }
        }
    }

    // Serialize and save history
    match serde_json::to_string_pretty(history) {
        Ok(content) => {
            if let Err(e) = fs::write(&history_path, content) {
                error!("Failed to write connection history: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to serialize connection history: {}", e);
        }
    }
}

/// Get connection history file path
pub fn get_history_file_path() -> PathBuf {
    dirs::config_dir()
        .expect("Could not find config directory")
        .join("rcp_client")
        .join("connection_history.json")
}

/// Add or update connection in history
pub fn add_to_connection_history(
    history: &mut Vec<ConnectionEntry>,
    address: &str,
    port: &str,
    username: Option<&str>,
    auth_method: &str,
    successful: bool,
) {
    // Look for an existing entry
    let mut found = false;
    for entry in history.iter_mut() {
        if entry.address == address && entry.port == port {
            // Update existing entry
            if let Some(uname) = username {
                entry.username = Some(uname.to_string());
            }
            entry.auth_method = auth_method.to_string();
            entry.last_connected = std::time::SystemTime::now();
            if successful {
                entry.mark_successful();
            }
            found = true;
            break;
        }
    }

    // Add new entry if not found
    if !found {
        let mut entry = ConnectionEntry::new(address, port, username, auth_method);
        if successful {
            entry.mark_successful();
        }
        history.push(entry);
    }

    // Sort by last connected time (most recent first)
    history.sort_by(|a, b| b.last_connected.cmp(&a.last_connected));

    // Limit history to 10 entries
    if history.len() > 10 {
        history.truncate(10);
    }

    // Save updated history
    save_connection_history(history);
}
