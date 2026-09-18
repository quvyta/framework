//! Checking settings against a schema and healing them, on realistic examples and on files broken
//! in every way we could think of.

use std::fs;
use std::path::PathBuf;

use super::{Schema, SettingKind, Settings};
use crate::diagnostics::Severity;

/// An application's settings shape: the built-in keys with the installed languages, and a
/// deploy section.
fn schema() -> Schema {
    Schema::builtin()
        .choice(Settings::LANGUAGE, ["en", "tr"], "tr")
        .flag("deploy.confirm", true)
        .choice("deploy.region", ["eu-west", "us-east", "ap-south"], "eu-west")
        .check("deploy.branch", String::new(), |branch| !branch.contains(char::is_whitespace))
}

/// The same shape with the opt-in capabilities: two optional deploy keys, the open
/// `plugins` table and one key declared inside it.
fn open_schema() -> Schema {
    schema()
        .optional("deploy.note", SettingKind::text())
        .optional("deploy.retries", SettingKind::check(|retries: &u8| (1..=5).contains(retries)))
        .open("plugins")
        .flag("plugins.enabled", true)
}

/// A settings file in a fresh directory of its own.
fn file(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("quvyta-healing-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("settings.toml");
    fs::write(&path, text).expect("write settings");
    path
}

fn located(settings: &Settings) -> Vec<String> {
    settings.diagnostics().iter().map(ToString::to_string).collect()
}

fn clean(path: &std::path::Path) {
    if let Some(dir) = path.parent() {
        let _ = fs::remove_dir_all(dir);
    }
}

#[test]
fn an_unknown_key_is_removed_and_the_rest_is_kept() {
    let text = "language = \"tr\"\ncolor = \"red\"\npillar = \"thick\"\nslide = true\nreduced-motion = false\n";
    let path = file("example", text);
    let settings = Settings::open(&path).schema(schema()).self_heal(true);
    assert_eq!(
        located(&settings),
        vec!["settings.toml:2:1: warning: `color` is not a known setting; removed".to_owned()]
    );
    assert_eq!(settings.language().as_deref(), Some("tr"));
    assert_eq!(settings.pillar_style(), Some(crate::icons::PillarStyle::Thick));
    assert_eq!((settings.slide(), settings.reduced_motion()), (Some(true), Some(false)));
    assert_eq!(
        fs::read_to_string(&path).expect("healed file"),
        "language = \"tr\"\npillar = \"thick\"\nslide = true\nreduced-motion = false\n",
        "saved once, in the same order, without color"
    );
    assert_eq!(fs::read_to_string(path.with_extension("toml.bak")).expect("backup"), text, "the file as it was");
    clean(&path);
}

#[test]
fn an_invalid_language_is_replaced_by_the_default() {
    let path = file("language", "language = \"sjds\"\npillar = \"thick\"\n");
    let settings = Settings::open(&path).schema(schema()).self_heal(true);
    assert_eq!(
        located(&settings),
        vec![
            "settings.toml:1:1: warning: `language` must be one of en, tr, found \"sjds\"; replaced with \"tr\""
                .to_owned()
        ]
    );
    assert_eq!(fs::read_to_string(&path).expect("healed"), "language = \"tr\"\npillar = \"thick\"\n");
    assert_eq!(Settings::open(&path).schema(schema()).self_heal(true).diagnostics(), &[], "healed files stay quiet");
    clean(&path);
}

#[test]
fn key_order_is_never_a_problem() {
    let text = "reduced-motion = false\nslide = true\n\n[deploy]\nregion = \"us-east\"\nconfirm = false\n";
    let text = format!("pillar = \"thin\"\nlanguage = \"en\"\n{text}");
    let path = file("order", &text);
    let settings = Settings::open(&path).schema(schema()).self_heal(true);
    assert_eq!(settings.diagnostics(), &[]);
    let keys: Vec<&str> = settings.keys().collect();
    assert_eq!(keys, ["pillar", "language", "reduced-motion", "slide", "deploy.region", "deploy.confirm"]);
    assert_eq!(fs::read_to_string(&path).expect("untouched"), text, "nothing to repair, nothing written");
    assert!(!path.with_extension("toml.bak").exists());
    clean(&path);
}

#[test]
fn without_self_healing_the_schema_only_warns() {
    let text = "language = \"sjds\"\ncolor = \"red\"\n\n[deploy]\nbranch = \"release candidate\"\n";
    let path = file("warn", text);
    let settings = Settings::open(&path).schema(schema());
    assert_eq!(
        located(&settings),
        vec![
            "settings.toml:1:1: warning: `language` must be one of en, tr, found \"sjds\"; it is ignored".to_owned(),
            "settings.toml:2:1: warning: `color` is not a known setting; it is ignored".to_owned(),
            "settings.toml:5:1: warning: `deploy.branch` must be a value this application accepts, found \"release candidate\"; it is ignored".to_owned(),
        ]
    );
    assert!(settings.diagnostics().iter().all(|d| d.severity == Severity::Warning));
    assert_eq!(settings.value("color").map(super::SettingValue::type_name), Some("string"), "kept");
    assert_eq!(fs::read_to_string(&path).expect("untouched"), text);
    let off = Settings::open(&path).schema(schema()).self_heal(false);
    assert_eq!(located(&off), located(&settings));
    assert_eq!(fs::read_to_string(&path).expect("still untouched"), text);
    clean(&path);
}

#[test]
fn healing_needs_the_applications_schema_and_the_call_order_does_not_matter() {
    let text = "theme = \"nordic\"\ndeploy-region = \"us-east\"\nicons = \"sparkly\"\n";
    let path = file("needs-schema", text);
    let alone = Settings::open(&path).self_heal(true);
    assert_eq!(alone.value("deploy-region").map(super::SettingValue::type_name), Some("string"));
    assert_eq!(
        located(&alone),
        vec!["settings.toml:3:1: warning: `icons` must be one of auto, nerd, unicode, ascii, found \"sparkly\"; it is ignored".to_owned()]
    );
    assert_eq!(fs::read_to_string(&path).expect("untouched"), text, "without a schema a key could be the app's own");

    let before = Settings::parse_str("settings.toml", text).self_heal(true).schema(schema());
    let after = Settings::parse_str("settings.toml", text).schema(schema()).self_heal(true);
    assert_eq!(before.to_toml(), after.to_toml());
    assert_eq!(before.diagnostics(), after.diagnostics());
    assert_eq!(
        located(&after),
        vec![
            "settings.toml:2:1: warning: `deploy-region` is not a known setting; removed".to_owned(),
            "settings.toml:3:1: warning: `icons` must be one of auto, nerd, unicode, ascii, found \"sparkly\"; replaced with \"auto\"".to_owned(),
        ]
    );
    let again = after.schema(schema());
    assert_eq!(again.diagnostics().len(), 2, "repairs stay reported when the check runs again");
    clean(&path);
}

#[test]
fn built_in_keys_are_checked_where_they_are_written() {
    let text = "# pick a theme and a pillar\n[theme-extras]\nslide-speed = 3\n";
    let settings = Settings::parse_str("settings.toml", text);
    assert_eq!(settings.diagnostics(), &[], "no schema: other keys are the application's business");
    let broken = "# pick a theme\ntheme = 3\n  pillar = \"wide\"\nslide = \"yes\"\n";
    let settings = Settings::parse_str("settings.toml", broken);
    assert_eq!(
        located(&settings),
        vec![
            "settings.toml:2:1: warning: `theme` must be a string, found 3; it is ignored".to_owned(),
            "settings.toml:3:3: warning: `pillar` must be one of thick, thin, found \"wide\"; it is ignored".to_owned(),
            "settings.toml:4:1: warning: `slide` must be a boolean, found \"yes\"; it is ignored".to_owned(),
        ]
    );
    assert_eq!((settings.pillar_style(), settings.slide()), (None, None));
}

#[test]
fn syntax_errors_and_unstorable_values_are_backed_up_and_repaired() {
    let text = "language = \"tr\"\nslide = 2026-09-16\ncolor = \n[deploy\nregion = \"us-east\"\n";
    let path = file("syntax", text);
    let settings = Settings::open(&path).schema(schema()).self_heal(true);
    let errors = settings.diagnostics().iter().filter(|d| d.severity == Severity::Error).count();
    assert!(errors >= 1, "{:?}", located(&settings));
    assert!(
        located(&settings).contains(
            &"settings.toml:2:1: warning: `slide` holds a value settings cannot store; replaced with true".to_owned()
        ),
        "{:?}",
        located(&settings)
    );
    assert_eq!(fs::read_to_string(path.with_extension("toml.bak")).expect("backup"), text);
    let healed = fs::read_to_string(&path).expect("healed");
    assert!(healed.starts_with("language = \"tr\"\nslide = true\n"), "{healed}");
    let reread = Settings::open(&path).schema(schema()).self_heal(true);
    assert_eq!(reread.diagnostics(), &[], "{healed}");
    clean(&path);
}

#[test]
fn a_syntax_error_is_located_by_line_and_character_column() {
    // The non-ASCII value before the error counts as one column per character, not per byte.
    let settings = Settings::parse_str("settings.toml", "theme = \"amber\"\nlanguage = \"ğüş\" x\n");
    assert_eq!(located(&settings), ["settings.toml:2:18: error: unexpected key or value, expected newline, `#`"]);
    assert_eq!(settings.theme().as_deref(), Some("amber"), "the line before the error is kept");
}

#[test]
fn read_errors_survive_the_schema_check() {
    let dir = std::env::temp_dir().join(format!("quvyta-healing-unreadable-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("settings.toml")).expect("a directory where the file should be");
    let settings = Settings::open(dir.join("settings.toml")).schema(schema()).self_heal(true);
    assert_eq!(settings.diagnostics().len(), 1, "{:?}", located(&settings));
    assert_eq!(settings.diagnostics()[0].severity, Severity::Error);
    let _ = fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn a_repair_that_cannot_be_saved_is_reported() {
    use std::os::unix::fs::PermissionsExt;
    let path = file("readonly", "color = \"red\"\n");
    let dir = path.parent().expect("dir").to_path_buf();
    let loaded = Settings::open(&path).schema(schema());
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).expect("read-only dir");
    let healed = loaded.self_heal(true);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).expect("writable again");
    // Root may write anyway; then the repair is simply saved.
    let saved = fs::read_to_string(&path).expect("file").is_empty();
    let last = healed.diagnostics().last().expect("reported");
    if saved {
        assert_eq!(last.severity, Severity::Warning);
    } else {
        assert_eq!(last.severity, Severity::Error);
        assert!(last.message.contains("repaired settings not saved"), "{last}");
    }
    assert!(healed.value("color").is_none(), "healed in memory either way");
    clean(&path);
}

#[test]
fn no_input_makes_healing_panic_and_healed_output_reads_back_clean() {
    let mut inputs: Vec<String> = [
        "",
        "=",
        "language",
        "language = ",
        "language = \"tr",
        "[[deploy]]\nregion = 1\n[[deploy]]\n",
        "[deploy]\n[deploy]\nregion = \"us-east\"\n",
        "language = \"tr\"\nlanguage = \"en\"\n",
        "deploy = \"flat\"\n[deploy]\nregion = \"us-east\"\n",
        "deploy.region.deeper = true\n",
        "\"\" = 1\n\"a.b\" = 2\n'deploy'.'region' = \"ap-south\"\n",
        "slide = [true, 2]\npillar = { style = \"thin\" }\n",
        "icons = \"\u{0}\"\ntheme = \"\\uD800\"\n",
        "reduced-motion = 1e999\nslide = nan\n",
        "\u{feff}language = \"tr\"\r\ncolor = \"red\"\r\n",
        "language = \"çok\"\n# yorum ğüşıöç\nbranch = \"a\nb\"\n",
        "[deploy]\nbranch = \"main\"\nbranch = \"dev\"\n[deploy.branch]\nx = 1\n",
    ]
    .map(str::to_owned)
    .to_vec();
    // Every cut of a realistic file, and a few bytes flipped by a fixed generator.
    let whole = "language = \"tr\"\ncolor = \"red\"\n[deploy]\nregion = \"us-east\"\nbranch = \"main\"\n";
    for cut in 0..=whole.len() {
        inputs.push(whole[..cut].to_owned());
    }
    let mut seed: u32 = 0x5eed;
    for _ in 0..300 {
        let mut bytes = whole.as_bytes().to_vec();
        for _ in 0..3 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let at = (seed >> 8) as usize % bytes.len();
            let noise = b"=[]\".\n #\\{}'";
            bytes[at] = noise[(seed >> 20) as usize % noise.len()];
        }
        inputs.push(String::from_utf8_lossy(&bytes).into_owned());
    }
    for input in &inputs {
        let healed = Settings::parse_str("settings.toml", input).schema(schema()).self_heal(true);
        let reread = Settings::parse_str("settings.toml", &healed.to_toml()).schema(schema());
        assert_eq!(reread.diagnostics(), &[], "input {input:?} healed to {:?}", healed.to_toml());
    }

    // The same sweep with optional keys and an open table, on files that use both.
    inputs.extend(
        [
            "[deploy]\nnote = 1\nretries = [2]\nnote = \"a\"\n",
            "[deploy]\nnote = 2026-09-17\nretries = 1e999\n",
            "deploy.note.more = \"x\"\n[deploy.note]\ny = 1\n",
            "plugins = \"flat\"\n[plugins]\nx = 1\n",
            "[plugins]\ngit = 1\n\"git.sign\" = true\n\"git.sign.key\" = \"abc\"\n",
            "[plugins]\n\"enabled.x\" = 1\nenabled = \"yes\"\n",
            "[plugins]\n\"\" = 1\n[plugins.\"\"]\n\"\" = 2\n[plugins.\"a b\".'c.d']\ne = [1, [2]]\n",
            "[[plugins.list]]\nx = 1\n[plugins.at]\nwhen = 2026-09-17T10:00:00Z\ninline = { a = { b = 1 } }\n",
            "[plugins\nx = 1\n[plugins.]\n",
        ]
        .map(str::to_owned),
    );
    let whole = "[deploy]\nnote = \"freeze\"\nretries = 9\n[plugins]\nenabled = \"yes\"\n[plugins.git]\nsign = true\n";
    for cut in 0..=whole.len() {
        inputs.push(whole[..cut].to_owned());
    }
    for _ in 0..300 {
        let mut bytes = whole.as_bytes().to_vec();
        for _ in 0..3 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let at = (seed >> 8) as usize % bytes.len();
            let noise = b"=[]\".\n #\\{}'";
            bytes[at] = noise[(seed >> 20) as usize % noise.len()];
        }
        inputs.push(String::from_utf8_lossy(&bytes).into_owned());
    }
    for input in &inputs {
        let healed = Settings::parse_str("settings.toml", input).schema(open_schema()).self_heal(true);
        let text = healed.to_toml();
        let reread = Settings::parse_str("settings.toml", &text).schema(open_schema()).self_heal(true);
        assert_eq!(reread.diagnostics(), &[], "input {input:?} healed to {text:?}");
        assert_eq!(reread.to_toml(), text, "input {input:?}: healing a healed file changes nothing");
    }
}

