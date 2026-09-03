use argh::FromArgs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

#[derive(FromArgs)]
/// Placement legalization
struct Args {
    /// recursive bisection
    #[argh(switch, short = 'b')]
    bisection: bool,

    /// tetris legalization
    #[argh(switch, short = 't')]
    tetris: bool,

    /// hcwt legalization
    #[argh(switch, short = 'h')]
    hcwt: bool,

    /// rowfill legalization
    #[argh(switch, short = 'r')]
    rowfill: bool,

    /// row number adjustment
    #[argh(option, short = 'd')]
    delta_row: Option<i32>,

    /// file to load
    #[argh(option, short = 'f')]
    file: Option<String>,

    /// output PL file
    #[argh(option, short = 'o')]
    output: Option<String>,

    /// postScript output file
    #[argh(option, short = 'P')]
    postscript: Option<String>,
}

/*
fn grid_test() {
    use legalize::legalize::{AreaCalculatorBuilder, CutCalculatorBuilder};

    let mut acb = AreaCalculatorBuilder::new(0.0, 0.0, 1000.0, 1000.0, 1.0);
    acb.add_block(100.0, 100.0, 900.0, 900.0);
    acb.add_block(100.0, 100.0, 900.0, 900.0);

    let area_calc = acb.build();

    //println!("{}", area_calc.area(100.0, 100.0, 999.0, 999.0));

    let mut ccb = CutCalculatorBuilder::new(0.0, 0.0, 200.0, 200.0, 10.0);
    ccb.add_block(50.0, 50.0, 110.0, 100.0, 7500.0);

    let cut_calc = ccb.build();
    //println!("{}", cut_calc.grid_vertical);
    //println!("{}", cut_calc.grid_horizontal);
}
*/

fn main() {
    //use std::process::exit;

    //grid_test();

    println!("Stand-alone placement legalizer");
    let arguments: Args = argh::from_env();

    legalize::legalize::draw_something();

    let mut lp;
    if arguments.file.is_some() {
        lp = legalize::legalize::load(&arguments.file.unwrap());
    } else {
        println!("Must specify an input file");
        return;
    }

    println!(
        "Min width: {}",
        lp.blocks
            .iter()
            .map(|b| b.w)
            .min_by(|w1, w2| w1.total_cmp(w2))
            .unwrap(),
    );

    println!(
        "Min height: {}",
        lp.blocks
            .iter()
            .map(|b| b.h)
            .min_by(|h1, h2| h1.total_cmp(h2))
            .unwrap(),
    );

    println!(
        "Avg width: {}",
        lp.blocks.iter().map(|b| b.w).sum::<f32>() / lp.blocks.len() as f32,
    );

    println!(
        "Avg height: {}",
        lp.blocks.iter().map(|b| b.h).sum::<f32>() / lp.blocks.len() as f32,
    );

    println!("{} blocks in total", lp.blocks.len());

    if arguments.bisection {
        /*
        println!("Initializing area grid . . .");
        let area_grid = legalize::legalize::AreaGrid::new(&lp.blocks, 0.1);

        println!("Initializing cut grid . . .");
        let cut_grid = legalize::legalize::CutGrid::new(&lp.blocks);
        */

        let regions = legalize::legalize::recursive_bisection(
            &lp.blocks,
            legalize::legalize::no_direction_heuristic,
            legalize::legalize::CutAndPenalizeStreamlined::new(
                10,
                legalize::legalize::streamlined_penalty(1.0),
            ),
            /*
            legalize::legalize::CutAndPenalizeCustom::new(
                &cut_grid.between_center_cut_heuristic(),
                Some(&mut legalize::legalize::original_penalty_heuristic(&area_grid, &cut_grid)),
            ),
            */
            &[], //&[&legalize::legalize::band_heuristic(0.99)],
            /*&[&legalize::legalize::band_heuristic(0.9)],*/
            legalize::legalize::min_penalty_heuristic,
            None,
        );

        /*
        let counts: Vec<usize> = regions.iter().map(|v| v.blocks.len()).collect();
        println!("Regions: ");
        for c in counts {
            print!("{} ", c);
        }
        println!();
        */

        legalize::legalize::draw_bisection(
            &lp.blocks,
            &regions,
            "goch_test_streamlined.ps",
            (400.0, 600.0),
        );

        let sharp_path = Path::new("output_sharp.txt");
        let mut sharp_file = match File::create(&sharp_path) {
            Err(err) => panic!("Can't create sharp file: {}", err),
            Ok(file) => file,
        };
        let sharp = legalize::legalize::to_sharp(&lp.blocks, &regions, vec![]);
        match sharp_file.write_all(sharp.as_bytes()) {
            Err(err) => panic!("Can't write sharp file: {}", err),
            Ok(_) => (),
        }

        legalize::legalize::draw_blocks(&lp.blocks, "ibm01_blocks.ps", (400.0, 600.0));

        println!(
            "{}",
            legalize::legalize::to_sharp(
                &lp.blocks,
                &regions,
                vec!["One comment".to_string(), "Another comment".to_string()],
            )
        );

        //legalize::legalize::print_tree(&regions);
    }

    if arguments.delta_row.is_some() {
        println!("Adjust number of rows by {}", arguments.delta_row.unwrap());
        lp.params.grid_y = (lp.params.grid_y as i32 + arguments.delta_row.unwrap()) as usize;
        lp.rescale();
    }

    let mut legal = Vec::new();
    if arguments.tetris {
        legal = legalize::legalize::tetris::legalize(&lp);
    }
    /*
    if arguments.hcwt {
        legal = legalize::legalize::hcwt_legal::legalize(&lp);
    }
    */
    if arguments.rowfill {
        legal = legalize::legalize::rowfill::legalize(&lp);
    }

    if arguments.postscript.is_some() {
        lp.postscript(&arguments.postscript.unwrap(), &legal);
    }
}
