//! Tools for converting a byte stream into a sequence of MODBUS messages
//!
//! See the `RecvBuffer` struct for details.

use crate::protocols::ModbusProtocol;
use crate::ModbusError;

// See https://stackoverflow.com/questions/53619695/
const fn const_max(a: usize, b: usize) -> usize {
    [a, b][(a < b) as usize]
}

// This just has to be the largest value used by any protocol
// If additional protocols are added, this value should be increased by adding
// nested const_max calls
//
// Hack until https://github.com/rust-lang/rust/issues/43408 is resolved
const BUFFER_LEN: usize = const_max(
    crate::protocols::TcpModbus::ADU_MAX_LENGTH,
    crate::protocols::ModbusRtu::ADU_MAX_LENGTH,
);

/// Converts a raw byte stream into a sequence of MODBUS packets
///
/// The critical method is `process`. By calling it with new data, will get broken-out MODBUS
/// packets. Each packet corresponds to an application data unit (ADU), and contains some header
/// data (dependent on the underlying transport protocol) and a protocol data unit (PDU) that does
/// not depend on the underlying transport protocol.
pub struct RecvBuffer<P: ModbusProtocol> {
    // This is a critical invariant:
    // If the buffer ever contains a complete APU, contains_complete must be true and size_used
    // must hold its actual length.
    raw_buffer: [u8; BUFFER_LEN],
    size_used: usize,
    contains_complete: bool,
    _protocol: core::marker::PhantomData<P>,
}

impl<P: ModbusProtocol> RecvBuffer<P> {
    /// Create a new receive buffer
    pub fn new() -> Self {
        RecvBuffer {
            raw_buffer: [0; BUFFER_LEN],
            size_used: 0,
            contains_complete: false,
            _protocol: Default::default(),
        }
    }

    /// Process some received data through the buffer
    ///
    /// Your packet data is appended to any data already in the buffer, and checked to see if it
    /// makes an ADU. If:
    ///
    /// - It makes exactly 1 ADU:
    ///     - You get `Ok` with that ADU and an empty slice
    /// - It makes more than 1 ADU:
    ///     - You get `Ok` with that ADU and a slice containing any excess data
    ///     - You should call `process` again with that slice after handling the ADU
    /// - It makes less than 1 ADU:
    ///     - You get `Err` with `ModbusError::NotEnoughData`
    ///     - The unfinished data is added to the buffer
    /// - It's somehow invalid (length too long, bad function code, etc.)
    ///     - You get `Err` with some other error
    ///     - All data in the buffer is cleared, including whatever you passed in
    pub fn process<'p, 'b>(
        &'b mut self,
        data: &'p [u8],
    ) -> Result<(Packet<'b, P>, &'p [u8]), ModbusError> {
        use crate::ModbusError::NotEnoughData;

        if self.contains_complete {
            self.clear_buffer();
            self.contains_complete = false;
        }

        let original_buffer_size = self.used();
        let length_to_add = core::cmp::min(self.space_left(), data.len());

        self.add_data(&data[..length_to_add]);

        let adu_length = match P::adu_length(&self.buffer()) {
            Ok(l) => l,

            // Not enough data to determine ADU length
            // For TCP MODBUS, we need 6 bytes
            // For MODBUS RTU, it might be more
            Err(NotEnoughData) => return Err(NotEnoughData),

            // Something is very wrong. Give up and purge any bad data
            Err(e) => {
                self.clear_buffer();
                return Err(e);
            }
        };

        // We got something in between enough to determine the length and a full ADU
        if self.used() < adu_length {
            return Err(NotEnoughData);
        }

        // This is where the remaining data starts, but it's also the amount of data added to the
        // buffer that became part of the ADU.
        // No underflow because if adu_length < original_buffer_size, that violates an invariant
        // of this struct
        let remaining_data_index = adu_length - original_buffer_size;

        // At this point, we know we have a complete ADU
        // Set up the tracking fields to maintain our invariants
        self.contains_complete = true;
        self.trim_to(adu_length);

        Ok((
            Packet {
                header: P::adu_header(self.buffer())?,
                pdu: P::pdu_body(self.buffer())?,
            },
            &data[remaining_data_index..],
        ))
    }

