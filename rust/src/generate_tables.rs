use super::ROW_LEFT_TABLE;
use super::ROW_RIGHT_TABLE;
use super::COL_UP_TABLE;
use super::COL_DOWN_TABLE;
use super::HEUR_SCORE_TABLE;
use super::SCORE_TABLE;

use super::board::*;

// Constants to tune game behaviour
const SCORE_LOST_PENALTY:       f32 = 200000.0;
const SCORE_MONOTONICITY_POWER: f32 = 4.0;
const SCORE_MONOTONICITY_WEIGHT:f32 = 47.0;
const SCORE_SUM_POWER:          f32 = 3.5;
const SCORE_SUM_WEIGHT:         f32 = 11.0;
const SCORE_MERGES_WEIGHT:      f32 = 700.0;
const SCORE_EMPTY_WEIGHT:       f32 = 270.0;


// Initialises the precomputed tables used to execute moves and score states
pub unsafe fn init_tables() {
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
