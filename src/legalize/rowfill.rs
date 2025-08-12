// Legalize in a simple row-by-row manner.
// Sort by Y axis, and then pack cells into rows, with
// a target row length.
// The target row length is simply the total width
// of the blocks, divided by the number of rows.

// use bookshelf_r::bookshelf::BookshelfCircuit;
use super::{LegalBlock, LegalParams, LegalPosition, LegalProblem};

fn make_row(row: &mut Vec<LegalBlock>, row_origin: f32, positions: &mut Vec<LegalPosition>) {
    row.sort_by(|a, b| a.x.partial_cmp(&b.x).expect("Could not compare"));
    let mut x = 0.0;
    for b in row {
        positions.push(LegalPosition {
            block_tag: b.tag,
            x: x,
            y: row_origin,
            h: b.h,
            w: b.w,
            original_x: b.x,
            original_y: b.y,
        });
        x = x + b.w;
    }
}

pub fn legalize(lp: &LegalProblem) -> Vec<LegalPosition> {
    let mut blocks = lp.blocks.clone();

    blocks.sort_by(|a, b| a.y.partial_cmp(&b.y).expect("Could not compare"));
    let nr = lp.params.grid_y;

    let mut total_width = 0.0;
    for b in &blocks {
        total_width += b.w;
    }
    let target = total_width as f32 / nr as f32 + 0.5;

    let mut positions = Vec::new();

    let mut row = Vec::new();
    let mut rn = 0.0;

    let mut width = 0.0;

    for b in &blocks {
        if width > target {
            make_row(&mut row, rn * lp.params.step_y, &mut positions);
            width = 0.0;
            rn += 1.0;
            #[cfg(feature="ldbg")]
            println!("Made row for {rn}");
            row = Vec::new();
        }
        row.push(b.clone());
        width += b.w;
    }
    // Last row
    make_row(&mut row, rn * lp.params.step_y, &mut positions);

    positions
}
