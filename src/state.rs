use std::sync::mpsc::Receiver;

#[derive(Default, Clone)]
pub struct ApplicationDetails {
    pub app_id: String,
    pub owner_id: String,
    pub app_name: String,
    pub flags: u64,
}

#[derive(Default)]
pub struct AppState {
    pub token: TokenState,
    pub widget: WidgetState,
}

#[derive(Default)]
pub struct TokenState {
    pub token: String,
    pub error: String,
    pub token_confirmed: bool,
    pub confirmed: bool,
    pub fetching: bool,
    pub details: Option<ApplicationDetails>,
    pub receiver: Option<Receiver<Result<ApplicationDetails, String>>>,
}

#[derive(Default)]
pub struct WidgetState {
    pub fetching: bool,
    pub config_json: String,
    pub sample_data: String,
    pub error: String,
    pub receiver: Option<Receiver<Result<String, String>>>,
    pub applying: bool,
    pub apply_success: Option<String>,
    pub apply_error: Option<String>,
    pub apply_receiver: Option<Receiver<Result<String, String>>>,
    pub publishing: bool,
    pub publish_success: Option<String>,
    pub publish_error: Option<String>,
    pub publish_receiver: Option<Receiver<Result<String, String>>>,
}
