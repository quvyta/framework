//! Markdown documents: guides, help screens and descriptions.

use std::cell::RefCell;
use std::fmt;
use std::ops::Range;
use std::sync::{Arc, Mutex, PoisonError};

use pulldown_cmark::{CodeBlockKind, Event as MdEvent, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::cells;
use super::code_view::{CodeRow, code_rows, gutter_width, padding_decoration, paint_rows};
use super::highlight::Language;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::style::CellStyle;
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// Inline formatting of a piece of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Format {
    strong: bool,
    emphasis: bool,
    code: bool,
    link: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Inline {
    text: String,
    format: Format,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Block {
    Heading(u8, Vec<Inline>),
    Paragraph(Vec<Inline>),
    Item { depth: u16, marker: Option<u64>, content: Vec<Inline> },
    Quote(Vec<Inline>),
    Code { language: Language, text: String },
    Rule,
}

fn parse(source: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut inlines: Vec<Inline> = Vec::new();
    let mut format = Format::default();
    let mut lists: Vec<Option<u64>> = Vec::new();
    let mut item_marker: Option<Option<u64>> = None;
    let mut heading: Option<u8> = None;
    let mut quote_depth = 0;
    let mut code: Option<(Language, String)> = None;

    let push_text = |inlines: &mut Vec<Inline>, text: &str, format: Format| {
        if let Some(last) = inlines.last_mut()
            && last.format == format
        {
            last.text.push_str(text);
            return;
        }
        inlines.push(Inline { text: text.to_owned(), format });
    };

    let flush = |blocks: &mut Vec<Block>,
                 inlines: &mut Vec<Inline>,
                 lists: &[Option<u64>],
                 item_marker: &mut Option<Option<u64>>,
                 heading: Option<u8>,
                 quote_depth: u32| {
        if inlines.is_empty() {
            return;
        }
        let content = std::mem::take(inlines);
        let block = if let Some(level) = heading {
            Block::Heading(level, content)
        } else if item_marker.is_some() || !lists.is_empty() {
            // The first block of an item carries its number; later blocks of the item get a bullet.
            let depth = u16::try_from(lists.len().saturating_sub(1)).unwrap_or(u16::MAX);
            Block::Item { depth, marker: item_marker.take().flatten(), content }
        } else if quote_depth > 0 {
            Block::Quote(content)
        } else {
            Block::Paragraph(content)
        };
        blocks.push(block);
    };

    for event in Parser::new_ext(source, Options::ENABLE_STRIKETHROUGH) {
        match event {
            MdEvent::Start(Tag::Heading { level, .. }) => {
                heading = Some(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    _ => 3,
                });
            }
            MdEvent::End(TagEnd::Heading(_)) => {
                flush(&mut blocks, &mut inlines, &lists, &mut item_marker, heading, quote_depth);
                heading = None;
            }
            MdEvent::Start(Tag::List(start)) => {
                flush(&mut blocks, &mut inlines, &lists, &mut item_marker, heading, quote_depth);
                lists.push(start);
            }
            MdEvent::End(TagEnd::List(_)) => {
                flush(&mut blocks, &mut inlines, &lists, &mut item_marker, heading, quote_depth);
                lists.pop();
            }
            MdEvent::Start(Tag::Item) => {
                flush(&mut blocks, &mut inlines, &lists, &mut item_marker, heading, quote_depth);
                let marker = lists.last_mut().and_then(|list| {
                    let current = *list;
                    if let Some(n) = list {
                        *n += 1;
                    }
                    current
                });
                item_marker = Some(marker);
            }
            MdEvent::End(TagEnd::Item | TagEnd::Paragraph | TagEnd::BlockQuote(_)) => {
                flush(&mut blocks, &mut inlines, &lists, &mut item_marker, heading, quote_depth);
                if let MdEvent::End(TagEnd::BlockQuote(_)) = event {
                    quote_depth = quote_depth.saturating_sub(1);
                }
            }
            MdEvent::Start(Tag::BlockQuote(_)) => quote_depth += 1,
            MdEvent::Start(Tag::CodeBlock(kind)) => {
                flush(&mut blocks, &mut inlines, &lists, &mut item_marker, heading, quote_depth);
                let language = match kind {
                    CodeBlockKind::Fenced(tag) => Language::from_tag(&tag),
                    CodeBlockKind::Indented => Language::Plain,
                };
                code = Some((language, String::new()));
            }
            MdEvent::End(TagEnd::CodeBlock) => {
                if let Some((language, text)) = code.take() {
                    blocks.push(Block::Code { language, text: text.trim_end_matches('\n').to_owned() });
                }
            }
            MdEvent::Start(Tag::Strong) => format.strong = true,
            MdEvent::End(TagEnd::Strong) => format.strong = false,
            MdEvent::Start(Tag::Emphasis) => format.emphasis = true,
            MdEvent::End(TagEnd::Emphasis) => format.emphasis = false,
            MdEvent::Start(Tag::Link { .. }) => format.link = true,
            MdEvent::End(TagEnd::Link) => format.link = false,
            MdEvent::Text(text) => match &mut code {
                Some((_, body)) => body.push_str(&text),
                None => push_text(&mut inlines, &text, format),
            },
            MdEvent::Code(text) => push_text(&mut inlines, &text, Format { code: true, ..format }),
            MdEvent::SoftBreak => push_text(&mut inlines, " ", format),
            MdEvent::HardBreak => push_text(&mut inlines, "\n", format),
            MdEvent::Rule => {
                flush(&mut blocks, &mut inlines, &lists, &mut item_marker, heading, quote_depth);
                blocks.push(Block::Rule);
            }
            _ => {}
        }
    }
    flush(&mut blocks, &mut inlines, &lists, &mut item_marker, heading, quote_depth);
    blocks
}

/// Rendered Markdown: headings with the accent pillar, wrapped paragraphs, lists, quotes and
/// highlighted code blocks. Horizontal rules become space, never a drawn line.
///
/// The document is a text selection region: a mouse drag selects inside it (turn it off with
/// [`NodeMut::selectable`](crate::widget::NodeMut::selectable)). Clean copies leave out the
/// pillars of headings and quotes with the cell after them, and the padding and line numbers of
/// code blocks.
///
/// Building one in `view` every frame is cheap: the last few documents parsed on a thread are
/// remembered by their source text, together with how they were laid out at the last few widths,
/// so an unchanged document is neither parsed nor wrapped again.
///
/// Style keys: `markdown-heading.h1|h2|h3` (`fg`, `bold`, `pillar`), `markdown-text`,
/// `markdown-strong`, `markdown-emphasis`, `markdown-code`, `markdown-link`, `markdown-bullet`,
/// `markdown-quote` (`fg`, `pillar`), `code` and `code-token.<kind>` for code blocks.
#[derive(Clone)]
pub struct Markdown {
    document: Arc<Document>,
}

impl Markdown {
    /// Parses `source`, or reuses the document parsed from the same source a moment ago.
    #[must_use]
    pub fn new(source: &str) -> Self {
        Self { document: Document::cached(source) }
    }
}

impl fmt::Debug for Markdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Markdown").field("blocks", &self.document.blocks).finish()
    }
}

