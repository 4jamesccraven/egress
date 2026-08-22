use crate::cli::ClientCLI;
use crate::error::ResultExt;
use crate::protocol::{CommandProtocol, ResponseProtocol};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};

pub type ClientError = ();

pub struct Client;

impl Client {
    pub async fn run(args: &ClientCLI) -> Result<(), ClientError> {
        let socket = Self::connect_sock().await?;
        let (mut reader, mut writer) = socket.into_split();

        let protocol: CommandProtocol = args.command.clone().into();

        Self::write_payload(&mut writer, protocol).await?;
        let response = Self::read_response(&mut reader).await?;

        println!("{}", response);

        Ok(())
    }

    async fn connect_sock() -> Result<UnixStream, ClientError> {
        let sock_path = crate::daemon::Daemon::socket_path();
        UnixStream::connect(&sock_path).await.map_err(|_| ())
    }

    async fn write_payload(
        writer: &mut OwnedWriteHalf,
        protocol: CommandProtocol,
    ) -> Result<(), ClientError> {
        let mut payload = serde_json::to_string(&protocol).map_err(|_| ())?;
        payload.push_str("\n");

        writer.write_all(payload.as_bytes()).await.map_err(|_| ())
    }

    async fn read_response(reader: &mut OwnedReadHalf) -> Result<String, ClientError> {
        let mut reader = BufReader::new(reader);
        let mut response = String::new();
        reader.read_line(&mut response).await.map_err(|_| ())?;

        let response: ResponseProtocol = serde_json::from_str(&response)
            .responsible_expect("could not deserialise response from daemon");

        Ok(response.text)
    }
}
