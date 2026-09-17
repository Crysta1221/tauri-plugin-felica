//! Buffered transport reads shared by CCID and SOF-framed chipsets.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::error::{Error, Result};

pub(crate) fn remaining_until(deadline: Instant) -> Option<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
}

pub(crate) fn timeout_error() -> Error {
    Error::CommunicationError("timeout while waiting for data".into())
}

/// Reads exactly `len` bytes, keeping surplus packet bytes in `buffer`.
pub(crate) fn read_exact(
    read_packet: &mut impl FnMut(Duration) -> Result<Vec<u8>>,
    buffer: &mut VecDeque<u8>,
    len: usize,
    deadline: Instant,
) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        while out.len() < len {
            match buffer.pop_front() {
                Some(byte) => out.push(byte),
                None => break,
            }
        }
        if out.len() == len {
            break;
        }
        let remaining = remaining_until(deadline).ok_or_else(timeout_error)?;
        match read_packet(remaining) {
            Ok(chunk) => buffer.extend(chunk),
            Err(err) if is_timeout(&err) => return Err(timeout_error()),
            Err(err) => return Err(err),
        }
    }
    Ok(out)
}

pub(crate) fn is_timeout(err: &Error) -> bool {
    match err {
        Error::CommunicationError(message) => {
            message.contains("timed out") || message.contains("timeout")
        }
        Error::Rusb(rusb::Error::Timeout) => true,
        _ => false,
    }
}
