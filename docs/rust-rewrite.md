# Rust rewrite — plan

Decided: **egui/eframe**, backends **CamillaDSP + Equalizer APO + PipeWire filter-chain**, CamillaDSP
driven over its **WebSocket control API**, built **core-first** in `rust/` with the C# app left alone
on `main`.

## Layout

```
rust/
  Cargo.toml            # workspace
  heq-core/             # no UI, no async runtime, testable
    dsp/{biquad,loudness}.rs
    model/{band,eq_model,ab}.rs
    storage/{preset,correction,settings,store}.rs
    backend/{mod,snapshot,apo,camilla,pipewire}.rs
  heq-app/              # eframe binary
    curve/{mod,render,input}.rs
    panels/{band_card,library,ab,correction}.rs
    theme.rs
```

`src/Heq` stays untouched until the Rust app reaches parity.

Dependencies, kept deliberately short: `serde`, `serde_json`, `serde_yaml`, `dirs`,
`tungstenite` (no tokio — one blocking worker thread), `eframe`/`egui`, `thiserror`. Windows registry
lookup via `windows-registry` behind `#[cfg(windows)]`.

## What changes shape, and why

**Observers become a revision counter.** `INotifyPropertyChanged` → `EqModel.revision: u64` bumped by
every mutating method, with `Batch` as a `hold` depth exactly as today. egui redraws every frame, so
the UI never needs a change event; the only consumer of `Changed` was the 140 ms apply debounce, which
becomes "revision != last_applied_revision && idle 140 ms" in the backend worker.

**`_suppress` disappears.** There is no programmatic-update echo in immediate mode — the model is read
each frame and written only by real input. This removes the single largest source of UI bugs in the
current design and nothing replaces it.

