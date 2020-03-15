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
            self.clear_buffer()
        }

        let original_buffer_size = self.size_used;
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
        if self.size_used < adu_length {
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
        self.raw_buffer[self.size_used..].copy_from_slice(data);
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
}

pub struct Packet<'p, P: ModbusProtocol> {
    pub pdu: &'p [u8],
    pub header: P::Header,
}