impl PartialEq for Markdown {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.document, &other.document) || self.document.blocks == other.document.blocks
    }
}

impl Eq for Markdown {}

/// How many parsed documents a thread remembers. A screen shows a handful of documents at once
/// (a guide, a reference, a note).
const CACHED_DOCUMENTS: usize = 8;

/// How many widths a document remembers its layout for. A scroll view may measure its content
/// with and without room for its scrollbar before it paints.
const CACHED_LAYOUTS: usize = 4;

thread_local! {
    /// The documents parsed last on this thread, most recently used first.
    static DOCUMENTS: RefCell<Vec<Arc<Document>>> = const { RefCell::new(Vec::new()) };
}

/// A parsed source with the layouts computed for it.
struct Document {
    source: String,
    blocks: Vec<Block>,
    /// For every block, its inline texts joined into one string with the byte range of each
    /// inline; empty for code blocks and rules.
    texts: Vec<(String, Vec<Range<usize>>)>,
    /// Layouts at recent widths, most recently used first.
    layouts: Mutex<Vec<Arc<Layout>>>,
}

impl Document {
    /// The document for `source`: the remembered one when this thread parsed the same source
    /// recently, otherwise a freshly parsed one that is remembered in place of the oldest.
    fn cached(source: &str) -> Arc<Self> {
        DOCUMENTS.with_borrow_mut(|documents| {
            let document = match documents.iter().position(|document| document.source == source) {
                Some(index) => documents.remove(index),
                None => Arc::new(Self::parse(source)),
            };
            documents.insert(0, Arc::clone(&document));
            documents.truncate(CACHED_DOCUMENTS);
            document
        })
    }

