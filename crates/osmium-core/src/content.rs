//! Renderer-neutral content IR compiled from package Markdown.
//!
//! The IR is deliberately small and inert: it carries only text, structure and
//! explicitly classified link targets. Raw HTML is preserved as literal text
//! and never becomes markup, no package code or data URL is embedded, and a
//! renderer can therefore build DOM nodes without an HTML sink.
//!
//! Media is not resolved here. The v0.1 resource format has no asset reference
//! field and the distribution reader rejects files that no entity references,
//! so a caller cannot ask this module for a package path outside the verified
//! distribution file map.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::Serialize;

/// Markdown input longer than this is rejected instead of rendered.
pub const MAX_MARKDOWN_BYTES: usize = 1024 * 1024;
/// Maximum number of emitted structural blocks per document.
pub const MAX_BLOCKS: usize = 4096;
/// Maximum inline/block nesting accepted from the parser.
const MAX_DEPTH: usize = 32;

/// One structural block of a rendered document.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Block {
    Heading {
        level: u8,
        spans: Vec<Span>,
    },
    Paragraph {
        spans: Vec<Span>,
    },
    List {
        ordered: bool,
        /// Tight lists are rendered without inter-item paragraph breaks.
        tight: bool,
        items: Vec<ListItem>,
    },
    BlockQuote {
        blocks: Vec<Block>,
    },
    Code {
        /// Author-declared info string, reduced to a conservative identifier.
        language: Option<String>,
        text: String,
    },
    /// A block-level HTML run, kept as literal text. It is never markup.
    Html {
        text: String,
    },
    Rule,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ListItem {
    pub blocks: Vec<Block>,
}

/// One inline run. Links carry a pre-classified destination so a renderer never
/// has to interpret a package-supplied URL itself.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Span {
    Text {
        text: String,
    },
    Code {
        text: String,
    },
    Emphasis {
        spans: Vec<Span>,
    },
    Strong {
        spans: Vec<Span>,
    },
    /// `url` is empty for an in-package relative target, whose lexical form
    /// stays in `href`. A rejected destination never becomes a `Link`.
    Link {
        url: String,
        href: String,
        spans: Vec<Span>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Content {
    pub blocks: Vec<Block>,
}

impl Content {
    /// Plain text of the whole document, for search and ad-hoc display.
    pub fn to_plain_text(&self) -> String {
        let mut output = String::new();
        blocks_text(&self.blocks, &mut output);
        output.trim().to_owned()
    }
}

fn blocks_text(blocks: &[Block], output: &mut String) {
    for block in blocks {
        match block {
            Block::Heading { spans, .. } | Block::Paragraph { spans } => {
                spans_text(spans, output);
                output.push('\n');
            }
            Block::List { items, .. } => {
                for item in items {
                    blocks_text(&item.blocks, output);
                }
            }
            Block::BlockQuote { blocks } => blocks_text(blocks, output),
            Block::Code { text, .. } | Block::Html { text } => {
                output.push_str(text);
                output.push('\n');
            }
            Block::Rule => output.push('\n'),
        }
    }
}

fn spans_text(spans: &[Span], output: &mut String) {
    for span in spans {
        match span {
            Span::Text { text } | Span::Code { text } => output.push_str(text),
            Span::Emphasis { spans } | Span::Strong { spans } => spans_text(spans, output),
            Span::Link { spans, .. } => spans_text(spans, output),
        }
    }
}

/// Classify a Markdown link destination.
///
/// Only `https`, `http` and `mailto` are exposed as absolute URLs. A relative
/// target keeps its lexical form only when it is a plain in-package reference
/// with no scheme, host, query or backslash and no dot-segment escape.
/// Everything else, including `javascript:`, `data:`, `file:` and `vbscript:`,
/// is rejected so no renderer can leak it into a sink.
pub fn classify_link(destination: &str) -> Option<(String, String)> {
    let trimmed = destination.trim();
    if trimmed.is_empty() || trimmed.len() > 2048 {
        return None;
    }
    if let Some(scheme_end) = trimmed.find(':') {
        let scheme = &trimmed[..scheme_end];
        let scheme_is_valid = !scheme.is_empty()
            && scheme.bytes().enumerate().all(|(index, byte)| match byte {
                b'a'..=b'z' | b'A'..=b'Z' => true,
                b'0'..=b'9' | b'+' | b'-' | b'.' => index > 0,
                _ => false,
            });
        if scheme_is_valid {
            if !matches!(
                scheme.to_ascii_lowercase().as_str(),
                "http" | "https" | "mailto"
            ) {
                return None;
            }
            if trimmed.contains(char::is_control)
                || trimmed.contains(['<', '>', '"', '\'', '\\', ' '])
            {
                return None;
            }
            return Some((trimmed.to_owned(), trimmed.to_owned()));
        }
    }
    // A relative reference: reject anything that is not a plain portable path.
    if trimmed.starts_with('/')
        || trimmed.contains(char::is_control)
        || trimmed.contains(['\\', '<', '>', '"', '\'', '?', '#', ' '])
        || trimmed.contains("//")
    {
        return None;
    }
    let mut components: Vec<&str> = Vec::new();
    for component in trimmed.split('/') {
        match component {
            "" | "." | ".." => return None,
            other if other.ends_with('.') => return None,
            other => components.push(other),
        }
    }
    if components.is_empty() {
        return None;
    }
    Some((String::new(), trimmed.to_owned()))
}

/// Reduce a fenced-code info string to a conservative language identifier.
fn sanitize_language(info: &str) -> Option<String> {
    let first = info.split_whitespace().next()?;
    if first.is_empty() || first.len() > 32 {
        return None;
    }
    if !first.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'_' | b'#' | b'.')
    }) {
        return None;
    }
    Some(first.to_ascii_lowercase())
}

