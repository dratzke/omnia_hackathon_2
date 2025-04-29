import logging
import os
from pathlib import Path

import util
import marble_client
import click
import subprocess
from typing import Optional
from concurrent.futures import ProcessPoolExecutor

# Import the generated modules
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(processName)s - %(levelname)s - %(message)s')


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
logger.setLevel(logging.INFO)


@click.command()
@click.option('--no-server', default=False, is_flag=True, help='Do not start the server')
@click.option('--clients', default=1, help='Number of clients to start')
@click.option('--game-seconds', default=30, help='Time the game runs until a winner is declared')
@click.option('--seed', default=1234, help='Seed for the game world generation')
@click.option('--server-headless', default=False, is_flag=True, help='Run the server in headless mode')
@click.option('--bin-path', default='../release/latest', help='Path to binaries')
def run(no_server: bool, clients: int, game_seconds: int, seed: int, server_headless: bool, bin_path: str):
    if isinstance(bin_path, str):
        bin_path = Path(bin_path)
    executable_suffix = '.exe' if os.name == 'nt' else ''
    server_executable = bin_path / f'server{executable_suffix}'
    if not no_server:
        server = util.start_server_process(4000, 5000, clients, game_seconds, seed, False, server_headless,
                                           server_executable=str(server_executable))

    client_executable = bin_path / f'client{executable_suffix}'
    with ProcessPoolExecutor(max_workers=clients) as executor:
        list(executor.map(run_client, [(i, seed, str(client_executable)) for i in range(clients)]))
    if server:
        server.kill()


def run_client(args: (int, int, str)) -> Optional[subprocess.Popen]:
    client_id, seed, executable_path = args
    name = 'A' + str(client_id)
    client = util.start_client_process(4000, '127.0.0.1', 5001 + client_id, name, 50051 + client_id, seed, False,
                                       executable=executable_path)

    bot = marble_client.MarbleClient('localhost', str(50051 + client_id), 'raw_screens_' + str(client_id), name)
    try:
        bot.run_interaction_loop()
    finally:
        df = bot.get_records_as_dataframe()
        df.to_parquet(f'marble_client_records_{client_id}.parquet', index=False)
        util.save_images_from_dataframe(df, f'output_images_{client_id}')

    if client:
        client.kill()
        logger.info(f'Client {client.pid} killed')
    else:
        logger.error('Client process failed to start or was None')


if __name__ == '__main__':
    run()