#[test]
fn a_valid_optional_key_is_kept_an_invalid_one_removed_and_a_missing_one_not_added() {
    let text = "language = \"tr\"\n\n[deploy]\nnote = \"freeze until Friday\"\nretries = 12\n";
    let path = file("optional", text);
    let warned = Settings::open(&path).schema(open_schema());
    assert_eq!(
        located(&warned),
        vec!["settings.toml:5:1: warning: `deploy.retries` must be a value this application accepts, found 12; it is ignored".to_owned()]
    );
    assert_eq!(fs::read_to_string(&path).expect("untouched"), text);

    let settings = Settings::open(&path).schema(open_schema()).self_heal(true);
    assert_eq!(
        located(&settings),
        vec![
            "settings.toml:5:1: warning: `deploy.retries` must be a value this application accepts, found 12; removed"
                .to_owned()
        ]
    );
    assert_eq!(settings.get::<String>("deploy.note").as_deref(), Some("freeze until Friday"));
    assert_eq!(settings.get::<u8>("deploy.retries"), None, "no default to read");
    let healed = fs::read_to_string(&path).expect("healed");
    assert_eq!(healed, "language = \"tr\"\n\n[deploy]\nnote = \"freeze until Friday\"\n", "nothing added");
    assert_eq!(fs::read_to_string(path.with_extension("toml.bak")).expect("backup"), text);
    let reread = Settings::open(&path).schema(open_schema()).self_heal(true);
    assert_eq!(reread.diagnostics(), &[], "{healed}");
    assert_eq!(fs::read_to_string(&path).expect("still healed"), healed, "a clean file is not written again");
    clean(&path);

    let path = file("optional-missing", "language = \"tr\"\n");
    let settings = Settings::open(&path).schema(open_schema()).self_heal(true);
    assert_eq!(settings.diagnostics(), &[]);
    assert_eq!((settings.value("deploy.note"), settings.value("deploy.retries")), (None, None));
    assert_eq!(fs::read_to_string(&path).expect("untouched"), "language = \"tr\"\n", "missing keys are not added");
    assert!(!path.with_extension("toml.bak").exists());
    clean(&path);
}

