# Architecture

This document provides a description of the design of the GemGame software from a technical standpoint. For a description of the high-level gameplay and intended user experience, please see the `GAMEPLAY.md` document.

## Overview

* As described in the `README.md` document, GemGame is an online multiplayer browser game where players may together explore a procedurally-generated game world in search of precious gems and artefacts.
* Gameplay takes place in the context of a tile-based game map that spans infinitely in all directions (made possible by splitting the map into 16x16 chunks of tiles which are loaded, unloaded, and procedurally-generated as necessary).

## Infrastructure/Hosting

* The application is containerise using Docker into 3 main components:
  * Game server application powered by Tokio.
  * PostgreSQL database.
  * NGINX server serving the WASM client application, HTML & JavaScript documents, and asset files.
* These containers are managed using Docker Compose (`docker-compose up` and `docker-compose down` commands).
* The 3 containers are hosted on an AWS EC2 t3.micro instance running Ubuntu Server in Stockholm. AWS EBS gp3 will be used for storage.
* The Europe North region was chosen as it includes the more powerful t3 instance in the AWS free tier. When my free tier expires, the server will probably be moved to either US East or Europe West.
* The current design would not easily facilitate horizontal scaling, however many concurrently players is unlikely so that (hopefully) will not be an issue.

## Client

* The client application is written primarily in Rust (compiled to WebAssembly) using the MacroQuad library for rendering. A small amount of JavaScript code is used for the purpose of interfacing with the browser's WebSocket API and for accessing local storage.

### Map Rendering

* Each frame, only the tiles and entities that are on-screen are rendered.
* For the rendering of tile, each tile is iteratively drawn using the MacroQuad texture drawing functions with MacroQuad left to handle geometry batching, etc.
* The rendering of entities is slightly more complex:
  * All player entity bodies are drawn are drawn at the origin of each entity first. This is done separately to the rendering of entity heads, hair, etc. so as to prevent the head of an entity directly in front of another entity from being drawn behind the other entity's body (entities are 1.5 tiles tall).
  * Next the upper halves of player entities are drawn.
    * Note that if the entity in question is moving and at an animation frame where an arm is shown extended forward then the following should all be drawn a sixteenth of a tile lower than specified.
    * First an entity's head and hands are drawn a quarter of a tile above the entity's origin. This draw is coloured based on the player's skin colour choice.
    * Hair is drawn 7 eighths a tile above the origin and is coloured based on the player's choice of hair colour.
    * Next the right eye is drawn 3 quarters of the tile above the origin while the left eye is flipped and position half a tile right of the left eye. These draws are to be coloured based on the player's choice of hair colour.
    * An entity's mouth is drawn a quarter of a tile above and a quarter of a tile right of the entity's origin point.

## Server

* The server is built on top of Tokio using Tungstenite for handling WebSocket connections.
* The server features a simple main loop that listens for incoming TCP/IP connections. When a connection is received, a new Tokio task is spawned to handle that connection.

### Connection Tasks

* Once a TCP/IP connection is established with a client, a dedicated Tokio task is created to handle it.
* The first duty of each connection task is to perform the TCP/IP and WebSocket handshakes with the client and then the exchange of 'hello' and 'welcome' messages (see the Handshake subsection below).

### Tracking Map Changes

* The game map is stored and shared between all connection tasks/threads using a mutex wrapped inside of an atomically reference-counted object (`std::sync::Arc`).
* Whenever a task wishes to modify the game world, it must do two things:
  * Lock the game world mutex for writing and make the desired changes.
  * Send message(s) on the Tokio broadcast channel informing other tasks of changes made.
* In addition to polling the WebSocket connection, each task must also poll the Tokio broadcast channel in order to check for changes to the game world. If those changes are relevant to that task's client (i.e. they're changes to chunks that that client has loaded) then that task's client must be sent messages via the WebSocket connection informing them of said changes.

## Network Protocol

* All messages between clients and the server are sent via the WebSocket protocol and encoded using Bincode.
* Messages from a client to the server take the form of a variant of the `ToServer` enum while messages from the server to the client use the `FromServer` enum (see the `messages` module in the `shared` library).

### Handshake

* The TCP and WebSocket handshakes must be complete upon establishing a connection.
* The client must then send a 'hello' message (`ToServer::Hello` variant). If this the client has played before then they may provide a client ID along with this message (see the following subsection).
* After receiving a 'hello' message, the server replies with a 'welcome' message (`FromServer::Welcome` variant). If a client ID is provided it will be looked up in the database (see the following subsection). The 'welcome' message will include the server's version as well as the client's ID and their player entity.

### Returning Clients

* Players may continue their game through a system making use of browser local storage (stored using `window.localStorage`) or filesystem storage when playing via the desktop application (stored in a text file simply called `clientid.txt`).
* When a client connects without providing an existing client ID, the sever generates a new ID and a new player entity. These are then inserted into the database before being returned to the player.
* A client can connect and provide a client ID to the server. If that client ID is found in the database, the corresponding entity is returned to the client. Otherwise, the provided ID is discarded and the server treats the client as if it were a new one.
* Whenever a returning client connects, the server updates their corresponding database record with the current time. This is done so that records for players who go some amount of time without playing can be removed from the database.

### Player Movement

* A client can move its player entity by sending a `ToServer::MoveMyEntity { request_number, direction }` message to the server.
* The client should check if the their player can actually move in the specified direction first (i.e. no blocking tiles or entities in the way).
* The client should then, immediately after sending a `MoveMyEntity` message, play the movement animation and locally update the player entity's position without waiting for the server's response.
* The client should incrementally number each movement message sent to the server (provide with `request_number` field) as well as keep track of what it expects the position of the player entity to be after each movement.
* The client should only send a new `MoveMyEntity` message after any on-going movement animations are complete. Note that rapidly sending movement messages will not allow a player to move any quicker as movement speed is limited on the server side so as to prevent cheating.
* When the server receives a `MoveMyEntity` message it must perform a few checks before responding with a `FromServer::YourEntityMoved { request_number, new_position }` message.
* The server must keep track of the last point in time that each player entity moved so as to prevent cheaters from modifying their client to send many `MoveMyEntity` messages in an effort to move quicker than other players. If the server receives a `MoveMyEntity` message from a client earlier than expected/allowed then the movement should be queued to run as soon as the required amount of time has passed.
* The server should not trust the client to only send valid movements and should therefore check that the direction the client wishes to move in is clear of blocking tiles and other entities. If it is, the client's player entity's coordinates should be updated accordingly.
* When a server routine/task changes a player entity's coordinates it should update all other tasks of that change using the world modification multi-producer, multi-consumer channels so that those tasks may inform their respective remote clients as necessary (using `FromServer::EntityMoved` messages).
* The server should include the same `request_number` value with its `YourEntityMoved` response message as was included in the `MoveMyEntity` message that triggered the movement process. This is so that the client may ensure that each prediction of the server's response made was correct. If a client finds that the position it believes its player entity would be at for a given `request_number` differs from the position specified by the received `YourEntityMoved` message, it should disregard its prediction and locally set the entity's position to that specified by the server.
