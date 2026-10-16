# Installation

pd200x-control runs on Linux with PipeWire (Fedora, Ubuntu 22.10 and later, Debian 12, Arch, openSUSE,
Bazzite, …) under KDE, GNOME or any other desktop. Download it from the
[latest release](https://github.com/firsttris/pd200x-control/releases/latest).

## AppImage (any distribution)

```sh
chmod +x pd200x-control_*_amd64.AppImage
./pd200x-control_*_amd64.AppImage
```

The AppImage needs the udev rule below once, so your user may talk to the microphone.

## .deb (Debian, Ubuntu, Linux Mint)

```sh
sudo apt install ./pd200x-control_*_amd64.deb
```

## .rpm (Fedora, openSUSE)

```sh
sudo dnf install ./pd200x-control-*.x86_64.rpm
```

The `.deb` and `.rpm` packages install the udev rule for you; plug the microphone in again afterwards.

## The udev rule

The app talks to the microphone through its `/dev/hidraw*` device. Many systems only let root open
it. The rule gives the logged-in user access to the PD200X and nothing else:

```sh
curl -LO https://raw.githubusercontent.com/firsttris/pd200x-control/main/packaging/99-pd200x.rules
sudo cp 99-pd200x.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```

Then unplug the microphone and plug it in again. Without the rule the app shows
"No permission for the microphone" with these commands.

??? note "On immutable systems (Bazzite, Silverblue, Kinoite)"
    `/etc/udev/rules.d/` is writable on these systems too; the commands above work as they are.
    Use the AppImage, or build in a toolbox (see [Development](development.md)).

## The command line tool

The CLI `pd200x` does everything the app does, from a terminal or a script:

```sh
tar -xzf pd200x-cli-*-x86_64-linux.tar.gz
install -m 755 pd200x ~/.local/bin/
pd200x status
```

See [Command line](command-line.md).

## What else is needed

- **PipeWire tools** for the system microphone volume and the test recording: `wpctl`, `pw-record`,
  `pw-play` (and `pw-dump`, which is optional). They come with PipeWire and WirePlumber on every
  distribution that uses PipeWire.
- **A tray** is optional. On desktops with a StatusNotifier tray (KDE, GNOME with the AppIndicator
  extension) the app adds an icon with mute, open and quit; it needs `libayatana-appindicator3`,
  which the AppImage brings along. Without it, closing the window quits the app.

## Building from source

See [Development](development.md): `just setup && just build`, or `just install` to put the AppImage
and the CLI into `~/.local/bin` with a desktop entry.

## Uninstalling

Delete the AppImage (or `sudo apt remove pd200x-control` / `sudo dnf remove pd200x-control`), the
settings in `~/.config/pd200x-control/` and, if you installed it by hand,
`/etc/udev/rules.d/99-pd200x.rules`. The microphone keeps its settings.
