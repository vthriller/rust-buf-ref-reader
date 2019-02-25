/*
SliceDeque is quite inconvenient, e.g.:
- you still need to resort to `unsafe {}`
  to append data through `&mut [u8]`
  and advance tail position accordingly,
- you still need to be careful with pointers to head/tail
  and actively prevent underruns/overruns,
- borrow checker still messes up reader methods
  (e.g. `buf.move_head()` after `buf.as_slice()`).
Hence we drop to a lower level thing.

But even this lower level buffer is not without its own warts:
it operates with an overall size of a mirrored buffer, not one of its halves,
which results in a lot of unnecessary `* 2` on the user's side
and lots of `/ 2`, `% 2`, and `assert!`s in the crate that implements it.
And, yes, this is just utterly confusing:
why report len() of X when you can only put X/2 elements inside?
*/
use slice_deque::{Buffer, AllocError};

pub struct MmapBuffer {
	buf: Buffer<u8>,
	/*
	We keep size of a `buf`'s size on our own because `buf.len()`:
	- returns twice the amount of data buffer can actually handle
	  (i.e. size of mmaped region with two mirrors),
	  which makes it confusing, and it's also an error waiting to happen,
	- it also causes an immutable borrowing of `buf`,
	  thus making most of manipulations with `buf`'s content inconvenient.
	*/
	bufsize: usize,
	// position of data within the `buf`
	start: usize,
	len: usize,
}
impl super::Buffer for MmapBuffer {
	type Error = AllocError;
	fn new(size: usize) -> Result<Self, AllocError> {
		let buf = Buffer::uninitialized(size * 2)?;
		// slice-deque will round bufsize to the nearest page size or something,
		// so we query it back here
		let bufsize = buf.len() / 2;
		Ok(MmapBuffer {
			buf, bufsize,
			start: 0, len: 0,
		})
	}
	fn filled(&self) -> &[u8] {
		&(unsafe {
			self.buf.as_slice()
		})[ self.start .. (self.start + self.len) ]
	}
	// make room for new data one way or the other
	fn enlarge(&mut self) -> Result<(), AllocError> {
		if self.start == 0 && self.len == self.bufsize {
			/*
			we used to have configurable increments for the bufsize
			now though we double buffer size, just like rust's vec/raw_vec do
			*/
			self.bufsize *= 2;
			let mut new = Buffer::uninitialized(self.bufsize * 2)?;
			// see .new() for th reasons why we read bufsize back
			self.bufsize = new.len() / 2;
			// move data at the start of new buffer
			unsafe {
				core::ptr::copy(
					self.buf.as_mut_slice()[self.start..].as_mut_ptr(),
					new.as_mut_slice().as_mut_ptr(),
					self.len,
				);
			}
			self.start = 0;
			self.buf = new;
		} else {
			// there's plenty of room in the buffer,
			// nothing to do here
		}
		Ok(())
	}
	/*
	return b-through-a:
	| a--b | a--b |
	|-b  a-|-b  a-|
	*/
	fn appendable(&mut self) -> &mut [u8] {
		let end = self.start + self.len;
		let remaining = self.bufsize - self.len;
		&mut (unsafe {
			self.buf.as_mut_slice()
		})[ end .. (end+remaining) ]
	}
	fn grow(&mut self, amount: usize) {
		self.len += amount;
	}
	/*
	returns reference to first half of the buffer
	up to the size of `amount`,
	which is going to be discarded
	after lifetime of returned slice comes to an end
	*/
	fn consume(&mut self, amount: usize) -> &[u8] {
		let start = self.start;
		let amount = std::cmp::min(amount, self.len());

		self.start += amount;
		if self.start >= self.bufsize {
			// keep self.start within bufsize
			self.start -= self.bufsize;
		}
		self.len -= amount;
		&(unsafe {
			self.buf.as_mut_slice()
		})[ start .. (start+amount) ]
	}
	fn len(&self) -> usize {
		self.len
	}
}
