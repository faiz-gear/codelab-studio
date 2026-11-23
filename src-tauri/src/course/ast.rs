#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CourseListItem {
    pub task_checked: Option<bool>,
    pub nodes: Vec<CourseNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CourseNode {
    Paragraph {
        children: Vec<CourseNode>,
    },
    Heading {
        level: u8,
        id: Option<String>,
        children: Vec<CourseNode>,
    },
    Text {
        value: String,
    },
    Strong {
        children: Vec<CourseNode>,
    },
    Emphasis {
        children: Vec<CourseNode>,
    },
    InlineCode {
        value: String,
    },
    CodeBlock {
        info_string: String,
        language: Option<String>,
        code: String,
    },
    List {
        ordered: bool,
        start: Option<u32>,
        items: Vec<CourseListItem>,
    },
    BlockQuote {
        children: Vec<CourseNode>,
    },
    Table {
        headers: Vec<Vec<CourseNode>>,
        rows: Vec<Vec<Vec<CourseNode>>>,
    },
    Link {
        href: String,
        title: Option<String>,
        children: Vec<CourseNode>,
    },
    Image {
        src: String,
        alt: String,
        title: Option<String>,
    },
    LineBreak,
    ThematicBreak,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CourseChapterRaw {
    pub raw_id_attr: Option<String>,
    pub title: String,
    pub nodes: Vec<CourseNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CourseDocumentRaw {
    pub title: Option<String>,
    pub intro_nodes: Vec<CourseNode>,
    pub chapters: Vec<CourseChapterRaw>,
}
