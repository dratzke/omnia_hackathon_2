use bevy::prelude::*;
use lightyear::prelude::*;

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerId(pub ClientId);

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerPosition(pub Vec3);

impl Linear for PlayerPosition {
    fn lerp(start: &Self, other: &Self, t: f32) -> Self {
        Self(start.0 * (1.0 - t) + other.0 * t)
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerColor(pub Color);

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Direction {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
}

impl Direction {
    pub fn is_some(self) -> bool {
        self.forward || self.back || self.left || self.right
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Inputs {
    Direction(Direction),
    Spawn,
    None,
}

pub struct ProtocolPlugin;

#[derive(Channel)]
pub struct Channel1;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.register_component::<PlayerId>(ChannelDirection::ServerToClient)
            .add_prediction(client::ComponentSyncMode::Once)
            .add_interpolation(client::ComponentSyncMode::Once);

        app.register_component::<PlayerPosition>(ChannelDirection::ServerToClient)
            .add_prediction(client::ComponentSyncMode::Full)
            .add_interpolation(client::ComponentSyncMode::Full)
            .add_linear_interpolation_fn();

        app.register_component::<PlayerColor>(ChannelDirection::ServerToClient)
            .add_prediction(client::ComponentSyncMode::Once)
            .add_interpolation(client::ComponentSyncMode::Once);

        app.add_plugins(InputPlugin::<Inputs>::default());

        app.add_channel::<Channel1>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..Default::default()
        });

        app.add_observer(on_player_spawn);
        app.add_observer(on_player_spawn2);
    }
}

fn on_player_spawn(t: Trigger<ClientEntitySpawnEvent>) {
    dbg!(t.event().entity());
}
fn on_player_spawn2(t: Trigger<ServerEntitySpawnEvent>) {
    dbg!(t.event().entity());
}
