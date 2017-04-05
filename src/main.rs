// Copyright 2016 Xavier Bestel -  All rights reserved.
//
// GPL goes here

//! Command line tool to exercise pulldown-cmark.

extern crate pulldown_cmark;
extern crate syntect;
extern crate ansi_term;
extern crate unicode_segmentation;
extern crate unicode_width;

mod ansi_renderer;

use pulldown_cmark::Parser;
use pulldown_cmark::{Options, OPTION_ENABLE_TABLES, OPTION_ENABLE_FOOTNOTES};

use std::io;
use std::env;
use std::fs::File;
use std::io::Read;

fn render_ansi(text: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(OPTION_ENABLE_TABLES);
    opts.insert(OPTION_ENABLE_FOOTNOTES);
    let mut s = String::with_capacity(text.len() * 3 / 2);
    let p = Parser::new_ext(&text, opts);
    ansi_renderer::push_ansi(&mut s, p);
    s
}

pub fn main() {
    let mut input = String::new();
    if let Some(arg1) = env::args().nth(1) {
        let mut f = File::open(arg1).expect("unable to open file");
        f.read_to_string(&mut input)
            .expect("unable to read file");
    } else {
        io::stdin()
            .read_to_string(&mut input)
            .expect("unable to read stdin");
    }
    print!("{}", render_ansi(&input));
}
