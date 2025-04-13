use std::sync::Arc;
use std::thread::JoinHandle;

use bevy::math::Vec3;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status, transport::Server};

// Import the generated proto code
pub mod marble {
    tonic::include_proto!("marble");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("service_descriptor");
}

use marble::marble_service_server::{MarbleService, MarbleServiceServer};
use marble::{EmptyResponse, GetStateRequest, InputRequest, ResultEntry, StateResponse};

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
    async fn input(&self, r: Request<InputRequest>) -> Result<Response<EmptyResponse>, Status> {
        self.service.input(r.into_inner()).await
    }
}

pub fn start_gprc_server(
    screen_mutex: Arc<Mutex<Vec<u8>>>,
    current_input_mutex: Arc<Mutex<Inputs>>,
    finished: Arc<Mutex<bool>>,
    linear_velocity: Arc<Mutex<Vec3>>,
    angular_velocity: Arc<Mutex<Vec3>>,
    results: Arc<Mutex<Vec<ResultEntry>>>,
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
                finished,
                linear_velocity,
                angular_velocity,
                results,
            },
        };
        let reflection = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(marble::FILE_DESCRIPTOR_SET)
            .build_v1()
            .unwrap();

        println!("Server listening on {}", addr);

        rt.block_on(async {
            Server::builder()
                .add_service(reflection)
                .add_service(MarbleServiceServer::new(greeter))
                .serve(addr)
                .await
                .unwrap();
        });
    })
}
