// An implementation of a 4x4 2048 board.
// Heavily inspired by the cpp implementation on github by user 'nneonneo'
extern crate rand;

use rand::Rng;
use std::time::SystemTime;
use std::cmp::max;
use std::collections::HashMap;
use std::thread;

type TransTable = HashMap<u64, TransTableEntry>; // Typedef to remove generics from the main code

// Tables which are filled with precomputed moves. Any row XORed with row_left_table[row] will be the result 
// of swiping that row left, and so on with the other directions.
static mut row_left_table:   [u16; 65536] = [0; 65536];
static mut row_right_table:  [u16; 65536] = [0; 65536];
static mut col_up_table:     [u64; 65536] = [0; 65536];
static mut col_down_table:   [u64; 65536] = [0; 65536];
// Precomputed heuristics and scores for single rows also
static mut heur_score_table: [f32; 65536] = [0.0; 65536];
static mut score_table:      [f32; 65536] = [0.0; 65536];

// Constants to tune game behaviour
const SCORE_LOST_PENALTY:       f32 = 200000.0;
const SCORE_MONOTONICITY_POWER: f32 = 4.0;
const SCORE_MONOTONICITY_WEIGHT:f32 = 47.0;
const SCORE_SUM_POWER:          f32 = 3.5;
const SCORE_SUM_WEIGHT:         f32 = 11.0;
const SCORE_MERGES_WEIGHT:      f32 = 700.0;
const SCORE_EMPTY_WEIGHT:       f32 = 270.0;

const CPROB_THRESH_BASE: f32 = 0.0002; // Will not evaluate nodes less likely than this
const CACHE_DEPTH_LIMIT: u32 = 15;     // Will not cache nodes deeper than this

// Masks to extract certain information from a u64 number
const ROW_MASK: u64 = 0xFFFF; 
const COL_MASK: u64 = 0x000F000F000F000F;

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

// Prints the bitboard in a human readable format
fn print_board(mut board: u64) {
    for _i in 0..4 {
        for _j in 0..4 {
            let power = board & 0xf; //Take the last byte in the number
            print!("{:5},", if power == 0 {0} else {2 << power-1}); //2<<power = 2^power
            board >>= 4; //Next byte
        }
        print!("\n");
    }
    print!("\n");
}

// Takes a column as a 16 bit number, and returns an empty bitboard with that column as the first column
fn unpack_col(row: u16) -> u64 {
    let tmp: u64 = row as u64;
    (tmp | (tmp << 12) | (tmp << 24) | (tmp << 36)) & COL_MASK
}

// Takes a row as a 16 bit number and returns the reverse of it
fn reverse_row(row: u16) -> u16 {
    (row >> 12) | ((row >> 4) & 0x00F0) | ((row << 4) & 0x0F00) | (row << 12)
}

// Takes a bitboard and returns the transposition of that board
// a b c d     a e i m
// e f g h  => b f j n
// i j k l     c g k o
// m n o p     d h l p
fn transpose(board: u64) -> u64 {
    let a1: u64 = board & 0xF0F00F0FF0F00F0F;
    let a2: u64 = board & 0x0000F0F00000F0F0;
    let a3: u64 = board & 0x0F0F00000F0F0000;
    let a : u64 = a1 | (a2 << 12) | (a3 >> 12);
    let b1: u64 = a & 0xFF00FF0000FF00FF;
    let b2: u64 = a & 0x00FF00FF00000000;
    let b3: u64 = a & 0x00000000FF00FF00;
    b1 | (b2 >> 24) | (b3 << 24)
}

// Returns the number of open spaces in the given bitboard
fn count_empty(mut board: u64) -> u64 {
    board |= (board >> 2) & 0x3333333333333333;
    board |=  board >> 1;
    board  = !board & 0x1111111111111111;

    board += board >> 32;
    board += board >> 16;
    board += board >>  8;
    board += board >>  4;
    
    board & 0xF as u64
}

