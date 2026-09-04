// Legalizers will move LegalBlocks into legal positions,
// returning a vector of LegalPosition indicating how things
// should move.
//
// Input to legalizer is a vector of LegalBlocks -- with the tag
// being used to refer back to the parent data structure (a subset
// of cells from a BookshelfCircuit, for example).
//
// pub mod hcwt_legal;
pub mod hcwt_dp;
pub mod rowfill;
pub mod tetris;

use std::fmt;

use scan_fmt::scan_fmt;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Write;
use std::path::Path;

use bookshelf_r::bookshelf::BookshelfCircuit;
use pstools;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum LegalKind {
    Tetris,
    HCwT,
}

#[derive(Copy, Clone)]
pub struct LegalPosition {
    pub block_tag: usize, // Refers to the index of a LegalBlock
    pub x: f32,           // Legalized position XY, lower left corner
    pub y: f32,
    pub h: f32,
    pub w: f32,
    pub original_x: f32,
    pub original_y: f32,
}

pub fn bounds(blocks: &Vec<LegalPosition>) -> pstools::bbox::BBox {
    let mut bb = pstools::bbox::BBox::new();
    for b in blocks {
        bb.addpoint(b.x, b.y);
        bb.addpoint(b.x + b.w, b.y + b.h);
    }

    bb
}

// Convert
#[derive(Copy, Clone)]
pub struct LegalBlock {
    pub tag: usize, // Refers to a parent data structure (Bookshelf cell for example)
    pub x: f32,     // Preferred X and Y location, lower left corner
    pub y: f32,
    pub h: f32, // Height and width of the block
    pub w: f32,
}
use std::cmp::Ordering;

impl Ord for LegalBlock {
    fn cmp(&self, &other: &Self) -> Ordering {
        self.tag.cmp(&other.tag)
    }
}

impl PartialOrd for LegalBlock {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for LegalBlock {
    fn eq(&self, other: &LegalBlock) -> bool {
        self.tag == other.tag
    }
}

impl Eq for LegalBlock {}

#[derive(Copy, Clone)]
pub struct LegalParams {
    pub grid_x: usize,
    pub grid_y: usize,
    pub origin_x: f32,
    pub origin_y: f32,
    pub step_x: f32,
    pub step_y: f32,
    pub alpha_right: f32,
    pub alpha_left: f32,
}

#[derive(Clone)]
pub struct LegalProblem {
    pub blocks: Vec<LegalBlock>,
    pub params: LegalParams,
}

pub fn load(filename: &String) -> LegalProblem {
    let f = File::open(filename).unwrap();
    let mut reader = BufReader::with_capacity(32000, f);

    let line = getline(&mut reader).unwrap();
    let (gx, gy, ox, oy, sx, sy) =
        scan_fmt!(&line, "{} {} {} {} {} {}", usize, usize, f32, f32, f32, f32).unwrap();

    let mut lp = LegalProblem {
        blocks: Vec::new(),
        params: LegalParams {
            grid_x: gx,
            grid_y: gy,
            origin_x: ox,
            origin_y: oy,
            step_x: sx,
            step_y: sy,
            alpha_right: 2.0,
            alpha_left: 0.5,
        },
    };

    let line = getline(&mut reader).unwrap();
    let (num_blocks) = scan_fmt!(&line, "{}", usize).unwrap();
    for _i in 0..num_blocks {
        let line = getline(&mut reader).unwrap();
        let (tag, x, y, w, h) =
            scan_fmt!(&line, "{} {} {} {} {}", usize, f32, f32, f32, f32).unwrap();
        lp.blocks.push(LegalBlock {
            tag: tag,
            x: x,
            y: y,
            h: h,
            w: w,
        });
    }

    lp
}
/*
impl LegalProblem {
    pub fn postscript(&self, filename: &String, legalization: Vec<LegalPosition>) {
        // let mut pst = pstools_r::pstools_r::PS
        let mut pst = pstools::PSTool::new();

        let ox = self.params.origin_x;
        let oy = self.params.origin_y;
        let urx = ox + self.params.step_x * self.params.grid_x as f32;
        let ury = oy + self.params.step_y * self.params.grid_y as f32;

        pst.add_box(ox, oy, urx, ury);

        pst.set_color(0.5, 0.5, 1.0, 1.0);
        for block in &self.blocks {
            pst.add_box(block.x, block.y, block.x + block.w, block.y + block.h);
        }

        pst.generate(filename.clone());
    }
}
*/

impl LegalProblem {
    pub fn save(&self, filepath: &String) {
        let mut f;

        // if the file path is empty, just print to standard out
        f = Box::new(File::create(filepath).unwrap()) as Box<dyn Write>;
        writeln!(
            &mut f,
            "{} {} {} {} {} {}",
            self.params.grid_x,
            self.params.grid_y,
            self.params.origin_x,
            self.params.origin_y,
            self.params.step_x,
            self.params.step_y
        )
        .unwrap();
        writeln!(&mut f, "{}", self.blocks.len()).unwrap();
        for b in &self.blocks {
            writeln!(&mut f, "{} {} {} {} {}", b.tag, b.x, b.y, b.w, b.h).unwrap();
        }
    }
    pub fn postscript(&self, filename: &String, legalization: &Vec<LegalPosition>) {
        let mut pst = pstools::PSTool::new();

        // Draw the border
        let ox = self.params.origin_x;
        let oy = self.params.origin_y;
        let urx = ox + self.params.step_x * self.params.grid_x as f32;
        let ury = oy + self.params.step_y * self.params.grid_y as f32;
        pst.add_box(ox, oy, urx, ury);

        // Draw displacement lines in red first (underneath the blocks)
        pst.set_color(1.0, 0.0, 0.0, 1.0);
        let mut displace = 0.0;
        let mut maxdisplace = 0.0;
        let mut maxdisplace_block = None;

        for block in legalization {
            // if let Some(block) = self.blocks.iter().find(|b| b.tag == pos.block) {
            // let block = &self.blocks[pos.block_tag];
            {
                // Draw line from original center to legalized center
                let orig_center_x = block.original_x + block.w / 2.0;
                let orig_center_y = block.original_y + block.h / 2.0;
                let legal_center_x = block.x + block.w / 2.0;
                let legal_center_y = block.y + block.h / 2.0;
                let d =
                    (orig_center_x - legal_center_x).abs() + (orig_center_y - legal_center_y).abs();
                displace += d;
                if d > maxdisplace {
                    maxdisplace = d;
                    maxdisplace_block = Some(block);
                    // println!("Block tag {} displace {}", block.block_tag, maxdisplace);
                }
                pst.add_line(orig_center_x, orig_center_y, legal_center_x, legal_center_y);
            }
        }

        pst.set_font(4.0, "Courier".to_string());
        // Use legalized coordinates instead of original coordinates
        pst.set_color(0.5, 0.5, 1.0, 1.0);
        for block in legalization {
            // if let Some(block) = self.blocks.iter().find(|b| b.tag == pos.block) {
            {
                // let block = &self.blocks[pos.block_tag];
                // Use the legalized coordinates (pos.x, pos.y)
                pst.add_box(block.x, block.y, block.x + block.w, block.y + block.h);
                pst.add_text(
                    block.x + block.w / 2.0,
                    block.y + block.h / 2.0,
                    format!("{}", block.block_tag),
                );
            }
        }

        pst.set_color(0.0, 0.0, 0.0, 1.0);
        pst.set_font(20.0, "Courier".to_string());
        pst.add_text(
            ox + 20.0,
            ury - 20.0,
            format!("Displace: {:10.1}", displace),
        );
        pst.add_text(
            ox + 20.0,
            ury - 50.0,
            format!("Max displace: {:6.1}", maxdisplace),
        );
        pst.add_text(
            ox + 20.0,
            ury - 80.0,
            format!("Avg displace: {:6.1}", displace / self.blocks.len() as f32),
        );

        if maxdisplace_block.is_some() {
            let block = maxdisplace_block.unwrap();
            // pst.set_fill(true);
            // pst.set_color(0.5, 0.1, 0.1, 1.0);
            let border = self.params.step_y / 2.0;
            pst.add_box(
                block.x - border,
                block.y - border,
                block.x + block.w + border,
                block.y + block.h + border,
            );
            let border = self.params.step_y / 1.5;
            pst.add_box(
                block.x - border,
                block.y - border,
                block.x + block.w + border,
                block.y + block.h + border,
            );
        }

        /*
                // Draw legalized positions in blue (on top of the lines)
                pst.set_color(0.2, 0.2, 0.8, 1.0);
                for pos in legalization {
                    if let Some(block) = self.blocks.iter().find(|b| b.tag == pos.block) {
                        pst.add_box(pos.x, pos.y, pos.x + block.w, pos.y + block.h);
                    }
                }
        */
        pst.set_border(self.params.step_y * 2.0);
        pst.generate(filename.clone());
    }

    pub fn postscript_fixed(&self, filename: &String) {
        let mut pst = pstools::PSTool::new();

        // Draw the border
        let ox = self.params.origin_x;
        let oy = self.params.origin_y;
        let urx = ox + self.params.step_x * self.params.grid_x as f32;
        let ury = oy + self.params.step_y * self.params.grid_y as f32;
        pst.add_box(ox, oy, urx, ury);

        pst.set_font(4.0, "Courier".to_string());
        // Use legalized coordinates instead of original coordinates
        pst.set_color(0.5, 0.5, 1.0, 1.0);
        for block in &self.blocks {
            // if let Some(block) = self.blocks.iter().find(|b| b.tag == pos.block) {
            {
                pst.add_box(block.x, block.y, block.x + block.w, block.y + block.h);
                pst.add_text(
                    block.x + block.w / 2.0,
                    block.y + block.h / 2.0,
                    format!("{}", block.tag),
                );
            }
        }
        pst.generate(filename.clone());
    }

    pub fn new() -> LegalProblem {
        LegalProblem {
            blocks: Vec::new(),
            params: LegalParams {
                grid_x: 0,
                grid_y: 0,
                origin_x: 0.0,
                origin_y: 0.0,
                step_x: 1.0,
                step_y: 1.0,
                alpha_left: 0.0,
                alpha_right: 0.0,
            },
        }
    }

    pub fn new_from(&self, positions: &Vec<LegalPosition>) -> LegalProblem {
        let mut new_lp = self.clone();

        new_lp.blocks = Vec::new();
        for pos in positions {
            new_lp.blocks.push(LegalBlock {
                tag: pos.block_tag,
                x: pos.x,
                y: pos.y,
                h: pos.h,
                w: pos.w,
            });
        }

        new_lp
    }

    pub fn move_blocks(&mut self, legalization: &Vec<LegalPosition>) {
        for pos in legalization {
            self.blocks[pos.block_tag].x = pos.x;
            self.blocks[pos.block_tag].y = pos.y;
        }
    }

    pub fn rotate(&mut self) {
        std::mem::swap(&mut self.params.grid_x, &mut self.params.grid_y);
        std::mem::swap(&mut self.params.origin_x, &mut self.params.origin_y);
        std::mem::swap(&mut self.params.step_x, &mut self.params.step_y);
        std::mem::swap(&mut self.params.alpha_left, &mut self.params.alpha_right);
        for b in &mut self.blocks {
            std::mem::swap(&mut b.x, &mut b.y);
            std::mem::swap(&mut b.h, &mut b.w);
        }
    }
    pub fn bounds(&self) -> pstools::bbox::BBox {
        let mut bbox = pstools::bbox::BBox::new();
        for block in &self.blocks {
            bbox.addpoint(block.x, block.y);
            bbox.addpoint(block.x + block.w, block.y + block.h);
        }
        bbox
    }
    pub fn move_to_origin(&mut self) {
        let bbox = self.bounds();
        let dx = bbox.llx - self.params.origin_x;
        let dy = bbox.lly - self.params.origin_y;
        for block in &mut self.blocks {
            block.x -= dx;
            block.y -= dy;
        }
    }
    pub fn area(&self) -> f32 {
        let mut total_area = 0.0;
        for block in &self.blocks {
            total_area += block.h * block.w;
        }
        total_area
    }
    pub fn rescale(&mut self) {
        // Determine total area
        let total_area = self.area();
        let target_width = total_area / ((self.params.grid_y as f32) * self.params.step_y);

        let bbox = self.bounds();
        let scale_x = target_width / bbox.dx();
        let target_height = self.params.grid_y as f32 * self.params.step_y;

        let scale_y = target_height / bbox.dy();
        #[cfg(feature = "ldbg")]
        {
            println!(
                "Rescale {} {} to {} {}",
                bbox.dx(),
                bbox.dy(),
                target_width,
                target_height
            );
            println!("Scale_x {} scale_y {}", scale_x, scale_y);
        }

        for block in &mut self.blocks {
            // New XY location
            block.x = block.x * scale_x;
            block.y = block.y * scale_y;
        }
    }
    pub fn mirror_x(&mut self) {
        let bbox = self.bounds();
        for block in &mut self.blocks {
            block.x = bbox.urx - (block.x + block.w);
        }
    }

    pub fn mirror_y(&mut self) {
        let bbox = self.bounds();
        for block in &mut self.blocks {
            block.y = bbox.ury - (block.y + block.h);
        }
    }

    pub fn pack_west(&mut self) {
        let leg = legalize_floorplan(self);
        self.move_blocks(&leg);
    }

    pub fn pack_east(&mut self) {
        self.mirror_x();
        let leg = legalize_floorplan(self);
        self.move_blocks(&leg);
        self.mirror_x();
    }

    pub fn pack_south(&mut self) {
        self.rotate();
        let leg = legalize_floorplan(self);
        self.move_blocks(&leg);
        self.rotate();
    }

