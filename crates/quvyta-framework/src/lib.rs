//! Quvyta framework: build beautiful terminal applications.
//!
//! quvyta-framework is a framework for building terminal applications in Rust. It was started to
//! design Quvyta's own applications and has grown into an open-source framework anyone can use.
//!
//! An application gives the framework three things: language files, theme files (with icon
//! files), and components built in code. A default theme, icon set and English and Turkish
//! strings are embedded, so an application runs without any files.
//!
//! An application is data, a `view` that draws it and an `update` that changes it:
//!
//! ```
//! use qframe::prelude::*;
//!
//! #[derive(Default)]
//! struct Counter {
//!     count: i32,
//! }
//!
//! #[derive(Clone)]
//! enum Msg {
//!     Increment,
//! }
//!
//! impl App for Counter {
//!     type Msg = Msg;
//!
//!     fn update(&mut self, msg: Msg) -> Command<Msg> {
//!         match msg {
//!             Msg::Increment => self.count += 1,
//!         }
//!         Command::none()
//!     }
//!
//!     fn view(&self, ui: &mut View<'_, Msg>) {
//!         ui.column(|ui| {
//!             ui.add(Text::new(format!("Count: {}", self.count)));
//!             ui.add(Button::new("Add one").variant("primary").on_press(Msg::Increment));
//!         })
//!         .gap(1);
//!     }
//! }
//!
//! // Tests drive the application without a terminal; a program calls
//! // `Runtime::new(Counter::default()).run()` in `main` instead.
//! let mut app = Harness::new(Counter::default(), 30, 4);
//! app.press("tab").press("enter");
//! assert!(app.screen().contains("Count: 1"));
//! ```
//!
//! Modules:
//!
//! - [`prelude`] — the names nearly every application uses.
//! - [`runtime`] — the [`App`](runtime::App) trait, commands, the terminal runtime, background
//!   tasks and the test harness.
//! - [`widget`] — the widget model, the view builder and layout.
//! - [`widgets`] — ready-made widgets.
//! - [`theme`] — colour tokens, motion values and CSS-like style rules; [`style`] and [`color`]
//!   hold the resolved styles and colours widgets paint with.
//! - [`icons`] — icon sets with Nerd Font, Unicode and ASCII glyphs; [`animation`] — one-cell
//!   animations defined in icon sets and themes.
//! - [`i18n`] — locales, plural forms and the [`t!`](crate::t!) macro.
//! - [`keymap`] — named actions bound to key chords; [`event`] — key, mouse and paste events.
//! - [`env`](mod@env) — the loaded theme, icons, language and keymap an application runs with.
//! - [`geometry`] and [`text`] — rectangles and padding in cells, and text measured in cells.
//! - [`date`] — calendar dates, times of day and the local time zone offset.
//! - [`uptime`] — the monotonic clocks that tell time awake from time the machine slept.
//! - [`motion`] — easing, moving values and cell-stepped progress.
//! - [`router`] — page navigation.
//! - [`storage`] — settings saved as TOML in the platform config directory, checked against a
//!   schema and optionally repaired; the config and data folders, atomic writes and the
//!   one-instance lock every application needs around its own files.
//! - [`document`] — an application's own data file: a schema'd TOML document that holds arrays
//!   of tables and is never repaired behind the application's back.
//!
//! Every loader reports problems as [`diagnostics::Diagnostic`]s with file, line and column
//! instead of failing, and built-in defaults are always available.

pub mod animation;
pub mod color;
pub mod date;
pub mod diagnostics;
pub mod document;
pub mod env;
pub mod event;
pub mod geometry;
pub mod i18n;
pub mod icons;
pub mod keymap;
pub mod motion;
pub mod prelude;
pub mod router;
pub mod runtime;
pub mod storage;
pub mod style;
pub mod text;
pub mod theme;
pub mod uptime;
pub mod widget;
pub mod widgets;

mod assets;
mod doc;

/// Compiles and runs the example in the repository's README, so it cannot go stale.
#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
struct ReadmeExample;
