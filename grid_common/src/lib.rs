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

//! Common structure definitions for Grid Online

#![warn(missing_docs)]

use std::fmt::Debug;

use serde::{Deserialize, Serialize};

/// The size of the game board
pub const BOARD_SIZE: usize = 11;
/// Hand size
pub const HAND_SIZE: usize = 5;

/// Game state visible to a client
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[expect(missing_docs)]
pub struct ClientVisibleGameState {
    pub board: Board,
    pub hand: Hand,
    pub deck: Deck,
    pub username: String,
    pub players: Vec<(String, u32)>,
    pub turn: usize,
}

/// The game board
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Board(pub [[Option<Card>; BOARD_SIZE]; BOARD_SIZE]);

/// A hand of cards
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Hand(pub Vec<Card>);

/// A deck of cards
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Deck(pub Vec<Card>);

/// A card
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card(pub Suit, pub Value);

/// The suit of a card
#[expect(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Suit {
    #[serde(rename = "C")]
    Clubs,
    #[serde(rename = "D")]
    Diamonds,
    #[serde(rename = "H")]
    Hearts,
    #[serde(rename = "S")]
    Spades,
}

/// The value of a card
#[expect(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Value {
    #[serde(rename = "A")]
    Ace = 1,
    #[serde(rename = "2")]
    Two,
    #[serde(rename = "3")]
    Three,
    #[serde(rename = "4")]
    Four,
    #[serde(rename = "5")]
    Five,
    #[serde(rename = "6")]
    Six,
    #[serde(rename = "7")]
    Seven,
    #[serde(rename = "8")]
    Eight,
    #[serde(rename = "9")]
    Nine,
    #[serde(rename = "T")]
    Ten,
    #[serde(rename = "J")]
    Jack,
    #[serde(rename = "Q")]
    Queen,
    #[serde(rename = "K")]
    King,
}

#[cfg(test)]
mod tests {
    use super::*;
}
