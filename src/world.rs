use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{
    player::GameEndCondition,
    protocol::PlayerPosition,
    track_gen::{BallModifier, RoadType, Track, TrackSegment},
    track_mesh::{TRACK_WIDTH, generate_mesh_for_block},
};

pub struct WorldPlugin {
    pub physics: bool,
}

#[derive(Component)]
pub struct TrackSegmentId(usize);

#[derive(Component, Debug)]
pub struct LastTouched {
    pub road_id: usize,
    pub at: f32,
    pub touching: bool,
}

#[derive(Component, Debug)]
pub struct Finished {
    pub at: f32,
}

#[derive(Component)]
pub struct ModifierTrigger(BallModifier);

#[derive(Component)]
pub struct GravityModifier {
    pub base_gravity: f32,
    pub remaining: Timer,
    pub current: f32,
}

#[derive(Resource)]
struct Physics(bool);

#[derive(Resource)]
pub struct Seed(pub u32);

#[derive(Resource)]
pub struct LowGpu(pub bool);

#[derive(Component)]
struct GoalLine;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_world);
        app.insert_resource(Physics(self.physics));
        if self.physics {
            app.add_systems(Update, (collision_system, apply_gravity_modification));
        }
    }
}

fn spawn_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    physics: Res<Physics>,
    asset_server: Res<AssetServer>,
    seed: Res<Seed>,
    low_gpu: Res<LowGpu>,
) {
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0, // Adjust the brightness as needed
            shadows_enabled: true, // Enable shadows if required
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 100.0, 0.0), // Position at (0, 100, 0)
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2), // Rotate to point straight down
            ..default()
        },
    ));
    let track = Track::generate(seed.0, 30.0);
    // let track = Track::debug_straight();
    for (i, segment) in track.segments.iter().enumerate() {
        let m = generate_mesh_for_block(segment.block_type.clone());
        let collider =
            Collider::from_bevy_mesh(&m, &ComputedColliderShape::TriMesh(TriMeshFlags::all()))
                .unwrap();
        let mut e = commands.spawn((
            Mesh3d(meshes.add(m)),
            MeshMaterial3d(materials.add(material_for_segment(&segment, &asset_server, low_gpu.0))),
            Transform::IDENTITY
                .with_translation(segment.transform.position)
                .with_rotation(segment.transform.rotation),
            TrackSegmentId(i),
        ));

        if physics.0 {
            let friciton = match segment.road_type {
                crate::track_gen::RoadType::Asphalt => 1.0,
                crate::track_gen::RoadType::Ice => 0.3,
            };

            e.insert((
                collider,
                ActiveEvents::COLLISION_EVENTS,
                Friction {
                    coefficient: friciton,
                    combine_rule: CoefficientCombineRule::Min,
                },
            ));
        }

        if let BallModifier::GravityChange { .. } = segment.modifier {
            spawn_gravity_booster_marker(
                &mut commands,
                &mut meshes,
                &mut materials,
                segment.transform.position + Vec3::Y * 1.0,
                segment.modifier,
                physics.0,
            );
        }
    }
    let mut goal_line = commands.spawn((
        Transform::from_translation(track.segments.last().unwrap().transform.position)
            .with_rotation(track.segments.last().unwrap().transform.rotation),
        MeshMaterial3d(materials.add(StandardMaterial {
            // Load textures from the concrete directory
            base_color: Color::BLACK.with_alpha(0.5),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Mesh3d(meshes.add(Cuboid::new(TRACK_WIDTH, 10.0, 10.0))),
        GoalLine,
    ));

    if physics.0 {
        goal_line.insert((
            Collider::cuboid(TRACK_WIDTH / 2.0, 5.0, 10.0),
            Sensor,
            ActiveEvents::COLLISION_EVENTS,
        ));
    }
}

fn spawn_gravity_booster_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    modifier: BallModifier,
    physics: bool,
) {
    let yellow_material = if physics {
        materials.add(StandardMaterial {
            base_color: Color::oklab(0.83, -0.01, 0.16).with_alpha(0.6),
            emissive: LinearRgba::rgb(150.0, 0.0, 0.0),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })
    } else {
        materials.add(StandardMaterial {
            base_color: Color::oklab(0.83, -0.01, 0.16).with_alpha(0.6),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })
    };
    if physics {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(yellow_material),
            Transform::IDENTITY.with_translation(position),
            ModifierTrigger(modifier),
            Collider::cuboid(0.5, 0.5, 0.5),
            Sensor,
            ActiveEvents::COLLISION_EVENTS,
        ));
    } else {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(yellow_material),
            Transform::IDENTITY.with_translation(position),
            ModifierTrigger(modifier),
        ));
    }
}

