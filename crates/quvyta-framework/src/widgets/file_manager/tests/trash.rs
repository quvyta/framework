//! The trash as a place of its own: what a manager put there is listed, put back where it came
//! from, and deleted for good behind a question — through the menus and the keys a person uses, on
//! trash folders of the test's own.

use super::*;

/// A manager whose deleting puts entries in the trash folder beside the root, with its root read.
/// The trash is on the same file system as the root, as a trash a rename can reach has to be; the
/// person's own trash is never touched.
fn trashed(scratch: &Scratch) -> Harness<Demo> {
    let mut h = Harness::new(Demo::trashing(scratch.root(), scratch.0.join("Trash")), SIZE.0, SIZE.1);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    h
}

/// Right-clicks the root's own row and opens the trash from its menu.
fn open_trash(h: &mut Harness<Demo>) {
    right_click(h, "Project");
    let menu = h.screen();
    assert!(menu.contains("Open the trash"), "the root's menu offers the trash:\n{menu}");
    h.click_text("Open the trash").advance(MOMENT);
}

/// Throws `name` of the manager's root into the trash, from its row's menu.
fn into_trash(h: &mut Harness<Demo>, name: &str) {
    right_click(h, name);
    h.click_text("Move to the trash").advance(MOMENT);
}

/// When a trashed entry's note says it went, as a column of the trash says it.
fn deleted_of(trash: &Path, name: &str) -> String {
    let note = fs::read_to_string(trash.join("info").join(format!("{name}.trashinfo"))).expect("its note");
    let stamp = note.lines().find_map(|line| line.strip_prefix("DeletionDate=")).expect("when it went");
    format!("{} {}", &stamp[..10], &stamp[11..16])
}

#[test]
fn an_entry_the_manager_trashed_is_listed_in_the_trash_under_its_own_name() {
    let scratch = Scratch::new("trash-place");
    let root = scratch.root();
    fs::write(root.join("plan.txt"), "a plan\n").expect("a second file to leave where it is");
    let mut h = trashed(&scratch);
    assert!(!h.screen().contains("Trash"), "the trash is not shown until it is asked for:\n{}", h.screen());

    into_trash(&mut h, "README.md");
    open_trash(&mut h);
    assert!(state(&h).in_trash(), "{}", h.screen());
    let screen = h.screen();
    assert!(screen.contains("Trash"), "the place has a row of its own:\n{screen}");
    assert!(screen.contains("README.md"), "the entry is listed under the name it has where it came from:\n{screen}");
    assert!(!screen.contains("plan.txt"), "and nothing else went with it:\n{screen}");

    // What the note says is what the row draws, and it is read from the note the trash wrote.
    let entry = state(&h).trashed_entry("README.md").expect("the entry is listed");
    assert_eq!(entry.origin, Some(root.join("README.md")));
    assert!(entry.deleted.is_some(), "when it went is known too");

    // Its own row is the way out of the place, as the row of a shown folder is.
    right_click(&mut h, "Trash");
    let menu = h.screen();
    assert!(menu.contains("Back to the folder") && menu.contains("Empty trash"), "the place's own menu:\n{menu}");
    h.click_text("Back to the folder").advance(MOMENT);
    assert!(!state(&h).in_trash(), "the trash is left:\n{}", h.screen());
    let screen = h.screen();
    assert!(screen.contains("plan.txt") && screen.contains("src"), "and the root is shown again:\n{screen}");
}

