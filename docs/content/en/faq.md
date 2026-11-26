# Questions

The price, macOS, syncing, a forgotten passphrase, the Windows warning, and where your data goes.

## Is DevNotes free?

Yes, and open source: it is published under the GPL-3.0 licence, its code is on [GitHub](https://github.com/vmillet-dev/devnotes-rs). No account, no subscription, no advertising.

## Is there a macOS version?

Yes, from the version after 1.1.0: one file for Macs with an Apple chip, another for Intel Macs. On first launch, macOS blocks it, DevNotes having no publisher certificate: open System Settings, Privacy & Security, then Open Anyway.

## Can I sync two computers?

DevNotes syncs nothing: that is what guarantees your notes do not leave your machine. To move notes from one computer to another, export them into a protected archive, then import it on the other side. An import never creates a duplicate.

## I have forgotten my passphrase.

It cannot be recovered: it wraps the key that seals your notes, and nothing else opens it. *Forgotten your passphrase?* archives the library, still sealed, and starts a new one. A plain export kept apart is the only copy that does not depend on it.

## Windows shows a warning on first launch.

Choose **More info**, then **Run anyway**. The warning is about the publisher’s identity: DevNotes has no code signing certificate. Each version publishes `SHA256SUMS.txt` to check the files, and updates are signed and verified before they are installed.

## Does DevNotes work offline?

Entirely. Only the API client, which contacts the servers you give it, and the update check need the network.

## How do I update?

DevNotes offers new versions itself, with their notes, and installs them for the Windows installer and the AppImage. A `.deb` or `.rpm` is updated by your distribution’s package manager. You can also download [the latest version](https://devnotes.fr/en/download) and install it over the old one: your libraries are not touched.

## Will my libraries from an earlier version open?

Yes, as they are. DevNotes is now one native application: no browser engine inside, so nothing like WebView2 or WebKitGTK to install.

## On Linux under Wayland, something is missing.

DevNotes uses X11 whenever it is there, including XWayland. On a Wayland session without XWayland, which is rare, the window’s position is not restored, closing to the tray only minimises, a file cannot be dropped from the file manager, and HTML pasted into a Note keeps its text only.

## Where are my notes?

In your profile folder: `%APPDATA%\com.devnotes.app\` on Windows, `~/Library/Application Support/com.devnotes.app/` on macOS, `~/.local/share/com.devnotes.app/` on Linux. The details are in [Security and backups](safety.md).

## I found a bug, or I have an idea.

Open an issue on [the GitHub repository](https://github.com/vmillet-dev/devnotes-rs/issues), saying what you were doing, what you expected and what happened. The version is in *About*.

> [!TIP]
> Nothing you type in DevNotes is sent with an issue: copy only what helps to understand.

## Ctrl Alt P does nothing.

Another application probably already holds that combination: DevNotes says so at startup. Choose another in *Preferences… → Shortcuts*, or free it in the other application. Global shortcuts work with the window closed as long as DevNotes runs in the system tray; once *Quit DevNotes* is chosen, <kbd>Ctrl Alt P</kbd> has no application left to wake.
