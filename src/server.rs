mod config;
mod player;
mod protocol;
mod server_input;
mod world;

use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{Arc, RwLock},
};

use async_compat::Compat;
use bevy::{prelude::*, tasks::IoTaskPool};
use bevy_rapier3d::prelude::*;
use clap::Parser;
use config::shared_config;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use lightyear::server::events::{ConnectEvent, DisconnectEvent};
use lightyear::{connection::netcode::PRIVATE_KEY_BYTES, prelude::ClientId::Netcode};
use player::{PlayerBundle, PlayerPlugin};
use protocol::{PlayerColor, PlayerPosition, ProtocolPlugin};
use rand::{TryRngCore, rngs::OsRng};
use server::{IoConfig, NetConfig, NetcodeConfig, ServerCommands, ServerConfig, ServerPlugins};
use server_input::ServerInputPlugin;
use tokio::io::AsyncWriteExt;
use world::WorldPlugin;

#[derive(Parser)]
struct ServerArgs {
    #[clap(long)]
    auth_port: u16,
    #[clap(long)]
    game_port: u16,
}

pub fn main() {
    let args = ServerArgs::parse();
    let auth_port = args.auth_port;
    let game_port = args.game_port;
    let key = get_key();

    let server_plugin = ServerPlugin {
        protocol_id: 0,
        private_key: key,
        game_server_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, game_port)),
        auth_server_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, auth_port)),
    };

    let mut app = App::new();
    app.add_plugins(server_plugin);
    app.run();
}

fn get_key() -> [u8; PRIVATE_KEY_BYTES] {
    let mut b = [0u8; 32];
    OsRng.try_fill_bytes(&mut b).unwrap();
    b
}

pub struct ServerPlugin {
    pub protocol_id: u64,
    pub private_key: Key,
    pub game_server_addr: SocketAddr,
    pub auth_server_addr: SocketAddr,
}
#[derive(Resource)]
struct ClientIds(Arc<RwLock<HashMap<u64, Entity>>>);

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        let client_ids = Arc::new(RwLock::new(HashMap::<u64, Entity>::new()));
        app.add_plugins(DefaultPlugins);
        app.add_plugins(build_server_plugin(
            self.game_server_addr.port(),
            self.private_key,
        ));
        app.add_plugins(ProtocolPlugin);
        app.add_plugins(PlayerPlugin { physics: true });
        app.add_plugins(ServerInputPlugin);
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        app.add_plugins(WorldPlugin { physics: true });

        app.add_systems(Startup, start_server);
        app.insert_resource(ClientIds(client_ids.clone()));

        app.add_observer(handle_disconnect_event);
        app.add_observer(handle_connect_event);
        start_netcode_authentication_task(
            self.game_server_addr,
            self.auth_server_addr,
            self.protocol_id,
            self.private_key,
            client_ids.clone(),
        );
    }
}

fn build_server_plugin(game_server_addr: u16, key: Key) -> ServerPlugins {
    let io = IoConfig {
        transport: server::ServerTransport::UdpSocket(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            game_server_addr,
        ))),
        ..Default::default()
    };
    let net_config = NetConfig::Netcode {
        config: NetcodeConfig {
            private_key: key,
            ..Default::default()
        },
        io,
    };
    let config = ServerConfig {
        net: vec![net_config],
        shared: shared_config(),
        replication: ReplicationConfig {
            send_interval: shared_config().server_replication_send_interval,
            ..Default::default()
        },
        ..Default::default()
    };
    ServerPlugins::new(config)
}

fn start_server(mut commands: Commands) {
    commands.start_server();
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn handle_disconnect_event(trigger: Trigger<DisconnectEvent>, client_ids: Res<ClientIds>) {
    if let Netcode(client_id) = trigger.event().client_id {
        client_ids.0.write().unwrap().remove(&client_id);
    }
}

fn handle_connect_event(
    trigger: Trigger<ConnectEvent>,
    client_ids: Res<ClientIds>,
    mut commands: Commands,
) {
    if let Netcode(client_id) = trigger.event().client_id {
        info!("client logged in");
        let entity = commands
            .spawn(PlayerBundle {
                position: PlayerPosition(
                    Vec3::new(
                        rand::random_range(-5.0..5.0),
                        rand::random_range(-5.0..5.0),
                        rand::random_range(-5.0..5.0),
                    ),
                    Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 0.0),
                ),
                color: PlayerColor(Color::oklab(0.50, -0.03, -0.09)),
            })
            .insert(Replicate {
                sync: SyncTarget {
                    prediction: NetworkTarget::Single(Netcode(client_id)),
                    interpolation: NetworkTarget::AllExceptSingle(Netcode(client_id)),
                },
                controlled_by: ControlledBy {
                    target: NetworkTarget::Single(Netcode(client_id)),
                    ..Default::default()
                },
                ..Default::default()
            })
            .id();
        client_ids.0.write().unwrap().insert(client_id, entity);
    }
}

fn start_netcode_authentication_task(
    game_server_addr: SocketAddr,
    auth_server_addr: SocketAddr,
    protocol_id: u64,
    private_key: Key,
    client_ids: Arc<RwLock<HashMap<u64, Entity>>>,
) {
    IoTaskPool::get()
        .spawn(Compat::new(async move {
            info!(
                "Listening for ConnectToken requests on {}",
                auth_server_addr
            );
            let listener = tokio::net::TcpListener::bind(auth_server_addr)
                .await
                .unwrap();
            loop {
                // received a new connection
                let (mut stream, _) = listener.accept().await.unwrap();

                // assign a new client_id
                let client_id = loop {
                    let client_id = rand::random();
                    if !client_ids.read().unwrap().contains_key(&client_id) {
                        break client_id;
                    }
                };

                let token =
                    ConnectToken::build(game_server_addr, protocol_id, client_id, private_key)
                        .generate()
                        .expect("Failed to generate token");

                let serialized_token = token.try_into_bytes().expect("Failed to serialize token");
                trace!(
                    "Sending token {:?} to client {}. Token len: {}",
                    serialized_token,
                    client_id,
                    serialized_token.len()
                );
                stream
                    .write_all(&serialized_token)
                    .await
                    .expect("Failed to send token to client");
            }
        }))
        .detach();
}
