<!-- Copied from ortho's ai/AGENTS.md by `cargo xtask vscode-grammar`. -->

# SysML v2 models

The `.sysml` files in this project are SysML v2 models that ortho draws as architecture views. Before you write, change or review one, read `.claude/skills/sysml-v2-ortho/SKILL.md` and the files beside it, and follow them.

In short: write only the SysML v2 that `LANGUAGE.md` there describes, render the model's folder after every change with `ortho render <model folder> -o <a temporary folder>`, and fix every error and warning at its cause. Never delete the line a problem points at, and never drop model elements to make a view fit its page: split the view instead.
