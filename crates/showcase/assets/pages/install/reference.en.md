## Methods

- `Install::package(name) -> Option<Install>` — the package with this machine's package manager, looked for on `PATH` in the order `pacman`, `apt-get`, `dnf`, `zypper`, `apk`, `brew`; `None` when there is none or `name` is not one package name (empty, starting with `-`, holding a space or a control character). Homebrew is passed over as root.
- `Install::package_with(name, lookup, root) -> Option<Install>` — the same with the search given: `lookup: impl Fn(&str) -> Option<PathBuf>` says where a program is, `root: bool` whether the application runs as root.
- `.name_for(manager, name) -> Install` — uses `name` when the manager is `manager`; nothing changes on the others or for a name that is not one package name.
- `.manager() -> Manager`, `.name() -> &str` — the manager found and the package's name for it.
- `.program() -> Vec<OsString>` — the command's words, the program first.
- `.command_line() -> String` — the command as the person reads it, such as `sudo pacman -S --needed libarchive`.
- `.confirm(on_yes) -> Confirm<Msg>` — the question: the package, the exact command line and that the manager may ask for a password, with an Install button. Chain `Confirm` options on it, such as `.on_cancel(msg)`.
- `.handoff(on_finish) -> Handoff<Msg>` — runs the command with the terminal handed over, a notice above the output and `pause(true)`; `on_finish` receives the `HandoffOutcome`.
- `Manager::Pacman`, `Apt`, `Dnf`, `Zypper`, `Apk`, `Brew` (non-exhaustive); `Manager::name() -> &'static str` is the program (`apt-get` for `Apt`); `Manager::all() -> &'static [Manager]` in search order.

## Commands

- `pacman -S --needed <package>`, `apt-get install <package>`, `dnf install <package>`, `zypper install <package>`, `apk add <package>`, `brew install <package>`, with `sudo` in front when not root, except for `brew`. No yes flag is ever added.

## Behaviour

- Nothing runs before the handoff is returned; return it only from the question's confirm message.
- The confirm button reads Install; Cancel has focus, and Cancel, Esc and the close mark install nothing.
- In a `Harness` the handoff is recorded in `handoffs()` with the command's program and arguments and `pause` on, and answered with `Finished { code: Some(0) }` unless the test sets another outcome.

## Theme keys

- The question is a `Confirm` and uses its keys: `modal`, `modal-title`, `close-mark`, `layer-backdrop`, `button.primary`.
- Language — `quvyta.install.title` (with `{package}`), `quvyta.install.runs`, `quvyta.install.asks` (with `{manager}`), `quvyta.install.confirm`, `quvyta.install.notice` (with `{package}` and `{manager}`), and `quvyta.handoff.pause` for the key press after the manager ends.