// Initialises the precomputed tables used to execute moves and score states
unsafe fn init_tables() {
    // Each possible row (16 bit number) has its results precomputed
    for row in 0..65536usize {
        // Convert the 16 bit number into an array of 4 parts (effectively 4 bit numbers)
        let mut line = [
            (row >> 0) & 0xF,
            (row >> 4) & 0xF,
            (row >> 8) & 0xF,
            (row >> 12) & 0xF
        ];
        
        // Calculate the score
        let mut score: f32 = 0.0;
        for i in 0..4 {
            let rank = line[i];
            if rank >= 2 {
                score += (rank as f32 - 1.0) * (1 << rank) as f32;
            }
        }
        score_table[row as usize] = score;

        // Calculate merges
        let mut sum: f32 = 0.0;
        let mut empty = 0;
        let mut merges = 0;

        let mut prev = 0;
        let mut counter = 0;
        for i in 0..4 {
            let rank = line[i];
            sum += (rank as f32).powf(SCORE_SUM_POWER);
            if rank == 0 {
                empty += 1;
            } else {
                if prev == rank {
                    counter += 1;
                } else if counter > 0 {
                    merges += 1 + counter;
                    counter = 0;
                }
                prev = rank;
            }
        }
        if counter > 0 {
            merges += 1 + counter;
        }

        // Calculate monotonicity
        let mut monotonicity_left : f32 = 0.0;
        let mut monotonicity_right: f32 = 0.0;
        for i in 1..4 {
            if line[i-1] > line[i] {
                monotonicity_left += (line[i-1] as f32).powf(SCORE_MONOTONICITY_POWER) - (line[i] as f32).powf(SCORE_MONOTONICITY_POWER);
            } else {
                monotonicity_right += (line[i] as f32).powf(SCORE_MONOTONICITY_POWER) - (line[i-1] as f32).powf(SCORE_MONOTONICITY_POWER);
            }
        }
        
        // Combine the components of the heuristic into one value
        heur_score_table[row as usize] = SCORE_LOST_PENALTY + 
            SCORE_EMPTY_WEIGHT * empty as f32 +
            SCORE_MERGES_WEIGHT * merges as f32 -
            SCORE_MONOTONICITY_WEIGHT * monotonicity_left.min(monotonicity_right)-
            SCORE_SUM_WEIGHT * sum as f32;

        //Exectute a move to the left
        let mut i = 0;
        while i < 3 {
            let mut j = i + 1;
            while j < 4 {
                if line[j] != 0 {break};
                j += 1;
            }
            if j == 4 {break};

            if line[i] == 0 {
                line[i] = line[j];
                line[j] = 0;
                // Retry
                if i > 0{
                    i -= 1;
                } else {
                    continue;
                }
            } else if line[i] == line[j] {
                if line[i] != 0xF {
                    line[i] += 1;
                }
                line[j] = 0;
            }
            i+= 1;
        }

        // Convert the result of the merge back into a 16 bit number
        let result: u16 = ((line[0] as u16) << 0) |
                          ((line[1] as u16)<< 4) |
                          ((line[2] as u16) << 8) |
                          ((line[3] as u16) << 12);
        let rev_result: u16 = reverse_row(result);
        let rev_row   : u16 = reverse_row(row as u16);
        
        // The result of each move is simply some modification of the left move, so we can store them all
        row_left_table  [    row]          =                row as u16  ^                result;
        row_right_table [rev_row as usize] =            rev_row         ^            rev_result;
        col_up_table    [    row]          = unpack_col(    row as u16) ^ unpack_col(    result);
        col_down_table  [rev_row as usize] = unpack_col(rev_row)        ^ unpack_col(rev_result);
    }            
}

// Swipe the given board up
unsafe fn execute_move_0(board: u64) -> u64 {
    // Every row has a precomputed result, so we simply transpose to convert columns to rows, and combine the
    // results of each row in turn.
    let mut ret = board;
    let t   = transpose(board);
    ret ^= col_up_table[((t >>  0) & ROW_MASK) as usize] << 0;
    ret ^= col_up_table[((t >> 16) & ROW_MASK) as usize] << 4;
    ret ^= col_up_table[((t >> 32) & ROW_MASK) as usize] << 8;
    ret ^= col_up_table[((t >> 48) & ROW_MASK) as usize] << 12;
    ret
}

// Swipe the given board down
unsafe fn execute_move_1(board: u64) -> u64 {
    let mut ret = board;
    let t   = transpose(board);
    ret ^= col_down_table[((t >>  0) & ROW_MASK) as usize] << 0;
    ret ^= col_down_table[((t >> 16) & ROW_MASK) as usize] << 4;
    ret ^= col_down_table[((t >> 32) & ROW_MASK) as usize] << 8;
    ret ^= col_down_table[((t >> 48) & ROW_MASK) as usize] << 12;
    ret
}

// Swipe the given board left
unsafe fn execute_move_2(board: u64) -> u64 {
    let mut ret = board;
    ret ^= (row_left_table[((board >>  0) & ROW_MASK) as usize] as u64) <<  0;
    ret ^= (row_left_table[((board >> 16) & ROW_MASK) as usize] as u64) << 16;
    ret ^= (row_left_table[((board >> 32) & ROW_MASK) as usize] as u64) << 32;
    ret ^= (row_left_table[((board >> 48) & ROW_MASK) as usize] as u64) << 48;
    ret
}

