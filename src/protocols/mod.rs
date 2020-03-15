//! Tools for implementing MODBUS over various transport layers
//!
//! The MODBUS protocol can be transported over different media, each of which wraps the MODBUS
//! message slightly differently. The inner MODBUS message is known as the protocol data unit (PDU),
//! and the wrapped message that is sent over the wire is known as the application data unit (ADU).
//! This module contains utilities for wrapping and unwrapping them.
//!
//! MODBUS protocols are implemented using an empty struct that implements the `ModbusProtocol`
//! trait. This defines several utility functions for ADU-PDU conversion, as well as a header type.
//!
//! The primary two MODBUS variants are TCP MODBUS, which uses a TCP stream as its transport, and
//! MODBUS RTU, which uses RS-232 or RS-485 as its transport.

use crate::ModbusError;

mod modbus_rtu;
mod tcp_modbus;

/// A MODBUS transport protocol
///
/// Generally, this is implemented on a zero-sized type.
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

pub use modbus_rtu::{ModbusRtu, ModbusRtuHeader};
pub use tcp_modbus::{TcpModbus, TcpModbusHeader};
