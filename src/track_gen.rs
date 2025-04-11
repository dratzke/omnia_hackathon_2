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
    Straight { length: f32 },
    Turn { angle: f32, radius: f32 },
    Slope { length: f32, height_change: f32 },
}

#[derive(Debug, Clone)]
pub enum RoadType {
    Asphalt,
    Ice,
}

#[derive(Debug, Clone)]
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
    noise: Perlin,
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
        };
        track.append_block(
            BlockType::Straight { length: 10.0 },
            RoadType::Asphalt,
            BallModifier::None,
        );
        // track.append_block(BlockType::Slope {
        //     length: 10.0,
        //     height_change: -10.0,
        // });
        track.append_block(
            BlockType::Turn {
                angle: PI,
                radius: 10.0,
            },
            RoadType::Asphalt,
            BallModifier::None,
        );
        track.append_block(
            BlockType::Turn {
                angle: PI / 2.0,
                radius: 10.0,
            },
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
            BlockType::Turn {
                angle: PI / 2.0,
                radius: 10.0,
            },
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
        };

        track.append_block(
            BlockType::Slope {
                length: initial_length,
                height_change: -3.0,
            },
            RoadType::Asphalt,
            BallModifier::None,
        );

        for _ in 0..500 {
            let next_block = track.select_next_block();
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
                    40.0 * PI as f64 + track.current_end.position.x as f64 * 0.5,
                    40.0 * PI as f64 + track.current_end.position.y as f64 * 0.5,
                    40.0 * PI as f64 + track.current_end.position.z as f64 * 0.5,
                ])
                .abs()
                > 0.9
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
        let noise_value = self.noise.get([self.segments.len() as f64 * 0.3, 0.0]);

        match (noise_value).abs() {
            v if v < 0.3 => BlockType::Straight { length: 10.0 },
            v if v < 0.6 => {
                let r = (self.noise.get([
                    self.current_end.position.x as f64 * 0.3,
                    self.current_end.position.y as f64 * 0.3,
                ]) as f32)
                    .abs();
                BlockType::Turn {
                    angle: PI * r + 0.3,
                    radius: self.turn_radius(),
                }
            }
            _ => BlockType::Slope {
                length: 15.0,
                height_change: -((self.noise.get([
                    self.current_end.position.y as f64 * 0.3,
                    self.current_end.position.x as f64 * 0.3,
                ]) as f32)
                    * 10.0
                    + 10.0),
            },
        }
    }

    fn turn_radius(&self) -> f32 {
        // if rand::random_bool(0.5) {
        let r = (self.noise.get([
            self.current_end.position.y as f64 * 0.3,
            self.current_end.position.x as f64 * 0.3,
        ]) as f32)
            .abs();
        r * 20.0 + 10.0
        // } else {
        // rand::random_range(-30.0..-10.0)
        // }
    }

    fn append_block(&mut self, block_type: BlockType, road_type: RoadType, modifier: BallModifier) {
        let end_transform = self.calculate_end_transform(&block_type);

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

#[cfg(test)]
mod tests {
    use super::*;
    use super::{BlockTransform, BlockType, Track};
    use approx::assert_relative_eq;
    use bevy::prelude::*;
    use std::f32::consts::FRAC_PI_2;

    const EPSILON: f32 = 0.001;

    fn calculate_end(block_type: &BlockType, start: BlockTransform) -> BlockTransform {
        match block_type {
            BlockType::Straight { length } => {
                let direction = start.rotation * Vec3::Z;
                BlockTransform {
                    position: start.position + direction * length,
                    rotation: start.rotation,
                }
            }
            BlockType::Turn { angle, radius } => {
                let rotation = start.rotation * Quat::from_rotation_y(*angle);
                BlockTransform {
                    position: start.position + rotation * Vec3::new(*radius, 0.0, *radius),
                    rotation,
                }
            }
            BlockType::Slope {
                length,
                height_change,
            } => BlockTransform {
                position: start.position + start.rotation * Vec3::new(0.0, *height_change, *length),
                rotation: start.rotation,
            },
        }
    }

    #[test]
    fn test_consecutive_segments_connect() {
        let track = Track::generate(123, 10.0);

        for i in 0..track.segments.len() - 1 {
            let current = &track.segments[i];
            let next = &track.segments[i + 1];

            let computed_end = calculate_end(&current.block_type, current.transform);

            assert_relative_eq!(
                computed_end.position.x,
                next.transform.position.x,
                epsilon = EPSILON
            );
            assert_relative_eq!(
                computed_end.position.y,
                next.transform.position.y,
                epsilon = EPSILON
            );
            assert_relative_eq!(
                computed_end.position.z,
                next.transform.position.z,
                epsilon = EPSILON
            );
        }
    }

    #[test]
    fn test_net_downward_slope() {
        let track = Track::generate(456, 15.0);
        assert!(
            track.current_end.position.y < 0.0,
            "Track should end below starting height"
        );

        let total_height_change =
            track.current_end.position.y - track.segments.first().unwrap().transform.position.y;
        assert!(
            total_height_change < -5.0,
            "Significant downward slope required. Actual: {}",
            total_height_change
        );
    }

    #[test]
    fn no_overlapping_positions() {
        let track = Track::generate(789, 12.0);
        let mut positions = Vec::new();

        for segment in &track.segments {
            positions.push(segment.transform.position);
        }
        positions.push(track.current_end.position);

        // Check all pairwise combinations
        for i in 0..positions.len() {
            for j in (i + 2)..positions.len() {
                let distance = positions[i].distance(positions[j]);
                assert!(
                    distance > 3.0,
                    "Potential overlap between positions {} and {} (distance: {})",
                    i,
                    j,
                    distance
                );
            }
        }
    }

    #[test]
    fn valid_rotations() {
        let track = Track::generate(321, 8.0);

        for segment in &track.segments {
            // Check rotation is normalized
            assert_relative_eq!(segment.transform.rotation.length(), 1.0, epsilon = EPSILON);

            // Check pitch doesn't exceed 45 degrees
            let (yaw, pitch, roll) = segment.transform.rotation.to_euler(EulerRot::YXZ);
            assert!(
                pitch.abs() < FRAC_PI_2 / 2.0,
                "Excessive pitch angle: {} radians",
                pitch
            );
        }
    }

    #[test]
    fn point_rotation() {
        let r = rotate_point_around(Vec2::ZERO, Vec2::new(0.0, 1.0), 0.0);
        assert_relative_eq!(r.x, 0.0);
        assert_relative_eq!(r.y, 0.0);
        let r = rotate_point_around(Vec2::ZERO, Vec2::new(0.0, 1.0), PI);
        assert_relative_eq!(r.x, 0.0);
        assert_relative_eq!(r.y, 2.0);
    }
}
