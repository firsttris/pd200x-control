# Command line

`pd200x` does everything the app does. It shares the settings with the app
(`~/.config/pd200x-control/settings.json`), so both show the same profiles.

```text
pd200x status [--json]            what the microphone reports and has, the system volume
pd200x level [--seconds 10]       live level meter in the terminal
pd200x gain <0-100>               hardware gain
pd200x mute on|off|toggle
pd200x monitor on|off             headphone monitoring
pd200x headphone <0-100>          headphone volume
pd200x nc on|off|<0-40>           noise cancellation, or its strength (turns it on)
pd200x eq list                    the presets and your own profiles
pd200x eq <preset|profile>        apply one: flat, low_cut, mid_boost, low_cut_mid_boost,
                                  bright, natural, classic, deep, or a profile name
pd200x compressor off|<-40…0>     compressor threshold in dB
pd200x limiter off|<-80…0>        limiter threshold in dB
pd200x light on|off|<colour>      white, red, orange, yellow, green, cyan, blue, purple, rainbow
pd200x light brightness <0-100>
pd200x light linkage on|off       light off while muted
pd200x boost [<0-18>|off]         system microphone volume above 100 %, in dB; without a value shows it
pd200x protect [on|off]           keep apps (Teams, Slack, browsers) from changing the volume
pd200x calibrate [--seconds 5] [--apply]
                                  measure your speech, propose gain and system volume
pd200x restore                    send the saved sound and light to the microphone again
```

## Examples

```console
$ pd200x status
PD200X  connected
  Mute          off
  Gain          100 / 100
  Monitor       on
  Headphone     41 / 100
  Noise cancel  off (strength 34 / 40)
  Boost         +14.6 dB (PipeWire)
  Tone          natural, compressor off, limiter -3 dB
  Light         cyan, 70 %

$ pd200x calibrate
Speak normally into the microphone for 5 seconds …
Speech peaks at -21.5 dB.
Proposal: gain 100 / 100, boost +12.5 dB → peaks at about -9.0 dB.
Run with --apply to use it.

$ pd200x boost 14.6        # the 175 % from the desktop's sound settings
$ pd200x light cyan && pd200x light brightness 70
```

`--json` on `status` gives the live state, the system volume in dB and the settings for scripts:

```sh
pd200x status --json | jq '.live.gain'
```

## Exit codes and errors

`pd200x` exits with 0, or with 1 and a message on stderr: no microphone, no permission (the
[udev rule](installation.md#the-udev-rule) is missing), PipeWire tools missing, a value out of range.

## Without hardware

`PD200X_SIMULATOR=1 pd200x status` runs against a simulated microphone, for trying it out and for the
tests.
