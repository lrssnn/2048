// An implementation of a 4x4 2048 board.
// Heavily inspired by the cpp implementation on github by user 'nneonneo'

use rand::Rng;

struct Game {
    board: u64; //Using the board as 64 bit number: cell as 4 bit nibble in board trick

    const row_mask: u64 = 0xFFFF;
    const col_mask: u64 = 0x000F000F000F000F;
}

impl Game {
    fn print_board(board: u64) {
        for i in 0..4 {
            for j in 0..4 {
                let power = board & 0xf; //Take the last byte in the number
                print!("{}", if power == 0 {0} else {2 << power}; //2<<power = 2^power
                board >>= 4; //Next byte
            }
            print!("\n");
        }
        print!("\n");
    }
}
