use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct ConnectionOption {
    address : String,
    port : i32,
    nickname : String,
    real_name: String,
    username: String,
    password : String
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

async fn establish_stream(in_address : &str, port : i32) -> Result<TcpStream, anyhow::Error> {
    let stream = TcpStream::connect(format!("{}:{}", in_address, port)).await?;
    Ok(stream)
}

async fn capabilities_negotiation(reader : &mut OwnedReadHalf, writer : &mut OwnedWriteHalf)-> Result<(), anyhow::Error> {
    let mut buffer = String::new();
    reader.read_to_string(&mut buffer);
    Ok(())
}

pub async fn connect(option : ConnectionOption) -> Result<(), anyhow::Error> {
    let mut stream = establish_stream(&option.address, option.port).await?;
    let (mut reader, mut writer) = stream.into_split();
    writer.write_all("CAP LS 302\r\n".as_bytes()).await?;
    if !option.password.is_empty() {
        writer.write_all(format!("PASS {}\r\n", option.password).as_bytes()).await?;
    }
    writer.write_all(format!("NICK {}\r\n", option.nickname).as_bytes()).await?;
    writer.write_all(format!("USER {} 0 * {}\r\n", option.username, option.real_name).as_bytes()).await?;
    capabilities_negotiation(&mut reader, &mut writer);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
