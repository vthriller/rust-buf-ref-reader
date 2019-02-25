/*!
Faster, growable buffering reader for when there's little to no need to modify data, nor to keep it alive past next read.

`std::io::BufReader` works by copying data from its internal buffer into user-provided `Vec`/`String`,
or, in case of `.lines()`, by emitting new heap-allocated `String` for each iteration.
While convenient and versatile, this is not the fastest approach.

Instead, `BufRefReader` references its internal buffer with each read, returning `&[u8]`.
Lack of extra allocations yields better read performance in situations where most (if not all) of read data:

- requires no modifications,
- is never used outside of a loop body and does not need to be duplicated into the heap for future use.

While being more performant, this approach also severely limits applicability of this reader:

- it does not (and cannot) implement `BufRead` and cannot be used as a direct replacement for `BufReader`;
- returned values are only valid between calls to reading functions (i.e. they cannot outlive even a single loop cycle), and Rust's borrow checker will prevent you from using stale references;
- consequently, `BufRefReader` cannot be turned into an `Iterator` (here's an easy way to think about it: what would `Iterator::collect()` return?);
- returned references are immutable;
- obviously, there's also nothing that can return `String`s or `&str`s for you.

Additionaly, [slice-deque](https://github.com/gnzlbg/slice_deque) poses limitations of its own
(platform support, min buffer size, memory allocator bypass,
possible overhead due to how kernel handles address spaces with lots of maps).

## Examples

Read data word by word:

```
use buf_ref_reader::*;

# fn main() -> Result<(), Error> {
// &[u8] implements Read, hence we use it as our data source for this example
let data = b"lorem ipsum dolor sit amet";
let mut r = BufRefReaderBuilder::new(&data[..])
	.capacity(4)
	.build()?;

assert_eq!(r.read_until(b' ')?, Some(&b"lorem "[..]));
assert_eq!(r.read_until(b' ')?, Some(&b"ipsum "[..]));
assert_eq!(r.read_until(b' ')?, Some(&b"dolor "[..]));
assert_eq!(r.read_until(b' ')?, Some(&b"sit "[..]));
assert_eq!(r.read_until(b' ')?, Some(&b"amet"[..]));
assert_eq!(r.read_until(b' ')?, None); // EOF
assert_eq!(r.read_until(b' ')?, None);

# Ok(())
# }
```
*/

#![warn(missing_docs)]

use quick_error::quick_error;

use std::io::{self, Read};
use memchr::memchr;

mod buffer;
use buffer::VecBuffer;
use buffer::MmapBuffer;
use slice_deque::AllocError;

/**
Buffering reader.

See [module-level docs](index.html) for examples.
*/
pub struct BufRefReader<R, B> {
	src: R,
	buf: B,
}

/**
Builder for [`BufRefReader`](struct.BufRefReader.html).

See [module-level docs](index.html) for examples.
*/
pub struct BufRefReaderBuilder<R> {
	src: R,
	bufsize: usize,
}
impl<R: Read> BufRefReaderBuilder<R> {
	/// Creates new builder with given reader and default options.
	pub fn new(src: R) -> Self {
		BufRefReaderBuilder {
			src,
			bufsize: 8192,
		}
	}

	/// Set initial buffer capacity.
	pub fn capacity(mut self, bufsize: usize) -> Self {
		self.bufsize = bufsize;
		self
	}