#[test]
fn an_optional_value_settings_cannot_store_is_removed() {
    let text = "[deploy]\nnote = 2026-09-17\nconfirm = 2026-09-17\n";
    let settings = Settings::parse_str("settings.toml", text).schema(open_schema()).self_heal(true);
    let located = located(&settings);
    assert!(
        located.contains(
            &"settings.toml:2:1: warning: `deploy.note` holds a value settings cannot store; removed".to_owned()
        ),
        "{located:?}"
    );
    assert!(
        located.contains(
            &"settings.toml:3:1: warning: `deploy.confirm` holds a value settings cannot store; replaced with true"
                .to_owned()
        )
    );
    assert_eq!(settings.to_toml(), "[deploy]\nconfirm = true\n");
}

#[test]
fn missing_keys_with_a_default_are_read_as_missing_and_never_written() {
    let path = file("missing-default", "[deploy]\ncolor = \"red\"\n");
    let settings = Settings::open(&path).schema(open_schema()).self_heal(true);
    assert_eq!(settings.language(), None, "the application falls back to its own default");
    assert!(settings.get_or("deploy.confirm", true));
    assert_eq!(fs::read_to_string(&path).expect("healed"), "", "the unknown key went, no default came in");
    clean(&path);
}

#[test]
fn an_open_prefix_keeps_unknown_keys_and_tables_as_they_are() {
    let text = "theme = \"nordic\"\ncolor = \"red\"\n\n[plugins]\nspellcheck = true\nlanguages = [\"en\", \"tr\"]\n\n[plugins.git-sync]\ninterval = 15\nremote = \"origin\"\n\n[plugins.git-sync.hooks]\nafter-pull = \"cargo check\"\n\n[plugins-extra]\nx = 1\n";
    let warned = Settings::parse_str("settings.toml", text).schema(open_schema());
    assert_eq!(
        located(&warned),
        vec![
            "settings.toml:2:1: warning: `color` is not a known setting; it is ignored".to_owned(),
            "settings.toml:16:1: warning: `plugins-extra.x` is not a known setting; it is ignored".to_owned(),
        ],
        "open keys are not reported, a look-alike prefix is"
    );
    let path = file("open", text);
    let settings = Settings::open(&path).schema(open_schema()).self_heal(true);
    assert_eq!(
        located(&settings),
        vec![
            "settings.toml:2:1: warning: `color` is not a known setting; removed".to_owned(),
            "settings.toml:16:1: warning: `plugins-extra.x` is not a known setting; removed".to_owned(),
        ]
    );
    assert_eq!(settings.get::<u16>("plugins.git-sync.interval"), Some(15));
    let healed = fs::read_to_string(&path).expect("healed");
    assert_eq!(
        healed,
        "theme = \"nordic\"\n\n[plugins]\nspellcheck = true\nlanguages = [\"en\", \"tr\"]\n\n[plugins.git-sync]\ninterval = 15\nremote = \"origin\"\n\n[plugins.git-sync.hooks]\nafter-pull = \"cargo check\"\n"
    );
    let reread = Settings::open(&path).schema(open_schema()).self_heal(true);
    assert_eq!(reread.diagnostics(), &[]);
    assert_eq!(reread.to_toml(), healed, "round trip");
    clean(&path);
}

