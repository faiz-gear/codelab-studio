use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use ammonia::Builder;

fn sanitizer() -> &'static Builder<'static> {
    static SANITIZER: OnceLock<Builder<'static>> = OnceLock::new();
    SANITIZER.get_or_init(|| {
        let tags: HashSet<&'static str> = HashSet::from([
            "p",
            "br",
            "hr",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "blockquote",
            "pre",
            "code",
            "ul",
            "ol",
            "li",
            "dl",
            "dt",
            "dd",
            "table",
            "thead",
            "tbody",
            "tfoot",
            "tr",
            "th",
            "td",
            "strong",
            "b",
            "em",
            "i",
            "del",
            "s",
            "sub",
            "sup",
            "a",
            "span",
            "div",
            "details",
            "summary",
            "img",
        ]);

        let mut tag_attributes: HashMap<&'static str, HashSet<&'static str>> = HashMap::new();
        tag_attributes.insert("a", HashSet::from(["href", "title"]));
        tag_attributes.insert("img", HashSet::from(["src", "alt", "title"]));
        tag_attributes.insert("th", HashSet::from(["align"]));
        tag_attributes.insert("td", HashSet::from(["align"]));
        tag_attributes.insert("details", HashSet::from(["open"]));

        let mut allowed_classes: HashMap<&'static str, HashSet<&'static str>> = HashMap::new();
        allowed_classes.insert(
            "code",
            HashSet::from([
                "language-go",
                "language-ts",
                "language-tsx",
                "language-js",
                "language-jsx",
                "language-typescript",
                "language-javascript",
                "language-python",
                "language-py",
                "language-java",
                "language-sql",
                "language-json",
                "language-yaml",
                "language-yml",
                "language-toml",
                "language-text",
                "language-txt",
                "language-http",
                "language-diff",
                "language-markdown",
                "language-md",
                "language-bash",
                "language-sh",
                "language-zsh",
            ]),
        );
        allowed_classes.insert("span", HashSet::from(["md-task", "md-task--done"]));

        let url_schemes: HashSet<&'static str> = HashSet::from(["http", "https", "mailto"]);

        let mut builder = Builder::empty();
        builder
            .tags(tags)
            .tag_attributes(tag_attributes)
            .allowed_classes(allowed_classes)
            .url_schemes(url_schemes)
            .link_rel(Some("noopener noreferrer"))
            .strip_comments(true);
        builder
    })
}

pub(crate) fn sanitize_html(raw: &str) -> String {
    sanitizer().clean(raw).to_string()
}
