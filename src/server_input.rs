use std::{collections::HashMap, f32::consts::PI};

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use lightyear::prelude::*;
use server::InputEvent;

use crate::{
    ClientIds,
    protocol::{Inputs, PlayerPosition},
    world::LastTouched,
};

pub struct ServerInputPlugin;

impl Plugin for ServerInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, movement);
        app.add_systems(PostUpdate, sync_positions);
    }
}

fn movement(
    mut position_query: Query<(&mut Velocity, &mut ExternalForce, &LastTouched)>,
    mut input_reader: EventReader<InputEvent<Inputs>>,
    client_ids: Res<ClientIds>,
    time: Res<Time>,
) {
    for input in input_reader.read() {
        let client_id = input.from();
        if let Some(input) = input.input() {
            let client_ids = client_ids.0.read().unwrap();
            if let Some(player_entity) = client_ids.get(&client_id.to_bits()) {
                if let Ok((velocity, force, last_touched)) = position_query.get_mut(*player_entity)
                {
                    if time.elapsed_secs() - last_touched.at < 1.0 || last_touched.touching {
                        torque_fucntion(velocity, force, input);
                    }
                }
            }
        }
    }
}

fn torque_fucntion(mut velocity: Mut<Velocity>, mut force: Mut<ExternalForce>, input: &Inputs) {
    let lin = velocity.linvel.normalize();
    let multiplier = 2.0f32;
    let up = Vec3::Y;

    match input {
        Inputs::Direction(direction) => {
            match (
                direction.forward,
                direction.back,
                direction.left,
                direction.right,
            ) {
                // No movement
                (false, false, false, false) => force.torque = Vec3::ZERO,

                // Single direction
                (true, false, false, false) => {
                    let forward_torque = up.cross(lin).normalize();
                    force.torque = forward_torque * multiplier;
                }
                (false, true, false, false) => {
                    let forward_torque = up.cross(lin).normalize();
                    force.torque = -forward_torque * multiplier;
                }
                (false, false, true, false) => {
                    let forward_torque = up.cross(lin).normalize();
                    force.torque = forward_torque * multiplier;
                    force.torque = Quat::from_rotation_y(PI * 0.75) * force.torque;
                }
                (false, false, false, true) => {
                    let forward_torque = up.cross(lin).normalize();
                    force.torque = forward_torque * multiplier;
                    force.torque = Quat::from_rotation_y(-PI * 0.75) * force.torque;
                }

                // Forward + Sideways
                (true, false, true, false) => {
                    let forward_torque = up.cross(lin).normalize();
                    force.torque = forward_torque * multiplier;
                    force.torque = Quat::from_rotation_y(PI * 0.5) * force.torque;
                }
                (true, false, false, true) => {
                    let forward_torque = up.cross(lin).normalize();
                    force.torque = forward_torque * multiplier;
                    force.torque = Quat::from_rotation_y(-PI * 0.5) * force.torque;
                }

                // Back + Sideways
                (false, true, true, false) => {
                    let forward_torque = up.cross(lin).normalize();
                    force.torque = -forward_torque * multiplier;
                    force.torque = Quat::from_rotation_y(-PI * 0.5) * force.torque;
                }
                (false, true, false, true) => {
                    let forward_torque = up.cross(lin).normalize();
                    force.torque = -forward_torque * multiplier;
                    force.torque = Quat::from_rotation_y(PI * 0.5) * force.torque;
                }

                // Sideways only (both)
                (false, false, true, true) => force.torque = Vec3::ZERO,

                // Forward + Back (conflicting?)
                (true, true, false, false) => force.torque = Vec3::ZERO,

                // Three directions (conflicting?)
                (true, true, true, false) => {
                    let forward_torque = up.cross(lin).normalize();
                    force.torque = forward_torque * multiplier;
                    force.torque = Quat::from_rotation_y(PI * 0.75) * force.torque;
                }
                (true, true, false, true) => {
                    let forward_torque = up.cross(lin).normalize();
                    force.torque = forward_torque * multiplier;
                    force.torque = Quat::from_rotation_y(-PI * 0.75) * force.torque;
                }
                (true, false, true, true) => {
                    let forward_torque = up.cross(lin).normalize();
                    force.torque = forward_torque * multiplier;
                }
                (false, true, true, true) => {
                    let forward_torque = up.cross(lin).normalize();
                    force.torque = -forward_torque * multiplier;
                }

                // All four directions (conflicting?)
                (true, true, true, true) => force.torque = Vec3::ZERO,
            }

            if direction.reset {
                velocity.angvel = Vec3::ZERO;
            }
        }
        Inputs::None => force.torque = Vec3::ZERO,
        _ => (),
    }
    if velocity.angvel != Vec3::ZERO {
        velocity.angvel = velocity.angvel.clamp_length(0.1, 500.0);
    }
}

fn sync_positions(mut players: Query<(&mut PlayerPosition, &Transform)>) {
    for (mut position, transform) in players.iter_mut() {
        *position = PlayerPosition(transform.translation, transform.rotation);
    }
}
