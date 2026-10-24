use super::*;
use aws_sdk_sqs::types::Message;

#[test]
fn visibility_timeouts_round_up_to_whole_seconds_within_the_sqs_limit() {
    assert_eq!(visibility_timeout_seconds(Duration::ZERO), 0);
    assert_eq!(visibility_timeout_seconds(Duration::from_millis(500)), 1);
    assert_eq!(
        visibility_timeout_seconds(Duration::from_millis(30_800)),
        31
    );
    assert_eq!(visibility_timeout_seconds(Duration::from_secs(300)), 300);
    assert_eq!(
        visibility_timeout_seconds(Duration::from_secs(86_400)),
        43_200
    );
    assert_eq!(visibility_timeout_seconds(Duration::MAX), 43_200);
}

#[test]
fn receive_count_reads_a_positive_approximate_receive_count() {
    let with_count = |count: &str| {
        Message::builder()
            .attributes(MessageSystemAttributeName::ApproximateReceiveCount, count)
            .build()
    };
    assert_eq!(receive_count(&with_count("3")), Some(3));
    assert_eq!(receive_count(&with_count("0")), None);
    assert_eq!(receive_count(&with_count("three")), None);
    assert_eq!(receive_count(&Message::builder().build()), None);
}
