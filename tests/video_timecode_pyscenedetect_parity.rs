use num_rational::Rational64;
use video_analysis_core::{FrameTimecode, Timebase};

fn fps(num: i64, den: i64) -> Rational64 {
    Rational64::new(num, den)
}

#[test]
fn parses_frame_second_and_hh_mm_ss_timecodes() {
    assert_eq!(
        FrameTimecode::parse("42", fps(30, 1)).unwrap().frame_index,
        42
    );
    assert_eq!(
        FrameTimecode::parse("2.0", fps(30, 1)).unwrap().frame_index,
        60
    );
    assert_eq!(
        FrameTimecode::parse("00:01:00.000", fps(30, 1))
            .unwrap()
            .frame_index,
        1800
    );
    assert_eq!(
        FrameTimecode::parse("00:00:00.500", fps(10, 1))
            .unwrap()
            .frame_index,
        5
    );
}

#[test]
fn rejects_negative_and_invalid_timecodes() {
    for input in ["-1", "-0.1", "1.9x", "1.0-", "01:01:00:00.001", "00:xx:01"] {
        assert!(
            FrameTimecode::parse(input, fps(30, 1)).is_err(),
            "expected `{input}` to be rejected"
        );
    }
}

#[test]
fn rounds_like_pyscenedetect_for_common_frame_rates() {
    assert_eq!(
        FrameTimecode::from_seconds(1.9999, fps(1, 1))
            .unwrap()
            .frame_index,
        2
    );
    assert_eq!(
        FrameTimecode::from_seconds(0.5, fps(10, 1))
            .unwrap()
            .frame_index,
        5
    );
    assert_eq!(
        FrameTimecode::from_seconds(1.0, fps(30_000, 1001))
            .unwrap()
            .frame_index,
        30
    );
    assert_eq!(
        FrameTimecode::from_seconds(1.5, fps(30, 1))
            .unwrap()
            .frame_index,
        45
    );
    assert_eq!(
        FrameTimecode::from_seconds(0.001, fps(1000, 1))
            .unwrap()
            .frame_index,
        1
    );
}

#[test]
fn frame_arithmetic_adds_and_saturates() {
    let timecode = FrameTimecode::from_frames(10, fps(10, 1));

    assert_eq!((timecode + 5).frame_index, 15);
    assert_eq!((timecode - 3).frame_index, 7);
    assert_eq!((timecode - 30).frame_index, 0);
    assert_eq!((timecode + 10).to_string(), "00:00:02.000");
}

#[test]
fn legacy_frame_timecode_does_not_carry_pts_metadata() {
    let timecode = FrameTimecode::from_frames(10, fps(10, 1));

    assert_eq!(timecode.pts(), None);
    assert_eq!(timecode.time_base(), None);
    assert_eq!(timecode.seconds(), 1.0);
    assert_eq!(timecode.position().timestamp.pts, 10);
    assert_eq!(timecode.position().timestamp.timebase, Timebase::new(1, 10));
}

#[test]
fn timestamped_frame_timecode_preserves_source_pts_and_timebase() {
    let timecode = FrameTimecode::from_pts(10, fps(30_000, 1001), 42_000, Timebase::new(1, 90_000));

    assert_eq!(timecode.frame_index(), 10);
    assert_eq!(timecode.fps(), fps(30_000, 1001));
    assert_eq!(timecode.pts(), Some(42_000));
    assert_eq!(timecode.time_base(), Some(Timebase::new(1, 90_000)));
    assert!((timecode.seconds() - 0.466_666_666_666_666_7).abs() < 1e-12);
    assert_eq!(timecode.timecode(3), "00:00:00.334");

    let position = timecode.position();
    assert_eq!(position.frame_index, 10);
    assert_eq!(position.timestamp.pts, 42_000);
    assert_eq!(position.timestamp.timebase, Timebase::new(1, 90_000));
}
