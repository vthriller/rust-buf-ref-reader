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

## Examples

Read data word by word:

```
use buf_ref_reader::BufRefReaderBuilder;

# fn main() -> std::io::Result<()> {
// &[u8] implements Read, hence we use it as our data source for this example
let data = b"lorem ipsum dolor sit amet";
let mut r = BufRefReaderBuilder::new(&data[..])
	.capacity(4)
	.increment(4)
	.build();

assert_eq!(r.read_until(b' ')?, Some(&b"lorem"[..]));
assert_eq!(r.read_until(b' ')?, Some(&b"ipsum"[..]));
assert_eq!(r.read_until(b' ')?, Some(&b"dolor"[..]));
assert_eq!(r.read_until(b' ')?, Some(&b"sit"[..]));
assert_eq!(r.read_until(b' ')?, Some(&b"amet"[..]));
assert_eq!(r.read_until(b' ')?, None); // EOF
assert_eq!(r.read_until(b' ')?, None);

# Ok(())
# }
```
*/

#![warn(missing_docs)]

use std::io::{Read, Result};
use memchr::memchr;
// https://github.com/rust-lang/rust/issues/54236
use copy_in_place::*;

/**
Buffering reader.

See [module-level docs](index.html) for examples.
*/
pub struct BufRefReader<R> {
	src: R,
	buf: Vec<u8>,
	incr: usize,
	// position of data within the `buf`
	start: usize,
	end: usize,
}

// XXX hack; see BufRefReader.filled below
macro_rules! filled {
	($self:ident) => (&$self.buf[ $self.start .. $self.end ])
}

/**
Builder for [`BufRefReader`](struct.BufRefReader.html).

See [module-level docs](index.html) for examples.
*/
pub struct BufRefReaderBuilder<R> {
	src: R,
	bufsize: usize,
	incr: usize,
}
impl<R: Read> BufRefReaderBuilder<R> {
	/// Creates new builder with given reader and default options.
	pub fn new(src: R) -> Self {
		BufRefReaderBuilder {
			src,
			bufsize: 8192,
			incr: 8192,
		}
	}

	/// Set initial buffer capacity.
	pub fn capacity(mut self, bufsize: usize) -> Self {
		self.bufsize = bufsize;
		self
	}

	/// Set buffer increments for when requested data does not fit into already existing buffer.
	pub fn increment(mut self, incr: usize) -> Self {
		if incr == 0 {
			panic!("non-positive buffer increments requested")
		}
		self.incr = incr;
		self
	}

	/// Create actual reader.
	pub fn build(self) -> BufRefReader<R> {
		let mut buf = Vec::with_capacity(self.bufsize);
		unsafe { buf.set_len(self.bufsize); }

		BufRefReader {
			src: self.src,
			buf,
			incr: self.incr,
			start: 0, end: 0,
		}
	}
}


impl<R: Read> BufRefReader<R> {
	/// Creates buffered reader with default options. Look for [`BufRefReaderBuilder`](struct.BufRefReaderBuilder.html) for tweaks.
	pub fn new(src: R) -> BufRefReader<R> {
		BufRefReaderBuilder::new(src)
			.build()
	}

	// returns Some(where appended data starts within the filled part of the buffer),
	// or None for EOF
	fn fill(&mut self) -> Result<Option<usize>> {
		if self.start == 0 && self.end == self.buf.len() {
			// this buffer is already full, expand
			self.buf.reserve(self.incr);
			unsafe { self.buf.set_len(self.buf.len() + self.incr) };
		} else {
			// reallocate and fill existing buffer
			if self.end - self.start != 0 {
				//self.buf.copy_within(self.start..self.end, 0)
				copy_in_place(&mut self.buf, self.start..self.end, 0)
			}
			// (A)..(A+B) → 0..B
			self.end -= self.start;
			self.start = 0;
		}

		let old_end = self.end;

		match self.src.read(&mut self.buf[self.end..])? {
			0 => Ok(None), // EOF
			n => {
				self.end += n;
				Ok(Some(old_end - self.start))
			}
		}
	}

