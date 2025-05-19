use eframe::egui;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use crate::ui::events::AppEvent;

/// Draw the authentication panel
pub fn draw_auth_panel(
    ui: &mut egui::Ui,
    username: &mut String,
    auth_method: &mut String,
    remember_credentials: &mut bool,
    event_tx: &mpsc::Sender<AppEvent>,
    rt_handle: &Handle,
) {
    egui::CollapsingHeader::new("Authentication")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Username:");
                let response = ui.text_edit_singleline(username);
                ui.label("").on_hover_text("Enter your username for authentication");
                let changed = response.changed();
                let focus_lost = response.lost_focus();
                
                if !username.is_empty() {
                    if username.len() >= 3 && !username.contains(char::is_whitespace) {
                        ui.colored_label(egui::Color32::GREEN, "✓");
                    } else {
                        ui.colored_label(egui::Color32::RED, "⚠");
                        ui.label("Username must be at least 3 characters with no spaces");
                    }
                }
                
                if focus_lost && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    ui.memory_mut(|mem| mem.request_focus(ui.next_auto_id()));
                }
            });
            
            ui.horizontal(|ui| {
                ui.label("Method:");
                let _auth_dropdown = egui::ComboBox::from_label("")
                    .selected_text(auth_method)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(auth_method, "password".to_string(), "Password");
                        ui.selectable_value(auth_method, "psk".to_string(), "Pre-Shared Key");
                        ui.selectable_value(auth_method, "native".to_string(), "Native OS");
                    });
                    
                // Add info tooltip based on selected auth method
                let info_text = match auth_method.as_str() {
                    "password" => "Standard password authentication",
                    "psk" => "Pre-shared key authentication",
                    "native" => "Use system-level authentication",
                    _ => "Unknown authentication method",
                };
                
                // Show a colored icon based on how secure the method is
                let (security_icon, security_color) = match auth_method.as_str() {
                    "password" => ("🔑", egui::Color32::YELLOW),   // Medium security
                    "psk" => ("🔒", egui::Color32::GREEN),         // High security
                    "native" => ("🛡", egui::Color32::LIGHT_GREEN), // Good security
                    _ => ("❓", egui::Color32::RED),                // Unknown
                };
                
                ui.colored_label(security_color, security_icon)
                    .on_hover_text(info_text);
            });
            
            // Additional auth options based on selected method
            match auth_method.as_str() {
                "password" => {
                    // Password input field with masking
                    ui.horizontal(|ui| {
                        ui.label("Password:");
                        
                        // Store password and visibility state
                        static mut PASSWORD: String = String::new();
                        static mut SHOW_PASSWORD: bool = false;
                        
                        let password = unsafe { &mut PASSWORD };
                        let show_password = unsafe { &mut SHOW_PASSWORD };
                        
                        // Create password display
                        let mut password_display = if !*show_password {
                            "•".repeat(password.len())
                        } else {
                            password.clone()
                        };
                        
                        let password_edit = ui.add(
                            egui::TextEdit::singleline(&mut password_display)
                                .password(!*show_password)
                                .hint_text("Enter password")
                        );
                        
                        if password_edit.changed() && *show_password {
                            *password = password_display.clone();
                        }
                        
                        // Toggle password visibility with button
                        if ui.button(if *show_password { "🙈" } else { "👁" }).clicked() {
                            *show_password = !*show_password;
                        }
                        
                        password_edit.on_hover_text("Enter your password for authentication");
                    });
                }
                "psk" => {
                    // PSK configuration could go here
                    ui.label("NOTE: PSK configuration is done in the client config file");
                }
                "native" => {
                    ui.label("Using native OS authentication mechanisms");
                }
                _ => {}
            }
            
            // Remember credentials checkbox with better feedback
            ui.horizontal(|ui| {
                let remember_label = ui.checkbox(remember_credentials, "Remember credentials")
                    .on_hover_text("Save connection credentials for future use");
                
                if *remember_credentials {
                    ui.colored_label(egui::Color32::LIGHT_GREEN, "✓")
                        .on_hover_text("Credentials will be saved when connecting");
                }
                
                if remember_label.changed() {
                    // If the user unchecks this, we should clear saved credentials
                    if !*remember_credentials {
                        let tx = event_tx.clone();
                        rt_handle.spawn(async move {
                            let _ = tx.send(AppEvent::ClearCredentials).await;
                        });
                    } else {
                        let tx = event_tx.clone();
                        rt_handle.spawn(async move {
                            let _ = tx.send(AppEvent::SaveCredentials).await;
                        });
                    }
                }
            });
        });
}
