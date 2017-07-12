// An implementation of a 4x4 2048 board.
// Heavily inspired by the cpp implementation on github by user 'nneonneo'
extern crate rand;
extern crate term;

mod scoring;
mod board;
mod search;

use scoring::{score_board, score_heur_board};
use board::{get_max_rank, insert_tile_rand, draw_tile, execute_move, print_board};
use board::{initial_board, unpack_col, reverse_row};
use search::{find_best_move};

use std::time::SystemTime;
use std::fs::OpenOptions;
use std::io::prelude::*;


// Tables which are filled with precomputed moves. Any row XORed with ROW_LEFT_TABLE[row] will be the result 
// of swiping that row left, and so on with the other directions.
static mut ROW_LEFT_TABLE:   [u16; 65536] = [0; 65536];
static mut ROW_RIGHT_TABLE:  [u16; 65536] = [0; 65536];
static mut COL_UP_TABLE:     [u64; 65536] = [0; 65536];
static mut COL_DOWN_TABLE:   [u64; 65536] = [0; 65536];

// Precomputed heuristics and scores for single rows also
static mut HEUR_SCORE_TABLE: [f32; 65536] = [0.0; 65536];
static mut SCORE_TABLE:      [f32; 65536] = [0.0; 65536];


// Constants to tune game behaviour
const SCORE_LOST_PENALTY:       f32 = 200000.0;
const SCORE_MONOTONICITY_POWER: f32 = 4.0;
const SCORE_MONOTONICITY_WEIGHT:f32 = 47.0;
const SCORE_SUM_POWER:          f32 = 3.5;
const SCORE_SUM_WEIGHT:         f32 = 11.0;
const SCORE_MERGES_WEIGHT:      f32 = 700.0;
const SCORE_EMPTY_WEIGHT:       f32 = 270.0;

// Masks to extract certain information from a u64 number
const ROW_MASK: u64 = 0xFFFF; 
const COL_MASK: u64 = 0x000F000F000F000F;

static mut CPROB_THRESH_BASE: f32 = 0.5; // Will not evaluate nodes less likely than this

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
        SCORE_TABLE[row as usize] = score;

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
        HEUR_SCORE_TABLE[row as usize] = SCORE_LOST_PENALTY + 
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
        ROW_LEFT_TABLE  [    row]          =                row as u16  ^                result;
        ROW_RIGHT_TABLE [rev_row as usize] =            rev_row         ^            rev_result;
        COL_UP_TABLE    [    row]          = unpack_col(    row as u16) ^ unpack_col(    result);
        COL_DOWN_TABLE  [rev_row as usize] = unpack_col(rev_row)        ^ unpack_col(rev_result);
    }            
}

// Uses expectimax search to play one game of 2048 to completion
fn play_game(run_num: u16, get_move: fn(u64) -> u8) -> (u64, f32, f32, f32, u16) {
    let mut board: u64 = initial_board();
    let mut moveno = 0;
    let mut scorepenalty: u32 = 0;
    let mut got_max_tile = false;

    let start = SystemTime::now();
    
    println!("\n\n\n\n\n"); // This is gross
    loop {

    	print_board(board);

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

        println!("Run {}, Mov #{}, current score={}, max_Tile={}",run_num, moveno, score_board(board) - scorepenalty as f32, 2<<(get_max_rank(board) -1));
        //std::io::stdout().flush();
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
            got_max_tile = true;
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
    
    let final_score = score_board(board) - scorepenalty as f32;
    let time = diff.as_secs();

    //println!("");
    //print_board(board);
    //println!("Game Over. Score: {}. Highest Tile: {}.", final_score, get_max_rank(board));

    // Return Time, Score, Moves/s, Pts/s, Highest Tile
    (time, final_score, moveno as f32/time as f32, final_score/time as f32, if !got_max_tile { get_max_rank(board) } else { 16 }) 
}

// Bootstrap: initialise tables and play a game
fn main() {
    
    const RUNS: u16 = 1;
    const TEST_VALUES: [f32; 1] = [0.01];

    unsafe{
        init_tables();
    
        //let mut results = String::new(); 

        for &threshold in &TEST_VALUES {
            CPROB_THRESH_BASE = threshold;

            let mut times = vec!();
            let mut scores = vec!();
            let mut move_rates = vec!();
            let mut score_rates = vec!();
            let mut max_tiles = vec!();

            for run in 1..RUNS+1 {
                let (time, score, mvsec, ptsec, maxtile) = play_game(run, find_best_move);    

                times.push(time);
                scores.push(score);
                move_rates.push(mvsec);
                score_rates.push(ptsec);
                max_tiles.push(maxtile);
                
                print!("\nPROB THRESH: {} ", CPROB_THRESH_BASE);
                println!("Run {:2} | Time: {:5.1} | Moves/Sec: {:3.2} | Points/Sec: {:3.2} | 2048%: {:3.1} | 4096%: {:3.1} | 8192%: {:3.1} | 16,384%: {:3.1} | 32,768%: {:3.1} | 65,536%: {:3.1}",
                    run,
                    avg2(&times),
                    avg(&move_rates),
                    avg(&score_rates),
                    percent_above(&max_tiles, 11),
                    percent_above(&max_tiles, 12),
                    percent_above(&max_tiles, 13),
                    percent_above(&max_tiles, 14),
                    percent_above(&max_tiles, 15),
                    percent_above(&max_tiles, 16));
                    
                cursor_up(7);

                // Log results to file.
                // We open and close each run so that information is not lost in the event of interruption
                {
                    let mut file = OpenOptions::new()
                                .append(true)
                                .open("results.txt").unwrap();

                    write!(file, "Prob Thresh: {} | Time: {:5.1} | Score: {:6.1} | Mv/Sec: {:3.2} | Pt/Sec: {:3.2} | Max Tile: {}\n",
                        CPROB_THRESH_BASE,
                        time,
                        score,
                        mvsec,
                        ptsec,
                        maxtile).unwrap();
                }
            }

            println!("\n\n\n\n\n\n\n");
        }
    }
}

fn avg(vec: &[f32]) -> f32 {
    let mut res: f32 = 0.0;
    
    for num in vec {
        res += *num;
    }
    
    res/vec.len() as f32
}

fn avg2(vec: &[u64]) -> f32 {
    let mut res: f32 = 0.0;
    
    for num in vec {
        res += *num as f32;
    }
    
    res/vec.len() as f32
}

fn percent_above(vec: &[u16], thresh: u16) -> f32 {
    let mut amnt = 0;

    for num in vec {
        if *num >= thresh {
            amnt += 1;
        }
    }

    (amnt as f32 / vec.len() as f32) * 100.0
}

fn cursor_up(num: u16) {
    let mut term = term::stdout().unwrap();
    
    for _i in 0..num {
        term.cursor_up().unwrap();   
    }
}
