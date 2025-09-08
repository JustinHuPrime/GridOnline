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

//! Client for Grid Online

mod scenes;
mod websocket;

use dioxus::prelude::*;
use grid_common::PlayerVisibleGameState;

use crate::scenes::*;
use crate::websocket::WebSocketClient;

static WEBSOCKET: GlobalSignal<Option<WebSocketClient>> = Global::new(|| None);

enum ClientState {
    Error(String),
    Login,
    WaitingForPlayers,
    NotYourTurn(PlayerVisibleGameState),
    YourTurn(PlayerVisibleGameState),
    YouLost(PlayerVisibleGameState),
    YouWin(PlayerVisibleGameState),
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let state = use_signal(|| ClientState::Login);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/main.css") }
        document::Link {
            rel: "stylesheet",
            href: "https://cdn.jsdelivr.net/npm/bootstrap@5.3.7/dist/css/bootstrap.min.css",
            integrity: "sha384-LN+7fdVzj6u52u30Kp6M/trliBMCMKTyK833zpbD+pXdCLuTusPj697FH4R/5mcr",
            crossorigin: "anonymous",
        }
        document::Script {
            src: "https://cdn.jsdelivr.net/npm/bootstrap@5.3.7/dist/js/bootstrap.bundle.min.js",
            integrity: "sha384-ndDqU0Gzau9qJ1lfW4pNLlhNTkCfHzAVBReH9diLvGRem5+R9g2FzA8ZGN954O5Q",
            crossorigin: "anonymous",
        }
        document::Link {
            rel: "apple-touch-icon",
            sizes: "180x180",
            href: asset!("/assets/apple-touch-icon.png"),
        }
        document::Link {
            rel: "icon",
            r#type: "image/png",
            sizes: "32x32",
            href: asset!("/assets/favicon-32x32.png"),
        }
        document::Link {
            rel: "icon",
            r#type: "image/png",
            sizes: "16x16",
            href: asset!("/assets/favicon-16x16.png"),
        }
        document::Link { rel: "manifest", href: asset!("/assets/site.webmanifest") }
        match *state.read() {
            ClientState::Login => {
                rsx! {
                    Join { state }
                }
            }
            ClientState::Error(ref message) => {
                rsx! {
                    Error { message }
                }
            }
            ClientState::WaitingForPlayers => {
                rsx! {
                    WaitingForPlayers { state }
                }
            }
            ClientState::NotYourTurn(ref game_state) => {
                rsx! {
                    NotYourTurn { state, game_state: game_state.clone() }
                }
            }
            ClientState::YourTurn(ref game_state) => {
                rsx! {
                    YourTurn { state, game_state: game_state.clone() }
                }
            }
            ClientState::YouLost(ref game_state) => {
                rsx! {
                    YouLost { game_state: game_state.clone() }
                }
            }
            ClientState::YouWin(ref game_state) => {
                rsx! {
                    YouWin { game_state: game_state.clone() }
                }
            }
        }
    }
}
