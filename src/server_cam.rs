use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::CursorGrabMode, // Required for cursor locking
};

// Define a plugin to encapsulate camera controller logic
pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MovementSettings>()
            .init_resource::<MouseSensitivity>()
            .add_systems(
                Update,
                (
                    camera_movement_system,
                    camera_rotation_system,
                    cursor_grab_system, // Add system to toggle cursor grab
                ),
            );
    }
}

// Component to mark the camera entity we want to control
#[derive(Component)]
pub struct CameraController;

// Resource to store movement speed
#[derive(Resource)]
pub struct MovementSettings {
    pub speed: f32,
}

impl Default for MovementSettings {
    fn default() -> Self {
        Self { speed: 15.0 } // Adjust speed as needed
    }
}

// Resource to store mouse sensitivity
#[derive(Resource)]
pub struct MouseSensitivity {
    pub value: f32,
}

impl Default for MouseSensitivity {
    fn default() -> Self {
        Self { value: 0.0015 } // Adjust sensitivity as needed
    }
}

// System to handle keyboard input for camera movement
fn camera_movement_system(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    settings: Res<MovementSettings>,
    mut query: Query<&mut Transform, With<CameraController>>,
) {
    for mut transform in query.iter_mut() {
        let mut direction = Vec3::ZERO;
        let forward = transform.forward().as_vec3();
        let right = transform.right().as_vec3();

        // Forward/Backward movement (W/S) relative to camera direction
        if keys.pressed(KeyCode::KeyW) {
            direction += forward;
        }
        if keys.pressed(KeyCode::KeyS) {
            direction -= forward;
        }

        // Left/Right movement (A/D) relative to camera direction
        if keys.pressed(KeyCode::KeyA) {
            direction -= right;
        }
        if keys.pressed(KeyCode::KeyD) {
            direction += right;
        }

        // Up/Down movement (E/C) along the global Y axis
        if keys.pressed(KeyCode::KeyE) {
            direction += Vec3::Y;
        }
        if keys.pressed(KeyCode::KeyC) {
            direction -= Vec3::Y;
        }

        // Normalize diagonal movement to prevent faster speed and apply movement
        if direction.length_squared() > 0.0 {
            direction = direction.normalize();
            transform.translation += direction * settings.speed * time.delta_secs();
        }
    }
}

// System to handle mouse input for camera rotation
fn camera_rotation_system(
    sensitivity: Res<MouseSensitivity>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut query: Query<&mut Transform, With<CameraController>>,
    windows: Query<&Window>, // Query windows to check cursor grab mode
) {
    // Check if the cursor is grabbed, only rotate if it is
    let window = windows.single();
    if window.cursor_options.grab_mode != CursorGrabMode::Locked {
        // Clear motion events if cursor is not locked to prevent sudden jumps
        mouse_motion_events.clear();
        return;
    }

    let mut delta: Vec2 = Vec2::ZERO;
    for event in mouse_motion_events.read() {
        delta += event.delta;
    }

    if delta.length_squared() == 0.0 {
        return; // No mouse motion, do nothing
    }

    for mut transform in query.iter_mut() {
        let mouse_sens = sensitivity.value;
        // Calculate yaw (rotation around the global Y axis)
        let yaw_delta = -delta.x * mouse_sens;
        // Calculate pitch (rotation around the local X axis)
        let pitch_delta = -delta.y * mouse_sens;

        // Apply yaw rotation
        transform.rotate_axis(Dir3::Y, yaw_delta);

        transform.rotate_local_axis(Dir3::X, pitch_delta);
    }
}

// System to toggle cursor grab mode with the Escape key
fn cursor_grab_system(mut windows: Query<&mut Window>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Escape) {
        let mut window = windows.single_mut();
        match window.cursor_options.grab_mode {
            CursorGrabMode::None => {
                window.cursor_options.grab_mode = CursorGrabMode::Locked;
                window.cursor_options.visible = false;
            }
            _ => {
                window.cursor_options.grab_mode = CursorGrabMode::None;
                window.cursor_options.visible = true;
            }
        }
    }
}
