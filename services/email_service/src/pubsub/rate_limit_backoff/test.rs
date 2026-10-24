use super::*;

#[test]
fn gmail_rate_limits_double_per_retry_up_to_five_minutes() {
    let expected = [
        (0, 15, 30),
        (1, 15, 30),
        (2, 30, 60),
        (3, 60, 120),
        (4, 120, 240),
        (5, 150, 300),
        (33, 150, 300),
        (u32::MAX, 150, 300),
    ];
    for (retry, min_secs, max_secs) in expected {
        assert_eq!(
            retry_delay(retry, RateLimitOrigin::Provider, None, |range| *range
                .start()),
            Duration::from_secs(min_secs),
            "retry {retry}"
        );
        assert_eq!(
            retry_delay(retry, RateLimitOrigin::Provider, None, |range| *range.end()),
            Duration::from_secs(max_secs),
            "retry {retry}"
        );
    }
}

#[test]
fn gate_refusals_wait_about_the_gates_retry_after() {
    let busy = Some(Duration::from_secs(1));
    let paused = Some(Duration::from_secs(5));
    let expected = [
        (1, busy, 2, 2),
        (1, paused, 6, 6),
        (4, busy, 5, 9),
        (10, busy, 151, 300),
    ];
    for (retry, retry_after, min_secs, max_secs) in expected {
        assert_eq!(
            retry_delay(retry, RateLimitOrigin::Local, retry_after, |range| *range
                .start()),
            Duration::from_secs(min_secs),
            "retry {retry}, retry_after {retry_after:?}"
        );
        assert_eq!(
            retry_delay(retry, RateLimitOrigin::Local, retry_after, |range| *range
                .end()),
            Duration::from_secs(max_secs),
            "retry {retry}, retry_after {retry_after:?}"
        );
    }
}

#[test]
fn retry_after_is_rounded_up_to_whole_seconds_and_capped_at_five_minutes() {
    assert_eq!(
        retry_delay(
            1,
            RateLimitOrigin::Provider,
            Some(Duration::from_millis(30_800)),
            |range| *range.start()
        ),
        Duration::from_secs(31)
    );
    assert_eq!(
        retry_delay(
            1,
            RateLimitOrigin::Local,
            Some(Duration::from_millis(200)),
            |range| *range.start()
        ),
        Duration::from_secs(1)
    );
    assert_eq!(
        retry_delay(
            4,
            RateLimitOrigin::Provider,
            Some(Duration::from_secs(10)),
            |range| *range.start()
        ),
        Duration::from_secs(120)
    );
    assert_eq!(
        retry_delay(
            1,
            RateLimitOrigin::Provider,
            Some(Duration::from_secs(3_600)),
            |range| *range.start()
        ),
        Duration::from_secs(300)
    );
}

#[test]
fn every_delivery_waiting_its_longest_still_ends_within_twelve_hours() {
    let total: Duration = (1..=101)
        .map(|retry| {
            retry_delay(
                retry,
                RateLimitOrigin::Provider,
                Some(Duration::from_secs(3_600)),
                |range| *range.end(),
            )
        })
        .sum();
    assert_eq!(total, Duration::from_secs(30_300));
    assert!(total < Duration::from_secs(12 * 3_600));
}

#[test]
fn retry_queue_receives_continue_the_primary_queue_attempt() {
    let cases = [(false, 5, 15, 30), (true, 1, 30, 60), (true, 3, 120, 240)];
    for (retry_worker, receive_count, min_secs, max_secs) in cases {
        let delay = next_retry_delay(retry_worker, receive_count, RateLimitOrigin::Provider, None);
        assert!(
            (Duration::from_secs(min_secs)..=Duration::from_secs(max_secs)).contains(&delay),
            "retry_worker {retry_worker}, receive {receive_count}: {delay:?}"
        );
    }
}
