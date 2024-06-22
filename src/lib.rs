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

## Choice a of buffer

Use [`MmapBuffer`](struct.MmapBuffer.html) unless:

- [vmap](https://github.com/kalamay/vmap-rs) is not available for your platform (e.g. no support for `mmap`),
- you need very small buffers (smaller than 1 memory page),
- you're about to create a lot of buffers in a short period of time ([`new()`](trait.Buffer.html#tymethod.new) is relatively expensive),
- you're expecting buffer to grow a lot (consider, if possible, preallocating larger buffers through [`BufRefReaderBuilder.capacity`](struct.BufRefReaderBuilder.html#method.capacity)),
- you have some very special concerns re: memory maps and malloc bypass (special allocators, possible kernel inefficiency due to large amount of mapped memory regions etc.).

## Examples

Read data word by word:

```
use buf_ref_reader::*;

fn read<B: Buffer>() -> Result<(), Error>
where
	Error: From<B::Error>,
	// add this if you plan to `unwrap()` errors returned by `read()` et al.
	//B::Error: std::fmt::Debug,
{
	// &[u8] implements Read, hence we use it as our data source for this example
	let data = b"lorem ipsum dolor sit amet";
	let mut r = BufRefReaderBuilder::new(&data[..])
		.capacity(4)
		.build::<B>()?;

	assert_eq!(r.read_until(b' ')?, Some(&b"lorem "[..]));
	assert_eq!(r.read_until(b' ')?, Some(&b"ipsum "[..]));
	assert_eq!(r.read_until(b' ')?, Some(&b"dolor "[..]));
	assert_eq!(r.read_until(b' ')?, Some(&b"sit "[..]));
	assert_eq!(r.read_until(b' ')?, Some(&b"amet"[..]));
	assert_eq!(r.read_until(b' ')?, None); // EOF
	assert_eq!(r.read_until(b' ')?, None);

	Ok(())
}

fn main() {
	read::<VecBuffer>().unwrap();
	read::<MmapBuffer>().unwrap();
}
```
*/

#![warn(missing_docs)]

use quick_error::quick_error;

use std::io::{self, Read};
use memchr::memchr;

mod buffer;
pub use buffer::{
	Buffer,
	VecBuffer,
	MmapBuffer,
};

use std::convert::From;

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
	pub fn build<B: Buffer>(self) -> Result<BufRefReader<R, B>, B::Error> {
		Ok(BufRefReader {
			src: self.src,
			buf: B::new(self.bufsize)?,
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
		Buf(err: vmap::Error) { from() }
	}
}
impl From<()> for Error {
	// VecBuffer never emits errors, it only panics
	fn from(_: ()) -> Self {
		unimplemented!()
	}
}

impl<R: Read, B: Buffer> BufRefReader<R, B>
where Error: From<B::Error>
{
	/// Creates buffered reader with default options. Look for [`BufRefReaderBuilder`](struct.BufRefReaderBuilder.html) for tweaks.
	pub fn new(src: R) -> Result<BufRefReader<R, B>, B::Error> {
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
	use std::fmt::Debug;

	fn read_until_empty_lines<B: Buffer>()
	where
		B::Error: Debug,
		Error: From<B::Error>,
	{
		// two spaces, three spaces, two spaces
		let mut r = BufRefReaderBuilder::new(&b"  lorem   ipsum  "[..])
			.capacity(4)
			.build::<B>()
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

	#[test] fn read_until_empty_lines_vec()  { read_until_empty_lines::<VecBuffer>() }
	#[test] fn read_until_empty_lines_mmap() { read_until_empty_lines::<MmapBuffer>() }

	fn read_until_words<B: Buffer>()
	where
		B::Error: Debug,
		Error: From<B::Error>,
	{
		let mut r = BufRefReaderBuilder::new(WORDS)
			.capacity(4)
			.build::<B>()
			.unwrap();
		let mut words = WORDS.split(|&c| c == b'\n');
		while let Ok(Some(slice_buf)) = r.read_until(b'\n') {
			let mut ref_word = words.next().unwrap()
				.to_vec();
			ref_word.push(b'\n');
			assert_eq!(slice_buf, &ref_word[..]);
		}

		// reader: returned immediately after hitting EOF past last b'\n'
		// words: this is .split(), hence empty string past last b'\n'
		assert_eq!(words.next(), Some(&b""[..]));

		assert_eq!(words.next(), None);
	}

	#[test] fn read_until_words_vec()  { read_until_words::<VecBuffer>() }
	#[test] fn read_until_words_mmap() { read_until_words::<MmapBuffer>() }

	// like read_until_words, but splits by rarest character, which is b'Q'
	// also uses slightly bigger initial buffers
	fn read_until_words_long<B: Buffer>()
	where
		B::Error: Debug,
		Error: From<B::Error>,
	{
		let mut r = BufRefReaderBuilder::new(WORDS)
			.capacity(32)
			.build::<B>()
			.unwrap();
		let mut words = WORDS.split(|&c| c == b'Q').peekable();
		while let Ok(Some(slice_buf)) = r.read_until(b'Q') {
			let mut ref_word = words.next().unwrap()
				.to_vec();
			if words.peek() != None {
				ref_word.push(b'Q');
			}
			assert_eq!(slice_buf, &ref_word[..]);
		}

		assert_eq!(words.next(), None);
	}

	#[test] fn read_until_words_long_vec()  { read_until_words_long::<VecBuffer>() }
	#[test] fn read_until_words_long_mmap() { read_until_words_long::<MmapBuffer>() }

	fn read<B: Buffer>()
	where
		B::Error: Debug,
		Error: From<B::Error>,
	{
		let mut r = BufRefReaderBuilder::new(&b"lorem ipsum dolor sit amet"[..])
			.capacity(4)
			.build::<B>()
			.unwrap();
		assert_eq!(r.read(5).unwrap(), Some(&b"lorem"[..]));
		assert_eq!(r.read(6).unwrap(), Some(&b" ipsum"[..]));
		assert_eq!(r.read(1024).unwrap(), Some(&b" dolor sit amet"[..]));
		assert_eq!(r.read(1).unwrap(), None);
	}

	#[test] fn read_vec()  { read::<VecBuffer>() }
	#[test] fn read_mmap() { read::<MmapBuffer>() }

	fn read_words<B: Buffer>(cap: usize, read: usize)
	where
		B::Error: Debug,
		Error: From<B::Error>,
	{
		let mut r = BufRefReaderBuilder::new(WORDS)
			.capacity(cap)
			.build::<B>()
			.unwrap();
		let mut words = WORDS.chunks(read);
		while let Ok(Some(slice_buf)) = r.read(read) {
			let ref_word = words.next().unwrap();
			assert_eq!(slice_buf, ref_word);
		}
		assert_eq!(words.next(), None);
	}

	#[test] fn read_words_vec_4x3() { read_words::<VecBuffer>(4, 3) }
	#[test] fn read_words_vec_4x5() { read_words::<VecBuffer>(4, 5) }
	#[test] fn read_words_mmap_4x3() { read_words::<MmapBuffer>(4, 3) }
	#[test] fn read_words_mmap_4x5() { read_words::<MmapBuffer>(4, 5) }
}
