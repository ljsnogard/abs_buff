#![feature(try_trait_v2)]

#![no_std]

// We always pull in `std` during tests, because it's just easier
// to write tests when you can assume you're on a capable platform
#[cfg(test)]
extern crate std;

mod peeker_;
mod reader_;
mod writer_;

pub use peeker_::{DisabledBuffPeeker, TrBuffPeeker};
pub use reader_::{DisabledBuffReader, TrBuffReader};
pub use writer_::{DisabledBuffWriter, TrBuffWriter};

pub mod x_deps {
    pub use abs_sync;

    pub use abs_sync::x_deps::atomex;
}
