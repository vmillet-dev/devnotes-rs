# Getting started

Installing DevNotes, choosing your library’s passphrase, and a tour of the sample notes of the first launch.

## Installing

DevNotes runs on Windows, on macOS (since version 1.2.0) and on Linux. Every file is on [the download page](https://devnotes.fr/en/download), each as a direct link: take the one that matches your system. There is no browser engine inside, so nothing else to install.

**The files of each version**

| System | File | Take it if |
| --- | --- | --- |
| Windows | `…_x64-setup.exe` | you simply want to install it: this is the one |
| Windows | `…-windows.exe` | you want nothing installed |
| macOS | `…_aarch64.dmg` | Macs with an Apple chip (M1 and later) |
| macOS | `…_x64.dmg` | Macs with an Intel processor |
| Linux | `…_x86_64.AppImage` | any distribution, nothing to install |
| Linux | `…_amd64.deb` | Debian, Ubuntu and derivatives |
| Linux | `…x86_64.rpm` | Fedora, RHEL and derivatives |
| Linux | `…-linux` | the bare executable |

The Windows installer installs for your account, without administrator rights. On Linux, an AppImage is made executable before it runs, and a package installs with your distribution’s package manager:

**Installing on Linux**

```sh
chmod +x devnotes_*_x86_64.AppImage       # then run it
sudo apt install ./devnotes_*_amd64.deb   # Debian and Ubuntu
sudo dnf install ./devnotes-*.x86_64.rpm  # Fedora and RHEL
```

> [!TIP]
> Windows shows a SmartScreen warning on first launch: choose **More info**, then **Run anyway**. It is about the publisher’s identity, not the file: DevNotes has no code signing certificate. Its updates, on the other hand, are signed and verified before they are installed.

## Choosing a passphrase

On first launch, DevNotes asks you to *Protect this library*: choose a passphrase of at least twelve characters, and type it twice. It is asked once at each launch, and stored nowhere.

Several unrelated words hold far better than one clever word followed by a digit: `lamp bicycle storm peony` is longer and easier to remember than `P@ssw0rd2026`.

> [!WARNING]
> **There is no recovery.** No account, no reset: a lost passphrase is a lost library. A plain export is the only copy that does not depend on it. See [Security and backups](safety.md).

## The first tour

A new library holds a *Getting started* space with a few sample notes: a welcome note, a *psql connection* snippet with fields to fill, an API response to open as a graph, a *Release day* Note, a snippet to format with Prettier and a *Things to try* list that walks you round:

1. Open a note by clicking its card.
2. Copy *psql connection* and fill its fields.
3. Open the API response as a graph.
4. Find a word in *Release day* with <kbd>Ctrl F</kbd>.
5. Format the code to format with <kbd>Shift Alt F</kbd>.
6. Open the tools (<kbd>Ctrl 2</kbd>) and save a result as a note.
7. Paste a note from another application with <kbd>Ctrl Alt P</kbd>.
8. Switch to the *Board* view and drag a note into a zone.
9. Turn this list into a Note, from the editor’s *Kind* menu.

Tick it off from its card as you go. The sample notes are deleted like any other once read.

## Three areas

The title bar switches from one area to another:

- **Notes** (<kbd>Ctrl 1</kbd>): your snippets, notes and lists, in their spaces and folders.
- **Tools** (<kbd>Ctrl 2</kbd>): some thirty offline utilities.
- **HTTP** (<kbd>Ctrl 3</kbd>): the API client, its collections and environments.

## Updates

DevNotes checks by itself whether a newer version exists and offers it, with its release notes: *Install now*, *Later* or *Do not offer version … again*. The *About* menu also offers to *Check for updates*. Nothing else leaves your machine: the check only asks GitHub whether a new version exists. The Windows installer, the macOS application and the AppImage install the update themselves, signed and verified first; a .deb or .rpm is updated by your package manager.
