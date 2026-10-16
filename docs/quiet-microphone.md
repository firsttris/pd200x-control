# A quiet microphone

**The PD200X and PD200XS are dynamic microphones, and over USB they are quiet, even at full gain.**
That is not a Linux problem and nothing is set wrong: dynamic capsules give little signal, and the
preamp in the microphone stops at about +42 dB. On Windows people raise MAONO Link's gain and their
input volume the same way.

## Check where you are

Speak normally and watch the live level on the basics page. Speech peaks belong at **−12 to −6 dB**.
With the gain at 100 and the system volume at 100 % many people land around −20 dB; others then hear
you too quietly, and apps with automatic gain raise the noise with you.

## What helps, in this order

1. **Speak into the top, close.** The PD200X is end-address: speak into the top of the microphone,
   not its side, from 3 to 8 cm. Halving the distance gives about +6 dB, more than anything else, and
   without extra noise.
2. **Gain up.** The hardware gain adds less noise than software amplification. Use it first.
3. **System volume above 100 %.** Raise *Microphone volume (system)* on the basics page; 150 % is
   about +10.6 dB, 175 % about +14.6 dB, 200 % +18 dB. KDE's slider stops at 150 %, the app goes to
   200 %. It is the same setting, so KDE then shows the new value.
4. **Or let the app do it.** **Find my level** measures five seconds of speech and proposes both
   values for peaks near −9 dB.
5. **Check with a test recording.** It contains the system volume and tells you whether the result is
   quiet, well balanced or too loud.

!!! warning "Calls turn it down again"
    Teams, Slack and browser calls have an automatic gain control that turns the system microphone
    volume down, often to about 80 %, and leaves it there. Turn on **Protect from apps** next to the
    volume on the basics page, and the limiter as a safety net.

!!! tip "Compressor and limiter"
    With a raised system volume, loud laughs can clip. Turn on the **limiter** on the sound page
    (around −3 dB). The **compressor** (around −20 dB) evens out quiet and loud passages.

## Why the headphones do not get louder

The headphone monitoring comes straight from the microphone, before the computer. The system volume
is applied in PipeWire, after it, so you do not hear it in the monitoring. The test recording does
contain it.

## XLR

The PD200X also has an XLR output. An audio interface with a strong preamp (60 dB or more), or an
inline booster such as a Cloudlifter, solves the level problem in hardware. The app's settings (EQ,
light, noise cancellation) only apply over USB.
