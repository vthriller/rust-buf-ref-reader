#![feature(copy_within)]
#![feature(test)]

use std::io::{Read, Result};
use memchr::memchr;

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

pub struct BufRefReaderBuilder<R> {
	src: R,
	bufsize: usize,
	incr: usize,
}
impl<R: Read> BufRefReaderBuilder<R> {
	pub fn new(src: R) -> Self {
		BufRefReaderBuilder {
			src,
			bufsize: 8192,
			incr: 8192,
		}
	}

	pub fn capacity(mut self, bufsize: usize) -> Self {
		self.bufsize = bufsize;
		self
	}

	pub fn increment(mut self, incr: usize) -> Self {
		if incr == 0 {
			panic!("non-positive buffer increments requested")
		}
		self.incr = incr;
		self
	}

	pub fn create(self) -> BufRefReader<R> {
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
	pub fn new(src: R) -> BufRefReader<R> {
		BufRefReaderBuilder::new(src)
			.create()
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
				self.buf.copy_within(self.start..self.end, 0)
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

	/// Returns bytes until `delim` or EOF is reached. If no content available, returns `None`.
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
	fn read_until() {
		let mut r = BufRefReaderBuilder::new(&b"lorem ipsum dolor sit amet"[..])
			.capacity(4)
			.increment(4)
			.create();
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"lorem"[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"ipsum"[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"dolor"[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"sit"[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"amet"[..]));
		assert_eq!(r.read_until(b' ').unwrap(), None);
		assert_eq!(r.read_until(b' ').unwrap(), None);
	}

	#[test]
	fn read_until_words() {
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(4)
			.increment(4)
			.create();
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
			.create();
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
			.create();
		assert_eq!(r.read(5).unwrap(), Some(&b"lorem"[..]));
		assert_eq!(r.read(6).unwrap(), Some(&b" ipsum"[..]));
		assert_eq!(r.read(1024).unwrap(), Some(&b" dolor sit amet"[..]));
		assert_eq!(r.read(1).unwrap(), None);
	}

	fn read_words(cap: usize, incr: usize, read: usize) {
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(cap)
			.increment(incr)
			.create();
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

#[cfg(test)] mod bench_read;
#[cfg(test)] mod bench_read_until;
#[cfg(test)] mod bench_hashmap;