	// returns usable part of `buf`
	// for now it is manually inlined with the filled!() macro
	// due to immutable borrowing of `self.start` (as part of `self` as a whole)
	// which causes E0506 when we try to advance `self.start` after using `self.filled()`
	/*
	#[inline]
	fn filled(&self) -> &[u8] {
		&self.buf[ self.start .. self.end ]
	}
	*/

	/**
	Returns requested amount of bytes, or less if EOF prevents reader from fulfilling the request.

	Returns:

	- `Ok(Some(data))` with, well, data,
	- `Ok(None)` if no more data is available,
	- `Err(err)`: see `std::io::Read::read()`
	*/
	pub fn read(&mut self, n: usize) -> Result<Option<&[u8]>> {
		while n > self.end - self.start {
			// fill and expand buffer until either:
			// - buffer starts holding the requested amount of data
			// - EOF is reached
			if self.fill()?.is_none() { break };
		}
		if self.start == self.end {
			// reading past EOF
			Ok(None)
		} else {
			let output = filled!(self);
			let output = if n < output.len() {
				&output[..n]
			} else {
				output
			};
			self.start += output.len();
			Ok(Some(output))
		}
	}

	/**
	Returns bytes until `delim` or EOF is reached. If no content is available, returns `None`.

	Returns:

	- `Ok(Some(data))` with, well, data,
	- `Ok(None)` if no more data is available,
	- `Err(err)`: see `std::io::Read::read()`
	*/
	pub fn read_until(&mut self, delim: u8) -> Result<Option<&[u8]>> {
		let mut len = None;
		// position within filled part of the buffer,
		// from which to continue search for character
		let mut pos = 0;
		loop {
			// fill and expand buffer until either:
			// - `delim` appears in the buffer
			// - EOF is reached
			if let Some(n) = memchr(delim, &filled!(self)[pos..]) {
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
				if self.start == self.end {
					Ok(None)
				} else {
					let output = &self.buf[ self.start .. self.end ];
					self.start = self.end;
					Ok(Some(output))
				}
			},
			Some(len) => {
				let output = &self.buf[ self.start .. self.start + len ];
				self.start += len + 1; // also silently consume delimiter
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
			.increment(4)
			.build();
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b""[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b""[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"lorem"[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b""[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b""[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"ipsum"[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b""[..]));
		assert_eq!(r.read_until(b' ').unwrap(), None);
	}

	#[test]
	fn read_until_words() {
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(4)
			.increment(4)
			.build();
		let mut words = WORDS.split(|&c| c == b'\n');
		while let Ok(Some(slice_buf)) = r.read_until(b'\n') {
			let slice_words = words.next().unwrap();
			assert_eq!(slice_buf, slice_words);
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
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(32)
			.increment(32)
			.build();
		let mut words = WORDS.split(|&c| c == b'Q');
		while let Ok(Some(slice_buf)) = r.read_until(b'Q') {
			let slice_words = words.next().unwrap();
			assert_eq!(slice_buf, slice_words);
		}

		assert_eq!(words.next(), None);
	}

	#[test]
	fn read() {
		let mut r = BufRefReaderBuilder::new(&b"lorem ipsum dolor sit amet"[..])
			.capacity(4)
			.increment(4)
			.build();
		assert_eq!(r.read(5).unwrap(), Some(&b"lorem"[..]));
		assert_eq!(r.read(6).unwrap(), Some(&b" ipsum"[..]));
		assert_eq!(r.read(1024).unwrap(), Some(&b" dolor sit amet"[..]));
		assert_eq!(r.read(1).unwrap(), None);
	}

	fn read_words(cap: usize, incr: usize, read: usize) {
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(cap)
			.increment(incr)
			.build();
		let mut words = WORDS.chunks(read);
		while let Ok(Some(slice_buf)) = r.read(read) {
			let slice_words = words.next().unwrap();
			assert_eq!(slice_buf, slice_words);
		}
		assert_eq!(words.next(), None);
	}

	#[test]
	fn read_words_4x4x3() {
		read_words(4, 4, 3)
	}

	#[test]
	fn read_words_4x4x5() {
		read_words(4, 4, 5)
	}
}
