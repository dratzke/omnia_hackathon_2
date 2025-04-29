import logging
import os
from pathlib import Path

from marble_neural_network import MarbleNeuralNetwork
import util
import marble_client
import click
import subprocess
from typing import Optional
from concurrent.futures import ProcessPoolExecutor
import torch.nn as nn
import torch
import pandas as pd

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

num_generations = 500
mutation_rate = 0.1
population_size = 10

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
    client_executable = bin_path / f'client{executable_suffix}'
    
    population = initialize_population(clients)
    for generation in range(num_generations):
        if not no_server:
            server = util.start_server_process(4000, 5000, clients, game_seconds, seed, False, server_headless,
                                            server_executable=str(server_executable))
        print("Generation:", generation + 1)
        best_individual = None
        
        args = [(idx, seed, str(client_executable), individual) for idx, individual in enumerate(population)]

        # Prepare arguments
        args = [(idx, seed, str(client_executable), individual) for idx, individual in enumerate(population)]

        # Run in parallel
        with ProcessPoolExecutor() as executor:
            results = list(executor.map(run_client, args))  # Each result is a df

        # Evaluate fitness
        fitness_scores = [fitness_function(df) for df in results]
        best_idx = max(range(len(fitness_scores)), key=lambda i: fitness_scores[i])
        best_individual = population[best_idx]
        best_fitness = fitness_scores[best_idx]
         
        print(f"Best fitness: {best_fitness}")

        # Create new population by mutating best
        new_population = [mutate(best_individual) for _ in range(clients)]
        new_population[0] = best_individual  # Elitism
        population = new_population
    
        if server:
            server.kill()


def fitness_function(df):
    last_data_frame = df.iloc[-1]

    client_name = last_data_frame["client_name"]
    
    result_list = last_data_frame["results"]

    bot_result = next(n for n in result_list if n['name']==client_name)
    
    bot_finish_time = bot_result['finish_time']
    bot_last_touched_road_id = bot_result['last_touched_road_id']
    bot_last_touched_road_time = bot_result['last_touched_road_time']
     
    max_finish_time = max(item['finish_time'] for item in result_list)
    max_last_touched_road_id = max(item['last_touched_road_id'] for item in result_list)
    max_last_touched_road_time = max(item['last_touched_road_time'] for item in result_list)

    score = 0
    if max_finish_time > 0:
        score =score + 0.5*bot_finish_time/max_finish_time
    if max_last_touched_road_id > 0:
        score = score + 0.3 * bot_last_touched_road_id/max_last_touched_road_id
    if max_last_touched_road_time > 0:
        score = score + 0.2* bot_last_touched_road_time/max_last_touched_road_time

    return score

def crossover(parent1, parent2):
    child1 = MarbleNeuralNetwork()
    child2 = MarbleNeuralNetwork()
    child1.fc1.weight.data = torch.cat((parent1.fc1.weight.data[:16], parent2.fc1.weight.data[16:]), dim=0)
    child2.fc1.weight.data = torch.cat((parent2.fc1.weight.data[:16], parent1.fc1.weight.data[16:]), dim=0)
    return child1, child2

def mutate(model):
    for param in model.parameters():
        if torch.rand(1).item() < mutation_rate:
            param.data += torch.randn_like(param.data) * 0.1  # Adding Gaussian noise with std=0.1
    return model

def run_client(args: (int, int, str, nn.Module)):
    client_id, seed, executable_path, neural_network = args
    name = 'A' + str(client_id)
    client = util.start_client_process(4000, '127.0.0.1', 5001 + client_id, name, 50051 + client_id, seed, False,
                                       executable=executable_path)

    bot = marble_client.MarbleClient('localhost', str(50051 + client_id), 'raw_screens_' + str(client_id), name, neural_network)
    df = pd.DataFrame()
    try:
        bot.run_interaction_loop()
    finally:
        df = bot.get_records_as_dataframe()
        #df.to_parquet(f'marble_client_records_{client_id}.parquet', index=False)
        #util.save_images_from_dataframe(df, f'output_images_{client_id}')

    if client:
        client.kill()
        logger.info(f'Client {client.pid} killed')
    else:
        logger.error('Client process failed to start or was None')
        
    return df


def initialize_population(population_size):
    population = []
    for i in range(population_size):
        model = MarbleNeuralNetwork()
        population.append(model)
    return population

if __name__ == '__main__':
    run()
