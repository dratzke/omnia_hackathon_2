use std::sync::Arc;
use std::thread::JoinHandle;

use tokio::sync::Mutex;
use tonic::{Request, Response, Status, transport::Server};

// Import the generated proto code
pub mod marble {
    tonic::include_proto!("marble");
}

use marble::marble_service_server::{MarbleService, MarbleServiceServer};
use marble::{GetStateRequest, StateResponse};

use crate::client_grpc_service::GRPCService;
use crate::protocol::Inputs;

pub struct GRPCServer {
    pub service: GRPCService,
}

#[tonic::async_trait]
impl MarbleService for GRPCServer {
    async fn get_state(
        &self,
        _: Request<GetStateRequest>,
    ) -> Result<Response<StateResponse>, Status> {
        self.service.get_state().await
    }
}

pub fn start_gprc_server(
    screen_mutex: Arc<Mutex<Vec<u8>>>,
    current_input_mutex: Arc<Mutex<Inputs>>,
    grpc_port: u16,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let addr = format!("[::1]:{grpc_port}").parse().unwrap();
        let greeter = GRPCServer {
            service: GRPCService {
                screen: screen_mutex,
                current_input: current_input_mutex,
            },
        };

        println!("Server listening on {}", addr);

        rt.block_on(async {
            Server::builder()
                .add_service(MarbleServiceServer::new(greeter))
                .serve(addr)
                .await
                .unwrap();
        });
    })
}