fn material_for_segment(
    segment: &TrackSegment,
    asset_server: &AssetServer,
    low_gpu: bool,
) -> StandardMaterial {
    // This assumes you have access to an asset_server in your system
    // You might need to get this from a parameter, resource, or closure

    if low_gpu {
        match segment.road_type {
            RoadType::Asphalt => StandardMaterial {
                // Load textures from the concrete directory
                base_color_texture: Some(asset_server.load("concrete/color.png")),
                ..default()
            },
            RoadType::Ice => StandardMaterial {
                // Load textures from the ice directory
                base_color_texture: Some(asset_server.load("ice/color.png")),
                ..default()
            },
        }
    } else {
        match segment.road_type {
            RoadType::Asphalt => StandardMaterial {
                // Load textures from the concrete directory
                base_color_texture: Some(asset_server.load("concrete/color.png")),
                normal_map_texture: Some(asset_server.load("concrete/normal.png")),
                metallic_roughness_texture: Some(asset_server.load("concrete/roughness.png")),
                // Material properties appropriate for concrete/asphalt
                perceptual_roughness: 0.9, // Concrete is rough
                metallic: 0.0,             // Not metallic
                reflectance: 0.05,         // Low reflectance
                ..default()
            },
            RoadType::Ice => StandardMaterial {
                // Load textures from the ice directory
                base_color_texture: Some(asset_server.load("ice/color.png")),
                normal_map_texture: Some(asset_server.load("ice/normal.png")),
                metallic_roughness_texture: Some(asset_server.load("ice/roughness.png")),
                // Material properties appropriate for ice
                perceptual_roughness: 0.1,  // Ice is smooth
                metallic: 0.0,              // Not metallic
                reflectance: 0.5,           // Higher reflectance
                ior: 1.31,                  // Index of refraction for ice[4][5]
                specular_transmission: 0.6, // Ice is somewhat transparent
                thickness: 0.5,             // Required for transmission effects
                ..default()
            },
        }
    }
}

fn apply_gravity_modification(
    mut query: Query<(&mut GravityScale, &mut GravityModifier)>,
    time: Res<Time>,
) {
    for (mut scale, mut modifier) in query.iter_mut() {
        modifier.remaining.tick(time.delta());
        if modifier.remaining.finished() {
            scale.0 = modifier.base_gravity;
        } else {
            scale.0 = modifier.current;
        }
    }
}

fn collision_system(
    mut collision_events: EventReader<CollisionEvent>,
    mut players: Query<(&mut LastTouched, &mut GravityModifier), With<PlayerPosition>>,
    goal_query: Query<Entity, With<GoalLine>>,
    player_query: Query<Entity, With<PlayerPosition>>,
    collision_modifiers: Query<&ModifierTrigger>,
    track_segments: Query<&TrackSegmentId>,
    time: Res<Time>,
    mut commands: Commands,
    mut end_conditon: ResMut<GameEndCondition>,
) {
    for collision_event in collision_events.read() {
        match collision_event {
            CollisionEvent::Started(entity1, entity2, _) => {
                if goal_query.get(*entity1).is_ok() && player_query.get(*entity2).is_ok() {
                    commands.get_entity(*entity2).unwrap().insert(Finished {
                        at: time.elapsed_secs(),
                    });
                    end_conditon.players_finished += 1;
                } else if goal_query.get(*entity2).is_ok() && player_query.get(*entity1).is_ok() {
                    commands.get_entity(*entity1).unwrap().insert(Finished {
                        at: time.elapsed_secs(),
                    });
                    end_conditon.players_finished += 1;
                } else {
                    process_potential_collision(
                        *entity1,
                        *entity2,
                        &mut players,
                        &track_segments,
                        &collision_modifiers,
                        &time,
                    );
                    process_potential_collision(
                        *entity2,
                        *entity1,
                        &mut players,
                        &track_segments,
                        &collision_modifiers,
                        &time,
                    );
                }
            }
            CollisionEvent::Stopped(entity1, entity2, _) => {
                process_potential_collision_stop(
                    *entity1,
                    *entity2,
                    &mut players,
                    &track_segments,
                    &time,
                );
                process_potential_collision_stop(
                    *entity2,
                    *entity1,
                    &mut players,
                    &track_segments,
                    &time,
                );
            }
        }
    }
}

fn process_potential_collision(
    potential_player: Entity,
    potential_track: Entity,
    players: &mut Query<(&mut LastTouched, &mut GravityModifier), With<PlayerPosition>>,
    track_segments: &Query<&TrackSegmentId>,
    collision_modifiers: &Query<&ModifierTrigger>,
    time: &Res<Time>,
) {
    // Only proceed if the entities match our requirements
    if let Ok((mut last_touched, mut gravity_modifier)) = players.get_mut(potential_player) {
        if let Ok(track_segment_id) = track_segments.get(potential_track) {
            // Update the LastTouchedId if the new id is higher
            if track_segment_id.0 > last_touched.road_id {
                last_touched.road_id = track_segment_id.0;
            }

            // Update the LastTouchTime to the current time
            last_touched.at = time.elapsed_secs();
            last_touched.touching = true;
        }
        if let Ok(modifiers) = collision_modifiers.get(potential_track) {
            match modifiers.0 {
                BallModifier::GravityChange { strength, duration } => {
                    gravity_modifier.remaining =
                        Timer::from_seconds(duration.as_secs_f32(), TimerMode::Once);
                    gravity_modifier.current = strength;
                }
                BallModifier::None => (),
            }
        }
    }
}

fn process_potential_collision_stop(
    potential_player: Entity,
    potential_track: Entity,
    players: &mut Query<(&mut LastTouched, &mut GravityModifier), With<PlayerPosition>>,
    track_segments: &Query<&TrackSegmentId>,
    time: &Res<Time>,
) {
    // Only proceed if the entities match our requirements
    if let Ok((mut last_touch, _)) = players.get_mut(potential_player) {
        if let Ok(_) = track_segments.get(potential_track) {
            last_touch.touching = false;
            last_touch.at = time.elapsed_secs();
        }
    }
}
