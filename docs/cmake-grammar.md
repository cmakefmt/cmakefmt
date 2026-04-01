# CMake Language Grammar Reference

Source: <https://cmake.org/cmake/help/latest/manual/cmake-language.7.html>

This document is the authoritative reference for writing `src/parser/cmake.pest`.
All constructs listed here must be handled by the parser.

---

## Formal Grammar (EBNF)

```ebnf
cmake_file        ::=  file_element*

file_element      ::=  command_invocation newline
                     | (bracket_comment | space)* newline

command_invocation ::=  space* identifier space* "(" arguments ")"

arguments         ::=  argument?
                       (sep argument)*
                       sep?

sep               ::=  space+
                     | line_ending

line_ending       ::=  line_comment? newline

space             ::=  [ \t]+
newline           ::=  \n

identifier        ::=  [A-Za-z_][A-Za-z0-9_]*

argument          ::=  bracket_argument
                     | quoted_argument
                     | unquoted_argument

(* ------------------------------------------------------------------ *)
(*  Bracket argument / bracket comment                                 *)
(* ------------------------------------------------------------------ *)

bracket_argument  ::=  "[" "="* "[" bracket_content "]" "="* "]"
                       (* The number of "=" signs in the opening and  *)
                       (* closing bracket must match.                 *)
                       (* No processing of the content occurs.        *)

bracket_content   ::=  (* any text not containing the matching close bracket *)

bracket_comment   ::=  "#" bracket_argument

(* ------------------------------------------------------------------ *)
(*  Line comment                                                       *)
(* ------------------------------------------------------------------ *)

line_comment      ::=  "#" (* any text to end of line, EXCEPT: if the  *)
                           (* next character after "#" starts a valid  *)
                           (* bracket_argument, it is a bracket_comment *)
                       newline?

(* ------------------------------------------------------------------ *)
(*  Quoted argument                                                    *)
(* ------------------------------------------------------------------ *)

quoted_argument   ::=  '"' quoted_element* '"'

quoted_element    ::=  (* any char except '\' or '"' *)
                     | escape_sequence
                     | quoted_continuation
                     | variable_reference    (* evaluated at runtime, opaque to formatter *)
                     | generator_expression  (* evaluated at build time, opaque *)

quoted_continuation ::= "\" newline          (* line continuation: newline is ignored *)

(* ------------------------------------------------------------------ *)
(*  Unquoted argument                                                  *)
(* ------------------------------------------------------------------ *)

unquoted_argument ::=  unquoted_element+

unquoted_element  ::=  (* any char except whitespace or one of: ( ) # " \ *)
                     | escape_sequence
                     | variable_reference
                     | generator_expression

(* NOTE: Semicolons in unquoted arguments are list separators at the  *)
(* CMake language level, but they are NOT syntax — they appear as     *)
(* literal characters and the formatter treats them as opaque.        *)

(* ------------------------------------------------------------------ *)
(*  Escape sequences                                                   *)
(* ------------------------------------------------------------------ *)

escape_sequence   ::=  escape_identity | escape_encoded | escape_semicolon

escape_identity   ::=  "\" <any non-alphanumeric, non-semicolon char>
                       (* The backslash is removed; the char is literal *)

escape_encoded    ::=  "\t"   (* horizontal tab, U+0009 *)
                     | "\r"   (* carriage return, U+000D *)
                     | "\n"   (* newline, U+000A *)

escape_semicolon  ::=  "\;"   (* literal semicolon; does not split list *)

(* ------------------------------------------------------------------ *)
(*  Variable references (opaque to formatter — preserve as-is)        *)
(* ------------------------------------------------------------------ *)

variable_reference ::=  "${" variable_name "}"
                       (* variable_name may itself contain a variable_reference *)
                     | "$ENV{" variable_name "}"
                     | "$CACHE{" variable_name "}"    (* CMake 3.25+ only *)

variable_name     ::=  (* any chars except "}" — may nest ${...} *)

(* ------------------------------------------------------------------ *)
(*  Generator expressions (opaque to formatter — preserve as-is)      *)
(* ------------------------------------------------------------------ *)

generator_expression ::=  "$<" genex_content ">"

genex_content     ::=  (* any chars; may nest $<...> and ${...} *)
```

---

## Key Rules and Edge Cases

### 0. Semicolons split unquoted arguments

This is the single most surprising behaviour for newcomers.

