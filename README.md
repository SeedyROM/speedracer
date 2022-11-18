A crate for racing `Future`s and getting ranked results back.

# Example

```rust
use tokio::time::sleep;
use std::time::Duration;

use speedracer::RaceTrack;

let mut race_track = RaceTrack::disqualify_after(Duration::from_millis(500));

race_track.add_racer("Racer #1", async move {
    println!("Racer #1 is starting");
    sleep(std::time::Duration::from_millis(100)).await;
    println!("Racer #1 is ending");

    Ok(())
});
race_track.add_racer("Racer #2", async move {
    println!("Racer #2 is starting");
    sleep(std::time::Duration::from_secs(200)).await;
    println!("Racer #2 is ending");

    Ok(())
});
race_track.add_racer("Racer #3", async move {
    println!("Racer #3 is starting");
    sleep(std::time::Duration::from_secs(700)).await;
    println!("Racer #3 is ending");

    Ok(())
});

race_track.run().await;
let rankings = race_track.rankings();

println!("Rankings: {:?}", rankings);

assert_eq!(rankings[0].name, "Racer #1");
assert_eq!(rankings[1].name, "Racer #2");
assert_eq!(rankings[2].name, "Racer #3");
assert_eq!(rankings[2].disqualified, true);

```
