use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{track_gen::Track, track_mesh::generate_mesh_for_block};

pub struct WorldPlugin {
    pub physics: bool,
}

#[derive(Resource)]
struct Physics(bool);

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_world);
        app.insert_resource(Physics(self.physics));
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
    // let mut e = commands.spawn((
    //     Mesh3d(meshes.add(Cuboid::new(100.0, 0.1, 100.0))),
    //     MeshMaterial3d(materials.add(Color::oklab(0.74, -0.12, 0.11))),
    // ));
    // e.insert(Transform::from_xyz(0.0, -2.0, 0.0));
    // if physics.0 {
    //     e.insert(Collider::cuboid(100.0, 0.1, 100.0));
    // }
    let track = Track::generate(1234, 30.0);
    // let track = Track::debug_straight();
    for segment in track.segments {
        let lum = rand::random_range(0.0..1.0);
        commands.spawn((
            Mesh3d(meshes.add(generate_mesh_for_block(segment.block_type))),
            MeshMaterial3d(materials.add(Color::oklab(lum, -0.12, 0.11))),
            Transform::IDENTITY
                .with_translation(segment.transform.position)
                .with_rotation(segment.transform.rotation),
        ));
    }
}
