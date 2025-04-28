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

use bevy_image_export::{GpuImageExportSource, ImageExport, ImageExportPlugin, ImageExportSource};
use lightyear::{
    prelude::client::{Predicted, Replicate},
    shared::replication::components::Controlled,
};
use std::{net::SocketAddr, sync::Arc, u32};

use bevy::{
    asset::RenderAssetUsages,
    log::LogPlugin,
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        render_asset::RenderAssets,
        render_resource::{Extent3d, Maintain, MapMode, TextureUsages},
        renderer::RenderDevice,
    },
    tasks::futures_lite,
    window::WindowResolution,
};
use clap::Parser;
use client::{Authentication, ClientCommands, ClientPlugins, IoConfig, NetConfig};
use client_cam::{ClientCameraPlugin, DirectionalCamera};
use client_grpc_server::marble::ResultEntry;
use client_grpc_server::start_gprc_server;
use config::shared_config;
use lightyear::{connection::netcode::CONNECT_TOKEN_BYTES, prelude::*};
use player::PlayerPlugin;
use player_input::PlayerInputPlugin;
use protocol::{GameResult, Inputs, PlayerName, ProtocolPlugin, VelocityShare};
use tokio::sync::{Mutex, oneshot};
use world::{LowGpu, Seed, WorldPlugin};

#[derive(Parser)]
struct ClientArgs {
    /// Ip address of the game server.
    #[clap(long, default_value_t = format!("127.0.0.1"))]
    server: String,
    /// Authentication port of the game server.
    #[clap(long, default_value_t = 4000)]
    auth_port: u16,
    /// Port used by the client for the bidirectional communication. Needs to be unqiue.
    #[clap(long, default_value_t = 5001)]
    client_port: u16,
    /// Port used to start a grpc server and remote control this client.
    #[clap(long)]
    grpc_port: Option<u16>,
    /// Player name chosen for the game.
    #[clap(long, default_value_t = format!("Player1"))]
    name: String,
    /// The seed for the game world. Needs to match between server and client to ensure that both have the same view of the world.
    #[clap(long, default_value_t = 1234)]
    seed: u32,
    /// Disables the physically based rendering materials to lower the gpu resource consumption. (This also disables the transparency of the ice road)
    #[clap(long)]
    low_gpu: bool,

    /// Verbose logging
    #[clap(long)]
    verbose: bool,
}

pub fn main() {
    let args = ClientArgs::parse();
    let host = args.server;
    let auth_port = args.auth_port;
    let client_port: u16 = args.client_port;
    let screen_mutex = Arc::new(Mutex::new(vec![]));
    let current_input_mutex = Arc::new(Mutex::new(Inputs::None));
    let finished = Arc::new(Mutex::new(false));
    let linear_velocity = Arc::new(Mutex::new(Vec3::ZERO));
    let angular_velocity = Arc::new(Mutex::new(Vec3::ZERO));
    let results = Arc::new(Mutex::new(Vec::new()));

    let _ = if let Some(grpc_port) = args.grpc_port {
        start_gprc_server(
            screen_mutex.clone(),
            current_input_mutex.clone(),
            finished.clone(),
            linear_velocity.clone(),
            angular_velocity.clone(),
            results.clone(),
            grpc_port,
        )
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
        name: args.name,
        finished,
        linear_velocity,
        angular_velocity,
        results,
        seed: args.seed,
        low_gpu: args.low_gpu,
        verbose: args.verbose,
    });
    app.run();
    // server_thread.join().unwrap();
}

#[derive(Resource)]
pub struct MyPlayerName(pub String, pub bool);
struct MyClientPlugin {
    auth_addr: SocketAddr,
    client_addr: SocketAddr,

    grpc: bool,
    screen: Arc<Mutex<Vec<u8>>>,
    current_input: Arc<Mutex<Inputs>>,
    finished: Arc<Mutex<bool>>,
    linear_velocity: Arc<Mutex<bevy::math::Vec3>>,
    angular_velocity: Arc<Mutex<bevy::math::Vec3>>,
    results: Arc<Mutex<Vec<ResultEntry>>>,

