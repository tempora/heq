# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

heq is a FabFilter Pro-Q style parametric EQ for headphones that writes into Equalizer APO's
config. One WPF project, `net10.0-windows`, **no NuGet dependencies** — keep it that way. C# with
`Nullable` and `ImplicitUsings` disabled, so write explicit `using` lines and no `?`-annotated types.

## Commands

```
dotnet build src/Heq/Heq.csproj -c Release        # build
dotnet run   --project src/Heq/Heq.csproj         # build and run
dotnet publish src/Heq/Heq.csproj -r win-x64 --self-contained true -o dist
```

There is **no test project**.

## Verifying changes

**A clean build is the default and usually the only check.** Do not launch the app, drive it with UI
automation, or take screenshots unless the user explicitly asks you to run or test it. This is a
desktop app on the user's own machine: launching it writes to the *real* Equalizer APO config and the
*real* preset library in `%APPDATA%\heq`, and screenshots capture the user's whole window.

When the user does ask for a run:

- One launch, one look. Do not re-verify a thing that already worked, and do not screenshot every
  intermediate state — reason about the code instead.
- Read state through UI Automation (`AutomationId` matches the XAML `x:Name`) rather than by
  screenshotting and squinting at pixels. Screenshot only when the question is genuinely visual.
- Where the output is what matters, read the generated `heq.txt` in APO's config directory
  (`C:\Program Files\EqualizerAPO\config`) — that is the real result, and it is text.
- Back up `%APPDATA%\heq\settings.json` first, and delete any folders or presets you created when you
  are done. The user's library must be left exactly as you found it.
- Close the app with `CloseMainWindow()`, not `Stop-Process` — heq saves its settings on close, and
  killing it loses the working EQ.

If a change is risky enough to want a run and the user has not asked for one, say so and let them
decide.

## Architecture

### The spine: edit → debounce → config file

Every mutation ends up in `heq.txt`, through one path worth knowing by heart:

`EqBand` (INotifyPropertyChanged) → `EqModel.OnBandPropertyChanged` → `EqModel.Touch()` →
`EqModel.Changed` → `MainWindow.OnModelChanged` → `AbTester.Refresh()` + UI refresh +
`ScheduleApply()` → 140 ms `DispatcherTimer` → `Apply()` → `ApoWriter.WriteEq` →
`ApoFormat.BuildConfig` → atomic write via temp file.

So *anything* that changes a band property automatically rewrites the config; nothing needs to call
the writer itself. The debounce exists so a drag does not make APO reload every frame.

### Preamp is layered, and the layers matter

- `EqModel.BasePreampDb` — what the user typed, or what auto gain derives from the curve peak.
- `EqModel.LoudnessTrimDb` — the A/B level match. Always ≤ 0, so matching cannot cause clipping.
- `EqModel.EffectivePreampDb` = base + trim, clamped. **This is what gets written to APO.**

The UI shows `BasePreampDb` in the preamp field. Showing the effective value there would fold the
A/B trim into the user's preamp the next time the field commits. If you touch preamp code, keep that
split.

### A/B (`Model/AbTester.cs`)

The side you are hearing *is* the live `EqModel`, so it stays editable; the other side is parked as a
`Preset` snapshot with its measured level cached. A switch stashes the live model into the snapshot
and applies the parked one. `_busy` guards against the model's own change notifications re-entering
while a switch is in progress.

Loudness matching is unconditional by design — an unmatched A/B compares volume, not tuning. Do not
add a switch for it. `Dsp/Loudness.cs` measures a side the way ITU-R BS.1770 measures programme
loudness: the curve applied to pink noise (equal power per octave, so a log frequency grid weights
every bin equally), K-weighted, **plus that side's `BasePreampDb`**. The louder side is trimmed down
to the quieter.

### The display (`Controls/EqCurveView.*.cs`)

A single `FrameworkElement` drawn entirely in `OnRender` — no child visuals, so dragging stays cheap.
Three partials: the core (state, properties, geometry), `.Render` (everything drawn) and `.Input`
(mouse, keyboard, hit-testing, the band menu). X is log frequency (20 Hz–20 kHz), Y is linear dB over
`±DbRange`. Anything drawn on the curve goes here; anything floating over it goes in
`MainWindow.xaml`.

`Overlay` is a second `EqModel` folded into the drawn totals but given no handles — the group's
left/right correction, so the curve on screen stays the curve APO is given.

### DSP (`Dsp/Biquad.cs`, `Model/EqBand.cs`)

