use super::ModbusProtocol;
use crate::ModbusError;

/// MODBUS RTU protocol implementation
///
/// This currently consists of unimplemented stubs, and will panic if used.
pub struct ModbusRtu;

/// MODBUS RTU header data
#[derive(Debug, Clone)]
pub struct ModbusRtuHeader {
    pub address: u8,
    pub crc: u16,
}

impl ModbusProtocol for ModbusRtu {
    const ADU_MAX_LENGTH: usize = 256;

    type Header = ModbusRtuHeader;

    fn adu_length(data: &[u8]) -> Result<usize, ModbusError> {
        panic!(
            "Not yet implemented: adu_length ({}-byte argument)",
            data.len()
        );
    }

    fn adu_header(data: &[u8]) -> Result<Self::Header, ModbusError> {
        panic!(
            "Not yet implemented: adu_header ({}-byte argument)",
            data.len()
        );
    }

    fn adu_check(data: &[u8]) -> Result<(), ModbusError> {
        panic!(
            "Not yet implemented: adu_check ({}-byte argument)",
            data.len()
        );
    }

    fn pdu_body(data: &[u8]) -> Result<&[u8], ModbusError> {
        panic!(
            "Not yet implemented: pdu_body ({}-byte argument)",
            data.len()
        );
    }
}
