from io import BytesIO

import grpc
import pandas as pd
import logging
import time
import os
import uuid
import numpy as np
import math
import cv2 as cv

from PIL import Image

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

    def __init__(self, host: str, port: int, screen_dir: str, name: str):
        """
        Initializes the MarbleClient.

        Args:
            host: The hostname or IP address of the gRPC server.
            port: The port number of the gRPC server.
        """
        self.host = host
        self.port = port
        self.name = name
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
        # Calculate current speed
        lv = state.linear_velocity
        current_speed = math.sqrt(lv.x ** 2 + lv.y ** 2 + lv.z ** 2)

        print(f"Current Speed: {current_speed} m/s - Linear Velocity: X: {lv.x} Y: {lv.y} Z: {lv.z}")

        # Define your desired average speed
        TARGET_SPEED = 25.0  # m/s, adjust this as you like
        SPEED_TOLERANCE = 0.5  # m/s, how much deviation we allow

        # Initialize controls
        forward = False
        back = False
        left = False
        right = False
        reset = False

        image_features = self.estimate_ball_position_with_shadow(state.screen)
        if image_features['ball_status'] == 'off_track':
            reset = True
        elif image_features['ball_status'] == 'on_track':
            pass
        elif current_speed < (TARGET_SPEED - SPEED_TOLERANCE) and image_features['ball_status'] == 'on_track':
            # Too slow: speed up
            forward = True
        elif current_speed > (TARGET_SPEED + SPEED_TOLERANCE) and image_features['ball_status'] == 'on_track':
            # Too fast: slow down
            back = True
        else:
            # Within acceptable range: do nothing
            pass

        time.sleep(0.2)  # Keep a small delay to match the environment

        return service_pb2.InputRequest(
            forward=forward,
            back=back,
            left=left,
            right=right,
            reset=reset
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

    @staticmethod
    def extract_frame(screen_bytes, widht=800, height=600):
        # image = Image.open(BytesIO(screen_bytes), formats=['png'])
        image = Image.frombytes('RGBA', (widht, height), screen_bytes)
        frame = np.array(image)
        frame = cv.cvtColor(frame, cv.COLOR_RGB2BGR)
        return frame

    def estimate_ball_position_with_shadow(self, screen_bytes,
                                           ball_hsv_low=np.array([5, 80, 80]),
                                           ball_hsv_high=np.array([25, 255, 255]),
                                           kernel_size=9,
                                           min_peak_distance=100,
                                           canny_threshold1=50,
                                           canny_threshold2=100,
                                           shadow_threshold=50):

        """
        Estimates ball position using shadow detection as an additional cue for height.
        """

        # Load the image
        img_bgr = self.extract_frame(screen_bytes)
        img_rgb = cv.cvtColor(img_bgr, cv.COLOR_BGR2RGB)
        gray = cv.cvtColor(img_bgr, cv.COLOR_BGR2GRAY)
        h, w = img_bgr.shape[:2]

        # Detect the ball (same as before)
        hsv = cv.cvtColor(img_bgr, cv.COLOR_BGR2HSV)
        mask = cv.inRange(hsv, ball_hsv_low, ball_hsv_high)
        kernel = cv.getStructuringElement(cv.MORPH_ELLIPSE, (kernel_size, kernel_size))
        mask_clean = cv.morphologyEx(mask, cv.MORPH_OPEN, kernel)
        contours, _ = cv.findContours(mask_clean, cv.RETR_EXTERNAL, cv.CHAIN_APPROX_SIMPLE)

        ball_found = False
        cx_ball, cy_ball, r_ball = 0, 0, 0

        # Create visualization
        overlay = img_rgb.copy()

        # Find track walls using edge detection
        edges = cv.Canny(gray, canny_threshold1, canny_threshold2)
        col_sum = edges.sum(axis=0)
        peaks = np.argsort(col_sum)[::-1]
        x1 = peaks[0]
        x2 = next(idx for idx in peaks if abs(idx - x1) > min_peak_distance)
        if x1 > x2:
            x1, x2 = x2, x1
        cx_track = (x1 + x2) // 2

        # Draw track walls
        cv.line(overlay, (x1, 0), (x1, h), (0, 255, 0), 2)  # left wall
        cv.line(overlay, (x2, 0), (x2, h), (0, 255, 0), 2)  # right wall
        cv.line(overlay, (cx_track, 0), (cx_track, h), (255, 0, 255), 2)  # centerline

        # Variables to store classification results
        ball_status = "unknown"
        status = "Ball not found"
        shadow_distance = 0
        shadow_found = False

        if contours:
            ball_found = True
            largest = max(contours, key=cv.contourArea)
            (cx_ball, cy_ball), r_ball = cv.minEnclosingCircle(largest)
            cx_ball, cy_ball, r_ball = int(cx_ball), int(cy_ball), int(r_ball)

            cv.circle(overlay, (cx_ball, cy_ball), r_ball, (255, 0, 0), 3)

            # Check if ball is between track walls
            within_walls = (x1 <= cx_ball <= x2)

            # Look for shadow below the ball
            # Define region of interest for shadow detection
            shadow_roi_top = cy_ball + r_ball  # Start from bottom of ball
            shadow_roi_bottom = min(h, shadow_roi_top + int(r_ball * 3))  # Look up to 3*radius below
            shadow_roi_left = max(0, cx_ball - int(r_ball * 2))
            shadow_roi_right = min(w, cx_ball + int(r_ball * 2))

            shadow_roi = gray[shadow_roi_top:shadow_roi_bottom,
                         shadow_roi_left:shadow_roi_right]

            if shadow_roi.size > 0:
                # Threshold to find dark areas (potential shadows)
                _, shadow_mask = cv.threshold(shadow_roi, shadow_threshold, 255,
                                              cv.THRESH_BINARY_INV)

                # Find contours in the shadow mask
                shadow_contours, _ = cv.findContours(shadow_mask,
                                                     cv.RETR_EXTERNAL,
                                                     cv.CHAIN_APPROX_SIMPLE)

                # Draw the shadow ROI on the overlay
                cv.rectangle(overlay,
                             (shadow_roi_left, shadow_roi_top),
                             (shadow_roi_right, shadow_roi_bottom),
                             (0, 255, 255), 1)

                if shadow_contours:
                    # Find the largest dark area which is likely to be the shadow
                    shadow_contour = max(shadow_contours, key=cv.contourArea)

                    if cv.contourArea(shadow_contour) > (r_ball * r_ball * 0.2):  # Min shadow size
                        shadow_found = True
                        # Get shadow bounding box and center
                        x, y, w_s, h_s = cv.boundingRect(shadow_contour)
                        shadow_cy = y + h_s // 2 + shadow_roi_top
                        shadow_cx = x + w_s // 2 + shadow_roi_left

                        # Draw shadow contour and center
                        cv.drawContours(overlay, [shadow_contour], -1, (0, 0, 255), 2,
                                        offset=(shadow_roi_left, shadow_roi_top))
                        cv.circle(overlay, (shadow_cx, shadow_cy), 3, (0, 0, 255), -1)

                        # Calculate distance between ball bottom and shadow
                        ball_bottom = cy_ball + r_ball
                        shadow_distance = shadow_cy - ball_bottom

                        # Draw a line connecting ball bottom to shadow
                        cv.line(overlay, (cx_ball, ball_bottom),
                                (shadow_cx, shadow_cy), (255, 255, 0), 2)

                        # Add shadow distance information
                        cv.putText(overlay, f"Shadow dist: {shadow_distance}px",
                                   (10, 60), cv.FONT_HERSHEY_SIMPLEX, 0.7,
                                   (255, 255, 255), 2)

            # Classification with shadow information
            if not within_walls:
                status = "Ball out of track"
                ball_status = "out"
            elif shadow_found:
                if shadow_distance < r_ball * 1.2:
                    status = "Ball on track (shadow close)"
                    ball_status = "on_track"
                else:
                    status = f"Ball above track (shadow dist: {shadow_distance}px)"
                    ball_status = "off_track"
            else:
                # No shadow found - could be very high or lighting issue
                status = "Ball likely off track (no shadow)"
                ball_status = "likely_off_track"

        return {
            'ball_status': ball_status,
            'ball_found': ball_found,
            'ball_position': (cx_ball, cy_ball, r_ball) if ball_found else None,
            'shadow_found': shadow_found,
            'shadow_distance': shadow_distance if shadow_found else None,
            'track_walls': (x1, x2),
            'track_center': cx_track,
            'status': status,
            'visualization': overlay
        }