    fn space_left(&self) -> usize {
        debug_assert!(self.size_used < self.raw_buffer.len());

        self.raw_buffer.len() - self.size_used
    }

    /// # Panics
    ///
    /// Panics if `data` is longer than the available length
    fn add_data(&mut self, data: &[u8]) {
        self.raw_buffer[self.size_used..self.size_used + data.len()].copy_from_slice(data);
        self.size_used += data.len();
    }

    fn clear_buffer(&mut self) {
        self.size_used = 0;
    }

    fn trim_to(&mut self, length: usize) {
        debug_assert!(length <= self.size_used);
        debug_assert!(self.size_used <= self.raw_buffer.len());

        self.size_used = length;
    }

    fn buffer(&self) -> &[u8] {
        &self.raw_buffer[..self.size_used]
    }

    /// Determine how much of the buffer is currently in use
    ///
    /// # Examples
    ///
    /// ```
    /// use modbus_core::recv_buffer::*;
    /// use modbus_core::protocols::*;
    /// use modbus_core::ModbusError::*;
    ///
    /// let mut buf: RecvBuffer<TcpModbus> = RecvBuffer::new();
    ///
    /// // The buffer starts off empty
    /// assert_eq!(buf.used(), 0);
    ///
    /// // 3 bytes isn't enough to make a full TCP MODBUS packet, so it gets buffered
    /// assert_eq!(buf.process(&[1, 2, 3]).unwrap_err(), NotEnoughData);
    /// assert_eq!(buf.used(), 3);
    /// ```
    pub fn used(&self) -> usize {
        self.size_used
    }
}

#[derive(PartialEq)]
/// A representation of a single MODBUS ADU
///
/// Consists of a protocol-dependent header, as well as a protocol data unit that is the same
/// regardless of the underlying transport protocol.
pub struct Packet<'p, P: ModbusProtocol> {
    pub pdu: &'p [u8],
    pub header: P::Header,
}

