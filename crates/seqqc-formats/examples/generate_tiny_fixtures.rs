use std::fs::{self, File};
use std::io::{self, Write as _};
use std::num::NonZero;
use std::path::{Path, PathBuf};

use flate2::{Compression, write::GzEncoder};
use noodles::core::Position;
use noodles::{bam, cram, fasta, sam};
use sam::alignment::{
    RecordBuf,
    io::Write as _,
    record::{Flags, MappingQuality, cigar::Op, cigar::op::Kind},
    record_buf::{QualityScores, Sequence},
};
use sam::header::record::value::{
    Map,
    map::{Header as HeaderValue, ReferenceSequence},
};

const FASTQ_BYTES: &[u8] =
    b"@mapped-1 synthetic\nACGT\n+\n????\n@unmapped-1 synthetic\nTTAA\n+\n5555\n";
const REFERENCE_BYTES: &[u8] = b">sq0 synthetic reference\nACGTACGT\n";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture_directory =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-data/synthetic/tiny-formats");
    fs::create_dir_all(&fixture_directory)?;

    fs::write(fixture_directory.join("tiny.fastq"), FASTQ_BYTES)?;
    write_gzip(&fixture_directory.join("tiny.fastq.gz"))?;

    let reference_path = fixture_directory.join("reference.fa");
    fs::write(&reference_path, REFERENCE_BYTES)?;
    let index = fasta::fs::index(&reference_path)?;
    fasta::fai::fs::write(append_extension(&reference_path, "fai"), &index)?;

    let reference_records = vec![fasta::Record::new(
        fasta::record::Definition::new("sq0", Some("synthetic reference".into())),
        fasta::record::Sequence::from(b"ACGTACGT".to_vec()),
    )];
    let header = sam::Header::builder()
        .set_header(Map::<HeaderValue>::default())
        .add_reference_sequence(
            "sq0",
            Map::<ReferenceSequence>::new(NonZero::new(8).expect("reference length is nonzero")),
        )
        .add_comment("deterministic non-human SeqQC Milestone 0 fixture")
        .build();
    let records = build_records();

    write_bam(&fixture_directory.join("tiny.bam"), &header, &records)?;
    write_cram(
        &fixture_directory.join("tiny.cram"),
        &header,
        &records,
        reference_records,
    )?;

    Ok(())
}

fn write_gzip(path: &Path) -> io::Result<()> {
    let mut writer = GzEncoder::new(File::create(path)?, Compression::default());
    writer.write_all(FASTQ_BYTES)?;
    writer.finish()?;
    Ok(())
}

fn build_records() -> [RecordBuf; 2] {
    let mapped = RecordBuf::builder()
        .set_name("mapped-1")
        .set_flags(Flags::empty())
        .set_reference_sequence_id(0)
        .set_alignment_start(Position::MIN)
        .set_mapping_quality(MappingQuality::new(60).expect("mapping quality is valid"))
        .set_cigar([Op::new(Kind::Match, 4)].into_iter().collect())
        .set_sequence(Sequence::from(b"ACGT"))
        .set_quality_scores(QualityScores::from(vec![30, 30, 30, 30]))
        .build();
    let unmapped = RecordBuf::builder()
        .set_name("unmapped-1")
        .set_flags(Flags::UNMAPPED)
        .set_sequence(Sequence::from(b"TTAA"))
        .set_quality_scores(QualityScores::from(vec![20, 20, 20, 20]))
        .build();
    [mapped, unmapped]
}

fn write_bam(path: &Path, header: &sam::Header, records: &[RecordBuf]) -> io::Result<()> {
    let mut writer = bam::io::Writer::new(File::create(path)?);
    writer.write_header(header)?;
    for record in records {
        writer.write_alignment_record(header, record)?;
    }
    writer.try_finish()
}

fn write_cram(
    path: &Path,
    header: &sam::Header,
    records: &[RecordBuf],
    reference_records: Vec<fasta::Record>,
) -> io::Result<()> {
    let repository = fasta::Repository::new(reference_records);
    let mut writer = cram::io::writer::Builder::default()
        .set_reference_sequence_repository(repository)
        .build_from_writer(File::create(path)?);
    writer.write_header(header)?;
    for record in records {
        writer.write_alignment_record(header, record)?;
    }
    writer.try_finish(header)
}

fn append_extension(path: &Path, extension: &str) -> PathBuf {
    let mut value = path.as_os_str().to_owned();
    value.push(".");
    value.push(extension);
    PathBuf::from(value)
}