    name: String,
    seed: u32,
    low_gpu: bool,

    verbose: bool,
}

#[derive(Resource)]
struct ControlViaGrpc {
    screen: Arc<Mutex<Vec<u8>>>,
    current_input: Arc<Mutex<Inputs>>,
    finished: Arc<Mutex<bool>>,
    linear_velocity: Arc<Mutex<bevy::math::Vec3>>,
    angular_velocity: Arc<Mutex<bevy::math::Vec3>>,
    results: Arc<Mutex<Vec<ResultEntry>>>,
    enabled: bool,
}

impl Plugin for MyClientPlugin {
    fn build(&self, app: &mut App) {
        if self.grpc {
            if self.verbose {
                app.add_plugins(
                    DefaultPlugins
                        .set(WindowPlugin {
                            primary_window: None,
                            exit_condition: bevy::window::ExitCondition::DontExit,
                            close_when_requested: false,
                            ..default()
                        })
                        .set(LogPlugin {
                            // Uncomment this to override the default log settings:
                            level: bevy::log::Level::TRACE,
                            // filter: "wgpu=warn,bevy_ecs=info".to_string(),
                            ..default()
                        }),
                );
            } else {
                app.add_plugins(DefaultPlugins.set(WindowPlugin {
                    primary_window: None,
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    close_when_requested: false,
                    ..default()
                }));
            }
        } else {
            app.add_plugins(DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1280.0, 720.0),
                    title: "client".into(),
                    ..default() // [1][5]
                }),
                ..default()
            }));
        }

        app.add_plugins(ImageExportPlugin::default());
        app.insert_resource(MyPlayerName(self.name.clone(), false));
        app.insert_resource(Seed(self.seed));
        app.insert_resource(LowGpu(self.low_gpu));
        app.add_systems(
            Update,
            (attach_name, sync_finished_grpc, sync_velocities_grpc),
        );
        app.insert_resource(ControlViaGrpc {
            screen: self.screen.clone(),
            current_input: self.current_input.clone(),
            enabled: self.grpc,
            finished: self.finished.clone(),
            linear_velocity: self.linear_velocity.clone(),
            angular_velocity: self.angular_velocity.clone(),
            results: self.results.clone(),
        });
        let render_app = app.sub_app_mut(RenderApp);

        render_app.insert_resource(ControlViaGrpc {
            screen: self.screen.clone(),
            current_input: self.current_input.clone(),
            enabled: self.grpc,
            finished: self.finished.clone(),
            linear_velocity: self.linear_velocity.clone(),
            angular_velocity: self.angular_velocity.clone(),
            results: self.results.clone(),
        });
        render_app.add_systems(
            Render,
            sync_screen_grpc
                .after(RenderSet::Render)
                .before(RenderSet::Cleanup),
        );
        app.add_plugins(build_client_plugin(self.auth_addr, self.client_addr));
        app.add_plugins(ProtocolPlugin);
        app.add_plugins(PlayerInputPlugin);
        app.add_plugins(WorldPlugin { physics: false });
        app.add_plugins(ClientCameraPlugin);

        app.add_plugins(PlayerPlugin {
            physics: false,
            player_count: 0,
            max_game_seconds: u32::MAX,
        });
        app.add_systems(Startup, connect_client);
    }
}

fn connect_client(
    mut commands: Commands,
    grpc: Res<ControlViaGrpc>,
    mut images: ResMut<Assets<Image>>,
    mut export_sources: ResMut<Assets<ImageExportSource>>,
) {
    if grpc.enabled {
        let size = Extent3d {
            width: 1280,
            height: 720,
            ..Default::default()
        };
        let mut image = Image::new_fill(
            size,
            bevy::render::render_resource::TextureDimension::D2,
            &[0, 0, 0, 0],
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST
            | TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::COPY_SRC;
        let image_handle = images.add(image);

        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
            DirectionalCamera::default(),
            Camera {
                target: bevy::render::camera::RenderTarget::Image(image_handle.clone()),
                clear_color: ClearColorConfig::Custom(Color::WHITE),
                ..Default::default()
            },
        ));
        commands.spawn(ImageExport(export_sources.add(image_handle)));
    } else {
        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
            DirectionalCamera::default(),
        ));
    }
    commands.connect_client();
}

