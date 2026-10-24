use super::*;

#[test]
fn delays_round_up_to_whole_seconds_within_the_sqs_limit() {
    assert_eq!(delay_seconds(Duration::ZERO), 0);
    assert_eq!(delay_seconds(Duration::from_millis(500)), 1);
    assert_eq!(delay_seconds(Duration::from_millis(30_800)), 31);
    assert_eq!(delay_seconds(Duration::from_secs(900)), 900);
    assert_eq!(delay_seconds(Duration::from_secs(3_600)), 900);
    assert_eq!(delay_seconds(Duration::MAX), 900);
}
