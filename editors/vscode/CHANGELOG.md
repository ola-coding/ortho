# Changelog

The extension has the version of the ortho it carries.

## 0.3.0 (2026-11-08)

The first release on the VS Code Marketplace and Open VSX.

- **A live preview** beside a `.sysml` file, with a tab for each view the
  model declares, drawn again on every save. Click a box to open the line
  that declares it.
- **Save the diagrams as SVG:** the view shown, or every view into a folder.
- **ortho's problems in the Problems panel,** underlined in the file.
- **SysML v2 colouring,** from the keywords ortho reads.
- **Add AI Instructions:** a Claude Code skill and GitHub Copilot instructions
  that teach an agent the SysML v2 ortho reads and to check its work with it.
- **Open Example:** the coffee machine or the survey drone, copied into a
  folder you pick.
- **ortho inside,** built for your system, also on the PATH of VS Code's
  terminals. In a folder you do not trust, the folder cannot choose another.

It carries ortho 0.3.0, which since 0.2.0:

- warns where two views of a model disagree: a function nothing performs, a
  use case no function tree refines, software no deployment puts on a host;
- refuses a part or an actor typed by behaviour or data;
- suggests the name meant when a segment of a chain names nothing;
- marks each box with the file and line that declare it;
- writes the views in the order a model is read, from use case to process;
- renders again on every save with `ortho render --watch`.