	/// Create actual reader.
<<<<<<< HEAD
	pub fn build(self) -> Result<BufRefReader<R, MmapBuffer>, AllocError> {
	pub fn build(self) -> BufRefReader<R, VecBuffer> {
>>>>>>> buf-factored-out
		Ok(BufRefReader {
			src: self.src,
<<<<<<< HEAD
			buf: MmapBuffer::new(self.bufsize)?,
			buf: VecBuffer::new(self.bufsize),
>>>>>>> buf-factored-out
		})
	}
}

quick_error! {
	/// Error type that reading functions might emit
	#[derive(Debug)]
	pub enum Error {
		/// Error reading from actual reader
		IO(err: io::Error) { from() }
		/// Indicates failure to create/grow buffer
		Buf(err: AllocError) { from() }
	}
}

<<<<<<< HEAD
impl<R: Read> BufRefReader<R, MmapBuffer> {
impl<R: Read> BufRefReader<R, VecBuffer> {
>>>>>>> buf-factored-out
	/// Creates buffered reader with default options. Look for [`BufRefReaderBuilder`](struct.BufRefReaderBuilder.html) for tweaks.
<<<<<<< HEAD
	pub fn new(src: R) -> Result<BufRefReader<R, MmapBuffer>, AllocError> {
	pub fn new(src: R) -> BufRefReader<R, VecBuffer> {
>>>>>>> buf-factored-out
		BufRefReaderBuilder::new(src)
			.build()
	}

	// returns Some(where appended data starts within the filled part of the buffer),
	// or None for EOF
	#[inline]
	fn fill(&mut self) -> Result<Option<usize>, Error> {
		self.buf.enlarge()?;

		let old_len = self.buf.len();

		match self.src.read(self.buf.appendable())? {
			0 => Ok(None), // EOF
			n => {
				self.buf.grow(n);
				Ok(Some(old_len))
			}
		}
	}

	/**
	Returns requested amount of bytes, or less if EOF prevents reader from fulfilling the request.

	Returns:

	- `Ok(Some(data))` with, well, data,
	- `Ok(None)` if no more data is available,
	- `Err(err)`: see `std::io::Read::read()`
	*/
	#[inline]
	pub fn read(&mut self, n: usize) -> Result<Option<&[u8]>, Error> {
		while n > self.buf.len() {
			// fill and expand buffer until either:
			// - buffer starts holding the requested amount of data
			// - EOF is reached
			if self.fill()?.is_none() { break };
		}
		if self.buf.len() == 0 {
			// reading past EOF
			Ok(None)
		} else {
			let output = self.buf.consume(n);
			Ok(Some(output))
		}
	}

	/**
	Returns bytes up until and including `delim`, or until EOF mark. If no content is available, returns `None`.

	Returns:

	- `Ok(Some(data))` with, well, data,
	- `Ok(None)` if no more data is available,
	- `Err(err)`: see `std::io::Read::read()`
	*/
	#[inline]
	pub fn read_until(&mut self, delim: u8) -> Result<Option<&[u8]>, Error> {
		let mut len = None;
		// position within filled part of the buffer,
		// from which to continue search for character
		let mut pos = 0;
		loop {
			// fill and expand buffer until either:
			// - `delim` appears in the buffer
			// - EOF is reached
			if let Some(n) = memchr(delim, &self.buf.filled()[pos..]) {
				len = Some(pos+n);
				break;
			}
			pos = match self.fill()? {
				None => break, // EOF
				Some(pos) => pos,
			};
		}

		match len {
			None => { // EOF
				if self.buf.len() == 0 {
					Ok(None)
				} else {
					let output = self.buf.consume(self.buf.len());
					Ok(Some(output))
				}
			},
			Some(len) => {
				let len = len + 1; // also include matching delimiter
				let output = self.buf.consume(len);
				Ok(Some(output))
			},
		}
	}
}

#[cfg(test)]
static WORDS: &'static [u8] = include_bytes!("/usr/share/dict/words");

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn read_until_empty_lines() {
		// two spaces, three spaces, two spaces
		let mut r = BufRefReaderBuilder::new(&b"  lorem   ipsum  "[..])
			.capacity(4)
			.build()
			.unwrap();
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b" "[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b" "[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"lorem "[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b" "[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b" "[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"ipsum "[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b" "[..]));
		assert_eq!(r.read_until(b' ').unwrap(), None);
	}

	#[test]
	fn read_until_words() {
		let mut r = BufRefReaderBuilder::new(WORDS)
			.capacity(4)
			.build()
			.unwrap();
		let mut words = WORDS.split(|&c| c == b'\n');
		while let Ok(Some(slice_buf)) = r.read_until(b'\n') {
			let mut slice_words = words.next().unwrap()
				.to_vec();
			slice_words.push(b'\n');
			assert_eq!(slice_buf, &slice_words[..]);
		}

		// reader: returned immediately after hitting EOF past last b'\n'
		// words: this is .split(), hence empty string past last b'\n'
		assert_eq!(words.next(), Some(&b""[..]));

		assert_eq!(words.next(), None);
	}

	// like read_until_words, but splits by rarest character, which is b'Q'
	// also uses slightly bigger initial buffers
	#[test]
	fn read_until_words_long() {
		let mut r = BufRefReaderBuilder::new(WORDS)
			.capacity(32)
			.build()
			.unwrap();
		let mut words = WORDS.split(|&c| c == b'Q').peekable();
		while let Ok(Some(slice_buf)) = r.read_until(b'Q') {
			let mut slice_words = words.next().unwrap()
				.to_vec();
			if words.peek() != None {
				slice_words.push(b'Q');
			}
			assert_eq!(slice_buf, &slice_words[..]);
		}

		assert_eq!(words.next(), None);
	}

	#[test]
	fn read() {
		let mut r = BufRefReaderBuilder::new(&b"lorem ipsum dolor sit amet"[..])
			.capacity(4)
			.build()
			.unwrap();
		assert_eq!(r.read(5).unwrap(), Some(&b"lorem"[..]));
		assert_eq!(r.read(6).unwrap(), Some(&b" ipsum"[..]));
		assert_eq!(r.read(1024).unwrap(), Some(&b" dolor sit amet"[..]));
		assert_eq!(r.read(1).unwrap(), None);
	}

	fn read_words(cap: usize, read: usize) {
		let mut r = BufRefReaderBuilder::new(WORDS)
			.capacity(cap)
			.build()
			.unwrap();
		let mut words = WORDS.chunks(read);
		while let Ok(Some(slice_buf)) = r.read(read) {
			let slice_words = words.next().unwrap();
			assert_eq!(slice_buf, slice_words);
		}
		assert_eq!(words.next(), None);
	}

	#[test]
	fn read_words_4x3() {
		read_words(4, 3)
	}

	#[test]
	fn read_words_4x5() {
		read_words(4, 5)
	}
}
