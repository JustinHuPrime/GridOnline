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

//! Game state for Grid online server

use clap::{ArgAction, Args, ValueEnum};
use grid_common::{
    BOARD_SIZE, Board, Card, Deck, HAND_SIZE, Hand, PlayerMove, PlayerVisibleGameState, Suit, Value,
};
use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng,
};

#[derive(Clone, Args)]
pub struct GameOptions {
    #[clap(long, action = ArgAction::Set)]
    sequester_cards: bool,
    #[clap(long)]
    taking_variant: TakingVariant,
}
#[derive(Clone, Copy, ValueEnum)]
pub enum TakingVariant {
    SameNumber,
    SameNumberOrSuitRanked,
}

pub struct GameState {
    game_options: GameOptions,
    board: Board,
    players: Vec<(String, PlayerState)>,
    turn: usize,
}
pub struct PlayerState {
    hand: Hand,
    deck: Deck,
}

impl PlayerState {
    /// Check if the player has any cards (in hand or deck)
    pub fn has_cards(&self) -> bool {
        !self.hand.0.is_empty() || !self.deck.0.is_empty()
    }
}

impl GameState {
    pub fn new(player_names: Vec<String>, game_options: GameOptions) -> Self {
        let num_players = player_names.len();

        // Generate a full deck of 52 cards
        let mut deck = Vec::new();
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            for value in [
                Value::Ace,
                Value::Two,
                Value::Three,
                Value::Four,
                Value::Five,
                Value::Six,
                Value::Seven,
                Value::Eight,
                Value::Nine,
                Value::Ten,
                Value::Jack,
                Value::Queen,
                Value::King,
            ] {
                deck.push(Card(suit, value));
            }
        }

        // Shuffle the deck
        let mut rng = rand::thread_rng();
        deck.shuffle(&mut rng);

        let mut players = Vec::new();

        if game_options.sequester_cards {
            // Deal cards evenly to all players plus an extra "sequester" player
            let effective_players = num_players + 1;
            let cards_per_player = deck.len() / effective_players;

            // Deal to actual players
            for (i, player_name) in player_names.into_iter().enumerate() {
                let player_cards =
                    deck[(i * cards_per_player)..((i + 1) * cards_per_player)].to_vec();

                let hand = Hand(player_cards[0..HAND_SIZE.min(player_cards.len())].to_vec());
                let remaining_cards = player_cards[HAND_SIZE.min(player_cards.len())..].to_vec();

                players.push((
                    player_name.clone(),
                    PlayerState {
                        hand,
                        deck: Deck(remaining_cards),
                    },
                ));
            }
        } else {
            // Deal cards evenly to all players, distribute extra cards randomly
            let cards_per_player = deck.len() / num_players;
            let extra_cards = deck.len() % num_players;
            let gets_extra_cards = (0..num_players).choose_multiple(&mut rng, extra_cards);

            for (i, player_name) in player_names.into_iter().enumerate() {
                let extra_card: usize = gets_extra_cards.contains(&i).into();

                let player_cards = deck
                    [(i * cards_per_player)..((i + 1) * cards_per_player + extra_card)]
                    .to_vec();

                let hand = Hand(player_cards[0..HAND_SIZE.min(player_cards.len())].to_vec());
                let remaining_cards = player_cards[HAND_SIZE.min(player_cards.len())..].to_vec();

                players.push((
                    player_name.clone(),
                    PlayerState {
                        hand,
                        deck: Deck(remaining_cards),
                    },
                ));
            }
        }

