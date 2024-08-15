use std::{collections::HashMap, str::FromStr};

use clap::{Parser, Subcommand};
use homie_device::{ColorFormat, HomieDevice, Node, Property};
use rumqttc::MqttOptions;
use rumqttd::{Broker, Config, ConnectionSettings, ServerSettings};

#[derive(Parser)]
#[command()]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Serve,
    Device,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Serve => {
            println!("serve");
            Broker::new(Config {
                v5: Some(HashMap::from([(
                    "foo".to_owned(),
                    ServerSettings {
                        name: "foo".to_owned(),
                        listen: std::net::SocketAddr::from_str("127.0.0.1:8000").unwrap(),
                        tls: None,
                        next_connection_delay_ms: 1,
                        connections: ConnectionSettings {
                            connection_timeout_ms: 5000,
                            max_payload_size: 20480,
                            max_inflight_count: 500,
                            auth: None,
                            external_auth: None,
                            dynamic_filters: false,
                        },
                    },
                )])),
                ..Default::default()
            })
            .start()
            .unwrap();
            Ok(())
        }
        Commands::Device => {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(device())?;
            Ok(())
        }
    }
}

async fn device() -> anyhow::Result<()> {
    println!("device");
    let mqtt_options = MqttOptions::new("device", "127.0.0.1", 8000);
    let builder = HomieDevice::builder("homie/device", "Device", mqtt_options);
    let (mut homie, homie_handle) = builder.spawn().await?;
    let node = Node::new(
        "light",
        "Light",
        "light",
        vec![
            Property::boolean("power", "On", true, true, None),
            Property::color("colour", "Colour", true, true, None, ColorFormat::Rgb),
        ],
    );
    homie.add_node(node).await?;
    homie.ready().await?;
    println!("ready");
    homie_handle.await?;
    Ok(())
}
