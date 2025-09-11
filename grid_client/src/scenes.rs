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

use dioxus::prelude::*;
use grid_common::{BOARD_SIZE, PlayerMove, PlayerVisibleGameState};

use crate::{ClientState, WEBSOCKET, display::Game, websocket::WebSocketClient};

#[component]
pub fn Join(state: Signal<ClientState>) -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut server_url = use_signal(|| "".to_string());
    let mut join_code = use_signal(|| "".to_string());
    let mut submitting = use_signal(|| false);
    let mut error_message: Signal<Option<String>> = use_signal(|| None);

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    rsx! {
        div { class: "container",
            h1 { class: "row mb-3", "Grid Online version {VERSION}" }
            div { class: "row mb-3",
                label {
                    r#for: "username",
                    class: "form-label col-sm-1 col-form-label",
                    "Username"
                }
                div { class: "col-sm-5",
                    input {
                        r#type: "text",
                        id: "username",
                        class: "form-control",
                        oninput: move |e| username.set(e.value()),
                    }
                }
            }
            div { class: "row mb-3",
                label {
                    r#for: "server-url",
                    class: "form-label col-sm-1 col-form-label",
                    "Server URL"
                }
                div { class: "col-sm-5",
                    input {
                        r#type: "text",
                        id: "server-url",
                        class: "form-control",
                        oninput: move |e| server_url.set(e.value()),
                    }
                }
            }
            div { class: "row mb-3",
                label {
                    r#for: "join-code",
                    class: "form-label col-sm-1 col-form-label",
                    "Join Code"
                }
                div { class: "col-sm-5",
                    input {
                        r#type: "password",
                        id: "join-code",
                        class: "form-control",
                        oninput: move |e| join_code.set(e.value()),
                    }
                }
            }
            if let Some(ref error) = *error_message.read() {
                div { class: "row",
                    p { class: "text-danger", "{error}" }
                }
            }
            button {
                class: "row btn btn-primary",
                r#type: "submit",
                onclick: move |_| {
                    submitting.set(true);
                    let Ok(mut client) = WebSocketClient::new(
                        &server_url.read(),
                        Some(format!("{}\n{}", username.read(), join_code.read())),
                    ) else {
                        error_message.set(Some("Couldn't connect to server".to_string()));
                        return;
                    };
                    client
                        .set_onmessage(
                            Some(
                                Box::new(move |message| {
                                    match message.as_str() {
                                        "ok" => {
                                            state.set(ClientState::WaitingForPlayers);
                                            WEBSOCKET
                                                .write()
                                                .as_mut()
                                                .expect("got message from socket")
                                                .set_onmessage(None);
                                        }
                                        "full" => {
                                            error_message.set(Some("No open seats".to_string()));
                                            *submitting.write() = false;
                                            *WEBSOCKET.write() = None;
                                        }
                                        "username" => {
                                            error_message
                                                .set(Some("Username already taken".to_string()));
                                            *submitting.write() = false;
                                            *WEBSOCKET.write() = None;
                                        }
                                        "join code" => {
                                            error_message.set(Some("Incorrect join code".to_string()));
                                            *submitting.write() = false;
                                            *WEBSOCKET.write() = None;
                                        }
                                        _ => {
                                            protocol_error(state);
                                        }
                                    }
                                }),
                            ),
                        );
                    client
                        .set_onerror(
                            Some(
                                Box::new(move |err| {
                                    state
                                        .set(
                                            ClientState::Error(format!("Connection lost\n{err:#?}")),
                                        );
                                }),
                            ),
                        );
                    *WEBSOCKET.write() = Some(client);
                },
                disabled: *submitting.read(),
                "Join Game"
            }
            div { class: "row",
                p {
                    "Grid is free software licenced under the "
                    a { href: "https://www.gnu.org/licenses/agpl.html",
                        "GNU Affero General Public License"
                    }
                    br {}
                    a { href: "https://github.com/JustinHuPrime/GridOnline",
                        "View the source code here"
                    }
                }
            }
        }
    }
}

#[component]
pub fn WaitingForPlayers(state: Signal<ClientState>) -> Element {
    WEBSOCKET
        .write()
        .as_mut()
        .expect("state transition guarded")
        .set_onmessage(Some(Box::new(move |message| {
            dispatch_next_game_state(state, message);
        })));
    rsx! {
        div { class: "container",
            h1 { "Waiting For Players..." }
        }
    }
}

