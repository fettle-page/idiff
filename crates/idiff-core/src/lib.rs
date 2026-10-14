//! Everything idiff decides, with no Windows calls: the snapshot model, the diff, who owns each
//! change, the verdict and the receipt. Tested anywhere with JSON fixtures.

pub mod command;
pub mod copy;
pub mod model;
