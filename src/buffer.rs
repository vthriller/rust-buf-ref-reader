// https://github.com/rust-lang/rust/issues/54236
use copy_in_place::*;

pub struct Buffer {
	buf: Vec<u8>,
	incr: usize,
	// where actual data resides within the `buf`
	start: usize,
	end: usize,
}
impl Buffer {
	pub(crate) fn new(size: usize, incr: usize) -> Self {
		let mut buf = Vec::with_capacity(size);
		unsafe { buf.set_len(size); }
		Buffer {
			buf, incr,
			start: 0, end: 0,
		}
	}
	// make room for new data one way or the other
	pub(crate) fn enlarge(&mut self) {
		//if self.start == 0 && self.end == self.buf.len() {
		if self.len() == self.buf.len() {
			// this buffer is already full, expand
			self.buf.reserve(self.incr);
			unsafe { self.buf.set_len(self.buf.len() + self.incr) };
		} else {
			// reallocate and fill existing buffer
			if self.end - self.start != 0 {
				//self.buf.copy_within(self.start..self.end, 0)
				copy_in_place(&mut self.buf, self.start..self.end, 0);
			}
			self.end -= self.start;
			self.start = 0;
		}
	}
	pub(crate) fn len(&self) -> usize {
		self.end - self.start
	}
	pub(crate) fn filled(&self) -> &[u8] {
		&self.buf[ self.start .. self.end ]
	}
	pub(crate) fn appendable(&mut self) -> &mut [u8] {
		&mut self.buf[ self.end .. ]
	}
	pub(crate) fn grow(&mut self, amount: usize) {
		self.end += amount;
	}
	/*
	before:
	|  xxxyyy |
	   |    |end
	   |start

	after:
	|  xxxyyy |
	   | || |end
	   | ||start
	   |-|return value
	*/
	pub(crate) fn consume(&mut self, amount: usize) -> &[u8] {
		let amount = std::cmp::min(amount, self.len());
		let start = self.start;
		self.start += amount;
		&self.buf[ start .. (start+amount) ]
	}
}
