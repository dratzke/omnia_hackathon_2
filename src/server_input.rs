use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use lightyear::prelude::*;
use server::InputEvent;

use crate::{
    ClientIds,
    protocol::{Inputs, PlayerPosition},
    world::LastTouchedTime,
};

pub struct ServerInputPlugin;

impl Plugin for ServerInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, movement);
        app.add_systems(PostUpdate, sync_positions);
    }
}

fn movement(
    mut position_query: Query<(&mut Velocity, &LastTouchedTime)>,
    mut input_reader: EventReader<InputEvent<Inputs>>,
    client_ids: Res<ClientIds>,
    time: Res<Time>,
) {
    for input in input_reader.read() {
        let client_id = input.from();
        if let Some(input) = input.input() {
            let client_ids = client_ids.0.read().unwrap();
            if let Some(player_entity) = client_ids.get(&client_id.to_bits()) {
                if let Ok((position, last_touched)) = position_query.get_mut(*player_entity) {
                    if time.elapsed_secs() - last_touched.0 < 1.0 || last_touched.1 {
                        shared_movement_behaviour(position, input);
                    }
                }
            }
        }
    }
}

fn shared_movement_behaviour(mut velocity: Mut<Velocity>, input: &Inputs) {
    let lin = velocity.linvel.normalize();

    match input {
        Inputs::Direction(direction) => {
            if direction.forward {
                velocity.linvel += lin * 0.1;
            }
            if direction.back {
                velocity.linvel -= lin * 0.1;
            }
            if direction.left {
                let rotated = Quat::from_rotation_y(PI * 0.5) * lin;
                velocity.linvel += rotated * 0.1;
            }
            if direction.right {
                let rotated = Quat::from_rotation_y(PI * 0.5) * lin;
                velocity.linvel -= rotated * 0.1;
            }
        }
        _ => {}
    }
}

fn sync_positions(mut players: Query<(&mut PlayerPosition, &Transform)>) {
    for (mut position, transform) in players.iter_mut() {
        *position = PlayerPosition(transform.translation, transform.rotation);
    }
}