An unquoted argument containing `;` is **split into multiple arguments** by CMake
at runtime (semicolon is CMake's list separator). The formatter preserves this
verbatim — it does NOT split, join, or reformat across semicolons. The formatter
treats unquoted arguments as opaque tokens.

```cmake
command(a;b;c)          # runtime: 3 arguments: "a", "b", "c"
command("a;b;c")        # runtime: 1 argument:  "a;b;c"  (quoted preserves)
command([[a;b;c]])      # runtime: 1 argument:  "a;b;c"  (bracket preserves)
command(a\;b)           # runtime: 1 argument:  "a;b"    (\; escapes semicolon)
```

### 1. Bracket argument matching

The number of `=` signs must match between the opening `[=..=[` and closing `]=..=]`.
Examples:

- `[[content]]` — 0 equals signs
- `[=[content]=]` — 1 equals sign
- `[==[content]==]` — 2 equals signs

Content is taken verbatim. No escape processing. No variable expansion.
A leading newline immediately after the opening bracket is ignored.

### 2. Bracket comment vs line comment

If the character after `#` is `[` followed by zero or more `=` followed by `[`,
it is a **bracket comment** (multi-line). Otherwise it is a **line comment** (to end of line).

```cmake
# This is a line comment
#[[ This is a bracket comment
    spanning multiple lines ]]
#[=[ Also a bracket comment ]=]
```

### 3. Quoted argument — continuation lines

Inside a quoted argument, `\` immediately followed by a newline causes the newline
to be ignored (line continuation). The formatter must preserve this exactly.

```cmake
set(VAR "line one \
line two")   # VAR = "line one line two"
```

### 4. Unquoted argument — forbidden characters

Characters that terminate an unquoted argument (or are forbidden entirely):
`(`, `)`, `#`, `"`, `\`, whitespace (space, tab, newline)

Semicolons `;` are **allowed** in unquoted arguments. They are syntactically
ordinary characters but act as list-element separators at CMake runtime
(see Rule 0 above).

**Legacy unquoted arguments** (CMake compat): A construct like `-Da="b c"` where
double-quotes appear in the middle of an unquoted argument is allowed for
backwards compatibility but is strongly discouraged. The formatter preserves
these as-is and does not attempt to normalise legacy quoting.

### 5. Unquoted argument — variable references

Variable references `${VAR}` and generator expressions `$<...>` are valid inside
unquoted arguments. The formatter preserves them as opaque strings.

```cmake
target_link_libraries(foo ${LIBS} $<TARGET_FILE:bar>)
```

### 6. Command name case

Command names are case-insensitive (`SET`, `set`, `Set` are identical).
The formatter normalises them according to `command_case` config option.

### 7. Identifier syntax

Command names (identifiers) match `[A-Za-z_][A-Za-z0-9_]*`.

### 8. Newline handling

CMake uses Unix newlines (`\n`). The formatter always outputs `\n`.
Input with `\r\n` (Windows) should have `\r` stripped during normalisation.

### 9. Separation between arguments

Arguments are separated by whitespace (`[ \t\n]+`) or line endings (with optional
line comments). The formatter controls this spacing — it does not preserve the
original spacing between arguments, only the arguments themselves.

### 10. Empty argument lists

```cmake
some_command()   # valid — zero arguments
```

---

## Construct Examples

### Bracket argument

```cmake
set(VAR [[
multi
line
value
]])

set(VAR [==[
contains ]] without closing
]==])
```

### All argument types together

```cmake
message(
    "quoted ${VAR} arg"          # quoted, with variable reference
    unquoted_arg                 # unquoted
    [[bracket arg]]              # bracket
    ${ANOTHER_VAR}               # unquoted variable ref
    $<TARGET_FILE:foo>           # generator expression
    "continuation \
line"                            # quoted with continuation
)
```

### Bracket comment

```cmake
#[=[
This entire block is a comment.
It can contain # characters without issue.
]=]
```

### Line comment after command

```cmake
find_package(Threads REQUIRED)  # needed for pthreads
```

### Comment inside argument list

```cmake
target_link_libraries(myapp
    # Core libraries
    Threads::Threads
    # Optional
    ${OPTIONAL_LIBS}
)
```

---

## What the Formatter Does NOT Change

- The **content** of any argument (quoted, bracket, or unquoted)
- Variable references (`${VAR}`, `$ENV{VAR}`, `$CACHE{VAR}`)
- Generator expressions (`$<...>`)
- Escape sequences (preserved verbatim)
- Bracket argument/comment delimiters (the `=` count is preserved)
- Line continuations inside quoted arguments

The formatter only changes:

- Whitespace **between** tokens (arguments, command name, parens)
- Blank lines **between** top-level statements
- Command name case (if `command_case != "unchanged"`)
- Indentation of continuation lines in argument lists

---

## Grammar Notes for Pest

When writing the pest grammar:

1. `WHITESPACE` rule should NOT be set globally — CMake whitespace is significant
   in determining argument separation. Handle it explicitly.

2. Bracket argument: use a custom rule with a Rust `#[tag]` to validate
   that the `=` count matches. Pest cannot express this directly in grammar;
   use a custom validator or a generated set of rules for 0..=N equals signs.
   Practical approach: match `[` `=`{0,10} `[` and capture the = count,
   then match `]` followed by the same number of `=` followed by `]`.
   This is best handled in a Rust post-processing step after pest captures
   the raw token.

3. Comments must appear in the parse tree (not be silenced with `_`).
   Use named rules so AST builder can identify them.

4. The `sep` rule (whitespace/newline between args) should be silent in pest
   (`_sep`) since the formatter controls this spacing itself.

5. Inline bracket comments between arguments are valid and must be handled:

   ```cmake
   message("First" #[[inline comment]] "Second")
   ```

---

## External Grammar References

These are useful cross-references when writing the pest grammar:

- [Official CMake Language Manual](https://cmake.org/cmake/help/latest/manual/cmake-language.7.html)
- [ANTLR v4 CMake Grammar](https://github.com/antlr/grammars-v4/blob/master/cmake/CMake.g4) — community formal grammar, good reference for edge cases
- [CMake Generator Expressions Manual](https://cmake.org/cmake/help/latest/manual/cmake-generator-expressions.7.html)
