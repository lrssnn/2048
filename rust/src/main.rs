// An implementation of a 4x4 2048 board.
// Heavily inspired by the cpp implementation on github by user 'nneonneo'
extern crate rand;

mod scoring;
mod board;
mod search;

mod generate_tables;
use generate_tables::init_tables;

use scoring::{score_board};
use board::{get_max_rank, insert_tile_rand, draw_tile, execute_move, print_board};
use board::{initial_board};
use search::{find_best_move};

use std::time::SystemTime;
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


// Masks to extract certain information from a u64 number
const ROW_MASK: u64 = 0xFFFF; 
const COL_MASK: u64 = 0x000F000F000F000F;

static mut CPROB_THRESH_BASE: f32 = 0.5; // Will not evaluate nodes less likely than this

// Bootstrap: initialise tables and play a game
fn main() {

    const RUNS: u16 = 5;
    const TEST_VALUES: [f32; 4] = [0.01, 0.005, 0.001, 0.0005];

    let mut summary = String::new();

    unsafe{
        init_tables();
    
        for &threshold in &TEST_VALUES {
            CPROB_THRESH_BASE = threshold;

            print!("Testing {}", threshold);
            std::io::stdout().flush();

            let mut times = vec!();
            let mut scores = vec!();
            let mut move_rates = vec!();
            let mut score_rates = vec!();
            let mut max_tiles = vec!();

            for run in 1..RUNS+1 {
                
                let (time, score, mvsec, ptsec, maxtile) = play_game(run, find_best_move); 

                print!("|");
                std::io::stdout().flush();   

                times.push(time);
                scores.push(score);
                move_rates.push(mvsec);
                score_rates.push(ptsec);
                max_tiles.push(maxtile);
            }

            println!();

            summary += &format!("{:4.4} | Time: {:5.1} | Moves/s: {:7.2} | Points/s: {:9.2} | 2k%: {:5.1} | 4k%: {:5.1} | 8k%: {:5.1} | 16k%: {:5.1} | 32k%: {:5.1} | 64k%: {:5.1}\n",
                    CPROB_THRESH_BASE,
                    avg2(&times),
                    avg(&move_rates),
                    avg(&score_rates),
                    percent_above(&max_tiles, 11),
                    percent_above(&max_tiles, 12),
                    percent_above(&max_tiles, 13),
                    percent_above(&max_tiles, 14),
                    percent_above(&max_tiles, 15),
                    percent_above(&max_tiles, 16));
        }
        println!("\n\n{}", summary);
    }
}


// Uses expectimax search to play one game of 2048 to completion
fn play_game(run_num: u16, get_move: fn(u64) -> u8) -> (u64, f32, f32, f32, u16) {
    let mut board: u64 = initial_board();
    let mut moveno = 0;
    let mut scorepenalty: u32 = 0;
    let mut got_max_tile = false;

    let start = SystemTime::now();
    
    loop {

    	//print_board(board);

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

        //println!("Run {}, Mov #{}, current score={}, max_Tile={}",run_num, moveno, score_board(board) - scorepenalty as f32, 2<<(get_max_rank(board) -1));
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