    pub fn pack_north(&mut self) {
        self.mirror_y();
        self.pack_south();
        self.mirror_y();
    }
}

fn getline(reader: &mut BufReader<File>) -> std::io::Result<String> {
    loop {
        let mut line = String::new();
        let _len = reader.read_line(&mut line).unwrap();
        // println!("Read in {} bytes, line {}", _len, line);

        if _len == 0 {
            return std::result::Result::Err(Error::new(ErrorKind::Other, "end of file"));
        }

        if line.starts_with("#") {
            // println!("Skip comment.");
            continue;
        }

        if _len == 1 {
            continue;
        }

        return Ok(line.trim().to_string());
    }
    // Error::new(ErrorKind::Other, "Not reachable FILE IO error");
}

pub fn legalize_circuit(bc: &mut BookshelfCircuit, kind: LegalKind) {
    let mut blocks = Vec::new();

    for c in 0..bc.cells.len() {
        if !bc.cells[c].terminal {
            blocks.push(LegalBlock {
                tag: c,
                x: bc.cellpos[c].x,
                y: bc.cellpos[c].y,
                h: bc.cells[c].h,
                w: bc.cells[c].w,
            });
        }
    }

    let b = bc.rows[0].bounds;

    let width = b.urx - b.llx;
    let height = b.ury - b.lly;

    let params = LegalParams {
        grid_x: (width / bc.rows[0].site_spacing) as usize,
        grid_y: bc.rows.len(),
        origin_x: bc.rows[0].bounds.llx,
        origin_y: bc.rows[0].bounds.lly,
        step_x: bc.rows[0].site_spacing,
        step_y: height,
        alpha_right: 2.0,
        alpha_left: 0.5,
    };

    // println!("Legalize {} blocks\nIn space: {}", blocks.len(), params);

    // if kind == LegalKind::Tetris {
    //     let result = tetris::legalize(&blocks, &params);
    // }

    // if kind == LegalKind::HCwT {
    //     let result = hcwt_legal::legalize(&blocks, &params);
    // }
}

use crate::legalize::tetris::legalize_floorplan;

impl fmt::Display for LegalParams {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} sites {} rows  origin {},{}   site width: {} row height {}",
            self.grid_x, self.grid_y, self.origin_x, self.origin_y, self.step_x, self.step_y
        )
    }
}

pub struct ScalarGrid {
    rows: usize,
    cols: usize,
    data: Vec<f32>,
    llx: f32,
    lly: f32,
    urx: f32,
    ury: f32,
    step_size: f32,
}

#[derive(Debug)]
enum GridErr {
    OutOfBounds {
        x: usize,
        y: usize,
        w: usize,
        h: usize,
    },
}

/// A ScalarGrid is an implementation of a 2D array
/// with the capability of matching an arbitrary point
/// to the closest point in the grid.
impl ScalarGrid {
    pub fn new(llx: f32, lly: f32, urx: f32, ury: f32, step_size: f32) -> Self {
        let rows = ((ury - lly) / step_size).ceil() as usize;
        let cols = ((urx - llx) / step_size).ceil() as usize;
        ScalarGrid {
            rows,
            cols,
            data: vec![0.0; rows * cols],
            llx,
            lly,
            urx,
            ury,
            step_size,
        }
    }

    /*
    fn to_index(&self, row: usize, col: usize) -> usize {
        if row >= self.rows || col >= self.cols || row < 0 || col < 0 {
            panic!(
                "Array index out of bounds: (row, col, rows, cols): ({}, {}, {}, {})",
                row, col, self.rows, self.cols,
            );
        } else {
            row * self.cols + row
        }
    }
    */

    pub fn get(&self, x: usize, y: usize) -> Result<f32, GridErr> {
        if x >= self.cols || y >= self.rows {
            Err(GridErr::OutOfBounds {
                x,
                y,
                w: self.cols,
                h: self.rows,
            })
        } else {
            Ok(self.data[x * self.rows + y])
        }
    }

    pub fn set(&mut self, x: usize, y: usize, val: f32) -> Result<f32, GridErr> {
        if x >= self.cols || y >= self.rows {
            Err(GridErr::OutOfBounds {
                x,
                y,
                w: self.cols,
                h: self.rows,
            })
        } else {
            self.data[x * self.rows + y] = val;
            Ok(self.data[x * self.rows + y])
        }
    }

    pub fn add(&mut self, x: usize, y: usize, val: f32) -> Result<f32, GridErr> {
        self.set(x, y, self.get(x, y)? + val)
    }

    pub fn snap_to_grid(&self, x: f32, y: f32) -> (usize, usize) {
        (
            (((x - self.llx) / self.step_size - (self.step_size / 2.0)).round() as usize)
                .min(self.cols - 1),
            (((y - self.lly) / self.step_size - (self.step_size / 2.0)) as usize)
                .min(self.rows - 1),
        )
    }

    pub fn snap_to_grid_unbounded(&self, x: f32, y: f32) -> (usize, usize) {
        (
            (((x - self.llx) / self.step_size - (self.step_size / 2.0)) as usize),
            (((y - self.lly) / self.step_size - (self.step_size / 2.0)) as usize),
        )
    }

    /// This function "integrates" the grid in place (i.e. The value
    /// of the output grid at (x, y) will be the sum of all points
    /// in the input grid in the rectangle with lower-left (0, 0)
    /// and upper-right (x, y).)  This allows for constant-time
    /// area approximation for an arbitrary region.
    /// This Wikipedia page describes it better than I can:
    /// https://en.wikipedia.org/wiki/Summed-area_table
    pub fn integrate(&mut self) {
        for x in 0..self.cols {
            for y in 0..self.rows {
                let A = if x > 0 {
                    self.get(x - 1, y).unwrap()
                } else {
                    0.0
                };
                let B = if y > 0 {
                    self.get(x, y - 1).unwrap()
                } else {
                    0.0
                };
                let C = if x > 0 && y > 0 {
                    self.get(x - 1, y - 1).unwrap()
                } else {
                    0.0
                };
                self.set(x, y, A + B - C + self.get(x, y).unwrap()).unwrap();
            }
        }
    }
}

impl fmt::Display for ScalarGrid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for y in (0..self.rows).rev() {
            write!(f, "[ ");
            for x in 0..self.cols {
                write!(f, "{:.1}, ", self.get(x, y).unwrap());
            }
            writeln!(f, "]");
        }
        Ok(())
    }
}

/*
pub struct AreaCalculatorBuilder {
    grid: ScalarGrid,
}
*/

/// This is the grid used to calculate approximate area
/// within an arbitrary region.
pub struct AreaGrid {
    grid: ScalarGrid,
}

/*
impl fmt::Display for AreaGrid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.grid)
    }
}
*/

impl AreaGrid {
    pub fn new(blocks: &[LegalBlock], step_size: f32) -> Self {
        // Get bounding box
        let mut bbox_llx = f32::INFINITY;
        let mut bbox_lly = f32::INFINITY;
        let mut bbox_urx = f32::NEG_INFINITY;
        let mut bbox_ury = f32::NEG_INFINITY;
        for block in blocks {
            bbox_llx = bbox_llx.min(block.x);
            bbox_lly = bbox_lly.min(block.y);
            bbox_urx = bbox_urx.max(block.x + block.w);
            bbox_ury = bbox_ury.max(block.y + block.h);
        }

        let mut grid = ScalarGrid::new(bbox_llx, bbox_lly, bbox_urx, bbox_ury, step_size);

        // Here, each block is encoded into the grid.
        for block in blocks {
            let (llx, lly) = grid.snap_to_grid_unbounded(block.x, block.y);
            let (urx, ury) = grid.snap_to_grid_unbounded(block.x + block.w, block.y + block.h);
            grid.add(llx, lly, 1.0);
            grid.add(llx, ury, -1.0);
            grid.add(urx, lly, -1.0);
            grid.add(urx, ury, 1.0);
        }

        // After the first integration, each block will correspond
        // to a region of ones in the grid.
        grid.integrate();

        // Integrate again to get the summed area table
        grid.integrate();

        AreaGrid { grid }
    }

    pub fn area(&self, llx: f32, lly: f32, urx: f32, ury: f32) -> f32 {
        let (llx, lly) = self.grid.snap_to_grid(llx, lly);
        let (urx, ury) = self.grid.snap_to_grid(urx, ury);

        //println!("(llx, lly, urx, ury)");
        //println!("({}, {}, {}, {})", llx, lly, urx, ury);

        let A = self.grid.get(urx, ury).unwrap();
        let B = if lly > 0 {
            self.grid.get(urx, lly - 1).unwrap()
        } else {
            0.0
        };
        let C = if llx > 0 {
            self.grid.get(llx - 1, ury).unwrap()
        } else {
            0.0
        };
        let D = if llx > 0 && lly > 0 {
            self.grid.get(llx - 1, lly - 1).unwrap()
        } else {
            0.0
        };

        //println!("A: {}", A);

        (A - B - C + D) * (self.grid.step_size * self.grid.step_size)
    }
}

/*
pub struct CutCalculatorBuilder {
    pub grid_vertical: ScalarGrid,
    pub grid_horizontal: ScalarGrid,
}

pub struct CutCalculator {
    pub grid_vertical: ScalarGrid,
    pub grid_horizontal: ScalarGrid,
}

impl CutCalculatorBuilder {
    pub fn new(
        llx: f32,
        lly: f32,
        urx: f32,
        ury: f32,
        step_size: f32
    ) -> Self {
        CutCalculatorBuilder {
            grid_vertical: ScalarGrid::new(llx, lly, urx, ury, step_size),
            grid_horizontal: ScalarGrid::new(llx, lly, urx, ury, step_size),
        }
    }

    pub fn add_block(
        &mut self,
        llx: f32,
        lly: f32,
        urx: f32,
        ury: f32,
        weight: f32,
    ) {
        let (mut llx, mut lly) = self.grid_vertical.snap_to_grid(llx, lly);
        let (mut urx, mut ury) = self.grid_vertical.snap_to_grid(urx, ury);

        // Number of rows ans columns taken up
        // by a block must both be odd (for now).
        if (urx - llx + 1) % 2 == 0 {
            urx -= 1;
        }
        if (ury - lly + 1) % 2 == 0 {
            ury -= 1;
        }

        // If the block is too small to encode
        if ury - lly + 1 < 3 || urx - llx + 1 < 3 {
            return
        }

        // Add to vertical grid
        {
            let peak = (weight / 2.0) / (ury - lly + 1) as f32;
            let delta = peak / ((urx - llx + 1) / 2 + 1) as f32;

            self.grid_vertical.add(llx, lly, delta);
            self.grid_vertical.add((llx + urx) / 2 + 1, lly, -2.0 * delta);
            self.grid_vertical.add(urx + 2, lly, delta);

            self.grid_vertical.add(llx, lly + 1, -delta);
            self.grid_vertical.add((llx + urx) / 2 + 1, lly + 1, 2.0 * delta);
            self.grid_vertical.add(urx + 2, lly + 1, -delta);

            self.grid_vertical.add(llx, ury + 1, -delta);
            self.grid_vertical.add((llx + urx) / 2 + 1, ury + 1, 2.0 * delta);
            self.grid_vertical.add(urx + 2, ury + 1, -delta);

            self.grid_vertical.add(llx, ury + 2, delta);
            self.grid_vertical.add((llx + urx) / 2 + 1, ury + 2, -2.0 * delta);
            self.grid_vertical.add(urx + 2, ury + 2, delta);
        }

        // Add to horizontal grid
        {
            let peak = (weight / 2.0) / (urx - llx + 1) as f32;
            let delta = peak / ((ury - lly + 1) / 2 + 1) as f32;

            self.grid_horizontal.add(llx, lly, delta);
            self.grid_horizontal.add(llx, (lly + ury) / 2 + 1, -2.0 * delta);
            self.grid_horizontal.add(llx, ury + 2, delta);

            self.grid_horizontal.add(llx + 1, lly, -delta);
            self.grid_horizontal.add(llx + 1, (lly + ury) / 2 + 1, 2.0 * delta);
            self.grid_horizontal.add(llx + 1, ury + 2, -delta);

            self.grid_horizontal.add(urx + 1, lly, -delta);
            self.grid_horizontal.add(urx + 1, (lly + ury) / 2 + 1, 2.0 * delta);
            self.grid_horizontal.add(urx + 1, ury + 2, -delta);

            self.grid_horizontal.add(urx + 2, lly, delta);
            self.grid_horizontal.add(urx + 2, (lly + ury) / 2 + 1, -2.0 * delta);
            self.grid_horizontal.add(urx + 2, ury + 2, delta);
        }
    }

    pub fn build(mut self) -> CutCalculator {
        self.grid_vertical.integrate();
        self.grid_vertical.integrate();
        //println!("{}", self.grid_vertical);
        self.grid_vertical.integrate();

        self.grid_horizontal.integrate();
        self.grid_horizontal.integrate();
        self.grid_horizontal.integrate();

        for x in 0..self.grid_horizontal.cols {
            for y in 0..self.grid_horizontal.rows {
                assert!(
                    self.grid_horizontal.get(x, y).unwrap() >= 0.0,
                    "grid_horizontal.get({}, {}) == {}",
                    x, y, self.grid_horizontal.get(x, y).unwrap(),
                );
            }
        }

        CutCalculator::new(
            self.grid_vertical,
            self.grid_horizontal,
        )
    }
}

impl CutCalculator {
    pub fn new(
        grid_vertical: ScalarGrid,
        grid_horizontal: ScalarGrid,
    ) -> Self {
        CutCalculator {
            grid_vertical,
            grid_horizontal,
        }
    }

    pub fn cut_vertical(
        &self,
        x: f32,
        y_bottom: f32,
        y_top: f32,
    ) -> f32 {
        // Get array indices from points
        let (i, j0) = self.grid_vertical.snap_to_grid(x, y_bottom);
        let (_, j1) = self.grid_vertical.snap_to_grid(x, y_top);

        let A = self.grid_vertical.get(i, j1).unwrap();
        let B = if i > 0 {
            self.grid_vertical.get(i - 1, j1).unwrap()
        } else {
            0.0
        };
        let C = if j0 > 0 {
            self.grid_vertical.get(i, j0 - 1).unwrap()
        } else {
            0.0
        };
        let D = if i > 0 && j0 > 0 {
            self.grid_vertical.get(i - 1, j0 - 1).unwrap()
        } else {
            0.0
        };

        (A - B) - (C - D)
    }

    pub fn cut_horizontal(
        &self,
        y: f32,
        x_left: f32,
        x_right: f32,
    ) -> f32 {
        let (i0, j) = self.grid_horizontal.snap_to_grid(x_left, y);
        let (i1, _) = self.grid_horizontal.snap_to_grid(x_right, y);

        let A = self.grid_horizontal.get(i1, j).unwrap();
        let B = if j > 0 {
            self.grid_horizontal.get(i1, j - 1).unwrap()
        } else {
            0.0
        };
        let C = if i0 > 0 {
            self.grid_horizontal.get(i0 - 1, j).unwrap()
        } else {
            0.0
        };
        let D = if j > 0 && i0 > 0 {
            self.grid_horizontal.get(i0 - 1, j - 1).unwrap()
        } else {
            0.0
        };

        (A - B) - (C - D)
    }
}
*/

