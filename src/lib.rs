//! Http , Https parser state machine
//!
//!

mod headers;
pub mod parser;
mod url;

#[cfg(test)]
mod tests {
    use super::*;
}
