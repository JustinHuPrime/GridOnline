// Copyright 2025 Justin Hu
//
// This file is part of Grid Online.
//
// Grid Online is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// Grid Online is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License
// for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with Grid Online. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Game server for Grid Online

mod model;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    Router,
    extract::{
        ConnectInfo, State,
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::get,
};
use clap::Parser;
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use rand::{distr::Alphanumeric, rng, seq::SliceRandom, Rng};
use tokio::{net::TcpListener, sync::Mutex};

use crate::model::{GameOptions, GameState};
use grid_common::PlayerMove;

#[derive(Parser)]
struct Args {
    #[clap(short)]
    num_players: usize,
    #[clap(short, long, default_value = "3030")]
    port: u16,
    #[clap(flatten)]
    options: GameOptions,
}

#[expect(clippy::large_enum_variant)]
enum ServerState {
    Lobby {
        options: GameOptions,
        num_players: usize,
        connections: HashMap<String, SplitSink<WebSocket, Message>>,
        join_code: String,
    },
    Running {
        game_state: GameState,
        connections: HashMap<String, SplitSink<WebSocket, Message>>,
        join_code: String,
    },
}
impl ServerState {
    /// Converts a Lobby state into a Running state
    ///
    /// Panics if state is already running
    async fn start(&mut self) {
        match self {
            ServerState::Lobby {
                options,
                connections,
                join_code,
                ..
            } => {
                // Extract player names from connections
                let mut player_names: Vec<String> = connections.keys().cloned().collect();
                player_names.shuffle(&mut rng());

                // Create the game state with the collected players
                let game_state = GameState::new(player_names, options.clone());

                // Convert to Running state by replacing self
                *self = ServerState::Running {
                    game_state,
                    connections: std::mem::take(connections),
                    join_code: join_code.clone(),
                };

                // Send game state to all players
                self.broadcast_state().await;
            }
            ServerState::Running { .. } => {
                panic!("Cannot start game: already running");
            }
        }
    }

    async fn broadcast_state(&mut self) {
        let ServerState::Running {
            game_state,
            connections,
            ..
        } = self
        else {
            panic!("tried to broadcast from a non-running server");
        };

        eprintln!(
            "broadcasting state to all {} believed-connected players",
            connections.len()
        );

        let mut disconnected_players = Vec::new();

        for (username, connection) in connections.iter_mut() {
            let player_state = game_state.state_for(
                game_state
                    .get_player_names()
                    .iter()
                    .position(|player_username| username == player_username)
                    .unwrap(),
            );
            let game_state_json = serde_json::to_string(&player_state).unwrap();

            if connection
                .send(Message::text(game_state_json))
                .await
                .is_err()
            {
                disconnected_players.push(username.clone());
            }
        }

        // Remove disconnected players
        for username in disconnected_players {
            self.lost_connection(&username);
        }
    }

    fn lost_connection(&mut self, username: &str) {
        let ServerState::Running { connections, .. } = self else {
            panic!("tried to disconnect from an non-running server");
        };
        eprintln!("disconnecting {username}");
        connections.remove(username);
    }

    async fn server_disconnect(&mut self, username: &str, reason: Message) {
        let ServerState::Running { connections, .. } = self else {
            panic!("tried to drop client from a non-running server");
        };
        let _ = connections
            .get_mut(username)
            .expect("should only drop connected players")
            .send(reason)
            .await;
        self.lost_connection(username);
    }

    /// Reset from Running state back to Lobby state for next game
    fn reset(&mut self, num_players: usize) {
        let ServerState::Running {
            game_state,
            join_code,
            ..
        } = self
        else {
            panic!("tried to reset a non-running server to lobby");
        };

        *self = ServerState::Lobby {
            options: game_state.get_options().clone(),
            num_players,
            join_code: join_code.clone(),
            connections: HashMap::new(),
        };
    }
}