#[derive(Clone)]
pub struct CutLineResult {
    pub horizontal: bool,
    pub left_blocks: Vec<usize>,
    pub right_blocks: Vec<usize>,
    pub cut_x: f32,
    pub left_area: f32,
    pub right_area: f32,
    pub penalty: Option<f32>,
}

/*
#[derive(Clone)]
pub struct CutLine {
    pub horizontal: bool,
    pub cut_coord: f32,
    pub cut_blocks: Vec<usize>,
    pub left_blocks: Vec<usize>,
    pub right_blocks: Vec<usize>,
    //pub center_score: usize,
    pub penalty: Option<f32>,
}

impl CutLine {
    pub fn new(
        blocks: &[LegalBlock],
        region: &Region,
        horizontal: bool,
        cut_coord: f32,
    ) -> Self {
        //println!("Finding cut blocks");
        let cut_blocks: Vec<usize> = if !horizontal {
            region.blocks
                .iter()
                .filter(|b| (cut_coord >= blocks[**b].x) && (cut_coord < blocks[**b].x + blocks[**b].w))
                .map(|b| *b)
                .collect()
        } else {
            region.blocks
                .iter()
                .filter(|b| (cut_coord >= blocks[**b].y) && (cut_coord < blocks[**b].y + blocks[**b].h))
                .map(|b| *b)
                .collect()
        };

        /*
        let center_score = if !horizontal {
            region.blocks
                .iter()
                .filter(|b| (cut_coord >= blocks[**b].w && (cut_coord < blocks[**b].x + blocks[**b].w)))
                .map(|b| {
                    let left_part = cut_line.cut_coord - b.x;
                    let right_part = b.x + b.w - cut_line.cot_coord;
                    let smaller_part = left_part.min(right_part);
                    let penalty = smaller_part / b.w;
                    penalty * (b.w * b.h)
                })
                .sum()
        };*/

        //println!("Computing centers");
        let blocks_with_center: Vec<(usize, f32)> = if !horizontal {
            region.blocks
                .iter()
                .map(|b| (*b, blocks[*b].x + (blocks[*b].w / 2.0)))
                .collect()
        } else {
            region.blocks
                .iter()
                .map(|b| (*b, blocks[*b].y + (blocks[*b].h / 2.0)))
                .collect()
        };

        //println!("Computing left and right blocks");
        let left_blocks = blocks_with_center
            .iter()
            .filter(|(_, center)| *center < cut_coord)
            .map(|(block, _)| *block)
            .collect();
        let right_blocks = blocks_with_center
            .iter()
            .filter(|(_, center)| *center >= cut_coord)
            .map(|(block, _)| *block)
            .collect();

        let (span, mid) = if !horizontal {(
            region.urx - region.llx,
            (region.llx + region.urx) / 2.0
        )} else {(
            region.ury - region.lly,
            (region.lly + region.ury) / 2.0
        )};

        //let center_score = (cut_coord - mid).abs() * (span / 2.0);

        CutLine {
            horizontal,
            cut_coord,
            cut_blocks,
            left_blocks,
            right_blocks,
            penalty: None,
        }
    }

    fn area(
        &self,
        blocks: &[LegalBlock],
    ) -> (f32, f32) {
        (
            self.left_blocks
                .iter()
                .map(|b| blocks[*b].w * blocks[*b].h)
                .sum(),
            self.right_blocks
                .iter()
                .map(|b| blocks[*b].w * blocks[*b].h)
                .sum(),
        )
    }
}
*/

pub enum RegionKind {
    Vertical,
    Horizontal,
    Leaf,
}

impl fmt::Display for RegionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RegionKind::Vertical => "Vertical",
                RegionKind::Horizontal => "Horizontal",
                RegionKind::Leaf => "Leaf",
            }
        )
    }
}

pub struct Region {
    pub kind: RegionKind,
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub subregions: Vec<usize>,
    pub blocks: Vec<usize>,
    pub parent: Option<usize>,
    pub cut_coord: Option<f32>,

    pub llx: f32,
    pub lly: f32,
    pub urx: f32,
    pub ury: f32,
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Kind: {}", self.kind);
        writeln!(f, "Subregions: {}", self.subregions.len());
        //writeln!(f, "Blocks: {}", self.blocks.len());
        if let Some(cut_coord) = self.cut_coord {
            writeln!(f, "Cut Coordinate: {}", cut_coord);
        }
        Ok(())
    }
}

/// Print the region tree in a tree format
pub fn print_tree(regions: &[Region]) {
    let mut root = 0;
    while let Some(parent) = regions[root].parent {
        root = parent;
    }

    let mut stack = vec![(root, 0)];

    while let Some((region, depth)) = stack.pop() {
        let display = format!("{}", regions[region]);
        let display = display
            .lines()
            .map(|line| "|---".repeat(depth) + line)
            .collect::<Vec<String>>()
            .join("\n");
        println!("{}", display);

        if let Some(right) = regions[region].right {
            stack.push((right, depth + 1));
        }

        if let Some(left) = regions[region].left {
            stack.push((left, depth + 1));
        }
    }
}

/*
pub trait CutHeuristic {
    fn run(
        &self,
        blocks: &[LegalBlock],
        region: &Region,
        horizontal: bool,
    ) -> Vec<CutLine>;
}

pub trait Penalty {
    fn run(
        blocks: &[LegalBlock],
        region: &Region,
        cut_line: &CutLine,
    ) -> f32;
}

pub struct CutAreaPenalty {
}
impl Penalty for CutAreaPenalty {
    fn run(
        blocks: &[LegalBlock],
        region: &Region,
        cut_line: &CutLine,
    ) -> f32 {
        let cut_penalty = 0.0;

        for i in 0..cut_line.cut_blocks.len() {
            let (ll_coord, span) = if !horizontal {
                (blocks[*j].x, blocks[*j].w)
            } else {
                (blocks[*j].y, blocks[*j].h)
            };
            let left_part = cut_line.cut_coord - ll_coord;
            let right_part = ll_coord + span - cut_line.cut_coord;
            let smaller_part = left_part.min(right_part);
            let penalty = smaller_part / span;
            cut_penalty += penalty * (blocks[*j].w * blocks[*j].h);
        }

        cut_penalty
    }
}

pub struct AreaImbalancePenalty {
}
impl Penalty for AreaImbalancePenalty {
    fn run(
        &self,
        blocks: &[LegalBlock],
        region: &Region,
        cut_line: &CutLine,
    ) -> f32 {
        let left_area = cut_line.left_blocks
            .iter()
            .map(|b| blocks[*b].w * blocks[*b].h)
            .sum();

        let right_area = cut_line.right_blocks
            .iter()
            .map(|b| blocks[*b].w * blocks[*b].h)
            .sum();

        (left_area - right_area).abs() / (left_area + right_area)
    }
}

pub trait FilterHeuristic {
    fn run(
        &self,
        blocks: &[LegalBlock],
        region: &Region,
        cut_line: &CutLine,
    ) -> bool;
}

/// Cut through the center of each block
pub struct CenterCutHeuristic {
}
impl CutHeuristic for CenterCutHeuristic {
    fn run(
        &self,
        blocks: &[LegalBlock],
        region: &Region,
        horizontal: bool,
    ) -> Vec<CutLine> {
        let cut_lines = if !horizontal {
            region.blocks
                .iter()
                .map(|b| blocks[*b].x + (blocks[*b].w / 2.0))
        } else {
            region.blocks
                .iter()
                .map(|b| blocks[*b].y + (blocks[*b].h / 2.0))
        }

        cut_lines
            .map(|cut_coord| CutLine::new(blocks, region, horiaontal, cut_coord))
            .collect();
    }
}

pub struct AreaRatioHeuristic {
    min_ratio: f32,
    max_ratio: f32,
}
impl FilterHeuristic for AreaRatioHeuristic {
    fn run(
        &self,
        blocks: &[LegalBlock],
        region: &Region,
        cut_line: &CutLine,
    ) -> bool {
        let left_area = cut_line.left_blocks
            .iter()
            .map(|b| blocks[*b].w * blocks[*b].h)
            .sum();

        let right_area = cut_line.right_blocks
            .iter()
            .map(|b| blocks[*b].w * blocks[*b].h)
            .sum();

        let min_area = min_ratio * (left_area + right_area);
        let max_area = max_ratio * (left_area + right_area);

        left_area >= min_area && left_area <= max_area
    }
}

pub struct BandHeuristic {
    band: f32,
}
impl FilterHeuristic for BandHeuristic {
    fn run(
        &self,
        blocks: &[LegalBlock],
        region: &Region,
        cut_line: &CutLine,
    ) -> bool {
        let (center, span) = if !cut_line.horizontal {(
            (region.llx + region.urx) / 2.0,
            region.urx - region.llx,
        )} else {(
            (region.lly + region.ury) / 2.0,
            region.ury - region.lly,
        )};

        (cut_line.cut_coord - center).abs() / (span / 2.0) <= band
    }
}
*/

pub struct Directions {
    pub vertical: bool,
    pub horizontal: bool,
}

/*
/// Heuristics to determine whether to cut vertically or horizontaly
pub trait DirectionHeuristic {
    fn run(
        &self,
        blocks: &[LegalBlock],
        region: &Region,
    ) -> Directions;
}

/*
pub fn find_optimal_cut(
    blocks: &[LegalBlock],
    //block_indices: &[usize],
    region: &Region,
    direction: Direction,
    cut_heuristic: &impl CutHeuristic,
    //band: f32,
    //min_ratio: f32,
    //max_ratio: f32,
) -> Option<CutLineResult> {
    //let heuristic = PerimeterHeuristic;
    //let direction = heuristic.run(blocks, &region.blocks);

    cut_heuristics.run(
        blocks,
        region,
        direction,
    );

    match direction {
        Direction::Vertical => {
            cut_heuristic.run(
                blocks,
                region,
                di
            )
        },
        Direction::Horizontal => {
            find(
                blocks,
                //block_indices,
                region,
                band,
                min_ratio,
                max_ratio,
            )
        }
    }
    /*
    if let Some(cut) = find_optimal_cut_vertical(
        blocks,
        block_indices,
        min_ratio,
        max_ratio,
    ) {
        Some(cut)
    } else if let Some(cut) = find_optimal_cut_horizontal(
        blocks,
        block_indices,
        min_ratio,
        max_ratio,
    ) {
        Some(cut)
    } else {
        None
    }
    */
}
*/
*/

/*
pub fn perimeter_heuristic(
    blocks: &[LegalBlock],
    region: &Region,
) -> Directions {
    let sum: f32 = region.blocks
        .iter()
        .map(|b| blocks[*b].w - blocks[*b].h)
        .sum();
    if sum > 0.0 {
        Directions {
            vertical: true,
            horizontal: false,
        }
    } else {
        Directions {
            vertical: false,
            horizontal: true,
        }
    }
}
*/

/// Any direction works
pub fn no_direction_heuristic(blocks: &[LegalBlock], region: &Region) -> Directions {
    Directions {
        vertical: true,
        horizontal: true,
    }
}

/*
pub fn center_cut_heuristic(
    blocks: &[LegalBlock],
    region: &Region,
    horizontal: bool,
) -> Vec<f32> {
    println!("Finding cut lines");

    let cut_lines: Vec<f32> = if !horizontal {
        region.blocks
            .iter()
            .map(|b| blocks[*b].x + (blocks[*b].w / 2.0))
            .collect()
    } else {
        region.blocks
            .iter()
            .map(|b| blocks[*b].y + (blocks[*b].h / 2.0))
            .collect()
    };

    println!("Initializing cut lines");
    cut_lines
        .iter()
        .map(|cut_coord| CutLine::new(blocks, region, horizontal, *cut_coord))
        .collect()
}
*/

/*
pub fn area_imbalance_penalty(
    blocks: &[LegalBlock],
    region: &Region,
    cut_line: &CutLine,
) -> f32 {
    let left_area: f32 = cut_line.left_blocks
        .iter()
        .map(|b| blocks[*b].w * blocks[*b].h)
        .sum();

    let right_area: f32 = cut_line.right_blocks
        .iter()
        .map(|b| blocks[*b].w * blocks[*b].h)
        .sum();

    (left_area - right_area).abs() / (left_area + right_area)
}
*/
/*

pub fn cut_penalty(
    blocks: &[LegalBlock],
    region: &Region,
    cut_line: &CutLine
) -> f32 {
    let mut cut_penalty = 0.0;

    for i in 0..cut_line.cut_blocks.len() {
        let (ll_coord, span) = if !cut_line.horizontal {
            (blocks[i].x, blocks[i].w)
        } else {
            (blocks[i].y, blocks[i].h)
        };

        let left_part = cut_line.cut_coord - ll_coord;
        let right_part = ll_coord + span - cut_line.cut_coord;
        let smaller_part = left_part.min(right_part);
        let penalty = smaller_part / span;
        cut_penalty += penalty + (blocks[i].w * blocks[i].h);
    }

    cut_penalty
}
*/

/// Filter out all cuts close to the edge of the region
pub fn band_heuristic(
    band: f32, // A number between 0.0 and 1.0 (proportion of the region to accept)
) -> impl Fn(&[LegalBlock], &Region, &CutLine) -> bool {
    move |blocks, region, cut_line| {
        let (center, span) = if !cut_line.horizontal {
            ((region.llx + region.urx) / 2.0, region.urx - region.llx)
        } else {
            ((region.lly + region.ury) / 2.0, region.ury - region.lly)
        };

        (cut_line.coord - center).abs() / (span / 2.0) <= band
    }
}

