// Keep the official parser, replay, scorer, and generator as the single source of truth. The
// browser-specific adapter below only converts an official frame into compact typed arrays.
#[path = "../../tools/src/lib.rs"]
#[allow(dead_code)]
mod official_tools;

use std::cell::RefCell;
use std::collections::HashMap;

const EMPTY_CELL: u16 = u16::MAX;

struct ParsedCase {
    input: official_tools::Input,
    output: official_tools::Output,
}

thread_local! {
    static SESSION: RefCell<Option<ParsedCase>> = RefCell::new(None);
}

pub struct PreparedData {
    pub N: usize,
    pub max_turn: usize,
    pub default_turn: usize,
    pub score: i64,
    pub error: String,
    pub grass: Vec<u8>,
    pub grass_cells: usize,
    pub max_time_left: i64,
}

pub struct FrameData {
    pub turn: usize,
    pub grid: Vec<u16>,
    pub money: i64,
    pub now: i64,
    pub arrival_id: i32,
    pub arrival_accepted: bool,
    pub arrival_s: i64,
    pub arrival_t: i64,
    pub arrival_p: usize,
    pub arrival_v: i64,
    pub moved: Vec<u16>,
    pub move_sources: Vec<u16>,
    pub departed: Vec<f64>,
    pub actives: Vec<f64>,
    pub cells_used: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub total_fee: i64,
    pub total_move_cost: i64,
    pub comment: String,
}

// Each record is [group id, cell count, row-major cell indices...]. Keeping only moved groups
// avoids sending another full 50 x 50 grid when most turns move few or no groups.
fn encode_move_sources(previous_grid: &[Vec<usize>], moved: &[usize]) -> Result<Vec<u16>, String> {
    if moved.is_empty() {
        return Ok(vec![]);
    }

    let slot_of: HashMap<usize, usize> = moved
        .iter()
        .copied()
        .enumerate()
        .map(|(slot, gid)| (gid, slot))
        .collect();
    let mut cells_of = vec![Vec::<u16>::new(); moved.len()];
    for (index, &gid) in previous_grid.iter().flatten().enumerate() {
        let Some(&slot) = slot_of.get(&gid) else {
            continue;
        };
        let index = u16::try_from(index)
            .map_err(|_| format!("Move source cell index {} does not fit in u16", index))?;
        cells_of[slot].push(index);
    }

    let encoded_len = cells_of.iter().map(|cells| cells.len() + 2).sum();
    let mut encoded = Vec::with_capacity(encoded_len);
    for (&gid, cells) in moved.iter().zip(cells_of) {
        if cells.is_empty() {
            return Err(format!("Move source for group {} is missing", gid));
        }
        encoded.push(
            u16::try_from(gid).map_err(|_| format!("Group index {} does not fit in u16", gid))?,
        );
        encoded.push(
            u16::try_from(cells.len())
                .map_err(|_| format!("Move source size for group {} does not fit in u16", gid))?,
        );
        encoded.extend(cells);
    }
    Ok(encoded)
}

pub fn generate(seed: i32) -> String {
    official_tools::gen(seed.max(0) as u64).to_string()
}

pub fn prepare(input_text: &str, output_text: &str) -> Result<PreparedData, String> {
    if input_text.trim().is_empty() {
        return Err("Input is empty.".to_owned());
    }

    let input = official_tools::parse_input(input_text);
    let output = official_tools::parse_output(&input, output_text);
    let max_turn = output.frames.len().saturating_sub(1);
    // A complete replay ends with the park empty. Match the official web visualizer and show the
    // final arrival instead; for an invalid/truncated replay the last valid frame is informative.
    let back = if output.error.is_some() { 1 } else { 2 };
    let default_turn = output.frames.len().saturating_sub(back);
    let grass: Vec<u8> = input
        .grass
        .iter()
        .flatten()
        .map(|&cell| u8::from(cell))
        .collect();
    let grass_cells = grass.iter().filter(|&&cell| cell != 0).count();
    let prepared = PreparedData {
        N: input.n,
        max_turn,
        default_turn,
        score: output.score,
        error: output.error.clone().unwrap_or_default(),
        grass,
        grass_cells,
        max_time_left: official_tools::time_scale_end(&input),
    };

    SESSION.with(|session| {
        *session.borrow_mut() = Some(ParsedCase { input, output });
    });
    Ok(prepared)
}

pub fn frame(turn: usize) -> Result<FrameData, String> {
    SESSION.with(|session| {
        let session = session.borrow();
        let parsed = session
            .as_ref()
            .ok_or_else(|| "Visualizer session is not prepared.".to_owned())?;
        let turn = turn.min(parsed.output.frames.len().saturating_sub(1));
        let frame = &parsed.output.frames[turn];

        let grid = frame
            .grid
            .iter()
            .flatten()
            .map(|&gid| {
                if gid == usize::MAX {
                    EMPTY_CELL
                } else {
                    gid as u16
                }
            })
            .collect();
        let moved = frame.moved.iter().map(|&gid| gid as u16).collect();
        let move_sources = if frame.moved.is_empty() {
            vec![]
        } else {
            let previous_turn = turn
                .checked_sub(1)
                .ok_or_else(|| format!("Previous frame for turn {} is missing", turn))?;
            let previous = parsed
                .output
                .frames
                .get(previous_turn)
                .ok_or_else(|| format!("Previous frame for turn {} is missing", turn))?;
            encode_move_sources(&previous.grid, &frame.moved)?
        };
        let departed = frame
            .departed
            .iter()
            .flat_map(|&(gid, fee)| [gid as f64, fee as f64])
            .collect();
        // Seven values per active group: id, P, V, T, L, Lmax, and the fee if it left now.
        let actives = frame
            .actives
            .iter()
            .flat_map(|active| {
                [
                    active.id as f64,
                    active.p as f64,
                    active.v as f64,
                    active.t as f64,
                    active.l as f64,
                    active.max_l as f64,
                    active.fee as f64,
                ]
            })
            .collect();

        let (arrival_id, arrival_accepted, arrival_s, arrival_t, arrival_p, arrival_v) =
            match frame.arrival {
                Some((gid, accepted)) => {
                    let group = parsed.input.groups[gid];
                    (gid as i32, accepted, group.s, group.t, group.p, group.v)
                }
                None => (-1, false, 0, 0, 0, 0),
            };

        Ok(FrameData {
            turn,
            grid,
            money: frame.money,
            now: frame.now,
            arrival_id,
            arrival_accepted,
            arrival_s,
            arrival_t,
            arrival_p,
            arrival_v,
            moved,
            move_sources,
            departed,
            actives,
            cells_used: frame.cells_used,
            accepted: frame.accepted,
            rejected: frame.rejected,
            total_fee: frame.total_fee,
            total_move_cost: frame.total_move_cost,
            comment: parsed
                .output
                .comments
                .get(turn)
                .cloned()
                .unwrap_or_default(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::encode_move_sources;

    #[test]
    fn move_sources_are_encoded_by_moved_order_and_row_major_cell_index() {
        let previous_grid = vec![vec![1, 1, 9], vec![3, 1, 9], vec![3, 9, 9]];

        let encoded = encode_move_sources(&previous_grid, &[1, 3]).unwrap();

        assert_eq!(encoded, vec![1, 3, 0, 1, 4, 3, 2, 3, 6]);
    }

    #[test]
    fn no_moves_produce_no_source_records() {
        let previous_grid = vec![vec![1, 1], vec![1, 1]];

        assert!(encode_move_sources(&previous_grid, &[]).unwrap().is_empty());
    }
}