#[test]
fn the_trash_lists_where_its_entries_came_from_and_when_they_went() {
    let scratch = Scratch::new("trash-columns");
    let trash = scratch.0.join("Trash");
    fs::write(scratch.root().join("plan.txt"), "a plan\n").expect("a second file");
    let mut h = trashed(&scratch);
    into_trash(&mut h, "README.md");
    into_trash(&mut h, "plan.txt");
    h.send(Msg::View(FileView::List));
    open_trash(&mut h);

    let screen = h.screen();
    // The place's own columns stand where the size and the permissions of a folder's rows stand,
    // and the changed column says when the entry went rather than when the file last changed.
    assert!(screen.contains("From") && screen.contains("Changed"), "the columns of the place:\n{screen}");
    assert!(!screen.contains("Permissions") && !screen.contains("Size"), "not the ones of a folder:\n{screen}");
    for name in ["README.md", "plan.txt"] {
        assert!(screen.contains(name), "{name} is listed:\n{screen}");
    }
    assert!(screen.contains(&deleted_of(&trash, "README.md")), "when it went, as the note wrote it:\n{screen}");
}

/// A trash written by hand with `entries`, each with a note of its own that says a day and an
/// hour apart, so an order by when they went is one a manager that wrote them itself could never
/// be shown.
fn trash_of(scratch: &Scratch, entries: &[(&str, &str)]) {
    let trash = scratch.0.join("Trash");
    fs::create_dir_all(trash.join("files")).expect("the entries of a trash of its own");
    fs::create_dir_all(trash.join("info")).expect("and the notes of it");
    for (name, when) in entries {
        fs::write(trash.join("files").join(name), "an entry\n").expect("an entry");
        let note = format!("[Trash Info]\nPath=/home/one/{name}\nDeletionDate={when}\n");
        fs::write(trash.join("info").join(format!("{name}.trashinfo")), note).expect("its note");
    }
}

#[test]
fn the_trash_is_put_in_order_and_narrowed_the_way_a_folder_is() {
    let scratch = Scratch::new("trash-order");
    trash_of(
        &scratch,
        &[
            ("zebra.txt", "2026-09-18T09:15:00"),
            ("apple.txt", "2026-09-19T09:15:00"),
            ("mango.md", "2026-09-20T09:15:00"),
        ],
    );
    let names = ["apple.txt", "mango.md", "zebra.txt"];
    let mut h = trashed(&scratch);
    h.send(Msg::View(FileView::List));
    open_trash(&mut h);
    assert_eq!(listed(&h, &names), names, "by name to begin with:\n{}", h.screen());

    // A press on a title sorts the place as it sorts a folder, and here the date it sorts by is
    // the day the entry went, which is not the order of the names.
    h.click_text("Changed").advance(MOMENT);
    assert_eq!(listed(&h, &names), ["zebra.txt", "apple.txt", "mango.md"], "the oldest went first:\n{}", h.screen());
    assert_eq!(h.app().manager.sort(), crate::widgets::Sort::by(crate::widgets::SortBy::Changed));
    h.click_text("Changed").advance(MOMENT);
    assert_eq!(
        listed(&h, &names),
        ["mango.md", "apple.txt", "zebra.txt"],
        "a second press turns it round:\n{}",
        h.screen()
    );

    // The field a folder is narrowed with narrows the trash too, on the name it has where it came
    // from.
    h.click_text("Name").advance(MOMENT);
    h.send(Msg::Files(FileManagerMsg::Filter(Some(String::new()))));
    h.advance(MOMENT);
    h.click_text("Type to narrow the folder").type_text("AN").advance(MOMENT);
    assert_eq!(listed(&h, &names), ["mango.md"], "whatever the case:\n{}", h.screen());
    h.press("esc").advance(MOMENT);
    assert_eq!(listed(&h, &names), names, "and Esc shows it all again, in the order the title says:\n{}", h.screen());
}

