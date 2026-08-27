//! Stable `SeqQC` facade for sequencing-format implementations.
//!
//! Third-party parser types do not cross this crate boundary. The facade is
//! intentionally small while the underlying APIs and scientific behavior are
//! evaluated during Milestone 0.

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use flate2::read::MultiGzDecoder;
use noodles::{bam, cram, fasta, fastq};

/// The parser implementation pinned for the current evidence set.
pub const NOODLES_VERSION: &str = "0.115.0";

/// Supported sequencing container or text format.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequencingFormat {
    Fastq,
    Bam,
    Cram,
}

/// Compression observed at the input boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Compression {
    None,
    Gzip,
    Bgzf,
    CramInternal,
}

/// Small, format-independent evidence emitted by the parser facade.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatSummary {
    pub format: SequencingFormat,
    pub compression: Compression,
    pub input_bytes: u64,
    pub record_count: u64,
    pub total_bases: u64,
    pub minimum_read_length: Option<usize>,
    pub maximum_read_length: Option<usize>,
    pub mapped_records: u64,
    pub unmapped_records: u64,
    pub paired_records: u64,
    pub reference_sequence_count: usize,
}

impl FormatSummary {
    fn new(format: SequencingFormat, compression: Compression, input_bytes: u64) -> Self {
        Self {
            format,
            compression,
            input_bytes,
            record_count: 0,
            total_bases: 0,
            minimum_read_length: None,
            maximum_read_length: None,
            mapped_records: 0,
            unmapped_records: 0,
            paired_records: 0,
            reference_sequence_count: 0,
        }
    }

    fn observe_read_length(&mut self, read_length: usize) -> io::Result<()> {
        self.record_count = self.record_count.checked_add(1).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "record count overflowed u64")
        })?;
        self.total_bases = self
            .total_bases
            .checked_add(u64::try_from(read_length).map_err(invalid_data)?)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "base count overflowed u64")
            })?;
        self.minimum_read_length = Some(
            self.minimum_read_length
                .map_or(read_length, |current| current.min(read_length)),
        );
        self.maximum_read_length = Some(
            self.maximum_read_length
                .map_or(read_length, |current| current.max(read_length)),
        );
        Ok(())
    }
}

/// Inspects a supported local input as a stream.
///
/// `reference_fasta` is used only for CRAM records that require reference bases.
/// The associated `.fai` must already exist; this function never downloads or
/// generates references.
///
/// # Errors
/// Returns an I/O error for unsupported extensions, unreadable inputs, malformed
/// records, missing CRAM reference assets, or count overflow.
pub fn inspect_path(
    path: impl AsRef<Path>,
    reference_fasta: Option<&Path>,
) -> io::Result<FormatSummary> {
    let path = path.as_ref();
    let input_bytes = path.metadata()?.len();
    let extension = path.extension().and_then(|value| value.to_str());
    let is_gzip = extension.is_some_and(|value| value.eq_ignore_ascii_case("gz"));
    let logical_extension = if is_gzip {
        path.file_stem()
            .and_then(|value| Path::new(value).extension())
            .and_then(|value| value.to_str())
    } else {
        extension
    };
    let is_fastq = logical_extension.is_some_and(|value| {
        value.eq_ignore_ascii_case("fastq") || value.eq_ignore_ascii_case("fq")
    });

    if is_fastq && is_gzip {
        let file = File::open(path)?;
        let reader = BufReader::new(MultiGzDecoder::new(file));
        inspect_fastq_reader(reader, Compression::Gzip, input_bytes)
    } else if is_fastq {
        let reader = BufReader::new(File::open(path)?);
        inspect_fastq_reader(reader, Compression::None, input_bytes)
    } else if logical_extension.is_some_and(|value| value.eq_ignore_ascii_case("bam")) {
        inspect_bam(path, input_bytes)
    } else if logical_extension.is_some_and(|value| value.eq_ignore_ascii_case("cram")) {
        inspect_cram(path, reference_fasta, input_bytes)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "supported inputs are FASTQ, FASTQ.GZ, BAM, and CRAM",
        ))
    }
}

fn inspect_fastq_reader<R>(
    reader: R,
    compression: Compression,
    input_bytes: u64,
) -> io::Result<FormatSummary>
where
    R: BufRead,
{
    let mut reader = fastq::io::Reader::new(reader);
    let mut summary = FormatSummary::new(SequencingFormat::Fastq, compression, input_bytes);

    for result in reader.records() {
        let record = result?;
        let read_length = record.sequence().len();
        if read_length != record.quality_scores().len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "FASTQ sequence and quality lengths differ",
            ));
        }
        summary.observe_read_length(read_length)?;
    }

    Ok(summary)
}

fn inspect_bam(path: &Path, input_bytes: u64) -> io::Result<FormatSummary> {
    let mut reader = bam::io::Reader::new(File::open(path)?);
    let header = reader.read_header()?;
    let mut summary = FormatSummary::new(SequencingFormat::Bam, Compression::Bgzf, input_bytes);
    summary.reference_sequence_count = header.reference_sequences().len();

    for result in reader.records() {
        let record = result?;
        summary.observe_read_length(record.sequence().len())?;
        observe_flags(&mut summary, record.flags())?;
    }

    Ok(summary)
}

fn inspect_cram(
    path: &Path,
    reference_fasta: Option<&Path>,
    input_bytes: u64,
) -> io::Result<FormatSummary> {
    let repository = reference_fasta
        .map(|reference| {
            fasta::io::indexed_reader::Builder::default()
                .build_from_path(reference)
                .map(fasta::repository::adapters::IndexedReader::new)
                .map(fasta::Repository::new)
        })
        .transpose()?
        .unwrap_or_default();
    let mut reader = cram::io::reader::Builder::default()
        .set_reference_sequence_repository(repository)
        .build_from_path(path)?;
    let header = reader.read_header()?;
    let mut summary = FormatSummary::new(
        SequencingFormat::Cram,
        Compression::CramInternal,
        input_bytes,
    );
    summary.reference_sequence_count = header.reference_sequences().len();

    for result in reader.records(&header) {
        let record = result?;
        summary.observe_read_length(record.sequence().len())?;
        observe_flags(&mut summary, record.flags())?;
    }

    Ok(summary)
}

fn observe_flags(
    summary: &mut FormatSummary,
    flags: noodles::sam::alignment::record::Flags,
) -> io::Result<()> {
    if flags.is_unmapped() {
        summary.unmapped_records = summary.unmapped_records.checked_add(1).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "unmapped count overflowed u64")
        })?;
    } else {
        summary.mapped_records = summary.mapped_records.checked_add(1).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "mapped count overflowed u64")
        })?;
    }
    if flags.is_segmented() {
        summary.paired_records = summary.paired_records.checked_add(1).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "paired count overflowed u64")
        })?;
    }
    Ok(())
}

fn invalid_data(error: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}
