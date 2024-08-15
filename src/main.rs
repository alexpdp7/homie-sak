use std::{collections::HashMap, str::FromStr};

use clap::{Parser, Subcommand};
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
}

fn main() {
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
        }
    }
}