fn generate_join_code() -> String {
    (0..16)
        .map(|_| rng().sample(Alphanumeric) as char)
        .collect()
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if !(2..=4).contains(&args.num_players) {
        eprintln!(
            "error: must have between 2 and 4 players, had {}",
            args.num_players
        );
        return;
    }

    println!("Grid Online server version {}", env!("CARGO_PKG_VERSION"));

    let join_code = generate_join_code();
    println!("Join code: {join_code}");
    let server_state = Arc::new(Mutex::new(ServerState::Lobby {
        options: args.options,
        num_players: args.num_players,
        join_code,
        connections: HashMap::new(),
    }));

    let app = Router::new()
        .route("/", get(websocket_handler))
        .with_state(server_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    println!("Starting WebSocket server on ws://{}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<Mutex<ServerState>>>,
) -> Response {
    eprintln!("New WebSocket connection established from {}", addr);
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: WebSocket, state: Arc<Mutex<ServerState>>) {
    let protocol_error = Message::Close(Some(CloseFrame {
        code: 4002,
        reason: "protocol error".into(),
    }));

    fn end_of_game(winner: &str) -> Message {
        Message::Close(Some(CloseFrame {
            code: 4000,
            reason: format!("player won\n{winner}").into(),
        }))
    }

    let (mut send, mut recv) = socket.split();

    let Some(Ok(Message::Text(login))) = recv.next().await else {
        let _ = send.send(protocol_error).await;
        return;
    };
    let login = login.split('\n').collect::<Vec<_>>();
    let [username, attempt_join_code] = *login.as_slice() else {
        let _ = send.send(protocol_error).await;
        return;
    };

    // login flow
    let mut state_guard = state.lock().await;
    match &mut *state_guard {
        ServerState::Lobby {
            num_players,
            connections,
            join_code,
            ..
        } => {
            eprintln!("{username:?} trying to join new game with code {attempt_join_code:?}");

            // check join code
            if join_code != attempt_join_code {
                drop(state_guard);
                let _ = send.send(Message::text("join code")).await;
                eprintln!("{username:?} rejected - bad join code");
                return;
            }

            // Check if game is full
            if connections.len() >= *num_players {
                drop(state_guard);
                let _ = send.send(Message::text("game full")).await;
                eprintln!("{username:?} rejected - game full");
                return;
            }

            // Check if username is already taken
            if let Some(connection) = connections.get_mut(username)
                && connection
                    .send(Message::Ping("live-check".into()))
                    .await
                    .is_ok()
            {
                drop(state_guard);
                let _ = send.send(Message::text("username taken")).await;
                eprintln!(
                    "{username:?} rejected - there is an existing connection for that username"
                );
                return;
            }

            // Send ok response
            if send.send(Message::text("ok")).await.is_err() {
                return;
            }

            // Add player to connections
            connections.insert(username.to_string(), send);

            // If game is full, start it
            if connections.len() == *num_players {
                state_guard.start().await;
                eprintln!("game starting");
            }
        }
        ServerState::Running {
            game_state,
            connections,
            join_code,
        } => {
            eprintln!("{username:?} trying to join existing game with code {attempt_join_code:?}");

            // Check join code
            if join_code != attempt_join_code {
                drop(state_guard);
                let _ = send.send(Message::text("join code")).await;
                eprintln!("{username:?} rejected - bad join code");
                return;
            }

            // Check if username is already in the game
            let player_names = game_state.get_player_names();
            let Some(player_index) = player_names.iter().position(|name| name == username) else {
                drop(state_guard);
                let _ = send.send(Message::text("full")).await;
                eprintln!("{username:?} rejected - game full");
                return;
            };

            // Check if username is already connected
            if let Some(connection) = connections.get_mut(username)
                && connection
                    .send(Message::Ping("live-check".into()))
                    .await
                    .is_ok()
            {
                drop(state_guard);
                let _ = send.send(Message::text("username")).await;
                eprintln!(
                    "{username:?} rejected - there is an existing connection for that username"
                );
                return;
            }

            // Send ok response
            if send.send(Message::text("ok")).await.is_err() {
                return;
            }

            // Send current game state to the reconnecting player
            let player_state = game_state.state_for(player_index);
            let game_state_json = serde_json::to_string(&player_state).unwrap();
            if send.send(Message::text(game_state_json)).await.is_err() {
                return;
            }

            // Add player to connections
            connections.insert(username.to_string(), send);
        }
    };
    drop(state_guard);

    // gameplay flow
    loop {
        // get a move
        let Some(Ok(Message::Text(text))) = recv.next().await else {
            state
                .lock()
                .await
                .server_disconnect(username, protocol_error)
                .await;
            eprintln!("disconnected {username:?} for sending a bad message");
            return;
        };

        // check if it's the current player's turn
        let mut state_guard = state.lock().await;
        let ServerState::Running { game_state, connections, .. } = &mut *state_guard else {
            unreachable!();
        };
        let current_player = game_state.current_player();
        if username != current_player.0 {
            // not the current player! protocol error!
            state_guard
                .server_disconnect(username, protocol_error)
                .await;
            eprintln!("disconnected {username:?} for playing a move out of turn");
            return;
        }

        // is current player - decode and try to apply the move
        let Ok(player_move) = serde_json::from_str::<PlayerMove>(&text) else {
            state_guard
                .server_disconnect(username, protocol_error)
                .await;
            eprintln!("disconnected {username:?} unable to parse move");
            return;
        };

        if !game_state.apply_move(player_move) {
            // Invalid move, disconnect player
            state_guard
                .server_disconnect(username, protocol_error)
                .await;
            eprintln!("disconnected {username:?} for playing a bad move");
            return;
        }

        if game_state.someone_has_won() {
            eprintln!("{username:?} has won");

            let winner_message = end_of_game(username);
            let to_disconnect = connections.keys().cloned().collect::<Vec<_>>();
            let num_players = game_state.get_player_names().len();

            for username in to_disconnect {
                let _ = state_guard
                    .server_disconnect(&username, winner_message.clone())
                    .await;
            }

            // Reset server to lobby for next game
            state_guard.reset(num_players);
            return;
        }

        // Broadcast updated game state to all players
        state_guard.broadcast_state().await;
        drop(state_guard);
    }
}
