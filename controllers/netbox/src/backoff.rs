//! # Fibonacci Backoff
//!
//! Provides a Fibonacci-based backoff mechanism for retries.
//! This provides a progressive backoff that grows more slowly than exponential backoff,
//! making it suitable for operations that may need multiple retries without overwhelming the system.
//!
//! The backoff sequence is calculated in minutes to align with GitOps tool conventions.
//! Sequence: 1m, 1m, 2m, 3m, 5m, 8m, 10m (max), then converted to seconds for use in the reconciler.

use std::time::Duration;

/// Fibonacci backoff calculator
///
/// Generates backoff durations following the Fibonacci sequence.
/// Calculations are performed in minutes (aligning with GitOps tool conventions),
/// then converted to seconds for use in the reconciler.
/// Each backoff is the sum of the previous two backoffs.
#[derive(Debug, Clone)]
pub struct FibonacciBackoff {
    /// Minimum backoff value in minutes (for reset)
    min_minutes: u64,
    /// Previous backoff value in minutes
    prev_minutes: u64,
    /// Current backoff value in minutes
    current_minutes: u64,
    /// Maximum backoff value in minutes
    max_minutes: u64,
}

impl FibonacciBackoff {
    /// Create a new Fibonacci backoff with specified minimum and maximum values in minutes
    ///
    /// Default sequence for reconciliation errors: 1m, 1m, 2m, 3m, 5m, 8m, 10m (max)
    /// Calculations are performed in minutes to align with GitOps tool conventions,
    /// then converted to seconds when returned via `next_backoff_seconds()`.
    ///
    /// # Arguments
    ///
    /// * `min_minutes` - Minimum backoff duration in minutes (used for first two values, typically 1)
    /// * `max_minutes` - Maximum backoff duration in minutes (caps the sequence, typically 10)
    #[must_use]
    pub fn new(min_minutes: u64, max_minutes: u64) -> Self {
        Self {
            min_minutes,
            prev_minutes: 0,
            current_minutes: min_minutes,
            max_minutes,
        }
    }

    /// Get the next backoff duration in seconds and advance the sequence
    ///
    /// Returns the current backoff value converted from minutes to seconds,
    /// and advances to the next Fibonacci number in minutes.
    /// The sequence is capped at `max_minutes`.
    pub fn next_backoff_seconds(&mut self) -> u64 {
        // Convert current minutes to seconds
        let result_seconds = self.current_minutes * 60;

        // Calculate next Fibonacci number in minutes
        let next_minutes = self.prev_minutes + self.current_minutes;

        // Update state (in minutes)
        self.prev_minutes = self.current_minutes;
        self.current_minutes = std::cmp::min(next_minutes, self.max_minutes);

        result_seconds
    }

    /// Get the next backoff duration as a `Duration` and advance the sequence
    #[must_use]
    #[allow(dead_code)] // Utility method, may be useful in the future
    pub fn next_backoff(&mut self) -> Duration {
        Duration::from_secs(self.next_backoff_seconds())
    }

    /// Reset the backoff to the initial state
    pub fn reset(&mut self) {
        self.prev_minutes = 0;
        self.current_minutes = self.min_minutes;
    }

