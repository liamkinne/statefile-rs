# statefile

Store application state as a file on disk. Designed to aid adding persistent
state to simple systemd services without needing to reach for heavier solutions
like sqlite.

## Example

```rust
use statefile::File;

// you must specify at least these derivations
#[derive(Serialize, Deserialize, Default)]
struct State {
    foo: String,
    bar: u32,
}

// create or open state file at given path
let mut state = File::<State>::new("mystate.json").await?;
// if the file doesn't exist or is empty, State will contain default values

let mut write_guard = file.write().await; // grab write access
write_guard.id = "".to_string();
write_guard.bar = 10;
drop(write_guard); // write state by explicitly dropping
```
