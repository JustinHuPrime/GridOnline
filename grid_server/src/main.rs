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
use rand::{Rng, distributions::Alphanumeric, thread_rng};
use tokio::{
    net::TcpListener,
    sync::{Barrier, Mutex},
};

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
                let player_names: Vec<String> = connections.keys().cloned().collect();

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

        let mut disconnected_players = Vec::new();

        for (i, (username, connection)) in connections.iter_mut().enumerate() {
            let player_state = game_state.state_for(i);
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
    fn reset_to_lobby(&mut self, num_players: usize) {
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
        .map(|_| thread_rng().sample(Alphanumeric) as char)
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
        .with_state((server_state, Arc::new(Barrier::new(args.num_players))));

    let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
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
    State((state, next_state)): State<(Arc<Mutex<ServerState>>, Arc<Barrier>)>,
) -> Response {
    println!("New WebSocket connection established from {}", addr);
    ws.on_upgrade(move |socket| handle_websocket(socket, state, next_state))
}

async fn handle_websocket(
    socket: WebSocket,
    state: Arc<Mutex<ServerState>>,
    next_state: Arc<Barrier>,
) {
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
    let [attempt_join_code, username] = *login.as_slice() else {
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
            // check join code
            if join_code != attempt_join_code {
                drop(state_guard);
                let _ = send.send(Message::text("join code")).await;
                return;
            }

            // Check if game is full
            if connections.len() >= *num_players {
                drop(state_guard);
                let _ = send.send(Message::text("game full")).await;
                return;
            }

            // Check if username is already taken
            if connections.contains_key(username) {
                drop(state_guard);
                let _ = send.send(Message::text("username taken")).await;
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
            }
        }
        ServerState::Running {
            game_state,
            connections,
            join_code,
        } => {
            // Check join code
            if join_code != attempt_join_code {
                drop(state_guard);
                let _ = send.send(Message::text("join code")).await;
                return;
            }

            // Check if username is already in the game
            let player_names = game_state.get_player_names();
            let Some(player_index) = player_names.iter().position(|name| name == username) else {
                drop(state_guard);
                let _ = send.send(Message::text("full")).await;
                return;
            };

            // Check if username is already connected
            if connections.contains_key(username) {
                drop(state_guard);
                let _ = send.send(Message::text("username")).await;
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
        // Wait at next_state barrier for next round
        next_state.wait().await;

        // Check if it's the current player's turn
        let state_guard = state.lock().await;
        let ServerState::Running { game_state, .. } = &*state_guard else {
            unreachable!();
        };
        let current_player = game_state.current_player();
        let is_current_player = current_player.0 == username;
        let current_player_has_cards = current_player.1.has_cards();
        let current_player_has_won = game_state.current_player_has_won();
        drop(state_guard);

        if is_current_player {
            // if current player has no cards, skip the current player's turn and rebroadcast state
            if !current_player_has_cards {
                let mut state_guard = state.lock().await;
                let ServerState::Running { game_state, .. } = &mut *state_guard else {
                    unreachable!();
                };
                game_state.skip_player();
                state_guard.broadcast_state().await;
                drop(state_guard);
            } else if current_player_has_won {
                // Disconnect everyone
                let mut state_guard = state.lock().await;
                let ServerState::Running {
                    connections,
                    game_state,
                    ..
                } = &mut *state_guard
                else {
                    unreachable!();
                };

                let winner_message = end_of_game(username);
                let to_disconnect = connections.keys().cloned().collect::<Vec<_>>();
                let num_players = game_state.get_player_names().len();

                for username in to_disconnect {
                    let _ = state_guard
                        .server_disconnect(&username, winner_message.clone())
                        .await;
                }

                // Reset server to lobby for next game
                state_guard.reset_to_lobby(num_players);
                return;
            } else {
                // Wait for a PlayerMove from current player
                let Some(Ok(Message::Text(text))) = recv.next().await else {
                    state
                        .lock()
                        .await
                        .server_disconnect(username, protocol_error)
                        .await;
                    return;
                };
                let Ok(player_move) = serde_json::from_str::<PlayerMove>(&text) else {
                    state
                        .lock()
                        .await
                        .server_disconnect(username, protocol_error)
                        .await;
                    return;
                };

                // Try to apply the move
                let mut state_guard = state.lock().await;
                let ServerState::Running { game_state, .. } = &mut *state_guard else {
                    unreachable!();
                };
                let move_valid = game_state.apply_move(player_move);

                if !move_valid {
                    // Invalid move, disconnect player
                    state
                        .lock()
                        .await
                        .server_disconnect(username, protocol_error)
                        .await;
                    return;
                }

                // Broadcast updated game state to all players
                state_guard.broadcast_state().await;
                drop(state_guard);
            }
        } else if current_player_has_won {
            return;
        }
    }
}
