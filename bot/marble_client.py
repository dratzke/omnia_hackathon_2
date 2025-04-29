import grpc
import pandas as pd
import logging
import time
import os
import uuid
import numpy as np
import math
import torch.nn as nn
import torch

# Note: You need to generate the Python protobuf files from your .proto file first.
# Run the following command in your terminal in the directory containing marble.proto:
# python -m grpc_tools.protoc -I. --python_out=. --grpc_python_out=. marble.proto
# This will create marble_pb2.py and marble_pb2_grpc.py.
try:
    from proto import service_pb2
    from proto import service_pb2_grpc
except ImportError:
    logging.error("Failed to import generated gRPC modules. "
                  "Did you run 'uv run python -m grpc_tools.protoc -I. --python_out=. --pyi_out=. --grpc_python_out=. proto/service.proto'?")
    exit(1)


class MarbleClient:
    """
    A gRPC client for interacting with the MarbleService.

    Connects to a MarbleService instance, allows getting state, sending input,
    running an interaction loop, and storing the state/input history
    in a pandas DataFrame.
    """

    def __init__(self, host: str, port: int, screen_dir: str, name: str, model: nn.Module):
        """
        Initializes the MarbleClient.

        Args:
            host: The hostname or IP address of the gRPC server.
            port: The port number of the gRPC server.
        """
        self.host = host
        self.port = port
        self.name = name
        self.model = model
        self.model.eval()
        # Create an insecure channel to connect to the server
        self.channel = grpc.insecure_channel(f'{self.host}:{self.port}')
        # Create a stub (client) for the MarbleService
        self.stub = service_pb2_grpc.MarbleServiceStub(self.channel)
        # List to store (state, input) tuples recorded during the loop
        self.records = []
        self.screen_dir = screen_dir
        os.makedirs(self.screen_dir, exist_ok=True)  # Ensure the directory exists
        print(f"MarbleClient initialized for {self.host}:{self.port}")

    def get_state(self) -> service_pb2.StateResponse:
        """
        Calls the GetState RPC method to retrieve the current state from the server.

        Returns:
            A StateResponse protobuf message.
        """
        try:
            request = service_pb2.GetStateRequest()
            response = self.stub.GetState(request)
            return response
        except grpc.RpcError as e:
            print(f"Error calling GetState: {e}")
            return None  # Or raise the exception

    def send_input(self, input_request: service_pb2.InputRequest) -> service_pb2.EmptyResponse:
        """
        Calls the Input RPC method to send user input to the server.

        Args:
            input_request: An InputRequest protobuf message.

        Returns:
            An EmptyResponse protobuf message.
        """
        try:
            response = self.stub.Input(input_request)
            return response
        except grpc.RpcError as e:
            print(f"Error calling Input: {e}")
            return None  # Or raise the exception

    def decision(self, state: service_pb2.StateResponse) -> service_pb2.InputRequest:
        """
        Determines the input to send based on the current state.

        Args:
            state: The current StateResponse message received from the server.

        Returns:
            An InputRequest protobuf message representing the desired action.

        Note:
            This function currently returns a default input (move forward).
            You should implement your logic here to decide the input based
            on the provided state information (e.g., screen data, velocity).
        """
        vec = [
            state.linear_velocity.x,
            state.linear_velocity.y,
            state.linear_velocity.z,
            state.angular_velocity.x,
            state.angular_velocity.y,
            state.angular_velocity.z,
            state.relative_angular_velocity.x,
            state.relative_angular_velocity.y,
            state.relative_angular_velocity.z
        ]
        input_tensor = torch.tensor(vec, dtype=torch.float32)

        # Get the neural network's prediction
        output = self.model(input_tensor)
        
        # Map the network output to the InputRequest
        return service_pb2.InputRequest(
            forward=output[0].item() > 0.5,
            back=output[1].item() > 0.5,
            left=output[2].item() > 0.5,
            right=output[3].item() > 0.5,
            reset=output[4].item() > 0.5
        )

    def run_interaction_loop(self):
        """
        Runs a loop that repeatedly gets state, determines input, sends input,
        and records the state/input pair.

        Args:
            iterations: The number of times to run the get_state/send_input cycle.
        """
        while True:
            current_state = self.get_state()
            if current_state is None:
                print("Failed to get state, stopping loop.")
                break

            # 2. Determine the input based on the state
            input_to_send = self.decision(current_state)

            # 3. Send the input
            response = self.send_input(input_to_send)
            if response is None:
                print("Failed to send input, stopping loop.")
                break

            # 4. Record the state and the input that was sent

            screen_file = os.path.join(self.screen_dir, f"screen_{uuid.uuid4()}")
            recorded_state = {
                'screen': screen_file,
                'linear_velocity': current_state.linear_velocity,
                'angular_velocity': current_state.angular_velocity,
                'relative_angular_velocity': current_state.relative_angular_velocity,
                'finished': current_state.finished,
                'results': current_state.results,
            }
            with open(screen_file, 'wb') as f:
                f.write(current_state.screen)

            self.records.append((recorded_state, input_to_send))
            if current_state.finished:
                for index, result in enumerate(current_state.results):
                    if result.name == self.name:
                        # Assuming result.name is a string, adjust as necessary
                        print(f"Result {index}: {result.name}, Finish Time: {result.finish_time}, "
                              f"Last Touched Road ID: {result.last_touched_road_id}, "
                              f"Last Touched Road Time: {result.last_touched_road_time}")
                print("Marble finished, stopping loop.")
                break

        print("Interaction loop finished.")

    def get_records_as_dataframe(self) -> pd.DataFrame:
        """
        Converts the recorded state/input pairs into a pandas DataFrame.

        Returns:
            A pandas DataFrame containing the recorded interaction history.
        """
        data = []
        for state, input_req in self.records:
            # Helper to handle optional fields in ResultEntry
            def get_optional_float(value):
                return value if value is not None else pd.NA

            def get_optional_uint64(value):
                return value if value is not None else pd.NA

            results_list = []
            if state['results']:
                results_list = [
                    {
                        'name': r.name,
                        'finish_time': get_optional_float(r.finish_time),
                        'last_touched_road_id': get_optional_uint64(r.last_touched_road_id),
                        'last_touched_road_time': get_optional_float(r.last_touched_road_time)
                    } for r in state['results']
                ]

            data.append({
                # State fields
                'screen': state['screen'],  # Keep as bytes, or process further if needed
                'linear_velocity_x': state['linear_velocity'].x,
                'linear_velocity_y': state['linear_velocity'].y,
                'linear_velocity_z': state['linear_velocity'].z,
                'angular_velocity_x': state['angular_velocity'].x,
                'angular_velocity_y': state['angular_velocity'].y,
                'angular_velocity_z': state['angular_velocity'].z,
                'relative_angular_velocity_x': state['relative_angular_velocity'].x,
                'relative_angular_velocity_y': state['relative_angular_velocity'].y,
                'relative_angular_velocity_z': state['relative_angular_velocity'].z,
                'finished': state['finished'],
                'client_name': self.name,
                'results': results_list,  # Store list of result dicts

                # Input fields
                'input_forward': input_req.forward,
                'input_back': input_req.back,
                'input_left': input_req.left,
                'input_right': input_req.right,
                'input_reset': input_req.reset
            })
        df = pd.DataFrame(data)
        return df

    def close(self):
        """Closes the gRPC channel."""
        if self.channel:
            self.channel.close()
            print("gRPC channel closed.")
