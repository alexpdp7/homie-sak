use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use clap::{Parser, Subcommand};
use homie_device::{HomieDevice, Node, Property as HomieDeviceProperty};
use rumqttc::MqttOptions;
use rumqttd::{Broker, Config, ConnectionSettings, RouterConfig, ServerSettings};
use serde::Deserialize;

#[derive(Parser)]
#[command()]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Serve,
    Device {
        configuration_path: PathBuf,
        host: String,
        mqtt_host: String,
        mqtt_port: u16,
    },
}

fn main() -> anyhow::Result<()> {
    let builder = tracing_subscriber::fmt()
        .pretty()
        .with_line_number(false)
        .with_file(false)
        .with_thread_ids(false)
        .with_thread_names(false);

    builder
        .try_init()
        .expect("initialized subscriber succesfully");

    let cli = Cli::parse();

    match &cli.command {
        Commands::Serve => {
            println!("serve");
            Broker::new(Config {
                v4: Some(HashMap::from([(
                    "v4".to_owned(),
                    ServerSettings {
                        name: "v4".to_owned(),
                        listen: std::net::SocketAddr::from_str("0.0.0.0:8000").unwrap(),
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
                ws: Some(HashMap::from([(
                    "ws".to_owned(),
                    ServerSettings {
                        name: "ws".to_owned(),
                        listen: std::net::SocketAddr::from_str("0.0.0.0:8001").unwrap(),
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
                router: RouterConfig {
                    max_connections: 10010,
                    max_outgoing_packet_count: 200,
                    max_segment_size: 104856700,
                    max_segment_count: 10,
                    ..Default::default()
                },
                ..Default::default()
            })
            .start()
            .unwrap();
            Ok(())
        }
        Commands::Device {
            configuration_path,
            host,
            mqtt_host,
            mqtt_port,
        } => {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(device(configuration_path, host, mqtt_host, *mqtt_port))?;
            Ok(())
        }
    }
}

#[derive(Debug, Deserialize)]
struct Configurations {
    configs: HashMap<String, Host>,
}

#[derive(Clone, Debug, Deserialize)]
struct Host {
    devices: HashMap<String, Device>,
}

#[derive(Clone, Debug, Deserialize)]
struct Device {
    properties: HashMap<String, Property>,
}

#[derive(Clone, Debug, Deserialize)]
struct Property {
    datatype: Datatype,
    settable: bool,
    retained: bool,
    run_command: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
enum Datatype {
    Boolean,
}

async fn device(
    configuration_path: &PathBuf,
    host: &String,
    mqtt_host: &String,
    mqtt_port: u16,
) -> anyhow::Result<()> {
    let devices = Arc::new(
        serde_yaml::from_str::<Configurations>(
            &std::fs::read_to_string(configuration_path).unwrap(),
        )?
        .configs
        .get(host)
        .unwrap()
        .devices
        .clone(),
    );
    let mqtt_options = MqttOptions::new(host, mqtt_host, mqtt_port);
    let mut builder = HomieDevice::builder(&format!("homie/{host}"), host, mqtt_options);
    let inner_devices = devices.clone();
    builder.set_update_callback(move |node_id, property_id, value| {
        let devices = inner_devices.clone();
        async move {
            println!("node_id {node_id} property_id {property_id} value {value}");
            let command = &devices
                .get(&node_id)
                .unwrap()
                .properties
                .get(&property_id)
                .unwrap()
                .run_command;
            println!("executing {command:?}");
            let mut command = VecDeque::from(command.clone());
            std::process::Command::new(command.pop_front().unwrap())
                .args(command)
                .status()
                .unwrap();
            Some("hi".to_owned())
        }
    });
    let (mut homie, homie_handle) = builder.spawn().await?;
    for (device_name, device) in devices.clone().iter() {
        let properties = device
            .properties
            .iter()
            .map(|(property_name, property)| match property.datatype {
                Datatype::Boolean => HomieDeviceProperty::boolean(
                    property_name,
                    property_name,
                    property.settable,
                    property.retained,
                    None,
                ),
            })
            .collect();
        let node = Node::new(device_name, device_name, device_name, properties);
        homie.add_node(node).await?;
    }
    homie.ready().await?;
    println!("ready");
    homie_handle.await?;
    Ok(())
}
