pub trait Buffer
where Self: std::marker::Sized
{
	type Error;
	fn new(cap: usize) -> Result<Self, Self::Error>;
	fn appendable(&mut self) -> &mut [u8];
	fn consume(&mut self, amount: usize) -> &[u8];
	fn len(&self) -> usize;
	fn enlarge(&mut self) -> Result<(), Self::Error>;
	fn filled(&self) -> &[u8];
	fn grow(&mut self, amount: usize);
}

mod vec;
pub use vec::*;

mod mmap;
pub use mmap::*;
