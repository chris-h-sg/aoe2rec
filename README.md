# aoe2rec

A Rust library for reading and writing Age of Empires 2 recorded game files.

## Differences from upstream ([`aoe2ct/aoe2rec`](https://github.com/aoe2ct/aoe2rec))

This fork keeps the same API and output format as the original library, while adding support for more game modes and extra match data:

- **Works with AI and single-player games:** Parses matches against the AI, campaign missions, resumed saves, and shared-control games (the original repo primarily targets standard PvP multiplayer).
- **More forgiving parser:** Skips over unknown or corrupted commands instead of failing the entire replay when AoE2 updates or adds new actions.
- **Extra game data:** Extracts starting units/buildings on the map, periodic per-player resource and unit counts, cheat usage, and an optional `--typed` / `get_typed()` mode that decodes building placement, production queues, and unit deletions.

## Usage

### Rust

Add this to your `Cargo.toml`:

```toml
[dependencies]
aoe2rec = "0.10"
```

### CLI (`aoe2js`)

```bash
# Full savegame JSON
cargo run --package aoe2js -- "replay.aoe2record"

# Full savegame JSON with decoded Build, Queue, and Delete actions
cargo run --package aoe2js -- --typed "replay.aoe2record"

# Match summary JSON
cargo run --package aoe2js -- --summary "replay.aoe2record"
```

### JavaScript / WASM (`aoe2rec-js`)

```js
import { parse_rec, parse_rec_typed, parse_rec_summary } from "aoe2rec-js";

const buffer = await file.arrayBuffer();

// Full parsed recording (raw byte arrays for Build, Queue, and Delete)
const rec = parse_rec(buffer);

// Full parsed recording with decoded fields for Build, Queue, and Delete
const typedRec = parse_rec_typed(buffer);

// Lightweight match summary (header, settings, teams, players, duration)
const summary = parse_rec_summary(buffer);
```

## Build the WASM package

```
cargo install wasm-pack
cd crates/aoe2rec-js
wasm-pack build --target bundler
```

This will create a `pkg` directory inside `crates/aoe2rec-js/`.

## License

This project is licensed under the [MIT](./LICENSE) license.

## Credits

This project is largely an adaptation of [aoc-mgz](https://github.com/happyleavesaoc/aoc-mgz). Another source of inspiration is the [genie-rs](https://github.com/SiegeEngineers/genie-rs/) project.
Many thanks to [happyleaves](https://github.com/happyleavesaoc) and [simonsan](https://github.com/simonsan), without whom this project would not exist.
