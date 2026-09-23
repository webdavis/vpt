use super::*;
use crate::fixtures::{box_of, inspect_bytes, m4a, mvhd, mvhd_v1, reader};

const CAPTURED: i64 = 1_787_604_456;

fn with_movie(movie: &[u8]) -> Vec<u8> {
    let mut bytes = box_of(b"ftyp", b"M4A ");
    bytes.extend(box_of(b"moov", movie));
    bytes
}

#[test]
fn a_whole_file_yields_capture_time_and_duration() {
    let container = inspect_bytes(&m4a(CAPTURED, 612, b"audio")).expect("whole");
    assert_eq!(container.creation_time, UtcInstant { secs: CAPTURED });
    assert_eq!(container.duration_secs, 612);
}

#[test]
fn a_fractional_duration_floors_to_whole_seconds() {
    let mut movie = mvhd(CAPTURED, 7);
    movie[24..28].copy_from_slice(&7_999u32.to_be_bytes());
    assert_eq!(
        inspect_bytes(&with_movie(&movie))
            .expect("fractional")
            .duration_secs,
        7
    );
}

#[test]
fn a_truncated_download_is_refused_past_end() {
    let mut bytes = m4a(CAPTURED, 612, b"audio");
    bytes.truncate(bytes.len() - 10);
    assert!(matches!(
        inspect_bytes(&bytes),
        Err(ContainerError::PastEnd { .. })
    ));
}

#[test]
fn a_file_without_moov_is_refused() {
    let mut bytes = box_of(b"ftyp", b"M4A ");
    bytes.extend(box_of(b"mdat", b"audio"));
    assert_eq!(inspect_bytes(&bytes), Err(ContainerError::MissingMoov));
}

#[test]
fn a_moov_without_mvhd_is_refused() {
    let bytes = with_movie(&box_of(b"udta", b""));
    assert_eq!(inspect_bytes(&bytes), Err(ContainerError::MissingMvhd));
}

#[test]
fn a_box_length_below_its_header_is_invalid() {
    let mut bytes = vec![0, 0, 0, 4];
    bytes.extend(b"ftyp");
    assert_eq!(
        inspect_bytes(&bytes),
        Err(ContainerError::InvalidLength { offset: 0 })
    );
}

#[test]
fn a_short_base_header_is_past_end_before_read() {
    let mut reads = 0;
    let outcome = inspect(7, |_offset, _buf| {
        reads += 1;
        Err(ReadFailure)
    });
    assert_eq!(outcome, Err(ContainerError::PastEnd { offset: 0 }));
    assert_eq!(reads, 0);
}

#[test]
fn an_extended_length_that_overflows_is_refused() {
    let mut bytes = box_of(b"free", b"");
    bytes.extend([0, 0, 0, 1]);
    bytes.extend(b"mdat");
    bytes.extend(u64::MAX.to_be_bytes());
    assert_eq!(inspect_bytes(&bytes), Err(ContainerError::Overflow));
}

#[test]
fn a_truncated_extended_header_is_past_end() {
    let mut bytes = vec![0, 0, 0, 1];
    bytes.extend(b"mdat");
    bytes.extend([0u8; 4]);
    assert_eq!(
        inspect_bytes(&bytes),
        Err(ContainerError::PastEnd { offset: 0 })
    );
}

#[test]
fn an_extended_box_can_hold_moov() {
    let movie = mvhd(CAPTURED, 2);
    let mut bytes = vec![0, 0, 0, 1];
    bytes.extend(b"moov");
    bytes.extend((16 + movie.len() as u64).to_be_bytes());
    bytes.extend(movie);
    assert_eq!(inspect_bytes(&bytes).expect("extended").duration_secs, 2);
}

#[test]
fn a_zero_size_top_level_box_ends_at_file_end() {
    let mut bytes = m4a(CAPTURED, 2, b"audio");
    let moov = box_of(b"moov", &mvhd(CAPTURED, 2));
    let start = bytes.len() - moov.len();
    bytes[start..start + 4].fill(0);
    assert_eq!(inspect_bytes(&bytes).expect("zero size").duration_secs, 2);
}

#[test]
fn a_zero_size_child_extending_past_its_parent_is_refused() {
    let mut zero_child = box_of(b"free", b"");
    zero_child[..4].fill(0);
    let mut bytes = with_movie(&zero_child);
    bytes.extend(box_of(b"free", b"trailing"));
    assert_eq!(
        inspect_bytes(&bytes),
        Err(ContainerError::PastEnd { offset: 20 })
    );
}

