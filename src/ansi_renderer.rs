// Copyright 2016 Xavier Bestel -  All rights reserved.
//
// GPL goes here

//! ANSI renderer for pulldown-cmark.

use std::fmt;
use std::borrow::Cow;

use pulldown_cmark::{Event, Tag};
use pulldown_cmark::Event::{Start, End, Text, Html, InlineHtml, SoftBreak, HardBreak,
                            FootnoteReference};

use ansi_term::{Style, Colour};
use ansi_term::{ANSIString, ANSIStrings};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub const DEFAULT_COLS: u16 = 80;

fn findsplit(s: &str, pos: usize) -> usize {
    if let Some(n) = UnicodeSegmentation::grapheme_indices(s, true).nth(pos) {
        return n.0;
    }
    s.len()
}

fn split_at_in_place<'a>(cow: &mut Cow<'a, str>, mid: usize) -> Cow<'a, str> {
    match *cow {
        Cow::Owned(ref mut s) => {
            let s2 = s[mid..].to_string();
            s.truncate(mid);
            Cow::Owned(s2)
        }
        Cow::Borrowed(s) => {
            let (s1, s2) = s.split_at(mid);
            *cow = Cow::Borrowed(s1);
            Cow::Borrowed(s2)
        }
    }
}

enum TermColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Purple,
    Cyan,
    White,
}

#[derive(Debug, Clone)]
struct DomColor(Option<u8>);

impl DomColor {
    fn default() -> DomColor {
        DomColor(None)
    }
    fn from_dark(color: TermColor) -> DomColor {
        DomColor(Some(color as u8))
    }
    fn from_light(color: TermColor) -> DomColor {
        DomColor(Some(color as u8 + 8))
    }
    fn from_grey(level: u8) -> DomColor {
        let mut level = level >> 4;
        level = match level {
            0 => 16,
            15 => 231,
            grey => 231 + grey,
        };
        DomColor(Some(level))
    }
    fn from_color(red: u8, green: u8, blue: u8) -> DomColor {
        let red = (red as u32 * 6 / 256) as u8;
        let green = (green as u32 * 6 / 256) as u8;
        let blue = (blue as u32 * 6 / 256) as u8;
        DomColor(Some(16 + red * 36 + green * 6 + blue))
    }
    fn index(&self) -> Option<u8> {
        self.0
    }
}

#[derive(Debug, Clone)]
enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone)]
struct DomStyle {
    bg: DomColor,
    fg: DomColor,
    bold: bool,
    underline: bool,
    strikethrough: bool,
    italic: bool,
    code: bool,
    align: TextAlign,
}

impl DomStyle {
    fn to_ansi(&self) -> Style {
        let mut astyle = Style::new();
        match self.fg.0 {
            None => {}
            Some(idx) => {
                astyle = astyle.fg(Colour::Fixed(idx));
            }
        }
        match self.bg.0 {
            None => {}
            Some(idx) => {
                astyle = astyle.on(Colour::Fixed(idx));
            }
        }
        if self.bold {
            astyle = astyle.bold();
        }
        if self.underline {
            astyle = astyle.underline();
        }
        if self.strikethrough {
            astyle = astyle.strikethrough();
        }
        if self.italic {
            astyle = astyle.italic();
        }
        astyle
    }
}

#[derive(Debug, Clone)]
enum BoxKind<'a> {
    Text(Cow<'a, str>),
    Break,
    InlineContainer,
    Inline,
    Block,
    Header(u8),
    BlockQuote,
    List(Option<usize>),
    ListItem,
    Table,
    TableColumn,
    TableItem,
    Image,
}

#[derive(Default, Debug, Copy, Clone)]
struct BoxCursor {
    container: BoxSize,
    x: u16,
    y: u16,
}

impl fmt::Display for BoxCursor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "[{} {}] [{} {} +{} +{}] [+{} +{} -{} -{}]",
               self.x,
               self.y,
               self.container.content.x,
               self.container.content.y,
               self.container.content.w,
               self.container.content.h,
               self.container.border.top,
               self.container.border.left,
               self.container.border.bottom,
               self.container.border.right)
    }
}

