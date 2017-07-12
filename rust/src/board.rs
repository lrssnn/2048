use super::rand;
use rand::Rng;
use std::cmp::max;

use super::ROW_LEFT_TABLE;
use super::ROW_RIGHT_TABLE;
use super::COL_UP_TABLE;
use super::COL_DOWN_TABLE;

use super::COL_MASK;
use super::ROW_MASK;

// Return the result of the specified move on the given board.
// mv: 0 -> up
//     1 -> down
//     2 -> right
//     3 -> left
// Any other value of mv will return a 0 board.
pub fn execute_move(mv: u8, board: u64) -> u64 {
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

// Swipe the given board up
unsafe fn execute_move_0(board: u64) -> u64 {
    // Every row has a precomputed result, so we simply transpose to convert columns to rows, and combine the
    // results of each row in turn.
    let mut ret = board;
    let t   = transpose(board);
    ret ^= COL_UP_TABLE[((t >>  0) & ROW_MASK) as usize] << 0;
    ret ^= COL_UP_TABLE[((t >> 16) & ROW_MASK) as usize] << 4;
    ret ^= COL_UP_TABLE[((t >> 32) & ROW_MASK) as usize] << 8;
    ret ^= COL_UP_TABLE[((t >> 48) & ROW_MASK) as usize] << 12;
    ret
}

// Swipe the given board down
unsafe fn execute_move_1(board: u64) -> u64 {
    let mut ret = board;
    let t   = transpose(board);
    ret ^= COL_DOWN_TABLE[((t >>  0) & ROW_MASK) as usize] << 0;
    ret ^= COL_DOWN_TABLE[((t >> 16) & ROW_MASK) as usize] << 4;
    ret ^= COL_DOWN_TABLE[((t >> 32) & ROW_MASK) as usize] << 8;
    ret ^= COL_DOWN_TABLE[((t >> 48) & ROW_MASK) as usize] << 12;
    ret
}

// Swipe the given board left
unsafe fn execute_move_2(board: u64) -> u64 {
    let mut ret = board;
    ret ^= (ROW_LEFT_TABLE[((board >>  0) & ROW_MASK) as usize] as u64) <<  0;
    ret ^= (ROW_LEFT_TABLE[((board >> 16) & ROW_MASK) as usize] as u64) << 16;
    ret ^= (ROW_LEFT_TABLE[((board >> 32) & ROW_MASK) as usize] as u64) << 32;
    ret ^= (ROW_LEFT_TABLE[((board >> 48) & ROW_MASK) as usize] as u64) << 48;
    ret
}

// Swipe the given board right
unsafe fn execute_move_3(board: u64) -> u64 {
    let mut ret = board;
    ret ^= (ROW_RIGHT_TABLE[((board >>  0) & ROW_MASK) as usize] as u64) <<  0;
    ret ^= (ROW_RIGHT_TABLE[((board >> 16) & ROW_MASK) as usize] as u64) << 16;
    ret ^= (ROW_RIGHT_TABLE[((board >> 32) & ROW_MASK) as usize] as u64) << 32;
    ret ^= (ROW_RIGHT_TABLE[((board >> 48) & ROW_MASK) as usize] as u64) << 48;
    ret
}

// Returns the maximum tile rank (power of 2) present on the given bitboard
pub fn get_max_rank(mut board: u64) -> u16 {
    let mut maxrank: u16 = 0;
    // Simply consume the board nibble by nibble and track the highest tile seen.
    while board != 0 {
        maxrank = max(maxrank, (board & 0xF) as u16);
        board >>= 4;
    }
    maxrank
}

// Returns the number of unique tiles on the board
pub fn count_distinct_tiles(mut board: u64) -> u32 {
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

// Takes a column as a 16 bit number, and returns an empty bitboard with that column as the first column
pub fn unpack_col(row: u16) -> u64 {
    let tmp: u64 = row as u64;
    (tmp | (tmp << 12) | (tmp << 24) | (tmp << 36)) & COL_MASK
}

// Takes a row as a 16 bit number and returns the reverse of it
pub fn reverse_row(row: u16) -> u16 {
    (row >> 12) | ((row >> 4) & 0x00F0) | ((row << 4) & 0x0F00) | (row << 12)
}

// Returns the number of open spaces in the given bitboard
pub fn count_empty(mut board: u64) -> u64 {
    board |= (board >> 2) & 0x3333333333333333;
    board |=  board >> 1;
    board  = !board & 0x1111111111111111;

    board += board >> 32;
    board += board >> 16;
    board += board >>  8;
    board += board >>  4;
    
    board & 0xF as u64
}

// Returns a 2 or a 4 tile randomly. 10% chance of a 4.
pub fn draw_tile() -> u64 {
    if rand::thread_rng().gen_range(0,10) < 9 {
        1
    } else {
        2
    }
}

// Inserts the given tile in the given board, in a randomly selected open space.
pub fn insert_tile_rand(board: u64, mut tile: u64) -> u64 {
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
    board | tile
}

// Returns a bitboard with two random tiles in it
pub fn initial_board() -> u64 {
    let board: u64 = draw_tile() << (4 * rand::thread_rng().gen_range(0, 16));
    insert_tile_rand(board, draw_tile())
}

// Prints the bitboard in a human readable format
pub fn print_board(mut board: u64) {
    for _i in 0..4 {
        for _j in 0..4 {
            let power = board & 0xf; //Take the last byte in the number
            print!("{:5},", if power == 0 {0} else {2 << (power-1)}); //2<<power = 2^power
            board >>= 4; //Next byte
        }
        println!("");
    }
    println!("");
}

// Takes a bitboard and returns the transposition of that board
// a b c d     a e i m
// e f g h  => b f j n
// i j k l     c g k o
// m n o p     d h l p
pub fn transpose(board: u64) -> u64 {
    let a1: u64 = board & 0xF0F00F0FF0F00F0F;
    let a2: u64 = board & 0x0000F0F00000F0F0;
    let a3: u64 = board & 0x0F0F00000F0F0000;
    let a : u64 = a1 | (a2 << 12) | (a3 >> 12);
    let b1: u64 = a & 0xFF00FF0000FF00FF;
    let b2: u64 = a & 0x00FF00FF00000000;
    let b3: u64 = a & 0x00000000FF00FF00;
    b1 | (b2 >> 24) | (b3 << 24)
}