#[test]
fn a_version_one_mvhd_uses_64_bit_fields() {
    let bytes = with_movie(&mvhd_v1(CAPTURED, 7_200));
    let container = inspect_bytes(&bytes).expect("v1");
    assert_eq!(container.creation_time.secs, CAPTURED);
    assert_eq!(container.duration_secs, 7_200);
}

#[test]
fn a_version_one_mvhd_keeps_values_above_32_bits() {
    let creation_unix = 2_212_122_496;
    let duration_secs = 4_294_968;
    let movie = mvhd_v1(creation_unix, duration_secs);
    assert_eq!(&movie[12..20], &4_294_967_296u64.to_be_bytes());
    assert_eq!(&movie[32..40], &4_294_968_000u64.to_be_bytes());
    let container = inspect_bytes(&with_movie(&movie)).expect("v1 wide values");
    assert_eq!(container.creation_time.secs, creation_unix);
    assert_eq!(container.duration_secs, duration_secs);
}

#[test]
fn unknown_mvhd_version_is_invalid() {
    let mut movie = mvhd(CAPTURED, 1);
    movie[8] = 2;
    assert_eq!(
        inspect_bytes(&with_movie(&movie)),
        Err(ContainerError::InvalidMvhd)
    );
}

#[test]
fn short_mvhd_fields_are_invalid() {
    for body in [Vec::new(), vec![0], vec![1, 0, 0, 0]] {
        assert_eq!(
            inspect_bytes(&with_movie(&box_of(b"mvhd", &body))),
            Err(ContainerError::InvalidMvhd)
        );
    }
}

#[test]
fn zero_timescale_is_invalid() {
    let mut movie = mvhd(CAPTURED, 1);
    movie[8 + 12..8 + 16].fill(0);
    assert_eq!(
        inspect_bytes(&with_movie(&movie)),
        Err(ContainerError::InvalidMvhd)
    );
}

#[test]
fn version_one_zero_timescale_is_invalid() {
    let mut movie = mvhd_v1(CAPTURED, 1);
    movie[8 + 20..8 + 24].fill(0);
    assert_eq!(
        inspect_bytes(&with_movie(&movie)),
        Err(ContainerError::InvalidMvhd)
    );
}

#[test]
fn unknown_version_zero_duration_is_invalid() {
    let mut v0 = mvhd(CAPTURED, 1);
    v0[8 + 16..8 + 20].fill(0xff);
    assert_eq!(
        inspect_bytes(&with_movie(&v0)),
        Err(ContainerError::InvalidMvhd)
    );
}

#[test]
fn unknown_version_one_duration_is_invalid() {
    let mut v1 = mvhd_v1(CAPTURED, 1);
    v1[8 + 24..8 + 32].fill(0xff);
    assert_eq!(
        inspect_bytes(&with_movie(&v1)),
        Err(ContainerError::InvalidMvhd)
    );
}

#[test]
fn creation_time_beyond_i64_is_invalid() {
    let mut movie = mvhd_v1(CAPTURED, 1);
    movie[8 + 4..8 + 12].copy_from_slice(&u64::MAX.to_be_bytes());
    assert_eq!(
        inspect_bytes(&with_movie(&movie)),
        Err(ContainerError::InvalidMvhd)
    );
}

#[test]
fn a_failed_read_is_unreadable() {
    let bytes = m4a(CAPTURED, 1, b"audio");
    assert_eq!(
        inspect(bytes.len() as u64, |_offset, _buf: &mut [u8]| Err(
            ReadFailure
        )),
        Err(ContainerError::Unreadable)
    );
}

#[test]
fn a_failed_mvhd_read_is_unreadable() {
    let bytes = m4a(CAPTURED, 1, b"audio");
    let mut source = reader(&bytes);
    assert_eq!(
        inspect(bytes.len() as u64, |offset, buf| {
            if buf.len() == 1 {
                Err(ReadFailure)
            } else {
                source(offset, buf)
            }
        }),
        Err(ContainerError::Unreadable)
    );
}

#[test]
fn reads_only_bounded_headers_and_not_payload() {
    let bytes = m4a(CAPTURED, 1, &[0u8; 100_000]);
    let payload_start = u64::from(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])) + 8;
    let payload_end = payload_start + 100_000;
    let mut source = reader(&bytes);
    let mut read = 0u64;
    inspect(bytes.len() as u64, |offset, buf| {
        read += buf.len() as u64;
        assert!(buf.len() <= 28);
        assert!(
            offset + buf.len() as u64 <= payload_start || offset >= payload_end,
            "read payload at {offset}"
        );
        source(offset, buf)
    })
    .expect("whole");
    assert!(read < 1_000, "read {read} bytes");
}
