use std::{
    fmt,
    error,
    collections::HashSet
};
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub board_size: u32,
    pub komi: u32,
    pub handicap: u32,
    pub fixed_time: u32,
    pub added_time: u32
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Stone {
    Black,
    White,
    Empty
}

impl From<Stone> for &str {
    fn from(value: Stone) -> Self {
        match value {
            Stone::Black => "black",
            Stone::White => "white",
            Stone::Empty => "empty"
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Turn {
    Handicap,
    Black,
    White,
    End
}

impl From<Turn> for &str {
    fn from(value: Turn) -> Self {
        match value {
            Turn::Handicap => "handicap",
            Turn::Black => "black",
            Turn::White => "white",
            Turn::End => "end"
        }
    }
}

fn neighbors(n: usize, board: &[Stone], pos: usize) -> Vec<usize> {
    let mut result = Vec::<usize>::new();
    if pos >= 1 && pos % n > 0 {
        result.push(pos - 1);
    }
    if pos + 1 < board.len() && pos % n < n - 1 {
        result.push(pos + 1);
    }
    if pos >= n {
        result.push(pos - n);
    }
    if pos + n < board.len() {
        result.push(pos + n);
    }
    result
}

fn connected_group(n: usize, board: &[Stone], pos: usize) -> Vec<usize> {
    debug_assert!(pos < board.len());
    let mut result = Vec::<usize>::new();
    let color = board[pos];
    let mut seen = vec![false; board.len()];
    seen[pos] = true;
    let mut stack = vec![pos];
    while let Some(pos) = stack.pop() {
        result.push(pos);
        for neighbor in neighbors(n, board, pos) {
            if board[neighbor] == color && !seen[neighbor] {
                seen[neighbor] = true;
                stack.push(neighbor);
            }
        }
    }
    result
}

fn liberty(n: usize, board: &[Stone], group: &[usize]) -> bool {
    for &pos in group {
        if board[pos] == Stone::Empty {
            return true;
        }
        for neighbor in neighbors(n, board, pos) {
            if board[neighbor] == Stone::Empty {
                return true;
            }
        }
    }
    false
}

fn place_stone(n: usize, board: &mut [Stone], stone: Stone, pos: usize) -> u32 {
    board[pos] = stone;
    let mut captures = 0;
    //Capture
    for neighbor in neighbors(n, &board, pos) {
        if board[neighbor] != stone && board[neighbor] != Stone::Empty {
            let group = connected_group(n, &board, neighbor);
            if !liberty(n, &board, &group) {
                captures += group.len() as u32;
                for pos in group {
                    board[pos] = Stone::Empty;
                }
            }
        }
    }
    //Self-capture
    let group = connected_group(n, &board, pos);
    if !liberty(n, &board, &group) {
        captures += group.len() as u32;
        for pos in group {
            board[pos] = Stone::Empty;
        }
    }
    captures
}

pub enum GameError {
    Creation,
    Handicap,
    Play
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Creation => writeln!(f, "Creation error"),
            Self::Handicap => writeln!(f, "Handicap error"),
            Self::Play => writeln!(f, "Play error"),
        }
    }
}

impl fmt::Debug for GameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Creation => writeln!(f, "Creation error"),
            Self::Handicap => writeln!(f, "Handicap error"),
            Self::Play => writeln!(f, "Play error"),
        }
    }
}

impl error::Error for GameError {}

#[derive(Clone)]
pub struct Game {
    //Settings
    pub board_size: usize,
    pub komi: u32,
    pub handicap: u32,
    //Game state
    pub board: Vec<Stone>,
    pub history: Vec<Vec<Stone>>,
    pub valid_moves: Vec<bool>,
    pub turn: Turn,
    pub passes: u32,
    //Score
    pub black_score: u32,
    pub white_score: u32
}

