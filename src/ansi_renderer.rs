// Copyright 2016 Xavier Bestel -  All rights reserved.
//
// GPL goes here

//! ANSI renderer for pulldown-cmark.

use crate::dombox::{split_at_in_place, BorderType, BoxKind, DomBox, DomColor, TermColor, XY};
use pulldown_cmark::{CodeBlockKind, CowStr, Event, HeadingLevel, Tag};
use syntect::easy::HighlightLines;
use syntect::highlighting;
use syntect::parsing::syntax_definition::SyntaxDefinition;
use syntect::parsing::SyntaxSet;

struct Ctx<'a, 'b, I> {
    iter: I,
    links: Option<DomBox<'a>>,
    footnotes: Option<DomBox<'a>>,
    syntaxes: &'b SyntaxSet,
    themes: &'b highlighting::ThemeSet,
    syntax: Option<&'b SyntaxDefinition>,
    pub theme: &'b str,
    highline: Option<HighlightLines<'b>>,
}

impl<'a, 'b, I: Iterator<Item = Event<'a>>> Ctx<'a, 'b, I> {
    pub fn new(iter: I, syntaxes: &'b SyntaxSet, themes: &'b highlighting::ThemeSet) -> Self {
        Ctx {
            iter: iter,
            links: None,
            footnotes: None,
            syntaxes: syntaxes,
            themes: themes,
            syntax: None,
            theme: "base16-eighties.dark",
            highline: None,
        }
    }
    fn build(&mut self, width: XY) -> DomBox<'a> {
        self.links = Some(DomBox::new_block());
        self.footnotes = Some(DomBox::new_block());
        let mut root = DomBox::new_root(width);
        self.build_dom(&mut root);
        if let Some(links) = self.links.take() {
            root.swallow(links);
        }
        if let Some(footnotes) = self.footnotes.take() {
            root.swallow(footnotes);
        }
        root
    }
    fn build_dom(&mut self, parent: &mut DomBox<'a>) {
        loop {
            match self.iter.next() {
                Some(event) => {
                    match event {
                        Event::Start(tag) => {
                            match tag {
                                Tag::Paragraph => {
                                    let child = parent.add_block();
                                    self.build_dom(child);
                                    child.size.border.bottom += 1;
                                }
                                Tag::Heading(level, _id, _classes) => {
                                    let child = parent.add_header(level as u8);
                                    child.size.border.bottom += 1;
                                    match level {
                                        HeadingLevel::H1 => {
                                            child.size.border.top += 1;
                                            child.size.border.left += 1;
                                            child.size.border.right += 1;
                                            child.style.border_type = BorderType::Thin;
                                        }
                                        HeadingLevel::H2 => {
                                            child.style.border_type = BorderType::Bold;
                                        }
                                        HeadingLevel::H3 => {
                                            child.style.border_type = BorderType::Double;
                                        }
                                        HeadingLevel::H4 => {
                                            child.style.border_type = BorderType::Thin;
                                        }
                                        HeadingLevel::H5 => {
                                            child.style.border_type = BorderType::Dash;
                                        }
                                        HeadingLevel::H6 => {}
                                    }
                                    child.style.fg = DomColor::from_dark(TermColor::Purple);
                                    self.build_dom(child);
                                }
                                Tag::Table(_) => {}
                                Tag::TableHead => {}
                                Tag::TableRow => {}
                                Tag::TableCell => {}
                                Tag::BlockQuote => {
                                    let child = parent.add_block();
                                    self.build_dom(child);
                                    child.size.border.left += 1;
                                    child.style.border_type = BorderType::Thin;
                                    child.style.fg = DomColor::from_dark(TermColor::Cyan);
                                    let newline = parent.add_block(); // XXX ugly
                                    newline.add_text(CowStr::from(""));
                                }
                                Tag::CodeBlock(info) => {
                                    {
                                        let child = parent.add_block();
                                        child.style.fg = DomColor::from_dark(TermColor::White);
                                        child.style.bg = DomColor::from_dark(TermColor::Black);
                                        if let CodeBlockKind::Fenced(syn) = info {
                                            self.syntax = self.syntaxes.find_syntax_by_token(&syn);
                                            if let Some(syn) = self.syntax {
                                                self.highline = Some(HighlightLines::new(
                                                    syn,
                                                    &self.themes.themes[self.theme],
                                                ));
                                            }
                                        }
                                        self.build_dom(child);
                                    }
                                    let newline = parent.add_block(); // XXX ugly
                                    newline.add_text(CowStr::from(""));
                                }
                                Tag::List(Some(start)) => {
                                    let child =
                                        parent.add_list(Some((start as usize).try_into().unwrap()));
                                    self.build_dom(child);
                                    child.size.border.bottom += 1;
                                }
                                Tag::List(None) => {
                                    let child = parent.add_list(None);
                                    self.build_dom(child);
                                    child.size.border.bottom += 1;
                                }
                                Tag::Item => {
                                    {
                                        let bullet = parent.add_bullet();
                                        bullet.style.fg = DomColor::from_light(TermColor::Yellow);
                                        bullet.size.border.right += 1;
                                    }
                                    let child = parent.add_block();
                                    self.build_dom(child);
                                }
                                Tag::Emphasis => {
                                    let child = parent.add_inline();
                                    child.style.italic = true;
                                    self.build_dom(child);
                                }
                                Tag::Strong => {
                                    let child = parent.add_inline();
                                    child.style.bold = true;
                                    self.build_dom(child);
                                }
                                Tag::Strikethrough => {
                                    let child = parent.add_inline();
                                    child.style.strikethrough = true;
                                    self.build_dom(child);
                                }
                                Tag::Link(_linktype, dest, _title) => {
                                    if let Some(mut links) = self.links.take() {
                                        {
                                            let child = links.add_text(dest);
                                            child.style.fg = DomColor::from_dark(TermColor::Blue);
                                            child.style.underline = true;
                                        }
                                        {
                                            links.add_break();
                                        }
                                        self.links = Some(links);
                                    }
                                    let child = parent.add_inline();
                                    child.style.underline = true;
                                    child.style.fg = DomColor::from_dark(TermColor::Blue);
                                    self.build_dom(child);
                                }
                                Tag::Image(_linktype, dest, title) => {
                                    {
                                        let child = parent.add_text(title);
                                        child.style.fg = DomColor::from_light(TermColor::Black);
                                        child.style.bg = DomColor::from_dark(TermColor::Yellow);
                                    }
                                    {
                                        let child = parent.add_text(dest);
                                        child.style.fg = DomColor::from_dark(TermColor::Blue);
                                        child.style.bg = DomColor::from_dark(TermColor::Yellow);
                                        child.style.underline = true;
                                    }
                                    let child = parent.add_inline();
                                    child.style.italic = true;
                                    self.build_dom(child);
                                }
                                Tag::FootnoteDefinition(name) => {
                                    if let Some(mut footnotes) = self.footnotes.take() {
                                        {
                                            let child = footnotes.add_text(name);
                                            child.style.fg = DomColor::from_dark(TermColor::Green);
                                            child.style.underline = true;
                                        }
                                        self.build_dom(&mut footnotes);
                                        self.footnotes = Some(footnotes);
                                    }
                                }
                            }
                        }
                        Event::End(tag) => {
                            match tag {
                                Tag::Paragraph => {
                                    break;
                                }
                                Tag::Heading(..) => {
                                    break;
                                }
                                Tag::Table(_) => {}
                                Tag::TableHead => {}
                                Tag::TableRow => {}
                                Tag::TableCell => {}
                                Tag::BlockQuote => {
                                    break;
                                }
                                Tag::CodeBlock(_) => {
                                    self.highline = None;
                                    self.syntax = None;
                                    break;
                                }
                                Tag::List(None) => {
                                    for child in &mut parent.children {
                                        {
                                            if let BoxKind::ListBullet = child.kind {
                                                child.add_text(CowStr::from("*"));
                                            }
                                        }
                                    }
                                    break;
                                }
                                Tag::List(Some(start)) => {
                                    let mut i = start;
                                    // TODO resize all bullets like the last one
                                    //let end = start + node.children.len() / 2;
                                    for child in &mut parent.children {
                                        {
                                            if let BoxKind::ListBullet = child.kind {
                                                child.add_text(CowStr::from(i.to_string()));
                                                i += 1;
                                            }
                                        }
                                    }
                                    break;
                                }
                                Tag::Item => {
                                    break;
                                }
                                Tag::Emphasis => {
                                    break;
                                }
                                Tag::Strong => {
                                    break;
                                }
                                Tag::Strikethrough => {
                                    break;
                                }
                                Tag::Link(..) => {
                                    break;
                                }
                                Tag::Image(..) => {
                                    break;
                                }
                                Tag::FootnoteDefinition(..) => {
                                    break;
                                }
                            }
                        }
                        // FIXME handle Code specially
                        Event::Text(mut text) | Event::Code(mut text) => {
                            if let Some(ref mut h) = self.highline {
                                match text {
                                    CowStr::Borrowed(text) => {
                                        let ranges = h.highlight(&text);
                                        for (style, mut text) in ranges {
                                            let mut add_break = false;
                                            if text.len() > 0 {
                                                // check if text ends with a newline
                                                let bytes = text.as_bytes();
                                                if bytes[bytes.len() - 1] == 10 {
                                                    add_break = true;
                                                }
                                            }
                                            if add_break {
                                                text = &text[..text.len() - 1];
                                            }
                                            {
                                                let child = parent.add_text(CowStr::Borrowed(text));
                                                child.style.fg = DomColor::from_color_lo(
                                                    style.foreground.r,
                                                    style.foreground.g,
                                                    style.foreground.b,
                                                );
                                                child.style.bold |= style
                                                    .font_style
                                                    .intersects(highlighting::FontStyle::BOLD);
                                                child.style.italic |= style
                                                    .font_style
                                                    .intersects(highlighting::FontStyle::ITALIC);
                                                child.style.underline |= style
                                                    .font_style
                                                    .intersects(highlighting::FontStyle::UNDERLINE);
                                            }
                                            if add_break {
                                                parent.add_break();
                                            }
                                        }
                                    }
                                    _ => unimplemented!(),
                                }
                            } else {
                                let mut add_break = false;
                                if text.len() > 0 {
                                    // check if text ends with a newline
                                    let bytes = text.as_bytes();
                                    if bytes[bytes.len() - 1] == 10 {
                                        add_break = true;
                                    }
                                }
                                if add_break {
                                    let pos = text.len() - 1;
                                    split_at_in_place(&mut text, pos);
                                }
                                parent.add_text(text);
                                if add_break {
                                    parent.add_break();
                                }
                            }
                        }
                        Event::TaskListMarker(checked) => {
                            let child =
                                parent.add_text(CowStr::from(if checked { "[ ]" } else { "[X]" }));
                            self.build_dom(child);
                        }
                        Event::Rule => {
                            let child = parent.add_block();
                            child.style.extend = true;
                            child.size.border.bottom += 1;
                            child.style.border_type = BorderType::Thin;
                            child.style.fg = DomColor::from_dark(TermColor::Yellow);
                        }
                        Event::Html(html) => {
                            let child = parent.add_text(html);
                            child.style.fg = DomColor::from_light(TermColor::Red);
                        }
                        Event::SoftBreak => {
                            parent.add_break();
                        }
                        Event::HardBreak => {
                            parent.add_break();
                        }
                        Event::FootnoteReference(name) => {
                            let child = parent.add_text(name);
                            child.style.fg = DomColor::from_dark(TermColor::Green);
                            child.style.underline = true;
                        }
                    }
                }
                None => break,
            }
        }
    }
}

pub fn push_ansi<'a, I: Iterator<Item = Event<'a>>>(iter: I, width: XY) {
    let syntaxes = SyntaxSet::load_defaults_newlines();
    let themes = highlighting::ThemeSet::load_defaults();
    let mut ctx = Ctx::new(iter, &syntaxes, &themes);
    let mut root = ctx.build(width);
    //println!("root:\n{:#?}\n", root);
    root.layout();
    //println!("root:\n{:#?}\n", root);
    root.render();
}
