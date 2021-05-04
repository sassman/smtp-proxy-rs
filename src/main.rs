mod proxy;

use clap::{crate_authors, crate_description, crate_version, App, AppSettings, Arg};
use log::{debug, error, info};

use anyhow::Result;
use tokio::net::TcpListener;

use crate::proxy::proxy_to_remote;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let matches = App::new("smtp-proxy")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .takes_value(false)
                .required(false)
                .help("enables more verbose logging"),
        )
        .arg(
            Arg::with_name("local-port")
                .short("p")
                .long("port")
                .takes_value(true)
                .required(true)
                .default_value("2525")
                .help("local port where the proxy will listening"),
        )
        .arg(
            Arg::with_name("smtp-server")
                .short("s")
                .long("server")
                .takes_value(true)
                .required(true)
                .help("remote smtp server address (name or ip)"),
        )
        .arg(
            Arg::with_name("smtp-server-port")
                .short("P")
                .long("server-port")
                .takes_value(true)
                .required(true)
                .default_value("25")
                .help("remote smtp server port"),
        )
        .get_matches();

    std::env::set_var(
        "RUST_LOG",
        if matches.is_present("verbose") {
            "debug"
        } else {
            "info"
        },
    );
    env_logger::init();

    let remote_addr = format!(
        "{}:{}",
        matches.value_of("smtp-server").unwrap(),
        matches.value_of("smtp-server-port").unwrap()
    );
    debug!("remote: {}", remote_addr);

    let local_addr = format!("0.0.0.0:{}", matches.value_of("local-port").unwrap());
    let listener = TcpListener::bind(&local_addr).await?;
    info!("listening on: {:?}", local_addr);
    loop {
        let (stream, _) = listener.accept().await?;
        let remote_addr = remote_addr.clone();
        tokio::spawn(async move {
            match proxy_to_remote(stream, remote_addr.as_str()).await {
                Ok(_) => info!("client disconnected"),
                Err(e) => error!("{}", e),
            }
        });
    }
}