impl<'p, P: ModbusProtocol> core::fmt::Debug for Packet<'p, P> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("Packet")
            .field("header", &self.header)
            .field("pdu", &format_args!("{} bytes", self.pdu.len()))
            .finish()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::protocols::*;
    use crate::test_data::*;
    use crate::ModbusError::NotEnoughData;

    const FOUR_ADUS_LEN: usize = 2 * (ADU1_TCP.len() + ADU2_TCP.len());

    /// A utility function that fills the given buffer (at least FOUR_ADUS_LEN long) with four TCP
    /// ADUs.
    fn four_tcp_adus(buffer: &mut [u8]) {
        let mut current = 0;

        for slice in [ADU1_TCP, ADU2_TCP, ADU2_TCP, ADU1_TCP].iter() {
            let next = current + slice.len();

            &mut buffer[current..next].copy_from_slice(slice);

            current = next;
        }
    }

    #[test]
    fn tcp_exactly_one_adu() {
        let mut buf = RecvBuffer::<TcpModbus>::new();

        let (packet, slice) = buf.process(ADU1_TCP).unwrap();

        assert!(slice.is_empty());
        assert_eq!(packet.header, ADU1_HEADER);
        assert_eq!(packet.pdu, ADU1_PDU());

        let (packet, slice) = buf.process(ADU2_TCP).unwrap();

        assert!(slice.is_empty());
        assert_eq!(packet.header, ADU2_HEADER);
        assert_eq!(packet.pdu, ADU2_PDU());
    }

    /// Put 4 ADUs in one buffer, then run it through 10 bytes at a time.
    ///
    /// They will mostly fail with not enough bytes, but a few known chunks will succeed.
    #[test]
    fn tcp_four_adus_chunked() {
        let chunk_size = 10;

        let mut input: [u8; FOUR_ADUS_LEN] = [0; FOUR_ADUS_LEN];
        four_tcp_adus(&mut input);
        let input = input;

        let mut buf = RecvBuffer::<TcpModbus>::new();

        for (index, chunk) in input.chunks(chunk_size).enumerate() {
            let result = buf.process(chunk);

            if index == 20 {
                // Finished first ADU (ADU1_TCP)
                let (packet, slice) = result.unwrap();

                assert_eq!(packet.pdu, ADU1_PDU());
                assert_eq!(packet.header, ADU1_HEADER);

                // First byte of next packet
                assert_eq!(slice, &ADU2_TCP[..1]);
                assert_eq!(buf.process(slice).unwrap_err(), NotEnoughData);
            } else if index == 22 {
                // Finished second ADU (ADU2_TCP)
                let (packet, slice) = result.unwrap();

                assert_eq!(packet.pdu, ADU2_PDU());
                assert_eq!(packet.header, ADU2_HEADER);

                // First 9 bytes of next packet
                assert_eq!(slice, &ADU2_TCP[..9]);
                assert_eq!(buf.process(slice).unwrap_err(), NotEnoughData);
            } else if index == 23 {
                // Finished third ADU (ADU2_TCP)
                let (packet, slice) = result.unwrap();

                assert_eq!(packet.pdu, ADU2_PDU());
                assert_eq!(packet.header, ADU2_HEADER);

                // First 7 bytes of next packet
                assert_eq!(slice, &ADU1_TCP[..7]);
                assert_eq!(buf.process(slice).unwrap_err(), NotEnoughData);
            } else if index == 44 {
                // Finished fourth ADU (ADU1_TCP)
                let (packet, slice) = result.unwrap();

                assert_eq!(packet.pdu, ADU1_PDU());
                assert_eq!(packet.header, ADU1_HEADER);

                // No data remaining
                assert_eq!(slice, &[]);
            } else {
                assert_eq!(result.unwrap_err(), NotEnoughData);
            }
        }
    }

    #[test]
    fn tcp_four_adus_straight_through() {
        let mut input: [u8; FOUR_ADUS_LEN] = [0; FOUR_ADUS_LEN];
        four_tcp_adus(&mut input);
        let input = input;

        let mut buf = RecvBuffer::<TcpModbus>::new();

        let (packet, slice) = buf.process(&input).unwrap();
        assert_eq!(slice, &input[ADU1_TCP.len()..]);
        assert_eq!(packet.pdu, ADU1_PDU());
        assert_eq!(packet.header, ADU1_HEADER);

        let (packet, slice) = buf.process(slice).unwrap();
        assert_eq!(slice, &input[ADU1_TCP.len() + ADU2_TCP.len()..]);
        assert_eq!(packet.pdu, ADU2_PDU());
        assert_eq!(packet.header, ADU2_HEADER);

        let (packet, slice) = buf.process(slice).unwrap();
        assert_eq!(slice, &input[ADU1_TCP.len() + 2 * ADU2_TCP.len()..]);
        assert_eq!(packet.pdu, ADU2_PDU());
        assert_eq!(packet.header, ADU2_HEADER);

        let (packet, slice) = buf.process(slice).unwrap();
        assert_eq!(slice, &[]);
        assert_eq!(packet.pdu, ADU1_PDU());
        assert_eq!(packet.header, ADU1_HEADER);
    }

    #[test]
    fn tcp_four_adus_byte_by_byte() {
        let mut input: [u8; FOUR_ADUS_LEN] = [0; FOUR_ADUS_LEN];
        four_tcp_adus(&mut input);
        let input = input;

        let mut buf = RecvBuffer::<TcpModbus>::new();

        for (index, byte) in input.chunks(1).enumerate() {
            let result = buf.process(byte);

            dbg!(index);

            if (index == 208) {
                let (packet, slice) = result.unwrap();

                assert_eq!(slice, &[]);
                assert_eq!(packet.pdu, ADU1_PDU());
                assert_eq!(packet.header, ADU1_HEADER);
            } else if (index == 220) {
                let (packet, slice) = result.unwrap();

                assert_eq!(slice, &[]);
                assert_eq!(packet.pdu, ADU2_PDU());
                assert_eq!(packet.header, ADU2_HEADER);
            } else if (index == 232) {
                let (packet, slice) = result.unwrap();

                assert_eq!(slice, &[]);
                assert_eq!(packet.pdu, ADU2_PDU());
                assert_eq!(packet.header, ADU2_HEADER);
            } else if (index == 441) {
                let (packet, slice) = result.unwrap();

                assert_eq!(slice, &[]);
                assert_eq!(packet.pdu, ADU1_PDU());
                assert_eq!(packet.header, ADU1_HEADER);
            } else {
                assert_eq!(result.unwrap_err(), NotEnoughData);
            }
        }
    }
}
