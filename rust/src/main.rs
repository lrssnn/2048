#![allow(dead_code)]
// An implementation of a 4x4 2048 board.
// Heavily inspired by the cpp implementation on github by user 'nneonneo'
extern crate rand;

use rand::Rng;
use std::time::SystemTime;
use std::cmp::max;
use std::collections::HashMap;

type TransTable = HashMap<u64, TransTableEntry>;

static mut row_left_table:   [u16; 65536] = [0; 65536];
static mut row_right_table:  [u16; 65536] = [0; 65536];
static mut col_up_table:     [u64; 65536] = [0; 65536];
static mut col_down_table:   [u64; 65536] = [0; 65536];
static mut heur_score_table: [f32; 65536] = [0.0; 65536];
static mut score_table:      [f32; 65536] = [0.0; 65536];

const SCORE_LOST_PENALTY:       f32 = 200000.0;
const SCORE_MONOTONICITY_POWER: f32 = 4.0;
const SCORE_MONOTONICITY_WEIGHT:f32 = 47.0;
const SCORE_SUM_POWER:          f32 = 3.5;
const SCORE_SUM_WEIGHT:         f32 = 11.0;
const SCORE_MERGES_WEIGHT:      f32 = 700.0;
const SCORE_EMPTY_WEIGHT:       f32 = 270.0;


const ROW_MASK: u64 = 0xFFFF; 
const COL_MASK: u64 = 0x000F000F000F000F;

