#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{LazyLock, mpsc};

use eframe::egui::{self, Color32, RichText, TextEdit};
use regex::Regex;

use crate::state::{AppState, ApplicationDetails};

mod state;
mod validator;

const TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("^[A-Za-z0-9_-]{24,26}\\.[A-Za-z0-9_-]{6}\\.[A-Za-z0-9_-]{27,40}$").expect(":(")
});

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Widget Identity Creator",
        native_options,
        Box::new(|cc| Ok(Box::new(WidgetIdentityCreatorApp::new(cc)))),
    )
    .expect("Could not start app");
}

#[derive(Default)]
struct WidgetIdentityCreatorApp {
    state: AppState,
}

impl WidgetIdentityCreatorApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
}

#[derive(serde::Deserialize)]
struct AppOwner {
    id: String,
}

#[derive(serde::Deserialize)]
struct AppTeam {
    owner_user_id: String,
}

#[derive(serde::Deserialize)]
struct AppInfoResponse {
    id: String,
    owner: Option<AppOwner>,
    team: Option<AppTeam>,
    name: String,
}

impl eframe::App for WidgetIdentityCreatorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.state.token.receiver {
            if let Ok(res) = rx.try_recv() {
                self.state.token.fetching = false;
                self.state.token.receiver = None;
                match res {
                    Ok(details) => {
                        self.state.token.details = Some(details);
                        self.state.token.token_confirmed = true;
                        self.state.token.error = String::new();
                    }
                    Err(err) => {
                        self.state.token.error = err;
                        self.state.token.token_confirmed = false;
                    }
                }
            }
        }

        if let Some(rx) = &self.state.widget.receiver {
            if let Ok(res) = rx.try_recv() {
                self.state.widget.fetching = false;
                self.state.widget.receiver = None;
                match res {
                    Ok(json) => {
                        if crate::validator::extract_dynamic_fields(&json).is_empty() {
                            self.state.widget.sample_data = "{\"data\":{}}".to_string();
                        }
                        self.state.widget.config_json = json;
                        self.state.widget.error = String::new();
                    }
                    Err(err) => {
                        self.state.widget.error = err;
                    }
                }
            }
        }

        if let Some(rx) = &self.state.widget.apply_receiver {
            if let Ok(res) = rx.try_recv() {
                self.state.widget.applying = false;
                self.state.widget.apply_receiver = None;
                match res {
                    Ok(msg) => {
                        self.state.widget.apply_success = Some(msg);
                        self.state.widget.apply_error = None;
                    }
                    Err(err) => {
                        self.state.widget.apply_error = Some(err);
                        self.state.widget.apply_success = None;
                    }
                }
            }
        }

        if let Some(rx) = &self.state.widget.publish_receiver {
            if let Ok(res) = rx.try_recv() {
                self.state.widget.publishing = false;
                self.state.widget.publish_receiver = None;
                match res {
                    Ok(msg) => {
                        self.state.widget.publish_success = Some(msg);
                        self.state.widget.publish_error = None;
                    }
                    Err(err) => {
                        self.state.widget.publish_error = Some(err);
                        self.state.widget.publish_success = None;
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("main_scroll").show(ui, |ui| {
            ui.heading("Widget Identity Creator");
            ui.label("This app will guide you through applying the application identity for your widget.");

            ui.separator();

            ui.label("Please put in the BOT token for your widget application below");
            let is_disabled = self.state.token.token_confirmed || self.state.token.fetching;

            ui.add_enabled_ui(!is_disabled, |ui| {
                if is_disabled {
                    ui.add(TextEdit::singleline(&mut self.state.token.token).password(true));
                } else {
                    ui.text_edit_singleline(&mut self.state.token.token);
                }
            });

            if !self.state.token.error.is_empty() {
                ui.label(RichText::new(self.state.token.error.clone()).color(Color32::RED));
            }

            if self.state.token.fetching {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Fetching application details...");
                });
            } else if !self.state.token.token_confirmed {
                if ui.button("Confirm").clicked() {
                    if !TOKEN_REGEX.is_match(&self.state.token.token) {
                        self.state.token.error = String::from("Invalid bot token, make sure you paste in the full bot token as provided on the Discord developer portal.");
                    } else {
                        self.state.token.error = String::new();
                        self.state.token.fetching = true;
                        let token = self.state.token.token.clone();
                        let ctx = ui.ctx().clone();
                        let (tx, rx) = mpsc::channel();
                        self.state.token.receiver = Some(rx);

                        std::thread::spawn(move || {
                            let client = reqwest::blocking::Client::new();
                            let res = client
                                .get("https://discord.com/api/v10/oauth2/applications/@me")
                                .header("Authorization", format!("Bot {token}"))
                                .send();

                            let result = match res {
                                Ok(resp) => {
                                    if resp.status().is_success() {
                                        match resp.json::<AppInfoResponse>() {
                                            Ok(info) => {
                                                let owner_id = if let Some(owner) = info.owner {
                                                    owner.id
                                                } else if let Some(team) = info.team {
                                                    team.owner_user_id
                                                } else {
                                                    String::from("Unknown")
                                                };
                                                Ok(ApplicationDetails {
                                                    app_id: info.id,
                                                    app_name: info.name,
                                                    owner_id,
                                                })
                                            }
                                            Err(err) => Err(format!("Failed to parse application details: {err}")),
                                        }
                                    } else {
                                        Err(format!("Discord API returned error: {}", resp.status()))
                                    }
                                }
                                Err(err) => Err(format!("Request failed: {err}")),
                            };

                            let _ = tx.send(result);
                            ctx.request_repaint();
                        });
                    }
                }
            }

            if self.state.token.token_confirmed {
                let app_id = self.state.token.details.as_ref().map(|d| d.app_id.clone()).unwrap_or_default();
                if let Some(details) = &mut self.state.token.details {
                    ui.add_enabled_ui(false, |ui| {
                        ui.label("Application Name:");
                        ui.text_edit_singleline(&mut details.app_name);
                        ui.label("Application ID:");
                        ui.text_edit_singleline(&mut details.app_id);
                    });

                    ui.add_enabled_ui(!self.state.token.confirmed, |ui| {
                        ui.label("User ID:");
                        ui.text_edit_singleline(&mut details.owner_id);
                    });
                }

                ui.horizontal(|ui| {
                    if !self.state.token.confirmed {
                        if ui.button("Confirm").clicked() {
                            self.state.token.confirmed = true;
                            self.state.widget.fetching = true;
                            let token = self.state.token.token.clone();
                            let ctx = ui.ctx().clone();
                            let (tx, rx) = mpsc::channel();
                            self.state.widget.receiver = Some(rx);

                            std::thread::spawn(move || {
                                let client = reqwest::blocking::Client::new();
                                let url = format!("https://discord.com/api/v10/applications/{app_id}/widget-configs");
                                let res = client
                                    .get(&url)
                                    .header("Authorization", format!("Bot {token}"))
                                    .send();

                                let result = match res {
                                    Ok(resp) => {
                                        if resp.status().is_success() {
                                            match resp.json::<serde_json::Value>() {
                                                Ok(val) => match serde_json::to_string_pretty(&val) {
                                                    Ok(pretty) => Ok(pretty),
                                                    Err(_) => Ok(val.to_string()),
                                                },
                                                Err(err) => Err(format!("Failed to parse JSON: {err}")),
                                            }
                                        } else {
                                            Err(format!("Discord API returned error: {}", resp.status()))
                                        }
                                    }
                                    Err(err) => Err(format!("Request failed: {err}")),
                                };

                                let _ = tx.send(result);
                                ctx.request_repaint();
                            });
                        }
                    }
                    if ui.button("Reset").clicked() {
                        self.state = Default::default();
                    }
                });

                if self.state.widget.fetching {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Fetching widget configuration...");
                    });
                }

                if !self.state.widget.error.is_empty() {
                    ui.label(RichText::new(self.state.widget.error.clone()).color(Color32::RED));
                }

                if !self.state.widget.config_json.is_empty() {
                    ui.separator();

                    ui.columns(2, |cols| {
                        cols[0].vertical(|ui| {
                            ui.label("Found Widget Configuration:");

                            let config_errors = crate::validator::validate_widget_config(&self.state.widget.config_json);
                            if !config_errors.is_empty() {
                                ui.add_space(4.0);
                                ui.label(RichText::new("Widget Configuration Errors:").color(Color32::RED).strong());
                                for err in &config_errors {
                                    ui.label(RichText::new(format!("- {err}")).color(Color32::RED));
                                }
                                ui.add_space(4.0);
                            }

                            egui::ScrollArea::vertical()
                                .id_salt("config_scroll")
                                .min_scrolled_height(400.0)
                                .max_height(500.0)
                                .show(ui, |ui| {
                                    ui.add_enabled_ui(false, |ui| {
                                        ui.add(
                                            egui::TextEdit::multiline(&mut self.state.widget.config_json)
                                                .desired_width(f32::INFINITY)
                                                .desired_rows(20)
                                                .code_editor(),
                                        );
                                    });
                                });
                        });

                        cols[1].vertical(|ui| {
                            let specs = crate::validator::extract_dynamic_fields(&self.state.widget.config_json);
                            if !specs.is_empty() {
                                ui.label("Manual Dynamic Fields Editor:");
                                egui::Grid::new("manual_editor_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                                    for spec in &specs {
                                        ui.label(format!("{} ({}):", spec.name, spec.presentation_type));
                                        let mut input_val = crate::validator::get_field_value(&self.state.widget.sample_data, &spec.name, &spec.presentation_type);
                                        if ui.text_edit_singleline(&mut input_val).changed() {
                                            crate::validator::update_sample_data(
                                                &mut self.state.widget.sample_data,
                                                &spec.name,
                                                &spec.presentation_type,
                                                &input_val,
                                            );
                                        }
                                        ui.end_row();
                                    }
                                });

                                ui.add_space(10.0);
                                ui.label("Raw Sample Data JSON:");
                            } else if !self.state.widget.config_json.is_empty() {
                                if self.state.widget.sample_data.is_empty() {
                                    self.state.widget.sample_data = "{\"data\":{}}".to_string();
                                }

                                ui.label("Sample Data:");
                                ui.label(
                                    RichText::new("No dynamic fields found in config, no sample data required!")
                                        .weak(),
                                );
                            } else {
                                ui.label("Sample Data:");
                            }

                            egui::ScrollArea::vertical()
                                .id_salt("sample_scroll")
                                .min_scrolled_height(300.0)
                                .max_height(400.0)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.state.widget.sample_data)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(20)
                                            .code_editor(),
                                    );
                                });

                            if self.state.widget.sample_data.is_empty() {
                                ui.label(
                                    RichText::new("Please insert the sample JSON data from the widget editor or fill in the fields using the dynamic fields editor above!")
                                        .color(Color32::YELLOW),
                                );
                            } else if !self.state.widget.config_json.is_empty() {
                                let errors = crate::validator::validate_sample_data(
                                    &self.state.widget.config_json,
                                    &self.state.widget.sample_data,
                                );
                                if errors.is_empty() {
                                    ui.label(RichText::new("Sample data matches widget config requirements!").color(Color32::GREEN));
                                } else {
                                    ui.label(RichText::new("Validation Errors:").color(Color32::RED).strong());

                                    for err in errors {
                                        ui.label(RichText::new(format!("- {err}")).color(Color32::RED));
                                    }
                                }
                            }
                        });
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(4.0);

                    if !self.state.widget.sample_data.is_empty() {
                        let errors = crate::validator::validate_sample_data(
                            &self.state.widget.config_json,
                            &self.state.widget.sample_data,
                        );
                        if errors.is_empty() {
                            if self.state.widget.applying {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("Applying sample data to application identity...");
                                });
                            } else {
                                if ui.button("Apply to Application Identity").clicked() {
                                    self.state.widget.applying = true;
                                    self.state.widget.apply_success = None;
                                    self.state.widget.apply_error = None;

                                    let app_id = self.state.token.details.as_ref().map(|d| d.app_id.clone()).unwrap_or_default();
                                    let user_id = self.state.token.details.as_ref().map(|d| d.owner_id.clone()).unwrap_or_default();
                                    let token = self.state.token.token.clone();
                                    let sample_data = self.state.widget.sample_data.clone();
                                    let ctx = ui.ctx().clone();
                                    let (tx, rx) = mpsc::channel();
                                    self.state.widget.apply_receiver = Some(rx);

                                    std::thread::spawn(move || {
                                        let client = reqwest::blocking::Client::new();
                                        let url = format!("https://discord.com/api/v9/applications/{app_id}/users/{user_id}/identities/0/profile");
                                        let res = client
                                            .patch(&url)
                                            .header("Content-Type", "application/json")
                                            .header("Authorization", format!("Bot {token}"))
                                            .body(sample_data)
                                            .send();

                                        let result = match res {
                                            Ok(resp) => {
                                                if resp.status().is_success() {
                                                    Ok(String::from("Application identity updated successfully!"))
                                                } else {
                                                    let status = resp.status();
                                                    let body = resp.text().unwrap_or_else(|_| String::from("No response body"));
                                                    Err(format!("Discord API error ({status}): {body}"))
                                                }
                                            }
                                            Err(err) => Err(format!("Request failed: {err}")),
                                        };

                                        let _ = tx.send(result);
                                        ctx.request_repaint();
                                    });
                                }
                            }

                            if let Some(msg) = &self.state.widget.apply_success {
                                ui.label(RichText::new(msg).color(Color32::GREEN).strong());
                            }
                            if let Some(err) = &self.state.widget.apply_error {
                                if err.contains("50025") || err.contains("Invalid OAuth2 access token") {
                                    ui.add_space(4.0);
                                    ui.label(RichText::new("Error: Invalid OAuth2 access token (code 50025)").color(Color32::RED).strong());
                                    ui.label(RichText::new("Please complete the application's Social SDK application form in the Discord Developer Portal.").color(Color32::YELLOW));

                                    let app_id = self.state.token.details.as_ref().map(|d| d.app_id.clone()).unwrap_or_default();
                                    let mut auth_url = format!("https://discord.com/oauth2/authorize?client_id={app_id}&response_type=token&scope=sdk.social_layer_presence");

                                    ui.label("Then authorize your application using the URL below:");
                                    ui.hyperlink_to("Open Authorization URL in Browser", &auth_url);
                                    ui.text_edit_singleline(&mut auth_url);
                                } else {
                                    ui.label(RichText::new(err).color(Color32::RED));
                                }
                            }
                        }
                    }

                    if let Some((config_id, status)) = crate::validator::get_widget_config_info(&self.state.widget.config_json) {
                        if status.eq_ignore_ascii_case("draft") {
                            ui.add_space(8.0);
                            ui.label(RichText::new("Warning: This widget config is currently in 'draft' status and widgets will only be visible to the applications developers.").color(Color32::YELLOW).strong());

                            if self.state.widget.publishing {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("Publishing widget configuration...");
                                });
                            } else {
                                if ui.button("Publish Widget Config").clicked() {
                                    self.state.widget.publishing = true;
                                    self.state.widget.publish_success = None;
                                    self.state.widget.publish_error = None;

                                    let app_id = self.state.token.details.as_ref().map(|d| d.app_id.clone()).unwrap_or_default();
                                    let token = self.state.token.token.clone();
                                    let ctx = ui.ctx().clone();
                                    let (tx, rx) = mpsc::channel();
                                    self.state.widget.publish_receiver = Some(rx);

                                    std::thread::spawn(move || {
                                        let client = reqwest::blocking::Client::new();
                                        let url = format!("https://discord.com/api/v10/applications/{app_id}/widget-configs/{config_id}/publish");
                                        let res = client
                                            .post(&url)
                                            .header("Authorization", format!("Bot {token}"))
                                            .send();

                                        let result = match res {
                                            Ok(resp) => {
                                                if resp.status().is_success() {
                                                    Ok(String::from("Widget config published successfully!"))
                                                } else {
                                                    let status = resp.status();
                                                    let body = resp.text().unwrap_or_else(|_| String::from("No response body"));
                                                    Err(format!("Discord API error ({status}): {body}"))
                                                }
                                            }
                                            Err(err) => Err(format!("Request failed: {err}")),
                                        };

                                        let _ = tx.send(result);
                                        ctx.request_repaint();
                                    });
                                }
                            }

                            if let Some(msg) = &self.state.widget.publish_success {
                                ui.label(RichText::new(msg).color(Color32::GREEN).strong());
                            }
                            if let Some(err) = &self.state.widget.publish_error {
                                ui.add_space(4.0);
                                ui.label(RichText::new("The widget config could not be published.").color(Color32::RED).strong());
                                ui.label("Please attempt to publish your widget config directly through the Discord Developer Portal to see any specific issues or requirements.");

                                let app_id = self.state.token.details.as_ref().map(|d| d.app_id.clone()).unwrap_or_default();
                                if !app_id.is_empty() {
                                    let dev_portal_url = format!("https://discord.com/developers/applications/{app_id}");
                                    ui.hyperlink_to("Open Discord Developer Portal", &dev_portal_url);
                                }

                                ui.label(format!("Details: {err}"));
                            }
                        }
                    }
                }

                if self.state.widget.config_json == "[]" {
                    ui.label(RichText::new("No widget configuration found. Please create a widget in the developer portal!").color(Color32::RED));
                }
            }
            });
        });
    }
}
