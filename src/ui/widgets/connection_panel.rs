use eframe::egui;

/// Draw the connection details panel (only shown when connected)
pub fn draw_connection_panel(
    ui: &mut egui::Ui,
    server_address: &str,
    server_port: &str,
    username: &str,
    auth_method: &str,
    use_tls: bool,
) {
    egui::CollapsingHeader::new("Connection Details")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Connected to:");
                ui.strong(format!("{}:{}", server_address, server_port));
            });
            
            ui.horizontal(|ui| {
                ui.label("User:");
                ui.strong(username);
            });
            
            ui.horizontal(|ui| {
                ui.label("Auth method:");
                ui.strong(auth_method);
            });
            
            ui.horizontal(|ui| {
                ui.label("Encryption:");
                if use_tls {
                    ui.strong("TLS Encrypted 🔒");
                } else {
                    ui.colored_label(egui::Color32::YELLOW, "Unencrypted ⚠");
                }
            });
        });
}
