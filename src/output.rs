use crate::{Result, history::Track, storage::atomic_write};
use std::{io::Write, path::Path};

#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub enum Format {
    #[default]
    Table,
    List,
    Csv,
    Query,
}

fn date(track: &Track, full: bool) -> String {
    track
        .timestamp
        .map(|t| {
            t.format(if full {
                "%Y-%m-%d %H:%M:%S"
            } else {
                "%Y-%m-%d"
            })
            .to_string()
        })
        .unwrap_or_else(|| "N/A".into())
}
fn csv(out: &mut dyn Write, tracks: &[Track], export: bool) -> Result<()> {
    let mut writer = csv::Writer::from_writer(out);
    let mut header = vec!["Index", "Date", "Title", "Artist", "URL"];
    if export {
        header.extend(["CoverArt", "Key"]);
    }
    writer.write_record(header)?;
    for (i, t) in tracks.iter().enumerate() {
        let mut row = vec![
            (i + 1).to_string(),
            date(t, true),
            t.title.clone(),
            t.artist.clone(),
            t.url.clone(),
        ];
        if export {
            row.extend([t.cover_art.clone(), t.key.clone()]);
        }
        writer.write_record(row)?;
    }
    writer.flush()?;
    Ok(())
}
pub fn write(
    out: &mut dyn Write,
    tracks: &[Track],
    format: Format,
    suffix: &str,
    agent: bool,
) -> Result<()> {
    if matches!(format, Format::Csv) {
        return csv(out, tracks, false);
    }
    if matches!(format, Format::Table) && !agent && !tracks.is_empty() {
        let mut table = comfy_table::Table::new();
        table.set_header(["INDEX", "DATE", "TITLE", "ARTIST", "SHAZAM URL"]);
        for (i, t) in tracks.iter().enumerate() {
            table.add_row([
                (i + 1).to_string(),
                date(t, false),
                t.title.clone(),
                t.artist.clone(),
                t.url.clone(),
            ]);
        }
        writeln!(out, "{table}")?;
        return Ok(());
    }
    for (i, t) in tracks.iter().enumerate() {
        match format {
            Format::Query => writeln!(
                out,
                "{} - {}{}{}",
                t.artist,
                t.title,
                if suffix.is_empty() { "" } else { " " },
                suffix
            )?,
            Format::List => writeln!(
                out,
                "[{}] {} - {} ({})",
                i + 1,
                t.title,
                t.artist,
                date(t, false)
            )?,
            Format::Table if agent => writeln!(
                out,
                "index: {}\ndate: {}\ntitle: {}\nartist: {}\nurl: {}",
                i + 1,
                date(t, false),
                t.title,
                t.artist,
                t.url
            )?,
            Format::Table => writeln!(
                out,
                "{}  {}  {} — {}  {}",
                i + 1,
                date(t, false),
                t.title,
                t.artist,
                t.url
            )?,
            Format::Csv => unreachable!(),
        }
    }
    Ok(())
}
pub fn export(path: &Path, tracks: &[Track]) -> Result<()> {
    atomic_write(path, |out| {
        if path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("json"))
        {
            serde_json::to_writer_pretty(&mut *out, tracks)?;
            writeln!(out)?;
            Ok(())
        } else {
            csv(out, tracks, true)
        }
    })
}
