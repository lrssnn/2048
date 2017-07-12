use std::cmp::max;
use std::collections::HashMap;
use std::thread;

use super::board::{execute_move};
use super::board::{count_empty, count_distinct_tiles};
use super::scoring::{score_heur_board};

use super::CPROB_THRESH_BASE; // Will not evaluate nodes less likely than this
const CACHE_DEPTH_LIMIT: u32 = 15;     // Will not cache nodes deeper than this

type TransTable = HashMap<u64, TransTableEntry>; // Typedef to remove generics from the main code

// An entry in the game cache. Stores a heuristic value and the depth at which it was computed
struct TransTableEntry {
    depth: u8,
    heuristic: f32
}

// The state of the current evaluation
struct EvalState {
    trans_table: TransTable, // The cache for this evaluation
    maxdepth: u32,           // The maximum depth seen in this evaluation
    curdepth: u32,           // The current depth of evaluation
    cachehits: u32,          // Number of times a cached result has been reused
    moves_evaled: u64,       // Number of game states evaluated in this evaluation
    depth_limit: u32,        // The maximum depth to look in this evaluation
}

// Takes a board and returns the most effective move to make on it
pub fn find_best_move(board: u64) -> u8 {
    let mut best: f32 = 0.0;
    let mut bestmove: u8 = 0;

    // For each possible move, evaluate the move with expectimax search and track the best result.
    // Concurrent
    let mut threads = vec!();
    for mv in 0..4 {
        let handle = thread::spawn(move || {
            (score_toplevel_move(board, mv), mv)
        });
        
        threads.push(handle);
    }

    for thread in threads {

        let (res, mv) = thread.join().unwrap();

        if res > best {
            best = res;
            bestmove = mv;
        }
    }
    bestmove
}

// Returns the value of a player node in the game tree.
// Plays the part of the Maximiser node in the Expectimax search.
fn score_move_node(mut state: &mut EvalState, board: u64, cprob: f32) -> f32 {
    let mut best: f32 = 0.0;
    state.curdepth+= 1;
    // Look at each possible move and track the highest value
    for mv in 0..4 {
        let newboard: u64 = execute_move(mv, board);
        state.moves_evaled+= 1;

        if board != newboard {
            unsafe {
                best = best.max(score_tilechoose_node(&mut state, newboard, cprob));
            }
        }
    }
    state.curdepth -= 1;

    best
}

// Returns the value of a computer node in the game tree.
// Plays the part of the Expected Value node in the Expectimax search.
unsafe fn score_tilechoose_node(mut state: &mut EvalState, board:u64, mut cprob:f32) -> f32 {
    // Base case: simply return the heuristic if the current state is less likely than the threshold
    // or deeper than the depth limit
    if cprob < CPROB_THRESH_BASE || state.curdepth >= state.depth_limit {
        state.maxdepth = max(state.curdepth, state.maxdepth);
        return score_heur_board(board);
    }
    // If the current depth is less than the cache depth limit, look in the cache in case we already know 
    // the value of this board.
    if state.curdepth < CACHE_DEPTH_LIMIT {
        
        let entry = state.trans_table.get(&board);  
        // If we have cached this entry, return the cached value
        if let Some(entry) = entry {
            if entry.depth <= state.curdepth as u8 {
                state.cachehits+= 1;
                return entry.heuristic;
            }
        }
    }

    // We have not cached this board, calculate the value
    // Scale the probability of the children of this node by the number of possible choices.
    let num_open = count_empty(board);
    cprob /= num_open as f32;

    let mut res: f32 = 0.0;
    let mut tmp = board;
    let mut tile_2: u64 = 1;
    
    // For each empty tile on the board, add a two and a four to it and calculate the value of it by 
    // simulating another human (move_node) move. 
    while tile_2 != 0 {
        if (tmp & 0xF) == 0 {
            res += score_move_node(&mut state, board |  tile_2      , cprob * 0.9) * 0.9;
            res += score_move_node(&mut state, board | (tile_2 << 1), cprob * 0.1) * 0.1;
        }
        tmp >>= 4;
        tile_2 <<= 4;
    }

    res /= num_open as f32;

    // If we aren't too deep, cache this result for next time.
    if state.curdepth < CACHE_DEPTH_LIMIT {
        let entry = TransTableEntry {depth: state.curdepth as u8, heuristic: res};
        state.trans_table.insert(board, entry);
    }

    res
}

// Takes a move and a board and evaluates the value of that move. Begins the expectimax search on this state
fn _score_toplevel_move(mut state: &mut EvalState, board: u64, mv: u8) -> f32 {
    let newboard = execute_move(mv, board);

    if board == newboard {
        return 0.0;
    }

    unsafe {
        score_tilechoose_node(&mut state, newboard, 1.0) + 0.000001
    }
}

// Takes a board and a move and sets up the infrastructure to perform the expectimax search on it.
fn score_toplevel_move(board: u64, mv: u8) -> f32 {
    let mut state = EvalState{maxdepth: 0, curdepth: 0, moves_evaled: 0, cachehits:0, depth_limit:0, trans_table: TransTable::new()};
    state.depth_limit = max(3, (count_distinct_tiles(board) - 2));

    _score_toplevel_move(&mut state, board, mv)
}