/// Compile Markdown to inert content IR. Raw HTML stays literal text.
pub fn compile_markdown(markdown: &str) -> Result<Content, String> {
    if markdown.len() > MAX_MARKDOWN_BYTES {
        return Err(format!(
            "markdown exceeds {MAX_MARKDOWN_BYTES} bytes; refusing to render"
        ));
    }
    let mut state = State::new();
    for event in Parser::new_ext(markdown, Options::empty()) {
        state.event(event)?;
    }
    // A tight list item, and a document whose last line has no blank line after
    // it, end with inline text that was never wrapped in a paragraph tag.
    state.flush_inline_paragraph()?;
    if state.depth != 0 || state.frames.len() != 1 || state.code.is_some() || state.sinks.len() != 1
    {
        return Err(format!(
            "markdown ended inside an unclosed construct: depth={} frames={} code={} sinks={}",
            state.depth,
            state.frames.len(),
            state.code.is_some(),
            state.sinks.len()
        ));
    }
    Ok(Content {
        blocks: state.sinks.pop().expect("document sink").blocks,
    })
}

/// An inline container being built. `spans` holds the completed children of
/// this level, so nesting is explicit and never needs unwinding.
struct Frame {
    kind: FrameKind,
    spans: Vec<Span>,
}

enum FrameKind {
    Root,
    Emphasis,
    Strong,
    Link {
        url: String,
        href: String,
    },
    /// A rejected link or an image: the label survives, the target does not.
    LabelOnly,
}

struct CodeState {
    language: Option<String>,
    text: String,
}

/// A structural container receiving finished blocks. One sink per open
/// container makes block placement independent of how deeply markup nests.
struct Sink {
    blocks: Vec<Block>,
    /// For an item sink: the list whose item owns it, so nested lists can
    /// remove exactly their own sink after inner sinks were removed.
    list: Option<usize>,
}

struct ListState {
    ordered: bool,
    items: Vec<ListItem>,
    /// Index of this list's own sink in `State::sinks`.
    sink: usize,
}

struct State {
    frames: Vec<Frame>,
    sinks: Vec<Sink>,
    lists: Vec<ListState>,
    depth: usize,
    count: usize,
    heading: Option<u8>,
    code: Option<CodeState>,
}

impl State {
    fn new() -> Self {
        Self {
            frames: vec![Frame {
                kind: FrameKind::Root,
                spans: Vec::new(),
            }],
            sinks: vec![Sink {
                blocks: Vec::new(),
                list: None,
            }],
            lists: Vec::new(),
            depth: 0,
            count: 0,
            heading: None,
            code: None,
        }
    }

    fn enter(&mut self) -> Result<(), String> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err("markdown nesting exceeds the supported depth".to_owned());
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Hand a finished block to the innermost open container. An open list
    /// always owns the innermost item sink, so a block that completes while a
    /// list is open belongs to that item.
    fn deliver(&mut self, block: Block) -> Result<(), String> {
        self.count += 1;
        if self.count > MAX_BLOCKS {
            return Err("markdown produces too many blocks".to_owned());
        }
        let sink = if let Some(list) = self.lists.last() {
            let _ = list;
            self.item_sink(self.lists.len() - 1)
                .unwrap_or_else(|| self.sinks.len() - 1)
        } else {
            self.sinks.len() - 1
        };
        self.sinks[sink].blocks.push(block);
        Ok(())
    }

