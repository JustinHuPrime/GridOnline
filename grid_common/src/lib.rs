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

use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// The size of the game board
pub const BOARD_SIZE: usize = 11;
/// Hand size
pub const HAND_SIZE: usize = 5;

/// Game state visible to a player
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[expect(missing_docs)]
pub struct PlayerVisibleGameState {
    pub board: Board,
    pub hand: Hand,
    pub deck: Deck,
    pub username: String,
    pub players: Vec<(String, u32)>,
    pub turn: usize,
}

/// A move a player can make
#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerMove {
    /// Which card, indexed from their hand
    pub card: usize,
    /// Where, as indexes into the board position
    pub location: (usize, usize),
}

/// The game board
///
/// Row-major order (i.e. innermost array = a row)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Board(pub [[Option<Card>; BOARD_SIZE]; BOARD_SIZE]);

impl Board {
    /// Check if a card can be played at the given position
    /// Returns true if the position is valid according to game rules:
    /// - If board is empty, only center position is valid
    /// - If board has cards, position must be adjacent to an existing card
    pub fn can_play_at(&self, row: usize, col: usize) -> bool {
        // Check bounds
        if row >= BOARD_SIZE || col >= BOARD_SIZE {
            return false;
        }

        // Check if position is already occupied
        if self.0[row][col].is_some() {
            return false;
        }

        // Check if board is empty
        let is_board_empty = self
            .0
            .iter()
            .all(|board_row| board_row.iter().all(|cell| cell.is_none()));

        if is_board_empty {
            // First move must be in center
            return row == BOARD_SIZE / 2 && col == BOARD_SIZE / 2;
        }

        // Board is not empty, check if position is adjacent to an existing card
        for dr in -1..=1 {
            for dc in -1..=1 {
                if dr == 0 && dc == 0 {
                    continue; // Skip the current position
                }
                let adj_row = row as i32 + dr;
                let adj_col = col as i32 + dc;

                // Check bounds and if there's a card at this adjacent position
                if adj_row >= 0
                    && adj_row < BOARD_SIZE as i32
                    && adj_col >= 0
                    && adj_col < BOARD_SIZE as i32
                    && self.0[adj_row as usize][adj_col as usize].is_some()
                {
                    return true;
                }
            }
        }

        false
    }
}

/// A hand of cards
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Hand(pub Vec<Card>);

/// A deck of cards
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Deck(pub Vec<Card>);

/// A card
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card(pub Suit, pub Value);
impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut character = match self.0 {
            Suit::Clubs => 0x1f0a0,
            Suit::Diamonds => 0x1f0b0,
            Suit::Hearts => 0x1f0c0,
            Suit::Spades => 0x1f0d0,
        };
        character |= match self.1 {
            Value::Ace => 0x1,
            Value::Two => 0x2,
            Value::Three => 0x3,
            Value::Four => 0x4,
            Value::Five => 0x5,
            Value::Six => 0x6,
            Value::Seven => 0x7,
            Value::Eight => 0x8,
            Value::Nine => 0x9,
            Value::Ten => 0xa,
            Value::Jack => 0xb,
            Value::Queen => 0xd,
            Value::King => 0xe,
        };
        write!(
            f,
            "{}",
            char::from_u32(character).expect("constructed from constants")
        )
    }
}

/// The suit of a card
#[expect(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
impl Suit {
    /// Get the display colour of this suit
    pub fn colour(&self) -> &'static str {
        match *self {
            Suit::Clubs | Suit::Spades => "#000000",
            Suit::Diamonds | Suit::Hearts => "#ff0000",
        }
    }
}

/// The value of a card
#[expect(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    fn create_empty_board() -> Board {
        Board([[None; BOARD_SIZE]; BOARD_SIZE])
    }

    fn create_board_with_center_card() -> Board {
        let mut board = create_empty_board();
        board.0[BOARD_SIZE / 2][BOARD_SIZE / 2] = Some(Card(Suit::Hearts, Value::Ace));
        board
    }

    #[test]
    fn test_can_play_at_empty_board_center() {
        let board = create_empty_board();
        let center = BOARD_SIZE / 2;

        // Center position should be valid on empty board
        assert!(board.can_play_at(center, center));
    }

    #[test]
    fn test_can_play_at_empty_board_non_center() {
        let board = create_empty_board();

        // Non-center positions should be invalid on empty board
        assert!(!board.can_play_at(0, 0)); // Corner
        assert!(!board.can_play_at(1, 1)); // Near corner
        assert!(!board.can_play_at(BOARD_SIZE / 2, BOARD_SIZE / 2 + 1)); // Adjacent to center
        assert!(!board.can_play_at(BOARD_SIZE / 2 + 1, BOARD_SIZE / 2)); // Adjacent to center
    }

    #[test]
    fn test_can_play_at_out_of_bounds() {
        let board = create_empty_board();

        // Out of bounds positions should be invalid
        assert!(!board.can_play_at(BOARD_SIZE, BOARD_SIZE));
        assert!(!board.can_play_at(BOARD_SIZE + 1, 0));
        assert!(!board.can_play_at(0, BOARD_SIZE + 1));
    }

    #[test]
    fn test_can_play_at_occupied_position() {
        let board = create_board_with_center_card();
        let center = BOARD_SIZE / 2;

        // Occupied position should be invalid
        assert!(!board.can_play_at(center, center));
    }

    #[test]
    fn test_can_play_at_orthogonal_adjacency() {
        let board = create_board_with_center_card();
        let center = BOARD_SIZE / 2;

        // Orthogonally adjacent positions should be valid
        assert!(board.can_play_at(center - 1, center)); // North
        assert!(board.can_play_at(center + 1, center)); // South
        assert!(board.can_play_at(center, center - 1)); // West
        assert!(board.can_play_at(center, center + 1)); // East
    }

    #[test]
    fn test_can_play_at_diagonal_adjacency() {
        let board = create_board_with_center_card();
        let center = BOARD_SIZE / 2;

        // Diagonally adjacent positions should be valid
        assert!(board.can_play_at(center - 1, center - 1)); // Northwest
        assert!(board.can_play_at(center - 1, center + 1)); // Northeast
        assert!(board.can_play_at(center + 1, center - 1)); // Southwest
        assert!(board.can_play_at(center + 1, center + 1)); // Southeast
    }

    #[test]
    fn test_can_play_at_non_adjacent() {
        let board = create_board_with_center_card();
        let center = BOARD_SIZE / 2;

        // Non-adjacent positions should be invalid
        assert!(!board.can_play_at(0, 0)); // Far corner
        assert!(!board.can_play_at(center - 2, center)); // Two spaces north
        assert!(!board.can_play_at(center + 2, center)); // Two spaces south
        assert!(!board.can_play_at(center, center - 2)); // Two spaces west
        assert!(!board.can_play_at(center, center + 2)); // Two spaces east
        assert!(!board.can_play_at(center - 2, center + 1)); // Knight's move pattern
    }

    #[test]
    fn test_can_play_at_chaining() {
        let mut board = create_board_with_center_card();
        let center = BOARD_SIZE / 2;

        // Add a second card adjacent to center
        board.0[center][center + 1] = Some(Card(Suit::Spades, Value::Two));

        // Now positions adjacent to the second card should be valid
        // even if they're not adjacent to the center
        assert!(board.can_play_at(center, center + 2)); // East of second card
        assert!(board.can_play_at(center - 1, center + 1)); // North of second card
        assert!(board.can_play_at(center + 1, center + 1)); // South of second card

        // But positions not adjacent to any card should still be invalid
        assert!(!board.can_play_at(center - 3, center - 3)); // Isolated position
    }
}
