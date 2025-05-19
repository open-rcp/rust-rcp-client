use eframe::egui;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use crate::ui::events::AppEvent;

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
) {
    egui::CollapsingHeader::new("Connection Details")
        .default_open(true)
        .show(ui, |ui| {
            // Use a grid layout for better organization
            egui::Grid::new("connection_details_grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    // Server connection info
                    ui.label("Connected to:");
                    ui.strong(format!("{}:{}", server_address, server_port));
                    ui.end_row();
                    
                    // User info
                    ui.label("User:");
                    ui.strong(username);
                    ui.end_row();
                    
                    // Authentication method
                    ui.label("Auth method:");
                    ui.strong(auth_method);
                    ui.end_row();
                    
                    // Encryption status
                    ui.label("Encryption:");
                    if use_tls {
                        ui.horizontal(|ui| {
                            ui.strong("TLS Encrypted");
                            ui.label("🔒").on_hover_text("Your connection is secure");
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::YELLOW, "Unencrypted");
                            ui.label("⚠").on_hover_text("Your connection is not encrypted");
                        });
                    }
                    ui.end_row();
                });
                
            // Add a little space after the grid
            ui.add_space(8.0);
            
            // Add a status indicator at the bottom
            ui.horizontal(|ui| {
                ui.label("Status:");
                let connection_status = egui::RichText::new("Connected")
                    .color(egui::Color32::GREEN)
                    .strong();
                ui.label(connection_status);
                
                // Push the disconnect button to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Disconnect").clicked() {
                        // Send disconnect event
                        let tx = event_tx.clone();
                        rt_handle.spawn(async move {
                            let _ = tx.send(AppEvent::Disconnect).await;
                        });
                    }
                });
            });
        });
}
