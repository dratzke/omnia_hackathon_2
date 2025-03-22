use bevy::prelude::*;
use lightyear::prelude::*;
use server::InputEvent;

use crate::{
    ClientIds,
    protocol::{Inputs, PlayerPosition},
};

pub struct ServerInputPlugin;

impl Plugin for ServerInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, movement);
        app.add_systems(Update, sync_positions);
    }
}

fn movement(
    mut position_query: Query<&mut PlayerPosition>,
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

fn shared_movement_behaviour(mut position: Mut<PlayerPosition>, input: &Inputs) {
    const MOVE_SPEED: f32 = 0.1;
    match input {
        Inputs::Direction(direction) => {
            dbg!(direction);
            if direction.forward {
                position.0.x += MOVE_SPEED;
            }
            if direction.back {
                position.0.x -= MOVE_SPEED;
            }
            if direction.left {
                position.0.z -= MOVE_SPEED;
            }
            if direction.right {
                position.0.z += MOVE_SPEED;
            }
        }
        _ => {}
    }
}

fn sync_positions(mut players: Query<(&mut Transform, &PlayerPosition)>) {
    for (mut transform, position) in players.iter_mut() {
        *transform = transform.with_translation(position.0);
    }
}