// Swipe the given board right
unsafe fn execute_move_3(board: u64) -> u64 {
    let mut ret = board;
    ret ^= (row_right_table[((board >>  0) & ROW_MASK) as usize] as u64) <<  0;
    ret ^= (row_right_table[((board >> 16) & ROW_MASK) as usize] as u64) << 16;
    ret ^= (row_right_table[((board >> 32) & ROW_MASK) as usize] as u64) << 32;
    ret ^= (row_right_table[((board >> 48) & ROW_MASK) as usize] as u64) << 48;
    ret
}

// Return the result of the specified move on the given board.
// mv: 0 -> up
//     1 -> down
//     2 -> right
//     3 -> left
// Any other value of mv will return a 0 board.
fn execute_move(mv: u8, board: u64) -> u64 {
    unsafe{
        match mv {
            0 => execute_move_0(board),
            1 => execute_move_1(board),
            2 => execute_move_2(board),
            3 => execute_move_3(board),
            _ => {println!("INVALID_MOVE"); 0}
        }
    }
}

// Returns the maximum tile rank (power of 2) present on the given bitboard
fn get_max_rank(mut board: u64) -> u16 {
    let mut maxrank: u16 = 0;
    // Simply consume the board nibble by nibble and track the highest tile seen.
    while board != 0 {
        maxrank = max(maxrank, (board & 0xF) as u16);
        board >>= 4;
    }
    maxrank
}

// Returns the number of unique tiles on the board
fn count_distinct_tiles(mut board: u64) -> u32 {
    let mut bitset: u16 = 0;
    while board != 0 {
        bitset |= 1 << (board & 0xF);
        board >>= 4;
    }

    bitset >>= 1;

    let mut count = 0;
    while bitset != 0 {
        bitset &= bitset - 1;
        count+= 1;
    }
    max(2, count)
}

// Returns the heuristic score of the board.
fn score_heur_board(board: u64) -> f32 {
    // Consider the board and the transpose because things like monotonicity matter in the x and y directions
    unsafe{
        score_helper(          board , &heur_score_table) +
        score_helper(transpose(board), &heur_score_table)
    }
}

// Returns the actual score of the board.
fn score_board(board: u64)  -> f32 {
    unsafe{
        score_helper(board, &score_table)
    }
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
            best = best.max(score_tilechoose_node(&mut state, newboard, cprob));
        }
    }
    state.curdepth -= 1;

    best
}

// Returns the value of a computer node in the game tree.
// Plays the part of the Expected Value node in the Expectimax search.
fn score_tilechoose_node(mut state: &mut EvalState, board:u64, mut cprob:f32) -> f32 {
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
        match entry {
            Some(entry) => {
                // We have found a cached entry, return the cached value.
                if entry.depth <= state.curdepth as u8 {
                    state.cachehits+= 1;
                    return entry.heuristic;
                }
            }
            // We have not cached this board, do nothing and continue to calculate the value
            None => {}
        }
    }

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

    res = res / num_open as f32;

    // If we aren't too deep, cache this result for next time.
    if state.curdepth < CACHE_DEPTH_LIMIT {
        let entry = TransTableEntry {depth: state.curdepth as u8, heuristic: res};
        state.trans_table.insert(board, entry);
    }

    res
}

// Sums the scores held in the given table for each row in the given board.
fn score_helper(board: u64, table: &[f32]) -> f32{
    table[((board >>  0) & ROW_MASK) as usize] +
    table[((board >> 16) & ROW_MASK) as usize] +
    table[((board >> 32) & ROW_MASK) as usize] +
    table[((board >> 48) & ROW_MASK) as usize] 
}

// Takes a move and a board and evaluates the value of that move. Begins the expectimax search on this state
fn _score_toplevel_move(mut state: &mut EvalState, board: u64, mv: u8) -> f32 {
    let newboard = execute_move(mv, board);

    if board == newboard {
        return 0.0;
    }

    score_tilechoose_node(&mut state, newboard, 1.0) + 0.000001
}

