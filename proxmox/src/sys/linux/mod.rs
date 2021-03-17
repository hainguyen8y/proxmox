//! Linux specific helpers and syscall wrapper

use anyhow::*;
use getrandom::getrandom;
pub mod magic;
pub mod pid;
pub mod procfs;
pub mod pty;
pub mod tty;

/// Get pseudo random data (/dev/urandom)
pub fn random_data(size: usize) -> Result<Vec<u8>, Error> {
    let mut buffer = crate::tools::vec::undefined(size);
    fill_with_random_data(&mut buffer)?;

    Ok(buffer)
}

/// Fill buffer with pseudo random data (/dev/urandom)
///
/// This code uses the Linux syscall getrandom() - see "man 2 getrandom".
pub fn fill_with_random_data(buffer: &mut [u8]) -> Result<(), Error> {
    getrandom(buffer)?;
    Ok(())
}
