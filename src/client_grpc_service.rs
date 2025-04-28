use std::{sync::Arc, time::Instant};

use bevy::math::Quat;
use tokio::sync::Mutex;
use tonic::{Response, Status};

use crate::{
    client_grpc_server::marble::{EmptyResponse, InputRequest, ResultEntry, StateResponse, Vec3},
    protocol::{Direction, Inputs},
};

pub struct GRPCService {
    pub screen: Arc<Mutex<Vec<u8>>>,
    pub current_input: Arc<Mutex<Inputs>>,
    pub finished: Arc<Mutex<bool>>,
    pub linear_velocity: Arc<Mutex<bevy::math::Vec3>>,
    pub angular_velocity: Arc<Mutex<bevy::math::Vec3>>,
    pub results: Arc<Mutex<Vec<ResultEntry>>>,
    pub last_used: Arc<Mutex<Instant>>,
}

impl GRPCService {
    pub async fn get_state(&self) -> Result<Response<StateResponse>, Status> {
        {
            let mut n = self.last_used.lock().await;
            *n = Instant::now();
        }
        let screen_copy = {
            let s = self.screen.lock().await;
            s.clone()
        };
        let finished = { *self.finished.lock().await };
        let lin = {
            let l = self.linear_velocity.lock().await;
            Vec3 {
                x: l.x,
                y: l.y,
                z: l.z,
            }
        };
        let ang = {
            let l = self.angular_velocity.lock().await;
            Vec3 {
                x: l.x,
                y: l.y,
                z: l.z,
            }
        };
        let results = { self.results.lock().await.clone() };
        let relative = angular_velocity_relative_to_movement(
            bevy::math::Vec3::new(ang.x, ang.y, ang.z),
            bevy::math::Vec3::new(lin.x, lin.y, lin.z),
        );
        Ok(Response::new(StateResponse {
            screen: screen_copy,
            linear_velocity: Some(lin),
            angular_velocity: Some(ang),
            relative_angular_velocity: Some(Vec3 {
                x: relative.x,
                y: relative.y,
                z: relative.z,
            }),
            finished,
            results,
        }))
    }

    pub async fn input(&self, r: InputRequest) -> Result<Response<EmptyResponse>, Status> {
        {
            let mut n = self.last_used.lock().await;
            *n = Instant::now();
        }
        let d = Direction {
            forward: r.forward,
            back: r.back,
            left: r.left,
            right: r.right,
            reset: r.reset,
        };
        let i = if d.is_some() {
            Inputs::Direction(d)
        } else {
            Inputs::None
        };
        let mut current = self.current_input.lock().await;
        *current = i;

        Ok(Response::new(EmptyResponse {}))
    }
}

fn angular_velocity_relative_to_movement(
    angular_velocity: bevy::math::Vec3,
    linear_velocity: bevy::math::Vec3,
) -> bevy::math::Vec3 {
    let Some(forward) = linear_velocity.try_normalize() else {
        return bevy::math::Vec3::ZERO;
    };

    let align_rotation = Quat::from_rotation_arc(bevy::math::Vec3::X, forward);

    let inverse_align_rotation = align_rotation.inverse();

    inverse_align_rotation * angular_velocity
}
