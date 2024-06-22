/**
This trait abstracts common operations with actual buffer from implementation details

## Example usage

```no_run
use buf_ref_reader::Buffer;
use memchr::memchr;
use std::io::Read;

# fn foo<SomeBuffer: Buffer, R: Read>(mut input: R) -> Result<(), SomeBuffer::Error> {
// allocate 128 bytes of buffer or more
let mut buf = SomeBuffer::new(128)?;

// write data into free part of the buffer
let read = input.read(buf.appendable()).unwrap();
// append actually written bytes
buf.mark_appended(read);

// read part of written data back
// this slice is only valid until another call to one of `buf`'s methods
let chunk = buf.consume(16);
let _ = chunk.len();

// we can also peek into filled part of the buffer
// as with `consume()`, this slice also has limited shelf life
let nl = memchr(b'\n', buf.filled());

if buf.appendable().len() == 0 {
	// reserve some space before appending even more data
	buf.enlarge()?;
}
let read = input.read(buf.appendable()).unwrap();
buf.mark_appended(read);

// borrow checker will prevent `chunk` from being used at this point,
// and that makes sense as data might've been reallocated or destroyed
// during further manipulations with the buffer (e.g. `enlarge()`)
//let _ = chunk.len();

# Ok(())
# }
```
*/
pub trait Buffer
where Self: std::marker::Sized
{
	/// Error type emitted if failed to (re)allocate the buffer
	type Error;
	/// Allocate new buffer of at least size `cap`, or more.
	fn new(cap: usize) -> Result<Self, Self::Error>;
	/**
	Part of the buffer next to the [`filled()`](#tymethod.filled) that can be used to append data.

	Use [`mark_appended()`](#tymethod.mark_appended) to actually append data written to this slice.
	*/
	fn appendable(&mut self) -> &mut [u8];
	/// Attaches `amount` bytes of [`appendable()`](#tymethod.appendable)
	/// to [`filled()`](#tymethod.filled) part of the buffer
	fn mark_appended(&mut self, amount: usize);
	/**
	Split [`filled()`](#tymethod.filled) part of the buffer,
	returning up to `amount` bytes from the beginning while also marking them as discarded
	right after lifetime of returned slice ends (i.e. before another call to any of `Buffer`'s methods that accepts `&mut self`).
	*/
	fn consume(&mut self, amount: usize) -> &[u8];
	/**
	Grow [`appendable()`](#tymethod.appendable) part of the buffer one way or the other
	(by e.g. reallocating filled part of the buffer, or reallocating buffer itself)

	Does nothing if `appendable()` has some capacity left.
	*/
	fn enlarge(&mut self) -> Result<(), Self::Error>;
	/// Return filled part of the buffer
	fn filled(&self) -> &[u8];
	/**
	Size of [`filled()`](#tymethod.filled) part of the buffer

	This is generally faster (and a bit more readable) than equivalent call to `.filled().len()`.
	*/
	fn len(&self) -> usize;
}

mod vec;
pub use vec::*;

mod mmap;
pub use mmap::*;
