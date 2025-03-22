use bevy::prelude::*;

use crate::protocol::{PlayerColor, PlayerPosition};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_player_model);
    }
}

fn attach_player_model(
    player_query: Query<(&PlayerPosition, &PlayerColor, Entity), Without<Transform>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (position, color, entity) in player_query.iter() {
        info!("attach palyer model");
        commands.get_entity(entity).unwrap().insert((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(color.0)),
            Transform::from_xyz(position.0.x, position.0.y, position.0.z),
        ));
    }
}

#[derive(Bundle)]
pub struct PlayerBundle {
    pub position: PlayerPosition,
    pub color: PlayerColor,
}