RBJ cookbook coefficients, the same ones APO applies internally, so the curve on screen is the curve
you get. A band expands to one or more `Biquad` sections; cuts above 12 dB/oct become a Butterworth
cascade (one `HPQ`/`LPQ` line each) and their Q is fixed by the alignment, so the Q field is disabled
there.

### Storage (`Storage/`)

The library is exactly two levels: `%APPDATA%\heq\presets\<folder>\<preset>.json`. **The interface
calls a folder a group**, and the presets in it are that group's presets — usually a headphone and
its tunings, but never say "headphone" in user-visible text. `PresetStore` is a static API over
`(folder, name)` pairs; `MigrateLoosePresets()` moves files written by the pre-folder version into
`Unsorted` on first run. One type per file; `BandDto` is the only place the stored band shape is
defined.

A group also holds `_correction.json` — its left/right fix, applied to every preset in it. It lives
inside the folder so renaming, moving or deleting a group carries it along, and `ListPresets` filters
the reserved name out.

**heq ships with no presets and must never create one by itself.** The library holds what the user
saved and nothing else. Groups are only created when the user names one.

### APO integration (`Apo/`)

heq owns `heq.txt` and rewrites it in full. It adds one `Include: heq.txt` line to `config.txt`, after
backing that file up once to `config.txt.heq-backup`. **An include that already exists is never
moved** — APO's own Editor and Peace also rewrite `config.txt`, and heq cannot tell a wrong position
from a deliberate one, so it reports what it found in the status bar instead. `config.txt` is touched
only at startup and on device change.

All APO output goes through `CultureInfo.InvariantCulture`. A comma decimal separator on a
non-English Windows would corrupt the config.

## UI conventions

- **`_suppress` in `MainWindow`** guards every programmatic control update. Handlers return early
  when it is set; without it a UI refresh echoes back into the model. Set it with `using
  (Suppressed())`, which restores the previous value so nested blocks nest correctly.
- **`MainWindow` is split by area** — the core window, plus `.Library`, `.Ab` and `.Correction`
  partials. Put new work in the partial that owns the concern.
- **`Ui/Theme.xaml` holds every control template.** There is no third-party UI library and no default
  WPF chrome anywhere — a new control needs a style here or it will look like Windows 98.
- **Set `IsSynchronizedWithCurrentItem="False"`** on any `ComboBox`/`ListBox` fed from `ItemsSource`.
  Otherwise the default collection view can reset the selection asynchronously, firing your
  `SelectionChanged` handler after `_suppress` has already been cleared.
- **Toggles whose visual must match app state** (the A/B pill) drive their template through
  `VisualStateManager` states and are re-pinned from code with `GoToState`. Trigger `EnterActions`/
  `ExitActions` storyboards can outlive a state set twice in one dispatcher pass and leave the thumb
  on the wrong side. Give those groups **no `VisualTransition`**: a generated transition runs before
  the state's own storyboard, so the two together take twice as long and the switch reads as lagging.
- **Freeze WPF objects once, at creation.** `EqCurveView.Stroke()` is the only place a `Pen` is built
  and it takes every option a caller needs, because setting a property on a frozen `Freezable` throws
  at render time — which means it crashes the app, not the line.
- The interface carries **no instructional text**. The hover preview shows what a click will do, the
  band card shows what a band is; anything that genuinely needs explaining goes in the README.

## Working on this repo

`TODO.md` is the backlog and drives the work. Tick an item only when it is genuinely finished, and
edit nothing else in that file unless asked.

**Never edit `README.md` unless the user asks for it.** It is the user's own writing about their own
app. If a change makes something in it stale, say so and let them decide.

**Keep documentation short. The code is the documentation.** Write the few lines a reader cannot
work out for themselves — a command to run, a constraint they would otherwise trip over — and stop.
Do not restate what a signature already says, narrate a design, or add a comment to explain code
that could just be clearer. The same goes for anything user-facing: no instructional text in the
interface, and tooltips only where a control is icon-only, kept to a few words.

**No XML doc comments in C#**, and **comments in C# and XAML are section labels only** — `// mouse`,
`<!-- ==== floating band card ==== -->` — not an account of what the code does or why you wrote it.
A prose comment above a statement is nearly always the wrong fix; name the thing better instead.

Two narrow exceptions: a short trailing note giving a unit or a magic value its meaning
(`// headroom for inter-sample peaks`), and a line that stops a plausible edit from silently
breaking something (an empty `catch`, the background rect the hit-testing depends on). Everything
else that has to survive belongs in this file.

When a TODO item is short or could point at more than one part of the interface, say which control
you are about to change and confirm before writing code. "Icons for the type of filter instead of
text box" meant the badge on the hover preview, not the picker on the band card.
