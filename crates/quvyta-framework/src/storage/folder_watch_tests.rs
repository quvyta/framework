use super::*;

fn change(folder: &str, name: Option<&str>, kind: FolderChangeKind) -> FolderChange {
    FolderChange { folder: PathBuf::from(folder), name: name.map(OsString::from), kind }
}

#[test]
fn a_batch_pairs_a_rename_inside_one_folder_and_splits_a_move_between_two() {
    let mut batch = Batch::default();
    batch.moved_from(7, Path::new("/a"), "old".into());
    batch.moved_to(7, Path::new("/a"), "new".into());
    batch.moved_from(8, Path::new("/a"), "leaves".into());
    batch.moved_to(8, Path::new("/b"), "arrives".into());
    batch.moved_to(9, Path::new("/b"), "from outside".into());
    batch.moved_from(10, Path::new("/a"), "to outside".into());
    assert_eq!(
        batch.finish(),
        vec![
            change("/a", Some("new"), FolderChangeKind::Renamed { from: "old".into() }),
            change("/a", Some("leaves"), FolderChangeKind::Removed),
            change("/b", Some("arrives"), FolderChangeKind::Created),
            change("/b", Some("from outside"), FolderChangeKind::Created),
            change("/a", Some("to outside"), FolderChangeKind::Removed),
        ]
    );
}

#[test]
fn a_batch_keeps_each_change_once_in_its_last_place() {
    let mut batch = Batch::default();
    let folder = Path::new("/a");
    batch.push(folder, Some("x".into()), FolderChangeKind::Created);
    batch.push(folder, Some("x".into()), FolderChangeKind::Modified);
    batch.push(folder, Some("x".into()), FolderChangeKind::Modified);
    batch.push(folder, Some("x".into()), FolderChangeKind::Removed);
    batch.push(folder, Some("x".into()), FolderChangeKind::Created);
    assert_eq!(
        batch.finish(),
        vec![
            change("/a", Some("x"), FolderChangeKind::Modified),
            change("/a", Some("x"), FolderChangeKind::Removed),
            change("/a", Some("x"), FolderChangeKind::Created),
        ],
        "the entry ends up there, as it is on disk"
    );
}

