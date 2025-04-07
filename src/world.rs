use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{protocol::PlayerPosition, track_gen::Track, track_mesh::generate_mesh_for_block};

pub struct WorldPlugin {
    pub physics: bool,
}

#[derive(Component)]
pub struct TrackSegmentId(usize);

#[derive(Component)]
pub struct LastTouchedId(pub usize);

#[derive(Component)]
pub struct LastTouchedTime(pub f32, pub bool);

#[derive(Resource)]
struct Physics(bool);

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_world);
        app.insert_resource(Physics(self.physics));
        if self.physics {
            app.add_systems(Update, track_segment_collision_system);
        }
    }
}

fn spawn_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    physics: Res<Physics>,
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
    let track = Track::generate(1234, 30.0);
    // let track = Track::debug_straight();
    for (i, segment) in track.segments.into_iter().enumerate() {
        let lum = rand::random_range(0.0..1.0);
        let m = generate_mesh_for_block(segment.block_type);
        let collider =
            Collider::from_bevy_mesh(&m, &ComputedColliderShape::TriMesh(TriMeshFlags::all()))
                .unwrap();
        let mut e = commands.spawn((
            Mesh3d(meshes.add(m)),
            MeshMaterial3d(materials.add(Color::oklab(lum, -0.12, 0.11))),
            Transform::IDENTITY
                .with_translation(segment.transform.position)
                .with_rotation(segment.transform.rotation),
            TrackSegmentId(i),
        ));

        if physics.0 {
            e.insert((collider, ActiveEvents::COLLISION_EVENTS));
        }
    }
}

fn track_segment_collision_system(
    mut collision_events: EventReader<CollisionEvent>,
    mut players: Query<(&mut LastTouchedId, &mut LastTouchedTime), With<PlayerPosition>>,
    track_segments: Query<&TrackSegmentId>,
    time: Res<Time>,
) {
    for collision_event in collision_events.read() {
        match collision_event {
            CollisionEvent::Started(entity1, entity2, _) => {
                process_potential_collision(
                    *entity1,
                    *entity2,
                    &mut players,
                    &track_segments,
                    &time,
                );
                process_potential_collision(
                    *entity2,
                    *entity1,
                    &mut players,
                    &track_segments,
                    &time,
                );
            }
            CollisionEvent::Stopped(entity1, entity2, _) => {
                process_potential_collision_stop(*entity1, *entity2, &mut players, &track_segments);
                process_potential_collision_stop(*entity2, *entity1, &mut players, &track_segments);
            }
        }
    }
}

fn process_potential_collision(
    potential_player: Entity,
    potential_track: Entity,
    players: &mut Query<(&mut LastTouchedId, &mut LastTouchedTime), With<PlayerPosition>>,
    track_segments: &Query<&TrackSegmentId>,
    time: &Res<Time>,
) {
    // Only proceed if the entities match our requirements
    if let Ok((mut last_touched_id, mut last_touch_time)) = players.get_mut(potential_player) {
        if let Ok(track_segment_id) = track_segments.get(potential_track) {
            // Update the LastTouchedId if the new id is higher
            if track_segment_id.0 > last_touched_id.0 {
                last_touched_id.0 = track_segment_id.0;
            }

            // Update the LastTouchTime to the current time
            last_touch_time.0 = time.elapsed_secs();
            last_touch_time.1 = true;
        }
    }
}
fn process_potential_collision_stop(
    potential_player: Entity,
    potential_track: Entity,
    players: &mut Query<(&mut LastTouchedId, &mut LastTouchedTime), With<PlayerPosition>>,
    track_segments: &Query<&TrackSegmentId>,
) {
    // Only proceed if the entities match our requirements
    if let Ok((_, mut last_touch_time)) = players.get_mut(potential_player) {
        if let Ok(_) = track_segments.get(potential_track) {
            last_touch_time.1 = false;
        }
    }
}
