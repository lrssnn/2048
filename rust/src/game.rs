// An implementation of a 4x4 2048 board.
// Heavily inspired by the cpp implementation on github by user 'nneonneo'

use rand::Rng;

static row_left_table:   [u16: 65536];
static row_right_table:  [u16: 65536];
static col_up_table:     [u64: 65536];
static col_down_table:   [u64: 65536];
static heur_score_table: [f32: 65536];
static score_table:      [f32: 65536];

const SCORE_LOST_PENALTY:       f32 = 200000.0;
const SCORE_MONOTONICITY_POWER: f32 = 4.0;
const SCORE_MONOTONICITY_WEIGHT:f32 = 47.0;
const SCORE_SUM_POWER:          f32 = 3.5;
const SCORE_SUM_WEIGHT:         f32 = 11.0;
const SCORE_MERGES_WEIGHT:      f32 = 700.0;
const SCORE_MERGES_POWER:       f32 = 270.0;

struct Game {
    board: u64; //Using the board as 64 bit number: cell as 4 bit nibble in board trick

    const ROW_MASK: u64 = 0xFFFF; 
    const COL_MASK: u64 = 0x000F000F000F000F;
}

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

fn unpack_col(row: u16) -> u64 {
    let tmp: u64 = row;
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

fn count_empty(board: u64) -> u8 {
    board |= (board >> 2) & 0x3333333333333333;
    board |= (board >> 1);
    board  = !x & 0x1111111111111111;

    board += x >> 32;
    board += x >> 16;
    board += x >>  8;
    board += x >>  4;
    
    x & 0xF
}

fn init_tables() {
    for row in 0..65536 {
        let line = [
            (row >> 0) & 0xF,
            (row >> 4) & 0xF,
            (row >> 8) & 0xF,
            (row >> 12) & 0xF
        ];

        let score: f32 = 0.0;
        for i in 0..4 {
            let rank = line[i];
            if rank >= 2 {
                score += (rank - 1) * (1 << rank);
            }
        }
        score_table[row] = score;

        let sum: f32 = 0;
        let empty = 0;
        let merges = 0;

        let prev = 0;
        let counter = 0;
        for i in 0..4 {
            let rank = line[i];
            sum += rank.powf(SCORE_SUM_POWER);
            if (rank == 0) {
                empty++;
            } else {
                if (prev == rank) {
                    counter++;
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

        let monotonicity_left = 0;
        let monotonicity_right = 0;
        for i in 1..4 {
            if line[i-1] > line[i] {
                monotonicity_left += line[i-1].powf(SCORE_MONOTONICITY_POWER) - line[i].powf(SCORE_MONOTONICITY_POWER);
            } else {
                monotonicity_right += line[i].powf(SCORE_MONOTONICITY_POWER) - line[i-1].powf(SCORE_MONOTONICITY_POWER);
            }
        }

        heur_score_table[row] = SCORE_LOST_PENALTY + 
            SCORE_EMPTY_WEIGHT * empty +
            SCORE_MERGES_WEIGHT * merges -
            SCORE_MONOTONICITY_WEIGHT * min(monotonicity_left, monotonicity_right) -
            SCORE_SUM_WEIGHT * sum;

        //Exectute a move to the left
        let i = 0;
        while i < 3 {
            let j = 0;
            while j < 4 {
                if (line[j] != 0) break;
                j++;
            }
            if j == 4 break;

            if line[i] == 0 {
                line[i] = line[j];
                line[j] = 0;
                i--;
            } else if line[i] == line[j] {
                if line[i] != 0xF {
                    line[i]++;
                }
                line[j] = 0;
            }
            i++;
        }

        let result: u16 = (line[0] << 0) |
                          (line[1] << 4) |
                          (line[2] << 8) |
                          (line[3] << 12);
        let rev_result: u16 = reverse_row(result);
        let rev_row = reverse_row(row);

        row_left_table  [    row] =                row  ^                result;
        row_right_table [rev_row] =            rev_row  ^            rev_result;
        col_left_table  [    row] = unpack_col(    row) ^ unpack_col(    result);
        col_right_table [rev_row] = unpack_col(rev_row) ^ unpack_col(rev_result);
    }            
}

fn execute_move_0(board: u64) -> u64 {
    let ret = board;
    let t   = transpose(board);
    ret ^= col_up_table[(t >>  0) & ROW_MASK] << 0;
    ret ^= col_up_table[(t >> 16) & ROW_MASK] << 4;
    ret ^= col_up_table[(t >> 32) & ROW_MASK] << 8;
    ret ^= col_up_table[(t >> 48) & ROW_MASK] << 12;
    ret
}

fn execute_move_1(board: u64) -> u64 {
    let ret = board;
    let t   = transpose(board);
    ret ^= col_down_table[(t >>  0) & ROW_MASK] << 0;
    ret ^= col_down_table[(t >> 16) & ROW_MASK] << 4;
    ret ^= col_down_table[(t >> 32) & ROW_MASK] << 8;
    ret ^= col_down_table[(t >> 48) & ROW_MASK] << 12;
    ret
}

fn execute_move_2(board: u64) -> u64 {
    let ret = board;
    ret ^= row_left_table[(t >>  0) & ROW_MASK] << 0;
    ret ^= row_left_table[(t >> 16) & ROW_MASK] << 16;
    ret ^= row_left_table[(t >> 32) & ROW_MASK] << 32;
    ret ^= row_left_table[(t >> 48) & ROW_MASK] << 48;
    ret
}

fn execute_move_3(board: u64) -> u64 {
    let ret = board;
    ret ^= row_right_table[(t >>  0) & ROW_MASK] << 0;
    ret ^= row_right_table[(t >> 16) & ROW_MASK] << 16;
    ret ^= row_right_table[(t >> 32) & ROW_MASK] << 32;
    ret ^= row_right_table[(t >> 48) & ROW_MASK] << 48;
    ret
}

fn execute_move(move: u8, board: u64) -> u64 {
    match move {
        0 => execute_move_0(board);
        1 => execute_move_1(board);
        2 => execute_move_2(board);
        3 => execute_move_3(board);
    }
}

fn get_max_rank(board: u64) -> u16 {
    let maxrank: u16 = 0;
    while board != 0 {
        maxrank = max(maxrank, board & 0xF);
        board >>= 4;
    }
    maxrank
}

fn count_distinct_tiles(board: u64) -> u16 {
    let bitset: u16 = 0;
    while board != 0 {
        bitset |= 1 << (board & 0xF);
        board >>= 4;
    }

    bitset >>= 1;

    let count = 0;
    while bitset != 0 {
        bitset &= bitset - 1;
        count++;
    }
    count
}

struct eval_state {
    trans_table: Trans_Table;
    maxdepth: u32;
    curdepth: u32;
    cachehits: u32;
    moves_evaled: u64;
    depth_limit: u32;
}

fn score_heur_board(board: u64) -> f32 {
    score_helper(          board , heur_score_table) +
    score_helper(transpose[board], heur_score_table)
}

fn score_board(board: u64)  -> f32 {
    score_helper(board, score_table)
}

const CPROB_THRESH_BASE: f32 = 0.0001; // Will not evaluate nodes less likely than this
const CACHE_DEPTH_LIMIT: u32 = 15;

fn score_move_node(&state: eval_state, board: u64, cprop: f32) -> f32 {
    let best: f32 = 0;
    state.curdepth++;
    for mv in 0..4 {
        newboard: u64 = execute_move(move, board);
        state.moves_evaled++;

        if board != newboard {
            best = max(best, score_tilechoose_node(state, newboard, cprob));
        }
    }
    state.curdepth--;

    best
}

fn score_tilechoose_node(&state: eval_state, board:u64, cprob:f32) -> f32 {
    if cprob < CPROB_THRESH_BASE || state.curdepth >= state.depth_limit {
        state.maxdepth = max(state.curdepth, state.maxdepth);
        return score_heur_board(board);
    }
    if state.curdepth < CACHE_DEPTH_LIMIT {
        let i = state.trans_table.find(board);  // Not sure how to handle this in rust.
        if i != state.trans_table.end() {       // i should be an iterator
            entry: trans_table_entry_t = t.second;

            if entry.depth <= state.curdepth {
                state.cachehits++;
                return entry.heuristic;
            }
        }
    }

    let num_open = count_empty(board);
    cprob /= num_open;

    let res: f32 = 0;
    let tmp = board;
    let tile_2: u64 = 1;

    while tile_2 != 0 {
        if (tmp & 0xF) == 0 {
            res += score_move_node(state, board |  tile_2      , cprob * 0.9) * 0.9;
            res += score_move_node(state, board | (tile_2 << 1), cprob * 0.1) * 0.1;
        }
        tmp >>= 4;
        tile_2 <<= 4;
    }
    res = res / num_open;

    if state.curdepth < CACHE_DEPTH_LIMIT {
        entry: trans_table_entry_t = (state.curdepth, res);
        state.trans_table[board] = entry;
    }

    res
}
}

fn score_helper(board: u64, table: [u32]){
    table[(board >>  0) & ROW_MASK] +
    table[(board >> 16) & ROW_MASK] +
    table[(board >> 32) & ROW_MASK] +
    table[(board >> 48) & ROW_MASK] 
}

fn _score_toplevel_move(state: eval_state, board: u64, mv: u32) -> f32 {
    newboard = execute_move(mv, board);

    if board == newboard {
        return 0;
    }

    score_tilechoose_node(state, newboard, 1) + 0.000001
}

fn score_toplevel_move(board: u64, mv: u32){
    let res: f32;
    let start, diff: SystemTime;

    let state: eval_state;
    state.depth_limit = max(3, count_distinct_tiles(board) - 2);

    start = SystemTime::now();
    res = _score_toplevel_move(state, board, mv);
    diff = SystemTime.now().duration_since(start);
    
    println!("Move {}: result {}: eval'd {} moves ({} cache hits, {} cache size) in {} seconds (maxdepth={}",
        mv,
        res,
        state.moves_evaled,
        state.cachehits,
        state.trans_table.size(),
        elapsed,
        state.maxdepth);

    res
}

fn find_best_move(board: u64) -> u32 {
    let mv: u32;
    let best: f32 = 0;
    let bestMove: u32 = -1;

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

fn insert_tile_rand(board: u64, tile: u64) {
    let index: u32 = rand::thread_rng().gen_range(0, count_empty(board));
    let tmp: u64 = board;
    loop {
        while (tmp & 0xF) != 0 {
            tmp >>= 4;
            tile <<= 4;
        }
        if index == 0 { break; }
        --index;
        tmp >>= 4;
        tile <<= 4;
    }
    return board | tile;
}

fn initial_board() -> u64 {
    board: u64 = draw_tile() << (4 * rand::thread_rng().gen_range(0, 16));
    insert_tile_rand(board, draw_tile());
}

fn play_game(get_move: get_move_func_t) {
    let board: u64 = initial_board();
    let moveno = 0;
    let scorepenalty = 0;

    loop {
        let mv: u32;
        let newboard: u64;

        for mv in 0..4 {
            if execute_move(mv, board) != board {
                break;
            }
        }
        if mv == 4 {
            break;
        }

        println!("Mov #{}, current score={}", ++moveno, score_board - scorepenalty);

        mv = get_move(board);
        if mv < 0 {
            break;
        }

        newboard = execute_move(mv, board);
        if newboard == board {
            println!("Illegal Move");
            moveno--;
            continue;
        }

        let tile: u64 = draw_tile();
        if (tile == 2) scorepenalty += 4;
        board = insert_tile_rand(newboard, tile);
    }

    print_board(board);
    println!("Game Over. Score: {}. Highest Tile: {}.", score_board(board) - scorepenalty, get_max_rank(board);
}

fn main() {
    init_tables();
    play_game(find_best_move);
}

