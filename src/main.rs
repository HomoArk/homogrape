use anyhow::Result;
use dashmap::DashSet;
use homogrape::{download_profile_photo, get_chats_map, get_me, load_chats_with_offset, send_message};
use std::collections::HashMap;
use std::fs::OpenOptions;
use tokio;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<()> {
    let debug_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("main.log")?;

    tracing_subscriber::registry()
        // feel free to choose desired layers
        // NOTICE: stdout layer may stuck your terminal
        // also stdout may slow down the execution
        // and thus prevent the deadlock, at least on my PC :(
        .with(console_subscriber::spawn())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(debug_file)
                .with_ansi(false)
        )
        // .with(tracing_subscriber::fmt::layer().pretty())
        .init();

    let me = get_me().await?;
    info!("Me: {:?}", me);

    tokio::spawn(high_pressure_load());

    let _ = homogrape::run().await?;

    loop {
        // wait for command line input
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        match input.trim() {
            "q" => break Ok(()),
            "m" => {
                info!("Me: {:?}", me);
            }
            "s" => {
                send_message(me.chat_id, "Hello from Homogrape!".to_string(), None).await?;
            }
            _ => {
                info!("Unknown command: {}", input);
            }
        }
    }
}

async fn high_pressure_load() {
    load_chats_with_offset(HashMap::new()).await.expect("load chats failed");
    let chats_map = get_chats_map().await;

    for chat_id in chats_map.iter() {
        tokio::spawn({
            info!("Downloading profile photo for chat_id: {}", chat_id.key());
            download_profile_photo(*chat_id.key())
            // By calling this, I just want to simulate the real scenario on my device
            // the core logic is to invoke unpack_chat
            // and no photo will be downloaded if it's already downloaded
            // so don't worry about your disk space :)
        });
    }
}