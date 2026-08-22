use serde_json::json;

pub async fn send_message(chat_id: u64, text: &str) {
    let cfg = crate::config::Config::get();
    let body = json!({ "chat_id": chat_id, "text": text });

    let response = reqwest::Client::new()
        .post(format!(
            "https://api.telegram.org/bot{}/sendMessage",
            cfg.telegram_token,
        ))
        .json(&body)
        .send()
        .await
        .unwrap();

    if !response.status().is_success() {
        eprintln!("telegram response: {}", response.text().await.unwrap());
    }
}
