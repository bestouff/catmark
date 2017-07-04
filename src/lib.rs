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

mod ansi_renderer;
mod dombox;

use pulldown_cmark::Parser;
use pulldown_cmark::{Options, OPTION_ENABLE_TABLES, OPTION_ENABLE_FOOTNOTES};

pub enum OutputKind {
    Color,
    Plain,
}

pub fn render_ansi(text: &str, width: u16, kind: OutputKind) -> String {
    let mut opts = Options::empty();
    opts.insert(OPTION_ENABLE_TABLES);
    opts.insert(OPTION_ENABLE_FOOTNOTES);
    let p = Parser::new_ext(&text, opts);
    ansi_renderer::push_ansi(p, width, kind)
}
