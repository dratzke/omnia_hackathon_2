use bevy::prelude::*;
use lightyear::shared::replication::components::Controlled;

pub struct ClientCameraPlugin;
impl Plugin for ClientCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, align_camera_with_movement);
    }
}

// Component to store previous position for calculating direction
#[derive(Component)]
pub struct DirectionalCamera {
    previous_position: Vec3,
    smoothing_factor: f32,
    height_offset: f32,
    distance: f32,
}

impl Default for DirectionalCamera {
    fn default() -> Self {
        Self {
            previous_position: Vec3::ZERO,
            smoothing_factor: 5.0, // Adjust for smoother or more responsive rotation
            height_offset: 2.0,    // Camera height above player
            distance: 5.0,         // Distance behind player
        }
    }
}

// This system aligns the camera with the direction of travel but always points at the player
fn align_camera_with_movement(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Controlled>, Without<DirectionalCamera>)>,
    mut camera_query: Query<(&mut Transform, &mut DirectionalCamera), Without<Controlled>>,
) {
    let player_transform = match player_query.get_single() {
        Ok(transform) => transform,
        Err(_) => return, // Player not found
    };

    let (mut camera_transform, mut directional_camera) = match camera_query.get_single_mut() {
        Ok(result) => result,
        Err(_) => return, // Camera not found
    };

    let current_position = player_transform.translation;

    // Calculate movement direction
    let movement_vector = current_position - directional_camera.previous_position;

    // Only update camera position if we have significant movement
    if movement_vector.length_squared() > 0.001 {
        let movement_direction = movement_vector.normalize();

        // Calculate the target position behind the player based on movement direction
        let offset_direction = if movement_vector.length_squared() > 0.001 {
            -movement_direction
        } else {
            // If not moving, use the current camera-to-player direction
            (camera_transform.translation - current_position).normalize()
        };

        // Calculate target camera position
        let target_position = current_position
            + offset_direction * directional_camera.distance
            + Vec3::new(0.0, directional_camera.height_offset, 0.0);

        // Smoothly interpolate the camera position
        camera_transform.translation = camera_transform.translation.lerp(
            target_position,
            time.delta_secs() * directional_camera.smoothing_factor,
        );
    }

    // Always make the camera look at the player, regardless of movement
    camera_transform.look_at(player_transform.translation, Vec3::Y);

    // Store current position for next frame
    directional_camera.previous_position = current_position;
}