/// Select the cut with the lowest penalty
pub fn min_penalty_heuristic(
    blocks: &[LegalBlock],
    region: &Region,
    cut_lines: (&[CutLine], &[CutLine]),
) -> Option<CutLine> {
    let error_msg = "Can't ger maximum penalty if penalties were not computed.";
    cut_lines
        .0
        .iter()
        .chain(cut_lines.1.iter())
        .min_by(|a, b| {
            a.penalty
                .expect(error_msg)
                .total_cmp(&b.penalty.expect(error_msg))
        })
        .cloned()
}

/*
pub fn area_ratio_heuristic(
    min_ratio: f32,
    max_ratio: f32,
) -> impl Fn(
    &[LegalBlock],
    &Region,
    &CutLine,
) -> bool {
    move |blocks, region, cut_line| {
        let left_area: f32 = cut_line.left_blocks.iter().map(|b| blocks[*b].w * blocks[*b].h).sum();
        let right_area: f32 = cut_line.right_blocks.iter().map(|b| blocks[*b].w * blocks[*b].h).sum();
        let total_area = left_area + right_area;

        left_area >= total_area * min_ratio && left_area < total_area * max_ratio
    }
}
*/

#[derive(Clone, Debug)]
pub struct CutLine {
    pub coord: f32,
    pub horizontal: bool,
    pub penalty: Option<f32>,
}

pub trait CutAndPenalize {
    fn run(
        &mut self,
        blocks: &[LegalBlock],
        region: &Region,
        directions: &Directions,
    ) -> (Vec<CutLine>, Vec<CutLine>);
}

/*
type CutHeuristic = &impl Fn(
    &[LegalBlock],
    &Region,
    &Directions,
) -> (Vec<CutLine>, Vec<CutLine>);

type PenaltyHeuristic = &impl FnMut(
    &[LegalBlock],
    &Region,
    (&mut [CutLine], &mut [CutLine]),
) -> ();
*/

pub struct CutAndPenalizeCustom<C, P>
where
    C: Fn(&[LegalBlock], &Region, &Directions) -> (Vec<CutLine>, Vec<CutLine>),
    P: FnMut(&[LegalBlock], &Region, (&mut [CutLine], &mut [CutLine])) -> (),
{
    cut_heuristic: C,
    penalty_heuristic: Option<P>,
}

impl<C, P> CutAndPenalizeCustom<C, P>
where
    C: Fn(&[LegalBlock], &Region, &Directions) -> (Vec<CutLine>, Vec<CutLine>),
    P: FnMut(&[LegalBlock], &Region, (&mut [CutLine], &mut [CutLine])) -> (),
{
    pub fn new(cut_heuristic: C, mut penalty_heuristic: Option<P>) -> Self {
        Self {
            cut_heuristic,
            penalty_heuristic,
        }
    }
}

impl<C, P> CutAndPenalize for CutAndPenalizeCustom<C, P>
where
    C: Fn(&[LegalBlock], &Region, &Directions) -> (Vec<CutLine>, Vec<CutLine>),
    P: FnMut(&[LegalBlock], &Region, (&mut [CutLine], &mut [CutLine])) -> (),
{
    fn run(
        &mut self,
        blocks: &[LegalBlock],
        region: &Region,
        directions: &Directions,
    ) -> (Vec<CutLine>, Vec<CutLine>) {
        let (mut cuts_v, mut cuts_h) = (self.cut_heuristic)(blocks, region, directions);

        if let Some(penalty_heuristic) = &mut self.penalty_heuristic {
            println!("Extracted penalty.");
            penalty_heuristic(blocks, region, (&mut cuts_v, &mut cuts_h));
        }

        (cuts_v, cuts_h)
    }
}

pub struct CutAndPenalizeStreamlined<P>
where
    P: Fn(
        f32,  // Area of left side
        f32,  // Area of right side
        f32,  // Cut penalty
        f32,  // Aspect ratio
        bool, // Horizontal
    ) -> f32,
    /*
    D: Fn(
        &CutLine, // Best vertical cut
        &CutLine, // Best horizontal cut
        f32, // Aspect ratio
    ) -> bool, // Horizontal
    */
{
    num_big_blocks: usize,
    penalty: P,
    //direction_heuristic: D,
}

impl<P> CutAndPenalizeStreamlined<P>
where
    P: Fn(
        f32,  // Area of left side
        f32,  // Area of right side
        f32,  // Cut penalty
        f32,  // Aspect ratio
        bool, // Horizontal
    ) -> f32,
    /*
    D: Fn(
        &CutLine, // Best vertical cut
        &CutLine, // Best horizontal cut
        f32, // Aspect ratio
    ) -> bool, // Horizontal
    */
{
    pub fn new(
        num_big_blocks: usize,
        penalty: P,
        //direction_heuristic: D,
    ) -> Self {
        Self {
            num_big_blocks,
            penalty,
            //direction_heuristic,
        }
    }
}

/*
#[derive(Debug)]
struct FloatAndIndex(f32, usize);

impl PartialOrd for FloatAndIndex {
    fn partial_cmp(&self, other: Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FloatAndIndex {
    fn cmp(&self, other: Self) -> Ordering {
        self.0.total_cmp(other.0);
    }
}

// Find the k largest numbers in O(n*log(k)) time
// https://www.geeksforgeeks.org/dsa/k-largestor-smallest-elements-in-an-array/
fn max_k_values<T>(values: &[T], k: usize)  -> Vec<T>
where
    T: Ord + PartialOrd
{
    use std::collections::BinaryHeap;
    use std::cmp::Reverse;

    assert!(k > 0 && k <= values.len(), "Cannot select top {} elements from {} items.", k, values.len());

    let heap = BinaryHeap::new();

    for i in 0..k {
        heap.push(Reverse(values[i]));
    }

    for i in k..values.len() {
        let Some(Reverse(top)) = heap.peek() else {
            panic!("Heap is empty after inserting k items and then only adding and removing items in pairs.");
        };
        if values[i] > top {
            heap.pop();
            heap.push(Reverse(values[i]));
        }
    }

    let result = Vec::with_capacity(k);

    for _ in 0..k {
        result.push(heap.pop());
    }

    result
}
*/

impl<P> CutAndPenalize for CutAndPenalizeStreamlined<P>
where
    P: Fn(
        f32,  // Area of left side
        f32,  // Area of right side
        f32,  // Cut penalty
        f32,  // Aspect ratio
        bool, // Horizontal
    ) -> f32,
    /*
    D: Fn(
        &CutLine, // Best vertical cut
        &CutLine, // Best horizontal cut
        f32, // Aspect ratio
    ) -> bool, // Horizontal
    */
{
    fn run(
        &mut self,
        blocks: &[LegalBlock],
        region: &Region,
        directions: &Directions,
    ) -> (Vec<CutLine>, Vec<CutLine>) {
        let mut cuts_v = Vec::new();
        let mut cuts_h = Vec::new();

        // Find biggest blocks
        let mut biggest = region.blocks.clone();
        biggest.sort_by(|a, b| {
            (blocks[*b].w * blocks[*b].h).total_cmp(&(blocks[*a].w * blocks[*a].h))
        });
        biggest.truncate(self.num_big_blocks);

        // Sort blocks from left to right
        let mut sorted = region.blocks.clone();
        sorted.sort_by(|a, b| {
            (blocks[*a].x + (blocks[*a].w / 2.0)).total_cmp(&(blocks[*b].x + (blocks[*b].w / 2.0)))
        });
        // No need to de-duplicate -- we could miss a block!
        // sorted.dedup_by_key(
        //     |b| blocks[*b].x + (blocks[*b].w / 2.0)
        // );

        // Make the walk from left to right
        if sorted.len() == 2 {
            cuts_v.push(CutLine {
                coord: ((blocks[sorted[0]].x + (blocks[sorted[0]].w / 2.0))
                    + (blocks[sorted[1]].x + (blocks[sorted[1]].w / 2.0)))
                    / 2.0,
                horizontal: false,
                penalty: Some((self.penalty)(
                    blocks[sorted[0]].w * blocks[sorted[0]].h,
                    blocks[sorted[1]].w * blocks[sorted[1]].h,
                    0.0,
                    (region.urx - region.llx) / (region.ury - region.lly),
                    false,
                )),
            });
        } else if sorted.len() > 2 {
            let mut left_area = blocks[sorted[0]].w * blocks[sorted[0]].h;
            let mut right_area = sorted
                .iter()
                .map(|b| blocks[*b].w * blocks[*b].h)
                .sum::<f32>()
                - left_area;
            for i in 1..(sorted.len() - 1) {
                let x = blocks[sorted[i]].x + (blocks[sorted[i]].w / 2.0);
                let area = blocks[sorted[i]].w * blocks[sorted[i]].h;
                left_area += area;
                right_area -= area;

                let cut_penalty = biggest
                    .iter()
                    .map(|b|
                        // Compute absolute distance from cut line to the
                        // center of the cut block, divide by half the width
                        // of the cut block, subtract that from one (to make
                        // sure the penalty is highest at the midline), then
                        // clamp at zero (for the case where the line does not
                        // intersect with the block at all)
                        (1.0 - ((x - (blocks[*b].x + (blocks[*b].w / 2.0))).abs() / (blocks[*b].w / 2.0))).max(0.0)
                        * (blocks[*b].w * blocks[*b].h)
                    )
                    .sum();

                cuts_v.push(CutLine {
                    coord: x,
                    horizontal: false,
                    penalty: Some((self.penalty)(
                        left_area,
                        right_area,
                        cut_penalty,
                        (region.urx - region.llx) / (region.ury - region.lly),
                        false,
                    )),
                });
            }
        }

        // Sort blocks from bottom to top
        let mut sorted = region.blocks.clone();
        sorted.sort_by(|a, b| {
            (blocks[*a].y + (blocks[*a].h / 2.0)).total_cmp(&(blocks[*b].y + (blocks[*b].h / 2.0)))
        });
        // The list should contain all of the blocks -- and we move one block over at each
        // step along the way.  No need to dedup if two blocks have the same X or Y location,
        // and this could result in a block going "missing"
        // sorted.dedup_by_key(
        //     |b| blocks[*b].y + (blocks[*b].h / 2.0)
        // );

        // Make the walk from bottom to top
        if sorted.len() == 2 {
            cuts_v.push(CutLine {
                coord: ((blocks[sorted[0]].y + (blocks[sorted[0]].h / 2.0))
                    + (blocks[sorted[1]].y + (blocks[sorted[1]].h / 2.0)))
                    / 2.0,
                horizontal: true,
                penalty: Some((self.penalty)(
                    blocks[sorted[0]].w * blocks[sorted[0]].h,
                    blocks[sorted[1]].w * blocks[sorted[1]].h,
                    0.0,
                    (region.urx - region.llx) / (region.ury - region.lly),
                    false,
                )),
            });
        } else if sorted.len() > 2 {
            let mut bottom_area = blocks[sorted[0]].w * blocks[sorted[0]].h;
            let mut top_area = sorted
                .iter()
                .map(|b| blocks[*b].w * blocks[*b].h)
                .sum::<f32>()
                - bottom_area;
            for i in 1..(sorted.len() - 1) {
                let y = blocks[sorted[i]].y + (blocks[sorted[i]].h / 2.0);
                let area = blocks[sorted[i]].w * blocks[sorted[i]].h;
                bottom_area += area;
                top_area -= area;

                let cut_penalty = biggest
                    .iter()
                    .map(|b|
                        // Compute absolute distance from cut line to the
                        // center of the cut block, divide by half the height
                        // of the cut block, subtract that from one (to make
                        // sure the penalty is highest at the midline), then
                        // clamp at zero (for the case where the line does not
                        // intersect with the block at all)
                        (1.0 - ((y - (blocks[*b].y + (blocks[*b].h / 2.0))).abs() / (blocks[*b].h / 2.0))).max(0.0)
                        * (blocks[*b].w * blocks[*b].h)
                    )
                    .sum();

                cuts_h.push(CutLine {
                    coord: y,
                    horizontal: true,
                    penalty: Some((self.penalty)(
                        bottom_area,
                        top_area,
                        cut_penalty,
                        (region.urx - region.llx) / (region.ury - region.lly),
                        true,
                    )),
                });
            }
        }

        (cuts_v, cuts_h)
    }
}

pub fn streamlined_penalty(direction_factor: f32) -> impl Fn(f32, f32, f32, f32, bool) -> f32 {
    move |left_area, right_area, cut_penalty, aspect_ratio, horizontal| {
        let area_penalty = (right_area - left_area).abs();
        let direction_penalty = if !horizontal {
            1.0
        } else {
            aspect_ratio.powf(direction_factor)
        };
        (area_penalty + cut_penalty) * direction_penalty
    }
}

pub fn to_sharp(blocks: &[LegalBlock], regions: &[Region], comments: Vec<String>) -> String {
    let mut internal_lines = Vec::with_capacity(regions.len());
    let mut leaf_lines = Vec::with_capacity(regions.len());
    for (id, region) in regions.iter().enumerate() {
        let num_children = match region.kind {
            RegionKind::Leaf => 0,
            _ => 2,
        };

        if num_children == 2 {
            let vh = match region.kind {
                RegionKind::Vertical => "V",
                RegionKind::Horizontal => "H",
                RegionKind::Leaf => panic!("Node is simultaneously a leaf and an internal node."),
            };

            internal_lines.push(format!(
                "{} {} 2 {} {} {} {} {} 0\n{} {}\n",
                id,
                vh,
                region.blocks.len(),
                region.llx,
                region.lly,
                region.urx,
                region.ury,
                region.left.unwrap(),
                region.right.unwrap(),
            ));
        } else {
            leaf_lines.push(format!(
                "{} H 0 {} {} {} {} {} 0\n{}",
                id,
                region.blocks.len(),
                region.llx,
                region.lly,
                region.urx,
                region.ury,
                region
                    .blocks
                    .iter()
                    .map(|b| format!(
                        "{} {} {}\n",
                        *b,
                        blocks[*b].x + (blocks[*b].w / 2.0),
                        blocks[*b].y + (blocks[*b].h / 2.0),
                    ))
                    .collect::<String>()
            ));
        }
    }

    let mut all_lines = Vec::with_capacity(
        comments.len() +
        1 + // for the total number of nodes
        internal_lines.len() +
        leaf_lines.len(),
    );

    for line in comments {
        all_lines.push(format!("# {}", line));
    }

    all_lines.push(regions.len().to_string());

    for line in internal_lines {
        all_lines.push(line);
    }

    for line in leaf_lines {
        all_lines.push(line);
    }

    all_lines.into_iter().collect()
}

