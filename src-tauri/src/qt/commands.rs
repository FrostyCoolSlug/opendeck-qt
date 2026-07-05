//! Test commands and their registration

use crate::qt::events::emit;
use command_macros::{command, generate_handler};
use serde_json::json;

#[command]
pub async fn echo(text: String) -> String {
    text
}

#[command]
pub async fn upper(text: String) -> String {
    emit("upper-triggered", json!({"text": text}));
    text.to_uppercase()
}

// registers into dispatcher
generate_handler![echo, upper];
