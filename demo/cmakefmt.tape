# cmakefmt demo — generates demo.gif
#
# Before running, make sure cmakefmt is on your PATH:
#   export PATH="../../../rust/cmakefmt/codebase/target/release:$PATH"
#
# Then run from the oomph-lib directory:
#   cd /Users/PuneetMatharu/Dropbox/programming/oomph-lib/oomph-lib-repos/forked-oomph-lib
#   vhs ../../../rust/cmakefmt/codebase/demo/cmakefmt.tape

Output ../../../rust/cmakefmt/codebase/demo/cmakefmt.gif

Set Shell bash
Set FontFamily "Consolas"
Set FontSize 18
Set Width 1400
Set Height 800
Set Padding 20
Set Theme "Github"
Set TypingSpeed 25ms
Set CursorBlink false
Set Margin 20
Set MarginFill "#674EFF"
Set BorderRadius 10
Set Framerate 60
Set PlaybackSpeed 0.5 # Make output 2 times slower
Set TypingSpeed 20ms

# ─── Scene 1: Show a messy CMake file ────────────────────────────────

Type "# A messy CMake file with inconsistent formatting..."
Enter
Sleep 500ms

Type "cat messy_demo.cmake"
Enter
Sleep 1s

Hide
Type "clear"
Enter
Show

# ─── Scene 2: Format it ─────────────────────────────────────────────

Type "# Now let's see what would happen if we formatted it with cmakefmt!"
Enter
Sleep 500ms

Type "cmakefmt messy_demo.cmake"
Sleep 50ms
Enter
Sleep 2s

Hide
Type "clear"
Enter
Show

# ─── Scene 3: Show the diff ─────────────────────────────────────────

Type "# Prefer a 'git diff'-like view instead?"
Enter
Sleep 500ms

Type "cmakefmt --diff --color always messy_demo.cmake"
Sleep 50ms
Enter
Sleep 2s

Hide
Type "clear"
Enter
Show

# ─── Scene 4: Check 600+ files at once ──────────────────────────────

Type "# Now let's run it on 600+ CMake files across a large codebase..."
Enter
Sleep 500ms

Type "cmakefmt --summary --progress-bar ."
Sleep 50ms
Enter
Sleep 2s

Hide
Type "clear"
Enter
Show

# ─── Scene 5: Parse tree dump ────────────────────────────────────────

Type "# Interested in what the parse tree looks like for debugging?"
Enter
Sleep 500ms

Type "cmakefmt --color always dump parse messy_demo.cmake"
Sleep 50ms
Enter
Sleep 4s
