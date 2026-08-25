use crate::error::TelegramError;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::sqlite::SqliteRow;
use tabled::Tabled;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Tabled)]
pub struct TelegramMessage {
    pub chat_id: i64,
    pub message_id: i64,
    #[tabled(display = "display_time")]
    pub sent_at: i64,
}

impl From<SqliteRow> for TelegramMessage {
    fn from(row: SqliteRow) -> Self {
        use sqlx::Row;
        TelegramMessage {
            chat_id: row.get("chat_id"),
            message_id: row.get("message_id"),
            sent_at: row.get("sent_at"),
        }
    }
}

fn display_time(timestamp: &i64) -> String {
    use jiff::{Timestamp, tz::TimeZone};

    let time = Timestamp::from_second(*timestamp)
        .expect("jiff made this timestamp")
        .to_zoned(TimeZone::system());

    time.strftime("%A, %B %-d, %Y %H:%M:%S").to_string()
}

pub fn validate_api_response(response: Value) -> Result<Value, TelegramError> {
    match response["ok"].as_bool() {
        Some(true) => Ok(response),

        Some(false) => match response["description"].as_str() {
            Some(why) => Err(TelegramError::Api(why.into())),
            None => Err(TelegramError::Unknown),
        },
        None => Err(TelegramError::Unknown),
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