#[derive(Default, Debug, Copy, Clone)]
struct BoxSize {
    content: Rect,
    border: Edges,
}

#[derive(Default, Debug, Copy, Clone)]
struct Rect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

#[derive(Default, Debug, Copy, Clone)]
struct Edges {
    top: u16,
    bottom: u16,
    left: u16,
    right: u16,
}

#[derive(Debug)]
enum LayoutRes<T> {
    Normal,
    CutHere(T),
    Reject,
}

#[derive(Debug)]
struct DomBox<'a> {
    kind: BoxKind<'a>,
    size: BoxSize,
    style: DomStyle,
    children: Vec<DomBox<'a>>,
}

impl<'a> DomBox<'a> {
    fn new_root(width: u16) -> DomBox<'a> {
        let mut dombox = DomBox {
            size: Default::default(),
            kind: BoxKind::Block,
            style: DomStyle {
                fg: DomColor::default(),
                bg: DomColor::default(),
                bold: false,
                underline: false,
                strikethrough: false,
                italic: false,
                code: false,
                align: TextAlign::Left,
            },
            children: vec![],
        };
        dombox.size.content.w = width;
        dombox
    }
    fn get_inline_container(&mut self) -> &mut DomBox<'a> {
        match self.kind {
            BoxKind::Inline | BoxKind::InlineContainer => self,
            _ => {
                match self.children.last() {
                    Some(&DomBox { kind: BoxKind::InlineContainer, .. }) => {}
                    _ => {
                        self.children
                            .push(DomBox {
                                      size: Default::default(),
                                      kind: BoxKind::InlineContainer,
                                      style: self.style.clone(),
                                      children: vec![],
                                  })
                    }
                }
                self.children.last_mut().unwrap()
            }
        }
    }
    fn add_text(&mut self, text: Cow<'a, str>) -> &mut DomBox<'a> {
        let inline_container = self.get_inline_container();
        inline_container
            .children
            .push(DomBox {
                      size: Default::default(),
                      kind: BoxKind::Text(text),
                      style: inline_container.style.clone(),
                      children: vec![],
                  });
        inline_container.children.last_mut().unwrap()
    }
    fn add_inline(&mut self) -> &mut DomBox<'a> {
        let inline_container = self.get_inline_container();
        inline_container
            .children
            .push(DomBox {
                      size: Default::default(),
                      kind: BoxKind::Inline,
                      style: inline_container.style.clone(),
                      children: vec![],
                  });
        inline_container.children.last_mut().unwrap()
    }
    fn add_block(&mut self) -> &mut DomBox<'a> {
        self.children
            .push(DomBox {
                      size: Default::default(),
                      kind: BoxKind::Block,
                      style: self.style.clone(),
                      children: vec![],
                  });
        self.children.last_mut().unwrap()
    }
    fn add_header(&mut self, level: u8) -> &mut DomBox<'a> {
        self.children
            .push(DomBox {
                      size: Default::default(),
                      kind: BoxKind::Header(level),
                      style: self.style.clone(),
                      children: vec![],
                  });
        self.children.last_mut().unwrap()
    }
    fn add_break(&mut self) -> &mut DomBox<'a> {
        self.children
            .push(DomBox {
                      size: Default::default(),
                      kind: BoxKind::Break,
                      style: self.style.clone(),
                      children: vec![],
                  });
        self.children.last_mut().unwrap()
    }
    fn layout(&mut self) {
        let mut cursor = BoxCursor {
            x: 0,
            y: 0,
            container: self.size,
        };
        self.layout_generic(&mut cursor);
    }
    fn inline_children_loop(&mut self,
                            res: LayoutRes<DomBox<'a>>,
                            dorej: bool)
                            -> LayoutRes<DomBox<'a>> {
        let mut res = res;
        let mut subcursor = BoxCursor {
            x: self.size.content.x,
            y: self.size.content.y,
            container: self.size,
        };
        let mut i = 0;
        while i < self.children.len() {
            if let BoxKind::Break = self.children[i].kind {
                self.children.remove(i);
                res = LayoutRes::CutHere(DomBox {
                                             kind: self.kind.clone(),
                                             size: self.size.clone(),
                                             style: self.style.clone(),
                                             children: self.children.split_off(i),
                                         });
                break;
            }
            match self.children[i].layout_generic(&mut subcursor) {
                LayoutRes::Normal => (),
                LayoutRes::CutHere(next) => {
                    self.children.insert(i + 1, next);
                    res = LayoutRes::CutHere(DomBox {
                                                 kind: self.kind.clone(),
                                                 size: self.size.clone(),
                                                 style: self.style.clone(),
                                                 children: self.children.split_off(i + 1),
                                             });
                    break;
                }
                LayoutRes::Reject => {
                    if i == 0 {
                        if dorej {
                            res = LayoutRes::Reject;
                        } else {
                            panic!("can't reject from first {:?}", self.children[i].kind);
                        }
                    } else {
                        res = LayoutRes::CutHere(DomBox {
                                                     kind: self.kind.clone(),
                                                     size: self.size.clone(),
                                                     style: self.style.clone(),
                                                     children: self.children.split_off(i),
                                                 });
                    }
                    break;
                }
            }
            i += 1;
        }
        self.size.content.w = subcursor.x - self.size.content.x;
        res
    }
    fn layout_generic(&mut self, cursor: &mut BoxCursor) -> LayoutRes<DomBox<'a>> {
        let res = match self.kind {
            BoxKind::Block |
            BoxKind::BlockQuote |
            BoxKind::Header(_) => self.layout_block(cursor),
            BoxKind::InlineContainer => self.layout_inline_container(cursor),
            BoxKind::Text(_) | BoxKind::Inline => self.layout_inline(cursor),
            BoxKind::Break => panic!("shouldn't layout a break"),
            _ => unimplemented!(),
        };
        res
    }
    fn layout_block(&mut self, cursor: &mut BoxCursor) -> LayoutRes<DomBox<'a>> {
        let res = LayoutRes::Normal;
        self.size.content.w = if cursor.container.content.w >
                                 self.size.border.left + self.size.border.right {
            cursor.container.content.w - self.size.border.left - self.size.border.right
        } else {
            1
        };
        self.size.content.h = 0;
        self.size.content.x = cursor.x + self.size.border.left;
        self.size.content.y = cursor.y + self.size.border.top;
        let mut subcursor = BoxCursor {
            x: self.size.content.x,
            y: self.size.content.y,
            container: self.size,
        };
        let mut i = 0;
        while i < self.children.len() {
            if let BoxKind::Break = self.children[i].kind {
                self.children.remove(i);
                continue;
            }
            match self.children[i].layout_generic(&mut subcursor) {
                LayoutRes::Normal => (),
                LayoutRes::CutHere(next) => self.children.insert(i + 1, next),
                LayoutRes::Reject => {
                    panic!("can't reject a {:?}", self.children[i].kind);
                }
            }
            self.size.content.h += self.children[i].size.content.h +
                                   self.children[i].size.border.top +
                                   self.children[i].size.border.bottom;
            i += 1;
        }
        cursor.y += self.size.content.h + self.size.border.top + self.size.border.bottom;
        res
    }
    // OK I changed my mind, this is a LINE, and when split will give n lines
    fn layout_inline_container(&mut self, cursor: &mut BoxCursor) -> LayoutRes<DomBox<'a>> {
        let mut res = LayoutRes::Normal;
        self.size.content.w = if cursor.container.content.w >
                                 self.size.border.left + self.size.border.right {
            cursor.container.content.w - self.size.border.left - self.size.border.right
        } else {
            1
        };
        self.size.content.h = 1;
        self.size.content.x = cursor.x + self.size.border.left;
        self.size.content.y = cursor.y + self.size.border.top;
        res = self.inline_children_loop(res, false);
        cursor.y += self.size.content.h + self.size.border.top + self.size.border.bottom;
        res
    }
    // this one can ask to be splitted if needs be, in this case the returned
    // element must be inserted right after the current one
    fn layout_inline(&mut self, cursor: &mut BoxCursor) -> LayoutRes<DomBox<'a>> {
        let mut res = LayoutRes::Normal;
        self.size.content.h = 1;
        self.size.content.x = cursor.x + self.size.border.left;
        self.size.content.y = cursor.y + self.size.border.top;
        self.size.content.w = cursor.container.content.w - (cursor.x - cursor.container.content.x) -
                              (self.size.border.left + self.size.border.right);
        match self.kind {
            BoxKind::Text(ref mut text) => {
                let width = UnicodeWidthStr::width(&text[..]) as u16;
                if self.size.content.w == 0 {
                    res = LayoutRes::Reject;
                } else if width > self.size.content.w {
                    let pos = findsplit(text, self.size.content.w as usize);
                    let remains = split_at_in_place(text, pos);
                    res = LayoutRes::CutHere(DomBox {
                                                 kind: BoxKind::Text(remains),
                                                 size: self.size.clone(),
                                                 style: self.style.clone(),
                                                 children: vec![],
                                             });
                } else {
                    self.size.content.w = width;
                }
            }
            BoxKind::Inline => {
                res = self.inline_children_loop(res, true);
            }
            _ => {
                panic!("can't layout_inline {:?}", self.kind);
            }
        };
        cursor.x += self.size.content.w;
        res
    }
    fn render(&mut self) {
        let mut strings = Vec::new();
        for line in 0..(self.size.content.h + self.size.border.top + self.size.border.bottom) {
            self.render_line(line, &mut strings);
            strings.push(Style::default().paint("\n"));
        }
        println!("{}", ANSIStrings(&strings));
    }
    fn render_line(&self, line: u16, strings: &mut Vec<ANSIString<'a>>) -> (u16, u16) {
        if line < self.size.content.y - self.size.border.top ||
           line >= self.size.content.y + self.size.content.h + self.size.border.bottom {
            // out of the box, don't render anything
            return (0, 0);
        }
        if line < self.size.content.y || line >= self.size.content.y + self.size.content.h {
            // in the border
            let mut s = String::with_capacity((self.size.content.w + self.size.border.left +
                                               self.size.border.right) as
                                              usize);
            for _ in 0..self.size.content.w + self.size.border.left + self.size.border.right {
                s.push('-');
            }
            let s = self.style.to_ansi().paint(s);
            strings.push(s);
            return (self.size.content.x - self.size.border.left,
                    self.size.content.w + self.size.border.left + self.size.border.right);
        }
        let mut s = String::with_capacity(self.size.border.left as usize);
        for _ in 0..self.size.border.left {
            s.push('|');
        }
        let s = self.style.to_ansi().paint(s);
        strings.push(s);
        let mut pos = self.size.content.x;
        match self.kind {
            BoxKind::Text(ref text) => {
                let s = self.style.to_ansi().paint(text.to_string());
                strings.push(s);
                pos += UnicodeWidthStr::width(&text[..]) as u16;
                assert!(pos <= self.size.content.x + self.size.content.w);
            }
            _ => {
                for child in &self.children {
                    let insert_point = strings.len();
                    let (start, len) = child.render_line(line, strings);
                    if len == 0 {
                        continue;
                    }
                    assert!(start >= pos);
                    assert!(start + len <= self.size.content.x + self.size.content.w);
                    if start > pos {
                        let mut s = String::with_capacity((start - pos) as usize);
                        for _ in 0..(start - pos) {
                            s.push(' ');
                        }
                        let s = self.style.to_ansi().paint(s);
                        strings.insert(insert_point, s);
                    }
                    pos = start + len;
                }
                assert!(pos <= self.size.content.x + self.size.content.w);
            }
        }
        if pos < self.size.content.x + self.size.content.w {
            let mut s = String::with_capacity((self.size.content.x + self.size.content.w - pos) as
                                              usize);
            for _ in 0..(self.size.content.x + self.size.content.w - pos) {
                s.push(' ');
            }
            let s = self.style.to_ansi().paint(s);
            strings.push(s);
        }
        let mut s = String::with_capacity(self.size.border.right as usize);
        for _ in 0..self.size.border.right {
            s.push('|');
        }
        let s = self.style.to_ansi().paint(s);
        strings.push(s);
        return (self.size.content.x - self.size.border.left,
                self.size.content.w + self.size.border.left + self.size.border.right);
    }
}

