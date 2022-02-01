//! Module that provides methods for handling file I/O related to running hooks
use std::{
    io::SeekFrom,
    ops::{Bound, Range, RangeBounds},
    path::PathBuf,
};

use anyhow::Context;
use chrono::Utc;
use directories_next::ProjectDirs;
use serde_json::to_string_pretty as json_pretty_string;
use substring::Substring;
use tokio::{
    fs::{create_dir_all, read_to_string, write, File},
    io::{copy, AsyncRead, AsyncReadExt, AsyncSeekExt},
};
use uuid::Uuid;

use crate::{
    config::Hook,
    error::ApiError,
    model::{Info, Request},
};

/// Reads the stdout or stderr stream of a hook instance
pub async fn read_log(
    stream: &str,
    id: &Uuid,
    dirs: &ProjectDirs,
    range: Option<(Bound<u64>, Bound<u64>)>,
) -> Result<(String, Option<Range<u64>>), ApiError> {
    let (_, mut log_path) = get_hook_files(dirs, id, false).await?;
    ensure_file_exists(log_path.clone(), "No hook with the matching ID was found")?;
    log_path.push(format!("{stream}.txt"));
    ensure_file_exists(
        log_path.clone(),
        "Hook with the matching ID exists, but log doesn't exist",
    )?;

    if let Some(range) = range {
        let file = File::open(&log_path)
            .await
            .context(format!("Couldn't open file {log_path:?} for instance {id}"))?;

        let ReadRange { start, mut buf } = read_range(file, range)
            .await
            .context(format!("Couldn't read file {log_path:?} for instance {id}"))?;

        // trimming to the last newline here ensures we end at a char boundary
        let mut trimmed_len = buf.len();
        for c in buf.iter().rev() {
            if *c == b'\n' {
                break;
            }
            trimmed_len -= 1;
        }
        buf.truncate(trimmed_len);

        Ok((
            String::from_utf8(buf).context(format!(
                "Bytes read from file {log_path:?} for instance {id} aren't valid utf-8 data"
            ))?,
            Some(Range {
                start,
                end: start + (trimmed_len as u64),
            }),
        ))
    } else {
        Ok((
            read_to_string(log_path)
                .await
                .context(format!("Couldn't read {stream} for instance {id}"))?,
            None,
        ))
    }
}

/// Range using [`SeekFrom`] as start point together with an optional length
struct SeekRange {
    /// range start
    seek: SeekFrom,
    /// len is None if unbounded
    len: Option<u64>,
}

/// Converts a RangeBounds object interpreted cruelly as a HTTP range into a [`SeekRange`] object
fn range_to_seek(range: impl RangeBounds<u64> + std::fmt::Debug) -> Result<SeekRange, ApiError> {
    let start = match range.start_bound() {
        Bound::Included(start) => Some(*start),
        Bound::Excluded(start) => Some(*start + 1),
        Bound::Unbounded => None,
    };

    let (seek, len) = match (start, range.end_bound()) {
        // Included/Excluded doens't make semantically sense because where talking about lengths not start or endpoints
        // so we just treat them the same ¯\_(ツ)_/¯
        (None, Bound::Included(len)) | (None, Bound::Excluded(len)) => {
            (SeekFrom::End(-(*len as i64)), Some(*len))
        }
        (Some(start), Bound::Included(end)) => {
            let end = *end + 1;
            if start >= end {
                return Err(ApiError::InvalidRange("range must end at or after start"));
            }
            (SeekFrom::Start(start), Some(end - start))
        }
        (Some(start), Bound::Excluded(end)) => {
            if start >= *end {
                return Err(ApiError::InvalidRange("range must end at or after start"));
            }
            (SeekFrom::Start(start), Some(*end - start))
        }
        (Some(start), Bound::Unbounded) => (SeekFrom::Start(start), None),
        (None, Bound::Unbounded) => {
            return Err(ApiError::InvalidRange(
                "either range start or suffix length has to be defined",
            ))
        }
    };

    Ok(SeekRange { seek, len })
}

/// The read portion of the file together with the start index
struct ReadRange {
    /// start index of the read bytes
    start: u64,
    /// bytes we've read
    buf: Vec<u8>,
}

