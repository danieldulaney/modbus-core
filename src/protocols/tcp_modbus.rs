use super::ModbusProtocol;
use crate::ModbusError;

/// TCP MODBUS protocol implementation
///
/// TCP MODBUS has a header known as the MODBUS Application Protocol header (MBAP). It includes a
/// length field that can be used to easily separate Application Data Units (ADUs) from each other.
/// However, the length field includes everything after itself, including the function code (not
/// part of the MBAP) and the unit identifier field (which is part of the MBAP). This means that
/// the MBAP length (7 bytes) and the excluded length (6 bytes) are different values.
///
/// Immediately after the MBAP, the protocol data unit (PDU) begins, starting with the function
/// code.
///
/// Visually, a TCP MODBUS ADU looks like this:
///
/// <table>
///   <tr>
///     <th>Offset</th>
///     <th>Field</th>
///     <th>Section</th>
///     <th>Included in length?</th>
///   </tr>
///   <tr>
///     <td>0</td>
///     <td rowspan="2" style="vertical-align:middle">Transaction ID</td>
///     <td rowspan="7" style="vertical-align:middle">MBAP</td>
///     <td rowspan="6" style="vertical-align:middle">No</td>
///   </tr>
///   <tr>
///     <td>1</td>
///   </tr>
///   <tr>
///     <td>2</td>
///     <td rowspan="2" style="vertical-align:middle">Protocol ID</td>
///   </tr>
///   <tr>
///     <td>3</td>
///   </tr>
///   <tr>
///     <td>4</td>
///     <td rowspan="2" style="vertical-align:middle">Length</td>
///   </tr>
///   <tr>
///     <td>5</td>
///   </tr>
///   <tr>
///     <td>6</td>
///     <td>Unit ID</td>
///     <td rowspan="3" style="vertical-align:middle">Yes</td>
///   </tr>
///   <tr>
///     <td>7</td>
///     <td>Function Code</td>
///     <td rowspan="2" style="vertical-align:middle">PDU</td>
///   </tr>
///   <tr>
///     <td>8...</td>
///     <td>Continuing PDU Data</td>
///   </tr>
/// </table>
///
/// This has some implications for implementing `ModbusProtocol` for TCP.
/// - `Header` includes all of the items in the MBAP, including the unit ID, but not the function
///   code.
/// - `adu_length` returns the length field + 6, because the length field already includes the unit
///   ID.
/// - `pdu_body` returns PDU data starting at index 7. If you want the unit ID, you need to get it
///   with `adu_header`.
pub struct TcpModbus;

// Length of the MODBUS Application Protocol header
// 2-byte transaction ID, 2-byte protocol ID, 2-byte length, 1-byte unit ID
const MBAP_LENGTH: usize = 7;

// Number of APU bytes excluded from the length field
// This is slightly different from the MBAP length because the 1-byte unit ID is
// included in the MBAP but falls after the length field, and thus excluded from
// the length field
const EXCLUDED_LENGTH: usize = 6;

const MAX_PDU_LENGTH: usize = 253;

