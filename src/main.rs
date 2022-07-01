// Copyright 2016 Xavier Bestel -  All rights reserved.
//
// GPL goes here

//! Markdown (CommonMark) ANSI renderer.

mod ansi_renderer;
mod dombox;
mod xy;

use pulldown_cmark::{Options, Parser};

use std::env;
use std::fs::File;
use std::io;
use std::io::Read;

pub const DEFAULT_COLS: u16 = 80;

fn render_ansi(text: &str, width: u16) {
    let p = Parser::new_ext(&text, Options::all());
    ansi_renderer::push_ansi(p, width.into());
}

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
        io::stdin()
            .read_to_string(&mut input)
            .expect("unable to read stdin");
    }
    render_ansi(&input, width);
}
