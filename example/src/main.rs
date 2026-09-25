// This is free and unencumbered software released into the public domain.
// See example/UNLICENSE.

use log::{LevelFilter, error, warn};

mod e0_quickstart;
mod e1_dev;
mod e2_block_bad;
mod e3_block_good;
mod e4_container;

use e0_quickstart::run_quickstart;
use e1_dev::run_dev;
use e2_block_bad::run_block_bad;
use e3_block_good::run_block_good;

use crate::e4_container::run_containers;

fn help(arg_0: &String) {
    println!(
        "nlhgui tests and examples. Copyright (C) 2026 Nils L. Hake.
Usage:
  - {0} <ID>
    runs the example specified by ID (non-negative integer).
  - {0} --help
    shows this help message.
Exampless:
  - 0: quickstart
  - 1: development (chaotic)
  - 2: how you should NOT write blocking Code
  - 3: blocking code",
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
            "No command line argument provided. Try {} --help. Running example 0 (quickstart).",
            &args[0]
        );
        x = 0;
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
        0 => run_quickstart(),
        1 => run_dev(),
        2 => run_block_bad(),
        3 => run_block_good(),
        4 => run_containers(),
        wrong => {
            error!("No example {}. Try {} --help.", &args[0], wrong);
        }
    };
}