#[test]
fn restoring_an_entry_puts_it_back_where_it_came_from_and_takes_its_note_away() {
    let scratch = Scratch::new("trash-restore");
    let trash = scratch.0.join("Trash");
    let mut h = trashed(&scratch);
    into_trash(&mut h, "README.md");
    open_trash(&mut h);

    right_click(&mut h, "README.md");
    let menu = h.screen();
    assert!(menu.contains("Restore"), "an entry of the trash can be put back:\n{menu}");
    assert!(!menu.contains("New file") && !menu.contains("Paste"), "and nothing can be made there:\n{menu}");
    h.click_text("Restore").advance(MOMENT);

    let root = scratch.root();
    assert_eq!(
        fs::read_to_string(root.join("README.md")).ok().as_deref(),
        Some("hello\n"),
        "back where it was:\n{}",
        h.screen()
    );
    assert!(!trash.join("files/README.md").exists(), "out of the trash");
    assert!(!trash.join("info/README.md.trashinfo").exists(), "and its note is gone with it");
    assert!(state(&h).trashed_entry("README.md").is_none(), "the trash no longer lists it:\n{}", h.screen());
}

#[test]
fn a_name_taken_again_is_asked_about_and_nothing_is_overwritten() {
    let scratch = Scratch::new("trash-clash");
    let root = scratch.root();
    let mut h = trashed(&scratch);
    into_trash(&mut h, "README.md");
    // Something else took the name while the entry was away, and it is the person's own file.
    fs::write(root.join("README.md"), "not the one that went\n").expect("a file of the same name");

    open_trash(&mut h);
    right_click(&mut h, "README.md");
    h.click_text("Restore").advance(MOMENT);
    let asked = h.screen();
    assert!(asked.contains("README.md is in the way"), "the person is asked:\n{asked}");
    assert!(asked.contains("README.md.1"), "with a name that is free there offered:\n{asked}");
    assert_eq!(
        fs::read_to_string(root.join("README.md")).ok().as_deref(),
        Some("not the one that went\n"),
        "nothing was overwritten before the answer"
    );

    h.click_text("Cancel").advance(MOMENT);
    assert!(!h.screen().contains("is in the way"), "the question is gone:\n{}", h.screen());
    assert_eq!(
        fs::read_to_string(root.join("README.md")).ok().as_deref(),
        Some("not the one that went\n"),
        "and nothing was overwritten by the answer"
    );
    assert!(scratch.0.join("Trash/files/README.md").is_file(), "the entry is still in the trash:\n{}", h.screen());

    // Answered yes, it goes back under the name it was offered.
    right_click(&mut h, "README.md");
    h.click_text("Restore").advance(MOMENT);
    h.click_text("Put it back as README.md.1").advance(MOMENT);
    assert_eq!(
        fs::read_to_string(root.join("README.md.1")).ok().as_deref(),
        Some("hello\n"),
        "back under the offered name:\n{}",
        h.screen()
    );
    assert_eq!(
        fs::read_to_string(root.join("README.md")).ok().as_deref(),
        Some("not the one that went\n"),
        "and the file that was there is untouched"
    );
}

#[test]
fn an_entry_whose_note_says_nothing_about_where_it_came_from_is_listed_and_is_said_to_be() {
    let scratch = Scratch::new("trash-broken");
    let trash = scratch.0.join("Trash");
    fs::create_dir_all(trash.join("files")).expect("the entries of a trash of its own");
    fs::create_dir_all(trash.join("info")).expect("and the notes of it");
    fs::write(trash.join("files/left-behind.txt"), "no note of this\n").expect("an entry without a note");
    fs::write(trash.join("files/half.txt"), "a note of its own\n").expect("another one");
    fs::write(trash.join("info/half.txt.trashinfo"), "[Trash Info]\nDeletionDate=2026-09-20T14:32:11\n")
        .expect("a note that says nothing about where it came from");

    let mut h = trashed(&scratch);
    open_trash(&mut h);
    let screen = h.screen();
    assert!(
        screen.contains("left-behind.txt") && screen.contains("half.txt"),
        "both are listed all the same:\n{screen}"
    );
    assert!(
        state(&h).trashed_entry("half.txt").expect("listed").deleted.is_some(),
        "what the note does say is still read"
    );

    // Where they came from is said in the column of the place, which is where a path has room.
    h.send(Msg::View(FileView::List));
    let screen = h.screen();
    assert!(
        screen.contains("left-behind.txt") && screen.contains("half.txt"),
        "one broken note never drops the rest of the list:\n{screen}"
    );
    assert!(screen.contains("unknown"), "with where they came from unknown:\n{screen}");

    // One of them cannot be put back, and says so rather than going nowhere.
    h.send(Msg::View(FileView::Tree));
    right_click(&mut h, "left-behind.txt");
    h.click_text("Restore").advance(MOMENT);
    let said = h.screen();
    assert!(said.contains("Nothing says where this came from"), "the reason is said:\n{said}");
    assert!(trash.join("files/left-behind.txt").is_file(), "and it is still in the trash");
    assert!(h.screen().contains("left-behind.txt"), "and still listed:\n{}", h.screen());
}

