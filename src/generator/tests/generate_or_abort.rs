use super::Scru128Generator;

/// Generates increasing IDs even with decreasing or constant timestamp
#[test]
fn generates_increasing_ids_even_with_decreasing_or_constant_timestamp() {
    let ts = 0x0123_4567_89abu64;
    let mut g = Scru128Generator::new();

    let mut prev = g.generate_or_abort_with_ts(ts).unwrap();
    assert_eq!(prev.timestamp(), ts);

    for i in 0..100_000u64 {
        let curr = g.generate_or_abort_with_ts(ts - i.min(9_999)).unwrap();
        assert!(prev < curr);
        prev = curr;
    }
    assert!(prev.timestamp() >= ts);
}

/// Returns None if timestamp goes backwards a lot
#[test]
fn returns_none_if_timestamp_goes_backwards_a_lot() {
    let ts = 0x0123_4567_89abu64;
    let mut g = Scru128Generator::new();

    let prev = g.generate_or_abort_with_ts(ts).unwrap();
    assert_eq!(prev.timestamp(), ts);

    let mut curr = g.generate_or_abort_with_ts(ts - 10_000);
    assert!(prev < curr.unwrap());

    curr = g.generate_or_abort_with_ts(ts - 10_001);
    assert!(curr.is_none());

    curr = g.generate_or_abort_with_ts(ts - 10_002);
    assert!(curr.is_none());
}