fn sync_velocities_grpc(
    grpc: Res<ControlViaGrpc>,
    player_q: Query<&VelocityShare, (With<Controlled>, Without<Predicted>)>,
) {
    let mut c = 0;
    for player in player_q.iter() {
        futures_lite::future::block_on(async {
            let mut lin = grpc.linear_velocity.lock().await;
            *lin = player.linear;
        });

        futures_lite::future::block_on(async {
            let mut ang = grpc.angular_velocity.lock().await;
            *ang = player.angular;
        });
        c += 1;
    }
    if c > 1 {
        panic!()
    }
}

fn sync_finished_grpc(grpc: Res<ControlViaGrpc>, finished: Query<&GameResult>) {
    let mut c = 0;
    for r in finished.iter() {
        c += 1;
        futures_lite::future::block_on(async {
            let mut finished = grpc.finished.lock().await;
            if !*finished {
                *finished = true;
            }
        });

        let results: Vec<_> = r
            .players
            .iter()
            .map(|p| ResultEntry {
                name: p.0.clone(),
                finish_time: match p.1 {
                    protocol::Finish::Time(t) => Some(t),
                    protocol::Finish::TrackProgress(_, _) => None,
                },
                last_touched_road_id: match p.1 {
                    protocol::Finish::Time(_) => None,
                    protocol::Finish::TrackProgress(i, _) => Some(i as u64),
                },
                last_touched_road_time: match p.1 {
                    protocol::Finish::Time(_) => None,
                    protocol::Finish::TrackProgress(_, t) => Some(t),
                },
            })
            .collect();

        futures_lite::future::block_on(async {
            let mut results_lock = grpc.results.lock().await;
            *results_lock = results;
        });
    }
    if c > 1 {
        panic!()
    }
}

fn sync_screen_grpc(
    grpc: Res<ControlViaGrpc>,
    export_bundles: Query<&ImageExport>,
    sources: Res<RenderAssets<GpuImageExportSource>>,
    render_device: Res<RenderDevice>,
) {
    for export in &export_bundles {
        if let Some(gpu_source) = sources.get(&export.0) {
            let mut image_bytes = {
                let slice = gpu_source.buffer.slice(..);

                {
                    let (mapping_tx, mapping_rx) = oneshot::channel();

                    render_device.map_buffer(&slice, MapMode::Read, move |res| {
                        mapping_tx.send(res).unwrap();
                    });

                    render_device.poll(Maintain::Wait);
                    futures_lite::future::block_on(mapping_rx).unwrap().unwrap();
                }

                slice.get_mapped_range().to_vec()
            };

            gpu_source.buffer.unmap();

            let bytes_per_row = gpu_source.bytes_per_row as usize;
            let padded_bytes_per_row = gpu_source.padded_bytes_per_row as usize;
            let source_size = gpu_source.source_size;

            if bytes_per_row != padded_bytes_per_row {
                let mut unpadded_bytes =
                    Vec::<u8>::with_capacity(source_size.height as usize * bytes_per_row);

                for padded_row in image_bytes.chunks(padded_bytes_per_row) {
                    unpadded_bytes.extend_from_slice(&padded_row[..bytes_per_row]);
                }

                image_bytes = unpadded_bytes;
            }
            futures_lite::future::block_on(async {
                let mut l = grpc.screen.lock().await;
                *l = image_bytes
            });
        }
    }
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

fn attach_name(mut my_name: ResMut<MyPlayerName>, mut commands: Commands) {
    if !my_name.1 {
        commands.spawn((
            PlayerName(my_name.0.clone()),
            Replicate {
                target: client::ReplicateToServer,
                authority: HasAuthority,
                replicating: Replicating,
                ..Default::default()
            },
        ));
        my_name.1 = true;
    }
}
