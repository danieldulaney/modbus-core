#![no_std]

pub mod bit_pack;
pub mod protocols;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coil {
    On,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModbusError {
    BadFuncCode,
    BadErrorCheck,
    NotEnoughData,
}
