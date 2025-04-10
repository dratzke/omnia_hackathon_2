# Set the default shell to bash
set shell := ["bash", "-c"]

# Default recipe to show available commands
default:
    @just --list

# Build all binaries
build:
    cargo build --bin server --release
    cargo build --bin client --release

# Run the server binary
server:
    cargo run --bin server --release -- --auth-port 5001 --game-port 5000

# Run the client binary
client:
    cargo run --bin client --release -- --auth-port 5001 --server 127.0.0.1 --client-port 4000

# Clean the project
clean:
    cargo clean

# Run tests for all binaries
test:
    cargo test --release

# Check code formatting
format-check:
    cargo fmt -- --check

# Format code
format:
    cargo fmt

# Run clippy for all binaries
clippy:
    cargo clippy -- -D warnings

run-both-grpc: build
    ./target/release/server --auth-port 5001 --game-port 5000 & 
    echo $$ > server.pid 
    sleep 2 
    ./target/release/client --auth-port 5001 --server 127.0.0.1 --client-port 4000 --grpc-port 50051

run-both: build
    # Start server in background, save PID, wait 5s, run client, then kill server
    ./target/release/server --auth-port 5001 --game-port 5000 & 
    echo $$ > server.pid 
    sleep 2 
    ./target/release/client --auth-port 5001 --server 127.0.0.1 --client-port 4000 &
    echo $$ > client1.pid
    sleep 2
    ./target/release/client --auth-port 5001 --server 127.0.0.1 --client-port 4001
    pkill -F server.pid
    pkill -F client1.pid
    rm server.pid
    rm client1.pid

fowrard:
    grpcurl -d '{"forward": true, "back": false, "left": false, "right": false, "reset": false}' -plaintext '[::1]:50051' marble.MarbleService/Input
back:
    grpcurl -d '{"forward": false, "back": true, "left": false, "right": false, "reset": false}' -plaintext '[::1]:50051' marble.MarbleService/Input
left:
    grpcurl -d '{"forward": false, "back": false, "left": true, "right": false, "reset": false}' -plaintext '[::1]:50051' marble.MarbleService/Input
right:
    grpcurl -d '{"forward": false, "back": false, "left": false, "right": true, "reset": false}' -plaintext '[::1]:50051' marble.MarbleService/Input
reset:
    grpcurl -d '{"forward": false, "back": false, "left": false, "right": true, "reset": false}' -plaintext '[::1]:50051' marble.MarbleService/Input