    /// Emit any inline text that was never wrapped in a paragraph tag, which
    /// happens for tight list items and for a document's final line.
    fn flush_inline_paragraph(&mut self) -> Result<(), String> {
        let spans = self.take_spans();
        if spans.is_empty() {
            return Ok(());
        }
        self.deliver(Block::Paragraph { spans })
    }

    /// Index of the item sink owned by `list`.
    fn item_sink(&self, list: usize) -> Option<usize> {
        self.sinks.iter().rposition(|sink| sink.list == Some(list))
    }

    fn top_spans(&mut self) -> &mut Vec<Span> {
        &mut self.frames.last_mut().expect("root frame").spans
    }

    fn emit(&mut self, span: Span) {
        self.top_spans().push(span);
    }

    /// Pop the innermost inline frame into its parent.
    fn close_frame(&mut self) {
        if self.frames.len() <= 1 {
            return;
        }
        let frame = self.frames.pop().expect("frame");
        let spans = frame.spans;
        let wrapped = match frame.kind {
            FrameKind::Root | FrameKind::LabelOnly => spans,
            FrameKind::Emphasis => vec![Span::Emphasis { spans }],
            FrameKind::Strong => vec![Span::Strong { spans }],
            FrameKind::Link { url, href } => vec![Span::Link { url, href, spans }],
        };
        self.top_spans().extend(wrapped);
    }

    /// Finish the current inline run and reset the frame stack.
    fn take_spans(&mut self) -> Vec<Span> {
        while self.frames.len() > 1 {
            self.close_frame();
        }
        let mut frame = self.frames.pop().expect("root frame");
        let spans = std::mem::take(&mut frame.spans);
        self.frames.push(Frame {
            kind: FrameKind::Root,
            spans: Vec::new(),
        });
        spans
    }

    fn finish_block(&mut self, block: Block) -> Result<(), String> {
        self.leave();
        self.deliver(block)
    }