    /// Calculate the Fibonacci backoff duration for a given error count (stateless)
    ///
    /// This is a stateless function that calculates the nth Fibonacci number in the sequence
    /// without maintaining internal state. Useful for one-off calculations based on error count.
    ///
    /// The sequence starts at `min_minutes` for error_count 0 and 1, then follows the Fibonacci
    /// sequence: min, min, min*2, min*3, min*5, min*8, etc., capped at `max_minutes`.
    ///
    /// # Arguments
    ///
    /// * `error_count` - The number of consecutive errors (0-indexed)
    /// * `min_minutes` - Minimum backoff duration in minutes (typically 1)
    /// * `max_minutes` - Maximum backoff duration in minutes (typically 10)
    ///
    /// # Returns
    ///
    /// The backoff duration as a `Duration`, capped at `max_minutes`.
    #[must_use]
    #[allow(dead_code)] // Utility method, may be useful in the future
    pub fn calculate_for_error_count(
        error_count: u32,
        min_minutes: u64,
        max_minutes: u64,
    ) -> Duration {
        if error_count == 0 || error_count == 1 {
            // First two values are both min_minutes
            return Duration::from_secs(min_minutes * 60);
        }

        // Calculate Fibonacci sequence: F(n) = F(n-1) + F(n-2)
        // Start with prev = min, current = min
        let mut prev_minutes = min_minutes;
        let mut current_minutes = min_minutes;

        // Calculate up to error_count
        for _ in 2..=error_count {
            let next_minutes = prev_minutes + current_minutes;
            prev_minutes = current_minutes;
            current_minutes = std::cmp::min(next_minutes, max_minutes);

            // If we've hit the max, we can stop early
            if current_minutes >= max_minutes {
                break;
            }
        }

        Duration::from_secs(current_minutes * 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibonacci_backoff_sequence() {
        let mut backoff = FibonacciBackoff::new(1, 10);

        // Reconciliation error sequence in minutes: 1m, 1m, 2m, 3m, 5m, 8m, 10m (max)
        // Converted to seconds: 60s, 60s, 120s, 180s, 300s, 480s, 600s
        assert_eq!(backoff.next_backoff_seconds(), 60); // 1m = 60s
        assert_eq!(backoff.next_backoff_seconds(), 60); // 1m = 60s
        assert_eq!(backoff.next_backoff_seconds(), 120); // 2m = 120s
        assert_eq!(backoff.next_backoff_seconds(), 180); // 3m = 180s
        assert_eq!(backoff.next_backoff_seconds(), 300); // 5m = 300s
        assert_eq!(backoff.next_backoff_seconds(), 480); // 8m = 480s
        assert_eq!(backoff.next_backoff_seconds(), 600); // 10m = 600s (max)
    }

    #[test]
    fn test_fibonacci_backoff_max_cap() {
        let mut backoff = FibonacciBackoff::new(1, 10);

        // Should cap at 600 seconds (10 minutes)
        assert_eq!(backoff.next_backoff_seconds(), 60); // 1m = 60s
        assert_eq!(backoff.next_backoff_seconds(), 60); // 1m = 60s
        assert_eq!(backoff.next_backoff_seconds(), 120); // 2m = 120s
        assert_eq!(backoff.next_backoff_seconds(), 180); // 3m = 180s
        assert_eq!(backoff.next_backoff_seconds(), 300); // 5m = 300s
        assert_eq!(backoff.next_backoff_seconds(), 480); // 8m = 480s
        assert_eq!(backoff.next_backoff_seconds(), 600); // 10m = 600s (max)
        // Next would be 13m (8+5), but should be capped at 10m = 600s
        assert_eq!(backoff.next_backoff_seconds(), 600);
        // Should stay at max
        assert_eq!(backoff.next_backoff_seconds(), 600);
    }

    #[test]
    fn test_fibonacci_backoff_reset() {
        let mut backoff = FibonacciBackoff::new(1, 10);

        assert_eq!(backoff.next_backoff_seconds(), 60); // 1m = 60s
        assert_eq!(backoff.next_backoff_seconds(), 60); // 1m = 60s
        assert_eq!(backoff.next_backoff_seconds(), 120); // 2m = 120s
        assert_eq!(backoff.next_backoff_seconds(), 180); // 3m = 180s

        backoff.reset();

        // Should restart from beginning after success
        assert_eq!(backoff.next_backoff_seconds(), 60); // 1m = 60s
        assert_eq!(backoff.next_backoff_seconds(), 60); // 1m = 60s
        assert_eq!(backoff.next_backoff_seconds(), 120); // 2m = 120s
    }
}

