## When to use

Use it when your application needs a program the machine does not have: an archive viewer without `bsdtar`, an editor without a language server, a tool without `git`. Instead of telling the person to copy a command into a terminal, offer to do it here, in front of them, after they agreed. When no package manager is found, say which package is needed instead.

## Step by step

1. When the program is missing, find the package manager: `Install::package("libarchive")`. It gives `None` when this machine has none the framework knows.
2. If the package has another name on one distribution, say so: `.name_for(Manager::Apt, "libarchive-tools")`.
3. Show the command where the person decides, for example beside the button: `install.command_line()`.
4. When the button is pressed, ask: `Command::confirm(install.confirm(Msg::Install))`. Nothing runs yet.
5. Only when the answer arrives, hand the terminal over: `Msg::Install => Command::handoff(install.handoff(Msg::Installed))`.
6. In `Msg::Installed(outcome)`, look again for the program and tell the person how it went: `Finished { code: Some(0) }` installed it, any other outcome did not.

## How it works

- **The machine's own package manager.** `pacman`, `apt-get`, `dnf`, `zypper`, `apk` and `brew` are looked for on `PATH` in that order and the first one found is used. AUR helpers such as `paru` are left out on purpose: they build from source as the user, which is not what "install this package" means.
- **The exact command, shown before it runs.** `sudo pacman -S --needed libarchive`: the manager's own install command, `sudo` in front when the application is not root (Homebrew never takes it, and is passed over as root because it refuses to run so). The question shows this same line.
- **No yes flag.** The package manager asks its own question, such as "Proceed with installation?", and the person answers it on the terminal.
- **The password goes to the package manager.** The handoff gives the terminal to `sudo` and the manager for as long as they run; your application never sees the password.
- **The output stays readable.** The handoff waits for a key press after the manager ends, so its last lines, an error included, can be read before your screen comes back.
- **Esc installs nothing.** Cancel, Esc and the close mark all decline; add `.on_cancel(msg)` to the question to hear it.
- **Tests never install.** `Install::package_with(name, lookup, root)` takes the program search and the root question, and a `Harness` records the handoff instead of running it.

## Common mistakes

- **Handing over in the same step as the question.** Return the handoff only from the confirm message; before the answer nothing may run.
- **Adding `-y` or `--noconfirm`.** The person agreed to one package; the manager's own question shows what else it brings.
- **Assuming it worked.** A non-zero exit code or a cancelled password prompt means nothing was installed; look for the program again.
- **Building the command yourself.** Use `install.command_line()` and `install.handoff(..)`, so the line the person reads is the line that runs.
