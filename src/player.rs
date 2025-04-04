use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::protocol::{PlayerColor, PlayerPosition};

pub struct PlayerPlugin {
    pub physics: bool,
}

#[derive(Resource)]
struct Physics(bool);

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_player_model);
        app.insert_resource(Physics(self.physics));
    }
}

fn attach_player_model(
    player_query: Query<(&PlayerPosition, &PlayerColor, Entity), Without<Transform>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    physics: Res<Physics>,
) {
    for (position, color, entity) in player_query.iter() {
        info!(position=?position,"attach palyer model");
        commands.get_entity(entity).unwrap().insert((
            Mesh3d(meshes.add(Sphere::new(0.5))),
            MeshMaterial3d(materials.add(color.0)),
            Transform::from_xyz(position.0.x, position.0.y, position.0.z),
        ));
        if physics.0 {
            commands
                .get_entity(entity)
                .unwrap()
                .insert(Collider::ball(0.5))
                .insert(Restitution::coefficient(0.7))
                .insert(RigidBody::Dynamic)
                .insert(Velocity {
                    linvel: Vec3::new(0.0, 0.0, 0.0),
                    angvel: Vec3::new(0.0, 0.0, 0.0),
                })
                .insert(GravityScale(1.0));
        }
    }
}

#[derive(Bundle)]
pub struct PlayerBundle {
    pub position: PlayerPosition,
    pub color: PlayerColor,
}