        Self {
            game_options,
            board: Board([[None; BOARD_SIZE]; BOARD_SIZE]),
            players,
            turn: 0,
        }
    }

    pub fn state_for(&self, player_index: usize) -> PlayerVisibleGameState {
        if player_index >= self.players.len() {
            panic!(
                "Invalid player index: {} (only {} players exist)",
                player_index,
                self.players.len()
            );
        }

        let (player_name, player_state) = &self.players[player_index];

        // Create list of all players with their card counts (hand + deck)
        let players: Vec<(String, u32)> = self
            .players
            .iter()
            .map(|(name, state)| {
                let card_count = state.hand.0.len() + state.deck.0.len();
                (name.clone(), card_count as u32)
            })
            .collect();

        PlayerVisibleGameState {
            board: self.board.clone(),
            hand: player_state.hand.clone(),
            deck: player_state.deck.clone(),
            username: player_name.clone(),
            players,
            turn: self.turn,
        }
    }

    pub fn get_options(&self) -> &GameOptions {
        &self.game_options
    }

    pub fn get_player_names(&self) -> Vec<String> {
        self.players.iter().map(|(name, _)| name.clone()).collect()
    }

    pub fn current_player(&self) -> (&str, &PlayerState) {
        self.players
            .get(self.turn)
            .map(|(name, state)| (name.as_str(), state))
            .unwrap()
    }

    /// Check if any player has won (exactly one player has cards)
    pub fn someone_has_won(&self) -> bool {
        // note - zero should not be possible here, since one move ago exactly one player had a card
        self.players
            .iter()
            .filter(|(_, state)| state.has_cards())
            .count()
            <= 1
    }

    /// Make a move
    ///
    /// If move is invalid, return false
    pub fn apply_move(&mut self, player_move: PlayerMove) -> bool {
        let (_, current_player) = &mut self.players[self.turn];

        // Check - move must specify valid card within the current player's hand
        if player_move.card >= current_player.hand.0.len() {
            return false; // Card index out of bounds
        }

        // Check - validate move location according to game rules
        let (row, col) = player_move.location;
        if !self.board.can_play_at(row, col) {
            return false;
        }

        // Play the card
        let card = current_player.hand.0.remove(player_move.card);
        self.board.0[row][col] = Some(card);

        // Find cards to take before making any mutations
        let cards_to_take = match self.game_options.taking_variant {
            TakingVariant::SameNumber => {
                // Find furthest-away cards orthogonally and diagonally with the same value
                Self::find_taking_cards(&self.board, row, col, |target_card| {
                    target_card.1 == card.1
                })
            }
            TakingVariant::SameNumberOrSuitRanked => {
                // Find furthest-away cards orthogonally and diagonally with either the same value or the same suit and a lesser value
                Self::find_taking_cards(&self.board, row, col, |target_card| {
                    target_card.1 == card.1
                        || (target_card.0 == card.0 && (target_card.1 as u8) < (card.1 as u8))
                })
            }
        };

        // If any were found, remove those cards, all cards between them, and the just-played card
        let mut taken_cards = cards_to_take
            .into_iter()
            .filter_map(|(row, col)| self.board.0[row][col].take())
            .collect::<Vec<_>>();
        taken_cards.shuffle(&mut thread_rng());
        current_player.deck.0.extend(taken_cards);

        // Draw cards from deck to fill hand to HAND_SIZE
        while !current_player.deck.0.is_empty() && current_player.hand.0.len() < HAND_SIZE {
            current_player.hand.0.push(current_player.deck.0.remove(0));
        }

        // Move to next player's turn, skip players with no cards (must have at least one player with cards)
        self.turn = (self.turn + 1) % self.players.len();
        while !self.current_player().1.has_cards() {
            self.turn = (self.turn + 1) % self.players.len();
        }

        true
    }

    /// Find cards that can be taken based on the given predicate
    ///
    /// Returns positions of cards to be taken
    fn find_taking_cards(
        board: &Board,
        card_row: usize,
        card_col: usize,
        predicate: impl Fn(Card) -> bool,
    ) -> Vec<(usize, usize)> {
        let mut to_take = Vec::new();

        // Define the 8 directions: 4 orthogonal + 4 diagonal
        let directions = [
            // orthogonal
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            // diagonal
            (-1, -1),
            (-1, 1),
            (1, -1),
            (1, 1),
        ];

        for (dr, dc) in directions {
            // Search in this direction for the last matching card
            let mut row = card_row as i32 + dr;
            let mut col = card_col as i32 + dc;
            let mut found = None;
            while (0..BOARD_SIZE as i32).contains(&row) && (0..BOARD_SIZE as i32).contains(&col) {
                if let Some(board_card) = board.0[row as usize][col as usize]
                    && predicate(board_card)
                {
                    found = Some((row, col))
                }

                row += dr;
                col += dc;
            }

            if let Some((end_row, end_col)) = found {
                let mut row = card_row as i32;
                let mut col = card_col as i32;
                while row != end_row || col != end_col {
                    to_take.push((row as usize, col as usize));
                    row += dr;
                    col += dc;
                }
                // Also take the final matching card
                to_take.push((end_row as usize, end_col as usize));
            }
        }

        to_take
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_options(sequester: bool) -> GameOptions {
        GameOptions {
            sequester_cards: sequester,
            taking_variant: TakingVariant::SameNumber,
        }
    }

    #[test]
    fn test_game_state_creation_basic() {
        let player_names = vec!["Alice".to_string(), "Bob".to_string()];
        let options = create_test_options(false);

        let game_state = GameState::new(player_names.clone(), options);

        assert_eq!(game_state.players.len(), 2);
        assert_eq!(game_state.players[0].0, "Alice");
        assert_eq!(game_state.players[1].0, "Bob");
        assert_eq!(game_state.turn, 0);
    }

    #[test]
    fn test_game_state_creation_with_sequester() {
        let player_names = vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ];
        let options = create_test_options(true);

        let game_state = GameState::new(player_names.clone(), options);

        assert_eq!(game_state.players.len(), 3);

        // With sequester_cards=true, cards should be divided among 4 effective players (3 real + 1 sequester)
        // 52 cards / 4 = 13 cards per player
        for (_, player_state) in &game_state.players {
            let total_cards = player_state.hand.0.len() + player_state.deck.0.len();
            assert_eq!(
                total_cards, 13,
                "Each player should have 13 cards with sequester mode"
            );
        }
    }

    #[test]
    fn test_game_state_creation_without_sequester() {
        let player_names = vec!["Alice".to_string(), "Bob".to_string()];
        let options = create_test_options(false);

        let game_state = GameState::new(player_names.clone(), options);

        // With sequester_cards=false, cards should be divided among actual players
        // 52 cards / 2 = 26 cards per player
        for (_, player_state) in &game_state.players {
            let total_cards = player_state.hand.0.len() + player_state.deck.0.len();
            assert_eq!(
                total_cards, 26,
                "Each player should have 26 cards without sequester mode"
            );
        }
    }

    #[test]
    fn test_hand_size_limit() {
        let player_names = vec!["Alice".to_string()];
        let options = create_test_options(false);

        let game_state = GameState::new(player_names, options);

        // Hand should never exceed HAND_SIZE (5 cards)
        assert!(game_state.players[0].1.hand.0.len() <= HAND_SIZE);
        assert_eq!(game_state.players[0].1.hand.0.len(), HAND_SIZE.min(52)); // Should be 5
    }

    #[test]
    fn test_deck_contains_remaining_cards() {
        let player_names = vec!["Alice".to_string()];
        let options = create_test_options(false);

        let game_state = GameState::new(player_names, options);

        let player_state = &game_state.players[0].1;
        let total_cards = player_state.hand.0.len() + player_state.deck.0.len();

        assert_eq!(total_cards, 52); // All cards should be accounted for
        assert_eq!(player_state.hand.0.len(), 5); // Hand should have 5 cards
        assert_eq!(player_state.deck.0.len(), 47); // Deck should have remaining 47 cards
    }

    #[test]
    fn test_state_for_valid_player() {
        let player_names = vec!["Alice".to_string(), "Bob".to_string()];
        let options = create_test_options(false);

        let game_state = GameState::new(player_names, options);
        let alice_state = game_state.state_for(0);

        assert_eq!(alice_state.username, "Alice");
        assert_eq!(alice_state.players.len(), 2);
        assert_eq!(alice_state.players[0].0, "Alice");
        assert_eq!(alice_state.players[1].0, "Bob");
        assert_eq!(alice_state.turn, 0);

        // Alice should see her own cards but only card counts for others
        assert_eq!(alice_state.players[0].1, 26); // Alice's card count
        assert_eq!(alice_state.players[1].1, 26); // Bob's card count
    }

    #[test]
    fn test_state_for_different_players() {
        let player_names = vec!["Alice".to_string(), "Bob".to_string()];
        let options = create_test_options(false);

        let game_state = GameState::new(player_names, options);
        let alice_state = game_state.state_for(0);
        let bob_state = game_state.state_for(1);

        // Each player should see their own username
        assert_eq!(alice_state.username, "Alice");
        assert_eq!(bob_state.username, "Bob");

        // Each player should see the same board and turn
        assert_eq!(alice_state.board.0, bob_state.board.0);
        assert_eq!(alice_state.turn, bob_state.turn);

        // But different hands and decks
        assert_ne!(alice_state.hand.0, bob_state.hand.0);
        assert_ne!(alice_state.deck.0, bob_state.deck.0);
    }

    #[test]
    #[should_panic(expected = "Invalid player index: 2 (only 2 players exist)")]
    fn test_state_for_invalid_player_index() {
        let player_names = vec!["Alice".to_string(), "Bob".to_string()];
        let options = create_test_options(false);

        let game_state = GameState::new(player_names, options);
        let _ = game_state.state_for(2); // Should panic
    }

    #[test]
    fn test_board_initialization() {
        let player_names = vec!["Alice".to_string()];
        let options = create_test_options(false);

        let game_state = GameState::new(player_names, options);

        // Board should be initialized with all None values
        for row in &game_state.board.0 {
            for cell in row {
                assert_eq!(*cell, None);
            }
        }
    }

    #[test]
    fn test_card_distribution_fairness() {
        let player_names = vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ];
        let options = create_test_options(false);

        let game_state = GameState::new(player_names, options);

        // 52 cards / 3 players = 17 cards per player, with 1 extra card
        // So we should have 2 players with 17 cards and 1 player with 18 cards
        let card_counts: Vec<usize> = game_state
            .players
            .iter()
            .map(|(_, state)| state.hand.0.len() + state.deck.0.len())
            .collect();

        card_counts.iter().for_each(|&count| {
            assert!(
                count == 17 || count == 18,
                "Card count should be 17 or 18, got {}",
                count
            );
        });

        let total_cards: usize = card_counts.iter().sum();
        assert_eq!(total_cards, 52, "Total cards should be 52");
    }

    #[test]
    fn test_different_taking_variants() {
        let player_names = vec!["Alice".to_string(), "Bob".to_string()];

        let options1 = GameOptions {
            sequester_cards: false,
            taking_variant: TakingVariant::SameNumber,
        };

        let options2 = GameOptions {
            sequester_cards: false,
            taking_variant: TakingVariant::SameNumberOrSuitRanked,
        };

        let game_state1 = GameState::new(player_names.clone(), options1);
        let game_state2 = GameState::new(player_names, options2);

        // Both should create valid game states regardless of taking variant
        assert_eq!(game_state1.players.len(), 2);
        assert_eq!(game_state2.players.len(), 2);
    }

    #[test]
    fn test_first_move_must_be_center() {
        let player_names = vec!["Alice".to_string()];
        let options = create_test_options(false);
        let mut game_state = GameState::new(player_names, options);

        // First move must be in center (5, 5) on 11x11 board
        let move_corner = PlayerMove {
            card: 0,
            location: (0, 0),
        };
        assert!(!game_state.apply_move(move_corner));

        let move_center = PlayerMove {
            card: 0,
            location: (5, 5),
        };
        assert!(game_state.apply_move(move_center));
    }

    #[test]
    fn test_move_validation() {
        let player_names = vec!["Alice".to_string()];
        let options = create_test_options(false);
        let mut game_state = GameState::new(player_names, options);

        // Place first card in center
        let center_move = PlayerMove {
            card: 0,
            location: (5, 5),
        };
        assert!(game_state.apply_move(center_move));

        // Try to place card on occupied space
        let invalid_move = PlayerMove {
            card: 0,
            location: (5, 5),
        };
        assert!(!game_state.apply_move(invalid_move));

        // Try to place card out of bounds
        let out_of_bounds = PlayerMove {
            card: 0,
            location: (15, 15),
        };
        assert!(!game_state.apply_move(out_of_bounds));

        // Try to use invalid card index
        let invalid_card = PlayerMove {
            card: 10,
            location: (4, 4),
        };
        assert!(!game_state.apply_move(invalid_card));
    }

    #[test]
    fn test_same_number_taking_orthogonal() {
        let player_names = vec!["Alice".to_string()];
        let options = GameOptions {
            sequester_cards: false,
            taking_variant: TakingVariant::SameNumber,
        };
        let mut game_state = GameState::new(player_names, options);

        // Manually set up board for testing
        let test_card_ace_clubs = Card(Suit::Clubs, Value::Ace);
        let test_card_ace_hearts = Card(Suit::Hearts, Value::Ace);

        // Place cards manually on board
        game_state.board.0[5][5] = Some(test_card_ace_clubs); // Center
        game_state.board.0[5][7] = Some(test_card_ace_hearts); // Two spaces right

        // Set up player's hand with an Ace
        game_state.players[0].1.hand.0[0] = test_card_ace_clubs;

        // Place Ace at (5, 6) - between center and (5, 7), should take both
        let move_between = PlayerMove {
            card: 0,
            location: (5, 6),
        };

        let initial_deck_size = game_state.players[0].1.deck.0.len();
        assert!(game_state.apply_move(move_between));

        // Check that the move took cards (board should be empty, cards in deck)
        assert!(game_state.board.0[5][5].is_none());
        assert!(game_state.board.0[5][6].is_none()); // Played card also taken
        assert!(game_state.board.0[5][7].is_none());

        // Check that cards were added to deck
        assert!(game_state.players[0].1.deck.0.len() > initial_deck_size);
    }

    #[test]
    fn test_same_number_taking_diagonal() {
        let player_names = vec!["Alice".to_string()];
        let options = GameOptions {
            sequester_cards: false,
            taking_variant: TakingVariant::SameNumber,
        };
        let mut game_state = GameState::new(player_names, options);

        let test_card_king = Card(Suit::Clubs, Value::King);

        // Place cards diagonally
        game_state.board.0[4][4] = Some(test_card_king);
        game_state.board.0[7][7] = Some(test_card_king);

        // Set up player's hand
        game_state.players[0].1.hand.0[0] = test_card_king;

        // Place King at (5, 5) - on diagonal between the two existing Kings
        let diagonal_move = PlayerMove {
            card: 0,
            location: (5, 5),
        };

        let initial_deck_size = game_state.players[0].1.deck.0.len();
        assert!(game_state.apply_move(diagonal_move));

        // Check that diagonal taking worked
        assert!(game_state.board.0[3][3].is_none());
        assert!(game_state.board.0[5][5].is_none());
        assert!(game_state.board.0[7][7].is_none());
        assert!(game_state.players[0].1.deck.0.len() > initial_deck_size);
    }

    #[test]
    fn test_same_number_or_suit_ranked_taking() {
        let player_names = vec!["Alice".to_string()];
        let options = GameOptions {
            sequester_cards: false,
            taking_variant: TakingVariant::SameNumberOrSuitRanked,
        };
        let mut game_state = GameState::new(player_names, options);

        let card_five_hearts = Card(Suit::Hearts, Value::Five);
        let card_three_hearts = Card(Suit::Hearts, Value::Three); // Same suit, lower value
        let card_five_clubs = Card(Suit::Clubs, Value::Five); // Same value, different suit

        // Place cards on board
        game_state.board.0[5][4] = Some(card_three_hearts); // Should be taken (same suit, lower)
        game_state.board.0[5][7] = Some(card_five_clubs); // Should be taken (same value)

        // Set up player's hand
        game_state.players[0].1.hand.0[0] = card_five_hearts;

        // Place Five of Hearts at center
        let center_move = PlayerMove {
            card: 0,
            location: (5, 5),
        };

        let initial_deck_size = game_state.players[0].1.deck.0.len();
        assert!(game_state.apply_move(center_move));

        // Both cards should be taken
        assert!(game_state.board.0[5][4].is_none()); // Three of Hearts taken
        assert!(game_state.board.0[5][5].is_none()); // Played card taken
        assert!(game_state.board.0[5][7].is_none()); // Five of Clubs taken
        assert!(game_state.players[0].1.deck.0.len() > initial_deck_size);
    }

    #[test]
    fn test_no_taking_when_no_matches() {
        let player_names = vec!["Alice".to_string()];
        let options = GameOptions {
            sequester_cards: false,
            taking_variant: TakingVariant::SameNumber,
        };
        let mut game_state = GameState::new(player_names, options);

        let card_ace = Card(Suit::Clubs, Value::Ace);
        let card_king = Card(Suit::Hearts, Value::King);

        // Place different card on board
        game_state.board.0[5][6] = Some(card_king);

        // Set up player's hand
        game_state.players[0].1.hand.0[0] = card_ace;

        // Place Ace at center - no taking should occur
        let center_move = PlayerMove {
            card: 0,
            location: (5, 5),
        };

        let initial_deck_size = game_state.players[0].1.deck.0.len();
        let initial_hand_size = game_state.players[0].1.hand.0.len();
        assert!(game_state.apply_move(center_move));

        // Card should remain on board, no taking
        assert!(game_state.board.0[5][5].is_some()); // Played card stays
        assert!(game_state.board.0[5][6].is_some()); // King stays

        // Deck size should decrease by 1 (drew 1 card to refill hand after playing 1)
        assert_eq!(game_state.players[0].1.deck.0.len(), initial_deck_size - 1);
        // Hand size should remain the same (played 1, drew 1)
        assert_eq!(game_state.players[0].1.hand.0.len(), initial_hand_size);
    }

    #[test]
    fn test_intervening_cards_taken() {
        let player_names = vec!["Alice".to_string()];
        let options = GameOptions {
            sequester_cards: false,
            taking_variant: TakingVariant::SameNumber,
        };
        let mut game_state = GameState::new(player_names, options);

        let card_ace = Card(Suit::Clubs, Value::Ace);
        let card_two = Card(Suit::Hearts, Value::Two);

        // Place cards with intervening card
        game_state.board.0[5][3] = Some(card_ace);
        game_state.board.0[5][5] = Some(card_two); // Intervening card (different value)
        game_state.board.0[5][7] = Some(card_ace);

        // Set up player's hand
        game_state.players[0].1.hand.0[0] = card_ace;

        // Place Ace at (5, 4) - should take all cards in the line including intervening
        let move_with_intervening = PlayerMove {
            card: 0,
            location: (5, 4),
        };

        let initial_deck_size = game_state.players[0].1.deck.0.len();
        let initial_hand_size = game_state.players[0].1.hand.0.len();
        assert!(game_state.apply_move(move_with_intervening));

        // All cards should be taken, including the intervening non-matching card
        assert!(game_state.board.0[5][3].is_none()); // Matching card taken
        assert!(game_state.board.0[5][4].is_none()); // Played card taken
        assert!(game_state.board.0[5][5].is_none()); // Intervening card taken
        assert!(game_state.board.0[5][7].is_none()); // Matching card taken

        // 4 cards added to deck (3 taken + 1 played), then 1 card drawn to refill hand
        // Net change: +3 cards to deck
        assert_eq!(game_state.players[0].1.deck.0.len(), initial_deck_size + 3);
        assert_eq!(game_state.players[0].1.hand.0.len(), initial_hand_size);
    }

    #[test]
    fn test_multiple_direction_taking() {
        let player_names = vec!["Alice".to_string()];
        let options = GameOptions {
            sequester_cards: false,
            taking_variant: TakingVariant::SameNumber,
        };
        let mut game_state = GameState::new(player_names, options);

        let card_queen = Card(Suit::Clubs, Value::Queen);

        // Place Queens in multiple directions from center
        game_state.board.0[5][4] = Some(card_queen); // West
        game_state.board.0[5][7] = Some(card_queen); // East  
        game_state.board.0[3][5] = Some(card_queen); // North
        game_state.board.0[7][5] = Some(card_queen); // South

        // Set up player's hand
        game_state.players[0].1.hand.0[0] = card_queen;

        // Place Queen at center - should take all 4 directions
        let center_move = PlayerMove {
            card: 0,
            location: (5, 5),
        };

        let initial_deck_size = game_state.players[0].1.deck.0.len();
        let initial_hand_size = game_state.players[0].1.hand.0.len();
        assert!(game_state.apply_move(center_move));

        // All Queens should be taken
        assert!(game_state.board.0[5][3].is_none()); // West taken
        assert!(game_state.board.0[5][7].is_none()); // East taken
        assert!(game_state.board.0[3][5].is_none()); // North taken
        assert!(game_state.board.0[7][5].is_none()); // South taken
        assert!(game_state.board.0[5][5].is_none()); // Center (played) taken

        // 5 cards added to deck (4 taken + 1 played), then 1 card drawn to refill hand
        // Net change: +4 cards to deck
        assert_eq!(game_state.players[0].1.deck.0.len(), initial_deck_size + 4);
        // Hand size should remain the same (played 1, drew 1)
        assert_eq!(game_state.players[0].1.hand.0.len(), initial_hand_size);
    }
}
