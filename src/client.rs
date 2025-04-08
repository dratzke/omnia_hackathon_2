mod client_cam;
mod client_grpc_server;
mod client_grpc_service;
mod config;
mod player;
mod player_input;
mod protocol;
mod track_gen;
mod track_mesh;
mod world;

use std::{net::SocketAddr, sync::Arc};

use bevy::prelude::*;
use clap::Parser;
use client::{Authentication, ClientCommands, ClientPlugins, IoConfig, NetConfig};
use client_cam::{ClientCameraPlugin, DirectionalCamera};
use client_grpc_server::start_gprc_server;
use config::shared_config;
use lightyear::{connection::netcode::CONNECT_TOKEN_BYTES, prelude::*};
use player::PlayerPlugin;
use player_input::PlayerInputPlugin;
use protocol::{Inputs, ProtocolPlugin};
use tokio::sync::Mutex;
use world::WorldPlugin;

#[derive(Parser)]
struct ClientArgs {
    #[clap(long)]
    server: String,
    #[clap(long)]
    auth_port: u16,
    #[clap(long)]
    client_port: u16,
    #[clap(long)]
    grpc_port: Option<u16>,
}

pub fn main() {
    let args = ClientArgs::parse();
    let host = args.server;
    let auth_port = args.auth_port;
    let client_port: u16 = args.client_port;
    let screen_mutex = Arc::new(Mutex::new(vec![]));
    let current_input_mutex = Arc::new(Mutex::new(Inputs::None));

    let server_thread = if let Some(grpc_port) = args.grpc_port {
        start_gprc_server(screen_mutex.clone(), current_input_mutex.clone(), grpc_port)
    } else {
        std::thread::spawn(|| {})
    };

    let mut app = App::new();
    app.add_plugins(MyClientPlugin {
        auth_addr: format!("{host}:{auth_port}").parse().unwrap(),
        client_addr: format!("0.0.0.0:{client_port}").parse().unwrap(),
        screen: screen_mutex,
        current_input: current_input_mutex,
        grpc: args.grpc_port.is_some(),
    });
    app.run();
    server_thread.join().unwrap();
}

struct MyClientPlugin {
    auth_addr: SocketAddr,
    client_addr: SocketAddr,

    grpc: bool,
    screen: Arc<Mutex<Vec<u8>>>,
    current_input: Arc<Mutex<Inputs>>,
}

#[derive(Resource)]
struct ControlViaGrpc {
    screen: Arc<Mutex<Vec<u8>>>,
    current_input: Arc<Mutex<Inputs>>,
    enabled: bool,
}

impl Plugin for MyClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins);
        app.insert_resource(ControlViaGrpc {
            screen: self.screen.clone(),
            current_input: self.current_input.clone(),
            enabled: self.grpc,
        });
        app.add_plugins(build_client_plugin(self.auth_addr, self.client_addr));
        app.add_plugins(ProtocolPlugin);
        app.add_plugins(PlayerInputPlugin);
        app.add_plugins(WorldPlugin { physics: false });
        app.add_plugins(ClientCameraPlugin);

        app.add_plugins(PlayerPlugin { physics: false });
        app.add_systems(Startup, connect_client);
    }
}

fn connect_client(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalCamera::default(),
    ));
    commands.connect_client();
}

fn build_client_plugin(auth_addr: SocketAddr, client_addr: SocketAddr) -> ClientPlugins {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let connect_token = rt.block_on(get_connect_token(auth_addr));
    let auth = Authentication::Token(connect_token);
    let io = IoConfig {
        transport: client::ClientTransport::UdpSocket(client_addr),
        ..Default::default()
    };
    let net_config = NetConfig::Netcode {
        auth,
        config: client::NetcodeConfig::default(),
        io,
    };
    let config = client::ClientConfig {
        shared: shared_config(),
        net: net_config,
        ..Default::default()
    };
    dbg!("build client");
    ClientPlugins::new(config)
}

async fn get_connect_token(auth_addr: SocketAddr) -> ConnectToken {
    let stream = tokio::net::TcpStream::connect(auth_addr).await.unwrap();
    stream.readable().await.unwrap();
    let mut buffer = [0u8; CONNECT_TOKEN_BYTES];
    stream.try_read(&mut buffer).unwrap();
    ConnectToken::try_from_bytes(&buffer).unwrap()
}
