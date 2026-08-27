use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::read::MultiGzDecoder;
use seqqc_formats::{Compression, SequencingFormat, inspect_path};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test-data/synthetic/tiny-formats")
        .join(name)
}

#[test]
fn facade_matches_declared_fixture_evidence() -> io::Result<()> {
    let fastq = inspect_path(fixture("tiny.fastq"), None)?;
    assert_eq!(fastq.format, SequencingFormat::Fastq);
    assert_eq!(fastq.compression, Compression::None);
    assert_common_read_evidence(&fastq);
    assert_eq!(fastq.mapped_records, 0);
    assert_eq!(fastq.unmapped_records, 0);

    let compressed_fastq = inspect_path(fixture("tiny.fastq.gz"), None)?;
    assert_eq!(compressed_fastq.compression, Compression::Gzip);
    assert_common_read_evidence(&compressed_fastq);

    let bam = inspect_path(fixture("tiny.bam"), None)?;
    assert_eq!(bam.format, SequencingFormat::Bam);
    assert_eq!(bam.compression, Compression::Bgzf);
    assert_alignment_evidence(&bam);

    let cram = inspect_path(fixture("tiny.cram"), Some(&fixture("reference.fa")))?;
    assert_eq!(cram.format, SequencingFormat::Cram);
    assert_eq!(cram.compression, Compression::CramInternal);
    assert_alignment_evidence(&cram);

    Ok(())
}

#[test]
fn independent_fixture_readers_agree_with_the_facade() -> io::Result<()> {
    let fastq_expected = independent_fastq_summary(&fs::read(fixture("tiny.fastq"))?)?;
    let fastq = inspect_path(fixture("tiny.fastq"), None)?;
    assert_eq!((fastq.record_count, fastq.total_bases), fastq_expected);

    let mut uncompressed_fastq = Vec::new();
    MultiGzDecoder::new(File::open(fixture("tiny.fastq.gz"))?)
        .read_to_end(&mut uncompressed_fastq)?;
    assert_eq!(
        independent_fastq_summary(&uncompressed_fastq)?,
        fastq_expected
    );

    let bam_expected = independent_bam_summary(&fixture("tiny.bam"))?;
    let bam = inspect_path(fixture("tiny.bam"), None)?;
    assert_eq!(
        (
            bam.record_count,
            bam.total_bases,
            bam.mapped_records,
            bam.unmapped_records,
        ),
        bam_expected
    );

    let cram_expected = independent_cram_container_count(&fixture("tiny.cram"))?;
    let cram = inspect_path(fixture("tiny.cram"), Some(&fixture("reference.fa")))?;
    assert_eq!(cram.record_count, cram_expected.record_count);
    assert_eq!((cram_expected.major, cram_expected.minor), (3, 0));

    Ok(())
}

#[test]
fn rejects_malformed_fastq_and_unsupported_extensions() -> io::Result<()> {
    let malformed_path = temporary_path("malformed.fastq");
    fs::write(&malformed_path, b"@bad\nACGT\n+\n!!!\n")?;
    let malformed_result = inspect_path(&malformed_path, None);
    fs::remove_file(&malformed_path)?;

    assert!(matches!(malformed_result, Err(error) if error.kind() == io::ErrorKind::InvalidData));
    assert!(matches!(
        inspect_path(fixture("README.md"), None),
        Err(error) if error.kind() == io::ErrorKind::InvalidInput
    ));
    Ok(())
}

#[test]
fn rejects_truncated_gzip_bam_and_cram_inputs() -> io::Result<()> {
    let cases = [
        ("tiny.fastq.gz", "truncated.fastq.gz", None),
        ("tiny.bam", "truncated.bam", None),
        ("tiny.cram", "truncated.cram", Some(fixture("reference.fa"))),
    ];
    for (source, label, reference) in cases {
        let path = temporary_path(label);
        let source_bytes = fs::read(fixture(source))?;
        fs::write(&path, &source_bytes[..source_bytes.len() / 2])?;
        let result = inspect_path(&path, reference.as_deref());
        fs::remove_file(path)?;
        assert!(result.is_err(), "truncated {source} must fail");
    }
    Ok(())
}

