# Marble Racing Bot

Welcome to the **Marble Racing Bot** project, developed to facilitate a hackathon!


---

## Features

- **Marble Control**: The bot manages the marble's movement through torque control, enabling it to navigate the track.
- **Screen Vision**: The challenge of the hackathon is to process the screen to determine how to navigate the track.
- **Dynamic Track Challenges**: It adapts to various track elements, including slippery ice, turns, and temporary modifiers.

---

## Architecture Overview

The project employs a **server-client model** with a **gRPC server** exposed by the client, facilitating seamless communication for bot gameplay.

### 1. **Server**
The server hosts the racing game environment and provides the following functionalities:
- Rendering the racing track and marbles.
- Generating real-time game states, including the track layout, marble positions, and environmental conditions (e.g., ice zones).
- Broadcasting game updates to connected clients.

### 2. **Client**
The client represents the bot's interface with the game and includes:
- A **gRPC server** that exposes endpoints to interact with the bot's logic.
- Logic to process incoming game state information and make decisions based on the bot's strategy.
- Control mechanisms to adjust the marble's torque and direction.

### 3. **gRPC Server**
The gRPC server serves as the communication bridge between the server and the bot. It handles:
- Receiving game state information from the server.
- Sending control commands (e.g., torque adjustments) to the server based on the bot's logic.
- Synchronizing gameplay in real-time to ensure smooth interactions.

---

## How It Works

1. **Initial Setup**: 
   - The server initializes the racing track and starts broadcasting the game state.
   - The client connects to the server and begins receiving track and marble data.

2. **Bot Decision-Making**:
   - The bot should process the screen exposed over gRPC to make decisions.
   - Based on the input, it computes torque adjustments and directional changes to navigate the track.
   - As part of the hackathon you will have to write this bot :)

3. **Gameplay Execution**:
   - Commands from the bot are transmitted to the client via the gRPC interface.
   - The server updates the game state based on the bot's actions and broadcasts the changes.

4. **Dynamic Challenges**:
   - The bot continuously adapts to changing conditions, such as slippery ice patches or avoiding collisions with other marbles.

---

## Getting Started

### Prerequisites
Download the release TODO LINK and extarct the zip file.
The zip contains the server and client binaries for linux and windows + the assets used for the ui.

On Linux you might need to install the following packages (ubuntu).

```bash
sudo apt-get install --no-install-recommends 'libasound2-dev' 'libudev-dev' 'libwayland-dev' 'libxkbcommon-dev' 'protobuf-compiler'
``
