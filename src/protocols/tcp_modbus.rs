use super::ModbusProtocol;
use crate::ModbusError;

pub struct TcpModbus;

// Length of the MODBUS Application Protocol header
// 2-byte transaction ID, 2-byte protocol ID, 2-byte length, 1-byte unit ID
const MBAP_LENGTH: usize = 7;

#[derive(Debug, Clone)]
pub struct TcpModbusHeader {
    pub transaction_id: u16,
    pub protocol_id: u16,
    pub length: u16,
    pub unit_id: u8,
}

impl TcpModbus {
    fn protocol_id(data: &[u8]) -> Option<u16> {
        Some(u16::from_be_bytes([*data.get(2)?, *data.get(3)?]))
    }

    fn transaction_id(data: &[u8]) -> Option<u16> {
        Some(u16::from_be_bytes([*data.get(0)?, *data.get(1)?]))
    }

    fn length(data: &[u8]) -> Option<u16> {
        Some(u16::from_be_bytes([*data.get(4)?, *data.get(5)?]))
    }

    fn unit_id(data: &[u8]) -> Option<u8> {
        data.get(6).map(|&x| x)
    }
}

impl ModbusProtocol for TcpModbus {
    const ADU_MAX_LENGTH: usize = 260;

    type Header = TcpModbusHeader;

    fn adu_length(data: &[u8]) -> Result<usize, ModbusError> {
        match Self::length(data) {
            None => Err(ModbusError::NotEnoughData),
            Some(v) => Ok(v as usize + MBAP_LENGTH),
        }
    }

    fn adu_header(data: &[u8]) -> Result<Self::Header, ModbusError> {
        use ModbusError::NotEnoughData;

        Ok(Self::Header {
            transaction_id: Self::transaction_id(data).ok_or(NotEnoughData)?,
            protocol_id: Self::protocol_id(data).ok_or(NotEnoughData)?,
            length: Self::length(data).ok_or(NotEnoughData)?,
            unit_id: Self::unit_id(data).ok_or(NotEnoughData)?,
        })
    }

    /// TCP MODBUS doesn't have checksums, so this just confirms that there's
    /// enough data to make up a whole ADU
    fn adu_check(data: &[u8]) -> Result<(), ModbusError> {
        use ModbusError::NotEnoughData;

        let length = Self::adu_length(data)?;

        if data.len() > length {
            Ok(())
        } else {
            Err(NotEnoughData)
        }
    }

    fn pdu_body(data: &[u8]) -> Result<&[u8], ModbusError> {
        Self::adu_check(data)?;

        // We just checked that the length is correct in adu_check, so this
        // won't panic
        Ok(&data[MBAP_LENGTH..])
    }
}