#[test]
fn deleting_an_entry_of_the_trash_for_good_asks_first_and_no_deletes_nothing() {
    let scratch = Scratch::new("trash-purge");
    let trash = scratch.0.join("Trash");
    let mut h = trashed(&scratch);
    into_trash(&mut h, "README.md");
    open_trash(&mut h);

    right_click(&mut h, "README.md");
    let menu = h.screen();
    assert!(menu.contains("Delete permanently"), "an entry of the trash can be taken out of it for good:\n{menu}");
    h.click_text("Delete permanently").advance(MOMENT);
    let asked = h.screen();
    assert!(asked.contains("Delete README.md for good?"), "{asked}");
    assert!(asked.contains("cannot be undone"), "{asked}");

    h.click_text("Cancel").advance(MOMENT);
    assert!(
        trash.join("files/README.md").is_file() && trash.join("info/README.md.trashinfo").is_file(),
        "\"No\" deletes nothing:\n{}",
        h.screen()
    );
    assert!(h.screen().contains("README.md"), "and the entry is still listed:\n{}", h.screen());

    right_click(&mut h, "README.md");
    h.click_text("Delete permanently").advance(MOMENT);
    h.click_text("Delete permanently").advance(MOMENT);
    assert!(!trash.join("files/README.md").exists(), "\"Yes\" takes the entry out of the trash:\n{}", h.screen());
    assert!(!trash.join("info/README.md.trashinfo").exists(), "and its note with it");
    assert!(!h.screen().contains("README.md"), "the trash no longer lists it:\n{}", h.screen());
}

#[test]
fn emptying_the_trash_asks_first_and_yes_empties_both_of_its_folders() {
    let scratch = Scratch::new("trash-empty");
    let trash = scratch.0.join("Trash");
    fs::write(scratch.root().join("plan.txt"), "a plan\n").expect("a second file");
    let mut h = trashed(&scratch);
    into_trash(&mut h, "README.md");
    into_trash(&mut h, "plan.txt");
    open_trash(&mut h);

    right_click(&mut h, "Trash");
    h.click_text("Empty trash").advance(MOMENT);
    let asked = h.screen();
    assert!(asked.contains("Empty the trash?") && asked.contains("cannot be undone"), "{asked}");
    assert_eq!(
        fs::read_dir(trash.join("files")).expect("the entries").count(),
        2,
        "nothing happened before the answer"
    );

    h.click_text("Cancel").advance(MOMENT);
    assert_eq!(
        fs::read_dir(trash.join("files")).expect("the entries").count(),
        2,
        "\"No\" deletes nothing:\n{}",
        h.screen()
    );

    right_click(&mut h, "Trash");
    h.click_text("Empty trash").advance(MOMENT);
    h.click_text("Empty trash").advance(MOMENT);
    assert_eq!(
        fs::read_dir(trash.join("files")).expect("the entries").count(),
        0,
        "\"Yes\" empties the entries:\n{}",
        h.screen()
    );
    assert_eq!(fs::read_dir(trash.join("info")).expect("the notes").count(), 0, "and the notes with them");
    assert!(h.screen().contains("empty"), "the empty trash says so:\n{}", h.screen());
    // An empty trash has nothing to empty, so nothing is offered for it.
    right_click(&mut h, "Trash");
    assert!(!h.screen().contains("Empty trash"), "there is nothing left to empty:\n{}", h.screen());
}