// We're checking aspect ratio here -- largest
// dimension over smaller to get the penalty
fn max_ratio(a: f32, b: f32) -> f32 {
    if a > b {
        return a / b;
    }
    b / a
}

// If we split a region, we have some percentage of the area
// on the left, and some percentage on the right.  The bias
// is the area on the left, divided by the total area.
// If we split exactly evenly, that's the best spot -- but if
// the region we split is a tall vertical area, splitting down
// the middle vertically is worse than splitting somewhere
// horizontally.
// We use the max ratio to find the penalty of the split.
// If it's 1:3 or 3:1, both should wind up with a penalty
// of "3" -- so it's the larger over the smaller
fn ar_penalty(bias: f32, split_dimension: f32, alternate_dimension: f32) -> f32 {
    let a = split_dimension * bias;

    let r1 = max_ratio(a, alternate_dimension);
    let r2 = max_ratio(split_dimension - a, alternate_dimension);
    // println!("Bias {bias}  {wide}x{tall} --> {r1} {r2} {}", r1 * r2);

    r1 * r2
}

fn block_area(block: &LegalBlock) -> f32 {
    block.w * block.h
}

// find_cut scans through the blocks that belong to a
// region, looking for the best place to split it into two
// groups.  The cut will be either horizontal (with the blocks
// being ordered by the Y axis, bottom to top), or not
// horizontal (a vertical cut, blocks ordered along X axis).
//
// Passed in is the shape of the region we are splitting -- ideally,
// we avoid generating long and thin regions, either horizontally
// or vertically.
//
// Each potential cut location is considered, such that there
// is at least one element on either side of the cut.  The
// return is the minimum penalty found, the list of blocks
// on each side, and the total areas of the blocks on each
// side.
fn find_cut(
    blocks: &[LegalBlock],
    region: &Region,
    horizontal: bool,
    width: f32,
    height: f32,
) -> (f32, Vec<usize>, f32, Vec<usize>, f32) {
    let mut a_area = 0.0;
    let mut b_area = 0.0;
    for b in &region.blocks {
        b_area += blocks[*b].w * blocks[*b].h;
    }
    let total_area = b_area;

    // Blocks that we're going to split, ordered left-right or bottom-top
    let mut block_order = region.blocks.clone();
    if horizontal {
        block_order.sort_by(|a, b| {
            (blocks[*a].y + blocks[*a].h / 2.0).total_cmp(&(blocks[*b].y + blocks[*b].h / 2.0))
        });
    } else {
        block_order.sort_by(|a, b| {
            (blocks[*a].x + blocks[*a].w / 2.0).total_cmp(&(blocks[*b].x + blocks[*b].w / 2.0))
        });
    }

    // Best split point initially -- *after* block 0 in the list
    let mut best = 0;
    a_area += block_area(&blocks[block_order[0]]);
    let mut best_penalty;
    if horizontal {
        best_penalty = ar_penalty(a_area / total_area, height, width);
    } else {
        best_penalty = ar_penalty(a_area / total_area, width, height);
    }

    // We check the potential split point,
    for split in 1..(block_order.len() - 1) {
        // Add block to the left, subtract from the right
        let a = block_area(&blocks[block_order[split]]);
        a_area += a;
        // b_area -= a;

        // Compute the penalty for splitting up o split point
        let penalty;
        if horizontal {
            penalty = ar_penalty(a_area / total_area, height, width);
        } else {
            penalty = ar_penalty(a_area / total_area, width, height);
        }

        // Update the "best" if needed
        if penalty < best_penalty {
            best_penalty = penalty;
            best = split;
        }
    }

    // Now get the actual split locations, and the list of blocks on
    // each side
    a_area = 0.0;
    b_area = 0.0;
    let mut a_block_list = Vec::new();
    let mut b_block_list = Vec::new();
    for i in 0..block_order.len() {
        let a = block_area(&blocks[block_order[i]]);
        if i <= best {
            a_area += a;
            a_block_list.push(block_order[i]);
        } else {
            b_area += a;
            b_block_list.push(block_order[i]);
        }
    }

    println!(
        "SPLIT {:8.5}   {} {} blocks {} {} blocks  {}=={} total",
        best_penalty,
        a_area,
        a_block_list.len(),
        b_area,
        b_block_list.len(),
        a_area + b_area,
        total_area
    );

    (best_penalty, a_block_list, a_area, b_block_list, b_area)
}

pub fn render(pst: &mut pstools::PSTool, blocks: &[LegalBlock], regions: &Vec<Region>, i: usize, depth: usize, max_depth: usize) {
    if depth == max_depth || regions[i].left.is_none() {
        let (r, g, b) = pstools::PSTool::gen_color(i as i32);
        pst.set_color(r, g, b, 1.0);
        let reg = &regions[i];
        pst.set_line_width(4.0);
        pst.add_box(reg.llx, reg.lly, reg.urx, reg.ury);
        pst.set_line_width(1.0);
        for b in &regions[i].blocks {
            let block = &blocks[*b];
            pst.add_box(block.x, block.y, block.x + block.w, block.y + block.h);
        }
        return;
    }
    render(pst, blocks, regions, regions[i].left.unwrap(), depth + 1, max_depth);
    render(pst, blocks, regions, regions[i].right.unwrap(), depth + 1, max_depth);
}

pub fn recursive_bisection(
    blocks: &[LegalBlock],
    direction_heuristic: impl Fn(&[LegalBlock], &Region) -> Directions,
    mut cut_and_penalize: impl CutAndPenalize,
    filter_heuristics: &[&dyn Fn(&[LegalBlock], &Region, &CutLine) -> bool],
    selection_heuristic: impl Fn(&[LegalBlock], &Region, (&[CutLine], &[CutLine])) -> Option<CutLine>,
    max_depth: Option<usize>,
) -> Vec<Region> {
    //let block_indices: Vec<usize> = (0..blocks.len()).collect();
    let mut regions = Vec::new();

    // Compute bounding box
    let mut outer_llx = f32::INFINITY;
    let mut outer_lly = f32::INFINITY;
    let mut outer_urx = f32::NEG_INFINITY;
    let mut outer_ury = f32::NEG_INFINITY;
    for block in blocks {
        outer_llx = outer_llx.min(block.x);
        outer_lly = outer_lly.min(block.y);
        outer_urx = outer_urx.max(block.x + block.w);
        outer_ury = outer_ury.max(block.y + block.h);
    }

    
    // Add first region
    regions.push(Region {
        kind: RegionKind::Leaf,
        left: None,
        right: None,
        subregions: Vec::new(),
        blocks: (0..blocks.len()).collect(),
        parent: None,
        cut_coord: None,
        llx: outer_llx,
        lly: outer_lly,
        urx: outer_urx,
        ury: outer_ury,
    });

    // Initialize the stack
    let mut stack = Vec::new();
    stack.push(StackFrame {
        region_index: 0,
        depth: 0,
    });

    let mut count = 0;
    while let Some(frame) = stack.pop() {
        let region_index = frame.region_index;
        let depth = frame.depth;
        let dx = regions[region_index].urx - regions[region_index].llx;
        let dy = regions[region_index].ury - regions[region_index].lly;

        // BLOCKS IN THE REGION ARE ALREADY KNOWN
        let blocks_in_region = regions[region_index].blocks.len();
        println!("*REGION {}  {:6.1} x {:6.1}   {} blocks_in_region", region_index, dx, dy, blocks_in_region);
        println!(
            "regions[region_index].blocks.len(): {}",
            regions[region_index].blocks.len()
        );
        // If the region contains only one block
        if regions[region_index].blocks.len() <= 1 || depth >= max_depth.unwrap_or(usize::MAX) {
            println!(
                "Base case reached: (regions[region_index].blocks.len(), depth) = ({}, {})",
                regions[region_index].blocks.len(),
                depth,
            );
            continue;
        }

        println!("Region {}  {} x {}", region_index, dx, dy);
        let (hpenalty, h_a, h_area_a, h_b, h_area_b) =
            find_cut(blocks, &regions[region_index], true, dx, dy);
        let (vpenalty, v_a, v_area_a, v_b, v_area_b) =
            find_cut(blocks, &regions[region_index], false, dx, dy);

        println!("hpenalty {} vpenalty {}", hpenalty, vpenalty);

        // return Vec::new();

        let cut;
        let a_blocks;
        let b_blocks;
        let reg = &regions[region_index];
        if hpenalty < vpenalty {
            // Horizontal split is better
            let coord = reg.lly + dy * (h_area_a / (h_area_a + h_area_b));
            cut = CutLine {
                coord: coord,
                horizontal: true,
                penalty: Some(hpenalty),
            };
            a_blocks = h_a;
            b_blocks = h_b;
        } else {
            let coord = reg.llx + dx * (v_area_a / (v_area_a + v_area_b));
            // vertical split is better
            cut = CutLine {
                coord: coord,
                horizontal: false,
                penalty: Some(vpenalty),
            };
            a_blocks = v_a;
            b_blocks = v_b;
        }

        // // Compute cuts
        // let directions = direction_heuristic(blocks, &regions[region_index]);

        // let (mut cuts_v, mut cuts_h) =
        //     cut_and_penalize.run(blocks, &regions[region_index], &directions);

        // /*
        // let (mut cuts_v, mut cuts_h) = cut_heuristic(blocks, &regions[region_index], &directions);

        // // If there is a penalty heuristic, then apply it to the cuts
        // if let Some(penalty_heuristic) = &mut penalty_heuristic {
        //     penalty_heuristic(blocks, &regions[region_index], (&mut cuts_v, &mut cuts_h));
        // }
        // */
        // // Apply each filter heuristic one at a time.
        // for filter_heuristic in filter_heuristics {
        //     cuts_v = cuts_v
        //         .into_iter()
        //         .filter(|c| filter_heuristic(blocks, &regions[region_index], c))
        //         .collect();

        //     cuts_h = cuts_h
        //         .into_iter()
        //         .filter(|c| filter_heuristic(blocks, &regions[region_index], c))
        //         .collect();
        // }

        // // Make the final selection
        // let Some(cut) = selection_heuristic(blocks, &regions[region_index], (&cuts_v, &cuts_h))
        // else {
        //     continue;
        // };

        println!("cut: {cut:?}");

        // Compute the lower-left and upper-right coordinates of the new child regions
        let (a_llx, a_lly, a_urx, a_ury);
        let (b_llx, b_lly, b_urx, b_ury);        
        if cut.horizontal {
            // Horizontal cut, so Y dimensions change
            a_llx = reg.llx;
            a_lly = reg.lly;
            a_urx = reg.urx;
            a_ury = cut.coord;

            b_llx = reg.llx;
            b_lly = a_ury;
            b_urx = reg.urx;
            b_ury = reg.ury;
        } else {
            // Vertical cut, so X dimensions change
            a_llx = reg.llx;
            a_lly = reg.lly;
            a_urx = cut.coord;
            a_ury = reg.ury;

            b_llx = a_urx;
            b_lly = reg.lly;
            b_urx = reg.urx;
            b_ury = reg.ury;
        }

        // No need to actualy move the blocks -- we're just trying to find
        // cut lines, and we find the bounding box around things each time.

        // let (left_blocks, right_blocks): (Vec<usize>, Vec<usize>) = if !cut.horizontal {
        //     regions[region_index].blocks.iter().partition(|b| {
        //         let center = blocks[**b].x + (blocks[**b].w / 2.0);
        //         center < cut.coord
        //     })
        // } else {
        //     regions[region_index].blocks.iter().partition(|b| {
        //         let center = blocks[**b].y + (blocks[**b].h / 2.0);
        //         center < cut.coord
        //     })
        // };

        // Create the regions
        let a_region = Region {
            kind: RegionKind::Leaf,
            left: None,
            right: None,
            subregions: Vec::new(),
            blocks: a_blocks,
            parent: Some(region_index),
            cut_coord: None,
            llx: a_llx,
            lly: a_lly,
            urx: a_urx,
            ury: a_ury,
        };
        let b_region = Region {
            kind: RegionKind::Leaf,
            left: None,
            right: None,
            subregions: Vec::new(),
            blocks: b_blocks,
            parent: Some(region_index),
            cut_coord: None,
            llx: b_llx,
            lly: b_lly,
            urx: b_urx,
            ury: b_ury,
        };

        // Append the regions to the list, saving their indices
        let a_index = regions.len();
        let b_index = regions.len() + 1;
        regions.push(a_region);
        regions.push(b_region);

        // Update parent region
        regions[region_index].kind = if cut.horizontal {
            RegionKind::Horizontal
        } else {
            RegionKind::Vertical
        };
        regions[region_index].left = Some(a_index);
        regions[region_index].right = Some(b_index);
        regions[region_index].cut_coord = Some(cut.coord);

        // Update the new regions' ancestors
        // (Each newly created region is a subregion of each
        // of its ancestors.)
        let mut ancestor = Some(frame.region_index);
        while let Some(a) = ancestor {
            regions[a].subregions.push(a_index);
            regions[a].subregions.push(b_index);
            ancestor = regions[a].parent;
        }

        stack.push(StackFrame {
            region_index: a_index,
            depth: frame.depth + 1,
        });
        stack.push(StackFrame {
            region_index: b_index,
            depth: frame.depth + 1,
        });

        println!("Just finished iteration {count} with depth {depth}.");
        count += 1;
    }

    regions
}

struct StackFrame {
    pub region_index: usize,
    pub depth: usize,
}

pub fn draw_bisection(
    blocks: &[LegalBlock],
    regions: &[Region],
    filename: &str,
    dimensions: (f32, f32),
) {
    let mut pst = pstools::PSTool::new();

    // Compute the bounding box of the root region (all blocks)
    let mut outer = pstools::bbox::BBox::new();
    for b in blocks {
        outer.addpoint(b.x, b.y);
        outer.addpoint(b.x + b.w, b.y + b.h);
    }

    let (outer_w, outer_h) = (outer.urx - outer.llx, outer.ury - outer.lly);

    pst.set_color(0.0, 0.0, 0.0, 1.0);

    for (llx, lly, urx, ury) in regions
        .iter()
        .map(|region| (region.llx, region.lly, region.urx, region.ury))
    {
        pst.add_box(
            (llx / outer_w) * dimensions.0,
            (lly / outer_h) * dimensions.1,
            (urx / outer_w) * dimensions.0,
            (ury / outer_h) * dimensions.1,
        );
    }

    pst.generate(filename.to_string()).unwrap();
}

