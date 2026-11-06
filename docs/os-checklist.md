# OS checklist

What no automated test can drive: the system's own tray, dialogs, keyboard and packaging. It is
passed by hand once per release, on Windows 11 at 100 % and 150 %, Ubuntu GNOME under Wayland and
under X11, and KDE (plan § 8.5). Each pass is recorded below.

## The checks

| #   | Check                                                                                       |
| --- | ------------------------------------------------------------------------------------------- |
| 1   | Clean install: the package installs, the application opens on a new profile                 |
| 2   | Update from the previous egui version: offered, signature checked, installed, relaunched    |
| 3   | Tray: the icon shows, its menu is in the language on screen, Open reveals, Quit quits       |
| 4   | Close to tray, minimise to tray, and with no tray a close that ends the application         |
| 5   | Global shortcut: palette, new note and capture, from another application                    |
| 6   | Global shortcut held by another application: said in the banner and under its field         |
| 7   | Autostart: the switch writes the entry, the session starts the application, removal is read |
| 8   | Second instance: a second launch brings the first window forward and exits                  |
| 9   | Notification: a copy from the palette over another application shows a toast                |
| 10  | File dialogs: open and save; on Linux, a desktop without a portal says so                   |
| 11  | File drop from the file manager onto an open note                                           |
| 12  | Image paste into an open note; copy of the QR code as an image                              |
| 13  | IME: French accents, Japanese                                                               |
| 14  | Screen reader: the main flow, unlock, a note opened, a note written                         |
| 15  | Full screen of Present: taken, and given back as it was                                     |
| 16  | Two screens of different DPI: the window moves between them, its geometry comes back        |

## Passes

A cell says **ok**, **ko** with what happened, or **—** for not run. « Driven » means run on the
real window by a script, through system messages or synthetic keys, and not by a person's hands.

### 0.26.0 — first pass, partial

Windows 11 only, at the screen's own scale, by Claude Code on 2026-11-06. No Linux desktop was
available, and no package exists before phase 11: nothing here stands for a pass by a person.

| #   | Windows 11                                                                                           | Ubuntu GNOME Wayland | Ubuntu GNOME X11 | KDE |
| --- | ---------------------------------------------------------------------------------------------------- | -------------------- | ---------------- | --- |
| 1   | —                                                                                                    | —                    | —                | —   |
| 2   | —                                                                                                    | —                    | —                | —   |
| 3   | ok, driven: Open and Quit from the menu, a left click reveals; the icon and the labels not looked at | —                    | —                | —   |
| 4   | ok, driven: close and minimise hide, the tray brings back; the case with no tray is kittest only     | —                    | —                | —   |
| 5   | palette ok, driven, from a hidden window; new note and capture are kittest only                      | —                    | —                | —   |
| 6   | ok, driven: refused with another copy holding the keys, read in the log; the banner is unit-tested   | —                    | —                | —   |
| 7   | entry written, read and removed, driven; a real session start not run                                | —                    | —                | —   |
| 8   | ok, scenarios and `app` tests                                                                        | —                    | —                | —   |
| 9   | the call returned without an error; the toast not looked at                                          | —                    | —                | —   |
| 10  | open shown and cancelled, driven: nothing attached, nothing said; save not run                       | —                    | —                | —   |
| 11  | —                                                                                                    | —                    | —                | —   |
| 12  | image paste ok, driven: a real Ctrl+V attached the image; QR copy not run                            | —                    | —                | —   |
| 13  | —                                                                                                    | —                    | —                | —   |
| 14  | —                                                                                                    | —                    | —                | —   |
| 15  | ok, driven: the window took the whole screen and came back to the same rectangle                     | —                    | —                | —   |
| 16  | —                                                                                                    | —                    | —                | —   |
