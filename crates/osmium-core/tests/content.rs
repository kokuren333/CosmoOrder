//! Content IR contract: structure is preserved, and nothing package-supplied
//! can become markup, script or an outbound navigation target.

use osmium_core::content::{
    Block, Content, MAX_MARKDOWN_BYTES, Span, classify_link, compile_markdown,
};

fn compile(markdown: &str) -> Content {
    compile_markdown(markdown).expect("markdown compiles")
}

fn paragraph_spans(content: &Content) -> &Vec<Span> {
    match &content.blocks[0] {
        Block::Paragraph { spans } => spans,
        other => panic!("expected a paragraph, found {other:?}"),
    }
}

fn flatten(spans: &[Span], output: &mut String) {
    for span in spans {
        match span {
            Span::Text { text } | Span::Code { text } => output.push_str(text),
            Span::Emphasis { spans } | Span::Strong { spans } | Span::Strikethrough { spans } => {
                flatten(spans, output)
            }
            Span::Math { tex, .. } => output.push_str(tex),
            Span::Link { spans, .. } => flatten(spans, output),
        }
    }
}

fn block_text(block: &Block, output: &mut String) {
    match block {
        Block::Heading { spans, .. } | Block::Paragraph { spans } => flatten(spans, output),
        Block::Code { text, .. } | Block::Html { text } => output.push_str(text),
        Block::Math { tex } => output.push_str(tex),
        Block::Table { headers, rows } => {
            for cell in headers.iter().chain(rows.iter().flatten()) {
                flatten(cell, output);
            }
        }
        Block::List { items, .. } => {
            for item in items {
                for block in &item.blocks {
                    block_text(block, output);
                }
            }
        }
        Block::BlockQuote { blocks } => {
            for block in blocks {
                block_text(block, output);
            }
        }
        Block::Rule => {}
    }
}

#[test]
fn compiles_tables_strikethrough_inline_and_display_math_to_typed_ir() {
    let content = compile(
        "| Term | Meaning |\n|---|---|\n| **A** | ~~old~~\n\nInline $x^2$\n\n$$\n\\frac{1}{2}\n$$\n",
    );
    let Block::Table { headers, rows } = &content.blocks[0] else {
        panic!("table IR expected: {:?}", content.blocks);
    };
    assert_eq!(headers.len(), 2);
    assert_eq!(rows.len(), 1);
    assert!(matches!(rows[0][1][0], Span::Strikethrough { .. }));
    let Block::Paragraph { spans } = &content.blocks[1] else {
        panic!("paragraph expected");
    };
    assert!(
        spans
            .iter()
            .any(|span| matches!(span, Span::Math { tex, display: false } if tex == "x^2"))
    );
    assert!(matches!(&content.blocks[2], Block::Math { tex } if tex.contains("frac")));
}

fn document_text(content: &Content) -> String {
    let mut text = String::new();
    for block in &content.blocks {
        block_text(block, &mut text);
        text.push('\n');
    }
    text
}

#[test]
fn compiles_headings_paragraphs_lists_and_code() {
    let content = compile(
        "# Title\n\nIntro **bold** and *em* and `code`.\n\n## Steps\n\n1. first\n2. second\n\n- a\n  - nested\n\n```rust\nfn main() {}\n```\n\n---\n\n> quoted\n",
    );
    assert!(matches!(content.blocks[0], Block::Heading { level: 1, .. }));
    assert!(matches!(content.blocks[1], Block::Paragraph { .. }));
    assert!(matches!(content.blocks[2], Block::Heading { level: 2, .. }));
    let Block::List {
        ordered,
        items,
        tight,
    } = &content.blocks[3]
    else {
        panic!("expected an ordered list, found {:?}", content.blocks[3]);
    };
    assert!(*ordered);
    assert!(*tight);
    assert_eq!(items.len(), 2);
    let mut item_text = String::new();
    for item in items {
        for block in &item.blocks {
            block_text(block, &mut item_text);
        }
    }
    assert_eq!(item_text, "firstsecond");
    let Block::List {
        ordered: nested_ordered,
        items: nested_items,
        ..
    } = &content.blocks[4]
    else {
        panic!("expected a bullet list, found {:?}", content.blocks[4]);
    };
    assert!(!*nested_ordered);
    assert_eq!(nested_items.len(), 1);
    assert!(
        nested_items[0]
            .blocks
            .iter()
            .any(|block| matches!(block, Block::List { .. })),
        "a nested list stays nested inside its item: {:?}",
        nested_items[0].blocks
    );
    let Block::Code { language, text } = &content.blocks[5] else {
        panic!("expected a code block, found {:?}", content.blocks[5]);
    };
    assert_eq!(language.as_deref(), Some("rust"));
    assert_eq!(text, "fn main() {}\n");
    assert_eq!(content.blocks[6], Block::Rule);
    let Block::BlockQuote { blocks } = &content.blocks[7] else {
        panic!("expected a block quote, found {:?}", content.blocks[7]);
    };
    let mut quoted = String::new();
    for block in blocks {
        block_text(block, &mut quoted);
    }
    assert_eq!(quoted, "quoted");
}

#[test]
fn emphasis_and_strong_nest_correctly() {
    let content = compile("a **b *c* d** e");
    let spans = paragraph_spans(&content);
    let mut text = String::new();
    flatten(spans, &mut text);
    assert_eq!(text, "a b c d e");
    let strong = spans
        .iter()
        .find_map(|span| match span {
            Span::Strong { spans } => Some(spans),
            _ => None,
        })
        .expect("strong span");
    assert!(
        strong
            .iter()
            .any(|span| matches!(span, Span::Emphasis { .. }))
    );
}