/// Some code I write to make sure I understand the PostScript library
pub fn draw_something() {
    let mut pst = pstools::PSTool::new();

    let bboxes = vec![
        pstools::bbox::BBox {
            valid: true,
            llx: 0.0,
            lly: 0.0,
            urx: 100.0,
            ury: 100.0,
        },
        pstools::bbox::BBox {
            valid: true,
            llx: 100.0,
            lly: 100.0,
            urx: 200.0,
            ury: 200.0,
        },
    ];

    pst.set_color(0.0, 0.0, 0.0, 1.0);

    for bbox in bboxes {
        pst.add_box(bbox.llx, bbox.lly, bbox.urx, bbox.ury);
    }

    pst.generate(String::from("something.ps")).unwrap();
}

/*
/// convert the cutting results to LegalProblem format
pub fn cut_problem(
    lp: &LegalProblem,
    max_depth: usize,
    min_ratio: f32,
    max_ratio: f32,
) -> Vec<LegalProblem> {
    let groups = recursive_bisection(&lp.blocks, max_depth, min_ratio, max_ratio);

    groups
        .into_iter()
        .map(|region| {
            let mut new_lp = lp.clone();
            new_lp.blocks = region.blocks.into_iter().map(|index| lp.blocks[index]).collect();
            new_lp
        })
        .collect()
}
*/

struct BBox {
    pub llx: f32,
    pub lly: f32,
    pub urx: f32,
    pub ury: f32,
}

impl BBox {
    pub fn new(blocks: &[LegalBlock]) -> Self {
        let mut llx = f32::INFINITY;
        let mut lly = f32::INFINITY;
        let mut urx = f32::NEG_INFINITY;
        let mut ury = f32::NEG_INFINITY;

        for block in blocks {
            if block.x < llx {
                llx = block.x;
            }
            if block.y < lly {
                lly = block.y;
            }
            if block.x + block.w > urx {
                urx = block.x + block.w;
            }
            if block.y + block.h > ury {
                ury = block.y + block.h;
            }
        }

        BBox { llx, lly, urx, ury }
    }

    pub fn intersect(&self, other: &BBox) -> BBox {
        BBox {
            llx: self.llx.max(other.llx),
            lly: self.lly.max(other.lly),
            urx: self.urx.min(other.urx),
            ury: self.ury.min(other.ury),
        }
    }
}

struct Simple2DArray {
    rows: usize,
    cols: usize,
    data: Vec<f32>,
}

