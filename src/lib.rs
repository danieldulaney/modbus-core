#![no_std]

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
    BadFuncCode,
    BadErrorCheck,
    NotEnoughData,
}