#[test]
fn raw_html_never_becomes_markup() {
    let content = compile("<script>alert(1)</script>\n\nnormal <b>bold</b> text\n");
    let text = document_text(&content);
    assert!(
        text.contains("<script>alert(1)</script>"),
        "block HTML survives as literal text: {text}"
    );
    assert!(
        text.contains("<b>bold</b>"),
        "inline HTML survives as literal text: {text}"
    );
    // HTML produced no markup structure: a block HTML run is its own inert
    // block and inline HTML only ever becomes text spans.
    assert!(
        content
            .blocks
            .iter()
            .any(|block| matches!(block, Block::Html { .. }))
    );
    for block in &content.blocks {
        if let Block::Paragraph { spans } = block {
            assert!(spans.iter().all(|span| matches!(span, Span::Text { .. })));
        }
    }
}

#[test]
fn unsafe_link_targets_are_rejected() {
    for destination in [
        "javascript:alert(1)",
        "JavaScript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "file:///C:/Windows/System32",
        "vbscript:msgbox(1)",
        "//evil.example/x",
        "../../outside.md",
        "/absolute/path.md",
        "#fragment",
        "content/../outside.md",
    ] {
        assert!(
            classify_link(destination).is_none(),
            "{destination} must be rejected"
        );
    }
    let content = compile("[click](javascript:alert(1))");
    let spans = paragraph_spans(&content);
    assert!(
        spans.iter().all(|span| !matches!(span, Span::Link { .. })),
        "a rejected destination must not survive as a link: {spans:?}"
    );
    let mut text = String::new();
    flatten(spans, &mut text);
    assert_eq!(text, "click");
}

#[test]
fn safe_links_keep_a_classified_destination() {
    let content = compile("[docs](https://example.test/a) and [inside](content/other.md)");
    let spans = paragraph_spans(&content);
    let links: Vec<&Span> = spans
        .iter()
        .filter(|span| matches!(span, Span::Link { .. }))
        .collect();
    assert_eq!(links.len(), 2);
    let Span::Link { url, href, .. } = links[0] else {
        unreachable!()
    };
    assert_eq!(url, "https://example.test/a");
    assert_eq!(href, "https://example.test/a");
    let Span::Link { url, href, .. } = links[1] else {
        unreachable!()
    };
    assert!(url.is_empty(), "an in-package link is not an absolute URL");
    assert_eq!(href, "content/other.md");
}

#[test]
fn images_keep_alt_text_only() {
    let content = compile("before ![alt text](https://example.test/tracker.png) after");
    let spans = paragraph_spans(&content);
    let mut text = String::new();
    flatten(spans, &mut text);
    assert_eq!(text, "before alt text after");
    assert!(spans.iter().all(|span| !matches!(span, Span::Link { .. })));
}

#[test]
fn plain_text_export_matches_the_document() {
    let content = compile("# 足し算\n\n1個と1個で **2個** です。\n");
    assert_eq!(content.to_plain_text(), "足し算\n1個と1個で 2個 です。");
}

#[test]
fn oversized_markdown_is_rejected() {
    let oversized = "a".repeat(MAX_MARKDOWN_BYTES + 1);
    assert!(compile_markdown(&oversized).is_err());
}

#[test]
fn excessive_table_cells_are_rejected_within_the_document_limit() {
    let mut markdown = String::from("| A | B |\n|---|---|\n");
    for _ in 0..MAX_MARKDOWN_BYTES / 16 {
        markdown.push_str("| x | y |\n");
    }
    assert!(compile_markdown(&markdown).is_err());
}

#[test]
fn golden_lesson_compiles_to_expected_structure() {
    let markdown = include_str!("../../../examples/arithmetic/content/introduction.md");
    let content = compile(markdown);
    assert!(matches!(content.blocks[0], Block::Heading { level: 1, .. }));
    assert!(matches!(content.blocks[1], Block::Paragraph { .. }));
    let text = content.to_plain_text();
    assert!(
        text.contains("足し算"),
        "plain text keeps the lesson: {text}"
    );
    assert!(text.contains("2個"));
}

#[test]
fn cross_domain_resources_compile_to_renderer_neutral_structures() {
    let medicine = compile(include_str!(
        "../../../examples/medicine-pressure-test/content/fluid.md"
    ));
    assert!(
        medicine
            .blocks
            .iter()
            .any(|block| matches!(block, Block::Table { .. }))
    );

    let mathematics = compile(include_str!(
        "../../../examples/mathematics-pressure-test/content/conditional.md"
    ));
    assert!(
        mathematics
            .blocks
            .iter()
            .any(|block| matches!(block, Block::Math { .. }))
    );
    assert!(mathematics.blocks.iter().any(|block| matches!(block, Block::Paragraph { spans } if spans.iter().any(|span| matches!(span, Span::Math { display: false, .. })))));

    let language = compile(include_str!(
        "../../../examples/language-pressure-test/content/politeness.md"
    ));
    assert!(language.to_plain_text().contains("駅への行き方"));
    assert!(language.blocks.iter().any(|block| matches!(block, Block::Paragraph { spans } if spans.iter().any(|span| matches!(span, Span::Strong { .. })))));

    let programming = compile(include_str!(
        "../../../examples/programming-pressure-test/content/values.md"
    ));
    assert!(programming.blocks.iter().any(|block| matches!(block, Block::Code { language: Some(language), .. } if language == "python")));
}
