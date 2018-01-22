// Copyright 2016 Xavier Bestel -  All rights reserved.
//
// GPL goes here

//! Markdown (CommonMark) ANSI renderer.

extern crate pulldown_cmark;
extern crate syntect;
extern crate ansi_term;
extern crate term_size;
extern crate unicode_segmentation;
extern crate unicode_width;
extern crate html2runes;

extern crate catmark;

use std::io;
use std::env;
use std::fs::File;
use std::io::Read;

use catmark::OutputKind;

pub const DEFAULT_COLS: u16 = 80;

pub fn main() {
    let mut input = String::new();
    let mut width = DEFAULT_COLS;
    if let Some((w, _)) = term_size::dimensions() {
        width = w as u16;
    }
    if let Some(arg1) = env::args().nth(1) {
        let mut f = File::open(arg1).expect("unable to open file");
        f.read_to_string(&mut input).expect("unable to read file");
    } else {
        io::stdin().read_to_string(&mut input).expect(
            "unable to read stdin",
        );
    }
    let result = catmark::render_ansi(&input, width, OutputKind::Color);

    println!("{}", result);
}