    fn parse(source: &str) -> Self {
        let blocks = parse(source);
        let texts = blocks
            .iter()
            .map(|block| match block {
                Block::Heading(_, content)
                | Block::Paragraph(content)
                | Block::Quote(content)
                | Block::Item { content, .. } => joined(content),
                Block::Code { .. } | Block::Rule => (String::new(), Vec::new()),
            })
            .collect();
        Self { source: source.to_owned(), blocks, texts, layouts: Mutex::new(Vec::new()) }
    }

    /// The document laid out `width` cells wide with code blocks padded by `code_padding`
    /// (rows, columns); computed once per width and remembered.
    fn layout(&self, width: u16, code_padding: (u16, u16)) -> Arc<Layout> {
        let mut layouts = self.layouts.lock().unwrap_or_else(PoisonError::into_inner);
        let key = (width, code_padding);
        let layout = match layouts.iter().position(|layout| (layout.width, layout.code_padding) == key) {
            Some(index) => layouts.remove(index),
            None => Arc::new(self.lay_out(width, code_padding)),
        };
        layouts.insert(0, Arc::clone(&layout));
        layouts.truncate(CACHED_LAYOUTS);
        layout
    }

    fn lay_out(&self, width: u16, code_padding: (u16, u16)) -> Layout {
        let mut rows = 0u16;
        let mut blocks = Vec::with_capacity(self.blocks.len());
        for (index, block) in self.blocks.iter().enumerate() {
            let placed = self.place(index, width, code_padding, rows);
            rows = rows.saturating_add(placed.height);
            if block.gap_after(self.blocks.get(index + 1)) && index + 1 < self.blocks.len() {
                rows = rows.saturating_add(1);
            }
            blocks.push(placed);
        }
        Layout { width, code_padding, blocks }
    }

    /// Block `index` wrapped to `width` cells, starting at row `top`.
    fn place(&self, index: usize, width: u16, code_padding: (u16, u16), top: u16) -> Placed {
        let block = &self.blocks[index];
        let (rows, lines, code) = match block {
            Block::Heading(..) | Block::Paragraph(_) | Block::Quote(_) | Block::Item { .. } => {
                let lines = text::wrap_ranges(&self.texts[index].0, width.saturating_sub(indent_of(block)));
                (lines.len(), lines, None)
            }
            Block::Code { language, text } => {
                let sides = code_padding.1.saturating_mul(2).saturating_add(gutter_width(text));
                let inner = width.saturating_sub(sides).max(1);
                let rows = code_rows(text, *language, inner);
                (rows.len() + usize::from(code_padding.0.saturating_mul(2)), Vec::new(), Some((inner, rows)))
            }
            Block::Rule => (0, Vec::new(), None),
        };
        Placed { top, height: clamp_u16(i32::try_from(rows).unwrap_or(i32::MAX)), lines, code }
    }
}

/// A document laid out at one width.
struct Layout {
    width: u16,
    code_padding: (u16, u16),
    /// One entry per block, top to bottom.
    blocks: Vec<Placed>,
}

impl Layout {
    /// Rows the whole document takes.
    fn height(&self) -> u16 {
        self.blocks.last().map_or(0, |placed| placed.top.saturating_add(placed.height))
    }
}

/// Where a block goes and how it wraps.
struct Placed {
    /// First row, counted from the top of the document.
    top: u16,
    height: u16,
    /// Byte ranges into the block's joined text, one per wrapped line.
    lines: Vec<Range<usize>>,
    /// For a code block: the width its code was wrapped to, and the rows.
    code: Option<(u16, Vec<CodeRow>)>,
}

impl Block {
    /// Whether a blank row separates this block from the next one; list items stay together.
    fn gap_after(&self, next: Option<&Self>) -> bool {
        !(matches!(self, Self::Item { .. }) && matches!(next, Some(Self::Item { .. })))
    }
}

fn joined(content: &[Inline]) -> (String, Vec<Range<usize>>) {
    let mut out = String::new();
    let mut ranges = Vec::new();
    for inline in content {
        let start = out.len();
        out.push_str(&inline.text);
        ranges.push(start..out.len());
    }
    (out, ranges)
}

