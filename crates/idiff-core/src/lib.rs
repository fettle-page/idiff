//! Everything idiff decides, with no Windows calls: the snapshot model, the diff, who owns each
//! change, the verdict and the receipt. Tested anywhere with JSON fixtures.

pub mod attribute;
pub mod command;
pub mod copy;
pub mod diff;
pub mod model;
pub mod noise;
pub mod publisher;
pub mod redact;
pub mod subject;
pub mod verdict;
