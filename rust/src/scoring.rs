// THESE CONSTANTS SHOULD BE CREATED BY A MACRO ONCE I
// WORK OUT HOW TO DO THAT
use super::HEUR_SCORE_TABLE;
use super::SCORE_TABLE;
use super::ROW_MASK;
use super::board::transpose;

// Returns the actual score of the board.
pub fn score_board(board: u64)  -> f32 {
    unsafe{
        score_helper(board, &SCORE_TABLE)
    }
}

// Returns the heuristic score of the board.
pub fn score_heur_board(board: u64) -> f32 {
    // Consider the board and the transpose because things like monotonicity matter in the x and y directions
    unsafe{
        score_helper(          board , &HEUR_SCORE_TABLE) +
        score_helper(transpose(board), &HEUR_SCORE_TABLE)
    }
}

// Sums the scores held in the given table for each row in the given board.
fn score_helper(board: u64, table: &[f32]) -> f32{
    table[((board >>  0) & ROW_MASK) as usize] +
    table[((board >> 16) & ROW_MASK) as usize] +
    table[((board >> 32) & ROW_MASK) as usize] +
    table[((board >> 48) & ROW_MASK) as usize] 
}
