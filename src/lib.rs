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
		self.incr= incr;
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

	// returns true for EOF
	fn fill(&mut self) -> Result<bool> {
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

		match self.src.read(&mut self.buf[self.end..])? {
			0 => Ok(true), // EOF
			n => {
				self.end += n;
				Ok(false)
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
			if self.fill()? { break };
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
		loop {
			// fill and expand buffer until either:
			// - `delim` appears in the buffer
			// - EOF is reached
			if let Some(n) = memchr(delim, filled!(self)) {
				len = Some(n);
				break;
			}
			if self.fill()? { break };
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
}

#[cfg(test)]
mod bench_read {
	extern crate test;
	use test::{Bencher, black_box};
	use super::*;
	use std::io::{BufRead, BufReader};

	#[bench]
	fn bufref(b: &mut Bencher) {
		b.iter(|| {
			let mut r = BufRefReaderBuilder::new(&include_bytes!("/usr/share/dict/words")[..])
				.capacity(16)
				.increment(16)
				.create();
			while r.read(4).unwrap() != None {}
		})
	}

	#[bench]
	fn std(b: &mut Bencher) {
		b.iter(|| {
			let mut r = BufReader::with_capacity(16, &include_bytes!("/usr/share/dict/words")[..]);
			let mut buf = [0; 4];
			while r.read(&mut buf[..]).unwrap() != 0 {}
		})
	}
}

#[cfg(test)]
mod bench_read_until {
	extern crate test;
	use test::{Bencher, black_box};
	use super::*;
	use std::io::{BufRead, BufReader};


	#[bench]
	fn bufref(b: &mut Bencher) {
		b.iter(|| {
			let mut r = BufRefReaderBuilder::new(&include_bytes!("/usr/share/dict/words")[..])
				.capacity(16)
				.increment(16)
				.create();
			while r.read_until(b'\n').unwrap() != None {}
		})
	}

	#[bench]
	fn std_read_until(b: &mut Bencher) {
		b.iter(|| {
			let mut r = BufReader::with_capacity(16, &include_bytes!("/usr/share/dict/words")[..]);
			let mut buf = vec![];
			while r.read_until(b'\n', &mut buf).unwrap() != 0 {}
		})
	}

	// this is obviously slow due to utf8 validation
	/*
	#[bench]
	fn std_lines(b: &mut Bencher) {
		b.iter(|| {
			let mut r = BufReader::with_capacity(16, &include_bytes!("/usr/share/dict/words")[..]);
			for i in r.lines() {
				black_box(i);
			}
		})
	}
	*/
}