#[test]
fn a_rule_inside_an_open_prefix_still_heals_its_own_key() {
    let text = "[plugins]\nenabled = \"yes\"\nspellcheck = \"yes\"\n";
    let settings = Settings::parse_str("settings.toml", text).schema(open_schema()).self_heal(true);
    assert_eq!(
        located(&settings),
        vec![
            "settings.toml:2:1: warning: `plugins.enabled` must be a boolean, found \"yes\"; replaced with true"
                .to_owned()
        ]
    );
    assert_eq!(
        settings.to_toml(),
        "[plugins]\nenabled = true\nspellcheck = \"yes\"\n",
        "the open neighbour is left alone"
    );
    let optional = Schema::default().open("plugins").optional("plugins.theme", SettingKind::choice(["dark", "light"]));
    let text = "[plugins]\ntheme = \"sepia\"\nother = 1\n";
    let settings = Settings::parse_str("settings.toml", text).schema(optional).self_heal(true);
    assert_eq!(settings.to_toml(), "[plugins]\nother = 1\n", "an invalid optional key under the prefix is removed");
}

#[test]
fn open_keys_that_one_file_cannot_hold_together_are_separated() {
    // A quoted segment with a dot reads as a deeper key, next to a plain value at the same place.
    let text = "[plugins]\ngit = 1\n\"git.sign\" = true\n\"enabled.x\" = 2\nenabled = false\n";
    let settings = Settings::parse_str("settings.toml", text).schema(open_schema()).self_heal(true);
    assert_eq!(
        located(&settings),
        vec![
            "settings.toml:3:1: warning: `plugins.git.sign` cannot sit next to `plugins.git` in one file; removed"
                .to_owned(),
            "settings.toml:4:1: warning: `plugins.enabled.x` cannot sit next to `plugins.enabled` in one file; removed"
                .to_owned(),
        ],
        "the first written stays, and a declared key wins"
    );
    assert_eq!(settings.to_toml(), "[plugins]\ngit = 1\nenabled = false\n");
}
