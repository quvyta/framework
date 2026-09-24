use std::ffi::OsString;
use std::path::Path;

use super::*;

const FILE: &str = "/home/ada/my \"odd\" file's $name.txt";

fn run(exec: &str) -> Option<Vec<OsString>> {
    run_with(exec, Some("accessories-text-editor"))
}

fn run_with(exec: &str, icon: Option<&str>) -> Option<Vec<OsString>> {
    expand(
        exec,
        &Fields {
            file: Path::new(FILE),
            name: "Metin Düzenleyici",
            icon,
            entry: Path::new("/usr/share/applications/editor.desktop"),
        },
    )
}

fn args(list: &[&str]) -> Option<Vec<OsString>> {
    Some(list.iter().map(OsString::from).collect())
}

#[test]
fn the_file_is_one_argument_whatever_its_name_holds() {
    assert_eq!(run("editor %f"), args(&["editor", FILE]));
    assert_eq!(run("editor %u"), args(&["editor", FILE]));
    assert_eq!(run("editor --new-window %F"), args(&["editor", "--new-window", FILE]));
    assert_eq!(run("editor %U"), args(&["editor", FILE]));
}

#[test]
fn a_field_code_inside_a_word_is_filled_in_place() {
    assert_eq!(
        run("editor --file=%f --title=%c"),
        args(&["editor", &format!("--file={FILE}"), "--title=Metin Düzenleyici"])
    );
}

#[test]
fn a_line_with_no_file_code_gets_the_file_last() {
    assert_eq!(run("viewer --fullscreen"), args(&["viewer", "--fullscreen", FILE]));
}

#[test]
fn a_doubled_percent_is_a_percent() {
    assert_eq!(run("printf 100%% %f"), args(&["printf", "100%", FILE]));
    assert_eq!(run("printf \"50%%\" %f"), args(&["printf", "50%", FILE]));
}

#[test]
fn the_icon_code_is_two_arguments_or_none() {
    assert_eq!(run("editor %i %f"), args(&["editor", "--icon", "accessories-text-editor", FILE]));
    assert_eq!(run_with("editor %i %f", None), args(&["editor", FILE]));
}

#[test]
fn the_name_and_the_entrys_path_are_filled_in() {
    assert_eq!(
        run("editor --class %c --desktop %k %f"),
        args(&["editor", "--class", "Metin Düzenleyici", "--desktop", "/usr/share/applications/editor.desktop", FILE,])
    );
}

#[test]
fn deprecated_codes_are_dropped() {
    assert_eq!(run("old %d %D %n %N %v %m %f"), args(&["old", FILE]));
    assert_eq!(run("old -x%m %f"), args(&["old", "-x", FILE]));
}

#[test]
fn quoted_arguments_keep_spaces_and_their_escapes() {
    assert_eq!(
        run(r#""/opt/My Editor/bin/editor" --say "a \"quoted\" \`word\` \$HOME \\ \x" %f"#),
        args(&["/opt/My Editor/bin/editor", "--say", r#"a "quoted" `word` $HOME \ \x"#, FILE,])
    );
    assert_eq!(run(r#"editor "" %f"#), args(&["editor", "", FILE]), "an empty quote is an argument");
    assert_eq!(run(r#"editor --x="a b"c %f"#), args(&["editor", "--x=a bc", FILE]));
}

#[test]
fn a_file_code_inside_quotes_is_quoted_for_the_shell_that_reads_it() {
    let got = run(r#"sh -c "exec viewer %f""#).expect("valid");
    assert_eq!(
        got,
        args(&["sh", "-c", r#"exec viewer '/home/ada/my "odd" file'\''s $name.txt'"#]).expect("list"),
        "a quote, a `$` or a space in the name stays part of the name"
    );
}

#[test]
fn spaces_between_arguments_may_repeat() {
    assert_eq!(run("  editor \t  %f  "), args(&["editor", FILE]));
}

#[test]
fn a_broken_line_gives_no_command() {
    assert_eq!(run(r#"editor "unclosed %f"#), None);
    assert_eq!(run(r#"editor "ends in a backslash\"#), None);
    assert_eq!(run("editor %"), None);
    assert_eq!(run("editor %z"), None, "a field code the standard does not know");
    assert_eq!(run(""), None);
    assert_eq!(run("   "), None);
    assert_eq!(run(r#""" %f"#), None, "an empty program");
    assert_eq!(run("%d"), None, "nothing is left to run");
}

#[cfg(unix)]
#[test]
fn a_file_name_that_is_not_utf8_passes_through_unchanged() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let file = Path::new(OsStr::from_bytes(b"/tmp/f\xffle"));
    let got = expand("viewer %f", &Fields { file, name: "Viewer", icon: None, entry: Path::new("/x.desktop") });
    assert_eq!(got, Some(vec![OsString::from("viewer"), file.as_os_str().to_owned()]));
}
