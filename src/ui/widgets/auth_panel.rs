use eframe::egui;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::ui::events::AppEvent;
use crate::ui::models::AppState;

/// Draw the authentication panel
pub fn draw_auth_panel(
    ui: &mut egui::Ui,
    username: &mut String,
    auth_method: &mut String,
    remember_credentials: &mut bool,
    event_tx: &mpsc::Sender<AppEvent>,
    rt_handle: &Handle,
    app_state: &Arc<Mutex<AppState>>,
) {
    egui::CollapsingHeader::new("Authentication")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Username:");
                let response = ui.text_edit_singleline(username);
                ui.label("").on_hover_text("Enter your username for authentication");
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
                    .selected_text(auth_method.as_str())
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
                        
                        // Get password and visibility state from app state
                        let state_mutex = app_state.clone();
                        let mut password_display;
                        let show_password;
                        
                        // Only hold the lock for a short time - using non-blocking approach
                        {
                            // Try to lock without blocking
                            if let Ok(state) = state_mutex.try_lock() {
                                show_password = state.show_password;
                                password_display = state.password.clone();
                            } else {
                                // If we can't get the lock, use default values
                                show_password = false;
                                password_display = String::new();
                            }
                        }
                        
                        let password_edit = ui.add(
                            egui::TextEdit::singleline(&mut password_display)
                                .password(!show_password)
                                .hint_text("Enter password")
                        );
                        
                        // Update password in app state if needed - always update regardless of show_password
                        if password_edit.changed() {
                            // Use non-blocking try_lock instead of block_on
                            if let Ok(mut state) = state_mutex.try_lock() {
                                state.password = password_display.clone();
                            }
                        }
                        
                        // Toggle password visibility with button
                        if ui.button(if show_password { "🙈" } else { "👁" }).clicked() {
                            // Use non-blocking try_lock instead of block_on
                            if let Ok(mut state) = state_mutex.try_lock() {
                                state.show_password = !state.show_password;
                            }
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
