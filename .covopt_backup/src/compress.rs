//! Zero-allocation time-series compression algorithms for `#![no_std]`.
//!
//! This module implements high-performance, branchless-preferred compression
//! techniques designed for edge devices and IoT sensor streams.
//! It utilizes ZigZag encoding to map signed integers to unsigned space,
//! and LEB128 (Variable-Length Quantity) encoding to pack integers into minimal bytes.

/// ZigZag encoding for 32-bit signed integers.
/// Maps signed integers to unsigned integers so that numbers
/// with a small absolute value have a small unsigned value.
/// e.g., 0 -> 0, -1 -> 1, 1 -> 2, -2 -> 3
#[inline]
pub const fn zigzag_encode_i32(n: i32) -> u32 {
    ((n << 1) ^ (n >> 31)) as u32
}

/// ZigZag decoding for 32-bit integers.
#[inline]
pub const fn zigzag_decode_u32(n: u32) -> i32 {
    ((n >> 1) as i32) ^ (-((n & 1) as i32))
}

/// ZigZag encoding for 64-bit signed integers.
#[inline]
pub const fn zigzag_encode_i64(n: i64) -> u64 {
    ((n << 1) ^ (n >> 63)) as u64
}

/// ZigZag decoding for 64-bit integers.
#[inline]
pub const fn zigzag_decode_u64(n: u64) -> i64 {
    ((n >> 1) as i64) ^ (-((n & 1) as i64))
}

/// LEB128 encodes an unsigned 32-bit integer into a byte slice.
/// Returns the number of bytes written.
/// 
/// # Panics
/// Panics if the provided buffer is too small. A u32 can take up to 5 bytes.
#[inline]
pub fn leb128_encode_u32(mut value: u32, buf: &mut [u8]) -> usize {
    let mut i = 0;
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if unlikely(value != 0) {
            byte |= 0x80; // Set continuation bit
        }
        buf[i] = byte;
        i += 1;
        if value == 0 {
            break;
        }
    }
    i
}

/// LEB128 encodes an unsigned 64-bit integer into a byte slice.
/// Returns the number of bytes written.
/// 
/// # Panics
/// Panics if the provided buffer is too small. A u64 can take up to 10 bytes.
#[inline]
pub fn leb128_encode_u64(mut value: u64, buf: &mut [u8]) -> usize {
    let mut i = 0;
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf[i] = byte;
        i += 1;
        if value == 0 {
            break;
        }
    }
    i
}

/// LEB128 decodes an unsigned 32-bit integer from a byte slice.
/// Returns a tuple of (decoded_value, bytes_read).
/// Returns `None` if the buffer is malformed (e.g. no termination bit found within 5 bytes).
#[inline]
pub fn leb128_decode_u32(buf: &[u8]) -> Option<(u32, usize)> {
    let mut result = 0u32;
    let mut shift = 0;
    let mut i = 0;

    loop {
        if unlikely(i >= buf.len() || i >= 5) {
            return None;
        }
        let byte = buf[i];
        result |= ((byte & 0x7F) as u32) << shift;
        i += 1;
        if likely((byte & 0x80) == 0) {
            break;
        }
        shift += 7;
    }
    Some((result, i))
}

/// LEB128 decodes an unsigned 64-bit integer from a byte slice.
/// Returns a tuple of (decoded_value, bytes_read).
/// Returns `None` if the buffer is malformed (e.g. no termination bit found within 10 bytes).
#[inline]
pub fn leb128_decode_u64(buf: &[u8]) -> Option<(u64, usize)> {
    let mut result = 0u64;
    let mut shift = 0;
    let mut i = 0;

    loop {
        if i >= buf.len() || i >= 10 {
            return None;
        }
        let byte = buf[i];
        result |= ((byte & 0x7F) as u64) << shift;
        i += 1;
        if (byte & 0x80) == 0 {
            break;
        }
        shift += 7;
    }
    Some((result, i))
}

/// Compresses an i32 into a byte buffer using ZigZag and LEB128 encoding.
/// Returns the number of bytes written.
#[inline]
pub fn compress_i32(value: i32, buf: &mut [u8]) -> usize {
    let z = zigzag_encode_i32(value);
    leb128_encode_u32(z, buf)
}

/// Decompresses an i32 from a byte buffer.
/// Returns a tuple of (decoded_value, bytes_read), or `None` if malformed.
#[inline]
pub fn decompress_i32(buf: &[u8]) -> Option<(i32, usize)> {
    let (z, size) = leb128_decode_u32(buf)?;
    Some((zigzag_decode_u32(z), size))
}

