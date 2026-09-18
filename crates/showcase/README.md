# quvyta-framework-showcase

Every component of [quvyta-framework](https://crates.io/crates/quvyta-framework), the Rust
framework for terminal applications, running live in your terminal.

## Install

```sh
cargo install quvyta-framework-showcase
qframe
```

The same program is also installed as `quvyta-framework-showcase`. Everything it shows is built
into the binary, so it starts from any folder.

## What is inside

- **A page for every component and system**: buttons, inputs, lists, tables, trees, charts,
  dialogs, menus, forms, Markdown, a terminal and more, plus complete example screens.
- **Four sections per page**: a live demo with a playground, the code behind it, a guide and a
  reference.
- **Themes**: switch between the built-in themes while you browse.
- **Languages**: English and Turkish, switched at run time.
- **Icon modes**: Nerd Font, Unicode and ASCII glyphs, so you can see how each looks in your
  terminal.

A few demos touch your machine, and only to read or to show: the tree, the file picker and the
file explorer list your home folder, the terminal page opens your shell there, and the handoff
page can open your `$EDITOR`. The storage demo writes only into a folder of its own in the system's
temporary folder and removes it when you quit. Theme, language and icon choices are remembered in
the showcase's own settings file.

## The library

To build your own terminal application:

```sh
cargo add quvyta-framework
```

Documentation: <https://docs.rs/quvyta-framework>. Source: <https://github.com/quvyta/framework>.

## Licence

MIT.
