# Quvyta framework

**quvyta-framework** is a Rust framework for building terminal applications. It was started for
Quvyta's own applications and is open source under the MIT licence.

It provides the runtime, a set of widgets, and theme, icon and language files. Its visual rule is
simple: shape comes from colour, never from bracket or box-drawing characters.

This repository holds two crates:

- `crates/quvyta-framework`: the library.
- `crates/showcase`: the `quvyta-framework-showcase` application, with a page for every component,
  each with a live demo, its code, a guide and a reference, in English and Turkish.

## Installation

```sh
cargo add quvyta-framework                 # the library
cargo install quvyta-framework-showcase    # try the components: run `qframe-showcase`
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

## Running the showcase

Installed, the showcase is the `qframe-showcase` command (also installed as
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