    fn event(&mut self, event: Event<'_>) -> Result<(), String> {
        if let Some(code) = self.code.as_mut() {
            match event {
                Event::Text(text) => code.text.push_str(&text),
                Event::End(TagEnd::CodeBlock) => {
                    let code = self.code.take().expect("code state");
                    self.finish_block(Block::Code {
                        language: code.language,
                        text: code.text,
                    })?;
                }
                // A code block carries no other meaningful inline events.
                _ => {}
            }
            return Ok(());
        }
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => {
                self.emit(Span::Text {
                    text: text.into_string(),
                });
                Ok(())
            }
            Event::Code(text) => {
                self.emit(Span::Code {
                    text: text.into_string(),
                });
                Ok(())
            }
            // Inline HTML is never executed or converted to markup: the tags
            // survive only as inert text for the reader to display verbatim.
            Event::InlineHtml(html) => {
                self.emit(Span::Text {
                    text: html.into_string(),
                });
                Ok(())
            }
            // A block-level HTML run becomes an inert block of literal text.
            Event::Html(html) => self.deliver(Block::Html {
                text: html.into_string(),
            }),
            Event::SoftBreak => {
                self.emit(Span::Text {
                    text: " ".to_owned(),
                });
                Ok(())
            }
            Event::HardBreak => {
                self.emit(Span::Text {
                    text: "\n".to_owned(),
                });
                Ok(())
            }
            Event::Rule => self.deliver(Block::Rule),
            Event::FootnoteReference(name) => {
                self.emit(Span::Text {
                    text: format!("[{name}]"),
                });
                Ok(())
            }
            // Math is not rendered in v1; the TeX source is kept as inert text
            // so a later math renderer can consume the same IR.
            Event::InlineMath(tex) | Event::DisplayMath(tex) => {
                self.emit(Span::Code {
                    text: tex.into_string(),
                });
                Ok(())
            }
            Event::TaskListMarker(checked) => {
                self.emit(Span::Text {
                    text: if checked { "[x] " } else { "[ ] " }.to_owned(),
                });
                Ok(())
            }
        }
    }

    fn start(&mut self, tag: Tag<'_>) -> Result<(), String> {
        match tag {
            Tag::Paragraph => self.enter(),
            Tag::Heading { level, .. } => {
                self.enter()?;
                self.heading = Some(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                });
                Ok(())
            }
            Tag::BlockQuote(_) => {
                self.enter()?;
                self.sinks.push(Sink {
                    blocks: Vec::new(),
                    list: None,
                });
                Ok(())
            }
            Tag::CodeBlock(kind) => {
                self.enter()?;
                let language = match kind {
                    CodeBlockKind::Fenced(info) => sanitize_language(&info),
                    CodeBlockKind::Indented => None,
                };
                self.code = Some(CodeState {
                    language,
                    text: String::new(),
                });
                Ok(())
            }
            Tag::List(start) => {
                self.enter()?;
                let ordered = start.is_some();
                // One sink per list marks the list's place in the block tree;
                // while the list is open, blocks land in its item sinks.
                let sink = self.sinks.len();
                self.sinks.push(Sink {
                    blocks: Vec::new(),
                    list: None,
                });
                self.lists.push(ListState {
                    ordered,
                    items: Vec::new(),
                    sink,
                });
                Ok(())
            }
            Tag::Item => {
                self.enter()?;
                let owner = self.lists.len() - 1;
                self.sinks.push(Sink {
                    blocks: Vec::new(),
                    list: Some(owner),
                });
                Ok(())
            }
            Tag::Emphasis => {
                self.enter()?;
                self.frames.push(Frame {
                    kind: FrameKind::Emphasis,
                    spans: Vec::new(),
                });
                Ok(())
            }
            Tag::Strong => {
                self.enter()?;
                self.frames.push(Frame {
                    kind: FrameKind::Strong,
                    spans: Vec::new(),
                });
                Ok(())
            }
            Tag::Link { dest_url, .. } => {
                self.enter()?;
                match classify_link(&dest_url) {
                    Some((url, href)) => self.frames.push(Frame {
                        kind: FrameKind::Link { url, href },
                        spans: Vec::new(),
                    }),
                    None => self.frames.push(Frame {
                        kind: FrameKind::LabelOnly,
                        spans: Vec::new(),
                    }),
                }
                Ok(())
            }
            Tag::Image { .. } => {
                self.enter()?;
                self.frames.push(Frame {
                    kind: FrameKind::LabelOnly,
                    spans: Vec::new(),
                });
                Ok(())
            }
            Tag::FootnoteDefinition(_) => self.enter(),
            // Extensions that the v1 renderer does not model keep their text
            // content and add no structure of their own.
            Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
            | Tag::HtmlBlock
            | Tag::MetadataBlock(_)
            | Tag::Strikethrough
            | Tag::Superscript
            | Tag::Subscript => self.enter(),
        }
    }

    fn end(&mut self, tag: TagEnd) -> Result<(), String> {
        match tag {
            TagEnd::Paragraph => {
                let spans = self.take_spans();
                if spans.is_empty() {
                    self.leave();
                    Ok(())
                } else {
                    self.finish_block(Block::Paragraph { spans })
                }
            }
            TagEnd::Heading(_) => {
                let level = self.heading.take().unwrap_or(1);
                let spans = self.take_spans();
                self.finish_block(Block::Heading { level, spans })
            }
            TagEnd::BlockQuote(_) => {
                self.leave();
                self.flush_inline_paragraph()?;
                let sink = self.sinks.pop().expect("quote sink");
                let blocks = sink.blocks;
                if blocks.is_empty() {
                    return Ok(());
                }
                self.deliver(Block::BlockQuote { blocks })
            }
            TagEnd::CodeBlock => Ok(()),
            TagEnd::List(tight) => {
                self.leave();
                self.flush_inline_paragraph()?;
                let list = self.lists.pop().expect("list state");
                let _own_sink = self.sinks.remove(list.sink);
                self.deliver(Block::List {
                    ordered: list.ordered,
                    tight,
                    items: list.items,
                })
            }
            TagEnd::Item => {
                self.leave();
                self.flush_inline_paragraph()?;
                let owner = self.lists.len() - 1;
                let item_sink = self.item_sink(owner).expect("open item sink");
                let sink = self.sinks.remove(item_sink);
                if let Some(list) = self.lists.last_mut() {
                    list.items.push(ListItem {
                        blocks: sink.blocks,
                    });
                }
                Ok(())
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Link | TagEnd::Image => {
                self.leave();
                self.close_frame();
                Ok(())
            }
            TagEnd::FootnoteDefinition
            | TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition
            | TagEnd::Table
            | TagEnd::TableHead
            | TagEnd::TableRow
            | TagEnd::TableCell
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript => {
                self.leave();
                Ok(())
            }
            TagEnd::HtmlBlock | TagEnd::MetadataBlock(_) => {
                self.leave();
                Ok(())
            }
        }
    }
}