fn indent_of(block: &Block) -> u16 {
    match block {
        Block::Heading(..) | Block::Quote(_) => 2,
        Block::Item { depth, marker, .. } => cells::sum([2, depth.saturating_mul(2), marker_width(*marker), 1]),
        Block::Paragraph(_) | Block::Code { .. } | Block::Rule => 0,
    }
}

/// Cells taken by a list marker: one for a bullet, the digits and dot for a number.
fn marker_width(marker: Option<u64>) -> u16 {
    match marker {
        None => 1,
        Some(n) => text::width(&n.to_string()).saturating_add(1),
    }
}

impl Markdown {
    fn inline_styles(cx: &mut PaintCx<'_>, content: &[Inline], base: CellStyle) -> Vec<CellStyle> {
        content
            .iter()
            .map(|inline| {
                let mut style = base;
                if inline.format.strong {
                    style = merge(style, cx.style("markdown-strong", None, &[]).text());
                }
                if inline.format.emphasis {
                    style = merge(style, cx.style("markdown-emphasis", None, &[]).text());
                }
                if inline.format.link {
                    style = merge(style, cx.style("markdown-link", None, &[]).text());
                }
                if inline.format.code {
                    style = merge(style, cx.style("markdown-code", None, &[]).text());
                }
                style
            })
            .collect()
    }

    /// Draws the inline content of block `index` inside `area`, wrapped as `placed` says.
    fn paint_inline(&self, cx: &mut PaintCx<'_>, area: Rect, index: usize, placed: &Placed, base: CellStyle) {
        let (Block::Heading(_, content)
        | Block::Paragraph(content)
        | Block::Quote(content)
        | Block::Item { content, .. }) = &self.document.blocks[index]
        else {
            return;
        };
        let (text, ranges) = &self.document.texts[index];
        let styles = Self::inline_styles(cx, content, base);
        for (row, line) in placed.lines.iter().enumerate() {
            let y = area.y + i32::try_from(row).unwrap_or(0);
            let mut x = area.x;
            for (range, style) in ranges.iter().zip(&styles) {
                let start = range.start.max(line.start);
                let end = range.end.min(line.end);
                if start < end {
                    x += i32::from(cx.text(x, y, &text[start..end], *style, area.width));
                }
            }
        }
    }
}

/// Layers `over` onto `base`: colours and switched-on attributes of `over` win.
fn merge(base: CellStyle, over: CellStyle) -> CellStyle {
    CellStyle {
        fg: over.fg.or(base.fg),
        bg: over.bg.or(base.bg),
        bold: base.bold || over.bold,
        italic: base.italic || over.italic,
        underline: base.underline || over.underline,
        dim: base.dim || over.dim,
    }
}

