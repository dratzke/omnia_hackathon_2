use std::sync::Arc;

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
}

impl GRPCService {
    pub async fn get_state(&self) -> Result<Response<StateResponse>, Status> {
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
        Ok(Response::new(StateResponse {
            screen: screen_copy,
            linear_velocity: Some(lin),
            angular_velocity: Some(ang),
            finished,
            results,
        }))
    }

    pub async fn input(&self, r: InputRequest) -> Result<Response<EmptyResponse>, Status> {
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