**Selection becomes an id.** `EqBand` references (`_selected`, `_hovered`, `_dragBand`, `_justAdded`,
A/B's parked band lists) cannot be `&EqBand` in Rust. Bands get a `BandId(u64)` assigned on insert;
selection, hover and drag all hold `Option<BandId>`. Ids are not serialised.

**`AbTester` borrows rather than owns.** It keeps the parked `Preset` and cached level, and every
method takes `&mut EqModel`. Loudness matching stays unconditional.

**The preamp split is preserved verbatim**: `base_preamp_db` (typed or auto), `loudness_trim_db`
(≤ 0), `effective_preamp_db` = clamped sum, and only the effective value reaches a backend.

## Backends

```rust
pub struct Snapshot {           // everything a backend needs, no model borrow
    pub bands: Vec<EqBand>,     // user bands
    pub correction: Vec<EqBand>,// the group's left/right fix
    pub preamp_db: f64,         // effective
    pub bypassed: bool,
    pub sample_rate: f64,
}

pub trait Backend {
    fn name(&self) -> &str;
    fn apply(&mut self, s: &Snapshot) -> Result<Applied, BackendError>;
    fn status(&self) -> Status;         // what the status bar shows
}
```

**APO** (`#[cfg(windows)]` for device/registry parts, format code compiled everywhere so it can be
tested on Linux). Straight port of `ApoFormat`, `ApoWriter`, `ApoInstall`: same CRLF, same
`{:.2}`-style trimming, same atomic temp-file write, same rule that an existing `Include:` is reported
and never moved. Rust has no `CultureInfo` hazard, but keep every number formatted through one helper
so the output stays diffable against the C# writer.

**CamillaDSP.** Bands map onto config `filters`:

| heq | CamillaDSP |
|---|---|
| Bell | `Biquad` / `Peaking` (freq, gain, q) |
| Low/High Shelf | `Lowshelf` / `Highshelf` |
| Notch, BandPass, AllPass | `Notch`, `Bandpass`, `Allpass` |
| Low/High Cut ≤ 12 dB/oct | `Highpass` / `Lowpass` with the band's Q |
| Low/High Cut > 12 dB/oct | one `Highpass`/`Lowpass` per Butterworth section, our own Qs |
| preamp | a `Gain` filter, first in the pipeline |
| disabled band | omitted from the pipeline |

Two `Filter` pipeline steps, channel 0 and 1; `Both` bands go in each, `Left`/`Right` bands only in
theirs — the same partition `ApoFormat` does with `Channel:` lines. Because both backends build from
`Biquad`'s own RBJ coefficients, the curve on screen stays the curve you get.

Transport: a worker thread owns a `tungstenite` connection to `ws://127.0.0.1:1234`, receives
`Snapshot`s on a channel, debounces 140 ms, and sends `SetConfigJson`. On no connection it writes the
YAML to the configured path and reports "not connected" rather than failing. Target the CamillaDSP v3
schema and probe `GetVersion` on connect, refusing older majors with a clear status message.

**PipeWire.** Writes `~/.config/pipewire/pipewire.conf.d/99-heq.conf` as a `filter-chain` of
`bq_peaking` / `bq_lowshelf` / `bq_highshelf` / `bq_highpass` / `bq_lowpass` / `bq_notch` /
`bq_allpass` nodes. This backend cannot update live — it needs `systemctl --user restart pipewire` —
so it is an export target with an explicit "restart PipeWire to hear this" status, not the default on
Linux.

## Storage

Byte-compatible with the C# library, so `%APPDATA%\heq` and `~/.config/heq` hold the same files and a
preset written by either app loads in the other. That compatibility is checked by loading a real
preset in the running app, not by a round-trip test:

- `BandDto` keeps field names `Type/Freq/Gain/Q/Slope/Enabled/Channel`; enums serialise as the same
  strings (`#[serde(rename_all = "PascalCase")]` plus explicit variant names).
- Two levels only: `presets/<group>/<preset>.json`, `_correction.json` reserved and filtered out.
- `MigrateLoosePresets` ports as-is.
- Roots: `%APPDATA%\heq` on Windows, `~/Library/Application Support/heq` on macOS,
  `$XDG_CONFIG_HOME/heq` on Linux.
- heq still ships no presets and creates none by itself.

## UI

`curve/` is the port that matters: one egui `Painter` over a `Response` rect, identical geometry
(log x 20 Hz–20 kHz, linear y over `±DbRange`, 18 px bottom / 30 px right axis gutters), identical
hit radii, identical drag modes (freq+gain, Q on right-drag), the shift-ghost preview, the hover
badge, the band context menu. WPF `Pen` freezing has no analogue; strokes become plain `Stroke`
values.

`theme.rs` replaces `Theme.xaml` — a `Visuals` built once with heq's palette, plus small helpers for
the pieces egui has no widget for (the A/B pill, list rows, the number fields). An embedded font is
required; egui's default will not match the current look.

Custom chrome: `eframe` with decorations off and a hand-drawn title bar, which also removes the
Win32-specific `DarkTitleBar`. Expect a round of platform fiddling here on Linux CSD.

## Phases

There is no test project here either — the work is iterative, checked by reading the generated
config and by running the app, exactly as the C# side is.

1. **`heq-core` DSP + model.** `Biquad`, `Loudness`, `EqBand`, `EqModel`, `AbTester`.
2. **Storage.** Same JSON on disk as the C# app writes.
3. **Backends.** APO first, because its output is text you can diff against the `heq.txt` the current
   app produces. Then CamillaDSP, then PipeWire.
4. **`heq-app` shell.** Window, theme, curve view, band card. Runnable and audible on Linux here.
5. **Library, A/B, correction panels.** Parity with the WPF partials.
6. **Packaging.** `flake.nix` gains a Rust toolchain devShell and a `heq` package built with
   `rustPlatform.buildRustPackage`; the dotnet shell stays for the C# app.

Phases 1–3 land with no UI and no way to touch the user's real config, so they are safe to build and
test on this machine. From phase 4 on, a run writes real audio config.

## Open questions

- CamillaDSP does not exist as a system service by default — does heq launch and supervise it, or
  assume the user already runs it? Assumed the latter for now.
- Windows: the Rust app keeps APO, but should it also offer CamillaDSP there? Deferred.
- The C# app is deleted only when the Rust one reaches parity, on a separate commit.
