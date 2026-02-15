use super::Scru128Generator;

/// Generates increasing IDs even with decreasing or constant timestamp
#[test]
fn generates_increasing_ids_even_with_decreasing_or_constant_timestamp() {
    let ts = 0x0123_4567_89abu64;
    let mut g = Scru128Generator::new();

    let mut prev = g.generate_or_reset_with_ts(ts);
    assert_eq!(prev.timestamp(), ts);

    for i in 0..100_000u64 {
        let curr = g.generate_or_reset_with_ts(ts - i.min(9_999));
        assert!(prev < curr);
        prev = curr;
    }
    assert!(prev.timestamp() >= ts);
}

/// Breaks increasing order of IDs if timestamp goes backwards a lot
#[test]
fn breaks_increasing_order_of_ids_if_timestamp_goes_backwards_a_lot() {
    let ts = 0x0123_4567_89abu64;
    let mut g = Scru128Generator::new();

    let mut prev = g.generate_or_reset_with_ts(ts);
    assert_eq!(prev.timestamp(), ts);

    let mut curr = g.generate_or_reset_with_ts(ts - 10_000);
    assert!(prev < curr);

    prev = curr;
    curr = g.generate_or_reset_with_ts(ts - 10_001);
    assert!(prev > curr);
    assert_eq!(curr.timestamp(), ts - 10_001);

    prev = curr;
    curr = g.generate_or_reset_with_ts(ts - 10_002);
    assert!(prev < curr);
}
