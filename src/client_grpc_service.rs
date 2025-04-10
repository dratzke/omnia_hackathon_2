use std::sync::Arc;

use tokio::sync::Mutex;
use tonic::{Response, Status};

use crate::{
    client_grpc_server::marble::{EmptyResponse, InputRequest, StateResponse},
    protocol::{Direction, Inputs},
};

pub struct GRPCService {
    pub screen: Arc<Mutex<Vec<u8>>>,
    pub current_input: Arc<Mutex<Inputs>>,
}

impl GRPCService {
    pub async fn get_state(&self) -> Result<Response<StateResponse>, Status> {
        let screen_copy = {
            let s = self.screen.lock().await;
            s.clone()
        };
        Ok(Response::new(StateResponse {
            screen: screen_copy,
        }))
    }

    pub async fn input(&self, r: InputRequest) -> Result<Response<EmptyResponse>, Status> {
        let d = Direction {
            forward: r.forward,
            back: r.back,
            left: r.left,
            right: r.right,
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
