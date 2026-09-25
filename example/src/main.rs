// This is free and unencumbered software released into the public domain.
// See example/UNLICENSE.

use log::{LevelFilter, error, warn};

mod e1_dev;
mod e2_block_bad;
mod e3_block_good;
mod e4_simple;

use e1_dev::run_dev;
use e2_block_bad::run_block_bad;
use e3_block_good::run_block_good;
use e4_simple::simple_main;

fn help(arg_0: &String) {
    println!(
        "nlhgui tests and examples. Copyright (C) 2026 Nils L. Hake.
Usage:
  - {0} <ID>
    runs the example specified by ID (positive integer).
  - {0} --help
    shows this help message.",
        arg_0
    );
}

fn main() {
    // nlhgui doesn't require a specific backend for the log crate
    colog::default_builder()
        .filter_level(LevelFilter::Debug)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let x: usize;
    if args.len() == 1 {
        warn!(
            "No command line argument provided. Try {} --help. Running example 1.",
            &args[0]
        );
        x = 1;
    } else if args.len() == 2 {
        let arg = &args[1];
        if arg == "--help" {
            return help(&args[0]);
        }
        x = match arg.parse::<usize>() {
            Ok(v) => v,
            Err(_) => {
                error!("Bad argument (failed to parse). Try {} --help.", &args[0]);
                return;
            }
        };
    } else {
        error!("Too many command line arguments. Try {} --help.", &args[0]);
        return;
    }

    match x {
        1 => run_dev(),
        2 => run_block_bad(),
        3 => run_block_good(),
        4 => simple_main(),
        wrong => {
            error!("No example {}. Try {} --help.", &args[0], wrong);
        }
    };
}
