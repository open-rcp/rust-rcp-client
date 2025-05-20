use crate::ui::events::AppEvent;
use crate::ui::models::AppState;
use eframe::egui;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

/// Draw the connection details panel (only shown when connected)
pub fn draw_connection_panel(
    ui: &mut egui::Ui,
    server_address: &str,
    server_port: &str,
    username: &str,
    auth_method: &str,
    use_tls: bool,
    event_tx: &mpsc::Sender<AppEvent>,
    rt_handle: &Handle,
    app_state: &Arc<Mutex<AppState>>,
) {
    // Calculate available width for responsive layou
    let available_width = ui.available_width();

    // Container with a frame for better visual appearance
    egui::Frame::group(ui.style())
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .stroke(ui.style().visuals.widgets.noninteractive.bg_stroke)
        .rounding(ui.style().visuals.widgets.noninteractive.rounding)
        .inner_margin(egui::vec2(10.0, 10.0))
        .show(ui, |ui| {
            // Connection header with status indicator
            ui.horizontal(|ui| {
                // Connection status with colored indicator
                let status_color = if use_tls {
                    egui::Color32::GREEN
                } else {
                    egui::Color32::GOLD
                };

                let status_circle = egui::RichText::new("●")
                    .color(status_color)
                    .size(16.0);

                ui.label(status_circle);

                // Header with connection info
                let header = egui::RichText::new("Connection Details")
                    .size(18.0)
                    .strong();
                ui.heading(header);

                // Add timestamp on the right if space permits
                if available_width > 400.0 {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Try to get connection time from app state (non-blocking)
                        let time_display = if let Ok(state) = app_state.try_lock() {
                            if let Some(connect_time) = state.connection_time {
                                if let Ok(duration) = connect_time.duration_since(UNIX_EPOCH) {
                                    let timestamp = duration.as_secs();
                                    chrono::DateTime::from_timestamp(timestamp as i64, 0)
                                        .map(|dt| dt.format("%H:%M:%S").to_string())
                                        .unwrap_or_else(|| "Unknown time".to_string())
                                } else {
                                    "Invalid time".to_string()
                                }
                            } else {
                                "Just now".to_string()
                            }
                        } else {
                            // Fallback if we can't get a lock
                            "Just connected".to_string()
                        };

                        ui.label(format!("Connected since: {}", time_display));
                    });
                }
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Connection details in a responsive grid layou
            egui::Grid::new("connection_details_grid")
                .num_columns(2)
                .spacing([available_width * 0.1, 8.0]) // Responsive spacing
                .min_col_width(available_width * 0.3) // Responsive column width
                .striped(true)
                .show(ui, |ui| {
                    // Server connection info with icon
                    ui.horizontal(|ui| {
                        ui.label("🖥️");
                        ui.label("Connected to:");
                    });
                    ui.horizontal(|ui| {
                        let server_text = egui::RichText::new(format!("{}:{}", server_address, server_port))
                            .strong()
                            .monospace();
                        ui.label(server_text);
                    });
                    ui.end_row();

                    // User info with icon
                    ui.horizontal(|ui| {
                        ui.label("👤");
                        ui.label("User:");
                    });
                    ui.horizontal(|ui| {
                        let user_text = egui::RichText::new(username).strong();
                        ui.label(user_text);
                    });
                    ui.end_row();

                    // Authentication method with icon
                    ui.horizontal(|ui| {
                        ui.label("🔑");
                        ui.label("Auth method:");
                    });
                    ui.horizontal(|ui| {
                        let auth_text = egui::RichText::new(auth_method).strong();
                        ui.label(auth_text);
                    });
                    ui.end_row();

                    // Encryption status with icon
                    ui.horizontal(|ui| {
                        if use_tls {
                            ui.label("🔒");
                        } else {
                            ui.label("⚠️");
                        }
                        ui.label("Encryption:");
                    });

                    if use_tls {
                        ui.horizontal(|ui| {
                            let secure_text = egui::RichText::new("TLS Encrypted")
                                .color(egui::Color32::GREEN)
                                .strong();
                            ui.label(secure_text);

                            let info_btn = ui.small_button("ℹ️");
                            if info_btn.clicked() {
                                // Could show more details in a future enhancemen
                            }
                            info_btn.on_hover_text("Your connection is secure with TLS encryption");
                        });
                    } else {
                        ui.horizontal(|ui| {
                            let warning_text = egui::RichText::new("Unencrypted")
                                .color(egui::Color32::GOLD)
                                .strong();
                            ui.label(warning_text);

                            let warning_btn = ui.small_button("⚠️");
                            if warning_btn.clicked() {
                                // Could show security warning in a future enhancemen
                            }
                            warning_btn.on_hover_text("Warning: Your connection is not encrypted. Your data may be vulnerable.");
                        });
                    }
                    ui.end_row();
                });

            ui.add_space(12.0);

            // Bottom area with connection status and action buttons
            ui.horizontal(|ui| {
                // Connection status indicator
                ui.horizontal(|ui| {
                    let status_text = egui::RichText::new("Connected")
                        .color(egui::Color32::GREEN)
                        .strong();
                    ui.label(status_text);
                });

                // Push buttons to the right with horizontal spacing
                let button_layout = if available_width > 320.0 {
                    // Wider layout with buttons side by side
                    egui::Layout::right_to_left(egui::Align::Center)
                } else {
                    // Narrower layout with buttons stacked
                    egui::Layout::top_down(egui::Align::RIGHT)
                };

                ui.with_layout(button_layout, |ui| {
                    // Connection details button - replaced disconnect button
                    let details_button = egui::Button::new(
                        egui::RichText::new("Details")
                            .color(ui.visuals().widgets.active.fg_stroke.color)
                    )
                    .fill(egui::Color32::from_rgb(80, 120, 200));

                    if ui.add(details_button).clicked() {
                        // Could show more detailed connection information in the future
                    }

                    // Optional refresh button if space allows
                    if available_width > 380.0 {
                        if ui.button("Refresh").clicked() {
                            // Could add refresh functionality in the future
                        }
                    }
                });
            });
        });
}

