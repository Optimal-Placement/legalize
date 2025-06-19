// Legalizers will move LegalBlocks into legal positions,
// returning a vector of LegalPosition indicating how things
// should move.
//
// Input to legalizer is a vector of LegalBlocks -- with the tag
// being used to refer back to the parent data structure (a subset
// of cells from a BookshelfCircuit, for example).
//
pub mod hcwt_legal;
pub mod tetris;

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
    pub block: usize, // Refers to the index of a LegalBlock
    pub x: f32,       // Legalized position XY, lower left corner
    pub y: f32,
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

pub struct LegalProblem {
    pub blocks: Vec<LegalBlock>,
    pub params: LegalParams,
}

pub fn load(filename: &String) -> LegalProblem {
    let f = File::open(filename).unwrap();
    let mut reader = BufReader::with_capacity(32000, f);

    let line = getline(&mut reader).unwrap();
    let (gx, gy, ox, oy, sx, sy) = scan_fmt!(&line, "{} {} {} {} {} {}", usize, usize, f32, f32, f32, f32).unwrap();

    let mut lp = LegalProblem {
        blocks: Vec::new(),
        params: LegalParams { grid_x: gx, grid_y: gy, origin_x: ox, origin_y: oy, step_x: sx, step_y: sy , alpha_right: 2.0 , alpha_left :0.5},
    };
    
    let line = getline(&mut reader).unwrap();
    let (num_blocks) = scan_fmt!(&line, "{}", usize).unwrap();
    for _i in 0..num_blocks {
        let line = getline(&mut reader).unwrap();
        let (tag, x, y, w, h) = scan_fmt!(&line, "{} {} {} {} {}", usize, f32, f32, f32, f32).unwrap();
        lp.blocks.push(LegalBlock { tag: tag, x: x, y: y, h: h, w: w });
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
    pub fn postscript(&self, filename: &String, legalization: Vec<LegalPosition>) {
        let mut pst = pstools::PSTool::new();

        // Draw the border
        let ox = self.params.origin_x;
        let oy = self.params.origin_y;
        let urx = ox + self.params.step_x * self.params.grid_x as f32;
        let ury = oy + self.params.step_y * self.params.grid_y as f32;
        pst.add_box(ox, oy, urx, ury);

 
        // Draw displacement lines in red first (underneath the blocks)
        pst.set_color(1.0, 0.0, 0.0, 1.0);
        for pos in &legalization {
            if let Some(block) = self.blocks.iter().find(|b| b.tag == pos.block) {
                // Draw line from original center to legalized center
                let orig_center_x = block.x + block.w / 2.0;
                let orig_center_y = block.y + block.h / 2.0;
                let legal_center_x = pos.x + block.w / 2.0;
                let legal_center_y = pos.y + block.h / 2.0;
                pst.add_line(orig_center_x, orig_center_y, legal_center_x, legal_center_y);
            }
        }

        // Use legalized coordinates instead of original coordinates
        pst.set_color(0.5, 0.5, 1.0, 1.0);
        for pos in legalization {
            if let Some(block) = self.blocks.iter().find(|b| b.tag == pos.block) {
                // Use the legalized coordinates (pos.x, pos.y)
                pst.add_box(pos.x, pos.y, pos.x + block.w, pos.y + block.h);
                
            }
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
        pst.generate(filename.clone());
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

use std::fmt;

impl fmt::Display for LegalParams {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} sites {} rows  origin {},{}   site width: {} row height {}",
            self.grid_x, self.grid_y, self.origin_x, self.origin_y, self.step_x, self.step_y
        )
    }
}