/// TCP MODBUS header data
#[derive(Debug, Clone, PartialEq)]
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
        use ModbusError::{NotEnoughData, BadLength};

        // The ADU length is the value of the length field + the number of bytes
        // excluded from that field
        let adu_length = Self::length(data).ok_or(NotEnoughData)? as usize + EXCLUDED_LENGTH;

        // Check if the length is not too long
        if adu_length <= Self::ADU_MAX_LENGTH {
            Ok(adu_length)
        } else {
            Err(BadLength)
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

    /// TCP MODBUS doesn't have application-layer checksums, so this just confirms that there's
    /// at least enough data to make up a whole ADU
    fn adu_check(data: &[u8]) -> Result<(), ModbusError> {
        use ModbusError::NotEnoughData;

        let length = Self::adu_length(data)?;

        if data.len() >= length {
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

#[cfg(test)]
mod test {

    use super::*;
    use crate::test_data::*;
    use crate::ModbusError::*;

    #[test]
    fn tcp_adu_length() {
        for i in 0..=ADU1_TCP.len() {
            let len = TcpModbus::adu_length(&ADU1_TCP[..i]);

            if i < EXCLUDED_LENGTH {
                assert_eq!(len, Err(NotEnoughData));
            } else {
                assert_eq!(len, Ok(ADU1_ADU_LENGTH));
            }
        }

        for i in 0..=ADU2_TCP.len() {
            let len = TcpModbus::adu_length(&ADU2_TCP[..i]);

            if i < EXCLUDED_LENGTH {
                assert_eq!(len, Err(NotEnoughData));
            } else {
                assert_eq!(len, Ok(ADU2_ADU_LENGTH));
            }
        }
    }

    #[test]
    fn tcp_adu_header() {
        for i in 0..=ADU1_TCP.len() {
            let header = TcpModbus::adu_header(&ADU1_TCP[..i]);

            if i < MBAP_LENGTH {
                assert_eq!(header, Err(NotEnoughData));
            } else {
                assert_eq!(
                    header,
                    Ok(TcpModbusHeader {
                        length: ADU1_LENGTH,
                        protocol_id: ADU1_PROTO_ID,
                        transaction_id: ADU1_TRANS_ID,
                        unit_id: ADU1_UNIT_ID,
                    })
                );
            }
        }

        for i in 0..=ADU2_TCP.len() {
            let header = TcpModbus::adu_header(&ADU2_TCP[..i]);

            if i < MBAP_LENGTH {
                assert_eq!(header, Err(NotEnoughData));
            } else {
                assert_eq!(
                    header,
                    Ok(TcpModbusHeader {
                        length: ADU2_LENGTH,
                        protocol_id: ADU2_PROTO_ID,
                        transaction_id: ADU2_TRANS_ID,
                        unit_id: ADU2_UNIT_ID,
                    })
                );
            }
        }
    }

    #[test]
    fn tcp_adu_check() {
        for i in 0..=ADU1_TCP.len() {
            let result = TcpModbus::adu_check(&ADU1_TCP[..i]);

            if i < ADU1_ADU_LENGTH {
                assert_eq!(result, Err(NotEnoughData));
            } else {
                assert_eq!(result, Ok(()));
            }
        }

        for i in 0..=ADU2_TCP.len() {
            let result = TcpModbus::adu_check(&ADU2_TCP[..i]);

            if i < ADU2_ADU_LENGTH {
                assert_eq!(result, Err(NotEnoughData));
            } else {
                assert_eq!(result, Ok(()));
            }
        }
    }

    #[test]
    fn pdu_body() {
        for i in 0..=ADU1_TCP.len() {
            let result = TcpModbus::pdu_body(&ADU1_TCP[..i]);

            if i < ADU1_ADU_LENGTH {
                assert_eq!(result, Err(NotEnoughData));
            } else {
                assert_eq!(result, Ok(ADU1_PDU()));
            }
        }

        for i in 0..=ADU2_TCP.len() {
            let result = TcpModbus::pdu_body(&ADU2_TCP[..i]);

            if i < ADU2_ADU_LENGTH {
                assert_eq!(result, Err(NotEnoughData));
            } else {
                assert_eq!(result, Ok(ADU2_PDU()));
            }
        }
    }

    #[test]
    fn tcp_adu_bad_length() {
        let one_shorter: &[u8] = &[0, 0, 0, 0, 0, 253]; // ADU length 259
        let max_len: &[u8] = &[0, 0, 0, 0, 0, 254]; // ADU length 260
        let too_long: &[u8] = &[0, 0, 0, 0, 0, 255]; // ADU length 261
        let one_more: &[u8] = &[0, 0, 0, 0, 1, 0]; // ADU length 262

        assert_eq!(TcpModbus::adu_length(one_shorter), Ok(259));
        assert_eq!(TcpModbus::adu_length(max_len), Ok(260));
        assert_eq!(TcpModbus::adu_length(too_long), Err(BadLength));
        assert_eq!(TcpModbus::adu_length(one_more), Err(BadLength));
    }
}
