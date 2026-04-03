/// A parsed CMake source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// Top-level statements in source order.
    pub statements: Vec<Statement>,
}

/// A top-level statement in a CMake file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    /// A command invocation, e.g. `target_link_libraries(foo PUBLIC bar)`.
    Command(CommandInvocation),
    /// A top-level configure-file placeholder line such as `@PACKAGE_INIT@`.
    ///
    /// These occur in `.cmake.in` templates and must be preserved verbatim.
    TemplatePlaceholder(String),
    /// A standalone comment (on its own line).
    Comment(Comment),
    /// One or more consecutive blank lines between statements.
    /// The value is the number of blank lines (>= 1).
    BlankLines(usize),
}

/// A CMake command invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    /// The command name, e.g. "target_link_libraries". Case as written in source.
    pub name: String,
    /// The argument list, in source order.
    pub arguments: Vec<Argument>,
    /// A comment that appears after the closing paren on the same line.
    pub trailing_comment: Option<Comment>,
    /// Byte span (start, end) in the original source.
    pub span: (usize, usize),
}

/// A single argument (or inline comment) in an argument list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    /// `[[...]]`, `[=[...]=]`, etc. Content is verbatim.
    Bracket(BracketArgument),
    /// `"..."` — includes the surrounding quotes verbatim.
    Quoted(String),
    /// Any other token — unquoted argument, variable ref, generator expr.
    Unquoted(String),
    /// A comment that appears inline between arguments.
    InlineComment(Comment),
}

impl Argument {
    /// The source text of this argument.
    pub fn as_str(&self) -> &str {
        match self {
            Argument::Bracket(b) => &b.raw,
            Argument::Quoted(s) | Argument::Unquoted(s) => s,
            Argument::InlineComment(c) => c.as_str(),
        }
    }

    /// Returns `true` when the argument is an inline comment placeholder rather
    /// than a normal CMake argument token.
    pub fn is_comment(&self) -> bool {
        matches!(self, Argument::InlineComment(_))
    }
}

/// A bracket argument with its "=" nesting level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BracketArgument {
    /// Number of `=` characters between the outer brackets. 0 = `[[...]]`.
    pub level: usize,
    /// The raw source text, e.g. `[==[content]==]`.
    pub raw: String,
}

/// A CMake comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comment {
    /// `# text to end of line` (stored with the leading `#`).
    Line(String),
    /// `#[[...]]` or `#[=[...]=]` (stored as the full raw text including `#`).
    Bracket(String),
}

impl Comment {
    /// Return the raw source text of the comment, including the leading `#`.
    pub fn as_str(&self) -> &str {
        match self {
            Comment::Line(s) | Comment::Bracket(s) => s,
        }
    }
}
