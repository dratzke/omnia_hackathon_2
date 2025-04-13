use std::collections::HashMap;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use lightyear::prelude::{Replicated, client::Predicted, server::ControlledBy, server::Replicate};

use crate::{
    protocol::{Finish, GameResult, PlayerColor, PlayerName, PlayerPosition, VelocityShare},
    world::{Finished, GravityModifier, LastTouched},
};

pub struct PlayerPlugin {
    pub physics: bool,
    pub player_count: u8,
    pub max_game_seconds: u32,
}

#[derive(Resource, Debug)]
pub struct GameEndCondition {
    pub physics_start_time: u32,
    pub max_game_seconds: u32,
    pub players_finished: u8,
    pub evaluated: bool,
    pub has_started: bool,
}

#[derive(Resource)]
pub struct SpawnedPlayersCount {
    pub current: u8,
    pub max: u8,
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        if self.physics {
            app.add_systems(Update, attach_player_model_server);
            app.add_systems(Update, game_end_system);
            app.add_systems(Update, sync_velocity_physics);
        } else {
            app.add_systems(Update, attach_player_model_client);
        }
        app.insert_resource(SpawnedPlayersCount {
            current: 0,
            max: self.player_count,
        });
        app.insert_resource(GameEndCondition {
            physics_start_time: 0,
            max_game_seconds: self.max_game_seconds,
            players_finished: 0,
            evaluated: false,
            has_started: false,
        });
    }
}

fn sync_velocity_physics(mut players_q: Query<(&mut VelocityShare, &Velocity)>) {
    for (mut share, real) in players_q.iter_mut() {
        share.linear = real.linvel;
        share.angular = real.angvel;
    }
}

fn attach_player_model_server(
    player_query: Query<
        (&PlayerPosition, &PlayerColor, Entity),
        (Without<Transform>, Without<Predicted>),
    >,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_count: Res<SpawnedPlayersCount>,
    mut game_end_condition: ResMut<GameEndCondition>,
    time: Res<Time>,
) {
    if player_count.current == player_count.max && !game_end_condition.has_started {
        let mut c = 0;
        for (position, color, entity) in player_query.iter() {
            c += 1;
            commands.get_entity(entity).unwrap().insert((
                Mesh3d(meshes.add(Sphere::new(0.5))),
                MeshMaterial3d(materials.add(color.0)),
                Transform::from_translation(position.0),
            ));
            commands.get_entity(entity).unwrap().log_components();
            commands
                .get_entity(entity)
                .unwrap()
                .insert(Collider::ball(0.5))
                .insert(Restitution::coefficient(0.7))
                .insert(RigidBody::Dynamic)
                .insert(ActiveEvents::COLLISION_EVENTS)
                .insert(Velocity {
                    linvel: Vec3::new(0.0, 0.0, 0.0),
                    angvel: Vec3::new(0.0, 0.0, 0.0),
                })
                .insert(ExternalForce {
                    force: Vec3::ZERO,
                    torque: Vec3::ZERO,
                })
                .insert(GravityScale(1.0))
                .insert(Ccd::enabled())
                .insert(LastTouched {
                    road_id: 0,
                    at: 0.0,
                    touching: false,
                })
                .insert(GravityModifier {
                    base_gravity: 1.0,
                    remaining: Timer::from_seconds(0.0, TimerMode::Once),
                    current: 1.0,
                });
        }
        if c != 0 {
            game_end_condition.physics_start_time = time.elapsed_secs() as u32;
            game_end_condition.has_started = true;
        }
    }
}

fn attach_player_model_client(
    player_query: Query<
        (&PlayerPosition, &PlayerColor, Entity),
        (Without<Transform>, Without<Predicted>),
    >,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (position, color, entity) in player_query.iter() {
        commands.get_entity(entity).unwrap().insert((
            Mesh3d(meshes.add(Sphere::new(0.5))),
            MeshMaterial3d(materials.add(color.0)),
            Transform::from_translation(position.0),
        ));
        commands.get_entity(entity).unwrap().log_components();
    }
}

fn game_end_system(
    mut game_end_condition: ResMut<GameEndCondition>,
    players: Query<(&LastTouched, Option<&Finished>, &ControlledBy)>,
    name_q: Query<(&PlayerName, &Replicated)>,
    time: Res<Time>,
    player_count: Res<SpawnedPlayersCount>,
    mut commands: Commands,
) {
    let condition = game_end_condition.has_started
        && !game_end_condition.evaluated
        && ((game_end_condition.max_game_seconds + game_end_condition.physics_start_time) as f32
            <= time.elapsed_secs()
            || game_end_condition.players_finished == player_count.max);
    if condition {
        let id_2_name: HashMap<_, _> = name_q.iter().map(|(n, c)| (c.from.unwrap(), n)).collect();
        dbg!(&id_2_name);
        let mut all_players: Vec<_> = players
            .iter()
            .map(|(l, f, c)| match c.target {
                lightyear::prelude::NetworkTarget::Single(client_id) => {
                    let name = id_2_name.get(&client_id).unwrap();
                    let f = if let Some(t) = f {
                        Finish::Time(t.at)
                    } else {
                        Finish::TrackProgress(l.road_id, l.at)
                    };
                    (name.0.to_string(), f)
                }
                _ => panic!(),
            })
            .collect();
        all_players.sort_unstable_by(|a, b| match (a.1, b.1) {
            (Finish::Time(a), Finish::Time(b)) => a.total_cmp(&b),
            (Finish::Time(_), Finish::TrackProgress(_, _)) => std::cmp::Ordering::Less,
            (Finish::TrackProgress(_, _), Finish::Time(_)) => std::cmp::Ordering::Greater,
            (Finish::TrackProgress(a_i, a_t), Finish::TrackProgress(b_i, b_t)) => {
                if a_i == b_i {
                    a_t.total_cmp(&b_t)
                } else {
                    a_i.cmp(&b_i)
                }
            }
        });

        info!(rankings = ?all_players, "game end ------------------" );
        commands.spawn((
            GameResult {
                players: all_players,
            },
            Replicate {
                target: lightyear::prelude::server::ReplicationTarget {
                    target: lightyear::prelude::NetworkTarget::All,
                },
                authority: lightyear::prelude::server::AuthorityPeer::Server,
                marker: lightyear::prelude::Replicating,
                ..Default::default()
            },
        ));

        game_end_condition.evaluated = true;
    }
}

#[derive(Bundle)]
pub struct PlayerBundle {
    pub position: PlayerPosition,
    pub color: PlayerColor,
}
