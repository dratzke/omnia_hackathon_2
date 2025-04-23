import pandas as pd
from PIL import Image
import os

import subprocess
import shlex
from typing import Optional
import time


def save_images_from_dataframe(
    df: pd.DataFrame,
    width: int = 1280,
    height: int = 720,
    output_dir: str = "output_images",
    prefix: str = 'image_'
) -> None:
    """
    Saves images from a DataFrame's 'screen' column (containing image bytes)
    to PNG files with alphabetically sortable filenames.

    Args:
        df (pd.DataFrame): DataFrame with a 'screen' column (bytes)
        output_dir (str): Directory to save the PNG files.
    """
    os.makedirs(output_dir, exist_ok=True)

    for index, row in df.iterrows():
        image_bytes = row['screen']

        base_filename = f"image_{index:05d}"

        filename = f"{base_filename}.png"
        filepath = os.path.join(output_dir, filename)

        try:
            image = Image.frombuffer('RGBA', (width, height), image_bytes, 'raw', 'RGBA', 0, 1)
            image.save(filepath, format='PNG')
            print(f"Saved {filepath}")

        except Exception as e:
            print(f"Error processing row {index}: {e}")


def start_server_process(
    auth_port: int,
    game_port: int,
    players: int,
    max_game_seconds: int,
    server_executable: str = "../server"
) -> Optional[subprocess.Popen]:
    """
    Starts the server executable as a background process with specified arguments.

    Args:
        auth_port: The port number for authentication.
        game_port: The port number for the game.
        players: The number of players required.
        max_game_seconds: The maximum duration of a game in seconds.
        server_executable: The path to the server executable file.

    Returns:
        A subprocess.Popen object representing the started process,
        or None if the process could not be started.
    """
    command = [
        server_executable,
        "--auth-port", str(auth_port),
        "--game-port", str(game_port),
        "--players", str(players),
        "--max-game-seconds", str(max_game_seconds),
    ]

    try:
        process = subprocess.Popen(
            command,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        time.sleep(2)
        print(f"Started server process with PID: {process.pid}")
        print(f"Command: {' '.join(shlex.quote(arg) for arg in command)}")
        return process
    except FileNotFoundError:
        print(f"Error: Server executable not found at '{server_executable}'")
        return None
    except Exception as e:
        print(f"Error starting server process: {e}")
        return None


def start_client_process(
    auth_port: int,
    server_host: str,
    client_port: int,
    player_name: str,
    grpc_port: int,
    executable: str = "../client"
) -> Optional[subprocess.Popen]:
    """
    Starts the client application as a separate process.

    Args:
        auth_port: The port number for the authentication service.
        server_host: The hostname or IP address of the game server.
        client_port: The port number the client will use.
        player_name: The desired name for the player in the game.
        executable: The file path to the client executable. Defaults to
            '../target/release/client'.

    Returns:
        An optional `subprocess.Popen` object representing the started client
        process. Returns `None` if the executable cannot be found at the specified
        path or if any other error occurs during process creation.
    """
    command = [
        executable,
        "--auth-port", str(auth_port),
        "--server", server_host,
        "--client-port", str(client_port),
        "--grpc-port", str(grpc_port),
        "--name", player_name,
    ]

    try:
        process = subprocess.Popen(
            command,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        time.sleep(2)
        print(f"Started server process with PID: {process.pid}")
        print(f"Command: {' '.join(shlex.quote(arg) for arg in command)}")
        return process
    except FileNotFoundError:
        print(f"Error: Client executable not found at '{executable}'")
        return None
    except Exception as e:
        print(f"Error starting client process: {e}")
        return None
