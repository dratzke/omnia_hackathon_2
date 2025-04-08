use std::sync::Arc;

use tokio::sync::Mutex;
use tonic::{Response, Status};

use crate::{client_grpc_server::marble::StateResponse, protocol::Inputs};

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
}
