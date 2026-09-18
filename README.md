# Quvyta framework

**quvyta-framework** is a Rust framework for building terminal applications. It was started for
Quvyta's own applications and is open source under the MIT licence.

It provides the runtime, a set of widgets, and theme, icon and language files. Its visual rule is
simple: shape comes from colour, never from bracket or box-drawing characters.

![The showcase on its dashboard example in the Nordic theme: a sparkline, gauges, bar chart, badges and a big clock](https://raw.githubusercontent.com/quvyta/framework/main/docs/screenshots/dashboard.svg)

This repository holds three crates:

- `crates/quvyta-framework`: the library.
- `crates/showcase`: the `quvyta-framework-showcase` application, with a page for every component,
  each with a live demo, its code, a guide and a reference, in English and Turkish.
- `crates/shots`: `quvyta-framework-shots`, which turns a test screen into the SVG and PNG
  pictures on this page (see [`docs/screenshots.md`](docs/screenshots.md)).

## Installation

```sh
cargo add quvyta-framework                 # the library
cargo install quvyta-framework-showcase    # try the components: run `qframe`
```

The package is named `quvyta-framework` and its library is named `qframe`, so code imports it as
`use qframe::prelude::*;`. Rust 1.95 or later is required.

## A minimal application

An application is data, a `view` that draws it and an `update` that changes it. The runtime turns
keys and clicks into messages and redraws only when something has changed.

```rust
use qframe::prelude::*;

#[derive(Default)]
struct Counter {
    count: i32,
}

#[derive(Clone)]
enum Msg {
    Increment,
}

impl App for Counter {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Increment => self.count += 1,
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.column(|ui| {
            ui.add(Text::new(format!("Count: {}", self.count)));
            ui.add(Button::new("Add one").variant("primary").on_press(Msg::Increment));
        })
        .gap(1);
    }
}

// Tests drive the same application without a terminal.
let mut app = Harness::new(Counter::default(), 30, 4);
app.press("tab").press("enter");
assert!(app.screen().contains("Count: 1"));

// A real program runs it in the terminal:
// fn main() -> std::io::Result<()> {
//     Runtime::new(Counter::default()).run()
// }
```

Text shown to users belongs in language files and is read with `t!("key")`; the example uses
plain strings to stay short. The Getting started page in the showcase walks through the same
steps.

## A look around

Every picture here is a test screen of the showcase, drawn by `quvyta-framework-shots`: no
terminal, no screen capture, the same file on every machine.

The command palette (`ctrl+p`), filtering every page and command as you type, in the Amber theme:

![The command palette filtering for "tab", over the Getting started page](https://raw.githubusercontent.com/quvyta/framework/main/docs/screenshots/command-palette.svg)

A heatmap of a year of focus, one cell a day, in the Iris theme:

![The heatmap page: a year of days in accent tones and per-category grids](https://raw.githubusercontent.com/quvyta/framework/main/docs/screenshots/heatmap.svg)

A multi-step setup wizard with a required field and its hint, in the Iris theme:

![The setup wizard example on its first step, with project name and location fields](https://raw.githubusercontent.com/quvyta/framework/main/docs/screenshots/setup-wizard.svg)

Markdown with headings, lists, inline code and quotes, in the Monochrome theme:

![The Markdown page rendering release notes](https://raw.githubusercontent.com/quvyta/framework/main/docs/screenshots/markdown.svg)

To draw them again after a change:
`cargo test -p quvyta-framework-showcase readme_shots -- --ignored`.

## Running the showcase

Installed, the showcase is the `qframe` command (also installed as
`quvyta-framework-showcase`). From a clone, with the toolchain pinned by `rust-toolchain.toml`:

```sh
cargo run -p quvyta-framework-showcase     # or ./showcase.sh
```

## Contributing

Before your first commit, enable the pre-commit checks (formatting, clippy, tests and docs):

```sh
git config core.hooksPath .githooks
```

## Documentation

- **API documentation:** `cargo doc -p quvyta-framework --open`.
- **Guides and references:** in the showcase, and as Markdown under
  `crates/showcase/assets/pages/<page>/{guide,reference}.{en,tr}.md`.
- **Component catalogue:** `crates/showcase/CATALOG.toml`, the single list of every component and
  system.
- **Design:** [`docs/design.md`](docs/design.md), the principles and architecture of the framework (in Turkish).
- **Repository:** <https://github.com/quvyta/framework>.

## Licence

MIT. See [LICENSE](LICENSE).
