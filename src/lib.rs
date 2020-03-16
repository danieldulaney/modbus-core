//#![no_std]

pub mod bit_pack;
pub mod protocols;
pub mod recv_buffer;

#[cfg(test)]
mod test_data;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coil {
    On,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Query,
    Response,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModbusError {
    /// Unrecognized function code
    BadFuncCode,

    /// Error checking failed
    ///
    /// This could be a CRC check (for example, for MODBUS RTU), or just correct-length check
    BadErrorCheck,

    /// Length is either too long or too short
    ///
    /// MODBUS sets the maximum PDU length at 253 characters.
    BadLength,

    /// There isn't enough data
    NotEnoughData,
}
