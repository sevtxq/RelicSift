# RelicSift

A conservative **Honkai: Star Rail** relic analyzer. Local, offline, and no game access.

**Keep** valuable pieces · **Potentially Useful** preserves uncertainty · **Safe Discard** requires strong evidence.

Import a Reliquary archive, inspect explained recommendations, and export a new file. Protection comes before scores; manual decisions survive reanalysis.

**Run on Windows** with Rust and the Microsoft C++ Build Tools installed:

```powershell
cargo run --locked --release
```

V1 covers 18 reviewed sets and 14 archetypes. Unknown sets stay protected; upgrade forecasts use explicit model assumptions.

Generate local scoring documentation with `cargo run --bin sync_rules`. The `docs/` folder is intentionally untracked.

[MIT](LICENSE) · Independent fan project. Not affiliated with HoYoverse.
