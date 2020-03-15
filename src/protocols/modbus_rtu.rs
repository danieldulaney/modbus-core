
use super::ModbusProtocol;
use crate::ModbusError;

pub struct ModbusRtu;

#[derive(Debug, Clone)]
pub struct ModbusRtuHeader {
    pub address: u8,
    pub crc: u16,
}

impl ModbusProtocol for ModbusRtu {
    const ADU_MAX_LENGTH: usize = 256;

    type Header = ModbusRtuHeader;

    fn adu_length(data: &[u8]) -> Result<usize, ModbusError> {
        todo!();
    }

    fn adu_header(data: &[u8]) -> Result<Self::Header, ModbusError> {
        todo!();
    }

    fn adu_check(data: &[u8]) -> Result<(), ModbusError> {
        todo!();
    }

    fn pdu_body(data: &[u8]) -> Result<&[u8], ModbusError> {
        todo!();
    }
}
