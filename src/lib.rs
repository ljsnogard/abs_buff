#![no_std]

// We always pull in `std` during tests, because it's just easier
// to write tests when you can assume you're on a capable platform
#[cfg(test)]
extern crate std;

mod empty_;
mod peeker_;
mod reader_;
mod writer_;

pub use empty_::EmptyBuffIter;
pub use peeker_::{TrBuffIterPeek, TrBuffIterTryPeek};
pub use reader_::{TrBuffIterRead, TrBuffIterTryRead};
pub use writer_::{TrBuffIterWrite, TrBuffIterTryWrite};

pub mod x_deps {
    pub use abs_sync;

    pub use abs_sync::x_deps::atomex;
}
