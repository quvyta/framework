//! Giving the keyboard to a file manager's rows by their name, the way an application does after
//! it takes the person to another folder, in all three views; and the rows keeping it when the
//! view changes under them.

use super::mouse::project;
use super::*;
use crate::env::Env;
use crate::keymap::Scope;

/// A Files window: a button that takes the person back to the rows, as a back button or a list of
/// places does after it has moved to another folder, above a manager whose rows are named.
struct Places {
    /// Whether the name is put on what `show` returns, as applications did before
    /// [`FileManager::id`] existed, instead of on the rows.
    named_outside: bool,
    manager: FileManagerState,
    view: FileView,
}

#[derive(Clone)]
enum Go {
    Files(FileManagerMsg),
    /// The button was pressed: the keyboard goes back to the rows.
    Back,
    /// The key that turns the rows into the next view was pressed.
    NextView,
}

impl App for Places {
    type Msg = Go;

    fn init(&mut self) -> Command<Go> {
        self.manager.load(Go::Files)
    }

    fn update(&mut self, msg: Go) -> Command<Go> {
        match msg {
            Go::Files(message) => self.manager.update(message, Go::Files),
            Go::Back => Command::focus("files"),
            Go::NextView => {
                let at = VIEWS.iter().position(|view| *view == self.view).unwrap_or(0);
                self.view = VIEWS[(at + 1) % VIEWS.len()];
                Command::none()
            }
        }
    }

    fn view(&self, ui: &mut View<'_, Go>) {
        ui.add(Button::new("Back to the files").on_press(Go::Back));
        if self.named_outside {
            FileManager::new(&self.manager, Go::Files).view(self.view).show(ui).id("files").fill();
        } else {
            FileManager::new(&self.manager, Go::Files).view(self.view).id("files").show(ui).fill();
        }
    }

    fn action(&self, name: &str) -> Option<Go> {
        (name == "next-view").then_some(Go::NextView)
    }
}

/// The window of `scratch` drawn in `view`, with Alt+V bound to the next view.
fn places(scratch: &Scratch, view: FileView) -> Harness<Places> {
    places_named(scratch, view, false)
}

/// Like [`places`], with the name put on what `show` returns when `named_outside`.
fn places_named(scratch: &Scratch, view: FileView, named_outside: bool) -> Harness<Places> {
    let mut env = Env::builtin();
    env.keymap_mut().bind(Scope::App, "next-view", &["alt+v".parse().expect("a chord")]);
    let app = Places { manager: FileManagerState::new(scratch.root()).confined(), view, named_outside };
    let mut h = Harness::with_env(app, env, SIZE.0, SIZE.1);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    h.advance(MOMENT);
    h
}

/// A click on `text`, then a pause long enough that the next click is a click of its own.
fn click_on(h: &mut Harness<Places>, text: &str) {
    let (x, y) = h.find(text).unwrap_or_else(|| panic!("`{text}` is on screen:\n{}", h.screen()));
    h.click(x, y).advance(MOMENT);
}

fn selected(h: &Harness<Places>) -> Option<&str> {
    h.app().manager.selected()
}

#[test]
fn command_focus_by_the_managers_name_gives_the_rows_the_keyboard_in_every_view() {
    // The row ↓ reaches from `docs`: the next entry in the tree and the list, and in the icons
    // the card under it on the next line of the grid.
    for (view, below) in [(FileView::Tree, "src"), (FileView::List, "src"), (FileView::Icons, "plan.txt")] {
        let scratch = project(&format!("focus-name-{view:?}"));
        let mut h = places(&scratch, view);
        click_on(&mut h, "docs");
        assert_eq!(selected(&h), Some("docs"), "{view:?}\n{}", h.screen());
        h.press("shift+tab").advance(MOMENT);
        assert!(!h.is_focused("files"), "{view:?}: the keyboard left the rows");
        h.press("down").advance(MOMENT);
        assert_eq!(selected(&h), Some("docs"), "{view:?}: away from the rows ↓ moves nothing in them");

        click_on(&mut h, "Back to the files");
        assert!(h.is_focused("files"), "{view:?}: the rows have the keyboard\n{}", h.screen());
        h.press("down").advance(MOMENT);
        assert_eq!(selected(&h), Some(below), "{view:?}: ↓ moved the cursor one row\n{}", h.screen());
    }
}

#[test]
fn the_rows_keep_the_keyboard_when_the_view_changes() {
    let scratch = project("focus-views");
    let mut h = places(&scratch, FileView::Tree);
    click_on(&mut h, "docs");
    h.press("shift+tab").advance(MOMENT);
    click_on(&mut h, "Back to the files");
    assert!(h.is_focused("files"));

    // Tree, then the list, the icons and the tree again, each by a key the application binds; in
    // each a key the rows answer moves the cursor from where the last view left it.
    for (view, key, to) in
        [(FileView::List, "down", "src"), (FileView::Icons, "left", "docs"), (FileView::Tree, "down", "src")]
    {
        h.press("alt+v").advance(MOMENT);
        assert_eq!(h.app().view, view);
        assert!(h.is_focused("files"), "{view:?}: the rows still have the keyboard\n{}", h.screen());
        h.press(key).advance(MOMENT);
        assert_eq!(selected(&h), Some(to), "{view:?}: {key} moved the cursor\n{}", h.screen());
    }
}

#[test]
fn a_name_put_on_what_show_returns_still_gives_the_rows_the_keyboard() {
    // Applications named the manager this way before it could name its rows; in the tree that
    // name was on the rows themselves, and it must keep working in every view.
    for (view, below) in [(FileView::Tree, "src"), (FileView::List, "src"), (FileView::Icons, "plan.txt")] {
        let scratch = project(&format!("focus-outside-{view:?}"));
        let mut h = places_named(&scratch, view, true);
        click_on(&mut h, "docs");
        h.press("shift+tab").advance(MOMENT);
        click_on(&mut h, "Back to the files");
        h.press("down").advance(MOMENT);
        assert_eq!(selected(&h), Some(below), "{view:?}: focus by the outer name reached the rows\n{}", h.screen());
    }
}
