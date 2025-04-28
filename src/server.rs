mod config;
mod player;
mod protocol;
mod server_cam;
mod server_input;
mod track_gen;
mod track_mesh;
mod world;

use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{Arc, RwLock},
};

use async_compat::Compat;
use bevy::{
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    prelude::*,
    tasks::IoTaskPool,
    window::{CursorGrabMode, WindowResolution},
};
use bevy_rapier3d::prelude::*;
use clap::Parser;
use config::shared_config;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use lightyear::server::events::{ConnectEvent, DisconnectEvent};
use lightyear::{connection::netcode::PRIVATE_KEY_BYTES, prelude::ClientId::Netcode};
use player::{PlayerBundle, PlayerPlugin, SpawnedPlayersCount};
use protocol::{PlayerColor, PlayerPosition, ProtocolPlugin, VelocityShare};
use rand::{TryRngCore, rngs::OsRng};
use server::{IoConfig, NetConfig, NetcodeConfig, ServerCommands, ServerConfig, ServerPlugins};
use server_cam::{CameraController, CameraControllerPlugin};
use server_input::ServerInputPlugin;
use tokio::io::AsyncWriteExt;
use world::{LowGpu, Seed, WorldPlugin};

#[derive(Parser)]
struct ServerArgs {
    /// port number used for authenticating clients to the server. If you run multiple servers concurrently each one needs a unqiue port number.
    #[clap(long, default_value_t = 4000)]
    auth_port: u16,
    /// port number used for the game. If you run multiple servers concurrently each one needs a unique port number.
    #[clap(long, default_value_t = 5000)]
    game_port: u16,
    /// Number of players expected to join. The game will start to run once the expected number of players have joined.
    #[clap(long, default_value_t = 1)]
    players: u8,
    /// Number of seconds the game will last at max. Once either every player has reached the finish line or this time has been reached, the rankings within the match will be calculated.
    #[clap(long, default_value_t = 120)]
    max_game_seconds: u32,
    /// The seed for the game world. Needs to match between server and client to ensure that both have the same view of the world.
    #[clap(long, default_value_t = 1234)]
    seed: u32,
    #[clap(long)]
    /// Disables the physically based rendering materials to lower the gpu resource consumption. (This also disables the transparency of the ice road)
    low_gpu: bool,
    /// Avoids drawing the game.
    #[clap(long)]
    headless: bool,
    /// Server ip addes. Only required for allowing remote clients to connect to the server. Should match the ip address of your machine in the local network
    #[clap(long)]
    server_ip: Option<String>,
}

pub fn main() {
    let args = ServerArgs::parse();
    let auth_port = args.auth_port;
    let game_port = args.game_port;
    let key = get_key();

    let game_server_addr = match args.server_ip {
        Some(a) => format!("{a}:{game_port}").parse().unwrap(),
        None => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, game_port)),
    };

    let server_plugin = ServerPlugin {
        protocol_id: 0,
        private_key: key,
        game_server_addr,
        auth_server_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, auth_port)),
        player_count: args.players,
        max_game_seconds: args.max_game_seconds,
        seed: args.seed,
        low_gpu: args.low_gpu,
        headless: args.headless,
    };

    let mut app = App::new();
    app.add_plugins(server_plugin);
    app.run();
    dbg!("closing");
}

fn get_key() -> [u8; PRIVATE_KEY_BYTES] {
    let mut b = [0u8; 32];
    OsRng.try_fill_bytes(&mut b).unwrap();
    b
}

struct ServerPlugin {
    protocol_id: u64,
    private_key: Key,
    game_server_addr: SocketAddr,
    auth_server_addr: SocketAddr,
    player_count: u8,
    max_game_seconds: u32,
    seed: u32,
    low_gpu: bool,
    headless: bool,
}
#[derive(Resource)]
struct ClientIds(Arc<RwLock<HashMap<u64, Entity>>>);

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        let client_ids = Arc::new(RwLock::new(HashMap::<u64, Entity>::new()));
        if self.headless {
            app.add_plugins(DefaultPlugins.set(WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                close_when_requested: false,
                ..default()
            }));
        } else {
            app.add_plugins(DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1280.0, 720.0),
                    title: "server".into(),
                    ..default() // [1][5]
                }),
                ..default()
            }));
        }

        app.add_plugins(build_server_plugin(
            self.game_server_addr.port(),
            self.private_key,
        ));
        app.insert_resource(Seed(self.seed));
        app.insert_resource(LowGpu(self.low_gpu));
        app.add_plugins(ProtocolPlugin);
        app.add_plugins(PlayerPlugin {
            physics: true,
            player_count: self.player_count,
            max_game_seconds: self.max_game_seconds,
        });
        app.add_plugins(ServerInputPlugin);
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        app.add_plugins(WorldPlugin { physics: true });
        if !self.headless {
            app.add_plugins(CameraControllerPlugin);
        }

        if self.headless {
            app.add_systems(Startup, start_server_headless);
        } else {
            app.add_systems(Startup, start_server);
        }
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

fn start_server(mut commands: Commands, mut windows: Query<&mut Window>) {
    commands.start_server();
    commands.spawn((
        CameraController,
        Camera3d::default(),
        Camera {
            hdr: true,
            ..Default::default()
        },
        Bloom::NATURAL,
        Tonemapping::TonyMcMapface,
        Transform::from_xyz(0.0, 10.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let mut window = windows.single_mut();
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    window.cursor_options.visible = false;
}

fn start_server_headless(mut commands: Commands, mut windows: Query<&mut Window>) {
    commands.start_server();
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
    mut player_count: ResMut<SpawnedPlayersCount>,
) {
    if let Netcode(client_id) = trigger.event().client_id {
        let pos = Vec3::new(
            distribute_space(player_count.max, player_count.current),
            9.0,
            4.0,
        );
        info!("client logged in");
        let entity = commands
            .spawn(PlayerBundle {
                position: PlayerPosition(pos, Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 0.0)),
                color: PlayerColor(Color::oklab(0.50, -0.03, -0.09)),
            })
            .insert(VelocityShare {
                linear: Vec3::ZERO,
                angular: Vec3::ZERO,
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
        player_count.current += 1;
        client_ids.0.write().unwrap().insert(client_id, entity);
    }
}

fn distribute_space(max: u8, i: u8) -> f32 {
    let range_start = -4.5;
    let range_end = 4.5;
    let range_width = range_end - range_start;

    // Calculate the width of each subdivision
    let subdivision_width = range_width / (max as f32);

    // Calculate the i-th point (center of the i-th subdivision)
    let point = range_start + (i as f32 + 0.5) * subdivision_width;

    point
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
