use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use lightyear::prelude::*;
use server::InputEvent;

use crate::{
    ClientIds,
    protocol::{Inputs, PlayerPosition},
};

pub struct ServerInputPlugin;

impl Plugin for ServerInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, movement);
        app.add_systems(PostUpdate, sync_positions);
    }
}

fn movement(
    mut position_query: Query<&mut Velocity>,
    mut input_reader: EventReader<InputEvent<Inputs>>,
    client_ids: Res<ClientIds>,
) {
    for input in input_reader.read() {
        let client_id = input.from();
        if let Some(input) = input.input() {
            let client_ids = client_ids.0.read().unwrap();
            if let Some(player_entity) = client_ids.get(&client_id.to_bits()) {
                if let Ok(position) = position_query.get_mut(*player_entity) {
                    shared_movement_behaviour(position, input);
                }
            }
        }
    }
}

fn shared_movement_behaviour(mut position: Mut<Velocity>, input: &Inputs) {
    const MOVE_SPEED: f32 = 0.1;
    match input {
        Inputs::Direction(direction) => {
            if direction.forward {
                position.linvel.x += MOVE_SPEED;
            }
            if direction.back {
                position.linvel.x -= MOVE_SPEED;
            }
            if direction.left {
                position.linvel.z -= MOVE_SPEED;
            }
            if direction.right {
                position.linvel.z += MOVE_SPEED;
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