/// Try's reading the specified range of bytes (cruelly interpreted as a HTTP range) from a file
async fn read_range<R>(mut file: File, range: R) -> Result<ReadRange, ApiError>
where
    R: Copy,
    R: RangeBounds<u64>,
    R: std::fmt::Debug,
{
    let SeekRange { mut seek, len } = range_to_seek(range).context("invalid http range")?;

    // seeking before the beginning of a file returns an error, so we have to check before calling `file.seek(seek)`
    // seeking after the end of a file is fine though
    let file_len = file
        .metadata()
        .await
        .context("could not retrieve file metadata")?
        .len();

    if let SeekFrom::End(offset) = seek {
        let offset = -offset;
        // requested suffix length is larger than the file, so just cap it at SeekFrom::Start(0)
        if offset as u64 > file_len {
            // alternatively we can throw a RangeNotSatisfiable
            // return Err(ApiError::RangeNotSatisfiable("requested suffix length larger than file"));
            seek = SeekFrom::Start(0);
        }
    }

    let seek_pos = file.seek(seek).await.context("could not seek file")?;

    if let SeekFrom::Start(offset) = seek {
        if seek_pos < offset {
            // the requested range starts after the end of our file
            return Err(ApiError::RangeNotSatisfiable(
                "requested range starts after end of file",
            ));
        }
    }

    let buf = match len {
        Some(len) => {
            let mut buf = Vec::<u8>::with_capacity(len as usize);
            file.take(len)
                .read_to_end(&mut buf)
                .await
                .context(format!("could not read range {range:?} from file"))?;
            buf
        }
        None => {
            let mut buf = Vec::<u8>::new();
            file.read_to_end(&mut buf)
                .await
                .context(format!("could not read range {range:?} from file"))?;
            buf
        }
    };

    Ok(ReadRange {
        start: seek_pos,
        buf,
    })
}

/// Reads the current hook status
pub async fn read_status(id: &Uuid, dirs: &ProjectDirs) -> Result<Info, ApiError> {
    let (info_path, _) = get_hook_files(dirs, id, false).await?;
    ensure_file_exists(info_path.clone(), "No hook with the matching ID was found")?;
    let info_string = read_to_string(info_path)
        .await
        .context("Couldn't read hook info")?;
    let info: Info = serde_json::from_str(&info_string)
        .context(format!("Couldn't parse json info for instance {id}"))?;
    Ok(info)
}

/// Ensures that a file exists. If it doesn't exist, this function returns an `ApiError::NotFound`
pub fn ensure_file_exists(path: PathBuf, error: &'static str) -> Result<(), ApiError> {
    if path.exists() {
        Ok(())
    } else {
        Err(ApiError::NotFound(error))
    }
}

/// Function for returning the info file path and the log directory of a given hook
pub async fn get_hook_files(
    dirs: &ProjectDirs,
    id: &Uuid,
    create: bool,
) -> Result<(PathBuf, PathBuf), ApiError> {
    let mut data_dir = dirs.data_dir().to_path_buf();
    let id_string = id.to_hyphenated().to_string();
    for i in 0..4 {
        data_dir.push(id_string.substring(2 * i, 2 * i + 2));
    }
    data_dir.push(id_string.substring(9, id_string.len()));
    if create {
        create_dir_all(&data_dir)
            .await
            .context("Couldn't create hook directory")?;
    } else {
        ensure_file_exists(data_dir.clone(), "No hook with matching ID was found")?;
    }
    let mut hook_info_path = data_dir.clone();
    hook_info_path.push("info.json");
    let mut hook_log_path = data_dir.clone();
    hook_log_path.push("log");
    create_dir_all(&hook_log_path)
        .await
        .context("Couldn't create hook log directory")?;
    Ok((hook_info_path, hook_log_path))
}

/// Helper function that takes the output stream of a hook instance and writes it to the respective
/// log file
pub async fn write_stream_to_file<T>(mut stream: T, path: PathBuf) -> Result<(), ApiError>
where
    T: AsyncRead + Send + Unpin,
{
    let mut file = File::create(path.clone()).await.context(format!(
        "Couldn't create log file {}",
        path.to_string_lossy()
    ))?;
    copy(&mut stream, &mut file).await.context(format!(
        "Couldn't write output to log file {}",
        path.to_string_lossy()
    ))?;
    Ok(())
}

/// Helper fuction that writes the hook info after the hook has been spawned
pub async fn write_initial_hook_info(
    hook: &Hook,
    request: Request,
    file: PathBuf,
) -> Result<(), ApiError> {
    let started = Utc::now();
    let info = Info {
        request,
        config: hook.clone(),
        running: true,
        success: None,
        started,
        finished: None,
    };
    let info = json_pretty_string(&info).context("Couldn't serialize hook info into string")?;
    write(file, info)
        .await
        .context("Couldn't write hook info file")?;
    Ok(())
}
