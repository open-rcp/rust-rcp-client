use eframe::egui;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use anyhow::Result;
use crate::ui::events::AppEvent;

/// Draw the action panel with connect/disconnect buttons
pub fn draw_action_panel(
    ui: &mut egui::Ui,
    is_connected: bool,
    connecting: bool,
    server_address: &str,
    server_port: &str,
    auth_method: &str,
    event_tx: &mpsc::Sender<AppEvent>,
    rt_handle: &Handle,
    save_config: &mut dyn FnMut() -> Result<()>,
    update_status: &mut dyn FnMut(String),
    connect: &dyn Fn(),
    disconnect: &dyn Fn(),
) {
    ui.horizontal(|ui| {
        // Input validation for connect button
        let inputs_valid = !server_address.is_empty() 
            && !server_port.is_empty()
            && server_port.parse::<u16>().is_ok()
            && server_address.chars().all(|c| {
                c.is_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ':'
            });
        
        // Create a bigger, more visually distinctive connect button
        let connect_text = if connecting {
            "Connecting..."
        } else if is_connected {
            "Reconnect"
        } else {
            "Connect"
        };
        
        // Calculate button color based on connection state
        let button_color = if is_connected {
            egui::Color32::from_rgb(100, 200, 100) // Green for connected
        } else if connecting {
            egui::Color32::from_rgb(200, 200, 100) // Yellow for connecting
        } else if inputs_valid {
            egui::Color32::from_rgb(100, 150, 255) // Blue for ready to connect
        } else {
            egui::Color32::from_rgb(180, 180, 180) // Gray for disabled
        };
        
        // Custom connect button with better visual appearance
        let connect_response = ui.add_enabled(
            !connecting && inputs_valid,
            egui::Button::new(
                egui::RichText::new(connect_text)
                    .size(18.0)
                    .color(if inputs_valid || is_connected || connecting { 
                        egui::Color32::WHITE 
                    } else { 
                        egui::Color32::GRAY 
                    })
            )
            .fill(button_color)
            .min_size(egui::Vec2::new(120.0, 32.0))
        );
        
        // Add a tooltip with connection details
        let tooltip_text = if !inputs_valid {
            "Please enter valid server address and port".to_string()
        } else if is_connected {
            format!("Reconnect to {}:{}", server_address, server_port)
        } else {
            format!("Connect to {}:{} using {} authentication", 
                server_address, 
                server_port,
                auth_method)
        };
        
        // Apply tooltip to the tooltip text
        ui.label("").on_hover_text(tooltip_text);
        
        // Handle click response separately
        if connect_response.clicked() {
            // First save the config
            if let Err(e) = save_config() {
                update_status(format!("Error saving config: {}", e));
            } else {
                update_status("Saving config and connecting...".to_string());
                connect();
            }
        }
        
        // Add keyboard shortcut for connect
        if ui.input_mut(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.ctrl) && inputs_valid && !connecting {
            if let Err(e) = save_config() {
                update_status(format!("Error saving config: {}", e));
            } else {
                update_status("Saving config and connecting...".to_string());
                connect();
            }
        }
        
        // Disconnect button
        let disconnect_response = ui.add_enabled(
            is_connected,
            egui::Button::new(
                egui::RichText::new("Disconnect")
                    .size(18.0)
                    .color(if is_connected { egui::Color32::WHITE } else { egui::Color32::GRAY })
            )
            .fill(if is_connected { egui::Color32::from_rgb(200, 100, 100) } else { egui::Color32::from_rgb(180, 180, 180) })
            .min_size(egui::Vec2::new(120.0, 32.0))
        );
        
        if disconnect_response.clicked() {
            update_status("Disconnecting...".to_string());
            disconnect();
        }
        
        let disconnect_tooltip = if is_connected {
            format!("Disconnect from {}:{}", server_address, server_port)
        } else {
            "Not currently connected".to_string()
        };
        ui.label("").on_hover_text(disconnect_tooltip);
        
        // Add keyboard shortcut for disconnect
        if ui.input_mut(|i| i.key_pressed(egui::Key::D) && i.modifiers.ctrl) {
            if is_connected {
                update_status("Disconnecting...".to_string());
                disconnect();
            }
        }
        
        // Save config button
        let save_button = ui.add(
            egui::Button::new(
                egui::RichText::new("Save")
                    .size(18.0)
            )
            .fill(egui::Color32::from_rgb(150, 150, 200))
            .min_size(egui::Vec2::new(80.0, 32.0))
        );
        
        if save_button.clicked() {
            if let Err(e) = save_config() {
                update_status(format!("Error initiating config save: {}", e));
            } else {
                update_status("Saving configuration...".to_string());
            }
        }
        
        ui.label("").on_hover_text("Save current configuration (Ctrl+S)");
        
        // Add keyboard shortcut for save
        if ui.input_mut(|i| i.key_pressed(egui::Key::S) && i.modifiers.ctrl) {
            if let Err(e) = save_config() {
                update_status(format!("Error initiating config save: {}", e));
            } else {
                update_status("Saving configuration...".to_string());
            }
        }
    });
}

/// Draw progress indicator for connection attempts
pub fn draw_connection_progress(
    ui: &mut egui::Ui,
    server_address: &str,
    server_port: &str,
) {
    ui.add_space(5.0);
    egui::Frame::none()
        .fill(egui::Color32::from_rgba_premultiplied(255, 255, 0, 25))
        .rounding(egui::Rounding::same(5.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.spinner(); // Show a spinner animation while connecting
                    ui.label(format!("Connecting to {}:{}...", server_address, server_port));
                });
                
                // Add connection attempt counter or timeout info
                ui.label("This may take a few seconds. Press ESC to cancel.");
                
                // Add a progress bar
                let time = ui.input(|i| i.time);
                let progress = (time % 3.0) as f32 / 3.0; // Create a cycling progress between 0-1
                ui.add(egui::ProgressBar::new(progress).animate(true));
            });
        });
}

/// Draw the help and status footer
pub fn draw_footer(ui: &mut egui::Ui) {
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        // Add keyboard shortcuts help section
        let help_frame = egui::Frame::none()
            .fill(egui::Color32::from_rgba_premultiplied(100, 100, 100, 25))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(10.0);
        
        help_frame.show(ui, |ui| {
            ui.collapsing("⌨ Keyboard Shortcuts", |ui| {
                ui.label("Tab / Shift+Tab: Navigate between fields");
                ui.label("Enter: Move to next field");
                ui.label("Ctrl+Enter: Connect to server");
                ui.label("Ctrl+S: Save configuration");
                ui.label("Ctrl+D: Disconnect from server");
                ui.label("Esc: Cancel connection attempt");
            });
            
            // Add a small vertical space
            ui.add_space(5.0);
            
            // Status indicator legend
            ui.collapsing("🏁 Status Indicators", |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::GREEN, "Green");
                    ui.label("Connected");
                });
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::YELLOW, "Yellow");
                    ui.label("Connecting");
                });
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::RED, "Red");
                    ui.label("Disconnected or Error");
                });
            });
        });
        
        ui.add_space(5.0);
        
        // Version and links
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.label("RCP Client v1.0.0 •");
            ui.hyperlink_to("Help", "https://github.com/open-rcp/rust-rcp-client");
            ui.label("•");
            ui.hyperlink_to("Report Bug", "https://github.com/open-rcp/rust-rcp-client/issues");
        });
    });
}