fn print_board(mut board: u64) {
    //let mut copy = board;
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

fn unpack_col(row: u16) -> u64 {
    let tmp: u64 = row as u64;
    (tmp | (tmp << 12) | (tmp << 24) | (tmp << 36)) & COL_MASK
}

fn reverse_row(row: u16) -> u16 {
    (row >> 12) | ((row >> 4) & 0x00F0) | ((row << 4) & 0x0F00) | (row << 12)
}

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

unsafe fn init_tables() {
    for row in 0..65536usize {
        let mut line = [
            (row >> 0) & 0xF,
            (row >> 4) & 0xF,
            (row >> 8) & 0xF,
            (row >> 12) & 0xF
        ];

        let mut score: f32 = 0.0;
        for i in 0..4 {
            let rank = line[i];
            if rank >= 2 {
                score += (rank as f32 - 1.0) * (1 << rank) as f32;
            }
        }
        score_table[row as usize] = score;

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

        let mut monotonicity_left : f32 = 0.0;
        let mut monotonicity_right: f32 = 0.0;
        for i in 1..4 {
            if line[i-1] > line[i] {
                monotonicity_left += (line[i-1] as f32).powf(SCORE_MONOTONICITY_POWER) - (line[i] as f32).powf(SCORE_MONOTONICITY_POWER);
            } else {
                monotonicity_right += (line[i] as f32).powf(SCORE_MONOTONICITY_POWER) - (line[i-1] as f32).powf(SCORE_MONOTONICITY_POWER);
            }
        }

        heur_score_table[row as usize] = SCORE_LOST_PENALTY + 
            SCORE_EMPTY_WEIGHT * empty as f32 +
            SCORE_MERGES_WEIGHT * merges as f32 -
            SCORE_MONOTONICITY_WEIGHT * monotonicity_left.min(monotonicity_right)-
            SCORE_SUM_WEIGHT * sum as f32;

        //Exectute a move to the left
        let mut i = 0;
        while i < 3 {
            let mut j = 0;
            while j < 4 {
                if line[j] != 0 {break};
                j += 1;
            }
            if j == 4 {break};

            if line[i] == 0 {
                line[i] = line[j];
                line[j] = 0;
                if i > 0{
                    i -= 1;
                }
            } else if line[i] == line[j] {
                if line[i] != 0xF {
                    line[i] += 1;
                }
                line[j] = 0;
            }
            i+= 1;
        }

        let result: u16 = ((line[0] as u16) << 0) |
                          ((line[1] as u16)<< 4) |
                          ((line[2] as u16) << 8) |
                          ((line[3] as u16) << 12);
        let rev_result: u16 = reverse_row(result);
        let rev_row   : u16 = reverse_row(row as u16);

        row_left_table  [    row] =                row as u16  ^                result;
        row_right_table [rev_row as usize] =            rev_row         ^            rev_result;
        col_up_table    [    row] = unpack_col(    row as u16) ^ unpack_col(    result);
        col_down_table  [rev_row as usize] = unpack_col(rev_row)        ^ unpack_col(rev_result);
    }            
}

unsafe fn execute_move_0(board: u64) -> u64 {
    let mut ret = board;
    let t   = transpose(board);
    println!("This: {}", col_up_table[((t >>  0) & ROW_MASK) as usize]);
    ret ^= col_up_table[((t >>  0) & ROW_MASK) as usize] << 0;
    ret ^= col_up_table[((t >> 16) & ROW_MASK) as usize] << 4;
    ret ^= col_up_table[((t >> 32) & ROW_MASK) as usize] << 8;
    ret ^= col_up_table[((t >> 48) & ROW_MASK) as usize] << 12;
    ret
}

unsafe fn execute_move_1(board: u64) -> u64 {
    let mut ret = board;
    let t   = transpose(board);
    ret ^= col_down_table[((t >>  0) & ROW_MASK) as usize] << 0;
    ret ^= col_down_table[((t >> 16) & ROW_MASK) as usize] << 4;
    ret ^= col_down_table[((t >> 32) & ROW_MASK) as usize] << 8;
    ret ^= col_down_table[((t >> 48) & ROW_MASK) as usize] << 12;
    ret
}

unsafe fn execute_move_2(board: u64) -> u64 {
    let mut ret = board;
    ret ^= (row_left_table[((board >>  0) & ROW_MASK) as usize] as u64) <<  0;
    ret ^= (row_left_table[((board >> 16) & ROW_MASK) as usize] as u64) << 16;
    ret ^= (row_left_table[((board >> 32) & ROW_MASK) as usize] as u64) << 32;
    ret ^= (row_left_table[((board >> 48) & ROW_MASK) as usize] as u64) << 48;
    ret
}

unsafe fn execute_move_3(board: u64) -> u64 {
    let mut ret = board;
    ret ^= (row_right_table[((board >>  0) & ROW_MASK) as usize] as u64) <<  0;
    ret ^= (row_right_table[((board >> 16) & ROW_MASK) as usize] as u64) << 16;
    ret ^= (row_right_table[((board >> 32) & ROW_MASK) as usize] as u64) << 32;
    ret ^= (row_right_table[((board >> 48) & ROW_MASK) as usize] as u64) << 48;
    ret
}

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

fn get_max_rank(mut board: u64) -> u16 {
    let mut maxrank: u16 = 0;
    while board != 0 {
        maxrank = max(maxrank, (board & 0xF) as u16);
        board >>= 4;
    }
    maxrank
}

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

struct EvalState {
    trans_table: TransTable,
    maxdepth: u32,
    curdepth: u32,
    cachehits: u32,
    moves_evaled: u64,
    depth_limit: u32,
}

fn score_heur_board(board: u64) -> f32 {
    unsafe{
        score_helper(          board , &heur_score_table) +
        score_helper(transpose(board), &heur_score_table)
    }
}

fn score_board(board: u64)  -> f32 {
    unsafe{
        score_helper(board, &score_table)
    }
}

const CPROB_THRESH_BASE: f32 = 0.0001; // Will not evaluate nodes less likely than this
const CACHE_DEPTH_LIMIT: u32 = 15;

fn score_move_node(mut state: &mut EvalState, board: u64, cprob: f32) -> f32 {
    let mut best: f32 = 0.0;
    state.curdepth+= 1;
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

fn score_tilechoose_node(mut state: &mut EvalState, board:u64, mut cprob:f32) -> f32 {
    if cprob < CPROB_THRESH_BASE || state.curdepth >= state.depth_limit {
        state.maxdepth = max(state.curdepth, state.maxdepth);
        return score_heur_board(board);
    }
    if state.curdepth < CACHE_DEPTH_LIMIT {
        // What I think is going on here:
        // getting an iterator to all the values that match board
        // iterating over them to find one that matches is at a lower depth

        let entry = state.trans_table.get(&board);  // Not sure how to handle this in rust.
        match entry {
            Some(entry) => {
                if entry.depth <= state.curdepth as u8 {
                    state.cachehits+= 1;
                    return entry.heuristic;
                }
            }
            None => {println!("Cache error");}
        }
    }

    let num_open = count_empty(board);
    cprob /= num_open as f32;

    let mut res: f32 = 0.0;
    let mut tmp = board;
    let mut tile_2: u64 = 1;

    while tile_2 != 0 {
        if (tmp & 0xF) == 0 {
            res += score_move_node(&mut state, board |  tile_2      , cprob * 0.9) * 0.9;
            res += score_move_node(&mut state, board | (tile_2 << 1), cprob * 0.1) * 0.1;
        }
        tmp >>= 4;
        tile_2 <<= 4;
    }
    res = res / num_open as f32;

    if state.curdepth < CACHE_DEPTH_LIMIT {
        let entry = TransTableEntry {depth: state.curdepth as u8, heuristic: res};
        state.trans_table.insert(board, entry);
    }

    res
}


fn score_helper(board: u64, table: &[f32]) -> f32{
    table[((board >>  0) & ROW_MASK) as usize] +
    table[((board >> 16) & ROW_MASK) as usize] +
    table[((board >> 32) & ROW_MASK) as usize] +
    table[((board >> 48) & ROW_MASK) as usize] 
}

fn _score_toplevel_move(mut state: &mut EvalState, board: u64, mv: u8) -> f32 {
    let newboard = execute_move(mv, board);

    if board == newboard {
        return 0.0;
    }

    score_tilechoose_node(&mut state, newboard, 1.0) + 0.000001
}

fn score_toplevel_move(board: u64, mv: u8) -> f32 {
    let mut state = EvalState{maxdepth: 0, curdepth: 0, moves_evaled: 0, cachehits:0, depth_limit:0, trans_table: TransTable::new()};
    state.depth_limit = max(3, (count_distinct_tiles(board) - 2));

    let start = SystemTime::now();
    let res = _score_toplevel_move(&mut state, board, mv);
    let diff = match SystemTime::now().duration_since(start) {
        Ok(duration) => duration,
        Err(duration) => {println!("Time error"); duration.duration()}
    };
    
    println!("Move {}: result {}: eval'd {} moves ({} cache hits, {} cache size) in {} seconds (maxdepth={}",
        mv,
        res,
        state.moves_evaled,
        state.cachehits,
        state.trans_table.len(),
        diff.as_secs(),
        state.maxdepth);

    res
}

fn find_best_move(board: u64) -> u8 {
    let mut best: f32 = 0.0;
    let mut bestmove: u8 = 0;

    print_board(board);
    println!("Current scores: heur {}, actual {}", score_heur_board(board), score_board(board));

    for mv in 0..4 {
        let res = score_toplevel_move(board, mv);

        if res > best {
            best = res;
            bestmove = mv;
        }
    }
    bestmove
}

fn draw_tile() -> u64 {
    if rand::thread_rng().gen_range(0,10) < 9 {
        return 1;
    } else {
        return 2;
    }
}

fn insert_tile_rand(board: u64, mut tile: u64) -> u64 {
    let mut index: u32 = rand::thread_rng().gen_range(0, count_empty(board) as u32);
    let mut tmp: u64 = board;
    loop {
        while (tmp & 0xF) != 0 {
            tmp >>= 4;
            tile <<= 4;
        }
        if index == 0 { break; }
        index -= 1;
        tmp >>= 4;
        tile <<= 4;
    }
    return board | tile;
}

fn initial_board() -> u64 {
    let board: u64 = draw_tile() << (4 * rand::thread_rng().gen_range(0, 16));
    insert_tile_rand(board, draw_tile())
}

fn play_game(get_move: fn(u64) -> u8) {
    let mut board: u64 = initial_board();
    let mut moveno = 0;
    let mut scorepenalty: u32 = 0;

'out: loop {
        let mv: u8;
        let newboard: u64;

        for i in 0..4 {
            if execute_move(i, board) != board {
                break 'out;
            }
        }

        println!("Mov #{}, current score={}", moveno, score_board(board) - scorepenalty as f32);
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
        }

        let tile: u64 = draw_tile();
        if tile == 2 {scorepenalty += 4};
        board = insert_tile_rand(newboard, tile);
    }

    print_board(board);
    println!("Game Over. Score: {}. Highest Tile: {}.", score_board(board) -(scorepenalty as f32), get_max_rank(board));
}

fn main() {
    
    unsafe{
        init_tables();
    }
   // play_game(find_best_move);
    
    let board: u64 = 0x1111000000000004;
    println!("Board:");
    print_board(board);
    println!("Transpose:");
    print_board(transpose(board));
 
    unsafe{
        println!("Move 0:");
        print_board(execute_move_0(board));
        println!("Move 1:");
        print_board(execute_move_1(board));
        println!("Move 2:");
        print_board(execute_move_2(board));
        println!("Move 3:");
        print_board(execute_move_3(board));
    }
    print_board(board);

    let col = 0x2223;
    print_board(unpack_col(col));
    print_board(unpack_col(col) << 4);
    print_board(unpack_col(col) << 8);
    print_board(unpack_col(col) << 12);
}

struct TransTableEntry {
    depth: u8,
    heuristic: f32
}