// Takes a board and a move and sets up the infrastructure to perform the expectimax search on it.
fn score_toplevel_move(board: u64, mv: u8) -> f32 {
    let mut state = EvalState{maxdepth: 0, curdepth: 0, moves_evaled: 0, cachehits:0, depth_limit:0, trans_table: TransTable::new()};
    state.depth_limit = max(3, (count_distinct_tiles(board) - 2));

    let start = SystemTime::now();
    let res = _score_toplevel_move(&mut state, board, mv);
    let diff = match SystemTime::now().duration_since(start) {
        Ok(duration) => duration,
        Err(duration) => {println!("Time error"); duration.duration()}
    };
    /*
    println!("Move {}: result {}: eval'd {} moves ({} cache hits, {} cache size) in {} seconds (maxdepth={}",
        mv,
        res,
        state.moves_evaled,
        state.cachehits,
        state.trans_table.len(),
        diff.as_secs(),
        state.maxdepth);
    */
    res
}

// Takes a board and returns the most effective move to make on it
fn find_best_move(board: u64) -> u8 {
    let mut best: f32 = 0.0;
    let mut bestmove: u8 = 0;

    print_board(board);
    println!("Current scores: heur {}, actual {}", score_heur_board(board), score_board(board));

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

// Returns a 2 or a 4 tile randomly. 10% chance of a 4.
fn draw_tile() -> u64 {
    if rand::thread_rng().gen_range(0,10) < 9 {
        return 1;
    } else {
        return 2;
    }
}

// Inserts the given tile in the given board, in a randomly selected open space.
fn insert_tile_rand(board: u64, mut tile: u64) -> u64 {
    let empty = count_empty(board) as u32;
    if empty == 0 {return board;} // Cannot insert to a full board.
    let mut index: u32 = rand::thread_rng().gen_range(0, empty);
    let mut tmp: u64 = board;

    // Find 'index' empty tiles before inserting the tile. That is, insert the tile in the 'index'th empty space
    loop {
        //Skip over non-empty tiles
        while (tmp & 0xF) != 0 {
            tmp >>= 4;
            tile <<= 4;
        }
        //Empty tile found. If we've already found enough, this is where we will insert.
        if index == 0 { break; }
        // If not skip this tile also.
        index -= 1;
        tmp >>= 4;
        tile <<= 4;
    }
    return board | tile;
}

// Returns a bitboard with two random tiles in it
fn initial_board() -> u64 {
    let board: u64 = draw_tile() << (4 * rand::thread_rng().gen_range(0, 16));
    insert_tile_rand(board, draw_tile())
}

// Uses expectimax search to play one game of 2048 to completion
fn play_game(run_num: u16, get_move: fn(u64) -> u8) -> (u64, f32, f32, f32, u16) {
    let mut board: u64 = initial_board();
    let mut moveno = 0;
    let mut scorepenalty: u32 = 0;

    let start = SystemTime::now();
    
    loop {
        let mv: u8;
        let newboard: u64;
        let mut i = 0;
        while i < 4 {
            if execute_move(i, board) != board {
                break;
            }
            i+=1;
        }
        if i >= 4 {
            break;
        }

        println!("Run {}, Mov #{}, current score={}",run_num, moveno, score_board(board) - scorepenalty as f32);
        moveno += 1;

        mv = get_move(board);
        if mv > 3 {
            break;
        }

        newboard = execute_move(mv, board);
        if newboard == board {
            println!("Illegal Move");
            moveno -= 1;
            continue;
        } else if score_board(newboard) < score_board(board) {
            println!("Merged two 32k tiles, losing score in the process");
            break;
        }

        let tile: u64 = draw_tile();
        if tile == 2 {scorepenalty += 4};
        board = insert_tile_rand(newboard, tile);
    }

    let diff = match SystemTime::now().duration_since(start) {
        Ok(duration) => duration,
        Err(duration) => {println!("Time error"); duration.duration()}
    };
    
    let finalScore = score_board(board) - scorepenalty as f32;
    let time = diff.as_secs();

    print_board(board);
    println!("Game Over. Score: {}. Highest Tile: {}.", finalScore, get_max_rank(board));

    // Return Time, Score, Moves/s, Pts/s, Highest Tile
    (time, finalScore, moveno as f32/time as f32, finalScore/time as f32, get_max_rank(board)) 
}

// Bootstrap: initialise tables and play a game
fn main() {
    
    unsafe{
        init_tables();
    }
    
    let mut results = String::new(); 
    for run  in 0..10 {
        let (time, score, mvsec, ptsec, maxtile) = play_game(run, find_best_move);    
        results.push_str(format!("Run {:2} | Time: {:6} | Score: {:8} | Mv/s: {:3.2} | Pt/s: {:3.2} | Max Tile: {:5}\n",
            run,
            time,
            score,
            mvsec,
            ptsec,
            2<<maxtile).as_str());
        println!("{}", results);
        
    }
}


