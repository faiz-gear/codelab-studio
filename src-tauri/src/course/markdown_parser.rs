use pulldown_cmark::{CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::course::ast::{CourseChapterRaw, CourseDocumentRaw, CourseListItem, CourseNode};
use crate::course::ast_text::{
    append_node_plain_text, collect_plain_text, extract_fence_language, sanitize_heading_id,
};
use crate::course::html_sanitize::{
    classify_inline_html_tag, sanitize_html_block_to_tokens, sanitize_image_src,
    sanitize_link_href, sanitize_to_plain_text, AllowedTag, HtmlToken, SingleTagAction,
    SuppressedTag,
};

pub(crate) fn parse_raw_markdown(source: &str) -> CourseDocumentRaw {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let parser = Parser::new_ext(source, options);
    let mut builder = AstBuilder::new();
    for event in parser {
        builder.handle_event(event);
    }
    builder.finish()
}

#[derive(Debug)]
enum Frame {
    Paragraph {
        children: Vec<CourseNode>,
    },
    Heading {
        level: u8,
        raw_id: Option<String>,
        is_doc_title: bool,
        starts_chapter: bool,
        children: Vec<CourseNode>,
    },
    BlockQuote {
        children: Vec<CourseNode>,
    },
    CodeBlock {
        info_string: String,
        language: Option<String>,
        code: String,
    },
    HtmlBlock {
        raw: String,
    },
    List {
        ordered: bool,
        start: Option<u32>,
        items: Vec<CourseListItem>,
    },
    ListItem {
        children: Vec<CourseNode>,
        task_checked: Option<bool>,
    },
    Table {
        headers: Vec<Vec<CourseNode>>,
        rows: Vec<Vec<Vec<CourseNode>>>,
    },
    TableHead {
        cells: Vec<Vec<CourseNode>>,
    },
    TableRow {
        cells: Vec<Vec<CourseNode>>,
    },
    TableCell {
        children: Vec<CourseNode>,
    },
    Emphasis {
        children: Vec<CourseNode>,
    },
    Strong {
        children: Vec<CourseNode>,
    },
    Strikethrough {
        children: Vec<CourseNode>,
    },
    Link {
        href: Option<String>,
        title: Option<String>,
        children: Vec<CourseNode>,
    },
    Image {
        src: Option<String>,
        title: Option<String>,
        alt: String,
    },
    Kbd {
        label: String,
    },
    Superscript {
        children: Vec<CourseNode>,
    },
    Subscript {
        children: Vec<CourseNode>,
    },
    Details {
        summary: String,
        children: Vec<CourseNode>,
    },
    Summary {
        text: String,
    },
}

struct AstBuilder {
    doc_title: Option<String>,
    intro_nodes: Vec<CourseNode>,
    chapters: Vec<CourseChapterRaw>,
    frames: Vec<Frame>,
    suppressed_tag: Option<SuppressedTag>,
    suppressed_block_depth: usize,
}

impl AstBuilder {
    fn new() -> Self {
        Self {
            doc_title: None,
            intro_nodes: Vec::new(),
            chapters: Vec::new(),
            frames: Vec::new(),
            suppressed_tag: None,
            suppressed_block_depth: 0,
        }
    }

    fn finish(mut self) -> CourseDocumentRaw {
        self.suppressed_tag = None;
        self.suppressed_block_depth = 0;
        while let Some(frame) = self.frames.pop() {
            self.finish_frame(frame);
        }
        CourseDocumentRaw {
            title: self.doc_title,
            intro_nodes: self.intro_nodes,
            chapters: self.chapters,
        }
    }

    fn current_nodes_mut(&mut self) -> &mut Vec<CourseNode> {
        if let Some(chapter) = self.chapters.last_mut() {
            &mut chapter.nodes
        } else {
            &mut self.intro_nodes
        }
    }

    fn only_details_or_empty(&self) -> bool {
        self.frames
            .iter()
            .all(|frame| matches!(frame, Frame::Details { .. }))
    }

    fn open_details_depth(&self) -> usize {
        self.frames
            .iter()
            .filter(|frame| matches!(frame, Frame::Details { .. }))
            .count()
    }

    fn is_at_block_level(&self) -> bool {
        matches!(
            self.frames.last(),
            None | Some(Frame::Details { .. })
                | Some(Frame::BlockQuote { .. })
                | Some(Frame::ListItem { .. })
        )
    }

    fn handle_event(&mut self, event: Event<'_>) {
        if let Some(active_suppressed) = self.suppressed_tag {
            match event {
                Event::InlineHtml(raw) => {
                    if let SingleTagAction::Token(HtmlToken::CloseSuppressed(closed)) =
                        classify_inline_html_tag(&raw)
                    {
                        if closed == active_suppressed {
                            self.suppressed_tag = None;
                            self.suppressed_block_depth = 0;
                        }
                    }
                    return;
                }
                Event::Start(tag) => {
                    if is_block_tag(&tag) {
                        self.suppressed_block_depth += 1;
                    }
                    return;
                }
                Event::End(tag_end) => {
                    if is_block_tag_end(&tag_end) {
                        if self.suppressed_block_depth > 0 {
                            self.suppressed_block_depth -= 1;
                            return;
                        }
                        self.suppressed_tag = None;
                        self.handle_end(tag_end);
                    }
                    return;
                }
                _ => return,
            }
        }

        match event {
            Event::Start(tag) => self.handle_start(tag),
            Event::End(tag_end) => self.handle_end(tag_end),
            Event::Text(text) => {
                if let Some(Frame::CodeBlock { code, .. }) = self.frames.last_mut() {
                    code.push_str(&text);
                } else if let Some(Frame::HtmlBlock { raw }) = self.frames.last_mut() {
                    raw.push_str(&text);
                } else {
                    self.push_node(CourseNode::Text {
                        value: text.into_string(),
                    });
                }
            }
            Event::Code(code) => self.push_node(CourseNode::InlineCode {
                value: code.into_string(),
            }),
            Event::SoftBreak => {
                if let Some(Frame::CodeBlock { code, .. }) = self.frames.last_mut() {
                    code.push('\n');
                } else {
                    self.push_node(CourseNode::Text {
                        value: " ".to_owned(),
                    });
                }
            }
            Event::HardBreak => self.push_node(CourseNode::LineBreak),
            Event::Rule => self.push_node(CourseNode::ThematicBreak),
            Event::TaskListMarker(checked) => {
                for frame in self.frames.iter_mut().rev() {
                    if let Frame::ListItem { task_checked, .. } = frame {
                        *task_checked = Some(checked);
                        break;
                    }
                }
            }
            Event::Html(html) => {
                if let Some(Frame::HtmlBlock { raw }) = self.frames.last_mut() {
                    raw.push_str(&html);
                } else {
                    self.process_html_block(&html);
                }
            }
            Event::InlineHtml(html) => self.process_inline_html(&html),
            Event::InlineMath(math) | Event::DisplayMath(math) | Event::FootnoteReference(math) => {
                self.push_node(CourseNode::Text {
                    value: math.into_string(),
                });
            }
        }
    }

    fn handle_start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.frames.push(Frame::Paragraph {
                children: Vec::new(),
            }),
            Tag::Heading { level, id, .. } => {
                let level_u8 = heading_level_to_u8(level);
                let starts_chapter = level_u8 == 2 && self.only_details_or_empty();
                if starts_chapter {
                    self.close_all_details_frames();
                }
                let is_doc_title = level_u8 == 1
                    && self.frames.is_empty()
                    && self.doc_title.is_none()
                    && self.chapters.is_empty();
                self.frames.push(Frame::Heading {
                    level: level_u8,
                    raw_id: id.map(CowStr::into_string),
                    is_doc_title,
                    starts_chapter,
                    children: Vec::new(),
                });
            }
            Tag::BlockQuote(_) => self.frames.push(Frame::BlockQuote {
                children: Vec::new(),
            }),
            Tag::CodeBlock(kind) => {
                let info_string = match kind {
                    CodeBlockKind::Fenced(info) => info.trim().to_owned(),
                    CodeBlockKind::Indented => String::new(),
                };
                let language = extract_fence_language(&info_string);
                self.frames.push(Frame::CodeBlock {
                    info_string,
                    language,
                    code: String::new(),
                });
            }
            Tag::HtmlBlock => self.frames.push(Frame::HtmlBlock { raw: String::new() }),
            Tag::List(start) => self.frames.push(Frame::List {
                ordered: start.is_some(),
                start: start.and_then(|n| u32::try_from(n).ok()),
                items: Vec::new(),
            }),
            Tag::Item => self.frames.push(Frame::ListItem {
                children: Vec::new(),
                task_checked: None,
            }),
            Tag::Table(_) => self.frames.push(Frame::Table {
                headers: Vec::new(),
                rows: Vec::new(),
            }),
            Tag::TableHead => self.frames.push(Frame::TableHead { cells: Vec::new() }),
            Tag::TableRow => self.frames.push(Frame::TableRow { cells: Vec::new() }),
            Tag::TableCell => self.frames.push(Frame::TableCell {
                children: Vec::new(),
            }),
            Tag::Emphasis => self.frames.push(Frame::Emphasis {
                children: Vec::new(),
            }),
            Tag::Strong => self.frames.push(Frame::Strong {
                children: Vec::new(),
            }),
            Tag::Strikethrough => self.frames.push(Frame::Strikethrough {
                children: Vec::new(),
            }),
            Tag::Superscript => self.frames.push(Frame::Superscript {
                children: Vec::new(),
            }),
            Tag::Subscript => self.frames.push(Frame::Subscript {
                children: Vec::new(),
            }),
            Tag::Link {
                dest_url, title, ..
            } => self.frames.push(Frame::Link {
                href: sanitize_link_href(&dest_url),
                title: non_empty_trimmed(&title),
                children: Vec::new(),
            }),
            Tag::Image {
                dest_url, title, ..
            } => self.frames.push(Frame::Image {
                src: sanitize_image_src(&dest_url),
                title: non_empty_trimmed(&title),
                alt: String::new(),
            }),
            _ => {}
        }
    }

    fn handle_end(&mut self, tag_end: TagEnd) {
        let Some(target_idx) = self
            .frames
            .iter()
            .rposition(|frame| frame_matches_tag_end(frame, &tag_end))
        else {
            return;
        };

        while self.frames.len() > target_idx + 1 {
            if let Some(unclosed) = self.frames.pop() {
                self.finish_frame(unclosed);
            }
        }

        if let Some(frame) = self.frames.pop() {
            self.finish_frame(frame);
        }
    }

    fn finish_frame(&mut self, frame: Frame) {
        match frame {
            Frame::HtmlBlock { raw } => self.process_html_block(&raw),
            Frame::CodeBlock {
                info_string,
                language,
                code,
            } => {
                let trimmed_code = strip_single_trailing_newline(code);
                self.push_node(CourseNode::CodeBlock {
                    info_string,
                    language,
                    code: trimmed_code,
                });
            }
            Frame::Paragraph { children } if !children.is_empty() => {
                self.push_node(CourseNode::Paragraph { children });
            }
            Frame::Paragraph { .. } => {}
            Frame::Heading {
                level,
                raw_id,
                is_doc_title,
                starts_chapter,
                children,
            } => {
                let plain = collect_plain_text(&children).trim().to_owned();
                if is_doc_title {
                    if !plain.is_empty() {
                        self.doc_title = Some(plain);
                    }
                } else if starts_chapter {
                    self.chapters.push(CourseChapterRaw {
                        raw_id_attr: raw_id,
                        title: plain,
                        nodes: Vec::new(),
                    });
                } else {
                    let id = raw_id.as_deref().and_then(sanitize_heading_id);
                    self.push_node(CourseNode::Heading {
                        level,
                        id,
                        children,
                    });
                }
            }
            Frame::BlockQuote { children } => {
                self.push_node(CourseNode::BlockQuote { children });
            }
            Frame::List {
                ordered,
                start,
                items,
            } => {
                self.push_node(CourseNode::List {
                    ordered,
                    start,
                    items,
                });
            }
            Frame::ListItem {
                children,
                task_checked,
            } => {
                if let Some(Frame::List { items, .. }) = self.frames.last_mut() {
                    items.push(CourseListItem {
                        task_checked,
                        nodes: children,
                    });
                }
            }
            Frame::TableCell { children } => match self.frames.last_mut() {
                Some(Frame::TableHead { cells }) | Some(Frame::TableRow { cells }) => {
                    cells.push(children);
                }
                _ => {}
            },
            Frame::TableHead { cells } => {
                if let Some(Frame::Table { headers, .. }) = self.frames.last_mut() {
                    *headers = cells;
                }
            }
            Frame::TableRow { cells } => {
                if let Some(Frame::Table { rows, .. }) = self.frames.last_mut() {
                    rows.push(cells);
                }
            }
            Frame::Table { headers, rows } => {
                self.push_node(CourseNode::Table { headers, rows });
            }
            Frame::Emphasis { children } => self.push_node(CourseNode::Emphasis { children }),
            Frame::Strong { children } => self.push_node(CourseNode::Strong { children }),
            Frame::Strikethrough { children } => {
                for child in children {
                    self.push_node(child);
                }
            }
            Frame::Superscript { children } if !children.is_empty() => {
                self.push_node(CourseNode::Superscript { children });
            }
            Frame::Subscript { children } if !children.is_empty() => {
                self.push_node(CourseNode::Subscript { children });
            }
            Frame::Superscript { .. } | Frame::Subscript { .. } => {}
            Frame::Kbd { label } => {
                let cleaned = label.trim().to_owned();
                if !cleaned.is_empty() {
                    self.push_node(CourseNode::Kbd { label: cleaned });
                }
            }
            Frame::Summary { text } => {
                if let Some(Frame::Details { summary, .. }) = self.frames.last_mut() {
                    *summary = text.trim().to_owned();
                }
            }
            Frame::Details { summary, children } => {
                self.push_node(CourseNode::Details { summary, children });
            }
            Frame::Link {
                href,
                title,
                children,
            } => {
                if let Some(safe_href) = href {
                    self.push_node(CourseNode::Link {
                        href: safe_href,
                        title,
                        children,
                    });
                } else {
                    for child in children {
                        self.push_node(child);
                    }
                }
            }
            Frame::Image { src, title, alt } => {
                if let Some(safe_src) = src {
                    self.push_node(CourseNode::Image {
                        src: safe_src,
                        alt,
                        title,
                    });
                } else if !alt.trim().is_empty() {
                    self.push_node(CourseNode::Text { value: alt });
                }
            }
        }
    }

    fn process_inline_html(&mut self, raw: &str) {
        let mut synthetic_paragraph_open = false;
        match classify_inline_html_tag(raw) {
            SingleTagAction::Token(token) => {
                self.apply_html_token(token, false, &mut synthetic_paragraph_open);
            }
            SingleTagAction::Ignore => {}
            SingleTagAction::FallbackToSanitizer => {
                let plain = sanitize_to_plain_text(raw);
                if !plain.is_empty() {
                    self.push_node(CourseNode::Text { value: plain });
                }
            }
        }
    }

    fn process_html_block(&mut self, raw: &str) {
        let tokens = sanitize_html_block_to_tokens(raw, self.open_details_depth());
        let mut synthetic_paragraph_open = false;
        for token in tokens {
            self.apply_html_token(token, true, &mut synthetic_paragraph_open);
        }
        self.close_synthetic_paragraph(&mut synthetic_paragraph_open);
    }

    fn apply_html_token(
        &mut self,
        token: HtmlToken,
        wrap_block_inlines: bool,
        synthetic_paragraph_open: &mut bool,
    ) {
        match token {
            HtmlToken::OpenSuppressed(tag) => {
                self.suppressed_tag = Some(tag);
                self.suppressed_block_depth = 0;
            }
            HtmlToken::CloseSuppressed(_) => {}
            HtmlToken::LineBreak => {
                if wrap_block_inlines {
                    self.ensure_inline_container(synthetic_paragraph_open);
                }
                self.push_node(CourseNode::LineBreak);
            }
            HtmlToken::Open(AllowedTag::Details) => {
                self.close_synthetic_paragraph(synthetic_paragraph_open);
                self.frames.push(Frame::Details {
                    summary: String::new(),
                    children: Vec::new(),
                });
            }
            HtmlToken::Close(AllowedTag::Details) => {
                self.close_synthetic_paragraph(synthetic_paragraph_open);
                self.close_details_frame();
            }
            HtmlToken::Open(AllowedTag::Summary) => {
                self.close_synthetic_paragraph(synthetic_paragraph_open);
                if matches!(self.frames.last(), Some(Frame::Details { .. })) {
                    self.frames.push(Frame::Summary {
                        text: String::new(),
                    });
                }
            }
            HtmlToken::Close(AllowedTag::Summary) => {
                self.close_nearest_html_frame(|f| matches!(f, Frame::Summary { .. }));
            }
            HtmlToken::Open(AllowedTag::Kbd) => {
                if wrap_block_inlines {
                    self.ensure_inline_container(synthetic_paragraph_open);
                }
                self.frames.push(Frame::Kbd {
                    label: String::new(),
                });
            }
            HtmlToken::Close(AllowedTag::Kbd) => {
                self.close_nearest_html_frame(|f| matches!(f, Frame::Kbd { .. }));
            }
            HtmlToken::Open(AllowedTag::Sup) => {
                if wrap_block_inlines {
                    self.ensure_inline_container(synthetic_paragraph_open);
                }
                self.frames.push(Frame::Superscript {
                    children: Vec::new(),
                });
            }
            HtmlToken::Close(AllowedTag::Sup) => {
                self.close_nearest_html_frame(|f| matches!(f, Frame::Superscript { .. }));
            }
            HtmlToken::Open(AllowedTag::Sub) => {
                if wrap_block_inlines {
                    self.ensure_inline_container(synthetic_paragraph_open);
                }
                self.frames.push(Frame::Subscript {
                    children: Vec::new(),
                });
            }
            HtmlToken::Close(AllowedTag::Sub) => {
                self.close_nearest_html_frame(|f| matches!(f, Frame::Subscript { .. }));
            }
            HtmlToken::Text(text) => {
                self.emit_sanitized_text(text, wrap_block_inlines, synthetic_paragraph_open);
            }
        }
    }

    fn emit_sanitized_text(
        &mut self,
        text: String,
        wrap_block_inlines: bool,
        synthetic_paragraph_open: &mut bool,
    ) {
        if text.is_empty() {
            return;
        }
        if matches!(
            self.frames.last(),
            Some(Frame::Summary { .. }) | Some(Frame::Kbd { .. })
        ) {
            self.push_node(CourseNode::Text { value: text });
            return;
        }
        if wrap_block_inlines && self.is_at_block_level() {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return;
            }
            self.ensure_inline_container(synthetic_paragraph_open);
            self.push_node(CourseNode::Text {
                value: trimmed.to_owned(),
            });
        } else {
            self.push_node(CourseNode::Text { value: text });
        }
    }

    fn ensure_inline_container(&mut self, synthetic_paragraph_open: &mut bool) {
        if self.is_at_block_level() {
            self.frames.push(Frame::Paragraph {
                children: Vec::new(),
            });
            *synthetic_paragraph_open = true;
        }
    }

    fn close_synthetic_paragraph(&mut self, synthetic_paragraph_open: &mut bool) {
        if *synthetic_paragraph_open {
            if let Some(target_idx) = self
                .frames
                .iter()
                .rposition(|f| matches!(f, Frame::Paragraph { .. }))
            {
                while self.frames.len() > target_idx {
                    if let Some(frame) = self.frames.pop() {
                        self.finish_frame(frame);
                    }
                }
            }
            *synthetic_paragraph_open = false;
        }
    }

    fn close_nearest_html_frame(&mut self, predicate: impl Fn(&Frame) -> bool) {
        let Some(target_idx) = self.frames.iter().rposition(&predicate) else {
            return;
        };
        let only_html_above = self.frames[target_idx + 1..]
            .iter()
            .all(is_inline_html_frame);
        if !only_html_above {
            return;
        }
        while self.frames.len() > target_idx {
            if let Some(frame) = self.frames.pop() {
                self.finish_frame(frame);
            }
        }
    }

    fn close_details_frame(&mut self) {
        let Some(details_idx) = self
            .frames
            .iter()
            .rposition(|f| matches!(f, Frame::Details { .. }))
        else {
            return;
        };
        let can_unwind_to_details = self.frames[details_idx + 1..].iter().all(|f| {
            is_inline_html_frame(f)
                || matches!(
                    f,
                    Frame::Paragraph { .. }
                        | Frame::Emphasis { .. }
                        | Frame::Strong { .. }
                        | Frame::Strikethrough { .. }
                )
        });
        if !can_unwind_to_details {
            return;
        }
        while self.frames.len() > details_idx {
            if let Some(frame) = self.frames.pop() {
                self.finish_frame(frame);
            }
        }
    }

    fn close_all_details_frames(&mut self) {
        while matches!(self.frames.last(), Some(Frame::Details { .. })) {
            if let Some(frame) = self.frames.pop() {
                self.finish_frame(frame);
            }
        }
    }

    fn push_node(&mut self, node: CourseNode) {
        match self.frames.last_mut() {
            Some(
                Frame::Paragraph { children }
                | Frame::Heading { children, .. }
                | Frame::BlockQuote { children }
                | Frame::ListItem { children, .. }
                | Frame::TableCell { children }
                | Frame::Emphasis { children }
                | Frame::Strong { children }
                | Frame::Strikethrough { children }
                | Frame::Link { children, .. }
                | Frame::Superscript { children }
                | Frame::Subscript { children }
                | Frame::Details { children, .. },
            ) => push_coalesced(children, node),
            Some(Frame::Image { alt, .. }) => append_node_plain_text(&node, alt),
            Some(Frame::Kbd { label }) => append_node_plain_text(&node, label),
            Some(Frame::Summary { text }) => append_node_plain_text(&node, text),
            None => push_coalesced(self.current_nodes_mut(), node),
            Some(
                Frame::CodeBlock { .. }
                | Frame::HtmlBlock { .. }
                | Frame::List { .. }
                | Frame::Table { .. }
                | Frame::TableHead { .. }
                | Frame::TableRow { .. },
            ) => {}
        }
    }
}

