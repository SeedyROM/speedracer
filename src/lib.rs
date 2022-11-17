//! A crate for racing `Future`s and getting ranked results back.
//!
//! # Example
//!
//! ```
//! use tokio::time::sleep;
//! use std::time::Duration;
//!
//! use speedracer::RaceTrack;
//!
//! #[tokio::main]
//! async fn main() {
//!    let mut race_track = RaceTrack::disqualify_after(Duration::from_millis(300));
//!   
//!    race_track.add_racer("Racer #1", async move {
//!       sleep(std::time::Duration::from_millis(100)).await;
//!       Ok(())
//!    });
//!    race_track.add_racer("Racer #2", async move {
//!       sleep(std::time::Duration::from_secs(200)).await;
//!       Ok(())
//!    });
//!    race_track.add_racer("Racer #3", async move {
//!       sleep(std::time::Duration::from_secs(700)).await;
//!       Ok(())
//!    });
//!   
//!    race_track.run().await;
//!    let rankings = race_track.rankings();
//!   
//!    assert_eq!(rankings[0].name, "Racer #1");
//!    assert_eq!(rankings[1].name, "Racer #2");
//!    assert_eq!(rankings[2].name, "Racer #3");
//!    assert_eq!(rankings[2].disqualified, true);
//! }
//!
//! ```

use std::{collections::BTreeMap, pin::Pin, time::Duration};

use futures::Future;

/// Simple type alias for a Boxed `std::error::Error`.
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

/// A wrapper around a `Future`.
struct Racer<T> {
    name: String,
    fut: Pin<Box<dyn Future<Output = Result<T, Error>>>>,
}

/// The rank and disqualification status of an executed Racer.
#[derive(Debug)]
pub struct RaceResult<T> {
    pub name: String,
    pub duration: Duration,
    pub disqualified: bool,
    pub error: Option<Error>,
    pub value: Option<T>,
}

/// Race a set of `Future`s and rank them.
pub struct RaceTrack<T> {
    timeout: Duration,
    racers: Vec<Racer<T>>,
    rankings: BTreeMap<usize, RaceResult<T>>,
}

impl<T> Default for RaceTrack<T> {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            rankings: BTreeMap::new(),
            racers: Vec::new(),
        }
    }
}

impl<T> RaceTrack<T>
where
    T: Clone + Send + Sync,
{
    /// Create a new `RaceTrack` with specified timeout.
    pub fn disqualify_after(timeout: Duration) -> Self {
        Self {
            timeout,
            ..Default::default()
        }
    }

    /// Add a `Future` to the `RaceTrack`.
    pub fn add_racer<F>(&mut self, name: impl Into<String>, fut: F)
    where
        F: Future<Output = Result<T, Error>> + 'static,
    {
        self.racers.push(Racer {
            name: name.into(),
            fut: Box::pin(fut),
        });
    }

    /// Run the `RaceTrack` and collect the rankings.
    pub async fn run(&mut self) {
        // Clear the rankings from the previous run.
        self.rankings.clear();

        // Run the racers.
        let mut tasks = Vec::new();
        for racer in self.racers.iter_mut() {
            let name = racer.name.clone();
            let timeout = self.timeout;
            tasks.push(async move {
                let start = std::time::Instant::now();
                let fut = racer.fut.as_mut();
                let res = tokio::time::timeout(timeout, fut).await;
                let duration = start.elapsed();
                let disqualified = res.is_err();

                // Do some magic on the timeout error and then split the result!
                let result = res.unwrap_or_else(|_| Err("Racer timed out".into()));
                let (value, error) = match result {
                    Ok(value) => (Some(value), None),
                    Err(error) => (None, Some(error)),
                };

                RaceResult {
                    name,
                    duration,
                    disqualified,
                    error,
                    value,
                }
            });
        }

        // RaceResult em up!
        let mut i = 0;
        for result in futures::future::join_all(tasks).await {
            self.rankings.insert(i, result);
            i += 1;
        }
    }

    /// Get the rankings for the previous `RaceTrack` run.
    pub fn rankings(&self) -> Vec<&RaceResult<T>> {
        self.rankings.values().into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::sleep;

    use super::*;

    #[tokio::test]
    async fn on_your_mark_get_set_go() {
        let mut race_track = RaceTrack::disqualify_after(Duration::from_millis(20));

        race_track.add_racer("Racer #1", async move {
            sleep(Duration::from_millis(5)).await;
            Ok(1)
        });
        race_track.add_racer("Racer #2", async move {
            sleep(Duration::from_millis(10)).await;
            Ok(2)
        });
        race_track.add_racer("Racer #3", async move {
            sleep(Duration::from_millis(15)).await;
            Ok(3)
        });
        race_track.add_racer("Racer #4", async move {
            sleep(Duration::from_millis(25)).await;
            Ok(4)
        });

        race_track.run().await;
        let rankings = race_track.rankings();

        assert_eq!(rankings[0].name, "Racer #1");
        assert_eq!(rankings[0].disqualified, false);
        assert_eq!(rankings[0].value, Some(1));

        assert_eq!(rankings[1].name, "Racer #2");
        assert_eq!(rankings[1].disqualified, false);
        assert_eq!(rankings[1].value, Some(2));

        assert_eq!(rankings[2].name, "Racer #3");
        assert_eq!(rankings[2].disqualified, false);
        assert_eq!(rankings[2].value, Some(3));

        assert_eq!(rankings[3].name, "Racer #4");
        assert_eq!(rankings[3].disqualified, true);
        assert_eq!(
            rankings[3].error.as_ref().unwrap().to_string(),
            "Racer timed out"
        );
        assert_eq!(rankings[3].value, None);
    }
}