#[component]
pub fn NotYourTurn(state: Signal<ClientState>, game_state: PlayerVisibleGameState) -> Element {
    WEBSOCKET
        .write()
        .as_mut()
        .expect("state transition guarded")
        .set_onmessage(Some(Box::new(move |message| {
            dispatch_next_game_state(state, message);
        })));
    rsx! {
        div { class: "container",
            div { class: "row",
                h1 { "{game_state.players[game_state.turn].0}'s turn" }
            }
            Game {
                game_state,
                on_hand_click: |_| {},
                on_board_click: |_| {},
            }
        }
    }
}

#[component]
pub fn YourTurn(state: Signal<ClientState>, game_state: PlayerVisibleGameState) -> Element {
    WEBSOCKET
        .write()
        .as_mut()
        .expect("state transition guarded")
        .set_onmessage(Some(Box::new(move |message| {
            dispatch_next_game_state(state, message);
        })));
    let mut to_play = use_signal(|| None);
    let mut sent = use_signal(|| false);

    rsx! {
        div { class: "container",
            div { class: "row",
                h1 { "Your turn" }
            }
            if !*sent.read()
                && game_state.board.0.iter().all(|row| row.iter().all(|card| card.is_none()))
            {
                Game {
                    game_state,
                    on_hand_click: move |index| {
                        WEBSOCKET
                            .write()
                            .as_mut()
                            .expect("state transition guarded")
                            .send(
                                &serde_json::to_string(
                                        &PlayerMove {
                                            card: index,
                                            location: (BOARD_SIZE / 2, BOARD_SIZE / 2),
                                        },
                                    )
                                    .expect("should always be able to serialize moves"),
                            );
                        *sent.write() = true;
                    },
                    on_board_click: |_| {},
                }
            } else if !*sent.read() {
                Game {
                    game_state,
                    to_play: *to_play.read(),
                    on_hand_click: move |index| {
                        let to_play = &mut *to_play.write();
                        match to_play {
                            Some(selected) if *selected == index => {
                                *to_play = None;
                            }
                            Some(_) | None => {
                                *to_play = Some(index);
                            }
                        }
                    },
                    on_board_click: move |location| {
                        if let Some(card) = *to_play.read() {
                            WEBSOCKET
                                .write()
                                .as_mut()
                                .expect("state transition guarded")
                                .send(
                                    &serde_json::to_string(&PlayerMove { card, location })
                                        .expect("should always be able to serialize moves"),
                                );
                            *sent.write() = true;
                        }
                    },
                }
            } else {
                Game {
                    game_state,
                    on_hand_click: |_| {},
                    on_board_click: |_| {},
                }
            }
        }
    }
}

#[component]
pub fn YouLost(game_state: PlayerVisibleGameState) -> Element {
    rsx! {
        div { class: "container",
            div { class: "row",
                h1 { "You lost ({game_state.players[game_state.turn].0}'s turn)" }
            }
            Game {
                game_state,
                on_hand_click: |_| {},
                on_board_click: |_| {},
            }
        }
    }
}

#[component]
pub fn YouWin(game_state: PlayerVisibleGameState) -> Element {
    rsx! {
        div { class: "container",
            div { class: "row",
                h1 { "You won" }
            }
            Game {
                game_state,
                on_hand_click: |_| {},
                on_board_click: |_| {},
            }
        }
    }
}

#[component]
pub fn Error(message: String) -> Element {
    rsx! {
        div { class: "container",
            h1 { "Something Went Wrong" }
            p { "{message}" }
            p {
                "To try again "
                a { href: "/", class: "btn btn-primary", "refresh the page" }
            }
        }
    }
}

fn protocol_error(mut state: Signal<ClientState>) {
    state.set(ClientState::Error(
        "Connection lost: protocol error".to_string(),
    ));
    *WEBSOCKET.write() = None;
}

fn dispatch_next_game_state(mut state: Signal<ClientState>, message: String) {
    let Ok(game_state) = serde_json::from_str::<PlayerVisibleGameState>(&message) else {
        protocol_error(state);
        return;
    };

    let Some((active_player, _)) = game_state.players.get(game_state.turn) else {
        protocol_error(state);
        return;
    };

    WEBSOCKET
        .write()
        .as_mut()
        .expect("state transition guarded")
        .set_onmessage(None);
    if *active_player == game_state.username {
        if game_state
            .players
            .iter()
            .all(|(player, cards)| game_state.username == *player || *cards == 0)
        {
            // if it's your turn and no-one else has cards, you win instead
            state.set(ClientState::YouWin(game_state));
        } else {
            state.set(ClientState::YourTurn(game_state));
        }
    } else {
        // cases where you aren't the active player
        if game_state
            .players
            .iter()
            .any(|(player, cards)| game_state.username == *player && *cards == 0)
        {
            // if it's not your turn and you don't have cards, you lost
            state.set(ClientState::YouLost(game_state));
        } else {
            state.set(ClientState::NotYourTurn(game_state));
        }
    }
}