impl Simple2DArray {
    pub fn new(cols: usize, rows: usize) -> Self {
        Simple2DArray {
            cols,
            rows,
            data: vec![0.0; cols * rows],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> f32 {
        assert!(x < self.cols);
        assert!(y < self.rows);

        self.data[y * self.cols + x]
    }

    pub fn set(&mut self, x: usize, y: usize, val: f32) {
        assert!(x < self.cols);
        assert!(y < self.rows);

        self.data[y * self.cols + x] = val;
    }
}

/// This grid is used to compute cut penalties.
pub struct CutGrid {
    key_points_v: Simple2DArray,
    key_points_v_sum: Simple2DArray,
    key_points_h: Simple2DArray,
    key_points_h_sum: Simple2DArray,
    vx_ticks: Vec<f32>,
    vy_ticks: Vec<f32>,
    hx_ticks: Vec<f32>,
    hy_ticks: Vec<f32>,
    pub centers_v: Vec<(usize, f32)>,
    pub centers_h: Vec<(usize, f32)>,
    pub endpoints_v: Vec<(usize, f32)>,
    pub endpoints_h: Vec<(usize, f32)>,
}

/*
impl fmt::Display for CutCalculatorNew {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "key_points_v:");
        for i in (0..self.key_points_v.rows).rev() {
            write!(f, "[ ");
            for j in 0..self.key_points_v.cols {
                write!(f, "{}, ", self.key_points_v.get(j, i));
            }
            writeln!(f, "]");
        }
        writeln!(f);

        writeln!(f, "key_points_h:");
        for i in (0..self.key_points_h.rows).rev() {
            write!(f, "[ ");
            for j in 0..self.key_points_h.cols {
                write!(f, "{}, ", self.key_points_h.get(j, i));
            }
            writeln!(f, "]");
        }
        writeln!(f);

        writeln!(f, "key_points_v_sum:");
        for i in (0..self.key_points_v.rows).rev() {
            write!(f, "[ ");
            for j in 0..self.key_points_v_sum.cols {
                write!(f, "{}, ", self.key_points_v_sum.get(j, i));
            }
            writeln!(f, "]");
        }
        writeln!(f);

        writeln!(f, "key_points_h_sum:");
        for i in (0..self.key_points_h_sum.rows).rev() {
            write!(f, "[ ");
            for j in 0..self.key_points_h_sum.cols {
                write!(f, "{}, ", self.key_points_h_sum.get(j, i));
            }
            writeln!(f, "]");
        }
        writeln!(f);

        writeln!(f, "vx_ticks:");
        write!(f, "[ ");
        for j in 0..self.vx_ticks.len() {
            write!(f, "{}, ", self.vx_ticks[j]);
        }
        writeln!(f, "]");

        writeln!(f, "vy_ticks:");
        write!(f, "[ ");
        for i in 0..self.vy_ticks.len() {
            write!(f, "{}, ", self.vy_ticks[i]);
        }
        writeln!(f, "]");

        writeln!(f, "hx_ticks:");
        write!(f, "[ ");
        for j in 0..self.hx_ticks.len() {
            write!(f, "{}, ", self.hx_ticks[j]);
        }
        writeln!(f, "]");

        writeln!(f, "hy_ticks:");
        write!(f, "[ ");
        for i in 0..self.hy_ticks.len() {
            write!(f, "{}, ", self.hy_ticks[i]);
        }
        writeln!(f, "]")
    }
}
*/

#[derive(Debug, PartialEq)]
enum BreakpointKind {
    Start,
    Center,
    End,
}

/// A breakpoint represents the start, centwe, or end of a block.
#[derive(Debug)]
struct Breakpoint {
    pub coord: f32,
    pub block: usize,
    pub delta_m: Option<f32>,
    pub kind: BreakpointKind,
}

impl CutGrid {
    pub fn new(blocks: &[LegalBlock]) -> Self {
        // Initialize list of breakpoints on the x-axis of the vertical grid
        let mut breakpoints_vx = Vec::with_capacity(blocks.len() * 3);
        for (block_index, block) in blocks.iter().enumerate() {
            let peak = (block.w * block.h) / 2.0;
            let delta_m = peak / (block.w / 2.0);
            breakpoints_vx.push(Breakpoint {
                coord: block.x,
                block: block_index,
                delta_m: Some(delta_m),
                kind: BreakpointKind::Start,
            });
            breakpoints_vx.push(Breakpoint {
                coord: block.x + (block.w / 2.0),
                block: block_index,
                delta_m: Some(-2.0 * delta_m),
                kind: BreakpointKind::Center,
            });
            breakpoints_vx.push(Breakpoint {
                coord: block.x + block.w,
                block: block_index,
                delta_m: Some(delta_m),
                kind: BreakpointKind::End,
            });
        }
        breakpoints_vx.sort_by(|bp1, bp2| bp1.coord.total_cmp(&bp2.coord));

        // Initialize list of breakpoints on the y-axis on the vertical grid
        let mut breakpoints_vy = Vec::with_capacity(blocks.len() * 2);
        for (block_index, block) in blocks.iter().enumerate() {
            breakpoints_vy.push(Breakpoint {
                coord: block.y,
                block: block_index,
                delta_m: None,
                kind: BreakpointKind::Start,
            });
            breakpoints_vy.push(Breakpoint {
                coord: block.y + block.h,
                block: block_index,
                delta_m: None,
                kind: BreakpointKind::End,
            });
        }
        breakpoints_vy.sort_by(|bp1, bp2| bp1.coord.total_cmp(&bp2.coord));

        // Initialize list of breakpoints in the x-axis of the horizontal grid
        let mut breakpoints_hx = Vec::with_capacity(blocks.len() * 2);
        for (block_index, block) in blocks.iter().enumerate() {
            breakpoints_hx.push(Breakpoint {
                coord: block.x,
                block: block_index,
                delta_m: None,
                kind: BreakpointKind::Start,
            });
            breakpoints_hx.push(Breakpoint {
                coord: block.x + block.w,
                block: block_index,
                delta_m: None,
                kind: BreakpointKind::End,
            });
        }
        breakpoints_hx.sort_by(|bp1, bp2| bp1.coord.total_cmp(&bp2.coord));

        // Initialize list of breakpoints in the y-axis of the horizontal grid
        let mut breakpoints_hy = Vec::with_capacity(blocks.len() * 3);
        for (block_index, block) in blocks.iter().enumerate() {
            let peak = (block.w * block.h) / 2.0;
            let delta_m = peak / (block.h / 2.0);

            breakpoints_hy.push(Breakpoint {
                coord: block.y,
                block: block_index,
                delta_m: Some(delta_m),
                kind: BreakpointKind::Start,
            });
            breakpoints_hy.push(Breakpoint {
                coord: block.y + (block.h / 2.0),
                block: block_index,
                delta_m: Some(-2.0 * delta_m),
                kind: BreakpointKind::Center,
            });
            breakpoints_hy.push(Breakpoint {
                coord: block.y + block.h,
                block: block_index,
                delta_m: Some(delta_m),
                kind: BreakpointKind::End,
            });
        }
        breakpoints_hy.sort_by(|bp1, bp2| bp1.coord.total_cmp(&bp2.coord));

        /*
        println!("breakpoints_hy:");
        for bp in &breakpoints_hy {
            println!("{:?}", bp);
        }
        */

        // Populate x- and y-ticks
        let vx_ticks: Vec<f32> = breakpoints_vx.iter().map(|bp| bp.coord).collect();
        let vy_ticks: Vec<f32> = breakpoints_vy.iter().map(|bp| bp.coord).collect();
        let hx_ticks: Vec<f32> = breakpoints_hx.iter().map(|bp| bp.coord).collect();
        let hy_ticks: Vec<f32> = breakpoints_hy.iter().map(|bp| bp.coord).collect();

        // Initialize key points for the vertical grid
        let mut key_points_v = Simple2DArray::new(blocks.len() * 3, blocks.len() * 2);

        // Initialize key points for the horizontal grid
        let mut key_points_h = Simple2DArray::new(blocks.len() * 2, blocks.len() * 3);

        // Initialize bitmap of blocks in the current row (vertical grid)
        let mut current_blocks = vec![false; blocks.len()];

        // Populate each row in the vertical grid with its penalty values
        for i in 0..(blocks.len() * 2 - 1) {
            // Update the current blocks bitmap
            match breakpoints_vy[i].kind {
                BreakpointKind::Start => {
                    current_blocks[breakpoints_vy[i].block] = true;
                }
                BreakpointKind::End => {
                    current_blocks[breakpoints_vy[i].block] = false;
                }
                BreakpointKind::Center => (),
            }

            for j in 0..(blocks.len() * 3) {
                // If the current breakpoint corresponds to a
                // block that is in this row
                if current_blocks[breakpoints_vx[j].block] {
                    let prop =
                        ((vy_ticks[i + 1] - vy_ticks[i]) / blocks[breakpoints_vx[j].block].h);
                    key_points_v.set(j, i, breakpoints_vx[j].delta_m.unwrap() * prop);
                }
            }

            // Convert slope changes to concrete x-coordinates
            let mut m = key_points_v.get(0, i);
            key_points_v.set(0, i, 0.0);
            for j in 1..(blocks.len() * 3) {
                let delta_x = breakpoints_vx[j].coord - breakpoints_vx[j - 1].coord;
                let delta_m = key_points_v.get(j, i);
                key_points_v.set(j, i, key_points_v.get(j - 1, i) + m * delta_x);
                m += delta_m;
            }
        }

        // Initialize bitmap of blocks in the current row (horizontal grid)
        let mut current_blocks = vec![false; blocks.len()];

        // Populate each row in the horizontal grid with its penalty values
        for j in 0..(blocks.len() * 2 - 1) {
            // Update the current blocks bitmap
            match breakpoints_hx[j].kind {
                BreakpointKind::Start => {
                    current_blocks[breakpoints_hx[j].block] = true;
                }
                BreakpointKind::End => {
                    current_blocks[breakpoints_hx[j].block] = false;
                }
                BreakpointKind::Center => (),
            }

            for i in 0..(blocks.len() * 3) {
                // If the current breakpoint corresponds to a
                // block that is in this column
                if current_blocks[breakpoints_hy[i].block] {
                    let prop =
                        ((hx_ticks[j + 1] - hx_ticks[j]) / blocks[breakpoints_hy[i].block].w);
                    key_points_h.set(j, i, breakpoints_hy[i].delta_m.unwrap() * prop);
                }
            }

            // Convert slope changes to concrete y-coordinates
            let mut m = key_points_h.get(j, 0);
            key_points_h.set(j, 0, 0.0);
            for i in 1..(blocks.len() * 3) {
                let delta_y = breakpoints_hy[i].coord - breakpoints_hy[i - 1].coord;
                let delta_m = key_points_h.get(j, i);
                key_points_h.set(j, i, key_points_h.get(j, i - 1) + m * delta_y);
                m += delta_m;
            }
        }

        // Compite prefix sums (vertical grid)
        let mut key_points_v_sum = Simple2DArray::new(blocks.len() * 3, blocks.len() * 2);
        for j in 0..(blocks.len() * 3) {
            key_points_v_sum.set(j, 0, key_points_v.get(j, 0));
        }
        for i in 1..(blocks.len() * 2) {
            for j in 0..(blocks.len() * 3) {
                key_points_v_sum.set(
                    j,
                    i,
                    key_points_v_sum.get(j, i - 1) + key_points_v.get(j, i),
                );
            }
        }

        // Compute prefix sums (horizontal grid)
        let mut key_points_h_sum = Simple2DArray::new(blocks.len() * 2, blocks.len() * 3);
        for i in 0..(blocks.len() * 3) {
            key_points_h_sum.set(0, i, key_points_h.get(0, i));
        }
        for j in 1..(blocks.len() * 2) {
            for i in 0..(blocks.len() * 3) {
                key_points_h_sum.set(
                    j,
                    i,
                    key_points_h_sum.get(j - 1, i) + key_points_h.get(j, i),
                );
            }
        }

        let centers_v: Vec<(usize, f32)> = breakpoints_vx
            .iter()
            .filter(|bp| bp.kind == BreakpointKind::Center)
            .map(|bp| (bp.block, bp.coord))
            .collect();

        let centers_h: Vec<(usize, f32)> = breakpoints_hy
            .iter()
            .filter(|bp| bp.kind == BreakpointKind::Center)
            .map(|bp| (bp.block, bp.coord))
            .collect();

        let endpoints_v: Vec<(usize, f32)> = breakpoints_vx
            .iter()
            .filter(|bp| bp.kind != BreakpointKind::Center)
            .map(|bp| (bp.block, bp.coord))
            .collect();

        let endpoints_h: Vec<(usize, f32)> = breakpoints_hy
            .iter()
            .filter(|bp| bp.kind != BreakpointKind::Center)
            .map(|bp| (bp.block, bp.coord))
            .collect();

        let ccn = CutGrid {
            key_points_v,
            key_points_v_sum,
            key_points_h,
            key_points_h_sum,
            vx_ticks,
            vy_ticks,
            hx_ticks,
            hy_ticks,
            centers_v,
            centers_h,
            endpoints_v,
            endpoints_h,
        };

        ccn
    }

    pub fn cut_vertical(&self, y_bottom: f32, y_top: f32, cuts: &[f32]) -> Vec<f32> {
        let y_bottom = y_bottom.max(self.vy_ticks[0]);
        let y_tpp = y_top.min(self.vy_ticks[self.vy_ticks.len() - 1]);

        // Use binary search to find which rows are cut
        let i_bottom = match self.vy_ticks.binary_search_by(|t| t.total_cmp(&y_bottom)) {
            Ok(i) => i,
            Err(i) => i,
        };
        let i_top = match self.vy_ticks.binary_search_by(|t| t.total_cmp(&y_top)) {
            Ok(i) => i,
            Err(i) => i - 1,
        };

        if i_top == 0 {
            return vec![0.0; cuts.len()];
        }

        // Compute sum
        let mut sum = vec![0.0; self.vx_ticks.len()];
        if i_bottom == 0 {
            for j in 0..self.vx_ticks.len() {
                sum[j] += self.key_points_v_sum.get(j, i_top);
            }
        } else {
            for j in 0..self.vx_ticks.len() {
                sum[j] += (self.key_points_v_sum.get(j, i_top)
                    - self.key_points_v_sum.get(j, i_bottom - 1));
            }
        }

        // Interpolate cuts with block bounds
        let mut cuts = cuts.to_vec();
        //cuts.sort_by(f32::total_cmp);
        assert!(cuts[0] > self.vx_ticks[0]);
        assert!(cuts[cuts.len() - 1] < self.vx_ticks[self.vx_ticks.len() - 1]);

        let mut penalties = Vec::with_capacity(cuts.len());
        let mut i = 0;
        let mut j = 0;
        while i < cuts.len() {
            if self.vx_ticks[j] < cuts[i] {
                j += 1;
            } else if self.vx_ticks[j] > cuts[i] {
                // Linear interpolation
                let x0 = self.vx_ticks[j - 1];
                let x1 = self.vx_ticks[j];
                let t = (cuts[i] - x0) / (x1 - x0);
                penalties.push(sum[j - 1] + t * (sum[j] - sum[j - 1]));
                i += 1;
            } else {
                penalties.push(sum[j]);
                i += 1;
            }
        }
        penalties
    }

    pub fn cut_horizontal(&self, x_left: f32, x_right: f32, cuts: &[f32]) -> Vec<f32> {
        let x_left = x_left.max(self.hy_ticks[0]);
        let x_right = x_right.min(self.hx_ticks[self.hx_ticks.len() - 1]);

        // Use binary search to find which columns are cut
        let j_left = match self.hx_ticks.binary_search_by(|t| t.total_cmp(&x_left)) {
            Ok(i) => i,
            Err(i) => i,
        };
        let j_right = match self.hx_ticks.binary_search_by(|t| t.total_cmp(&x_right)) {
            Ok(i) => i,
            Err(i) => i - 1,
        };

        if j_right == 0 {
            return vec![0.0; cuts.len()];
        }

        // Compute sums
        let mut sum = vec![0.0; self.hy_ticks.len()];
        if j_left == 0 {
            for i in 0..self.hy_ticks.len() {
                sum[i] += self.key_points_h_sum.get(j_right, i);
            }
        } else {
            for i in 0..self.hy_ticks.len() {
                sum[i] += (self.key_points_h_sum.get(j_right, i)
                    - self.key_points_h_sum.get(j_left - 1, i));
            }
        }

        // Interpolate cuts with block bounds
        let mut cuts = cuts.to_vec();
        //cuts.sort_by(f32::total_cmp);
        assert!(cuts[0] > self.hy_ticks[0]);
        assert!(cuts[cuts.len() - 1] < self.hy_ticks[self.hy_ticks.len() - 1]);

        let mut penalties = Vec::with_capacity(cuts.len());
        let mut i = 0;
        let mut j = 0;
        while i < cuts.len() {
            if self.hy_ticks[j] < cuts[i] {
                j += 1;
            } else if self.hy_ticks[j] > cuts[i] {
                // Linear interpolation
                let x0 = self.hy_ticks[j - 1];
                let x1 = self.hy_ticks[j];
                let t = (cuts[i] - x0) / (x1 - x0);
                penalties.push(sum[j - 1] + t * (sum[j] - sum[j - 1]));
                i += 1;
            } else {
                penalties.push(sum[j]);
                i += 1;
            }
        }
        penalties
    }

    pub fn between_center_cut_heuristic(
        &self,
    ) -> impl Fn(&[LegalBlock], &Region, &Directions) -> (Vec<CutLine>, Vec<CutLine>) {
        let mut between_v = Vec::with_capacity(self.centers_v.len() - 1);
        for j in 1..self.centers_v.len() {
            between_v.push((self.centers_v[j - 1].1 + self.centers_v[j].1) / 2.0);
        }
        let mut between_h = Vec::with_capacity(self.centers_h.len() - 1);
        for i in 1..self.centers_h.len() {
            between_h.push((self.centers_h[i - 1].1 + self.centers_h[i].1) / 2.0);
        }
        move |blocks, region, directions| {
            println!("region.llx: {}", region.llx);
            println!("region.lly: {}", region.lly);
            println!("region.urx: {}", region.urx);
            println!("region.ury: {}", region.ury);

            let mut between_v = between_v.clone();
            between_v.dedup();
            let mut between_h = between_h.clone();
            between_h.dedup();

            let mut j_left = match between_v.binary_search_by(|t| t.total_cmp(&region.llx)) {
                Ok(j) => j + 1,
                Err(j) => j,
            };
            while between_v[j_left] <= region.llx {
                j_left += 1;
            }
            let mut j_right = match between_v.binary_search_by(|t| t.total_cmp(&region.urx)) {
                Ok(j) => j - 1,
                Err(j) => j - 1,
            };
            while between_v[j_right] >= region.urx {
                j_right -= 1;
            }
            j_right = j_right.max(j_left);
            /*
            if centers_v[j_left].1 == region.llx {
                j_left += 1;
            }
            if centers_v[j_right].1 == region.urx {
                j_right -= 1;
            }
            */
            println!("j_left: {j_left}");
            println!("j_right: {j_right}");

            let mut between_cuts_v = Vec::with_capacity(j_right - j_left + 1);
            for j in j_left..=j_right {
                between_cuts_v.push(CutLine {
                    coord: between_v[j],
                    horizontal: false,
                    penalty: None,
                });
            }
            let mut i_bottom = match between_h.binary_search_by(|t| t.total_cmp(&region.lly)) {
                Ok(i) => i + 1,
                Err(i) => i,
            };
            while between_h[i_bottom] <= region.lly {
                i_bottom += 1;
            }
            let mut i_top = match between_h.binary_search_by(|t| t.total_cmp(&region.ury)) {
                Ok(i) => i - 1,
                Err(i) => i - 1,
            };
            while between_h[i_top] >= region.ury {
                i_top -= 1
            }
            i_top = i_top.max(i_bottom);
            /*
            if centers_h[i_bottom].1 == region.lly {
                i_bottom += 1;
            }
            if centers_h[i_top].1 == region.ury {
                i_top -= 1;
            }
            */
            println!("i_bottom: {i_bottom}");
            println!("i_top: {i_top}");

            let mut between_cuts_h = Vec::with_capacity(i_top - i_bottom + 1);
            for i in i_bottom..=i_top {
                between_cuts_h.push(CutLine {
                    coord: between_h[i],
                    horizontal: true,
                    penalty: None,
                });
            }

            println!(
                "between_cuts_v avg: {}, between_cuts_v min: {}, between_cuts_v max: {}",
                between_cuts_v.iter().map(|c| c.coord).sum::<f32>() / between_cuts_v.len() as f32,
                between_cuts_v
                    .iter()
                    .map(|c| c.coord)
                    .min_by(f32::total_cmp)
                    .unwrap(),
                between_cuts_v
                    .iter()
                    .map(|c| c.coord)
                    .max_by(f32::total_cmp)
                    .unwrap(),
            );
            println!(
                "between_cuts_h avg: {}, between_cuts_h min: {}, between_cuts_h max: {}",
                between_cuts_h.iter().map(|c| c.coord).sum::<f32>() / between_cuts_h.len() as f32,
                between_cuts_h
                    .iter()
                    .map(|c| c.coord)
                    .min_by(f32::total_cmp)
                    .unwrap(),
                between_cuts_h
                    .iter()
                    .map(|c| c.coord)
                    .max_by(f32::total_cmp)
                    .unwrap(),
            );

            (between_cuts_v, between_cuts_h)
        }
    }

    pub fn center_cut_heuristic(
        &self,
    ) -> impl Fn(&[LegalBlock], &Region, &Directions) -> (Vec<CutLine>, Vec<CutLine>) {
        let centers_v = self.centers_v.clone();
        let centers_h = self.centers_h.clone();
        move |blocks, region, directions| {
            println!("region.llx: {}", region.llx);
            println!("region.lly: {}", region.lly);
            println!("region.urx: {}", region.urx);
            println!("region.ury: {}", region.ury);

            let mut centers_v = centers_v.clone();
            centers_v.dedup();
            let mut centers_h = centers_h.clone();
            centers_h.dedup();

            let mut j_left = match centers_v.binary_search_by(|t| t.1.total_cmp(&region.llx)) {
                Ok(j) => j + 1,
                Err(j) => j,
            };
            while centers_v[j_left].1 <= region.llx {
                j_left += 1;
            }
            let mut j_right = match centers_v.binary_search_by(|t| t.1.total_cmp(&region.urx)) {
                Ok(j) => j - 1,
                Err(j) => j - 1,
            };
            while centers_v[j_right].1 >= region.urx {
                j_right -= 1;
            }
            j_right = j_right.max(j_left);
            /*
            if centers_v[j_left].1 == region.llx {
                j_left += 1;
            }
            if centers_v[j_right].1 == region.urx {
                j_right -= 1;
            }
            */
            println!("j_left: {j_left}");
            println!("j_right: {j_right}");

            let mut center_cuts_v = Vec::with_capacity(j_right - j_left + 1);
            for j in j_left..=j_right {
                center_cuts_v.push(CutLine {
                    coord: centers_v[j].1,
                    horizontal: false,
                    penalty: None,
                });
            }
            let mut i_bottom = match centers_h.binary_search_by(|t| t.1.total_cmp(&region.lly)) {
                Ok(i) => i + 1,
                Err(i) => i,
            };
            while centers_h[i_bottom].1 <= region.lly {
                i_bottom += 1;
            }
            let mut i_top = match centers_h.binary_search_by(|t| t.1.total_cmp(&region.ury)) {
                Ok(i) => i - 1,
                Err(i) => i - 1,
            };
            while centers_h[i_top].1 >= region.ury {
                i_top -= 1
            }
            i_top = i_top.max(i_bottom);
            /*;
            if centers_h[i_bottom].1 == region.lly {
                i_bottom += 1;
            }
            if centers_h[i_top].1 == region.ury {
                i_top -= 1;
            }
            */
            println!("i_bottom: {i_bottom}");
            println!("i_top: {i_top}");

            let mut center_cuts_h = Vec::with_capacity(i_top - i_bottom + 1);
            for i in i_bottom..=i_top {
                center_cuts_h.push(CutLine {
                    coord: centers_h[i].1,
                    horizontal: true,
                    penalty: None,
                });
            }

            println!(
                "center_cuts_v avg: {}, center_cuts_v min: {}, center_cuts_v max: {}",
                center_cuts_v.iter().map(|c| c.coord).sum::<f32>() / center_cuts_v.len() as f32,
                center_cuts_v
                    .iter()
                    .map(|c| c.coord)
                    .min_by(f32::total_cmp)
                    .unwrap(),
                center_cuts_v
                    .iter()
                    .map(|c| c.coord)
                    .max_by(f32::total_cmp)
                    .unwrap(),
            );
            println!(
                "center_cuts_h avg: {}, center_cuts_h min: {}, center_cuts_h max: {}",
                center_cuts_h.iter().map(|c| c.coord).sum::<f32>() / center_cuts_h.len() as f32,
                center_cuts_h
                    .iter()
                    .map(|c| c.coord)
                    .min_by(f32::total_cmp)
                    .unwrap(),
                center_cuts_h
                    .iter()
                    .map(|c| c.coord)
                    .max_by(f32::total_cmp)
                    .unwrap(),
            );

            (center_cuts_v, center_cuts_h)
        }
    }

    /*
    pub fn penalty_heuristic<'a>(&'a self) -> impl FnMut(
        &[LegalBlock],
        &Region,
        (&[CutLine], &[CutLine]),
    ) -> (Vec<f32>, Vec<f32>)  + 'a {
        move |blocks, region, (cuts_v, cuts_h)| {
            let penalties_v = self.cut_vertical(
                region.lly,
                region.ury,
                &cuts_v.iter().map(|c| c.coord).collect::<Vec<_>>(),
            );
            let penalties_h = self.cut_horizontal(
                region.llx,
                region.urx,
                &cuts_h.iter().map(|c| c.coord).collect::<Vec<_>>(),
            );

            (penalties_v, penalties_h)
        }
    }
    */
}

pub fn original_penalty_heuristic<'a>(
    area_grid: &'a AreaGrid,
    cut_grid: &'a CutGrid,
) -> impl 'a + FnMut(&[LegalBlock], &Region, (&mut [CutLine], &mut [CutLine])) -> () {
    |blocks, region, (cuts_v, cuts_h)| {
        let cut_penalties_v = cut_grid.cut_vertical(
            region.lly,
            region.ury,
            &cuts_v.iter().map(|c| c.coord).collect::<Vec<_>>(),
        );
        let cut_penalties_h = cut_grid.cut_horizontal(
            region.llx,
            region.urx,
            &cuts_h.iter().map(|c| c.coord).collect::<Vec<_>>(),
        );

        let area_penalties_v: Vec<f32> = cuts_v
            .iter()
            .map(|c| {
                let left_area = area_grid.area(region.llx, region.lly, c.coord, region.ury);
                let right_area = area_grid.area(c.coord, region.lly, region.urx, region.ury);
                (right_area - left_area).abs()
            })
            .collect();
        let area_penalties_h: Vec<f32> = cuts_h
            .iter()
            .map(|c| {
                let bottom_area = area_grid.area(region.llx, region.lly, region.urx, c.coord);
                let top_area = area_grid.area(region.llx, c.coord, region.urx, region.ury);
                (top_area - bottom_area).abs()
            })
            .collect();

        for i in 0..cuts_v.len() {
            cuts_v[i].penalty = Some(cut_penalties_v[i] + area_penalties_v[i]);
        }

        for i in 0..cuts_h.len() {
            cuts_h[i].penalty = Some(cut_penalties_h[i] + area_penalties_h[i]);
        }
    }
}

