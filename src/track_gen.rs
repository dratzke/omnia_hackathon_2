use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use std::{f32::consts::PI, time::Duration};

#[derive(Debug, Clone, Copy)]
pub struct BlockTransform {
    pub position: Vec3,
    pub rotation: Quat,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Straight {
        length: f32,
    },
    Turn {
        angle: f32,
        radius: f32,
    },
    BankedTurn {
        angle: f32,
        radius: f32,
        bank_height: f32,
    },
    Slope {
        length: f32,
        height_change: f32,
    },
    Bumpy {
        length: f32,
        pertubation: f32,
    },
}

#[derive(Debug, Clone)]
pub enum RoadType {
    Asphalt,
    Ice,
}

#[derive(Debug, Clone, Copy)]
pub enum BallModifier {
    GravityChange { strength: f32, duration: Duration },
    None,
}

#[derive(Debug, Clone)]
pub struct TrackSegment {
    pub block_type: BlockType,
    pub transform: BlockTransform,
    pub road_type: RoadType,
    pub modifier: BallModifier,
}

pub struct Track {
    pub segments: Vec<TrackSegment>,
    pub current_end: BlockTransform,
    pub noise: Perlin,
    turn_since_down: f32,
}

impl Track {
    pub fn debug_straight() -> Self {
        let mut track = Self {
            segments: Vec::new(),
            current_end: BlockTransform {
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            noise: Perlin::new(0),
            turn_since_down: 0.0,
        };
        track.append_block(
            BlockType::Slope {
                length: 10.0,
                height_change: -10.0,
            },
            RoadType::Asphalt,
            BallModifier::None,
        );
        track.append_block(
            BlockType::Bumpy {
                length: 10.0,
                pertubation: 2.0,
            },
            RoadType::Asphalt,
            BallModifier::None,
        );
        track.append_block(
            BlockType::Straight { length: 10.0 },
            RoadType::Asphalt,
            BallModifier::GravityChange {
                strength: 10.0,
                duration: Duration::from_secs(10),
            },
        );
        track.append_block(
            BlockType::Straight { length: 10.0 },
            RoadType::Asphalt,
            BallModifier::None,
        );
        track.append_block(
            BlockType::Straight { length: 10.0 },
            RoadType::Asphalt,
            BallModifier::None,
        );
        track.append_block(
            BlockType::Straight { length: 10.0 },
            RoadType::Asphalt,
            BallModifier::None,
        );
        track.append_block(
            BlockType::Slope {
                length: 10.0,
                height_change: -10.0,
            },
            RoadType::Asphalt,
            BallModifier::None,
        );

        track.append_block(
            BlockType::Straight { length: 10.0 },
            RoadType::Asphalt,
            BallModifier::None,
        );
        track.append_block(
            BlockType::Straight { length: 10.0 },
            RoadType::Asphalt,
            BallModifier::None,
        );
        track.append_block(
            BlockType::Straight { length: 10.0 },
            RoadType::Asphalt,
            BallModifier::None,
        );

        track
    }
    pub fn generate(seed: u32, initial_length: f32) -> Self {
        let mut track = Self {
            segments: Vec::new(),
            current_end: BlockTransform {
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            noise: Perlin::new(seed),
            turn_since_down: 0.0,
        };

        track.append_block(
            BlockType::Slope {
                length: initial_length,
                height_change: -3.0,
            },
            RoadType::Asphalt,
            BallModifier::None,
        );

        for _ in 0..100 {
            let next_block = track.select_next_block();
            match next_block {
                BlockType::Turn { angle, .. } => track.turn_since_down += angle.abs(),
                BlockType::BankedTurn { angle, .. } => track.turn_since_down += angle.abs(),
                BlockType::Slope { .. } => track.turn_since_down = 0.0,
                _ => (),
            }
            let road_type = if track
                .noise
                .get([
                    track.current_end.position.x as f64 * 0.5,
                    track.current_end.position.y as f64 * 0.5,
                    track.current_end.position.z as f64 * 0.5,
                ])
                .abs()
                < 0.1
            {
                RoadType::Ice
            } else {
                RoadType::Asphalt
            };
            let modifier = if track
                .noise
                .get([
                    40.0 * PI as f64 + track.current_end.position.x as f64,
                    40.0 * PI as f64 + track.current_end.position.y as f64,
                    40.0 * PI as f64 + track.current_end.position.z as f64,
                ])
                .abs()
                > 0.7
            {
                BallModifier::GravityChange {
                    strength: 4.0,
                    duration: Duration::from_secs(10),
                }
            } else {
                BallModifier::None
            };
            track.append_block(next_block, road_type, modifier);
        }

        track
    }

    fn select_next_block(&self) -> BlockType {
        if self.turn_since_down > 0.5 * PI {
            return BlockType::Slope {
                length: 25.0,
                height_change: -((self.noise.get([
                    self.current_end.position.y as f64 * 0.3,
                    self.current_end.position.x as f64 * 0.3,
                ]) as f32)
                    .abs()
                    * 10.0
                    + 10.0),
            };
        }
        let noise_value = self.noise.get([self.segments.len() as f64 * 0.3, 0.0]);

        match (noise_value).abs() {
            v if v < 0.15 => BlockType::Straight { length: 10.0 },
            v if v < 0.3 => {
                let r = (self.noise.get([
                    self.current_end.position.x as f64 * 0.3,
                    self.current_end.position.y as f64 * 0.3,
                ]) as f32)
                    .abs();
                let angle = PI * r + 0.3;
                BlockType::Turn {
                    angle,
                    radius: self.turn_radius(),
                }
            }
            v if v < 0.55 => BlockType::Bumpy {
                length: 15.0,
                pertubation: 0.4,
            },
            v if v < 0.7 => {
                let r = (self.noise.get([
                    self.current_end.position.x as f64 * 0.3,
                    self.current_end.position.y as f64 * 0.3,
                ]) as f32)
                    .abs();
                let angle = PI * r + 0.3;
                BlockType::BankedTurn {
                    angle,
                    radius: self.turn_radius(),
                    bank_height: angle / PI * 4.0,
                }
            }
            _ => BlockType::Slope {
                length: 15.0,
                height_change: -((self
                    .noise
                    .get([
                        self.current_end.position.y as f64 * 0.3,
                        self.current_end.position.x as f64 * 0.3,
                    ])
                    .abs() as f32)
                    * 10.0
                    + 10.0),
            },
        }
    }

    fn turn_radius(&self) -> f32 {
        let r = (self.noise.get([
            self.current_end.position.y as f64 * 0.3,
            self.current_end.position.x as f64 * 0.3,
        ]) as f32)
            .abs();
        r * 20.0 + 10.0
    }

    pub fn append_block(
        &mut self,
        block_type: BlockType,
        road_type: RoadType,
        modifier: BallModifier,
    ) {
        let end_transform = self.calculate_end_transform(&block_type);

        // Check for overlap
        self.segments.push(TrackSegment {
            block_type,
            transform: self.current_end,
            road_type,
            modifier,
        });
        self.current_end = end_transform;
    }

    fn calculate_end_transform(&self, block_type: &BlockType) -> BlockTransform {
        match block_type {
            BlockType::Straight { length } => BlockTransform {
                position: self.current_end.position + self.current_end.rotation * Vec3::Z * *length,
                rotation: self.current_end.rotation,
            },

            BlockType::Turn { angle, radius } => {
                let rotation = Quat::from_rotation_y(*angle) * self.current_end.rotation;
                // let angle = angle + PI;

                let position_offset =
                    rotate_point_around(Vec2::ZERO, Vec2::new(*radius, 0.0), -angle);
                let position = self.current_end.position
                    + self.current_end.rotation
                        * Vec3::new(position_offset.x, 0.0, position_offset.y);

                BlockTransform { position, rotation }
            }

            BlockType::Slope {
                length,
                height_change,
            } => {
                let rotation = self.current_end.rotation;

                BlockTransform {
                    position: self.current_end.position
                        + self.current_end.rotation * Vec3::new(0.0, *height_change, *length),
                    rotation,
                }
            }
            BlockType::BankedTurn { angle, radius, .. } => {
                let rotation = Quat::from_rotation_y(*angle) * self.current_end.rotation;
                // let angle = angle + PI;

                let position_offset =
                    rotate_point_around(Vec2::ZERO, Vec2::new(*radius, 0.0), -angle);
                let position = self.current_end.position
                    + self.current_end.rotation
                        * Vec3::new(position_offset.x, 0.0, position_offset.y);

                BlockTransform { position, rotation }
            }
            BlockType::Bumpy { length, .. } => BlockTransform {
                position: self.current_end.position + self.current_end.rotation * Vec3::Z * *length,
                rotation: self.current_end.rotation,
            },
        }
    }
}

pub fn rotate_point_around(point: Vec2, around: Vec2, angle: f32) -> Vec2 {
    // Translate point to origin
    let x_translated = point.x - around.x;
    let y_translated = point.y - around.y;

    // Rotate point
    let cos_angle = angle.cos();
    let sin_angle = angle.sin();

    let x_rotated = x_translated * cos_angle - y_translated * sin_angle;
    let y_rotated = x_translated * sin_angle + y_translated * cos_angle;

    // Translate back
    let x_final = x_rotated + around.x;
    let y_final = y_rotated + around.y;

    Vec2::new(x_final, y_final)
}
