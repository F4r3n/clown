pub mod client;
pub mod command;
pub mod conn;
pub mod message;
pub mod outgoing;

#[cfg(test)]
mod tests {
    use tokio::io::AsyncReadExt;
    use tokio::join;

    use crate::client;
    use crate::conn::Connection;
    use crate::conn::ConnectionConfig;
    use crate::outgoing::Outgoing;

    #[tokio::test]
    async fn test_connect() -> anyhow::Result<()> {
        let option = ConnectionConfig {
            address: "chat.freenode.net".into(),
            nickname: "farine".into(),
            password: None,
            port: 6697,
            real_name: "farine".into(),
            username: "farine".into(),
        };
        let conn = Connection::new(option);
        let client = client::Client::new();
        let tls_stream = conn.connect_tls().await?;
        let state = client.state();
        let sender = client.sender();

        let handle = client.spawn(tls_stream);
        // Send commands after delay
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if let Some(tx) = &sender.inner {
            tx.send("NICK mybot\r\n".to_string())?;
            tx.send("USER mybot 0 * :Rust Bot\r\n".to_string())?;
        }
        Ok(())
    }
}