pub fn draw_blocks(blocks: &[LegalBlock], filename: &str, dimensions: (f32, f32)) {
    let mut llx = f32::INFINITY;
    let mut lly = f32::INFINITY;
    let mut urx = f32::NEG_INFINITY;
    let mut ury = f32::NEG_INFINITY;

    for block in blocks {
        llx = llx.min(block.x);
        lly = lly.min(block.y);
        urx = urx.max(block.x + block.w);
        ury = ury.max(block.y + block.h);
    }

    let scale_x = dimensions.0 / (urx - llx);
    let scale_y = dimensions.1 / (ury - lly);

    let mut pst = pstools::PSTool::new();

    for block in blocks {
        pst.add_box(
            (block.x - llx) * scale_x,
            (block.y - lly) * scale_y,
            (block.x + block.w - llx) * scale_x,
            (block.y + block.h - lly) * scale_y,
        );
    }

    pst.generate(filename.to_string()).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // fn test_find_optimal_cut() {
    //     let blocks = vec![
    //         LegalBlock { tag: 0, x: 0.0, y: 0.0, w: 10.0, h: 10.0 },
    //         LegalBlock { tag: 1, x: 15.0, y: 0.0, w: 10.0, h: 10.0 },
    //         LegalBlock { tag: 2, x: 30.0, y: 0.0, w: 10.0, h: 10.0 },
    //     ];

    //     let region = Region {
    //         kind: RegionKind::Leaf,
    //         left: None,
    //         right: None,
    //         subregions: Vec::new(),
    //         blocks: (0..blocks.len()).collect(),
    //         parent: None,
    //         cut_coord: None,
    //         llx: blocks.iter().map(|b| b.x).min_by(|a, b| a.total_cmp(b)).unwrap(),
    //         lly: blocks.iter().map(|b| b.y).min_by(|a, b| a.total_cmp(b)).unwrap(),
    //         urx: blocks.iter().map(|b| b.x + b.w).max_by(|a, b| a.total_cmp(b)).unwrap(),
    //         ury: blocks.iter().map(|b| b.y + b.h).max_by(|a, b| a.total_cmp(b)).unwrap(),
    //     };

    //     let mut cuts = center_cut_heuristic(&blocks, &region, false);

    //     for cut in &mut cuts {
    //         cut.penalty = Some(cut_penalty(&blocks, &region, &cut));
    //     }

    //     cuts.iter().filter(|c| area_ratio_heuristic(0.4, 0.6)(&blocks, &region, c));

    //     let result = min_penalty_heuristic(&blocks, &region, &cuts);

    //     //let result = find_optimal_cut_vertical(&blocks, 0.4, 0.6);
    //     assert!(result.is_some());

    //     let cut = result.unwrap();
    //     assert_eq!(cut.left_blocks.len(), 2);
    //     assert_eq!(cut.right_blocks.len(), 1);
    // }
    #[test]
    // fn test_cut_penalty() {
    //     // create a large block
    //     let blocks = vec![
    //         LegalBlock { tag: 0, x: 0.0, y: 0.0, w: 100.0, h: 10.0 }, // big block
    //         LegalBlock { tag: 1, x: 120.0, y: 0.0, w: 10.0, h: 10.0 }, // small
    //     ];

    //     let region = Region {
    //         kind: RegionKind::Leaf,
    //         left: None,
    //         right: None,
    //         subregions: Vec::new(),
    //         blocks: (0..blocks.len()).collect(),
    //         parent: None,
    //         cut_coord: None,
    //         llx: blocks.iter().map(|b| b.x).min_by(|a, b| a.total_cmp(b)).unwrap(),
    //         lly: blocks.iter().map(|b| b.y).min_by(|a, b| a.total_cmp(b)).unwrap(),
    //         urx: blocks.iter().map(|b| b.x + b.w).max_by(|a, b| a.total_cmp(b)).unwrap(),
    //         ury: blocks.iter().map(|b| b.y + b.h).max_by(|a, b| a.total_cmp(b)).unwrap(),
    //     };

    //     let mut cuts = center_cut_heuristic(&blocks, &region, false);

    //     for cut in &mut cuts {
    //         cut.penalty = Some(cut_penalty(&blocks, &region, &cut));
    //     }

    //     cuts.iter().filter(|c| area_ratio_heuristic(0.4, 0.6)(&blocks, &region, c));

    //     let result = min_penalty_heuristic(&blocks, &region, &cuts);

    //     //let result = find_optimal_cut_vertical(&blocks, 0.4, 0.6);
    //     assert!(result.is_some());

    //     let cut = result.unwrap();
    //     // cutting line avoid crossing large blocks, it might be cut to the right of the block
    //     assert!(cut.cut_coord > 100.0);
    // }
    #[test]
    // fn test_area_grid() {
    //     let blocks = load(&String::from("./benches/ibm01.legal.txt")).blocks;

    //     let region_bbox = BBox {
    //         llx: 200.0,
    //         lly: 300.0,
    //         urx: 600.0,
    //         ury: 500.0,
    //     };

    //     /*
    //     let blocks = vec![
    //         LegalBlock {
    //             tag: 0,
    //             x: 0.0,
    //             y: 0.0,
    //             w: 4.0,
    //             h: 4.0,
    //         },
    //         LegalBlock {
    //             tag: 1,
    //             x: 6.0,
    //             y: 6.0,
    //             w: 3.0,
    //             h: 3.0,
    //         }
    //     ];

    //     let region_bbox = BBox {
    //         llx: 0.0,
    //         lly: 0.0,
    //         urx: 12.0,
    //         ury: 12.0,
    //     };
    //     */

    //     // Compute the area the old-fashioned way
    //     // but take into account split areas
    //     let mut old_area = 0.0;
    //     for block in &blocks {
    //         let block_bbox = BBox {
    //             llx: block.x,
    //             lly: block.y,
    //             urx: block.x + block.w,
    //             ury: block.y + block.h,
    //         };

    //         // Compute the intersection between the
    //         // region and the block
    //         let intersect = region_bbox.intersect(&block_bbox);

    //         // Check if there is an intersection and
    //         // skip this block if otherwise
    //         if intersect.urx < intersect.llx || intersect.ury < intersect.lly {
    //             continue;
    //         }

    //         // Compute and update the area
    //         old_area += (intersect.urx - intersect.llx) * (intersect.ury - intersect.lly);
    //     }

    //     // Compute the area the new way
    //     let outer_bbox = BBox::new(&blocks);
    //     let mut area_calc = AreaCalculatorBuilder::new(
    //         outer_bbox.llx,
    //         outer_bbox.lly,
    //         outer_bbox.urx,
    //         outer_bbox.ury,
    //         1.0,
    //     );

    //     for block in &blocks {
    //         area_calc.add_block(
    //             block.x,
    //             block.y,
    //             block.x + block.w,
    //             block.y + block.h,
    //         );
    //     }

    //     //println!("{}", area_calc.grid);

    //     let area_calc = area_calc.build();
    //     //println!("{}", area_calc.grid);

    //     let new_area = area_calc.area(
    //         region_bbox.llx,
    //         region_bbox.lly,
    //         region_bbox.urx,
    //         region_bbox.ury,
    //     );

    //     //println!("Old area: {}", old_area);
    //     //println!("New area: {}", new_area);

    //     assert_eq!(old_area, new_area);
    // }
    #[test]
    // fn test_cut_grid() {
    //     let blocks = load(&String::from("./benches/ibm01.legal.txt")).blocks;

    //     /*
    //     let blocks = vec![
    //         LegalBlock {
    //             tag: 0,
    //             x: 0.0,
    //             y: 0.0,
    //             w: 7.0,
    //             h: 5.0,
    //         },
    //         LegalBlock {
    //             tag: 4,
    //             x: 6.0,
    //             y: 0.0,
    //             w: 7.0,
    //             h: 6.0,
    //         },
    //         LegalBlock {
    //             tag: 1,
    //             x: 2.0,
    //             y: 6.0,
    //             w: 3.0,
    //             h: 3.0,
    //         },
    //         LegalBlock {
    //             tag: 2,
    //             x: 100.0,
    //             y: 100.0,
    //             w: 50.0,
    //             h: 50.0,
    //         },
    //         LegalBlock {
    //             tag: 3,
    //             x: 0.0,
    //             y: 0.0,
    //             w: 7.0,
    //             h: 5.0,
    //         },
    //     ];
    //     */

    //     let outer_bbox = BBox::new(&blocks);
    //     /*
    //     let mut cut_calc = CutCalculatorBuilder::new(
    //         outer_bbox.llx,
    //         outer_bbox.lly,
    //         outer_bbox.urx,
    //         outer_bbox.ury,
    //         1.0,
    //     );
    //      */

    //     let cut_calc = CutCalculatorNew::new(&blocks);

    //     /*
    //     for block in &blocks {
    //         cut_calc.add_block(
    //             block.x,
    //             block.y,
    //             block.x + block.w,
    //             block.y + block.h,
    //             block.w * block.h,
    //         );
    //     }
    //     */

    //     //println!("{}", cut_calc.grid_vertical);

    //     //let cut_calc = cut_calc.build();

    //     {
    //         let cut: f32 = 600.5;
    //         let y_bottom: f32 = 50.0;
    //         let y_top: f32 = 800.0;

    //         let mut old_penalty = 0.0;

    //         for block in &blocks {
    //             if block.x > cut || block.x + block.w < cut {
    //                 continue;
    //             }

    //             let cut_size = y_top.min(block.y + block.h) - y_bottom.max(block.y);
    //             if cut_size <= 0.0 {
    //                 continue;
    //             }
    //             let cut_prop = cut_size / block.h;

    //             let center = block.x + (block.w / 2.0);
    //             let score = (block.w / 2.0) - (cut - center).abs();
    //             let score = score / block.w;

    //             old_penalty += (block.w * block.h) * score * cut_prop;
    //         }

    //         let new_penalty = cut_calc.cut_vertical(y_bottom, y_top, &[cut]);

    //         println!("Old vertical penalty: {}", old_penalty);
    //         println!("New vertical penalty: {}", new_penalty[0]);
    //     }

    //     println!();

    //     {
    //         let cut: f32 = 400.5;
    //         let x_left: f32 = 100.0;
    //         let x_right: f32 = 900.0;

    //         let mut old_penalty = 0.0;

    //         for block in &blocks {
    //             if block.y > cut || block.y + block.h < cut {
    //                 continue;
    //             }

    //             let cut_size = x_right.min(block.x + block.w) - x_left.max(block.x);
    //             if cut_size <= 0.0 {
    //                 continue;
    //             }
    //             let cut_prop = cut_size / block.w;

    //             let center = block.y + (block.h / 2.0);
    //             let score = (block.h / 2.0) - (cut - center).abs();
    //             let score = score / block.h;

    //             old_penalty += (block.w * block.h) * score * cut_prop;
    //         }

    //         let new_penalty = cut_calc.cut_horizontal(x_left, x_right, &[cut]);

    //         println!("Old horizontal penalty: {}", old_penalty);
    //         println!("New horizontal penalty: {}", new_penalty[0]);
    //     }

    //     //println!("{}", cut_calc);

    //     panic!();
    // }

    // #[test]
    // fn test_basic_cut_grid() {
    //     let mut cut_calc = CutCalculatorBuilder::new(
    //         0.0,
    //         0.0,
    //         10.0,
    //         10.0,
    //         1.0,
    //     );

    //     cut_calc.add_block(
    //         2.0,
    //         2.0,
    //         5.0,
    //         3.0,
    //         100.0,
    //     );

    //     let mut grid = cut_calc.grid_vertical;

    //     grid.integrate();
    //     grid.integrate();

    //     println!("{}", grid);

    //     panic!();
    // }
    #[test]
    fn block_count() {
        let blocks = load(&String::from("./benches/ibm01.legal.txt")).blocks;
        let outer_bbox = BBox::new(&blocks);

        println!("Outer width: {}", outer_bbox.urx - outer_bbox.llx);
        println!("Outer height: {}", outer_bbox.ury - outer_bbox.lly);
        println!("{} blocks", blocks.len());

        panic!();
    }
}