#[test]
fn nothing_that_acts_below_the_root_reaches_an_entry_of_the_trash() {
    let scratch = Scratch::new("trash-guarded");
    let root = scratch.root();
    let mut h = trashed(&scratch);
    // A file of the root that shares its name with an entry of the trash: carrying the row of the
    // trash away must not move this one.
    into_trash(&mut h, "README.md");
    fs::write(root.join("README.md"), "the root's own\n").expect("and a file of the same name is there again");
    open_trash(&mut h);

    let (entry_x, entry_y) = h.find("README.md").expect("the row of the entry in the trash");
    h.click(entry_x, entry_y).press("ctrl+x").advance(MOMENT);
    assert!(state(&h).cut().is_empty(), "nothing is cut in the trash:\n{}", h.screen());

    let (place_x, place_y) = h.find("Trash").expect("the place's own row");
    h.drag((entry_x, entry_y), (place_x, place_y)).advance(MOMENT);
    assert_eq!(
        fs::read_to_string(root.join("README.md")).ok().as_deref(),
        Some("the root's own\n"),
        "and a drag onto the place moves nothing:\n{}",
        h.screen()
    );
}

#[test]
fn a_name_that_is_not_one_entry_of_the_trash_reaches_nothing_outside_it() {
    let scratch = Scratch::new("trash-escape");
    let trash = scratch.0.join("Trash");
    let mut h = trashed(&scratch);
    into_trash(&mut h, "README.md");
    // Beside the trash's entries, where `files/..` would lead: what a name built by hand must not reach.
    fs::write(trash.join("beside.txt"), "kept\n").expect("a file beside the trash's folders");

    // `README.md` is a real entry, but the trash is not shown, so no row of it was ever offered:
    // a key the manager did not list is no more an entry than one that leaves the folder.
    let keys = ["", ".", "..", "../beside.txt", "../info", "README.md"].map(str::to_owned).to_vec();
    h.send(Msg::Files(FileManagerMsg::PurgeConfirmed(keys.clone()))).advance(MOMENT);
    h.send(Msg::Files(FileManagerMsg::Restore(keys.clone()))).advance(MOMENT);
    for key in keys {
        h.send(Msg::Files(FileManagerMsg::RestoreAs(key, "back.txt".to_owned()))).advance(MOMENT);
    }

    assert!(trash.join("files/README.md").is_file(), "the entries stay:\n{}", h.screen());
    assert!(trash.join("info/README.md.trashinfo").is_file(), "and their notes");
    assert!(trash.join("beside.txt").is_file(), "and nothing beside the trash is touched");
}

#[test]
fn the_trash_itself_refuses_a_name_that_is_not_one_of_its_entries() {
    let scratch = Scratch::new("trash-escape-below");
    let trash = scratch.0.join("Trash");
    let mut h = trashed(&scratch);
    into_trash(&mut h, "README.md");
    fs::write(trash.join("beside.txt"), "kept\n").expect("a file beside the trash's folders");

    // Below the manager, where nothing has narrowed the names to the ones the trash lists.
    for name in ["", ".", "..", "../beside.txt", "../info"] {
        let refused = Err(FileError::Outside);
        assert_eq!(super::super::trash::purge(&trash, name), refused, "{name:?} is not deleted");
        assert_eq!(super::super::trash::restore(&trash, name, None), refused, "{name:?} is not put back");
    }
    assert!(trash.join("files/README.md").is_file(), "the entries stay");
    assert!(trash.join("info/README.md.trashinfo").is_file(), "and their notes");
    assert!(trash.join("beside.txt").is_file(), "and nothing beside the trash is touched");
}