/// Compresses an i64 into a byte buffer using ZigZag and LEB128 encoding.
/// Returns the number of bytes written.
#[inline]
pub fn compress_i64(value: i64, buf: &mut [u8]) -> usize {
    let z = zigzag_encode_i64(value);
    leb128_encode_u64(z, buf)
}

/// Decompresses an i64 from a byte buffer.
/// Returns a tuple of (decoded_value, bytes_read), or `None` if malformed.
#[inline]
pub fn decompress_i64(buf: &[u8]) -> Option<(i64, usize)> {
    let (z, size) = leb128_decode_u64(buf)?;
    Some((zigzag_decode_u64(z), size))
}

/// A zero-allocation encoder for time-series integer data.
/// It maintains the previous value and uses Delta-encoding combined with 
/// ZigZag and LEB128 to achieve extreme compression for continuous data.
#[repr(align(64))]
pub struct TimeSeriesEncoder {
    last_value: i32,
}

#[inline(always)]
fn likely(b: bool) -> bool { b }

#[inline(always)]
fn unlikely(b: bool) -> bool { b }

impl TimeSeriesEncoder {
    /// Creates a new encoder with an initial reference value (usually 0).
    pub const fn new(initial_value: i32) -> Self {
        Self {
            last_value: initial_value,
        }
    }

    /// Encodes the next value in the time-series.
    /// Calculates the delta, updates the state, and compresses the delta.
    /// Returns the number of bytes written to `buf`.
    #[inline]
    pub fn encode_next(&mut self, current: i32, buf: &mut [u8]) -> usize {
        let delta = current.wrapping_sub(self.last_value);
        self.last_value = current;
        compress_i32(delta, buf)
    }

    /// Returns the current state (last seen value).
    pub const fn current_state(&self) -> i32 {
        self.last_value
    }
}

/// A zero-allocation decoder for time-series integer data.
pub struct TimeSeriesDecoder {
    last_value: i32,
}

impl TimeSeriesDecoder {
    /// Creates a new decoder with an initial reference value (must match encoder).
    pub const fn new(initial_value: i32) -> Self {
        Self {
            last_value: initial_value,
        }
    }

    /// Decodes the next value from the compressed byte buffer.
    /// Returns a tuple of (decoded_absolute_value, bytes_read).
    /// Returns `None` if the buffer is malformed.
    #[inline]
    pub fn decode_next(&mut self, buf: &[u8]) -> Option<(i32, usize)> {
        let (delta, size) = decompress_i32(buf)?;
        let current = self.last_value.wrapping_add(delta);
        self.last_value = current;
        Some((current, size))
    }

    /// Returns the current state (last decoded value).
    pub const fn current_state(&self) -> i32 {
        self.last_value
    }
}

/// A zero-allocation encoder for time-series timestamps using Delta-of-Deltas.
pub struct TimestampEncoder {
    last_timestamp: u64,
    last_delta: i64,
}

impl TimestampEncoder {
    /// Creates a new encoder with an initial reference timestamp.
    pub const fn new(initial_timestamp: u64) -> Self {
        Self {
            last_timestamp: initial_timestamp,
            last_delta: 0,
        }
    }

    /// Encodes the next timestamp.
    /// Returns the number of bytes written to `buf`.
    #[inline]
    pub fn encode_next(&mut self, current: u64, buf: &mut [u8]) -> usize {
        let current_delta = current.wrapping_sub(self.last_timestamp) as i64;
        let delta_of_delta = current_delta.wrapping_sub(self.last_delta);
        
        self.last_timestamp = current;
        self.last_delta = current_delta;
        
        compress_i64(delta_of_delta, buf)
    }

    /// Returns the current state (last seen timestamp).
    pub const fn current_state(&self) -> u64 {
        self.last_timestamp
    }
}

/// A zero-allocation decoder for time-series timestamps.
pub struct TimestampDecoder {
    last_timestamp: u64,
    last_delta: i64,
}

impl TimestampDecoder {
    pub const fn new(initial_timestamp: u64) -> Self {
        Self {
            last_timestamp: initial_timestamp,
            last_delta: 0,
        }
    }

    #[inline]
    pub fn decode_next(&mut self, buf: &[u8]) -> Option<(u64, usize)> {
        let (delta_of_delta, size) = decompress_i64(buf)?;
        let current_delta = self.last_delta.wrapping_add(delta_of_delta);
        let current = self.last_timestamp.wrapping_add(current_delta as u64);
        
        self.last_timestamp = current;
        self.last_delta = current_delta;
        
        Some((current, size))
    }
}

/// A zero-allocation encoder for time-series integer data with Run-Length Encoding (RLE).
/// It shifts the ZigZag encoded values by +1 to reserve `0` as the RLE escape code.
pub struct RleTimeSeriesEncoder {
    last_value: i32,
    run_count: u32,
}

