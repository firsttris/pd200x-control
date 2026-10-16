# FAQ and troubleshooting

## Is there a Linux version of MAONO Link?

No, MAONO Link runs on Windows only. pd200x-control is an independent open-source app that speaks the
same USB protocol and sets the same things on Linux: gain, monitoring, noise cancellation, EQ
profiles and bands, compressor, limiter and the ring light. It does not update the firmware; use
MAONO Link on Windows for that.

## Does it work with the PD200XS?

Yes. The PD200X and PD200XS have the same USB id (`352f:0104`) and the same firmware interface.

## Why is my microphone so quiet on Linux?

Because it is a dynamic microphone; over USB it is quiet on every system, even at full gain. See
[A quiet microphone](quiet-microphone.md): speak close into the top, gain to 100, the system volume
above 100 %, or let **Find my level** set it.

## Is "Microphone volume (system)" the same as the volume in KDE or GNOME?

Yes. It is the volume of the microphone's PipeWire input, which every desktop's sound settings show.
The app only goes further than KDE's 150 %, up to 200 %.

## Why is my microphone volume back at 80 % after a call?

Teams, Slack and browsers use WebRTC's automatic gain control, which turns the system microphone
volume down during a call and does not restore it. Turn on **Protect from apps** on the basics page
(or `pd200x protect on`): PulseAudio clients may then no longer change it. Your level inside the call
stays fine, the apps adjust it internally. Restart running browsers and call apps once after
switching: the restart of `pipewire-pulse` cuts their audio connection.

## Does the app have to keep running?

No. The microphone stores its settings itself; the system volume is kept by WirePlumber. Close the
app when you are done.

## Does it work with XLR?

The app's settings apply to the USB connection only. Over XLR the microphone is a plain dynamic
microphone for your audio interface.

## Does it work with PulseAudio?

The microphone settings, EQ and light work everywhere. The system microphone volume and the test
recording need PipeWire (`wpctl`, `pw-record`, `pw-play`); without them the app says so next to the
control.

## Troubleshooting

### "No PD200X found"

- Is the microphone connected over **USB** (not only XLR)?
- `lsusb | grep 352f` should list `PD200X Podcast Microphone`. If not, try another cable or port.

### "No permission for the microphone"

The [udev rule](installation.md#the-udev-rule) is missing or the microphone was not plugged in again
after installing it. Check with `ls -l /dev/hidraw*` and
`grep -l PD200X /sys/class/hidraw/*/device/uevent`.

### "PipeWire does not know the microphone as an input"

The system volume and the test recording use the PipeWire source of the PD200X. Check that it
appears under *Sources* in `wpctl status`. On a fresh plug-in it can take a second.

### "wpctl or pw-dump is missing"

Install your distribution's PipeWire tools (`pipewire-utils` on Fedora, `pipewire-bin` and
`wireplumber` on Debian/Ubuntu).

### The test recording fails

`pw-record` and `pw-play` must be installed (they come with the PipeWire tools). The recording uses
the PD200X directly, not the default microphone.

### The tray icon is missing

The desktop needs a StatusNotifier tray (KDE has one; on GNOME install the AppIndicator extension).
Without one, the app runs without a tray and closing the window quits it.

### "Light off while muted" shows the wrong state

The microphone reports this setting with values that do not match what was written, so the app
remembers it itself. If you changed it in MAONO Link on Windows, set it once in the app.

### Where are the settings?

`~/.config/pd200x-control/settings.json`: your own profiles, the last EQ, dynamics and light the app
set, and the options. Delete the file to start fresh; the microphone keeps its own settings.
