# nix

```
nix develop                     # dotnet SDK 10 + wine, EnableWindowsTargeting set
nix build                       # heq.exe, self-contained win-x64  (needs deps.json, below)
nix run                         # build, then launch it under wine
```

`heq.nix` pins NuGet content through `deps.json`, which is not in the repo yet. Generate it once
(and again whenever the SDK version or a package reference changes):

```
nix build .#heq.passthru.fetch-deps && ./result nix/deps.json
```

heq is a WPF app: it builds anywhere, but only runs on Windows or under wine, and it writes into
Equalizer APO's config directory — which under wine means the prefix's
`drive_c/Program Files/EqualizerAPO/config`.