impl RleTimeSeriesEncoder {
    pub const fn new(initial_value: i32) -> Self {
        Self {
            last_value: initial_value,
            run_count: 0,
        }
    }

    /// Encodes the next value.
    /// If the value is the same as the last, it increments the run counter and returns 0 bytes written.
    /// If the value changes, it writes the buffered run (if any) and the new value.
    #[inline]
    pub fn encode_next(&mut self, current: i32, buf: &mut [u8]) -> usize {
        let delta = current.wrapping_sub(self.last_value);
        if delta == 0 && self.run_count < u32::MAX {
            self.run_count += 1;
            return 0;
        }

        let mut size = 0;
        // If we have an accumulated run, flush it first
        if self.run_count > 0 {
            size += self.flush_run(buf);
        }

        // Encode the new delta
        self.last_value = current;
        let z = zigzag_encode_i32(delta).wrapping_add(1);
        size += leb128_encode_u32(z, &mut buf[size..]);
        size
    }

    /// Flushes any pending RLE run into the buffer.
    #[inline]
    pub fn flush(&mut self, buf: &mut [u8]) -> usize {
        if self.run_count > 0 {
            self.flush_run(buf)
        } else {
            0
        }
    }

    #[inline]
    fn flush_run(&mut self, buf: &mut [u8]) -> usize {
        let mut size = 0;
        // RLE escape code (0)
        size += leb128_encode_u32(0, buf);
        // Write count
        size += leb128_encode_u32(self.run_count, &mut buf[size..]);
        self.run_count = 0;
        size
    }
}

/// A zero-allocation decoder for RLE-compressed time-series integer data.
pub struct RleTimeSeriesDecoder {
    last_value: i32,
    run_count: u32,
}

impl RleTimeSeriesDecoder {
    pub const fn new(initial_value: i32) -> Self {
        Self {
            last_value: initial_value,
            run_count: 0,
        }
    }