fn assert_common_read_evidence(summary: &seqqc_formats::FormatSummary) {
    assert_eq!(summary.record_count, 2);
    assert_eq!(summary.total_bases, 8);
    assert_eq!(summary.minimum_read_length, Some(4));
    assert_eq!(summary.maximum_read_length, Some(4));
}

fn assert_alignment_evidence(summary: &seqqc_formats::FormatSummary) {
    assert_common_read_evidence(summary);
    assert_eq!(summary.mapped_records, 1);
    assert_eq!(summary.unmapped_records, 1);
    assert_eq!(summary.paired_records, 0);
    assert_eq!(summary.reference_sequence_count, 1);
}

fn independent_fastq_summary(data: &[u8]) -> io::Result<(u64, u64)> {
    let lines = data
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let (records, remainder) = lines.as_chunks::<4>();
    if !remainder.is_empty() {
        return Err(invalid_data("FASTQ line count is not divisible by four"));
    }

    let mut record_count = 0_u64;
    let mut total_bases = 0_u64;
    for record in records {
        if !record[0].starts_with(b"@") || !record[2].starts_with(b"+") {
            return Err(invalid_data("FASTQ record markers are invalid"));
        }
        if record[1].len() != record[3].len() {
            return Err(invalid_data("FASTQ sequence and quality lengths differ"));
        }
        record_count += 1;
        total_bases += u64::try_from(record[1].len()).map_err(invalid_data_from)?;
    }
    Ok((record_count, total_bases))
}

