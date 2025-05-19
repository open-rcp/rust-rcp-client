use eframe::egui;
use log::info;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use crate::ui::events::AppEvent;
use crate::ui::history::{load_connection_history, save_connection_history};
use crate::ui::models::ConnectionEntry;

/// Draw the server configuration panel
pub fn draw_server_panel(
    ui: &mut egui::Ui,
    server_address: &mut String,
    server_port: &mut String,
    use_tls: &mut bool,
    event_tx: &mpsc::Sender<AppEvent>,
    rt_handle: &Handle,
    connection_history: &[ConnectionEntry],
) {
    egui::CollapsingHeader::new("Server Configuration")
        .default_open(true)
        .show(ui, |ui| {
            // Connection history dropdown
            if !connection_history.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Recent:");
                    egui::ComboBox::from_label("")
                        .selected_text("Select a recent connection")
                        .show_ui(ui, |ui| {
                            for entry in connection_history {
                                let display_text = entry.display_string();
                                let status_icon = if entry.successful {
                                    "✓ " // Checkmark for successful connections
                                } else {
                                    "⚠ " // Warning for failed connections
                                };
                                let full_text = format!("{}{}", status_icon, display_text);
                                
                                if ui.selectable_label(false, full_text).clicked() {
                                    *server_address = entry.address.clone();
                                    *server_port = entry.port.clone();
                                }
                            }
                        });
                        
                    if ui.button("🗑").on_hover_text("Clear connection history").clicked() {
                        let mut history = connection_history.to_vec();
                        history.clear();
                        save_connection_history(&history);
                    }
                });
                ui.add_space(5.0);
            }
            
            // Server address with tooltip and validation
            ui.horizontal(|ui| {
                ui.label("Address:");
                let response = ui.text_edit_singleline(server_address);
                
                // Use methods directly on the response, but only call each method once
                ui.label("").on_hover_text("Enter server hostname or IP address (Tab to navigate between fields)");
                let changed = response.changed();
                let lost_focus = response.lost_focus();
                
                // Validate address and trigger async validation if needed
                if !server_address.is_empty() {
                    let valid_address = !server_address.contains(' ');
                    if valid_address {
                        ui.colored_label(egui::Color32::GREEN, "✓");
                        static mut LAST_VALIDATED: Option<String> = None;
                        unsafe {
                            if changed {
                                if LAST_VALIDATED.as_deref() != Some(server_address) {
                                    let tx = event_tx.clone();
                                    let address = server_address.clone();
                                    LAST_VALIDATED = Some(address.clone());
                                    rt_handle.spawn(async move {
                                        let _ = tx.send(AppEvent::ValidateInput("server_address".to_string())).await;
                                    });
                                }
                            }
                        }
                    } else {
                        ui.colored_label(egui::Color32::RED, "⚠");
                        ui.label("Invalid server address");
                    }
                }
                // Allow Enter to advance to next field
                if lost_focus && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    ui.memory_mut(|mem| mem.request_focus(ui.next_auto_id()));
                }
            });
            
            // Port with validation
            ui.horizontal(|ui| {
                ui.label("Port:");
                let port_edit = ui.text_edit_singleline(server_port);
                ui.label("").on_hover_text("Enter server port (usually 8717)");
                
                // Validate port
                if !server_port.is_empty() {
                    match server_port.parse::<u16>() {
                        Ok(port) => {
                            if port > 0 {
                                ui.colored_label(egui::Color32::GREEN, "✓")
                                    .on_hover_text(format!("Valid port: {}", port));
                            } else {
                                ui.colored_label(egui::Color32::RED, "⚠");
                                ui.label("Port must be greater than 0");
                            }
                        }
                        Err(_) => {
                            ui.colored_label(egui::Color32::RED, "⚠");
                            ui.label("Port must be a number between 1-65535");
                        }
                    }
                }
                
                // Allow Enter to advance to next field
                if port_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    // Move focus to the use TLS checkbox
                    ui.memory_mut(|mem| mem.request_focus(ui.next_auto_id()));
                }
            });
            
            // Option to use TLS
            ui.horizontal(|ui| {
                ui.label("Use TLS encryption");
                let checkbox = ui.checkbox(use_tls, "");
                ui.label("🔒").on_hover_text("Secure the connection with TLS encryption");
                
                // Add more detailed explanation based on state
                if *use_tls {
                    ui.label("(Connection will be encrypted)")
                        .on_hover_text("TLS provides secure, encrypted communication with the server");
                } else {
                    ui.label("(Connection will be unencrypted)")
                        .on_hover_text("Warning: Unencrypted connections may expose sensitive data");
                }
                
                // Allow keyboard navigation
                if checkbox.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    // Find the Username field in the Authentication section and focus it
                    ui.memory_mut(|mem| mem.request_focus(ui.next_auto_id()));
                }
            });
        });
}
