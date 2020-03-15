use crate::ModbusError;

mod modbus_rtu;
mod tcp_modbus;

pub trait ModbusProtocol {
    /// The maximum allowable length of an Application Data Unit in this protocol
    const ADU_MAX_LENGTH: usize;

    /// A type representing the header for this particular packet
    ///
    /// It is not necessarily bit-compatible with the underlying representation.
    type Header: core::fmt::Debug + Clone;

    /// Extract the length of the given ADU
    ///
    /// If there is not enough data to extract the length, return None.
    ///
    /// If determining the length information requires examining the function code, an unrecognized
    /// function code is represented by `Some(Err(BadFuncCode))`.
    fn adu_length(data: &[u8]) -> Result<usize, ModbusError>;

    /// Extract the header data associated with the given ADU
    ///
    /// If there is not enough data to extract a complete header, return None.
    ///
    /// If determining the header information requires examining the function code, an unrecognized
    /// function code is represented by `Some(Err(BadFuncCode))`.
    fn adu_header(data: &[u8]) -> Result<Self::Header, ModbusError>;

    /// Determine if the ADU matches the checksum
    ///
    /// If determining the checksum status requires examining the function code, an unrecognized
    /// function code is represented by `Some(Err(BadFuncCode))`.
    fn adu_check(data: &[u8]) -> Result<(), ModbusError>;

    /// Get the header information the inner PDU data, checking the checksum first
    fn pdu_body(data: &[u8]) -> Result<&[u8], ModbusError>;
}

pub use modbus_rtu::ModbusRtu;
pub use tcp_modbus::TcpModbus;