/// Draw the connection controls panel (shown when not connected)
pub fn draw_connection_panel_controls(
    ui: &mut egui::Ui,
    server_address: &str,
    server_port: &str,
    auto_connect: &mut bool,
    auto_reconnect: &mut bool,
    is_connecting: bool,
    status_message: &str,
    event_tx: &mpsc::Sender<AppEvent>,
) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.heading("Connection Controls");
            ui.add_space(5.0);

            ui.checkbox(auto_connect, "Auto-connect on startup");
            ui.checkbox(auto_reconnect, "Auto-reconnect on disconnect");
            ui.add_space(10.0);

            // Connect Button Logic (adapted from action_panel.rs)
            let inputs_valid = !server_address.is_empty()
                && !server_port.is_empty()
                && server_port.parse::<u16>().is_ok();
            // Consider adding more validation for server_address if needed, e.g., no spaces.

            let connect_button_text = if is_connecting {
                "Connecting..."
            } else {
                "Connect"
            };

            let button_color = if is_connecting {
                egui::Color32::from_rgb(200, 200, 100) // Yellow for connecting
            } else if inputs_valid {
                egui::Color32::from_rgb(100, 150, 255) // Blue for ready to connec
            } else {
                egui::Color32::from_rgb(180, 180, 180) // Gray for disabled
            };

            let connect_response = ui.add_enabled(
                !is_connecting && inputs_valid,
                egui::Button::new(egui::RichText::new(connect_button_text).size(18.0).color(
                    if inputs_valid || is_connecting {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::GRAY
                    },
                ))
                .fill(button_color)
                .min_size(egui::Vec2::new(120.0, 32.0)),
            );

            if connect_response.clicked() {
                if !is_connecting {
                    let tx = event_tx.clone();
                    // It's generally better to spawn async tasks from the main runtime handle
                    // provided to the app, rather than tokio::spawn directly in a UI function
                    // if that handle is available. However, for sending a simple event,
                    // this might be acceptable if the event_tx is Send + Sync.
                    // For now, let's assume direct send or a small spawn is fine.
                    // Consider if SaveConfig should be sent here too, like in action_panel.
                    tokio::spawn(async move {
                        if let Err(e) = tx.send(AppEvent::Connect).await {
                            log::error!(
                                "Failed to send connect event from connection_panel_controls: {}",
                                e
                            );
                        }
                    });
                }
            }

            let tooltip_text = if !inputs_valid {
                "Please enter a valid server address and port in the Server Configuration panel."
                    .to_string()
            } else if is_connecting {
                "Attempting to connect...".to_string()
            } else {
                format!("Connect to {}:{}", server_address, server_port)
            };
            connect_response.on_hover_text(tooltip_text);

            ui.add_space(10.0);
            ui.label(format!("Status: {}", status_message));
            if is_connecting {
                ui.spinner();
            }
        });
}
