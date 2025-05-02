import logging
import os
from pathlib import Path

import util
import marble_client
import click
import subprocess
from typing import Optional
from concurrent.futures import ProcessPoolExecutor

log_level = logging.DEBUG

# Import the generated modules
logging.basicConfig(level=log_level, format='%(asctime)s - %(processName)s - %(levelname)s - %(message)s')


class SafeFormatter(logging.Formatter):
    def format(self, record):
        record.exc_type = getattr(record, 'exc_type', '')
        record.exc_msg = getattr(record, 'exc_msg', '')
        return super().format(record)


# Configure logging using the custom formatter
handler = logging.StreamHandler()

logger = logging.getLogger()
logger.handlers.clear()
logger.addHandler(handler)
logger.setLevel(log_level)


@click.command()
@click.option('--no-server', default=False, is_flag=True, help='Do not start the server')
@click.option('--clients', default=1, help='Number of clients to start')
@click.option('--game-seconds', default=30, help='Time the game runs until a winner is declared')
@click.option('--seed', default=1234, help='Seed for the game world generation')
@click.option('--server-headless', default=False, is_flag=True, help='Run the server in headless mode')
@click.option('--bin-path', default='../release/latest', help='Path to binaries')
@click.option('--competition', default=False, is_flag=True, help='Competition mode')
@click.option('--competition-server', default='172.20.11.63', help='Competition server host')
def run(
        no_server: bool,
        clients: int,
        game_seconds: int,
        seed: int,
        server_headless: bool,
        bin_path: str,
        competition: bool,
        competition_server: str
):
    if not no_server and not competition:
        logger.debug(f"Start local game server for {clients} player(s)")
        util.start_server_process(
            auth_port=4000,
            game_port=5000,
            players=clients,
            max_game_seconds=game_seconds,
            seed=seed,
            low_gpu=False,
            headless=server_headless,
            server_executable=build_executable(bin_path, 'server'))

    client_executable = build_executable(bin_path, 'client')
    if not competition:
        with ProcessPoolExecutor(max_workers=clients) as executor:
            list(executor.map(run_local_client, [(i, seed, client_executable) for i in range(clients)]))
    else:
        with ProcessPoolExecutor(max_workers=1) as executor:
            args = (client_executable, competition_server, seed)
            list(executor.map(run_competition_client, [args]))


def build_executable(bin_path: str | Path, bin_name: str) -> str:
    if isinstance(bin_path, str):
        bin_path = Path(bin_path)
    executable_suffix = '.exe' if os.name == 'nt' else ''
    return str(bin_path / f'{bin_name}{executable_suffix}')


def run_competition_client(args: (str, str, str)) -> Optional[subprocess.Popen]:
    executable_path, server_host, seed = args
    name = 'Penguballs'
    auth_port = 4000
    client_port = 5001
    grpc_port = 50051
    client = util.start_client_process(
        executable=executable_path,
        server_host=server_host,
        auth_port=auth_port,
        client_port=client_port,
        grpc_port=grpc_port,
        seed=seed,
        player_name=name,
        low_gpu=False
    )
    bot = marble_client.MarbleClient(
        host='localhost',
        port=str(grpc_port),
        screen_dir='raw_screens_competition',
        name=name)

    try:
        bot.run_interaction_loop()
    finally:
        if client is not None:
            logger.info("Kill competition client")
            client.kill()


def run_local_client(args: (int, int, str)) -> Optional[subprocess.Popen]:
    """
    Runs and connects to local game server. Please use 'run_competition_client' to connect to non-local game server
    """
    client_id, seed, executable_path = args
    name = 'A' + str(client_id)
    auth_port = 4000
    client_port = 5001 + client_id
    grpc_port = 50051 + client_id

    client = util.start_client_process(
        auth_port=auth_port,
        server_host='127.0.0.1',
        client_port=client_port,
        player_name=name,
        grpc_port=grpc_port,
        seed=seed,
        low_gpu=False,
        executable=executable_path)

    bot = marble_client.MarbleClient(
        host='localhost',
        port=str(grpc_port),
        screen_dir='raw_screens_' + str(client_id),
        name=name)
    try:
        bot.run_interaction_loop()
    finally:
        df = bot.get_records_as_dataframe()
        logger.debug(f"Store {len(df)} state records")
        df.to_parquet(f'marble_client_records_{client_id}.parquet', index=False)
        util.save_images_from_dataframe(df, f'output_images_{client_id}')
        if client:
            client.kill()
            logger.info(f'Client {client.pid} killed')
        else:
            logger.error('Client process failed to start or was None')


if __name__ == '__main__':
    run()
