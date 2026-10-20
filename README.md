<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/user-attachments/assets/173185ed-3b03-4e11-8e67-30436fa3cf98">
    <source media="(prefers-color-scheme: light)" srcset="https://github.com/user-attachments/assets/26e5455b-42ef-4deb-8e3e-4d78115ab6b6">
    <img src="https://github.com/user-attachments/assets/26e5455b-42ef-4deb-8e3e-4d78115ab6b6" alt="RelicSift" width="640">
  </picture>
</p>

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