fn is_inline_html_frame(frame: &Frame) -> bool {
    matches!(
        frame,
        Frame::Kbd { .. }
            | Frame::Superscript { .. }
            | Frame::Subscript { .. }
            | Frame::Summary { .. }
    )
}

fn is_block_tag(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::BlockQuote(_)
            | Tag::CodeBlock(_)
            | Tag::HtmlBlock
            | Tag::List(_)
            | Tag::Item
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
    )
}

fn is_block_tag_end(tag_end: &TagEnd) -> bool {
    matches!(
        tag_end,
        TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::BlockQuote(_)
            | TagEnd::CodeBlock
            | TagEnd::HtmlBlock
            | TagEnd::List(_)
            | TagEnd::Item
            | TagEnd::Table
            | TagEnd::TableHead
            | TagEnd::TableRow
            | TagEnd::TableCell
    )
}

fn frame_matches_tag_end(frame: &Frame, tag_end: &TagEnd) -> bool {
    matches!(
        (tag_end, frame),
        (TagEnd::HtmlBlock, Frame::HtmlBlock { .. })
            | (TagEnd::CodeBlock, Frame::CodeBlock { .. })
            | (TagEnd::Paragraph, Frame::Paragraph { .. })
            | (TagEnd::Heading(_), Frame::Heading { .. })
            | (TagEnd::BlockQuote(_), Frame::BlockQuote { .. })
            | (TagEnd::List(_), Frame::List { .. })
            | (TagEnd::Item, Frame::ListItem { .. })
            | (TagEnd::TableCell, Frame::TableCell { .. })
            | (TagEnd::TableHead, Frame::TableHead { .. })
            | (TagEnd::TableRow, Frame::TableRow { .. })
            | (TagEnd::Table, Frame::Table { .. })
            | (TagEnd::Emphasis, Frame::Emphasis { .. })
            | (TagEnd::Strong, Frame::Strong { .. })
            | (TagEnd::Strikethrough, Frame::Strikethrough { .. })
            | (TagEnd::Superscript, Frame::Superscript { .. })
            | (TagEnd::Subscript, Frame::Subscript { .. })
            | (TagEnd::Link, Frame::Link { .. })
            | (TagEnd::Image, Frame::Image { .. })
    )
}