//TODO: Fixed handicap placement
impl Game {
    pub fn new(board_size: usize, komi: u32, handicap: u32) -> Result<Game, GameError> {
        if board_size >= 5 && board_size <= 19 && handicap > 0 && handicap <= 9 {
            let tile_count = board_size * board_size;
            let board = vec![Stone::Empty; tile_count];
            Ok(Game {
                board_size,
                komi,
                handicap,
                board: board.clone(),
                history: vec![board.clone()],
                valid_moves: vec![true; tile_count],
                turn: if handicap == 1 {
                    Turn::Black
                } else {
                    Turn::Handicap
                },
                passes: 0,
                black_score: 0,
                white_score: 0
            })
        } else {
            Err(GameError::Handicap)
        }
    }
    pub fn play_handicap(&mut self, stone: Stone, positions: &[usize]) -> Result<(), GameError> {
        if stone == Stone::Black && self.turn == Turn::Handicap {
            let set: HashSet<usize> = positions.iter().copied()
                .filter(|&x| x < self.board.len())
                .collect();
            if set.len() == positions.len() && positions.len() <= self.handicap as usize {
                for &i in positions {
                    self.board[i] = stone;
                }
                self.turn = Turn::White;
                Ok(())
            } else {
                Err(GameError::Handicap)
            }
        } else {
            Err(GameError::Handicap)
        }
    }
    pub fn play(&mut self, stone: Stone, pos: usize) -> Result<(), GameError> {
        //Conditions
        if pos < self.board.len()
            && (
                (stone == Stone::Black && self.turn == Turn::Black)
                || (stone == Stone::White && self.turn == Turn::White)
            ) && self.valid_moves[pos] {
            //Place stone
            place_stone(self.board_size, &mut self.board, stone, pos);
            self.history.push(self.board.clone());
            //Advance turn
            self.turn = match self.turn {
                Turn::Black => Turn::White,
                Turn::White => Turn::Black,
                _ => return Err(GameError::Play)
            };
            self.passes = 0;
            //Generate next valid moves
            let next_stone = match stone {
                Stone::Black => Stone::White,
                Stone::White => Stone::Black,
                _ => return Err(GameError::Play)
            };
            for pos in 0..self.board.len() {
                if self.board[pos] == Stone::Empty {
                    let mut board = self.board.clone();
                    let captures = place_stone(self.board_size, &mut board, next_stone, pos);
                    self.valid_moves[pos] = true;
                    if captures > 0 {
                        for entry in &self.history {
                            if board == *entry {
                                self.valid_moves[pos] = false;
                            }
                        }
                    }
                } else {
                    self.valid_moves[pos] = false;
                }
            }
            //Scoring
            [self.black_score, self.white_score] = self.score();
            Ok(())
        } else {
            Err(GameError::Play)
        }
    }
    pub fn pass(&mut self, side: Stone) -> Result<(), GameError> {
        if (side == Stone::Black && self.turn == Turn::Black)
            || (side == Stone::White && self.turn == Turn::White) {
            self.passes += 1;
            if self.passes == 2 {
                self.turn = Turn::End;
            } else {
                match side {
                    Stone::Black => self.turn = Turn::White,
                    Stone::White => self.turn = Turn::Black,
                    _ => ()
                }
            }
            Ok(())
        } else {
            Err(GameError::Play)
        }
    }
    pub fn resign(&mut self, stone: Stone) -> Result<(), GameError> {
        if stone == Stone::Black && self.turn == Turn::Black {
            self.black_score = 0;
            self.white_score = (self.board_size * self.board_size) as u32;
            self.turn = Turn::End;
            Ok(())
        } else if stone == Stone::White && self.turn == Turn::White {
            self.white_score = 0;
            self.black_score = (self.board_size * self.board_size) as u32;
            self.turn = Turn::End;
            Ok(())
        } else {
            Err(GameError::Play)
        }
    }
    pub fn score(&self) -> [u32; 2] {
        //Determine territory
        let mut territory = [0, 0];
        let mut seen = vec![false; self.board.len()];
        for pos in 0..self.board.len() {
            if self.board[pos] == Stone::Empty && !seen[pos] {
                let group = connected_group(self.board_size, &self.board, pos);
                let mut bordered = [false, false];
                for &pos in &group {
                    seen[pos] = true;
                    for neighbor in neighbors(self.board_size, &self.board, pos) {
                        match self.board[neighbor] {
                            Stone::Black => bordered[0] = true,
                            Stone::White => bordered[1] = true,
                            _ => ()
                        }
                    }
                }
                if bordered[0] && !bordered[1] {
                    territory[0] += group.len() as u32;
                } else if !bordered[0] && bordered[1] {
                    territory[1] += group.len() as u32;
                }
            }
        }
        //Count stones
        let mut counts = [0, 0];
        for stone in &self.board {
            match stone {
                Stone::Black => counts[0] += 1,
                Stone::White => counts[1] += 1,
                _ => ()
            }
        }
        [territory[0] + counts[0], territory[1] + counts[1]]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_neighbors() {
        let board = [
            Stone::Black, Stone::Black, Stone::White,
            Stone::Black, Stone::Black, Stone::Empty,
            Stone::Empty, Stone::White, Stone::Black
        ];
        let expected = [1, 3, 5, 7];
        let mut neighbors = neighbors(3, &board, 4);
        neighbors.sort();
        assert!(neighbors == expected);
    }
    #[test]
    fn test_connected_group() {
        let board = [
            Stone::Black, Stone::Black, Stone::White,
            Stone::Black, Stone::Black, Stone::Empty,
            Stone::Empty, Stone::White, Stone::Black
        ];
        let expected: [usize; 4] = [0, 1, 3, 4];
        let mut group = connected_group(3, &board, 0);
        group.sort();
        for item in &group {
            println!("GROUP {}", item);
        }
        assert!(group == expected);
    }
    #[test]
    fn test_scoring() {
        let mut game = Game::new(5, 0, 1).unwrap();
        game.board_size = 3;
        game.board = vec![
            Stone::Empty, Stone::Empty, Stone::Black,
            Stone::Black, Stone::Black, Stone::Empty,
            Stone::Black, Stone::White, Stone::Empty
        ];
        let expected = [6, 1];
        assert_eq!(game.score(), expected);
    }
}
