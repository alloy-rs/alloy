//! Allocation-free checks before decoding contract-controlled dynamic ABI values.

use super::{CcipReadConfig, CcipReadError};
use alloy_primitives::U256;
use alloy_sol_types::Error;

/// Checks counts and total decoded dynamic data, counting overlapping references separately.
pub(super) fn offchain_lookup(data: &[u8], config: &CcipReadConfig) -> Result<(), CcipReadError> {
    let mut limits = Limits::new(config);
    let result = (|| {
        let data = data.get(4..).ok_or(Error::Overrun)?;
        let mut head = data;
        word(&mut head)?; // sender
        limits.urls(indirect(&mut head, data)?)?;
        limits.bytes(indirect(&mut head, data)?, false)?;
        word(&mut head)?; // callbackFunction
        limits.bytes(indirect(&mut head, data)?, false)
    })();
    result.map_err(|error| match error {
        CheckError::Abi(error) => CcipReadError::InvalidOffchainLookup(error),
        CheckError::Limit(message) => CcipReadError::ResourceLimit(message.into()),
    })
}

/// Checks every request before allocating an ENSIP-21 batch, including nested batches.
pub(super) fn batch(data: &[u8], config: &CcipReadConfig) -> Result<(), CcipReadError> {
    let mut limits = Limits::new(config);
    let result = (|| {
        if data.len() > config.max_revert_data_size {
            return Err(CheckError::Limit("batch data exceeds revert data size limit"));
        }
        let data = data.get(4..).ok_or(Error::Overrun)?;
        let mut head = data;
        let (count, array) = limits.array(
            indirect(&mut head, data)?,
            config.max_batch_size,
            "batch request count exceeds configured limit",
        )?;
        let mut head = array;
        for _ in 0..count {
            let request = indirect(&mut head, array)?;
            let mut fields = request;
            word(&mut fields)?; // sender
            limits.urls(indirect(&mut fields, request)?)?;
            limits.bytes(indirect(&mut fields, request)?, false)?;
        }
        Ok(())
    })();
    result.map_err(|error| match error {
        CheckError::Abi(error) => CcipReadError::InvalidBatch(error.to_string()),
        CheckError::Limit(message) => CcipReadError::ResourceLimit(message.into()),
    })
}

enum CheckError {
    Abi(Error),
    Limit(&'static str),
}

impl From<Error> for CheckError {
    fn from(error: Error) -> Self {
        Self::Abi(error)
    }
}

struct Limits<'a> {
    config: &'a CcipReadConfig,
    remaining: usize,
}

impl<'a> Limits<'a> {
    const fn new(config: &'a CcipReadConfig) -> Self {
        Self { config, remaining: config.max_revert_data_size }
    }

    fn charge(&mut self, size: usize) -> Result<(), CheckError> {
        self.remaining = self
            .remaining
            .checked_sub(size)
            .ok_or(CheckError::Limit("decoded ABI data exceeds revert data size limit"))?;
        Ok(())
    }

    fn array<'b>(
        &mut self,
        mut data: &'b [u8],
        max: usize,
        message: &'static str,
    ) -> Result<(usize, &'b [u8]), CheckError> {
        let count = offset(&mut data)?;
        if count > max {
            return Err(CheckError::Limit(message));
        }
        let size = count.checked_mul(32).ok_or(Error::Overrun)?;
        data.get(..size).ok_or(Error::Overrun)?;
        self.charge(size)?;
        // Element offsets are relative to the word after the array length.
        Ok((count, data))
    }

    fn urls(&mut self, data: &[u8]) -> Result<(), CheckError> {
        let (count, array) = self.array(
            data,
            self.config.max_gateway_urls,
            "gateway URL count exceeds configured limit",
        )?;
        let mut head = array;
        for _ in 0..count {
            self.bytes(indirect(&mut head, array)?, true)?;
        }
        Ok(())
    }

    fn bytes(&mut self, mut data: &[u8], string: bool) -> Result<(), CheckError> {
        let len = offset(&mut data)?;
        let bytes = data.get(..len).ok_or(Error::Overrun)?;
        self.charge(len)?;
        if string {
            // The ABI decoder replaces invalid UTF-8 with U+FFFD. Charge any extra bytes
            // before it allocates the owned String, without allocating a lossy copy here.
            for chunk in bytes.utf8_chunks() {
                if !chunk.invalid().is_empty() {
                    self.charge(3 - chunk.invalid().len())?;
                }
            }
        }
        Ok(())
    }
}

fn word<'a>(head: &mut &'a [u8]) -> Result<&'a [u8; 32], Error> {
    let (word, rest) = head.split_first_chunk::<32>().ok_or(Error::Overrun)?;
    *head = rest;
    Ok(word)
}

fn offset(head: &mut &[u8]) -> Result<usize, Error> {
    U256::from_be_bytes(*word(head)?).try_into().map_err(|_| Error::Overrun)
}

fn indirect<'a>(head: &mut &[u8], base: &'a [u8]) -> Result<&'a [u8], Error> {
    base.get(offset(head)?..).ok_or(Error::Overrun)
}
