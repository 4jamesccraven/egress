use crate::error::TelegramError;
use serde_json::{Value, json};

pub fn validate_api_response(response: Value) -> Result<Value, TelegramError> {
    match response["ok"].as_bool() {
        Some(true) => Ok(response),

        Some(false) => match response["description"].as_str() {
            Some(why) => Err(TelegramError::API(why.into())),
            None => Err(TelegramError::Unknown),
        },
        None => return Err(TelegramError::Unknown),
    }
}

async fn api_call(endpoint: &str, json: &Value) -> Result<Value, TelegramError> {
    let cfg = crate::config::Config::get();

    let response = reqwest::Client::new()
        .post(format!(
            "https://api.telegram.org/bot{}/{}",
            cfg.telegram_token, endpoint
        ))
        .json(json)
        .send()
        .await?;

    if !response.status().is_success() {
        eprintln!("warning: got bad response: {}", response.status())
    }

    let response: Value = response.json().await?;
    validate_api_response(response)
}

pub async fn send_message(chat_id: i64, text: &str) -> Result<i64, TelegramError> {
    let body = json!({ "chat_id": chat_id, "text": text });
    let response = api_call("sendMessage", &body).await?;

    let message_id = response["result"]["message_id"]
        .as_i64()
        .ok_or(TelegramError::Unknown)?;

    Ok(message_id)
}

pub async fn delete_message(chat_id: i64, message_id: i64) -> Result<(), TelegramError> {
    let body = json!({ "chat_id": chat_id, "message_id": message_id});
    api_call("deleteMessage", &body).await?;

    Ok(())
}