    /// Decodes the next value.
    /// If currently unpacking an RLE run, returns the same value and 0 bytes read.
    #[inline]
    pub fn decode_next(&mut self, buf: &[u8]) -> Option<(i32, usize)> {
        if self.run_count > 0 {
            self.run_count -= 1;
            return Some((self.last_value, 0));
        }

        let (z_shifted, size) = leb128_decode_u32(buf)?;
        if z_shifted == 0 {
            // RLE escape found, decode count
            let (count, count_size) = leb128_decode_u32(&buf[size..])?;
            if count > 0 {
                self.run_count = count - 1;
            }
            return Some((self.last_value, size + count_size));
        }

        let z = z_shifted.wrapping_sub(1);
        let delta = zigzag_decode_u32(z);
        let current = self.last_value.wrapping_add(delta);
        self.last_value = current;
        
        Some((current, size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use alloc::vec;

    #[test]
    fn test_zigzag_32() {
        assert_eq!(zigzag_encode_i32(0), 0);
        assert_eq!(zigzag_encode_i32(-1), 1);
        assert_eq!(zigzag_encode_i32(1), 2);
        assert_eq!(zigzag_encode_i32(-2), 3);
        assert_eq!(zigzag_encode_i32(2), 4);
        assert_eq!(zigzag_encode_i32(i32::MAX), u32::MAX - 1);
        assert_eq!(zigzag_encode_i32(i32::MIN), u32::MAX);

        assert_eq!(zigzag_decode_u32(0), 0);
        assert_eq!(zigzag_decode_u32(1), -1);
        assert_eq!(zigzag_decode_u32(2), 1);
        assert_eq!(zigzag_decode_u32(3), -2);
        assert_eq!(zigzag_decode_u32(u32::MAX - 1), i32::MAX);
        assert_eq!(zigzag_decode_u32(u32::MAX), i32::MIN);
    }

    #[test]
    fn test_leb128_32() {
        let mut buf = [0u8; 5];
        
        let size = leb128_encode_u32(0, &mut buf);
        assert_eq!(size, 1);
        assert_eq!(buf[0], 0x00);
        assert_eq!(leb128_decode_u32(&buf).unwrap(), (0, 1));

        let size = leb128_encode_u32(127, &mut buf);
        assert_eq!(size, 1);
        assert_eq!(buf[0], 0x7F);
        assert_eq!(leb128_decode_u32(&buf).unwrap(), (127, 1));

        let size = leb128_encode_u32(128, &mut buf);
        assert_eq!(size, 2);
        assert_eq!(buf[0..2], [0x80, 0x01]);
        assert_eq!(leb128_decode_u32(&buf).unwrap(), (128, 2));

        let size = leb128_encode_u32(u32::MAX, &mut buf);
        assert_eq!(size, 5);
        assert_eq!(leb128_decode_u32(&buf).unwrap(), (u32::MAX, 5));
    }

    #[test]
    fn test_compress_decompress_i32() {
        let mut buf = [0u8; 5];
        let values = [0, -1, 1, 64, -64, 127, -127, 128, -128, 10000, -10000, i32::MAX, i32::MIN];
        
        for &v in &values {
            let size = compress_i32(v, &mut buf);
            let (decoded, read_size) = decompress_i32(&buf).unwrap();
            assert_eq!(v, decoded);
            assert_eq!(size, read_size);
        }
    }

    #[test]
    fn test_time_series_encoder() {
        let n = core::hint::black_box(100);

        let mut encoder = TimeSeriesEncoder::new(20);
        let mut decoder = TimeSeriesDecoder::new(20);
        let mut buf = [0u8; 10];

        // Smooth sequence
        let sequence = [20, 21, 22, 23, 23, 23, 24];
        for &val in &sequence {
            let val = core::hint::black_box(val);
            let size = encoder.encode_next(val, &mut buf);
            let (decoded, read_size) = decoder.decode_next(&buf).unwrap();
            assert_eq!(val, decoded);
            assert_eq!(size, read_size);
            // Deltas: 0, 1, 1, 1, 0, 0, 1 -> all should compress to 1 byte!
            assert_eq!(size, 1);
        }

        for i in 0..n {
            let val = core::hint::black_box(24 + i);
            let size = encoder.encode_next(val, &mut buf);
            let (decoded, read_size) = decoder.decode_next(&buf).unwrap();
            assert_eq!(val, decoded);
            assert_eq!(size, read_size);
        }

        // Data spike
        let spike_val = core::hint::black_box(105); 
        let size = encoder.encode_next(spike_val, &mut buf);
        let (decoded, read_size) = decoder.decode_next(&buf).unwrap();
        assert_eq!(spike_val, decoded);
        assert_eq!(size, read_size);
        // Delta from 24+n-1 to 105 is large.
        
        // Back to smooth
        let val = core::hint::black_box(106);
        let size = encoder.encode_next(val, &mut buf);
        let (decoded, read_size) = decoder.decode_next(&buf).unwrap();
        assert_eq!(val, decoded);
        assert_eq!(size, read_size);
    }

    #[test]
    fn test_timestamp_encoder() {
        let mut encoder = TimestampEncoder::new(1600000000);
        let mut decoder = TimestampDecoder::new(1600000000);
        let mut buf = [0u8; 10];

        // Simulate 100ms intervals with minor jitter
        let sequence = [
            1600000100, // Delta: 100, DoD: 100
            1600000200, // Delta: 100, DoD: 0
            1600000300, // Delta: 100, DoD: 0
            1600000401, // Delta: 101, DoD: 1
            1600000499, // Delta: 98,  DoD: -3
        ];

        for &val in &sequence {
            let size = encoder.encode_next(val, &mut buf);
            let (decoded, read_size) = decoder.decode_next(&buf).unwrap();
            assert_eq!(val, decoded);
            assert_eq!(size, read_size);
            
            // The delta-of-deltas should be small enough to fit in 1-2 bytes
            assert!(size <= 2);
        }
    }

    #[test]
    fn test_rle_time_series_encoder() {
        let mut encoder = RleTimeSeriesEncoder::new(20);
        let mut decoder = RleTimeSeriesDecoder::new(20);
        let mut buf = vec![0u8; 1024];
        let mut written_size = 0;

        let sequence = [
            20, 20, 20, 20, 20, // 5 identical values
            21, // 1 change
            21, 21, 21, // 3 identical values
            25, // spike
        ];

        for &val in &sequence {
            written_size += encoder.encode_next(val, &mut buf[written_size..]);
        }
        written_size += encoder.flush(&mut buf[written_size..]);
        buf.truncate(written_size);

        // Verify size:
        // [20,20,20,20,20] -> Escape(0) + Count(5) = 2 bytes
        // 21 -> Delta(1) -> 1 byte
        // [21,21,21] -> Escape(0) + Count(3) = 2 bytes
        // 25 -> Delta(4) -> 1 byte
        // Total should be around 6 bytes for 10 values
        assert_eq!(written_size, 6);

        // Decode
        let mut offset = 0;
        let mut decoded_seq = Vec::new();
        while decoded_seq.len() < sequence.len() {
            let (val, read_bytes) = decoder.decode_next(&buf[offset..]).unwrap();
            decoded_seq.push(val);
            offset += read_bytes;
        }

        assert_eq!(sequence.to_vec(), decoded_seq);
    }
}
