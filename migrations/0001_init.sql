CREATE TABLE telegram_messages (
    id INTEGER PRIMARY KEY,
    chat_id INTEGER NOT NULL,
    message_id INTEGER NOT NULL,
    sent_at INTEGER NOT NULL
)