fn push_coalesced(target: &mut Vec<CourseNode>, node: CourseNode) {
    if let CourseNode::Text { value } = node {
        if value.is_empty() {
            return;
        }
        if let Some(CourseNode::Text { value: prev }) = target.last_mut() {
            prev.push_str(&value);
        } else {
            target.push(CourseNode::Text { value });
        }
    } else {
        target.push(node);
    }
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn strip_single_trailing_newline(mut value: String) -> String {
    if value.ends_with('\n') {
        value.pop();
        if value.ends_with('\r') {
            value.pop();
        }
    }
    value
}

fn non_empty_trimmed(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unclosed_inline_details_does_not_wedge_list_or_swallow_following_chapters() {
        let md = "\
# Course

## Chapter 1

- Item one <details><summary>Tip</summary>unclosed
- Item two

  ```bash cwd=api
  go run .
  ```

## Chapter 2

Second chapter paragraph.
";
        let doc = parse_raw_markdown(md);
        assert_eq!(doc.chapters.len(), 2);
        assert_eq!(doc.chapters[0].title, "Chapter 1");
        assert_eq!(doc.chapters[1].title, "Chapter 2");

        let list = doc.chapters[0]
            .nodes
            .iter()
            .find_map(|n| match n {
                CourseNode::List { items, .. } => Some(items),
                _ => None,
            })
            .expect("chapter 1 list");
        assert_eq!(list.len(), 2);
        assert!(list[1].nodes.iter().any(|n| matches!(
            n,
            CourseNode::CodeBlock { info_string, .. } if info_string == "bash cwd=api"
        )));
    }

    #[test]
    fn inline_closing_details_at_end_of_paragraph_closes_details_frame() {
        let md = "\
## Chapter 1

<details><summary>Hint</summary>

Inside paragraph </details>

After paragraph.
";
        let doc = parse_raw_markdown(md);
        assert_eq!(doc.chapters.len(), 1);
        assert!(matches!(
            &doc.chapters[0].nodes[0],
            CourseNode::Details { summary, children }
                if summary == "Hint" && !children.is_empty()
        ));
        assert!(matches!(
            &doc.chapters[0].nodes[1],
            CourseNode::Paragraph { .. }
        ));
    }

    #[test]
    fn inline_script_and_style_suppression_blocks_emphasis_and_self_closing_bypasses() {
        let md = "\
## Security

*<script>*alert(1)</script>

Safe <script/>alert(2)</script> end.

Safe <script src=https://evil.invalid/>alert(3)</script> end.

Safe <style/>body{display:none}</style> end.

x <script>*<style></style>*alert(4)</script> y

x <script>\"<script>\"alert(5)</script> z
";
        let doc = parse_raw_markdown(md);
        let text = collect_plain_text(&doc.chapters[0].nodes);
        for forbidden in ["alert(", "display:none"] {
            assert!(
                !text.contains(forbidden),
                "leaked '{forbidden}' in parsed text: {text}"
            );
        }
        assert!(text.contains("Safe"));
        assert!(text.contains("end."));
        assert!(text.contains("y"));
        assert!(text.contains("z"));
    }
}
