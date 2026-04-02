use pretty::RcDoc;

use crate::parser::ast::Comment;

type Doc = RcDoc<'static, ()>;

/// Convert an inline comment (between arguments) into a Doc IR node.
pub fn inline_comment_doc(comment: &Comment) -> Doc {
    match comment {
        // Line comments include the leading "#" from the parser.
        Comment::Line(text) => RcDoc::text(text.clone()),
        Comment::Bracket(raw) => literal_doc(raw),
    }
}

fn literal_doc(source: &str) -> Doc {
    let normalized = source.replace("\r\n", "\n");
    let mut parts = normalized.split('\n');
    let first = RcDoc::text(parts.next().unwrap_or_default().to_owned());

    parts.fold(first, |doc, part| {
        doc.append(RcDoc::hardline())
            .append(RcDoc::text(part.to_owned()))
    })
}