struct Ctx<I> {
    iter: I,
}

impl<'a, I: Iterator<Item = Event<'a>>> Ctx<I> {
    pub fn build_dom(&mut self, parent: &mut DomBox<'a>) {
        loop {
            match self.iter.next() {
                Some(event) => {
                    match event {
                        Start(tag) => {
                            match tag {
                                Tag::Paragraph => {
                                    {
                                        let child = parent.add_block();
                                        self.build_dom(child);
                                    }
                                    let newline = parent.add_block(); // XXX ugly
                                    newline.add_text(Cow::from(""));
                                }
                                Tag::Rule => {
                                    let child = parent.add_block();
                                    child.size.border.bottom = 1;
                                    child.style.fg = DomColor::from_dark(TermColor::Yellow);
                                }
                                Tag::Header(level) => {
                                    let child = parent.add_header(level as u8);
                                    child.size.border.bottom = 1;
                                    child.style.fg = DomColor::from_dark(TermColor::Purple);
                                    self.build_dom(child);
                                }
                                Tag::Table(_) => {}
                                Tag::TableHead => {}
                                Tag::TableRow => {}
                                Tag::TableCell => {}
                                Tag::BlockQuote => {}
                                Tag::CodeBlock(info) => {
                                    {
                                        let child = parent.add_block();
                                        child.style.code = true;
                                        child.style.fg = DomColor::from_dark(TermColor::White);
                                        child.style.bg = DomColor::from_dark(TermColor::Black);
                                        self.build_dom(child);
                                    }
                                    let newline = parent.add_block(); // XXX ugly
                                    newline.add_text(Cow::from(""));
                                }
                                Tag::List(Some(start)) => {}
                                Tag::List(None) => {}
                                Tag::Item => {}
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
                                Tag::Code => {
                                    let child = parent.add_inline();
                                    child.style.code = true;
                                    child.style.fg = DomColor::from_dark(TermColor::White);
                                    child.style.bg = DomColor::from_dark(TermColor::Black);
                                    self.build_dom(child);
                                }
                                Tag::Link(dest, title) => {
                                    let child = parent.add_inline();
                                    child.style.underline = true;
                                    child.style.fg = DomColor::from_dark(TermColor::Blue);
                                    self.build_dom(child);
                                }
                                Tag::Image(dest, title) => {}
                                Tag::FootnoteDefinition(name) => {}
                            }
                        }
                        End(tag) => {
                            match tag {
                                Tag::Paragraph => {
                                    break;
                                }
                                Tag::Rule => {
                                    break;
                                }
                                Tag::Header(_) => {
                                    break;
                                }
                                Tag::Table(_) => {}
                                Tag::TableHead => {}
                                Tag::TableRow => {}
                                Tag::TableCell => {}
                                Tag::BlockQuote => {}
                                Tag::CodeBlock(_) => {
                                    break;
                                }
                                Tag::List(Some(_)) => {}
                                Tag::List(None) => {}
                                Tag::Item => {}
                                Tag::Emphasis => {
                                    break;
                                }
                                Tag::Strong => {
                                    break;
                                }
                                Tag::Code => {
                                    break;
                                }
                                Tag::Link(dest, title) => {
                                    break;
                                }
                                Tag::Image(_, _) => (), // shouldn't happen, handled in start
                                Tag::FootnoteDefinition(name) => {}
                            }
                        }
                        Text(mut text) => {
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
                        Html(html) => {}
                        InlineHtml(html) => {}
                        SoftBreak => {
                            parent.add_break();
                        }
                        HardBreak => {
                            parent.add_break();
                        }
                        FootnoteReference(name) => {}
                    }
                }
                None => break,
            }
        }
    }
}

pub fn push_ansi<'a, I: Iterator<Item = Event<'a>>>(buf: &mut String, iter: I) {
    let mut ctx = Ctx { iter: iter };
    let mut root = DomBox::new_root(DEFAULT_COLS);
    ctx.build_dom(&mut root);
    println!("root:\n{:#?}\n", root);
    root.layout();
    // write!(buf, "{}\n{}\n", ANSIStrings(&ctx.store), ANSIStrings(&ctx.links)).ok();
    println!("root:\n{:#?}\n", root);
    root.render();
}