#[cfg(not(target_os = "linux"))]
#[test]
fn other_platforms_are_told_there_is_no_watch() {
    let error = FolderWatch::new().expect_err("no folder watch here");
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::fs;
    use std::sync::mpsc;
    use std::time::Instant;

    /// An empty directory of this test's own.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("quvyta-watch-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("test directory");
        dir
    }

    /// `next` on its own thread, so a test can say how long it is willing to wait.
    fn next_within(changes: &FolderChanges, limit: Duration) -> Option<Vec<FolderChange>> {
        let changes = changes.clone();
        let (send, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = send.send(changes.next());
        });
        receive.recv_timeout(limit).ok()
    }

    const PATIENCE: Duration = Duration::from_secs(5);

    fn names(batch: &[FolderChange]) -> Vec<(String, FolderChangeKind)> {
        batch
            .iter()
            .map(|change| {
                let name = change.name.as_ref().map(|name| name.to_string_lossy().into_owned());
                (name.unwrap_or_default(), change.kind.clone())
            })
            .collect()
    }

    #[test]
    fn a_bounded_wait_comes_back_empty_handed_and_the_watch_goes_on() {
        let dir = temp_dir("bounded");
        let mut watch = FolderWatch::new().expect("inotify");
        watch.watch(&dir).expect("watch");
        let changes = watch.changes();
        let started = Instant::now();
        assert_eq!(changes.next_within(Duration::from_millis(100)), None, "nothing changed within the bound");
        assert!(started.elapsed() < PATIENCE, "and it came back");
        fs::write(dir.join("late"), "").expect("create");
        let batch = changes.next_within(PATIENCE).expect("the change after a quiet wait");
        assert_eq!(names(&batch)[0], ("late".into(), FolderChangeKind::Created));
        drop(watch);
        assert_eq!(changes.next_within(PATIENCE), Some(Vec::new()), "a dropped watch still says stop");
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn creating_renaming_and_removing_a_file_are_reported_by_name() {
        let dir = temp_dir("basic");
        let mut watch = FolderWatch::new().expect("inotify");
        watch.watch(&dir).expect("watch");
        let changes = watch.changes();

        fs::write(dir.join("a"), "").expect("create");
        let batch = next_within(&changes, PATIENCE).expect("an answer");
        assert!(batch.iter().all(|change| change.folder == dir), "{batch:?}");
        assert_eq!(names(&batch)[0], ("a".into(), FolderChangeKind::Created));

        fs::rename(dir.join("a"), dir.join("b")).expect("rename");
        let batch = next_within(&changes, PATIENCE).expect("an answer");
        assert_eq!(names(&batch), vec![("b".into(), FolderChangeKind::Renamed { from: "a".into() })]);

        fs::write(dir.join("b"), "more").expect("write");
        let batch = next_within(&changes, PATIENCE).expect("an answer");
        assert_eq!(names(&batch), vec![("b".into(), FolderChangeKind::Modified)]);

        fs::remove_file(dir.join("b")).expect("remove");
        let batch = next_within(&changes, PATIENCE).expect("an answer");
        assert_eq!(names(&batch), vec![("b".into(), FolderChangeKind::Removed)]);
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_change_in_a_folder_below_is_not_reported_and_neither_is_an_unwatched_one() {
        let dir = temp_dir("below");
        let below = dir.join("below");
        fs::create_dir(&below).expect("subfolder");
        let mut watch = FolderWatch::new().expect("inotify");
        watch.watch(&dir).expect("watch");
        let changes = watch.changes();

        fs::write(below.join("deep"), "").expect("create below");
        assert!(next_within(&changes, Duration::from_millis(300)).is_none(), "not recursive");

        watch.unwatch(&dir);
        fs::write(dir.join("after"), "").expect("create after unwatch");
        assert!(next_within(&changes, Duration::from_millis(300)).is_none(), "no longer watched");
        drop(watch);
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_move_from_one_watched_folder_to_another_is_a_removal_and_a_creation() {
        let dir = temp_dir("move");
        let (left, right) = (dir.join("left"), dir.join("right"));
        fs::create_dir(&left).expect("left");
        fs::create_dir(&right).expect("right");
        fs::write(left.join("file"), "").expect("file");
        let mut watch = FolderWatch::new().expect("inotify");
        watch.watch(&left).expect("watch left");
        watch.watch(&right).expect("watch right");
        let changes = watch.changes();

        fs::rename(left.join("file"), right.join("file")).expect("move");
        let batch = next_within(&changes, PATIENCE).expect("an answer");
        assert_eq!(
            batch,
            vec![
                FolderChange { folder: left, name: Some("file".into()), kind: FolderChangeKind::Removed },
                FolderChange { folder: right, name: Some("file".into()), kind: FolderChangeKind::Created },
            ]
        );
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_burst_of_a_thousand_files_arrives_in_a_few_batches() {
        let dir = temp_dir("burst");
        let mut watch = FolderWatch::new().expect("inotify");
        watch.watch(&dir).expect("watch");
        let changes = watch.changes();

        let writer_dir = dir.clone();
        let writer = std::thread::spawn(move || {
            for index in 0..1000 {
                fs::write(writer_dir.join(format!("file-{index}")), "").expect("create");
            }
        });
        let mut created = std::collections::HashSet::new();
        let mut batches = 0;
        while created.len() < 1000 {
            let batch = next_within(&changes, PATIENCE).expect("an answer");
            batches += 1;
            for change in batch {
                if change.kind == FolderChangeKind::Created {
                    created.insert(change.name.expect("a name"));
                }
            }
        }
        writer.join().expect("writer");
        // A thousand writes take a few milliseconds; even on a busy machine running every other
        // test beside this one they span only a few tenths of a second.
        assert!(batches <= 10, "{batches} batches for one burst");
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn an_idle_watch_waits_and_dropping_it_wakes_the_waiter_with_nothing() {
        let dir = temp_dir("idle");
        let mut watch = FolderWatch::new().expect("inotify");
        watch.watch(&dir).expect("watch");
        let changes = watch.changes();

        let (send, receive) = mpsc::channel();
        let waiter = changes.clone();
        // The clock starts before the thread does: a thread scheduled late would otherwise count
        // less than the 400 ms this side already waited.
        let started = Instant::now();
        std::thread::spawn(move || {
            let batch = waiter.next();
            let _ = send.send((batch, started.elapsed()));
        });
        assert!(receive.recv_timeout(Duration::from_millis(400)).is_err(), "nothing changed, so it waits");

        drop(watch);
        let (batch, waited) = receive.recv_timeout(PATIENCE).expect("the drop woke it");
        assert!(batch.is_empty(), "a dropped watch answers nothing");
        assert!(waited >= Duration::from_millis(400), "it waited until the drop");
        assert!(changes.next().is_empty(), "and keeps answering nothing");
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_watched_folder_that_is_removed_is_reported_once() {
        let dir = temp_dir("gone");
        let watched = dir.join("watched");
        fs::create_dir(&watched).expect("folder");
        fs::write(watched.join("inside"), "").expect("file");
        let mut watch = FolderWatch::new().expect("inotify");
        watch.watch(&watched).expect("watch");
        let changes = watch.changes();

        fs::remove_dir_all(&watched).expect("remove the watched folder");
        let batch = next_within(&changes, PATIENCE).expect("an answer");
        let gone: Vec<_> = batch.iter().filter(|change| change.kind == FolderChangeKind::Gone).collect();
        assert_eq!(gone.len(), 1, "{batch:?}");
        assert_eq!(gone[0].folder, watched);

        // A new folder under the old name is a new folder: the watch does not follow it.
        fs::create_dir(&watched).expect("again");
        fs::write(watched.join("new"), "").expect("file");
        assert!(next_within(&changes, Duration::from_millis(300)).is_none(), "reported once");
        watch.unwatch(&watched);
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_watched_folder_moved_away_is_reported_once_and_not_followed() {
        let dir = temp_dir("moved");
        let (watched, elsewhere) = (dir.join("watched"), dir.join("elsewhere"));
        fs::create_dir(&watched).expect("folder");
        let mut watch = FolderWatch::new().expect("inotify");
        watch.watch(&watched).expect("watch");
        let changes = watch.changes();

        fs::rename(&watched, &elsewhere).expect("move away");
        let batch = next_within(&changes, PATIENCE).expect("an answer");
        assert_eq!(batch, vec![FolderChange { folder: watched, name: None, kind: FolderChangeKind::Gone }]);
        fs::write(elsewhere.join("later"), "").expect("file");
        assert!(next_within(&changes, Duration::from_millis(300)).is_none(), "not followed");
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_queue_that_overflows_asks_for_every_folder_to_be_read_again() {
        let Some(limit) = fs::read_to_string("/proc/sys/fs/inotify/max_queued_events")
            .ok()
            .and_then(|text| text.trim().parse::<usize>().ok())
            .filter(|limit| *limit <= 100_000)
        else {
            // A machine with a much larger queue would make this test slow; the unit tests above
            // cover everything but the overflow itself.
            return;
        };
        let dir = temp_dir("overflow");
        let (first, second) = (dir.join("first"), dir.join("second"));
        fs::create_dir(&first).expect("first");
        fs::create_dir(&second).expect("second");
        let mut watch = FolderWatch::new().expect("inotify");
        watch.watch(&first).expect("watch first");
        watch.watch(&second).expect("watch second");
        let changes = watch.changes();

        // Nobody reads while these are made, so the kernel's queue fills and overflows.
        for index in 0..limit + 100 {
            fs::File::create(first.join(index.to_string())).expect("create");
        }
        let mut overflowed = Vec::new();
        while overflowed.len() < 2 {
            let batch = next_within(&changes, PATIENCE).expect("an answer");
            overflowed.extend(batch.into_iter().filter(|change| change.kind == FolderChangeKind::Overflow));
        }
        let folders: Vec<_> = overflowed.iter().map(|change| change.folder.clone()).collect();
        assert_eq!(folders, vec![first, second]);
        assert!(overflowed.iter().all(|change| change.name.is_none()));
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_path_that_is_not_a_folder_cannot_be_watched() {
        let dir = temp_dir("errors");
        let mut watch = FolderWatch::new().expect("inotify");
        let missing = watch.watch(&dir.join("absent")).expect_err("missing");
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
        fs::write(dir.join("file"), "").expect("file");
        let file = watch.watch(&dir.join("file")).expect_err("a file");
        assert_eq!(file.kind(), io::ErrorKind::NotADirectory);
        // Watching twice is harmless.
        watch.watch(&dir).expect("watch");
        watch.watch(&dir).expect("watch again");
        fs::remove_dir_all(&dir).expect("clean");
    }
}
