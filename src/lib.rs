mod common;
pub mod entry;
pub mod fixtures;
mod id_tracker;
pub mod index;
pub mod segment;
pub mod segment_constructor;
pub mod spaces;
pub mod types;
pub mod vector_storage;

#[macro_use]
extern crate num_derive;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
