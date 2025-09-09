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
use grid_common::HAND_SIZE;

#[component]
pub fn Game(
    game_state: grid_common::PlayerVisibleGameState,
    to_play: Option<usize>,
    on_hand_click: Callback<usize, ()>,
    on_board_click: Callback<(usize, usize), ()>,
) -> Element {
    rsx! {
        div { class: "row",
            div { class: "col-4",
                Board { board: game_state.board, on_board_click }
            }
            div { class: "col-2",
                Standings { standings: game_state.players }
            }
        }
        div { class: "row",
            div { class: "col-4",
                Hand { hand: game_state.hand, to_play, on_hand_click }
            }
            div { class: "col-8",
                Deck { deck: game_state.deck }
            }
        }
    }
}

#[component]
fn Board(board: grid_common::Board, on_board_click: Callback<(usize, usize), ()>) -> Element {
    rsx! {
        table { class: "user-select-none",
            for (row_n , row) in board.0.into_iter().enumerate() {
                tr {
                    for (card_n , card) in row.into_iter().enumerate() {
                        match card {
                            Some(card) => {
                                rsx! {
                                    td { style: "font-size:200%;color:{card.0.colour()}", "{card}" }
                                }
                            }
                            None => rsx! {
                                td {
                                    style: "font-size:200%;color:#888888",
                                    role: "button",
                                    onclick: move |_| on_board_click((row_n, card_n)),
                                    "🂠"
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Deck(deck: grid_common::Deck) -> Element {
    rsx! {
        p {
            span { class: "user-select-none",
                for card in deck.0.iter() {
                    span { style: "font-size:200%;color:{card.0.colour()}", "{card}" }
                }
            }
            br {}
            "({deck.0.len()} in deck)"
        }
    }
}

#[component]
fn Hand(
    hand: grid_common::Hand,
    to_play: Option<usize>,
    on_hand_click: Callback<usize, ()>,
) -> Element {
    rsx! {
        table { class: "user-select-none",
            tr {
                for index in 0..HAND_SIZE {
                    {
                        let card = hand.0.get(index);
                        match card {
                            Some(card) => rsx! {
                                td {
                                    style: "font-size:400%;color:{card.0.colour()}",
                                    role: "button",
                                    onclick: move |_| on_hand_click(index),
                                    if to_play.is_some_and(|to_play| to_play == index) {
                                        b { "{card}" }
                                    } else {
                                        "{card}"
                                    }
                                }
                            },
                            None => rsx! {
                                td { style: "font-size:400%;color:#888888", "🂠" }
                            },
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Standings(standings: Vec<(String, u32)>) -> Element {
    rsx! {
        table {
            for (player , count) in standings {
                tr {
                    td { "{player}: {count} cards" }
                }
            }
        }
    }
}