impl<Msg: 'static> Widget<Msg> for Markdown {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let code_padding = cx.env().theme().style("code", None, &[]).pair("padding").unwrap_or((1, 2));
        let height = self.document.layout(available.width, code_padding).height();
        Size::new(available.width, height.min(available.height))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.selectable(area);
        let code_style = cx.style("code", None, &[]);
        let code_padding = code_style.padding();
        let layout = self.document.layout(area.width, (code_padding.top, code_padding.left));
        let clip = cx.clip();
        // Blocks are placed top to bottom, so the ones in the clip are one run: skip to its start
        // and stop after its end.
        let bottom = |placed: &Placed| area.y + i32::from(placed.top) + i32::from(placed.height);
        let first = layout.blocks.partition_point(|placed| bottom(placed) < clip.y);
        for (index, placed) in layout.blocks.iter().enumerate().skip(first) {
            let height = placed.height;
            let rect = Rect::new(area.x, area.y + i32::from(placed.top), area.width, height);
            if rect.y >= clip.bottom() {
                break;
            }
            let block = &self.document.blocks[index];
            let indent = indent_of(block);
            let body = Rect::new(rect.x + i32::from(indent), rect.y, rect.width.saturating_sub(indent), height);
            match block {
                Block::Heading(level, _) => {
                    let variant = format!("h{level}");
                    let style = cx.style("markdown-heading", Some(&variant), &[]);
                    if let Some(color) = style.color("pillar") {
                        cx.pillar(rect.x, rect.y, color);
                    }
                    // The pillar and its gap are not part of the heading's text.
                    cx.decoration(Rect::new(rect.x, rect.y, indent, height));
                    self.paint_inline(cx, body, index, placed, style.text());
                }
                Block::Paragraph(_) => {
                    let style = cx.style("markdown-text", None, &[]).text();
                    self.paint_inline(cx, body, index, placed, style);
                }
                Block::Quote(_) => {
                    let style = cx.style("markdown-quote", None, &[]);
                    if let Some(color) = style.color("pillar") {
                        for row in 0..height {
                            cx.pillar(rect.x, rect.y + i32::from(row), color);
                        }
                    }
                    cx.decoration(Rect::new(rect.x, rect.y, indent, height));
                    self.paint_inline(cx, body, index, placed, style.text());
                }
                Block::Item { depth, marker, .. } => {
                    let bullet_style = cx.style("markdown-bullet", None, &[]).text();
                    let bullet = match marker {
                        None => cx.env().icons().glyph("bullet").into_owned(),
                        Some(n) => format!("{n}."),
                    };
                    let bullet_x = rect.x + 2 + i32::from(*depth) * 2;
                    cx.text(bullet_x, rect.y, &bullet, bullet_style, marker_width(*marker));
                    let style = cx.style("markdown-text", None, &[]).text();
                    self.paint_inline(cx, body, index, placed, style);
                }
                Block::Code { language, text } => {
                    if let Some(bg) = code_style.text().bg {
                        cx.clear(rect, bg);
                    }
                    let inner = rect.inset(code_padding);
                    padding_decoration(cx, rect, inner);
                    let gutter = gutter_width(text);
                    let width = inner.width.saturating_sub(gutter).max(1);
                    match &placed.code {
                        // Wrapped at this width by the layout already.
                        Some((wrapped, rows)) if *wrapped == width => paint_rows(cx, inner, rows, gutter),
                        _ => paint_rows(cx, inner, &code_rows(text, *language, width), gutter),
                    }
                }
                Block::Rule => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo(&'static str);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(Markdown::new(self.0)).fill();
        }
    }

    #[test]
    fn documents_are_shared_by_source_and_the_cache_stays_bounded() {
        let first = Markdown::new("## Deploys\n\nAll green.");
        let again = Markdown::new("## Deploys\n\nAll green.");
        assert!(Arc::ptr_eq(&first.document, &again.document), "the same source is parsed once");
        assert_ne!(first, Markdown::new("## Deploys\n\nTwo failed."));
        for n in 0..CACHED_DOCUMENTS * 3 {
            let _ = Markdown::new(&format!("Release {n}"));
        }
        assert_eq!(DOCUMENTS.with_borrow(Vec::len), CACHED_DOCUMENTS);
        let fresh = Markdown::new("## Deploys\n\nAll green.");
        assert!(!Arc::ptr_eq(&first.document, &fresh.document), "the oldest documents are forgotten");
        assert_eq!(first, fresh, "a document parsed again is equal");
        for width in 10..20 {
            let _ = first.document.layout(width, (1, 2));
        }
        assert_eq!(first.document.layouts.lock().map(|layouts| layouts.len()).ok(), Some(CACHED_LAYOUTS));
    }

    #[test]
    fn remembered_layouts_paint_exactly_like_fresh_ones() {
        let source = "## Rollout\n\nThe **api** image `quvyta/api:2.4` rolls out to every region in turn.\n\n\
                      1. drain the old pods\n2. start the new ones\n\n> Watch the error rate.\n\n\
                      ```rust\nfn main() { println!(\"deploying quvyta/api:2.4 to every region\"); }\n```\n";
        let mut h = Harness::new(Demo(source), 30, 24);
        h.resize(52, 24).resize(41, 24).resize(30, 24).resize(52, 24);
        for width in [30, 41, 52] {
            let remembered = h.resize(width, 24).html("markdown");
            DOCUMENTS.with_borrow_mut(Vec::clear);
            let fresh = Harness::new(Demo(source), width, 24);
            assert_eq!(remembered, fresh.html("markdown"), "width {width}");
        }
    }

    #[test]
    fn parses_blocks_and_inlines() {
        let blocks = parse(
            "## When to use\n\nPress **Enter** or `space`.\n\n- one\n- two\n\n1. first\n\n---\n\n> note\n\n```toml\nbg = 1\n```\n",
        );
        assert!(matches!(&blocks[0], Block::Heading(2, _)));
        let Block::Paragraph(inlines) = &blocks[1] else { panic!("paragraph") };
        assert!(inlines.iter().any(|i| i.text == "Enter" && i.format.strong));
        assert!(inlines.iter().any(|i| i.text == "space" && i.format.code));
        assert!(matches!(blocks[2], Block::Item { depth: 0, marker: None, .. }));
        assert!(matches!(blocks[4], Block::Item { marker: Some(1), .. }));
        assert!(matches!(blocks[5], Block::Rule));
        assert!(matches!(blocks[6], Block::Quote(_)));
        assert!(matches!(&blocks[7], Block::Code { language: Language::Toml, text } if text == "bg = 1"));
    }

    #[test]
    fn later_blocks_of_an_item_get_a_bullet_and_long_numbers_show_whole() {
        let blocks = parse("1. first\n\n   more\n");
        assert!(matches!(blocks[0], Block::Item { depth: 0, marker: Some(1), .. }));
        assert!(matches!(blocks[1], Block::Item { depth: 0, marker: None, .. }), "{blocks:?}");
        let h = Harness::new(Demo("99. deploy\n100. verify\n\n     notes"), 24, 5);
        assert_eq!(h.screen(), "  99. deploy\n  100. verify\n  • notes\n\n\n");
    }

    #[test]
    fn renders_without_ascii_decoration() {
        let h = Harness::new(
            Demo("## Usage\n\nA widget that **wraps** nicely here.\n\n- first\n- second\n\n---\n\nEnd"),
            24,
            12,
        );
        assert_eq!(h.screen(), "▌ Usage\n\nA widget that wraps\nnicely here.\n\n  • first\n  • second\n\n\nEnd\n\n\n");
        let theme = h.env().theme();
        assert_eq!(h.fg(0, 0), theme.style("markdown-heading", Some("h2"), &[]).paint("pillar").map(|p| p.at(0.0)));
    }

    /// Rows of `harness` that hold nothing but closing punctuation.
    fn lone_punctuation(harness: &Harness<Demo>) -> Vec<String> {
        let screen = harness.screen();
        let rows = screen.lines().map(str::trim).filter(|row| !row.is_empty());
        rows.filter(|row| row.chars().all(|c| ".,;:)".contains(c))).map(str::to_owned).collect()
    }

    #[test]
    fn punctuation_after_inline_code_never_wraps_alone() {
        // A list step with inline code, at every width from a phone-narrow column to a wide one.
        let step =
            "2. Write an enum of everything that can happen: `Msg`. Every button, field and list sends one of these.";
        for width in 10..=110 {
            let h = Harness::new(Demo(step), width, 30);
            assert_eq!(lone_punctuation(&h), Vec::<String>::new(), "width {width}:\n{}", h.screen());
        }
        let h = Harness::new(Demo(step), 54, 4);
        assert_eq!(
            h.screen(),
            "  2. Write an enum of everything that can happen: Msg.\n     Every button, field and list sends one of these.\n\n\n"
        );

        // A code span wider than the line breaks inside it, and the closing punctuation keeps the
        // last character of the span company.
        let badge = "1. Add a neutral badge: `ui.add(Badge::new(\"Paused\"))`.";
        // Widths where a naive break leaves `).` alone on the last row (29) or a lone `.` (12).
        let h = Harness::new(Demo(badge), 29, 3);
        assert_eq!(h.screen(), "  1. Add a neutral badge: ui.\n     add(Badge::new(\"Pause\n     d\")).\n");
        let theme = h.env().theme();
        let code = theme.style("markdown-code", None, &[]).paint("fg").map(|p| p.at(0.0));
        let text = theme.style("markdown-text", None, &[]).paint("fg").map(|p| p.at(0.0));
        assert_eq!((h.fg(5, 2), h.fg(8, 2)), (code, code), "the carried-over code keeps its style");
        assert_eq!(h.fg(9, 2), text, "the full stop is plain text");
        assert_ne!(text, code);
        for width in 10..=60 {
            let h = Harness::new(Demo(badge), width, 12);
            assert_eq!(lone_punctuation(&h), Vec::<String>::new(), "width {width}:\n{}", h.screen());
        }
    }
}
