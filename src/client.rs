use crate::cli::ClientCLI;
use crate::error::ClientError;
use crate::protocol::{CommandProtocol, ResponseData, ResponseProtocol};

use tabled::settings::Style;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};

pub struct Client;

impl Client {
    pub async fn run(args: &ClientCLI) -> Result<bool, ClientError> {
        let socket = Self::connect_sock().await?;
        let (mut reader, mut writer) = socket.into_split();

        let protocol: CommandProtocol = args.command.clone().into();

        Self::write_payload(&mut writer, protocol).await?;
        let response = Self::read_response(&mut reader).await?;

        println!("{}", Self::format_response(&response));

        Ok(response.success)
    }

    fn format_response(response: &ResponseProtocol) -> String {
        use ResponseData::*;
        match response.data.clone() {
            Status { text } => text.clone(),
            NotifyLeft {
                success_count: success,
                total,
            } => {
                format!("Successfully notified {success} of {total} targets.")
            }
            Purge {
                success_count: success,
                failure,
                error,
            } => {
                if let Some(msg) = error {
                    format!("Unable to comply: {msg}")
                } else {
                    let mut out = format!("Successfully removed {success} messages.");

                    if !failure.is_empty() {
                        out.push_str(" Purge completed with errors:\n");

                        let errors = failure
                            .iter()
                            .map(|f| {
                                format!(
                                    "Failed to delete message {} in chat {}: {}",
                                    f.message_id, f.chat_id, f.error
                                )
                            })
                            .collect::<Vec<String>>()
                            .join("\n");

                        out.push_str(&errors);
                    }
                    out
                }
            }
            GetMessages { messages } => {
                let mut tbl = tabled::Table::new(messages);
                tbl.with(Style::psql());
                tbl.to_string()
            }
            _ => todo!(),
        }
    }

    async fn connect_sock() -> Result<UnixStream, ClientError> {
        let sock_path = crate::daemon::Daemon::socket_path();
        UnixStream::connect(&sock_path)
            .await
            .map_err(ClientError::ConnectionFailed)
    }

    async fn write_payload(
        writer: &mut OwnedWriteHalf,
        protocol: CommandProtocol,
    ) -> Result<(), ClientError> {
        let payload = protocol.to_serialized();

        writer
            .write_all(payload.as_bytes())
            .await
            .map_err(ClientError::WriteFailure)
    }

    async fn read_response(reader: &mut OwnedReadHalf) -> Result<ResponseProtocol, ClientError> {
        let mut reader = BufReader::new(reader);
        let mut response = String::new();
        match reader.read_line(&mut response).await {
            Ok(1..) => {
                let response: ResponseProtocol = serde_json::from_str(&response)?;
                Ok(response)
            }
            Ok(0) => Err(ClientError::NoResponse),
            Err(e) => Err(ClientError::ReadFailure(e)),
        }
    }
}