fn independent_bam_summary(path: &Path) -> io::Result<(u64, u64, u64, u64)> {
    let mut decoded = Vec::new();
    MultiGzDecoder::new(File::open(path)?).read_to_end(&mut decoded)?;
    let mut cursor = ByteCursor::new(&decoded);

    if cursor.take(4)? != b"BAM\x01" {
        return Err(invalid_data("BAM magic is invalid"));
    }
    let header_length = cursor.read_nonnegative_i32()?;
    cursor.take(header_length)?;
    let reference_count = cursor.read_nonnegative_i32()?;
    for _ in 0..reference_count {
        let name_length = cursor.read_nonnegative_i32()?;
        cursor.take(name_length)?;
        cursor.take(4)?;
    }

    let mut records = 0_u64;
    let mut bases = 0_u64;
    let mut mapped = 0_u64;
    let mut unmapped = 0_u64;
    while cursor.remaining() > 0 {
        let block_length = cursor.read_nonnegative_i32()?;
        let block = cursor.take(block_length)?;
        if block.len() < 32 {
            return Err(invalid_data("BAM record core is truncated"));
        }
        let flags = u16::from_le_bytes([block[14], block[15]]);
        let sequence_length = i32::from_le_bytes([block[16], block[17], block[18], block[19]]);
        let sequence_length = u64::try_from(sequence_length).map_err(invalid_data_from)?;
        records += 1;
        bases += sequence_length;
        if flags & 0x04 == 0 {
            mapped += 1;
        } else {
            unmapped += 1;
        }
    }
    Ok((records, bases, mapped, unmapped))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CramContainerEvidence {
    major: u8,
    minor: u8,
    record_count: u64,
}

fn independent_cram_container_count(path: &Path) -> io::Result<CramContainerEvidence> {
    let data = fs::read(path)?;
    let mut cursor = ByteCursor::new(&data);
    if cursor.take(4)? != b"CRAM" {
        return Err(invalid_data("CRAM magic is invalid"));
    }
    let major = cursor.read_u8()?;
    let minor = cursor.read_u8()?;
    cursor.take(20)?;

    let mut record_count = 0_u64;
    loop {
        let data_length = cursor.read_nonnegative_i32()?;
        let reference_sequence_id = cursor.read_itf8()?;
        let alignment_start = cursor.read_itf8()?;
        cursor.read_itf8()?;
        let container_records = cursor.read_itf8()?;
        cursor.read_ltf8()?;
        cursor.read_ltf8()?;
        let block_count = cursor.read_itf8()?;
        let landmark_count = cursor.read_itf8()?;
        for _ in 0..usize::try_from(landmark_count).map_err(invalid_data_from)? {
            cursor.read_itf8()?;
        }
        cursor.take(4)?;

        if data_length == 15
            && reference_sequence_id == -1
            && alignment_start == 4_542_278
            && block_count == 1
        {
            break;
        }
        record_count = record_count
            .checked_add(u64::try_from(container_records).map_err(invalid_data_from)?)
            .ok_or_else(|| invalid_data("CRAM record count overflow"))?;
        cursor.take(data_length)?;
    }

    Ok(CramContainerEvidence {
        major,
        minor,
        record_count,
    })
}

struct ByteCursor<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> ByteCursor<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    const fn remaining(&self) -> usize {
        self.data.len() - self.offset
    }

    fn take(&mut self, length: usize) -> io::Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| invalid_data("binary cursor overflow"))?;
        let value = self
            .data
            .get(self.offset..end)
            .ok_or_else(|| invalid_data("binary input is truncated"))?;
        self.offset = end;
        Ok(value)
    }

    fn read_u8(&mut self) -> io::Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn read_nonnegative_i32(&mut self) -> io::Result<usize> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| invalid_data("i32 input is truncated"))?;
        usize::try_from(i32::from_le_bytes(bytes)).map_err(invalid_data_from)
    }

    fn read_itf8(&mut self) -> io::Result<i32> {
        let first = u32::from(self.read_u8()?);
        let value = if first & 0x80 == 0 {
            first
        } else if first & 0x40 == 0 {
            (first & 0x7f) << 8 | u32::from(self.read_u8()?)
        } else if first & 0x20 == 0 {
            (first & 0x3f) << 16 | u32::try_from(self.read_be(2)?).map_err(invalid_data_from)?
        } else if first & 0x10 == 0 {
            (first & 0x1f) << 24 | u32::try_from(self.read_be(3)?).map_err(invalid_data_from)?
        } else {
            let tail = u32::try_from(self.read_be(4)?).map_err(invalid_data_from)?;
            (first & 0x0f) << 28 | (tail & 0xffff_fff0) >> 4 | tail & 0x0f
        };
        Ok(value.cast_signed())
    }

    fn read_ltf8(&mut self) -> io::Result<i64> {
        let first = u64::from(self.read_u8()?);
        let (prefix_mask, tail_length) = if first & 0x80 == 0 {
            return Ok(first.cast_signed());
        } else if first & 0x40 == 0 {
            (0x7f, 1)
        } else if first & 0x20 == 0 {
            (0x3f, 2)
        } else if first & 0x10 == 0 {
            (0x1f, 3)
        } else if first & 0x08 == 0 {
            (0x0f, 4)
        } else if first & 0x04 == 0 {
            (0x07, 5)
        } else if first & 0x02 == 0 {
            (0x03, 6)
        } else if first & 0x01 == 0 {
            (0x00, 7)
        } else {
            (0x00, 8)
        };
        let value = (first & prefix_mask) << (tail_length * 8) | self.read_be(tail_length)?;
        Ok(value.cast_signed())
    }

    fn read_be(&mut self, length: usize) -> io::Result<u64> {
        self.take(length)?.iter().try_fold(0_u64, |value, byte| {
            value
                .checked_shl(8)
                .and_then(|shifted| shifted.checked_add(u64::from(*byte)))
                .ok_or_else(|| invalid_data("big-endian integer overflow"))
        })
    }
}

fn temporary_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock must be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "seqqc-formats-{}-{nonce}-{label}",
        std::process::id()
    ))
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn invalid_data_from(error: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}
