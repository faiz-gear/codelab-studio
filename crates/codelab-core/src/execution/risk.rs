use tree_sitter::{Node, Parser};

pub(crate) fn is_high_risk(script: &str) -> bool {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_bash::LANGUAGE.into())
        .is_err()
    {
        return true;
    }
    // Unrecognized shell syntax must not silently bypass the confirmation.
    let Some(tree) = parser.parse(script, None) else {
        return true;
    };
    let root = tree.root_node();
    if root.has_error() {
        return true;
    }

    let mut cursor = root.walk();
    loop {
        if cursor.node().kind() == "command" && risky_command(cursor.node(), script) {
            return true;
        }
        if cursor.goto_first_child() || cursor.goto_next_sibling() {
            continue;
        }
        while cursor.goto_parent() {
            if cursor.goto_next_sibling() {
                break;
            }
        }
        if cursor.node() == root {
            return false;
        }
    }
}

fn risky_command(node: Node<'_>, script: &str) -> bool {
    let Some(name) = node.child_by_field_name("name") else {
        return false;
    };
    let mut cursor = node.walk();
    let arguments = node
        .children_by_field_name("argument", &mut cursor)
        .map(|argument| unquote(&script[argument.byte_range()]))
        .collect::<Vec<_>>();
    let mut words = std::iter::once(unquote(&script[name.byte_range()])).chain(arguments);
    let piped = |mut ancestor: Node<'_>| {
        while let Some(parent) = ancestor.parent() {
            match parent.kind() {
                "pipeline" => return true,
                "redirected_statement" => ancestor = parent,
                _ => return false,
            }
        }
        false
    };
    while let Some(word) = words.next() {
        let executable = word.rsplit('/').next().unwrap_or(&word);
        let executable = executable.strip_suffix(".exe").unwrap_or(executable);
        if matches!(
            executable,
            "command" | "exec" | "env" | "nohup" | "builtin" | "time"
        ) {
            while let Some(word) = words.clone().next().filter(|word| {
                word.starts_with('-')
                    || word.split_once('=').is_some_and(|(key, _)| {
                        !key.is_empty()
                            && key
                                .bytes()
                                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                    })
            }) {
                if executable == "env"
                    && (word.starts_with("-S") || word.starts_with("--split-string"))
                {
                    return true;
                }
                words.next();
            }
            continue;
        }

        let args = words.collect::<Vec<_>>();
        return match executable {
            "sudo" | "doas" | "mkfs" => true,
            name if name.starts_with("mkfs.") => true,
            "dd" => args
                .iter()
                .any(|arg| arg.starts_with("if=") || arg.contains(['$', '`'])),
            "rm" => {
                let flags = args.iter().take_while(|arg| *arg != "--");
                let has = |letter: char, long: &str| {
                    flags.clone().any(|arg| {
                        arg == long
                            || arg.strip_prefix('-').is_some_and(|short| {
                                !short.starts_with('-') && short.contains(letter)
                            })
                    })
                };
                (has('r', "--recursive") || has('R', "--recursive")) && has('f', "--force")
                    || args
                        .iter()
                        .any(|arg| arg.contains(['$', '`']) && arg.starts_with('-'))
            }
            "chmod" => {
                args.iter().any(|arg| {
                    arg == "--recursive"
                        || arg
                            .strip_prefix('-')
                            .is_some_and(|flags| !flags.starts_with('-') && flags.contains('R'))
                }) && args
                    .iter()
                    .any(|arg| matches!(arg.as_str(), "777" | "0777"))
                    && args.iter().any(|arg| arg == "/")
            }
            "sh" | "bash" | "zsh" | "dash" | "ksh" | "fish" | "ash" | "pwsh" | "powershell"
            | "cmd" => {
                piped(node)
                    || args.iter().any(|arg| {
                        arg.starts_with('-') && !arg.starts_with("--") && arg.contains('c')
                    })
            }
            // Dynamically interpreted shell text or executable names cannot be classified statically.
            "eval" | "source" | "." => true,
            name => name.contains(['$', '`']),
        };
    }
    false
}

fn unquote(word: &str) -> String {
    let mut result = String::new();
    let mut quote = None;
    let mut chars = word.chars();
    while let Some(character) = chars.next() {
        match (quote, character) {
            (None, '\'' | '"') => quote = Some(character),
            (Some(delimiter), character) if delimiter == character => quote = None,
            (None, '\\') | (Some('"'), '\\') => {
                if let Some(escaped) = chars.next()
                    && escaped != '\n'
                {
                    result.push(escaped);
                }
            }
            _ => result.push(character),
        }
    }
    result
}
